//! Headless navigation state machine, dive math, and journey replay.
//!
//! Moved from `universe-render` in M3 (#85) so the headless gate reaches
//! everything `--verify` needs without linking Bevy. The window keeps
//! working through thin resource newtypes in `universe-render` that deref
//! to these types. Positions stay bare `[f64; 3]` here; unit-tagged
//! positions are a follow-up sub-issue under #85.

use crate::coords::{Level, OpenUnits, ParentUnits};
use crate::r#gen::{Generated, MarkerKind};
use crate::nest::{
    CLOSE_ANGLE, MarkerPath, OPEN_ANGLE, Opened, angular_radius, append_preview_set,
    autopilot_candidates, autopilot_marker, child_ratio, child_world_position, generate_cell,
    marker_position, marker_radius, path_seed,
};
use crate::snapshot::snapshot_generated;

/// Root seed a universe derives every cell from.
///
/// Bare `u64` seeds flow opaquely; the navigation layer stores this type on
/// `Universe` and unwraps it to `u64` exactly where the unmoved
/// `universe-core::nest` APIs take bare seeds (`E-NEWTYPE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Seed(pub u64);

impl From<u64> for Seed {
    fn from(seed: u64) -> Seed {
        Seed(seed)
    }
}

/// Marker index inside a cell.
///
/// The navigation query surface (`Universe::marker`, `Universe::open`,
/// `preview_positions`) takes this type so indices cannot be mixed with
/// counts. `JourneyStep.marker` and event payloads stay `u32` where
/// `universe-app` reads them (`E-NEWTYPE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MarkerIndex(pub u32);

impl From<u32> for MarkerIndex {
    fn from(marker: u32) -> MarkerIndex {
        MarkerIndex(marker)
    }
}

