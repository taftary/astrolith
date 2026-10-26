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
//! both regenerate cells through [`generate_cell`](crate::nest::generate_cell) and agree byte for byte.

use crate::coords::{HALF_BOUND, Level, MAX_LEVEL};
use crate::density::DensityGenerator;
pub use crate::frame::Opened;
use crate::frame::{IDENTITY_UP, length};
use crate::r#gen::{Constraints, Generated, Generator, OctantGenerator, UniformGenerator};
pub use crate::home::{autopilot_candidates, autopilot_marker, autopilot_path, marker_position};
use crate::home::{
    is_home_l4_cell, is_home_planet_cell, is_home_system_cell, is_rich_cluster_cell,
};
use crate::seed::hash_cell;
use crate::sysgen::{GalaxyGenerator, RICH_CLUSTER_TOTAL};
use crate::terrain::TerrainSampler;

/// True order of magnitude `e_l = log10(S_l)` per rung (R5 anchors).
///
/// Index `l - 1`. Values and sources: `docs/universes/ladder.md`, R5
/// amendment for L1-L10. L11-L14 removed #770 (R11 tail retired).
/// The L2/L3 merge of #151 retired the 24.69 rung (old L3).
pub const LADDER_EXPONENTS: [f64; MAX_LEVEL as usize] = [
    26.94, 25.11, 23.15, 20.98, 18.49, 16.62, 16.17, 13.25, 9.14, 7.11,
];

/// Deepest level a marker can open into (L10, planet; #729).
pub const MAX_OPEN_LEVEL: u8 = 10;

/// Angular radius (radians) at which a targeted marker opens (`theta_min`).
///
/// About a third of the 45-degree view height (owner decision, R6).
pub const OPEN_ANGLE: f64 = 0.14;

/// Angular radius at which a planet's surface starts fading in before its
/// cell opens: half of [`OPEN_ANGLE`] (#603, Q2 A).
pub const APPROACH_START: f64 = OPEN_ANGLE * 0.5;

/// Strength of a planet surface seen at `angular` radius (#603, Q2 A).
///
/// `0.0` up to [`APPROACH_START`], `1.0` from [`OPEN_ANGLE`] (where the
/// planet cell opens), a smooth step between. It reads the same angular
/// radius as the open rule, so full strength is exactly the opening, and it
/// follows the camera both ways. Non-finite input reads as `0.0`.
#[must_use]
pub fn approach_fade(angular: f64) -> f64 {
    if !angular.is_finite() {
        return 0.0;
    }
    let t = ((angular - APPROACH_START) / (OPEN_ANGLE - APPROACH_START)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Angular radius below which the open cell closes back into its marker.
///
/// Well under [`OPEN_ANGLE`] so entry has room to breathe: a dive enters
/// at about 3.58 cell units and closes past 10.0, about three and a half
/// wheel notches apart, so small moves after entering never close the
/// cell on their own (#403, owner-approved R6 note in
/// `docs/universes/ladder.md`; was 0.10, one notch of slack).
pub const CLOSE_ANGLE: f64 = 0.05;

/// Angular radius above which a marker previews its interior (R7, #63).
///
/// Seven times below [`OPEN_ANGLE`]: a marker starts resolving into its
/// children long before you enter it, so entry changes nothing on screen.
pub const PREVIEW_ANGLE: f64 = 0.02;

/// Angular radius above which a marker draws its form instead of its dot
/// impostor (#384).
///
/// Between this and [`PREVIEW_ANGLE`] a marker shows its body at its own
/// size; below it a brightness-sized dot. Set at 0.006 rad (about 8 px at
/// 1080p), a third of the way up the preview slope, so the four stages
/// (dot, form, preview, open) resolve in order with no pop.
pub const FORM_ANGLE: f64 = 0.006;

/// Most markers previewed at once (the largest on screen win).
pub const PREVIEW_CAP: usize = 6;

/// Magnification milestones crossed when opening out of `level` (#151 T4, R11).
///
/// Parent-to-child spans over 1.5 decades are crossed through invisible
/// cells (Spec v1 §"What gets built" item 3): L1-L2 through L5-L6 take two,
/// L7-L8 two, L8-L9 three, L9-L10 two, L11-L12 two, L12-L13 two; short spans
/// (L6-L7, L10-L11, L13-L14) and the terminal level take none. Milestones
/// are silent exact no-ops; only the dive uses them, and generation never
/// sees them.
#[must_use]
pub const fn anon_cells(level: Level) -> usize {
    match level.get() {
        1..=5 => 2,
        6 => 0,
        7 => 2,
        8 => 3,
        9 => 2,
        10 => 0,
        11 => 2,
        12 => 2,
        13 => 0,
        _ => 0,
    }
}

/// Shell brightness kept at the open angle: a faint boundary remains.
pub const SHELL_FLOOR: f64 = 0.15;

/// Fewest markers any cell emits.
pub const MIN_MARKERS: u32 = 4;

/// Fraction of a cell's volume its child cells may fill before the count is
/// capped; keeps closely sized rungs (L2/L3, L7/L8) from overlapping wildly.
const PACKING_FRACTION: f64 = 0.3;

/// Returns `e_l` for `level`.
#[must_use]
pub fn scale_exponent(level: Level) -> f64 {
    #[expect(
        clippy::indexing_slicing,
        reason = "E-NO-PANIC: level is 1..=14, index 0..=13 into 14 entries"
    )]
    let exponent = LADDER_EXPONENTS[usize::from(level.get() - 1)];
    exponent
}

