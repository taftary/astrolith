//! Blocks resident near the camera in the live cells (#1104 T6, Spec v2
//! Goals 1, 2, 4).
//!
//! L1 and L5 cells hold their fixture after [`crate::nest::generate_cell`]
//! (the eight old regions, the central ball of 5,000 stars). Each frame
//! [`Streamed::refresh`](crate::streamed::Streamed::refresh) adds the blocks nearest the camera to every live
//! cell that streams (open, parent, grandparent), so the layers drawn
//! around the open cell read the same blocks. A block is made once and
//! kept while it stays resident; the cell's markers are rebuilt only when
//! the set of blocks changes, and a still camera does no work at all
//! (Spec v2 AC9). Every rebuild bumps [`Streamed::revision`](crate::streamed::Streamed::revision), which caches
//! keyed by the cell seed (labels, star glows) read too. Pure
//! (`E-CORE-NO-BEVY`, `E-DET-TIERS`).

use crate::blocks::{BlockGrid, BlockKey, block, resident};
use crate::r#gen::{Generated, Point};
use crate::nest::path_level;
use crate::regions_l1::{GRID as L1_GRID, Regions};
use crate::stars_l5::{OUTER_GRID, OuterStars, STAR_LEVEL};

/// Markers kept resident per streamed cell beyond its fixture: one and a
/// half times the High Marker detail budget, so the nearest ones drawn
/// never run short.
pub const RESIDENT_MARKERS: u32 = 7_500;

/// Which content a streamed cell holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// L1 regions ([`crate::regions_l1`]).
    Regions,
    /// L5 stars beyond the central ball ([`crate::stars_l5`]).
    Stars,
}

impl Field {
    /// The field of the cell reached by `chain`, if it streams.
    #[must_use]
    pub fn for_chain(chain: &[u32]) -> Option<Field> {
        if chain.is_empty() {
            return Some(Field::Regions);
        }
        (path_level(chain).get() == STAR_LEVEL).then_some(Field::Stars)
    }

    /// The block grid of this field.
    #[must_use]
    pub const fn grid(self) -> BlockGrid {
        match self {
            Field::Regions => L1_GRID,
            Field::Stars => OUTER_GRID,
        }
    }

    /// Blocks kept resident: enough for [`RESIDENT_MARKERS`].
    #[must_use]
    pub fn resident_blocks(self) -> usize {
        let slots = self.grid().slots.max(1);
        usize::try_from(RESIDENT_MARKERS.div_ceil(slots)).unwrap_or(1)
    }

    fn block(self, seed: u64, key: BlockKey) -> Vec<(u32, Point)> {
        match self {
            Field::Regions => block(&Regions, &L1_GRID, seed, key),
            Field::Stars => block(&OuterStars, &OUTER_GRID, seed, key),
        }
    }

    /// Marker `id` of this field in the cell seeded `seed`.
    #[must_use]
    pub fn marker(self, seed: u64, id: u32) -> Option<Point> {
        match self {
            Field::Regions => crate::regions_l1::marker(seed, id),
            Field::Stars => crate::stars_l5::outer_marker(seed, id),
        }
    }
}

/// One live cell's resident blocks.
#[derive(Clone, Debug, Default, PartialEq)]
struct Layer {
    /// Chain of the cell, to notice a different cell.
    chain: Vec<u32>,
    /// The cell's fixture (its markers before any block).
    fixture: Vec<Point>,
    fixture_ids: Vec<u32>,
    /// Resident blocks, in linear block order, with their markers.
    blocks: Vec<(BlockKey, Vec<(u32, Point)>)>,
    /// Camera of the last refresh, in this cell's units.
    camera: Option<[f64; 3]>,
    /// Load radius of the last refresh, in this cell's units.
    radius: f64,
    /// Scratch: block distances (reused, `E-HOT-NOALLOC`).
    distances: Vec<f64>,
}

/// Resident blocks of the live cells.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Streamed {
    layers: [Layer; 3],
    revision: u64,
    /// Open-cell camera of the last [`crate::nav::Universe::stream_blocks`].
    last_offset: Option<[f64; 3]>,
}