/// Component-wise subtraction of three-component vectors.
///
/// Mirrors glam's scalar `DVec3` operation order so the removal of the
/// render-side vector type changes no bit (`E-DET-TIERS`).
fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Component-wise addition of three-component vectors.
///
/// Mirrors glam's scalar `DVec3` operation order.
fn add3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// Scalar multiplication of a three-component vector.
///
/// Mirrors glam's scalar `DVec3` operation order.
fn mul3(v: [f64; 3], s: f64) -> [f64; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

/// Scalar division of a three-component vector.
///
/// Mirrors glam's scalar `DVec3` operation order.
fn div3(v: [f64; 3], s: f64) -> [f64; 3] {
    [v[0] / s, v[1] / s, v[2] / s]
}

/// Dot product of three-component vectors: `x*x + y*y + z*z`.
///
/// Mirrors glam's scalar `DVec3` operation order.
fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Euclidean length: `dot` then `sqrt`, mirroring glam's scalar `DVec3`.
fn length3(v: [f64; 3]) -> f64 {
    dot3(v, v).sqrt()
}

/// Fixed demo root seed `sigma_0` for the window and `--verify`.
///
/// Every cell along every path derives from it, so the universe is the same
/// on every run (determinism invariant).
pub const DEMO_SEED: u64 = 42;

/// Shallowest level: L1 (observable universe), the root cell.
pub const MIN_NAV_LEVEL: u8 = 1;

/// Deepest open level: L10 (planets, #151). Its markers do not open.
pub const MAX_NAV_LEVEL: u8 = 10;

/// Camera start position in root-cell units: outside the universe cell.
pub const START_OFFSET: [f64; 3] = [1.4, 1.0, 1.4];

/// Smallest angular radius a marker is drawn at (impostor clamp, section 7).
pub const MIN_MARKER_ANGLE: f64 = 0.004;

/// Fraction of the remaining distance kept per wheel notch (`h' = 0.75 h`).
pub const WHEEL_FACTOR: f64 = 0.75;

/// `k_speed` for held keys: `h' = h * exp(-k dt)`.
pub const KEY_RATE: f64 = 1.5;

/// `k_speed` for the autopilot journey.
pub const AUTOPILOT_RATE: f64 = 1.2;

/// Farthest the camera may rise from the root cell center, in root units.
pub const ROOT_MAX_DISTANCE: f64 = 6.0;

/// Radius markers of the open `level` are drawn at, in open-cell units.
///
/// The true child size (`docs/universe/ladder.md` ratios); L14 falls back to
/// the indicator radius since nothing is deeper.
#[must_use]
pub fn open_marker_radius(level: Level, indicator_radius: f64) -> f64 {
    marker_radius(level).unwrap_or(indicator_radius)
}

/// Drawn radius after the impostor clamp: at least [`MIN_MARKER_ANGLE`].
#[must_use]
pub fn drawn_radius(true_radius: f64, distance: f64) -> f64 {
    true_radius.max(distance * MIN_MARKER_ANGLE.tan())
}

/// One dive step toward (`factor < 1`) or away from (`factor > 1`) a sphere.
///
/// `h` is the gap between the camera and the sphere surface; the step keeps
/// `h' = factor * h` along the line of sight, which is `v = k * h`
/// integrated. The camera never crosses the surface. A camera sitting
/// exactly on the center steps along `+z`.
#[must_use]
pub fn dive_step(camera: OpenUnits, center: OpenUnits, radius: f64, factor: f64) -> OpenUnits {
    let to_camera = sub3(camera.0, center.0);
    let distance = length3(to_camera);
    let direction = if distance > 0.0 {
        div3(to_camera, distance)
    } else {
        [0.0, 0.0, 1.0]
    };
    let gap = (distance - radius).max(0.0);
    let next_gap = (gap * factor).max(radius * 1e-3);
    OpenUnits(add3(center.0, mul3(direction, radius + next_gap)))
}

/// Returns `true` when a marker of `radius` at `distance` should open.
#[must_use]
pub fn should_open(radius: f64, distance: f64) -> bool {
    angular_radius(radius, distance) > OPEN_ANGLE
}

/// Returns `true` when the open cell (radius 0.5) at `distance_to_center`
/// should close back into its marker.
#[must_use]
pub fn should_close(distance_to_center: f64) -> bool {
    angular_radius(0.5, distance_to_center) < CLOSE_ANGLE
}

/// Position and radius of a parent-cell sibling marker in open-cell units.
///
/// The open cell sits at `entered.position` in parent units with size ratio
/// `entered.ratio`, so a sibling at `sibling` lands at
/// `(sibling - position) / ratio` and every sibling has radius 0.5.
#[must_use]
pub fn sibling_in_open_units(entered: Opened, sibling: ParentUnits) -> (OpenUnits, f64) {
    let mut out = [0.0; 3];
    for ((slot, &anchor), &local) in out
        .iter_mut()
        .zip(entered.position.iter())
        .zip(sibling.0.iter())
    {
        *slot = (local - anchor) / entered.ratio;
    }
    (OpenUnits(out), 0.5)
}

/// The nested universe as seen by the observer: the open cell and its parent.
///
/// Exactly two generations live at a time (section 8 bounded memory). Both
/// regenerate from the path when it changes, so nothing is stored.
#[derive(Debug, Clone, PartialEq)]
pub struct Universe {
    /// Root seed every cell derives from.
    pub root: Seed,
    /// Observer path and camera offset.
    pub path: MarkerPath,
    /// Content of the open cell.
    pub open: Generated,
    /// Content of the parent cell, if the open cell is not the root.
    pub parent: Option<Generated>,
}

impl Universe {
    /// Starts at the root cell with the camera at [`START_OFFSET`].
    #[must_use]
    pub fn new(root: u64) -> Universe {
        let mut universe = Universe {
            root: Seed(root),
            path: MarkerPath::root(START_OFFSET),
            open: Generated {
                points: Vec::new(),
                child_constraints: Vec::new(),
            },
            parent: None,
        };
        universe.reload();
        universe
    }

    /// Regenerates the open and parent cells from the current path.
    pub fn reload(&mut self) {
        let mut parent_indices = self.path.indices();
        self.open = generate_cell(self.root.0, &parent_indices);
        self.parent = parent_indices
            .pop()
            .map(|_| generate_cell(self.root.0, &parent_indices));
    }

    /// Level of the open cell.
    #[must_use]
    pub fn level(&self) -> Level {
        self.path.level()
    }

    /// Seed of the open cell.
    #[must_use]
    pub fn open_seed(&self) -> u64 {
        path_seed(self.root.0, &self.path.indices())
    }

    /// Drawn-size radius of the open cell's markers (true child size).
    #[must_use]
    pub fn marker_radius(&self) -> f64 {
        open_marker_radius(self.level(), 0.01)
    }

    /// Position of marker `marker` in the open cell.
    #[must_use]
    pub fn marker(&self, marker: MarkerIndex) -> Option<[f64; 3]> {
        marker_position(&self.open, marker.0)
    }

    /// Opens `marker`: the camera stays put while the origin descends.
    ///
    /// Returns `false`, untouched, when the marker is missing, is a
    /// population point, or the level cannot open.
    pub fn open(&mut self, marker: MarkerIndex) -> bool {
        let Some(point) = self.open.points.get(marker.0 as usize) else {
            return false;
        };
        if point.kind != MarkerKind::Portal {
            return false;
        }
        let position = point.position;
        if !self.path.open(marker.0, position) {
            return false;
        }
        self.reload();
        true
    }

    /// Closes the open cell back into the marker it came from.
    pub fn close(&mut self) -> Option<Opened> {
        let opened = self.path.close()?;
        self.reload();
        Some(opened)
    }

    /// Autopilot's marker for the open cell (seeded, fixed per run).
    ///
    /// Chosen among portal markers only (populations never open), following
    /// the level pick rules in [`autopilot_candidates`](crate::nest::autopilot_candidates).
    #[must_use]
    pub fn autopilot_target(&self) -> Option<u32> {
        let candidates = autopilot_candidates(self.level(), self.open_seed(), &self.open.points);
        autopilot_marker(self.open_seed(), &candidates)
    }

    /// Canonical snapshot of the open cell.
    #[must_use]
    pub fn snapshot(&self) -> String {
        snapshot_generated(&self.open)
    }

    /// Moves the camera one dive step relative to `target` (or the cell
    /// center when `None`), opening or closing as thresholds are crossed.
    ///
    /// Returns the navigation event that happened, so callers (window and
    /// `--verify`) can re-target identically.
    pub fn dive(&mut self, target: Option<u32>, factor: f64) -> DiveEvent {
        let camera = OpenUnits(self.path.offset());
        let (center, radius) = match target.and_then(|m| self.marker(MarkerIndex(m))) {
            Some(position) => (OpenUnits(position), self.marker_radius()),
            None => (OpenUnits([0.0; 3]), 0.0),
        };
        let mut next = dive_step(camera, center, radius, factor);
        if self.path.chain().is_empty() {
            let distance = length3(next.0);
            if distance > ROOT_MAX_DISTANCE {
                next = OpenUnits(mul3(next.0, ROOT_MAX_DISTANCE / distance));
            }
        }
        self.path.set_offset(next.0);
        if let Some(marker) = target
            && self.path.can_open()
            && let Some(position) = self.marker(MarkerIndex(marker))
            && self
                .open
                .points
                .get(marker as usize)
                .is_some_and(|point| point.kind == MarkerKind::Portal)
        {
            let distance = length3(sub3(next.0, position));
            if should_open(self.marker_radius(), distance) && self.open(MarkerIndex(marker)) {
                return DiveEvent::Opened(marker);
            }
        }
        if should_close(self.path.distance_to_center())
            && let Some(opened) = self.close()
        {
            return DiveEvent::Closed(opened);
        }
        DiveEvent::Moved
    }
}

/// What a dive step did to the path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiveEvent {
    /// Camera moved; the open cell is unchanged.
    Moved,
    /// The targeted marker opened; the lock clears.
    Opened(u32),
    /// The open cell closed into this marker; it becomes the target again.
    Closed(Opened),
}

