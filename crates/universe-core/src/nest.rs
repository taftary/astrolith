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

use crate::astro::{GalaxyGenerator, RICH_CLUSTER_TOTAL};
use crate::coords::{HALF_BOUND, Level, MAX_LEVEL, ParentUnits};
use crate::density::{DensityGenerator, densest_portal_index};
use crate::r#gen::{
    Constraints, Generated, Generator, MarkerKind, OctantGenerator, Point, UniformGenerator,
};
use crate::seed::hash_cell;
use crate::terrain::TerrainSampler;

/// True order of magnitude `e_l = log10(S_l)` per rung (R5 anchors).
///
/// Index `l - 1`. Values and sources: `docs/universe/ladder.md`, R5
/// amendment. The L2/L3 merge of #151 retired the 24.69 rung (old L3);
/// L11-L14 keep the notion range midpoints (beyond MVP).
pub const LADDER_EXPONENTS: [f64; MAX_LEVEL as usize] = [
    26.94, 25.11, 23.15, 20.98, 18.49, 16.62, 16.17, 13.25, 9.14, 7.11, 5.5, 4.0, 1.5, 0.5,
];

/// Deepest level a marker can open into (L10, planets; #151 scope).
pub const MAX_OPEN_LEVEL: u8 = 10;

/// Angular radius (radians) at which a targeted marker opens (`theta_min`).
///
/// About a third of the 45-degree view height (owner decision, R6).
pub const OPEN_ANGLE: f64 = 0.14;

/// Angular radius below which the open cell closes back into its marker.
///
/// Lower than [`OPEN_ANGLE`] so one wheel notch never flickers a dimension.
pub const CLOSE_ANGLE: f64 = 0.10;

/// Angular radius above which a marker previews its interior (R7, #63).
///
/// Seven times below [`OPEN_ANGLE`]: a marker starts resolving into its
/// children long before you enter it, so entry changes nothing on screen.
pub const PREVIEW_ANGLE: f64 = 0.02;

/// Most markers previewed at once (the largest on screen win).
pub const PREVIEW_CAP: usize = 6;

