//! Marker-tree nesting: every marker is the next level's cell (R6, #58).
//!
//! The notion (section 5.1) names a place as a chain of integer cell indices
//! plus a float offset in the deepest cell. [`crate::coords::Frame`] realises
//! that chain with octants (the anonymous gap-filler cells of section 5.4).
//! This module realises it with **markers**: a cell at level `l` holds `N`
//! marker points, and marker `m` *is* the level `l+1` cell, sized by the true
//! ratio `10^(e_(l+1) - e_l)` of the frozen ladder anchors. A marker index is
//! an integer cell index, so the notion's `P` is unchanged.
//!
//! Everything here is pure and headless: the window and the `--verify` mode
//! both regenerate cells through [`generate_cell`] and agree byte for byte.

use crate::astro::GalaxyGenerator;
use crate::coords::{HALF_BOUND, Level, MAX_LEVEL};
use crate::density::DensityGenerator;
use crate::r#gen::{Constraints, Generated, Generator, UniformGenerator};
use crate::seed::hash_cell;
use crate::terrain::TerrainSampler;

/// True order of magnitude `e_l = log10(S_l)` per rung (R5 anchors).
///
/// Index `l - 1`. Values and sources: `docs/universe/ladder.md`, R5
/// amendment. L12-L15 keep the notion range midpoints (beyond MVP).
pub const LADDER_EXPONENTS: [f64; MAX_LEVEL as usize] = [
    26.94, 25.11, 24.69, 23.15, 20.98, 18.49, 16.62, 16.17, 13.25, 9.14, 7.11, 5.5, 4.0, 1.5, 0.5,
];

/// Deepest level a marker can open into (L11, planets; M5 scope).
pub const MAX_OPEN_LEVEL: u8 = 11;

/// Fewest markers any cell emits.
pub const MIN_MARKERS: u32 = 4;

/// Fraction of a cell's volume its child cells may fill before the count is
/// capped; keeps closely sized rungs (L2/L3, L7/L8) from overlapping wildly.
const PACKING_FRACTION: f64 = 0.3;

/// Returns `e_l` for `level`.
pub fn scale_exponent(level: Level) -> f64 {
    LADDER_EXPONENTS[usize::from(level.get() - 1)]
}

/// Size ratio of a child cell to its parent: `10^(e_(l+1) - e_l)`.
///
/// Always in `(0, 1)`; `None` at L15 (no deeper rung).
pub fn child_ratio(level: Level) -> Option<f64> {
    let deeper = level.deeper()?;
    Some(10f64.powf(scale_exponent(deeper) - scale_exponent(level)))
}

/// Marker radius in units of the open cell: half the child cell's size.
///
/// Returns `None` at L15.
pub fn marker_radius(level: Level) -> Option<f64> {
    child_ratio(level).map(|ratio| ratio * HALF_BOUND)
}

/// Baseline marker count per content era (M3 clusters, M4 stars, M5 terrain).
pub const fn base_count(level: Level) -> u32 {
    match level.get() {
        1..=4 => 48,
        5..=10 => 32,
        11 => 24,
        _ => 16,
    }
}

/// Budget for a cell at `level`: full density, count capped by true packing.
///
/// A cell can hold at most `PACKING_FRACTION / ratio^3` children before they
/// overlap, so the base count is capped there (floor of [`MIN_MARKERS`]).
/// Valid by construction.
pub fn level_budget(level: Level) -> Constraints {
    let packing_cap = child_ratio(level)
        .map(|ratio| (PACKING_FRACTION / ratio.powi(3)).floor())
        .filter(|cap| cap.is_finite())
        .map_or(u32::MAX, |cap| cap.min(f64::from(u32::MAX)) as u32);
    let max_count = base_count(level).min(packing_cap).max(MIN_MARKERS);
    let constraints = Constraints {
        density_multiplier: 1.0,
        min_count: MIN_MARKERS.min(max_count),
        max_count,
        allowed_extent: [HALF_BOUND; 3],
    };
    debug_assert!(constraints.is_valid());
    constraints
}

/// Seed of the L1 root cell under `root`.
pub fn root_cell_seed(root: u64) -> u64 {
    hash_cell(root, 1, 0, 0, 0)
}

/// Seed of the cell inside marker `marker` of a cell seeded `cell_seed`.
///
/// `child_level` is the level of the marker's interior. The ladder rule
/// `H(parent, level, i, j, k)` with `(i, j, k) = (marker, 0, 0)`.
pub fn marker_seed(cell_seed: u64, child_level: Level, marker: u32) -> u64 {
    hash_cell(cell_seed, child_level.get(), i64::from(marker), 0, 0)
}