/// Interiors of the open cell's largest-on-screen markers, drawn before entry
/// (R7, #63).
///
/// Holds at most [`PREVIEW_CAP`](crate::nest::PREVIEW_CAP) generations, keyed
/// by the open path and the marker index. [`PreviewCache::sync`] regenerates
/// only when the preview set changes, so the draw path never allocates per
/// frame (section 8 bounded memory: `2 + PREVIEW_CAP` generations alive at
/// most). A static camera neither regenerates nor reallocates.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PreviewCache {
    path: Vec<u32>,
    entries: Vec<(u32, Generated)>,
    /// Total cell generations performed by this cache (test/verify counter).
    pub regenerations: u64,
    /// Reused preview-set buffer: steady-state syncs allocate nothing.
    wanted: Vec<u32>,
    /// Camera offset of the last sync; `None` before the first sync.
    last_offset: Option<[f64; 3]>,
}

/// Compares stored marker indices against the live path without allocating.
fn path_matches(stored: &[u32], chain: &[Opened]) -> bool {
    stored.len() == chain.len()
        && stored
            .iter()
            .zip(chain.iter())
            .all(|(marker, opened)| *marker == opened.marker)
}

impl PreviewCache {
    /// Brings the cache in line with the camera: returns the current preview
    /// set and `true` when any cell was (re)generated.
    ///
    /// Allocation-free for a static camera: an unchanged path and offset
    /// return early, and the wanted set reuses a buffer otherwise.
    pub fn sync(&mut self, universe: &Universe) -> bool {
        let offset = universe.path.offset();
        if self.last_offset == Some(offset) && path_matches(&self.path, universe.path.chain()) {
            return false;
        }
        self.last_offset = Some(offset);
        if !path_matches(&self.path, universe.path.chain()) {
            self.path = universe.path.indices();
            self.entries.clear();
        }
        self.wanted.clear();
        if universe.path.can_open() {
            append_preview_set(
                &mut self.wanted,
                offset,
                &universe.open.points,
                universe.marker_radius(),
            );
        }
        let before = self.entries.len();
        self.entries
            .retain(|(marker, _)| self.wanted.contains(marker));
        let mut changed = self.entries.len() != before;
        for marker in self.wanted.iter().copied() {
            if self.entries.iter().any(|(m, _)| *m == marker) {
                continue;
            }
            let mut chain = self.path.clone();
            chain.push(marker);
            self.entries
                .push((marker, generate_cell(universe.root.0, &chain)));
            self.regenerations += 1;
            changed = true;
        }
        if changed {
            self.entries.sort_by_key(|(marker, _)| *marker);
        }
        changed
    }