/// Magnification milestones crossed when opening out of `level` (#151 T4).
///
/// Parent-to-child spans over 1.5 decades are crossed through invisible
/// cells (Spec v1 §"What gets built" item 3): L1-L2 through L5-L6 take two,
/// L7-L8 two, L8-L9 three, L9-L10 two; short spans (L6-L7) and the terminal
/// level take none. Milestones are silent exact no-ops; only the dive uses
/// them, and generation never sees them.
#[must_use]
pub const fn anon_cells(level: Level) -> usize {
    match level.get() {
        1..=5 => 2,
        6 => 0,
        7 => 2,
        8 => 3,
        9 => 2,
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

/// Baseline marker count per content era (M3 clusters, M4 stars, M5 terrain).
#[must_use]
pub const fn base_count(level: Level) -> u32 {
    match level.get() {
        1..=3 => 48,
        4..=9 => 32,
        10 => 24,
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
/// M3 density field (L1-L3), M4 galaxies (L4-L9), M5 terrain (L10), uniform
/// scatter beyond the MVP scope. L1 subdivides into 8 octant portals.
#[derive(Debug)]
pub enum LevelGenerator {
    /// L1 octant portals from the fixed octant generator.
    Octant(OctantGenerator),
    /// L2-L3 cluster points from the M3 density field.
    Density(DensityGenerator),
    /// L4-L9 star points from the M4 galaxy generator.
    Galaxy(GalaxyGenerator),
    /// L10 heightmap points from the M5 terrain sampler.
    Terrain(TerrainSampler),
    /// Uniform scatter for levels past the MVP scope.
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
                LevelGenerator::Galaxy(GalaxyGenerator::new(level, rich, home))
            }
            9 => {
                if chain.last() == Some(&0) {
                    LevelGenerator::Galaxy(GalaxyGenerator::new(level, false, false))
                } else {
                    LevelGenerator::Terrain(TerrainSampler::new())
                }
            }
            10 => LevelGenerator::Terrain(TerrainSampler::new()),
            _ => LevelGenerator::Uniform(UniformGenerator::new(base_count(level))),
        }
    }
}

/// Whether the L4 cell at `chain` is the Milky Way home cell (#154 Q5).
///
/// True exactly when `chain` (three markers: L1, L2, L3) matches the fixed
/// journey prefix under `root`: every marker equals the autopilot pick in
/// its cell. Prefix cells regenerate from strictly shorter chains, so the
/// walk always terminates (the same shape as `is_rich_cluster_cell`).
/// Every other chain is a procedural neighbour by definition.
#[must_use]
fn is_home_l4_cell(root: u64, chain: &[u32]) -> bool {
    if path_level(chain).get() != 4 || chain.len() != 3 {
        return false;
    }
    let mut prefix: Vec<u32> = Vec::with_capacity(3);
    for &marker in chain {
        let cell = generate_cell(root, &prefix);
        let level = path_level(&prefix);
        let seed = path_seed(root, &prefix);
        let candidates = autopilot_candidates(level, seed, &cell.points);
        if autopilot_marker(seed, &candidates) != Some(marker) {
            return false;
        }
        prefix.push(marker);
    }
    true
}

/// Whether the L4 cell at `chain` shows rich-cluster content (#151).
///
/// True exactly when the cell was entered through its parent's densest
/// cluster portal (the prominence rule: the anchor object of a rich family
/// is always rich, so Virgo is rich on every visit while other clusters
/// draw poor). Only L4 chains consult the parent; every other level is
/// poor by definition, so no parent is regenerated for them.
#[must_use]
fn is_rich_cluster_cell(root: u64, chain: &[u32]) -> bool {
    if path_level(chain).get() != 4 {
        return false;
    }
    let Some((&opened, parent)) = chain.split_last() else {
        return false;
    };
    let parent_cell = generate_cell(root, parent);
    let anchor = parent.last().copied().map_or(0, i64::from);
    densest_portal_index(path_seed(root, parent), anchor, &parent_cell.points) == Some(opened)
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

/// One opened marker on a [`MarkerPath`]: enough to close it exactly.
///
/// Anonymous entries are magnification milestones (#151 T4): they mark
/// span fractions crossed while approaching a targeted portal, carry no
/// frame state (`position` zero, `ratio` one, so unwinds are exact
/// no-ops), take no label, and never enter generation chains, snapshots,
/// or previews. Only the dive pushes them, silently.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opened {
    /// Marker index inside the parent cell.
    pub marker: u32,
    /// Marker position in parent-cell units (zero for milestones).
    pub position: [f64; 3],
    /// Child-to-parent size ratio used when opening (one for milestones).
    pub ratio: f64,
    /// Whether this entry is a magnification milestone rather than an open.
    pub anonymous: bool,
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
            anonymous: false,
        });
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
        });
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
pub(crate) fn length(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Angular radius (radians) of a sphere of `radius` at `distance`.
///
/// Returns `pi/2` when the camera is inside the sphere.
#[must_use]
pub fn angular_radius(radius: f64, distance: f64) -> f64 {
    if distance <= radius {
        std::f64::consts::FRAC_PI_2
    } else {
        (radius / distance).asin()
    }
}

/// World position (parent-cell units) of a child drawn inside a marker.
///
/// `marker` is the marker position, `ratio` the child/parent size ratio,
/// `child_local` the child's position in child-cell units (passed straight
/// from cell storage; no arithmetic is done on it outside this function).
/// This is the exact inverse of the offset map in [`MarkerPath::open`], so
/// a child drawn before opening sits where the open cell's marker appears
/// after opening.
#[must_use]
pub fn child_world_position(marker: ParentUnits, ratio: f64, child_local: [f64; 3]) -> ParentUnits {
    let mut out = [0.0; 3];
    for ((slot, &anchor), &local) in out.iter_mut().zip(marker.0.iter()).zip(child_local.iter()) {
        *slot = anchor + local * ratio;
    }
    ParentUnits(out)
}

/// Smoothstep of `x` between `edge0` and `edge1`, clamped.
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Brightness of a marker's shell (its dot) at `angular_radius`.
///
/// `1.0` up to [`PREVIEW_ANGLE`], easing down to [`SHELL_FLOOR`] at
/// [`OPEN_ANGLE`] while the children resolve, then easing to `0.0` as the
/// camera passes inside the sphere (`pi/2`). Continuous and non-increasing,
/// so neither opening nor closing produces a brightness step.
#[must_use]
pub fn shell_brightness(angular_radius: f64) -> f64 {
    if !angular_radius.is_finite() {
        return 0.0;
    }
    let resolve = smoothstep(PREVIEW_ANGLE, OPEN_ANGLE, angular_radius);
    let enter = smoothstep(OPEN_ANGLE, std::f64::consts::FRAC_PI_2, angular_radius);
    (1.0 - (1.0 - SHELL_FLOOR) * resolve) * (1.0 - enter)
}

/// Brightness of previewed children at the parent marker's `angular_radius`.
///
/// Complement of the shell's resolve phase: `0.0` at [`PREVIEW_ANGLE`],
/// `1.0` from [`OPEN_ANGLE`] on, so children are fully lit by the time the
/// marker opens and the open cell draws them at full brightness.
#[must_use]
pub fn children_brightness(angular_radius: f64) -> f64 {
    if !angular_radius.is_finite() {
        return 0.0;
    }
    smoothstep(PREVIEW_ANGLE, OPEN_ANGLE, angular_radius)
}

/// Deterministic autopilot marker for a cell: a seeded pick among `portals`.
///
/// Drawn from the cell seed, so the Spacebar journey is the same every run
/// (owner decision: fixed journey). Populations never open, so only portal
/// indices are candidates. Returns `None` for a cell with no portals.
#[must_use]
pub fn autopilot_marker(cell_seed: u64, portals: &[u32]) -> Option<u32> {
    if portals.is_empty() {
        return None;
    }
    let roll = hash_cell(cell_seed, 0, 0x4155_544f, 0, 0);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: remainder below portal count, a budgeted cell size"
    )]
    let pick = (roll % portals.len() as u64) as u32;
    // `pick` is below the portal count by construction; `u32 as usize`
    // widens on every supported target, so no truncation lint applies.
    portals.get(pick as usize).copied()
}