/// Size ratio of a child cell to its parent: `10^(e_(l+1) - e_l)`.
///
/// Always in `(0, 1)`; `None` at L14 (no deeper rung).
#[must_use]
pub fn child_ratio(level: Level) -> Option<f64> {
    let deeper = level.deeper()?;
    Some(10f64.powf(scale_exponent(deeper) - scale_exponent(level)))
}

/// Marker radius in units of the open cell: half the child cell's size.
///
/// Returns `None` at L14.
#[must_use]
pub fn marker_radius(level: Level) -> Option<f64> {
    child_ratio(level).map(|ratio| ratio * HALF_BOUND)
}

/// Baseline marker count per content era (M3 clusters, M4 stars, M5 terrain,
/// M6 tail).
#[must_use]
pub const fn base_count(level: Level) -> u32 {
    match level.get() {
        1..=3 => 48,
        4..=9 => 32,
        10 => 30,
        11..=13 => 8,
        14 => 12,
        _ => 16,
    }
}

/// Budget for a cell at `level`: full density, count capped by true packing.
///
/// A cell can hold at most `PACKING_FRACTION / ratio^3` children before they
/// overlap, so the base count is capped there (floor of [`MIN_MARKERS`]).
/// Valid by construction.
#[must_use]
pub fn level_budget(level: Level) -> Constraints {
    let packing_cap = child_ratio(level)
        .map(|ratio| (PACKING_FRACTION / ratio.powi(3)).floor())
        .filter(|cap| cap.is_finite())
        .map_or(u32::MAX, |cap| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: cap clamped to u32::MAX, `as` saturates identically"
            )]
            #[expect(
                clippy::cast_sign_loss,
                reason = "E-CAST: cap clamped to u32::MAX, `as` saturates identically"
            )]
            let capped = cap.min(f64::from(u32::MAX)) as u32;
            capped
        });
    let max_count = if level.get() == 4 {
        // Virgo-rich fixture cap (#151): rich cluster cells hold thousands.
        RICH_CLUSTER_TOTAL
    } else {
        base_count(level).min(packing_cap).max(MIN_MARKERS)
    };
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
#[must_use]
pub fn root_cell_seed(root: u64) -> u64 {
    hash_cell(root, 1, 0, 0, 0)
}