/// Seed of the cell reached by following `chain` from the root.
///
/// `chain[i]` is the marker opened at level `i + 1`, so the result is the
/// seed of the level `chain.len() + 1` cell. Depends only on the values, not
/// on visit order.
pub fn path_seed(root: u64, chain: &[u32]) -> u64 {
    chain
        .iter()
        .enumerate()
        .fold(root_cell_seed(root), |seed, (depth, &marker)| {
            let child_level = Level::new(depth as u8 + 2).unwrap_or(Level::MAX);
            marker_seed(seed, child_level, marker)
        })
}

/// Level of the cell reached by `chain` (its length plus one), clamped to L15.
pub fn path_level(chain: &[u32]) -> Level {
    Level::new((chain.len() + 1).min(usize::from(MAX_LEVEL)) as u8).unwrap_or(Level::MAX)
}

/// Era generator for a cell at `level`.
///
/// M3 density field (L1-L4), M4 galaxies (L5-L10), M5 terrain (L11), uniform
/// scatter beyond the MVP scope.
#[derive(Debug)]
pub enum LevelGenerator {
    /// L1-L4 cluster points from the M3 density field.
    Density(DensityGenerator),
    /// L5-L10 star points from the M4 galaxy generator.
    Galaxy(GalaxyGenerator),
    /// L11 heightmap points from the M5 terrain sampler.
    Terrain(TerrainSampler),
    /// Uniform scatter for levels past the MVP scope.
    Uniform(UniformGenerator),
}

impl LevelGenerator {
    /// Builds the era generator for the cell reached by `chain`.
    ///
    /// The density field is anchored to the last marker index so sibling
    /// cells read distinct lattice offsets.
    pub fn for_path(chain: &[u32]) -> LevelGenerator {
        let level = path_level(chain);
        match level.get() {
            1..=4 => {
                let anchor = chain.last().map_or(0, |&m| i64::from(m));
                LevelGenerator::Density(DensityGenerator::new(level, anchor, 0, 0))
            }
            5..=10 => LevelGenerator::Galaxy(GalaxyGenerator::new(base_count(level))),
            11 => LevelGenerator::Terrain(TerrainSampler::new()),
            _ => LevelGenerator::Uniform(UniformGenerator::new(base_count(level))),
        }
    }
}

impl Generator for LevelGenerator {
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        match self {
            LevelGenerator::Density(generator) => generator.generate(seed, parent),
            LevelGenerator::Galaxy(generator) => generator.generate(seed, parent),
            LevelGenerator::Terrain(generator) => generator.generate(seed, parent),
            LevelGenerator::Uniform(generator) => generator.generate(seed, parent),
        }
    }
}

/// Generates the cell reached by `chain` under `root`.
///
/// Pure: same `(root, chain)` always yields byte-identical content. Marker
/// positions are cell-local in `[-0.5, 0.5)`; their drawn radius is
/// [`marker_radius`] of the cell's level, not the indicator radius.
pub fn generate_cell(root: u64, chain: &[u32]) -> Generated {
    let level = path_level(chain);
    LevelGenerator::for_path(chain).generate(path_seed(root, chain), &level_budget(level))
}

/// One opened marker on a [`MarkerPath`]: enough to close it exactly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opened {
    /// Marker index inside the parent cell.
    pub marker: u32,
    /// Marker position in parent-cell units.
    pub position: [f64; 3],
    /// Child-to-parent size ratio used when opening.
    pub ratio: f64,
}

/// Observer frame over the marker tree: opened markers plus a float offset.
///
/// The open cell is the render origin; `offset` is the camera position in
/// units of that cell (origin shifting, notion section 7). [`MarkerPath::open`]
/// descends into a marker and [`MarkerPath::close`] is its exact inverse, so
/// zooming out lands where you were.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkerPath {
    chain: Vec<Opened>,
    offset: [f64; 3],
}

impl MarkerPath {
    /// Starts at the L1 root cell with the camera at `offset` (root units).
    pub fn root(offset: [f64; 3]) -> MarkerPath {
        MarkerPath {
            chain: Vec::new(),
            offset: sanitize(offset),
        }
    }

    /// Level of the open cell.
    pub fn level(&self) -> Level {
        path_level(&self.indices())
    }

    /// Marker indices from L1 down to the open cell.
    pub fn indices(&self) -> Vec<u32> {
        self.chain.iter().map(|opened| opened.marker).collect()
    }

    /// Opened markers from L1 down to the open cell.
    pub fn chain(&self) -> &[Opened] {
        &self.chain
    }

    /// Marker that was opened to reach the current cell, if any.
    pub fn entered(&self) -> Option<Opened> {
        self.chain.last().copied()
    }