/// Portal indices the autopilot may pick at `level` (#151 journey rules).
///
/// L3 picks among group-tier portals only (the home path runs through the
/// Local Group with Virgo as the rich sibling; groups always exist by the
/// `1 +` term, with an all-portals fallback that never triggers); L8 picks
/// the star (`portals[0]` by construction); every other level picks among
/// all portals. Empty exactly when the cell holds no portals.
#[must_use]
pub fn autopilot_candidates(level: Level, seed: u64, points: &[Point]) -> Vec<u32> {
    let mut portals: Vec<u32> = Vec::new();
    for (index, point) in points.iter().enumerate() {
        if point.kind == MarkerKind::Portal {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: marker index into a budgeted cell, always fits u32"
            )]
            let marker = index as u32;
            portals.push(marker);
        }
    }
    if level.get() == 8 {
        return portals.first().copied().into_iter().collect();
    }
    if level.get() == 3 {
        use crate::density::portal_tiers;
        let (clusters, _) = portal_tiers(level, seed, points.len());
        let groups: Vec<u32> = portals
            .iter()
            .copied()
            .filter(|marker| (*marker as usize) >= clusters)
            .collect();
        if !groups.is_empty() {
            return groups;
        }
    }
    portals
}

/// Marker indices the autopilot opens from L1 down to [`MAX_OPEN_LEVEL`].
#[must_use]
pub fn autopilot_path(root: u64) -> Vec<u32> {
    let mut chain = Vec::with_capacity(usize::from(MAX_OPEN_LEVEL - 1));
    while path_level(&chain).get() < MAX_OPEN_LEVEL {
        let cell = generate_cell(root, &chain);
        let level = path_level(&chain);
        let seed = path_seed(root, &chain);
        let candidates = autopilot_candidates(level, seed, &cell.points);
        let Some(marker) = autopilot_marker(seed, &candidates) else {
            break;
        };
        chain.push(marker);
    }
    chain
}