/// Distance of the `k`-th nearest block to `camera` (the load radius that
/// keeps `k` blocks resident).
fn load_radius(grid: &BlockGrid, camera: [f64; 3], k: usize, scratch: &mut Vec<f64>) -> f64 {
    scratch.clear();
    let n = grid.per_axis;
    for z in 0..n {
        for y in 0..n {
            for x in 0..n {
                scratch.push(grid.distance_to(BlockKey([x, y, z]), camera));
            }
        }
    }
    let index = k.clamp(1, scratch.len().max(1)) - 1;
    if scratch.is_empty() {
        return 0.0;
    }
    let (_, kth, _) = scratch.select_nth_unstable_by(index, f64::total_cmp);
    *kth
}

/// Inner edge of the fade at the edge of the resident blocks, as a share
/// of the load radius: a streamed marker shows fully inside it and fades
/// to nothing at the load radius, so blocks come and go unseen.
pub const FADE_INNER: f64 = 0.7;

impl Streamed {
    /// Weight of marker `id` at `at` (cell units) of live cell `slot` (0
    /// open, 1 parent, 2 grandparent): 1 for a fixture marker or a cell
    /// that does not stream, fading from 1 at [`FADE_INNER`] of the load
    /// radius to 0 at it (#1104 T7, Spec v2 Goal 4).
    #[must_use]
    pub fn fade(&self, slot: usize, id: u32, at: [f64; 3]) -> f64 {
        let Some(layer) = self.layers.get(slot) else {
            return 1.0;
        };
        let (Some(camera), Some(field)) = (layer.camera, Field::for_chain(&layer.chain)) else {
            return 1.0;
        };
        if id < field.grid().base || layer.radius <= 0.0 {
            return 1.0;
        }
        let d = [at[0] - camera[0], at[1] - camera[1], at[2] - camera[2]];
        let distance = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let inner = layer.radius * FADE_INNER;
        if distance <= inner {
            return 1.0;
        }
        let t = ((layer.radius - distance) / (layer.radius - inner)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// The fixture of live cell `slot` and its IDs, when it streams.
    #[must_use]
    pub fn fixture(&self, slot: usize) -> Option<(&[Point], &[u32])> {
        let layer = self.layers.get(slot)?;
        layer.camera?;
        Some((&layer.fixture, &layer.fixture_ids))
    }

    /// Content revision: bumped whenever a live cell's markers change.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Forgets every layer (the live cells were regenerated).
    pub fn reset(&mut self) {
        for layer in &mut self.layers {
            *layer = Layer::default();
        }
        self.last_offset = None;
        self.revision = self.revision.wrapping_add(1);
    }

    /// Brings the blocks of live cell `slot` (0 open, 1 parent, 2
    /// grandparent) up to date for the camera at `camera` in that cell's
    /// units; `true` when its markers changed. `cell` must be the cell
    /// [`crate::nest::generate_cell`] made for `chain`, or the one this
    /// call last filled.
    pub fn refresh(
        &mut self,
        slot: usize,
        chain: &[u32],
        seed: u64,
        camera: [f64; 3],
        cell: &mut Generated,
    ) -> bool {
        let Some(layer) = self.layers.get_mut(slot) else {
            return false;
        };
        let Some(field) = Field::for_chain(chain) else {
            if !layer.chain.is_empty() || layer.camera.is_some() {
                *layer = Layer::default();
            }
            return false;
        };
        if layer.camera.is_none() || layer.chain != chain {
            *layer = Layer {
                chain: chain.to_vec(),
                fixture: cell.points.clone(),
                fixture_ids: (0..cell.points.len()).map(|i| cell.id_at(i)).collect(),
                ..Layer::default()
            };
        } else if layer.camera == Some(camera) {
            return false;
        }
        layer.camera = Some(camera);
        let grid = field.grid();
        let radius = load_radius(&grid, camera, field.resident_blocks(), &mut layer.distances);
        layer.radius = radius;
        let held: Vec<BlockKey> = layer.blocks.iter().map(|(key, _)| *key).collect();
        // In ID order (linear block index), so the cell's IDs ascend and
        // `Generated::index_of` can search them by halves.
        let mut keys = resident(&grid, camera, radius, &held);
        keys.sort_unstable_by_key(|key| grid.linear(*key));
        if keys == held && !held.is_empty() {
            return false;
        }
        let mut blocks: Vec<(BlockKey, Vec<(u32, Point)>)> = Vec::with_capacity(keys.len());
        let mut old = std::mem::take(&mut layer.blocks).into_iter().peekable();
        for key in keys {
            while old
                .peek()
                .is_some_and(|(held, _)| grid.linear(*held) < grid.linear(key))
            {
                old.next();
            }
            match old.peek() {
                Some((held, _)) if *held == key => {
                    if let Some(kept) = old.next() {
                        blocks.push(kept);
                    }
                }
                _ => blocks.push((key, field.block(seed, key))),
            }
        }
        layer.blocks = blocks;
        let extra: usize = layer.blocks.iter().map(|(_, markers)| markers.len()).sum();
        cell.points.clear();
        cell.ids.clear();
        cell.points.reserve(layer.fixture.len() + extra);
        cell.ids.reserve(layer.fixture.len() + extra);
        cell.points.extend(layer.fixture.iter().copied());
        cell.ids.extend(layer.fixture_ids.iter().copied());
        for (_, markers) in &layer.blocks {
            for (id, point) in markers {
                cell.ids.push(*id);
                cell.points.push(*point);
            }
        }
        self.revision = self.revision.wrapping_add(1);
        true
    }
}

/// Camera position in the units of the child cell behind `marker` of the
/// open cell, if the marker is resident.
#[must_use]
pub fn child_camera(universe: &crate::nav::Universe, marker: u32) -> Option<[f64; 3]> {
    let at = universe.open.point_by_id(marker)?.portal_position();
    let ratio = crate::nest::child_ratio(universe.level())?;
    let offset = universe.path.offset();
    Some([
        (offset[0] - at[0]) / ratio,
        (offset[1] - at[1]) / ratio,
        (offset[2] - at[2]) / ratio,
    ])
}

/// The cell behind `chain` as a preview shows it: its fixture plus, for a
/// streamed cell, the blocks near `camera` in its units (the same rule the
/// open cell follows, so entering it changes only blocks at the edge).
#[must_use]
pub fn preview_cell(root: u64, chain: &[u32], camera: Option<[f64; 3]>) -> Generated {
    let mut cell = crate::nest::generate_cell(root, chain);
    if let Some(camera) = camera
        && Field::for_chain(chain).is_some()
    {
        let seed = crate::nest::path_seed(root, chain);
        let mut streamed = Streamed::default();
        streamed.refresh(0, chain, seed, camera, &mut cell);
    }
    cell
}

/// Whether a live cell (open, parent or grandparent) streams with the open
/// cell at `level`: L1 and L5 and the two levels below each.
const fn streams_near(level: u8) -> bool {
    matches!(level, 1..=3 | 5..=7)
}

impl crate::nav::Universe {
    /// Brings the blocks near the camera up to date in every live cell
    /// that streams (#1104 T6); `true` when any cell's markers changed.
    /// Cheap when the camera is still: nothing is computed.
    pub fn stream_blocks(&mut self) -> bool {
        // A still camera, or a level whose live cells never stream, does
        // nothing (no allocation per frame, `E-HOT-NOALLOC`).
        let offset = self.path.offset();
        if self.streamed.last_offset == Some(offset) || !streams_near(self.level().get()) {
            return false;
        }
        self.streamed.last_offset = Some(offset);
        let root = self.root.0;
        let mut chain = self.path.indices();
        let named: Vec<crate::nest::Opened> = self
            .path
            .chain()
            .iter()
            .filter(|opened| !opened.anonymous)
            .copied()
            .collect();
        let mut camera = self.path.offset();
        let mut changed = false;
        let seed = crate::nest::path_seed(root, &chain);
        changed |= self
            .streamed
            .refresh(0, &chain, seed, camera, &mut self.open);
        for (slot, opened) in named.iter().rev().take(2).enumerate() {
            if chain.pop().is_none() {
                break;
            }
            camera = opened.to_parent_point(camera);
            let seed = crate::nest::path_seed(root, &chain);
            let cell = if slot == 0 {
                self.parent.as_mut()
            } else {
                self.grandparent.as_mut()
            };
            if let Some(cell) = cell {
                changed |= self.streamed.refresh(slot + 1, &chain, seed, camera, cell);
            }
        }
        changed
    }

    /// [`Streamed::fade`] of marker `id` at `at` in live cell `slot`.
    #[must_use]
    pub fn stream_fade(&self, slot: usize, id: u32, at: [f64; 3]) -> f64 {
        self.streamed.fade(slot, id, at)
    }

    /// Content revision of the live cells: caches keyed by the open seed
    /// also compare this (#1104 T6).
    #[must_use]
    pub const fn content_revision(&self) -> u64 {
        self.streamed.revision()
    }
}

#[cfg(test)]
#[path = "streamed_tests.rs"]
mod tests;