    /// Camera position in units of the open cell.
    pub fn offset(&self) -> [f64; 3] {
        self.offset
    }

    /// Replaces the camera position (non-finite input is ignored).
    pub fn set_offset(&mut self, offset: [f64; 3]) {
        if offset.iter().all(|component| component.is_finite()) {
            self.offset = offset;
        }
    }

    /// Distance from the camera to the open cell's center, in open units.
    pub fn distance_to_center(&self) -> f64 {
        length(self.offset)
    }

    /// Returns `true` when the open cell's markers can be opened.
    pub fn can_open(&self) -> bool {
        self.level().get() < MAX_OPEN_LEVEL
    }

    /// Descends into `marker` at `position` (open-cell units).
    ///
    /// The camera offset is re-expressed in child units, so the world shifts
    /// origin without moving the camera. Returns `false`, untouched, past
    /// [`MAX_OPEN_LEVEL`] or for a non-finite position.
    pub fn open(&mut self, marker: u32, position: [f64; 3]) -> bool {
        if !self.can_open() || position.iter().any(|component| !component.is_finite()) {
            return false;
        }
        let Some(ratio) = child_ratio(self.level()) else {
            return false;
        };
        for (component, anchor) in self.offset.iter_mut().zip(position) {
            *component = (*component - anchor) / ratio;
        }
        self.chain.push(Opened {
            marker,
            position,
            ratio,
        });
        true
    }

    /// Ascends out of the open cell back into the marker it came from.
    ///
    /// Exact inverse of [`MarkerPath::open`]. Returns the marker closed, or
    /// `None` at the root.
    pub fn close(&mut self) -> Option<Opened> {
        let opened = self.chain.pop()?;
        for (component, anchor) in self.offset.iter_mut().zip(opened.position) {
            *component = *component * opened.ratio + anchor;
        }
        Some(opened)
    }
}

/// Replaces non-finite components by zero.
fn sanitize(offset: [f64; 3]) -> [f64; 3] {
    offset.map(|component| {
        if component.is_finite() {
            component
        } else {
            0.0
        }
    })
}