/// Seed of the cell inside marker `marker` of a cell seeded `cell_seed`.
///
/// `child_level` is the level of the marker's interior. The ladder rule
/// `H(parent, level, i, j, k)` with `(i, j, k) = (marker, 0, 0)`.
#[must_use]
pub fn marker_seed(cell_seed: u64, child_level: Level, marker: u32) -> u64 {
    hash_cell(cell_seed, child_level.get(), i64::from(marker), 0, 0)
}

/// Seed of the cell reached by following `chain` from the root.
///
/// `chain[i]` is the marker opened at level `i + 1`, so the result is the
/// seed of the level `chain.len() + 1` cell. Depends only on the values, not
/// on visit order.
#[must_use]
pub fn path_seed(root: u64, chain: &[u32]) -> u64 {
    chain
        .iter()
        .enumerate()
        .fold(root_cell_seed(root), |seed, (depth, &marker)| {
            #[expect(clippy::cast_possible_truncation, reason = "E-CAST: chain depth is a journey length; Level::new falls back to MAX when out of range")]
            let child_level = Level::new(depth as u8 + 2).unwrap_or(Level::MAX);
            marker_seed(seed, child_level, marker)
        })
}

/// Level of the cell reached by `chain` (its length plus one), clamped to L14.
#[must_use]
pub fn path_level(chain: &[u32]) -> Level {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: min'd with MAX_LEVEL (14), always fits u8"
    )]
    let value = (chain.len() + 1).min(usize::from(MAX_LEVEL)) as u8;
    Level::new(value).unwrap_or(Level::MAX)
}

/// Era generator for a cell at `level`.
///
/// M3 density field (L1-L3), M4 galaxies (L4-L9), M5 terrain (L9 interiors,
/// L10). Below L10 nothing opens (#729, #742), so deeper chains fall
/// through to the uniform scatter. L1 subdivides into 8 octant portals.
#[derive(Debug)]
pub enum LevelGenerator {
    /// L1 octant portals from the fixed octant generator.
    Octant(OctantGenerator),
    /// L2-L3 cluster points from the M3 density field.
    Density(DensityGenerator),
    /// L4-L9 star points from the M4 galaxy generator.
    Galaxy(GalaxyGenerator),
    /// L9-interior and L10 heightmap points from the M5 terrain sampler
    /// (plus the L10 region portals since #375).
    Terrain(TerrainSampler),
    /// Uniform scatter for levels outside 1..=14 (defensive: the ladder is
    /// fully covered above).
    Uniform(UniformGenerator),
}

impl LevelGenerator {
    /// Builds the era generator for the cell reached by `chain`.
    ///
    /// The density field is anchored to the last marker index so sibling
    /// cells read distinct lattice offsets. Richness (L4 Virgo-analogs) is
    /// decided here because only this point sees both `root` and `chain`;
    /// L9 cells entered through a non-star marker show planet-interior
    /// (terrain) content instead of the star close-up.
    #[must_use]
    pub fn for_path(root: u64, chain: &[u32]) -> LevelGenerator {
        if chain.is_empty() {
            return LevelGenerator::Octant(OctantGenerator);
        }
        let level = path_level(chain);
        match level.get() {
            1..=3 => {
                let anchor = chain.last().map_or(0, |&m| i64::from(m));
                LevelGenerator::Density(DensityGenerator::new(level, anchor, 0, 0))
            }
            4..=8 => {
                let rich = is_rich_cluster_cell(root, chain);
                let home = is_home_l4_cell(root, chain);
                let system_home = is_home_system_cell(root, chain);
                LevelGenerator::Galaxy(GalaxyGenerator::new(level, rich, home, system_home))
            }
            9 => {
                if chain.last() == Some(&0) {
                    let system_home = is_home_system_cell(root, chain);
                    LevelGenerator::Galaxy(GalaxyGenerator::new(level, false, false, system_home))
                } else {
                    LevelGenerator::Terrain(TerrainSampler::new())
                }
            }
            10 => {
                let home = is_home_planet_cell(root, chain);
                // Region portals sit on the one icosphere's anchor tiles
                // (ADR 0024), the same way as in L9 interiors (#543).
                let sampler = if home {
                    TerrainSampler::home()
                } else {
                    TerrainSampler::new()
                };
                // Sized as its system lists it (#995 T6).
                LevelGenerator::Terrain(
                    sampler.with_radius(crate::terrain::planet_radius_earth(root, chain)),
                )
            }
            11..=14 => LevelGenerator::Uniform(UniformGenerator::new(base_count(level))),
            _ => LevelGenerator::Uniform(UniformGenerator::new(base_count(level))),
        }
    }
}