    /// Number of previewed markers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` when nothing is previewed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Previewed content of `marker`, if cached.
    #[must_use]
    pub fn get(&self, marker: u32) -> Option<&Generated> {
        self.entries
            .iter()
            .find(|(m, _)| *m == marker)
            .map(|(_, generated)| generated)
    }

    /// Previewed markers with their content, by marker index.
    #[must_use]
    pub fn entries(&self) -> &[(u32, Generated)] {
        &self.entries
    }
}

/// World positions (open-cell units) of `marker`'s previewed children.
///
/// Empty when the marker is missing or the level cannot open.
#[must_use]
pub fn preview_positions(
    universe: &Universe,
    marker: MarkerIndex,
    content: &Generated,
) -> Vec<OpenUnits> {
    let (Some(marker_pos), Some(ratio)) = (universe.marker(marker), child_ratio(universe.level()))
    else {
        return Vec::new();
    };
    content
        .points
        .iter()
        .map(|point| {
            OpenUnits(child_world_position(ParentUnits(marker_pos), ratio, point.position).0)
        })
        .collect()
}

/// Largest relative error between `previewed` (pre-open, parent units,
/// re-expressed through `opened`) and the open cell's marker positions.
#[must_use]
pub fn preview_error(previewed: &[OpenUnits], opened: Opened, open: &Generated) -> f64 {
    let mut worst = 0.0f64;
    if previewed.len() != open.points.len() {
        return f64::INFINITY;
    }
    for (world, point) in previewed.iter().zip(&open.points) {
        for ((w, anchor), want) in world.0.iter().zip(opened.position).zip(point.position) {
            let back = (w - anchor) / opened.ratio;
            worst = worst.max((back - want).abs() / want.abs().max(1.0));
        }
    }
    worst
}

/// One opened level along a headless autopilot replay.
#[derive(Debug, Clone, PartialEq)]
pub struct JourneyStep {
    /// Level opened into.
    pub level: Level,
    /// Marker opened to get there.
    pub marker: u32,
    /// Seconds since the journey started.
    pub elapsed: f64,
    /// Canonical snapshot of the newly open cell.
    pub snapshot: String,
    /// Markers previewed in the frame before opening.
    pub preview_count: usize,
    /// Relative error between previewed child positions and the open cell.
    pub preview_error: f64,
    /// Generations alive in the frame before opening (open + parent + previews).
    pub alive: usize,
}