/// Returns the position of `marker` in `cell`, if it exists.
#[must_use]
pub fn marker_position(cell: &Generated, marker: u32) -> Option<[f64; 3]> {
    cell.points.get(marker as usize).map(|point| point.position)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::{DEMO_SEED, DiveEvent, DiveMode, MarkerIndex, Universe, WHEEL_FACTOR};
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
        assert!(child_ratio(level(8)).expect("L8") < 1e-4);
        // Stellar neighborhood -> outer system is the narrowest: under half a decade.
        assert!(child_ratio(level(6)).expect("L6") > 0.3);
    }

    #[test]
    fn budgets_cap_by_packing_and_stay_valid() {
        for n in 1..=MAX_LEVEL {
            let budget = level_budget(level(n));
            assert!(budget.is_valid(), "L{n} budget invalid");
            // L4 admits rich cells up to the fixture cap; every other level
            // stays within its base count (or the tighter packing cap).
            let cap = if n == 4 {
                RICH_CLUSTER_TOTAL
            } else {
                base_count(level(n)).max(MIN_MARKERS)
            };
            assert!(budget.max_count <= cap, "L{n} budget over cap");
        }
        assert!(level_budget(level(6)).max_count < 10, "L6 should be sparse");
        assert_eq!(level_budget(level(1)).max_count, 48);
        assert_eq!(
            level_budget(level(4)).max_count,
            RICH_CLUSTER_TOTAL,
            "L4 admits rich cells"
        );
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
    fn every_level_down_to_l10_has_markers() {
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
        assert!(!path.open(0, [0.0; 3]), "L10 markers must not open");
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
    fn anon_table_matches_spec_spans() {
        let table = [
            (1, 2),
            (2, 2),
            (3, 2),
            (4, 2),
            (5, 2),
            (6, 0),
            (7, 2),
            (8, 3),
            (9, 2),
            (10, 0),
        ];
        for (n, k) in table {
            assert_eq!(anon_cells(level(n)), k, "L{n} span milestones");
        }
        assert_eq!(anon_cells(Level::MAX), 0, "beyond scope takes none");
    }

    #[test]
    fn milestones_stay_invisible_to_generation() {
        let mut path = MarkerPath::root([0.0; 3]);
        assert!(path.is_at_root());
        assert!(path.open(3, [0.1, 0.2, 0.3]));
        path.open_anonymous(3);
        path.open_anonymous(5);
        assert_eq!(path.indices(), vec![3]);
        assert_eq!(path.level().get(), 2);
        assert_eq!(path.entered().map(|o| o.marker), Some(3));
        assert_eq!(path.top_anon_run(), 2);
        assert_eq!(path.anonymous_depth(), 2);
        assert!(!path.is_at_root());
    }

    #[test]
    fn milestones_unwind_exactly() {
        let start = [1.1, 0.7, -0.9];
        let mut path = MarkerPath::root(start);
        assert!(path.open(3, [0.1, 0.0, 0.0]));
        path.open_anonymous(3);
        path.open_anonymous(3);
        assert!(path.open(5, [-0.2, 0.1, 0.0]));
        path.open_anonymous(5);
        while path.close().is_some() {}
        assert!(path.is_at_root());
        assert_eq!(path.indices(), Vec::<u32>::new());
        for (got, want) in path.offset().iter().zip(start) {
            assert!((got - want).abs() < 1e-9, "offset drifted: {got} vs {want}");
        }
    }

    #[test]
    fn magnification_milestones_mark_long_spans() {
        let mut universe = Universe::new(DEMO_SEED);
        let marker = universe.autopilot_target().expect("root has markers");
        let mut max_run = 0usize;
        let mut opened = false;
        for _ in 0..1000 {
            match universe.dive(Some(marker), WHEEL_FACTOR, 0.0, DiveMode::Targeted) {
                DiveEvent::Opened(m) => {
                    assert_eq!(m, marker);
                    opened = true;
                    break;
                }
                DiveEvent::Closed(_) => panic!("closed while approaching"),
                DiveEvent::Moved => {}
            }
            max_run = max_run.max(universe.path.top_anon_run());
        }
        assert!(opened, "never opened L2");
        assert_eq!(max_run, 2, "L1-L2 span crosses 2 milestones");
        assert_eq!(universe.level().get(), 2);
    }

    #[test]
    fn milestones_need_target_portal_and_long_span() {
        // No target: dives to the center, pushes nothing.
        let mut universe = Universe::new(DEMO_SEED);
        for _ in 0..300 {
            universe.dive(None, WHEEL_FACTOR, 0.0, DiveMode::Targeted);
        }
        assert_eq!(universe.path.top_anon_run(), 0);
        assert_eq!(universe.level(), Level::MIN);
        // Population target: never opens, never milestones.
        let population = universe
            .open
            .points
            .iter()
            .position(|point| point.kind == MarkerKind::Population);
        if let Some(index) = population {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: test index into a budgeted cell, always fits u32"
            )]
            let marker = index as u32;
            for _ in 0..300 {
                assert_eq!(
                    universe.dive(Some(marker), WHEEL_FACTOR, 0.0, DiveMode::Targeted),
                    DiveEvent::Moved
                );
            }
            assert_eq!(universe.path.top_anon_run(), 0);
            assert_eq!(universe.level(), Level::MIN);
        }
        // Short span (L6-L7 takes none): approach opens with no milestones.
        // Manual opens leave a far offset, so the first dives may close back
        // out; retarget like the autopilot until L7 opens.
        let chain = autopilot_path(DEMO_SEED);
        let mut far = Universe::new(DEMO_SEED);
        for &m in &chain[..5] {
            assert!(far.open(MarkerIndex(m)), "open to L6");
        }
        assert_eq!(far.level().get(), 6);
        let mut target: Option<u32> = None;
        let mut opened = false;
        for _ in 0..10000 {
            if target.is_none() {
                target = far.autopilot_target();
            }
            match far.dive(target, WHEEL_FACTOR, 0.0, DiveMode::Targeted) {
                DiveEvent::Opened(_) => {
                    if far.level().get() == 7 {
                        opened = true;
                        break;
                    }
                    target = None;
                }
                DiveEvent::Closed(o) => {
                    target = Some(o.marker);
                }
                DiveEvent::Moved => {
                    if far.level().get() == 6 {
                        assert_eq!(far.path.top_anon_run(), 0, "short span milestones");
                    }
                }
            }
        }
        assert!(opened, "never opened L7");
        assert_eq!(far.level().get(), 7);
    }

    #[test]
    fn brightness_curves_are_continuous_and_monotonic() {
        assert_eq!(shell_brightness(0.0), 1.0);
        assert_eq!(shell_brightness(PREVIEW_ANGLE), 1.0);
        assert!((shell_brightness(OPEN_ANGLE) - SHELL_FLOOR).abs() < 1e-12);
        assert!(shell_brightness(std::f64::consts::FRAC_PI_2) < 1e-12);
        assert_eq!(children_brightness(PREVIEW_ANGLE), 0.0);
        assert_eq!(children_brightness(OPEN_ANGLE), 1.0);
        let eps = 1e-6;
        for edge in [PREVIEW_ANGLE, OPEN_ANGLE, CLOSE_ANGLE] {
            assert!((shell_brightness(edge - eps) - shell_brightness(edge + eps)).abs() < 1e-4);
            assert!(
                (children_brightness(edge - eps) - children_brightness(edge + eps)).abs() < 1e-4
            );
        }
        let mut prev_shell = f64::INFINITY;
        let mut prev_children = f64::NEG_INFINITY;
        let mut angle = 0.0;
        while angle <= std::f64::consts::FRAC_PI_2 {
            let shell = shell_brightness(angle);
            let children = children_brightness(angle);
            assert!(shell <= prev_shell + 1e-12 && (0.0..=1.0).contains(&shell));
            assert!(children >= prev_children - 1e-12 && (0.0..=1.0).contains(&children));
            prev_shell = shell;
            prev_children = children;
            angle += 0.001;
        }
        assert_eq!(shell_brightness(f64::NAN), 0.0);
    }

    #[test]
    fn marker_seeds_depend_on_index_and_level() {
        let base = marker_seed(7, level(2), 0);
        assert_ne!(base, marker_seed(7, level(2), 1));
        assert_ne!(base, marker_seed(7, level(3), 0));
        assert_eq!(base, marker_seed(7, level(2), 0));
    }

    #[test]
    fn home_journey_fixtures_hold() {
        let chain = autopilot_path(42);
        assert_eq!(chain.len(), 9, "home journey must open L2-L10");
        // Home L3 pick lands in the group tier (structural journey rule).
        let level3 = Level::new(3).expect("L3");
        let l3seed = path_seed(42, &chain[..2]);
        let l3 = generate_cell(42, &chain[..2]);
        let (clusters, _) = crate::density::portal_tiers(level3, l3seed, l3.points.len());
        let candidates = autopilot_candidates(level3, l3seed, &l3.points);
        let pick = autopilot_marker(l3seed, &candidates).expect("L3 pick");
        assert!(
            (pick as usize) >= clusters,
            "home L3 pick {pick} must be a group portal (clusters={clusters})"
        );
        // Home L4 reached through the group is poor, not rich.
        assert!(
            !is_rich_cluster_cell(42, &chain[..3]),
            "group-entered L4 must not be rich"
        );
        // The densest cluster portal of the home L3 cell opens rich content.
        let home_l3 = generate_cell(42, &chain[..2]);
        let anchor = chain[..2].last().copied().map_or(0, i64::from);
        let densest = densest_portal_index(l3seed, anchor, &home_l3.points)
            .expect("home L3 has a cluster portal");
        assert!(
            (densest as usize) < clusters,
            "densest portal must be a cluster portal"
        );
        let mut rich_chain = chain[..2].to_vec();
        rich_chain.push(densest);
        assert!(
            is_rich_cluster_cell(42, &rich_chain),
            "densest-cluster cell must be rich"
        );
        let rich = generate_cell(42, &rich_chain);
        assert_eq!(rich.points.len(), 2000, "rich L4 holds thousands");
        let rich_portals = rich
            .points
            .iter()
            .filter(|point| point.kind == MarkerKind::Portal)
            .count();
        assert_eq!(rich_portals, 160, "rich L4 portals for large members");
        // Home L6 shows systems; home L8 shows exactly 8 planets.
        let l6 = generate_cell(42, &chain[..5]);
        assert!(
            l6.points
                .iter()
                .any(|point| point.kind == MarkerKind::Portal),
            "home L6 must hold a system portal"
        );
        // Star portal sits at index 0; every other portal is a planet.
        let l8 = generate_cell(42, &chain[..7]);
        assert_eq!(l8.points[0].kind, MarkerKind::Portal, "L8 star first");
        let planets = l8
            .points
            .iter()
            .skip(1)
            .filter(|point| point.kind == MarkerKind::Portal);
        assert_eq!(planets.count(), 8, "Solar home cell holds 8 planets");
        // L9 agrees with L8 on the planet count through the shared seed.
        let l9 = generate_cell(42, &chain[..8]);
        let l9_planets = l9
            .points
            .iter()
            .filter(|point| point.kind == MarkerKind::Portal)
            .count();
        assert_eq!(l9_planets, 8, "L9 close-up shows the same 8 planets");
    }

    #[test]
    fn autopilot_skips_populations_deterministically() {
        assert!(autopilot_marker(1, &[]).is_none());
        assert!(autopilot_marker(1, &[0, 1, 2, 3, 4]).expect("index") < 5);
        let portals = [2u32, 5, 9];
        let first = autopilot_marker(7, &portals).expect("portal");
        assert!(portals.contains(&first));
        assert_eq!(first, autopilot_marker(7, &portals).expect("portal"));
        let other = autopilot_marker(8, &portals).expect("portal");
        assert!(portals.contains(&other));
    }

    #[test]
    fn home_l4_cell_holds_the_barred_milky_way_on_the_journey() {
        use crate::astro::MILKY_WAY_ORIENTATION;
        use crate::r#gen::GalaxyKind;
        let chain = autopilot_path(DEMO_SEED);
        assert!(chain.len() >= 4, "journey must reach L5");
        let home_chain = &chain[..3];
        assert!(is_home_l4_cell(DEMO_SEED, home_chain));
        assert!(!is_home_l4_cell(DEMO_SEED, &chain[..2]));
        let mut sibling = home_chain.to_vec();
        let last = sibling.len() - 1;
        sibling[last] = sibling[last].wrapping_add(1);
        assert!(!is_home_l4_cell(DEMO_SEED, &sibling));
        let home = generate_cell(DEMO_SEED, home_chain);
        let seed = path_seed(DEMO_SEED, home_chain);
        let candidates = autopilot_candidates(level(4), seed, &home.points);
        let pick = autopilot_marker(seed, &candidates).expect("home L4 pick");
        assert_eq!(pick, chain[3], "the journey opens the Milky Way");
        let milky_way = home.points[pick as usize];
        assert_eq!(milky_way.kind, MarkerKind::Portal);
        let shape = milky_way.galaxy.expect("L4 carries shape data");
        assert_eq!(shape.kind, GalaxyKind::Spiral);
        assert!(shape.barred, "the Milky Way is barred");
        assert_eq!(shape.orientation, MILKY_WAY_ORIENTATION);
        assert!(!shape.far_view, "the home galaxy is never an impostor");
    }
}