impl Generator for LevelGenerator {
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        match self {
            LevelGenerator::Octant(generator) => generator.generate(seed, parent),
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
#[must_use]
pub fn generate_cell(root: u64, chain: &[u32]) -> Generated {
    let level = path_level(chain);
    LevelGenerator::for_path(root, chain).generate(path_seed(root, chain), &level_budget(level))
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
    #[must_use]
    pub fn root(offset: [f64; 3]) -> MarkerPath {
        MarkerPath {
            chain: Vec::new(),
            offset: sanitize(offset),
        }
    }

    /// Level of the open cell.
    #[must_use]
    pub fn level(&self) -> Level {
        path_level(&self.indices())
    }

    /// Marker indices from L1 down to the open cell.
    ///
    /// Skips magnification milestones: generation, seeds, snapshots, and
    /// previews only ever see named opens.
    #[must_use]
    pub fn indices(&self) -> Vec<u32> {
        self.chain
            .iter()
            .filter(|opened| !opened.anonymous)
            .map(|opened| opened.marker)
            .collect()
    }

    /// Whether no named cell is open (at the root, milestones aside).
    #[must_use]
    pub fn is_at_root(&self) -> bool {
        !self.chain.iter().any(|opened| !opened.anonymous)
    }

    /// Consecutive magnification milestones on top of the stack.
    #[must_use]
    pub fn top_anon_run(&self) -> usize {
        self.chain
            .iter()
            .rev()
            .take_while(|opened| opened.anonymous)
            .count()
    }

    /// All magnification milestones in the stack (journey diagnostics).
    #[must_use]
    pub fn anonymous_depth(&self) -> usize {
        self.chain.iter().filter(|opened| opened.anonymous).count()
    }

    /// Marker of the top milestone, if the stack top is one.
    #[must_use]
    pub fn top_anon_marker(&self) -> Option<u32> {
        self.chain
            .last()
            .filter(|opened| opened.anonymous)
            .map(|opened| opened.marker)
    }

    /// Pops top milestones left by another target (silent exact no-ops).
    pub fn clear_foreign_milestones(&mut self, target: u32) {
        while self
            .chain
            .last()
            .is_some_and(|opened| opened.anonymous && opened.marker != target)
        {
            self.close();
        }
    }

    /// Opened markers from L1 down to the open cell.
    #[must_use]
    pub fn chain(&self) -> &[Opened] {
        &self.chain
    }

    /// Marker that was opened to reach the current cell, if any.
    ///
    /// Skips magnification milestones: siblings, retargeting, and previews
    /// only ever see named opens.
    #[must_use]
    pub fn entered(&self) -> Option<Opened> {
        self.chain
            .iter()
            .rev()
            .find(|opened| !opened.anonymous)
            .copied()
    }

    /// Camera position in units of the open cell.
    #[must_use]
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
    #[must_use]
    pub fn distance_to_center(&self) -> f64 {
        length(self.offset)
    }

    /// Returns `true` when the open cell's markers can be opened.
    #[must_use]
    pub fn can_open(&self) -> bool {
        self.level().get() < MAX_OPEN_LEVEL
    }

    /// Descends into `marker` at `position` (open-cell units).
    ///
    /// The camera offset is re-expressed in child units, so the world shifts
    /// origin without moving the camera. Returns `false`, untouched, past
    /// [`MAX_OPEN_LEVEL`] or for a non-finite position.
    pub fn open(&mut self, marker: u32, position: [f64; 3]) -> bool {
        self.open_oriented(marker, position, IDENTITY_UP)
    }

    /// Descends into `marker` at `position` with child `up` in parent units.
    ///
    /// Identity `up` behaves exactly like [`MarkerPath::open`]; a patch
    /// normal `up` lands the region's ground tangent to the planet (#394).
    /// Returns `false`, untouched, past [`MAX_OPEN_LEVEL`], for a non-finite
    /// position, or for a zero ratio.
    pub fn open_oriented(&mut self, marker: u32, position: [f64; 3], up: [f64; 3]) -> bool {
        if !self.can_open() || position.iter().any(|component| !component.is_finite()) {
            return false;
        }
        let Some(ratio) = child_ratio(self.level()) else {
            return false;
        };
        if !ratio.is_finite() || ratio <= 0.0 {
            return false;
        }
        let opened = Opened {
            marker,
            position,
            ratio,
            anonymous: false,
            up,
            spin: 0.0,
            axis: IDENTITY_UP,
        };
        self.offset = opened.to_child_point(self.offset);
        self.chain.push(opened);
        true
    }

    /// Pushes a magnification milestone for `marker` (#151 T4).
    ///
    /// Carries no frame state (zero position, unit ratio), so it never
    /// moves the camera and unwinds exactly; generation never sees it.
    /// Only [`Universe::dive`](crate::nav::Universe) calls this, silently.
    pub fn open_anonymous(&mut self, marker: u32) {
        self.chain.push(Opened {
            marker,
            position: [0.0; 3],
            ratio: 1.0,
            anonymous: true,
            up: IDENTITY_UP,
            spin: 0.0,
            axis: IDENTITY_UP,
        });
    }

    /// Ascends out of the open cell back into the marker it came from.
    ///
    /// Exact inverse of [`MarkerPath::open`] and [`MarkerPath::open_oriented`].
    /// Returns the marker closed, or `None` at the root.
    pub fn close(&mut self) -> Option<Opened> {
        let opened = self.chain.pop()?;
        self.offset = opened.to_parent_point(self.offset);
        Some(opened)
    }

    /// Moves the open cell's entry to `position` in parent units, keeping
    /// the camera's offset inside it (#981: a moving planet carries the
    /// camera along). A milestone or the root is left untouched.
    pub fn move_entry(&mut self, position: [f64; 3]) {
        if let Some(entry) = self.chain.last_mut()
            && !entry.anonymous
            && position.iter().all(|c| c.is_finite())
        {
            entry.position = position;
        }
    }

    /// Turns the open cell's entry to `spin` radians about the unit `axis`
    /// (#981 T3). With `keep_place` the camera keeps its place in the
    /// parent (a fresh open at a later time); without it, it keeps its
    /// place in the child and turns with the planet. A milestone or the
    /// root is left untouched.
    pub fn spin_entry(&mut self, spin: f64, axis: [f64; 3], keep_place: bool) {
        if let Some(entry) = self.chain.last_mut()
            && !entry.anonymous
            && spin.is_finite()
            && axis.iter().all(|c| c.is_finite())
        {
            if keep_place {
                // Back to the parent through the old turn, then into the
                // new one.
                let parent = crate::frame::turn_about(self.offset, entry.axis, entry.spin);
                self.offset = crate::frame::turn_about(parent, axis, -spin);
            }
            entry.spin = spin;
            entry.axis = axis;
        }
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

#[cfg(test)]
#[path = "nest_tests.rs"]
mod tests;