/// Replays the Spacebar journey headlessly at `dt` seconds per step.
///
/// Same math as the window: target the seeded marker, step with
/// [`AUTOPILOT_RATE`], open at [`OPEN_ANGLE`], repeat until L10 or
/// `max_secs`. Returns the opened levels in order and the final universe.
#[must_use]
pub fn replay_autopilot(root: u64, dt: f64, max_secs: f64) -> (Vec<JourneyStep>, Universe) {
    let mut universe = Universe::new(root);
    let mut previews = PreviewCache::default();
    let mut steps = Vec::new();
    let mut target = None;
    let mut elapsed = 0.0;
    let factor = (-AUTOPILOT_RATE * dt).exp();
    while universe.path.can_open() && elapsed < max_secs {
        if target.is_none() {
            target = universe.autopilot_target();
            if target.is_none() {
                break;
            }
        }
        previews.sync(&universe);
        let alive = 1 + usize::from(universe.parent.is_some()) + previews.len();
        let previewed = target
            .and_then(|m| {
                previews
                    .get(m)
                    .map(|c| preview_positions(&universe, MarkerIndex(m), c))
            })
            .unwrap_or_default();
        elapsed += dt;
        match universe.dive(target, factor) {
            DiveEvent::Opened(marker) => {
                target = None;
                let opened = universe.path.entered().unwrap_or(Opened {
                    marker,
                    position: [0.0; 3],
                    ratio: 1.0,
                });
                steps.push(JourneyStep {
                    level: universe.level(),
                    marker,
                    elapsed,
                    snapshot: universe.snapshot(),
                    preview_count: previews.len(),
                    preview_error: preview_error(&previewed, opened, &universe.open),
                    alive,
                });
            }
            DiveEvent::Closed(opened) => target = Some(opened.marker),
            DiveEvent::Moved => {}
        }
    }
    (steps, universe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nest::{CLOSE_ANGLE, OPEN_ANGLE, PREVIEW_CAP};

    #[test]
    fn dive_step_closes_and_never_crosses_surface() {
        let mut camera = OpenUnits([0.0, 0.0, 4.0]);
        let center = OpenUnits([0.0, 0.0, 0.0]);
        let radius = 0.2;
        let mut previous = 4.0;
        for step in 0..200 {
            camera = dive_step(camera, center, radius, WHEEL_FACTOR);
            let distance = length3(camera.0);
            if step < 20 {
                assert!(distance < previous, "did not approach at step {step}");
            } else {
                assert!(distance <= previous, "pulled away at step {step}");
            }
            assert!(distance > radius, "crossed the surface");
            previous = distance;
        }
        let away = dive_step(camera, center, radius, 1.0 / WHEEL_FACTOR);
        assert!(length3(away.0) > previous);
    }

    #[test]
    fn thresholds_have_hysteresis() {
        const { assert!(OPEN_ANGLE > CLOSE_ANGLE) };
        let open_distance = 0.5 / OPEN_ANGLE.sin();
        assert!(should_open(0.5, open_distance * 0.99));
        assert!(!should_open(0.5, open_distance * 1.01));
        assert!(!should_close(open_distance));
        assert!(should_close(0.5 / CLOSE_ANGLE.sin() * 1.01));
    }

    #[test]
    fn open_refuses_populations_but_opens_portals() {
        let mut universe = Universe::new(DEMO_SEED);
        let population = universe
            .open
            .points
            .iter()
            .position(|point| point.kind == MarkerKind::Population);
        let portal = universe
            .open
            .points
            .iter()
            .position(|point| point.kind == MarkerKind::Portal);
        if let Some(index) = population {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: test index into a budgeted cell, always fits u32"
            )]
            let marker = MarkerIndex(index as u32);
            assert!(!universe.open(marker), "population marker must not open");
            assert_eq!(universe.level(), Level::MIN, "failed open must not move");
        }
        if let Some(index) = portal {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: test index into a budgeted cell, always fits u32"
            )]
            let marker = MarkerIndex(index as u32);
            assert!(universe.open(marker), "portal marker must open");
            assert_eq!(universe.level().get(), 2);
        }
    }

    #[test]
    fn opening_keeps_camera_outside_child_cell() {
        let mut universe = Universe::new(DEMO_SEED);
        let marker = universe.autopilot_target().expect("root has markers");
        let mut target = Some(marker);
        let mut event = DiveEvent::Moved;
        for _ in 0..500 {
            event = universe.dive(target, WHEEL_FACTOR);
            if event != DiveEvent::Moved {
                break;
            }
        }
        assert_eq!(event, DiveEvent::Opened(marker));
        assert_eq!(universe.level().get(), 2);
        assert!(universe.path.distance_to_center() > 0.5);
        assert!(universe.parent.is_some());
        target = None;
        let mut closed = None;
        for _ in 0..500 {
            if let DiveEvent::Closed(opened) = universe.dive(target, 1.0 / WHEEL_FACTOR) {
                closed = Some(opened);
                break;
            }
        }
        assert_eq!(
            closed.map(|o| o.marker),
            Some(marker),
            "close must re-target"
        );
        assert_eq!(universe.level(), Level::MIN);
    }

    #[test]
    fn autopilot_replay_reaches_l10_deterministically() {
        let (steps, universe) = replay_autopilot(DEMO_SEED, 1.0 / 60.0, 600.0);
        assert_eq!(universe.level().get(), MAX_NAV_LEVEL);
        assert_eq!(steps.len(), usize::from(MAX_NAV_LEVEL - 1));
        for (index, step) in steps.iter().enumerate() {
            assert_eq!(usize::from(step.level.get()), index + 2);
            assert!(
                step.preview_count >= 1,
                "target was not previewed before opening"
            );
            assert!(step.preview_count <= PREVIEW_CAP);
            assert!(
                step.preview_error <= 1e-9,
                "preview drifted: {}",
                step.preview_error
            );
            assert!(step.alive <= 2 + PREVIEW_CAP);
        }
        let (again, _) = replay_autopilot(DEMO_SEED, 1.0 / 60.0, 600.0);
        assert_eq!(steps, again, "journey must be the same every run");
        assert!(
            steps.last().expect("steps").elapsed < 120.0,
            "journey too long"
        );
    }

    #[test]
    fn preview_cache_regenerates_only_when_the_set_changes() {
        let mut universe = Universe::new(DEMO_SEED);
        let mut cache = PreviewCache::default();
        assert!(!cache.sync(&universe), "far away nothing previews");
        assert!(cache.is_empty());
        let marker = universe.autopilot_target().expect("marker");
        let target = Some(marker);
        let mut synced_once = false;
        for _ in 0..500 {
            cache.sync(&universe);
            if !cache.is_empty() && !synced_once {
                synced_once = true;
                let before = cache.regenerations;
                assert!(!cache.sync(&universe), "same camera must not regenerate");
                assert_eq!(cache.regenerations, before);
                assert!(cache.len() <= PREVIEW_CAP);
            }
            if universe.dive(target, WHEEL_FACTOR) != DiveEvent::Moved {
                break;
            }
        }
        assert!(synced_once, "target never entered the preview set");
        assert_eq!(universe.level().get(), 2);
        cache.sync(&universe);
        assert!(
            cache
                .entries()
                .iter()
                .all(|(m, _)| universe.marker(MarkerIndex(*m)).is_some())
        );
    }

    #[test]
    fn sync_is_free_for_a_static_camera() {
        let mut universe = Universe::new(DEMO_SEED);
        let mut cache = PreviewCache::default();
        let marker = universe.autopilot_target().expect("marker");
        let target = Some(marker);
        for _ in 0..500 {
            cache.sync(&universe);
            if !cache.is_empty() {
                break;
            }
            if universe.dive(target, WHEEL_FACTOR) != DiveEvent::Moved {
                break;
            }
        }
        assert!(!cache.is_empty(), "nothing ever previewed");
        assert!(!cache.wanted.is_empty(), "wanted buffer never filled");
        let regens = cache.regenerations;
        let entries_ptr = cache.entries.as_ptr();
        let entries_cap = cache.entries.capacity();
        let wanted_ptr = cache.wanted.as_ptr();
        let wanted_cap = cache.wanted.capacity();
        for _ in 0..10 {
            assert!(!cache.sync(&universe), "static camera must not change");
        }
        assert_eq!(cache.regenerations, regens, "static camera regenerated");
        assert_eq!(cache.entries.as_ptr(), entries_ptr, "entries reallocated");
        assert_eq!(cache.entries.capacity(), entries_cap);
        assert_eq!(
            cache.wanted.as_ptr(),
            wanted_ptr,
            "wanted buffer reallocated"
        );
        assert_eq!(cache.wanted.capacity(), wanted_cap);
    }

    #[test]
    fn root_cannot_rise_past_cap_or_close() {
        let mut universe = Universe::new(DEMO_SEED);
        for _ in 0..100 {
            assert_eq!(universe.dive(None, 1.0 / WHEEL_FACTOR), DiveEvent::Moved);
        }
        assert!(universe.path.distance_to_center() <= ROOT_MAX_DISTANCE + 1e-9);
        assert_eq!(universe.level(), Level::MIN);
    }

    #[test]
    fn siblings_land_one_over_ratio_away() {
        let entered = Opened {
            marker: 0,
            position: [0.1, 0.0, 0.0],
            ratio: 0.01,
        };
        let (position, radius) = sibling_in_open_units(entered, ParentUnits([0.2, 0.0, 0.0]));
        assert!((position.0[0] - 10.0).abs() < 1e-9);
        assert_eq!(radius, 0.5);
        assert!(drawn_radius(1e-6, 10.0) > 1e-6);
        assert_eq!(drawn_radius(1.0, 10.0), 1.0);
    }
}