/// Euclidean length of `v`.
fn length(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Angular radius (radians) of a sphere of `radius` at `distance`.
///
/// Returns `pi/2` when the camera is inside the sphere.
pub fn angular_radius(radius: f64, distance: f64) -> f64 {
    if distance <= radius {
        std::f64::consts::FRAC_PI_2
    } else {
        (radius / distance).asin()
    }
}

/// Deterministic autopilot marker for a cell: a seeded index in `0..count`.
///
/// Drawn from the cell seed, so the Spacebar journey is the same every run
/// (owner decision: fixed journey). Returns `None` for an empty cell.
pub fn autopilot_marker(cell_seed: u64, count: usize) -> Option<u32> {
    if count == 0 {
        return None;
    }
    let roll = hash_cell(cell_seed, 0, 0x4155_544f, 0, 0);
    Some((roll % count as u64) as u32)
}

/// Marker indices the autopilot opens from L1 down to [`MAX_OPEN_LEVEL`].
pub fn autopilot_path(root: u64) -> Vec<u32> {
    let mut chain = Vec::with_capacity(usize::from(MAX_OPEN_LEVEL - 1));
    while path_level(&chain).get() < MAX_OPEN_LEVEL {
        let cell = generate_cell(root, &chain);
        let Some(marker) = autopilot_marker(path_seed(root, &chain), cell.points.len()) else {
            break;
        };
        chain.push(marker);
    }
    chain
}

/// Returns the position of `marker` in `cell`, if it exists.
pub fn marker_position(cell: &Generated, marker: u32) -> Option<[f64; 3]> {
    cell.points.get(marker as usize).map(|point| point.position)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::snapshot_generated;

    fn level(n: u8) -> Level {
        Level::new(n).expect("ladder level")
    }

    #[test]
    fn ratios_follow_ladder_anchors_and_shrink() {
        for n in 1..MAX_LEVEL {
            let ratio = child_ratio(level(n)).expect("has child");
            let expected =
                10f64.powf(LADDER_EXPONENTS[usize::from(n)] - LADDER_EXPONENTS[usize::from(n - 1)]);
            assert!((ratio - expected).abs() < 1e-12, "L{n} ratio");
            assert!(
                ratio > 0.0 && ratio < 1.0,
                "L{n} ratio out of (0,1): {ratio}"
            );
        }
        assert!(child_ratio(Level::MAX).is_none());
        // Heliopause -> Sun is the widest gap: four decades.
        assert!(child_ratio(level(9)).expect("L9") < 1e-4);
        // Cosmic web -> supercluster is the narrowest: under half a decade.
        assert!(child_ratio(level(2)).expect("L2") > 0.3);
    }

    #[test]
    fn budgets_cap_by_packing_and_stay_valid() {
        for n in 1..=MAX_LEVEL {
            let budget = level_budget(level(n));
            assert!(budget.is_valid(), "L{n} budget invalid");
            assert!(budget.max_count <= base_count(level(n)).max(MIN_MARKERS));
        }
        assert!(level_budget(level(2)).max_count < 10, "L2 should be sparse");
        assert!(level_budget(level(7)).max_count < 10, "L7 should be sparse");
        assert_eq!(level_budget(level(1)).max_count, 48);
    }

    #[test]
    fn same_path_regenerates_identical_cells() {
        let chain = [3u32, 1, 7, 0, 2];
        let first = snapshot_generated(&generate_cell(42, &chain));
        let second = snapshot_generated(&generate_cell(42, &chain));
        assert_eq!(first, second);
        assert_ne!(
            first,
            snapshot_generated(&generate_cell(42, &[3, 1, 7, 0, 3]))
        );
        assert_ne!(first, snapshot_generated(&generate_cell(43, &chain)));
        assert_eq!(path_seed(42, &chain), path_seed(42, &chain));
        assert_ne!(path_seed(42, &chain), path_seed(42, &chain[..4]));
    }

    #[test]
    fn every_level_down_to_l11_has_markers() {
        let chain = autopilot_path(42);
        assert_eq!(chain.len(), usize::from(MAX_OPEN_LEVEL - 1));
        for depth in 0..=chain.len() {
            let cell = generate_cell(42, &chain[..depth]);
            let budget = level_budget(path_level(&chain[..depth]));
            assert!(
                !cell.points.is_empty() && cell.points.len() <= budget.max_count as usize,
                "depth {depth} count {}",
                cell.points.len()
            );
            for point in &cell.points {
                assert!(point.position.iter().all(|c| c.abs() <= HALF_BOUND));
            }
        }
        assert_eq!(chain, autopilot_path(42), "autopilot path must be fixed");
    }

    #[test]
    fn open_then_close_is_exact_inverse() {
        let start = [1.1, 0.7, -0.9];
        let mut path = MarkerPath::root(start);
        let chain = autopilot_path(42);
        let mut positions = Vec::new();
        for &marker in &chain {
            let cell = generate_cell(42, &path.indices());
            let position = marker_position(&cell, marker).expect("marker exists");
            positions.push(position);
            assert!(path.open(marker, position));
            assert_eq!(path.indices().last(), Some(&marker));
        }
        assert_eq!(path.level().get(), MAX_OPEN_LEVEL);
        assert!(!path.can_open());
        assert!(!path.open(0, [0.0; 3]), "L11 markers must not open");
        for expected in positions.iter().rev() {
            let closed = path.close().expect("something to close");
            assert_eq!(closed.position, *expected);
        }
        assert!(path.close().is_none());
        assert_eq!(path.level(), Level::MIN);
        for (got, want) in path.offset().iter().zip(start) {
            assert!((got - want).abs() < 1e-9, "offset drifted: {got} vs {want}");
        }
    }

    #[test]
    fn opening_keeps_the_camera_where_it_was() {
        let mut path = MarkerPath::root([0.0, 0.0, 2.0]);
        let marker = [0.1, 0.0, 0.0];
        let ratio = child_ratio(Level::MIN).expect("L1 ratio");
        assert!(path.open(0, marker));
        let offset = path.offset();
        let expected = [-0.1 / ratio, 0.0, 2.0 / ratio];
        for (got, want) in offset.iter().zip(expected) {
            assert!((got - want).abs() < 1e-9);
        }
        assert!(
            path.distance_to_center() > 1.0,
            "camera must still be outside"
        );
    }

    #[test]
    fn angular_radius_is_monotonic_and_saturates() {
        assert_eq!(angular_radius(1.0, 0.5), std::f64::consts::FRAC_PI_2);
        let far = angular_radius(0.5, 10.0);
        let near = angular_radius(0.5, 2.0);
        assert!(far < near && far > 0.0);
    }

    #[test]
    fn marker_seeds_depend_on_index_and_level() {
        let base = marker_seed(7, level(2), 0);
        assert_ne!(base, marker_seed(7, level(2), 1));
        assert_ne!(base, marker_seed(7, level(3), 0));
        assert_eq!(base, marker_seed(7, level(2), 0));
        assert!(autopilot_marker(1, 0).is_none());
        assert!(autopilot_marker(1, 5).expect("index") < 5);
    }
}
