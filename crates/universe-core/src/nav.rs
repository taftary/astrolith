//! Headless navigation state machine, dive math, and journey replay.
//!
//! Moved from `universe-render` in M3 (#85) so the headless gate reaches
//! everything `--verify` needs without linking Bevy. The window keeps
//! working through thin resource newtypes in `universe-render` that deref
//! to these types. Positions stay bare `[f64; 3]` here; unit-tagged
//! positions are a follow-up sub-issue under #85.

use crate::coords::{Level, OpenUnits, ParentUnits};
use crate::frame::angular_radius;
use crate::r#gen::{Generated, MarkerKind};
use crate::nest::{
    CLOSE_ANGLE, MarkerPath, OPEN_ANGLE, Opened, anon_cells, autopilot_candidates,
    autopilot_marker, generate_cell, marker_position, marker_radius, path_seed,
};
use crate::preview::{PreviewCache, preview_error, preview_positions};
use crate::snapshot::snapshot_generated;
use crate::stream::{StreamLedger, working_chains};

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
pub(crate) fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Component-wise addition of three-component vectors.
///
/// Mirrors glam's scalar `DVec3` operation order.
pub(crate) fn add3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// Scalar multiplication of a three-component vector.
///
/// Mirrors glam's scalar `DVec3` operation order.
pub(crate) fn mul3(v: [f64; 3], s: f64) -> [f64; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

/// Scalar division of a three-component vector.
///
/// Mirrors glam's scalar `DVec3` operation order.
pub(crate) fn div3(v: [f64; 3], s: f64) -> [f64; 3] {
    [v[0] / s, v[1] / s, v[2] / s]
}

/// Dot product of three-component vectors: `x*x + y*y + z*z`.
///
/// Mirrors glam's scalar `DVec3` operation order.
pub(crate) fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Euclidean length: `dot` then `sqrt`, mirroring glam's scalar `DVec3`.
pub(crate) fn length3(v: [f64; 3]) -> f64 {
    dot3(v, v).sqrt()
}

/// Fixed demo root seed `sigma_0` for the window and `--verify`.
///
/// Every cell along every path derives from it, so the universe is the same
/// on every run (determinism invariant).
pub const DEMO_SEED: u64 = 42;

/// Shallowest level: L1 (observable universe), the root cell.
pub const MIN_NAV_LEVEL: u8 = 1;

/// Deepest open level: L14 (room, #375). Its markers do not open.
pub const MAX_NAV_LEVEL: u8 = 14;

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
/// The true child size (`docs/universes/ladder.md` ratios); L14 falls back to
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
///
/// The only close rule for the cell the camera is in (#403): the
/// travel-time horizon of ADR 0012 applies to streamed cells, never to the
/// open cell, so the open and close thresholds are one currency
/// ([`OPEN_ANGLE`] in, [`CLOSE_ANGLE`] out).
#[must_use]
pub fn should_close(distance_to_center: f64) -> bool {
    angular_radius(0.5, distance_to_center) < CLOSE_ANGLE
}

/// Portal markers the segment `from` -> `to` crosses, in ray order (#152).
///
/// A portal counts as crossed when it would open at the far end
/// ([`should_open`] with the level's marker `radius`); populations never
/// qualify. Order is the along-ray projection (nearest first, ties by
/// distance), so opening the head of the list each step walks the ray in
/// order. Pure: same inputs, same list (`E-DET-TIERS`).
#[must_use]
pub fn crossed_portals(from: [f64; 3], to: [f64; 3], open: &Generated, radius: f64) -> Vec<u32> {
    let direction = sub3(to, from);
    let along = dot3(direction, direction);
    if !along.is_finite() || along <= 0.0 {
        return Vec::new();
    }
    let mut order: Vec<(u32, f64, f64)> = Vec::new();
    for (index, point) in open.points.iter().enumerate() {
        if point.kind != MarkerKind::Portal {
            continue;
        }
        let at = point.portal_position();
        let distance = length3(sub3(at, to));
        if !should_open(radius, distance) {
            continue;
        }
        let reach = dot3(sub3(at, from), direction) / along;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: marker index into a budgeted cell, always fits u32"
        )]
        let marker = index as u32;
        order.push((marker, reach, distance));
    }
    order.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.2.total_cmp(&b.2)));
    order.into_iter().map(|(marker, _, _)| marker).collect()
}

/// Pushes magnification milestones for `marker` as its angular radius
/// crosses fractions of [`OPEN_ANGLE`] (#151 T4).
///
/// Silent exact no-ops, so every span in the table is crossed through
/// invisible cells. The dive calls it for the stored target on approach and
/// once more for a pass-through opening that took another marker.
fn push_span_milestones(path: &mut MarkerPath, marker: u32, radius: f64, distance: f64) {
    let steps = anon_cells(path.level());
    if steps == 0 {
        return;
    }
    let angular = angular_radius(radius, distance);
    let run = path.top_anon_run();
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: milestone indices below 4, exactly representable"
    )]
    let first = (run + 1) as f64;
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: milestone indices below 4, exactly representable"
    )]
    let slots = (steps + 1) as f64;
    let mut threshold = OPEN_ANGLE * first / slots;
    for _ in run..steps {
        if angular > threshold {
            path.open_anonymous(marker);
        }
        threshold += OPEN_ANGLE / slots;
    }
}

/// Whether a dive step may open crossed portals in passing (#152).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiveMode {
    /// Manual dive: every crossed portal opens in ray order while the
    /// stored target is only read, never touched.
    #[default]
    Passing,
    /// Autopilot: only the stored target opens, so the fixed seeded
    /// journey flies exactly as before.
    Targeted,
}

/// Position and radius of a parent-cell sibling marker in open-cell units.
///
/// The open cell sits at `entered.position` in parent units with size ratio
/// `entered.ratio`, so a sibling at `sibling` lands at the inverse-rotated
/// `(sibling - position) / ratio` (identity for every level but L11, whose
/// ground arrives tangent to its planet patch, #394) and every sibling has
/// radius 0.5.
#[must_use]
pub fn sibling_in_open_units(entered: Opened, sibling: ParentUnits) -> (OpenUnits, f64) {
    use crate::frame::parent_to_child;
    let relative = [
        sibling.0[0] - entered.position[0],
        sibling.0[1] - entered.position[1],
        sibling.0[2] - entered.position[2],
    ];
    let turned = parent_to_child(entered.up, relative);
    (
        OpenUnits([
            turned[0] / entered.ratio,
            turned[1] / entered.ratio,
            turned[2] / entered.ratio,
        ]),
        0.5,
    )
}

/// Planet context for an open tail cell, in open-cell units (#394).
///
/// Regenerated from the L10 ancestor whenever the path changes: the planet
/// radius at this level's scale, the air thickness at this scale (zero when
/// airless), and the air tint. Levels L1-L10 and missing ancestors yield
/// `None`. Pure data derived from the seed and the path; rendering maps it
/// to horizon rings and sky arcs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceContext {
    /// Planet radius in open-cell units.
    pub planet_radius: f64,
    /// Air thickness in open-cell units (`0.0` when airless).
    pub air_thickness: f64,
    /// Air tint, or `None` when airless.
    pub air_tint: Option<[f64; 3]>,
}

/// Planet context for `path` at its open level, if it sits in the tail.
///
/// Returns `None` outside L11-L14, when the named chain is too short for
/// an L10 ancestor, or when that ancestor holds no surface sample. The
/// radius comes from the ladder ratios, the air from the ancestor's first
/// surface point, scaled into open-cell units (#394).
#[must_use]
pub fn surface_context_for(root: u64, path: &MarkerPath) -> Option<SurfaceContext> {
    use crate::frame::planet_radius_cells;
    use crate::terrain::EARTH_RADIUS_KM;
    let level = path.level();
    if level.get() < 11 || level.get() > 14 {
        return None;
    }
    let indices = path.indices();
    if indices.len() < 9 {
        return None;
    }
    let l10_chain = indices.get(..9)?;
    let l10 = generate_cell(root, l10_chain);
    let surface = l10.points.iter().find_map(|point| point.surface)?;
    let planet_radius = planet_radius_cells(level)?;
    if !planet_radius.is_finite() || planet_radius <= 0.0 {
        return None;
    }
    let (air_thickness, air_tint) = match surface.air {
        None => (0.0, None),
        Some(air) => {
            if !surface.radius_earth.is_finite() || surface.radius_earth <= 0.0 {
                (0.0, None)
            } else {
                let planet_radius_km = surface.radius_earth * EARTH_RADIUS_KM;
                if !planet_radius_km.is_finite() || planet_radius_km <= 0.0 {
                    (0.0, None)
                } else {
                    let thickness = air.thickness_km / planet_radius_km * planet_radius;
                    if thickness.is_finite() && thickness > 0.0 {
                        (thickness, Some(air.tint))
                    } else {
                        (0.0, None)
                    }
                }
            }
        }
    };
    Some(SurfaceContext {
        planet_radius,
        air_thickness,
        air_tint,
    })
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
    /// Planet context for open tail levels, if an L10 ancestor exists.
    pub surface: Option<SurfaceContext>,
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
            surface: None,
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
        self.surface = surface_context_for(self.root.0, &self.path);
    }

    /// Level of the open cell.
    #[must_use]
    pub fn level(&self) -> Level {
        self.path.level()
    }

    /// Planet context for the open tail cell, if an L10 ancestor exists.
    ///
    /// `None` outside L11-L14 or when the L10 ancestor holds no surface.
    /// Regenerated in [`Universe::reload`], so drawing reads it per frame
    /// without regenerating cells (`E-HOT-NOALLOC`).
    #[must_use]
    pub fn surface_context(&self) -> Option<SurfaceContext> {
        self.surface
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

    /// Portal position of marker `marker` in the open cell.
    ///
    /// Dives aim here and openings store this; the body's drawn position
    /// stays on the point itself (#384).
    #[must_use]
    pub fn marker(&self, marker: MarkerIndex) -> Option<[f64; 3]> {
        marker_position(&self.open, marker.0)
    }

    /// Opens `marker`: the camera stays put while the origin descends.
    ///
    /// Returns `false`, untouched, when the marker is missing, is a
    /// population point, or the level cannot open. The stored position is
    /// the portal position, so closing returns to the portal (#384); L10
    /// region patches orient the region's ground tangent to the planet
    /// (#394).
    pub fn open(&mut self, marker: MarkerIndex) -> bool {
        let Some(point) = self.open.points.get(marker.0 as usize) else {
            return false;
        };
        if point.kind != MarkerKind::Portal {
            return false;
        }
        let position = point.portal_position();
        let up = if self.level().get() == 10 {
            match point.form {
                crate::r#gen::Form::Patch { normal } => normal,
                _ => crate::frame::IDENTITY_UP,
            }
        } else {
            crate::frame::IDENTITY_UP
        };
        if !self.path.open_oriented(marker.0, position, up) {
            return false;
        }
        self.reload();
        true
    }

    /// Closes the open cell back into the marker it came from.
    ///
    /// Milestone pops regenerate nothing: the named chain is unchanged, so
    /// the content is byte-identical without rebuilding it.
    pub fn close(&mut self) -> Option<Opened> {
        let opened = self.path.close()?;
        if !opened.anonymous {
            self.reload();
        }
        Some(opened)
    }

    /// Autopilot's marker for the open cell (seeded, fixed per run).
    ///
    /// Chosen among portal markers only (populations never open), following
    /// the level pick rules in [`autopilot_candidates`].
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
    /// Pass-through (#152): in [`DiveMode::Passing`] every crossed portal
    /// opens in along-ray order while the caller's stored target is never
    /// touched (it arrives as a parameter and is only read). In
    /// [`DiveMode::Targeted`] only the stored target opens, so the fixed
    /// seeded journey flies exactly as before.
    ///
    /// Returns the navigation event that happened, so callers (window and
    /// `--verify`) can re-target identically. The open cell closes on the
    /// angular rule alone ([`should_close`]), whose gap below the open
    /// angle is what keeps small moves after entry from closing it (#403).
    pub fn dive(&mut self, target: Option<u32>, factor: f64, mode: DiveMode) -> DiveEvent {
        let from = OpenUnits(self.path.offset());
        let (center, radius) = match target.and_then(|m| self.marker(MarkerIndex(m))) {
            Some(position) => (OpenUnits(position), self.marker_radius()),
            None => (OpenUnits([0.0; 3]), 0.0),
        };
        let mut next = dive_step(from, center, radius, factor);
        if self.path.is_at_root() {
            let distance = length3(next.0);
            if distance > ROOT_MAX_DISTANCE {
                next = OpenUnits(mul3(next.0, ROOT_MAX_DISTANCE / distance));
            }
        }
        self.path.set_offset(next.0);
        // No stored target means a center dive: nothing opens, as before.
        // With a target, the span run-up follows the stored target while
        // the opening itself goes to the nearest crossed portal in ray
        // order (pass-through, #152). The stored value is only read.
        if target.is_some() && self.path.can_open() {
            let radius = self.marker_radius();
            if let Some(marker) = target
                && let Some(position) = self.marker(MarkerIndex(marker))
                && self
                    .open
                    .points
                    .get(marker as usize)
                    .is_some_and(|point| point.kind == MarkerKind::Portal)
            {
                let distance = length3(sub3(next.0, position));
                push_span_milestones(&mut self.path, marker, radius, distance);
            }
            let mut order = crossed_portals(from.0, next.0, &self.open, radius);
            if mode == DiveMode::Targeted {
                order.retain(|&marker| Some(marker) == target);
            }
            // `push_span_milestones` is idempotent per state, so repeating
            // it for the chosen marker is a no-op when it is the target.
            if let Some(&chosen) = order.first()
                && let Some(position) = self.marker(MarkerIndex(chosen))
            {
                let distance = length3(sub3(next.0, position));
                self.path.clear_foreign_milestones(chosen);
                push_span_milestones(&mut self.path, chosen, radius, distance);
                if should_open(radius, distance) && self.open(MarkerIndex(chosen)) {
                    return DiveEvent::Opened(chosen);
                }
            }
        }
        if should_close(self.path.distance_to_center()) {
            // Pop exactly one entry; milestones unwind silently (their content
            // never changed, so no regeneration) and never report an event.
            match self.close() {
                Some(opened) if !opened.anonymous => return DiveEvent::Closed(opened),
                _ => {}
            }
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
    /// Cells live in the streaming working set in the frame before opening
    /// (open-chain prefixes plus preview chains, at most `STREAM_CAP`).
    pub alive: usize,
    /// Magnification milestones crossed on this span (#151 T4 diagnostics).
    pub anon_depth: usize,
}

/// Replays the Spacebar journey headlessly at `dt` seconds per step.
///
/// Same math as the window: target the seeded marker, step with
/// [`AUTOPILOT_RATE`], open at [`OPEN_ANGLE`], repeat until L14 or
/// `max_secs`. The autopilot re-picks the seeded marker per cell (the fixed
/// journey, #152 AC5); the stored-target preservation applies to manual
/// dives, whose target arrives by value and is only read. Returns the
/// opened levels in order and the final universe.
#[must_use]
pub fn replay_autopilot(root: u64, dt: f64, max_secs: f64) -> (Vec<JourneyStep>, Universe) {
    let mut universe = Universe::new(root);
    let mut previews = PreviewCache::default();
    let mut ledger = StreamLedger::default();
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
        // Working-set accounting for the bound proof (#152): every
        // open-chain prefix plus one chain per previewed marker.
        let chains = working_chains(&universe.path, &previews);
        ledger.sync(root, &chains);
        let alive = ledger.len();
        let previewed_before = target
            .and_then(|m| {
                previews
                    .get(m)
                    .map(|c| preview_positions(&universe, MarkerIndex(m), c))
            })
            .unwrap_or_default();
        elapsed += dt;
        let event = universe.dive(target, factor, DiveMode::Targeted);
        match event {
            DiveEvent::Opened(marker) => {
                target = None;
                let opened = universe.path.entered().unwrap_or(Opened {
                    marker,
                    position: [0.0; 3],
                    ratio: 1.0,
                    anonymous: false,
                    up: crate::frame::IDENTITY_UP,
                });
                steps.push(JourneyStep {
                    level: universe.level(),
                    marker,
                    elapsed,
                    snapshot: universe.snapshot(),
                    preview_count: previews.len(),
                    preview_error: preview_error(&previewed_before, opened, &universe.open),
                    alive,
                    anon_depth: universe.path.anonymous_depth(),
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
    use crate::stream::STREAM_CAP;

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
    fn small_moves_after_entry_stay_inside_and_the_slack_is_the_same_for_keys_and_wheel() {
        // Entry rests at 0.5 / sin(OPEN_ANGLE) child units and the cell
        // closes past 0.5 / sin(CLOSE_ANGLE); the slack between them is a
        // pure distance ratio, so wheel notches and held-key frames spend
        // it identically (#403 AC3, replacing the dive-counted settle).
        let slack = OPEN_ANGLE.sin() / CLOSE_ANGLE.sin();
        assert!(
            slack > 1.0 / WHEEL_FACTOR.powi(3),
            "entry must survive three outward wheel notches"
        );
        let mut universe = Universe::new(DEMO_SEED);
        let marker = universe.autopilot_target().expect("root has markers");
        let mut event = DiveEvent::Moved;
        for _ in 0..500 {
            event = universe.dive(Some(marker), WHEEL_FACTOR, DiveMode::Targeted);
            if event != DiveEvent::Moved {
                break;
            }
        }
        assert_eq!(event, DiveEvent::Opened(marker));
        let rest = universe.path.distance_to_center();
        for notch in 1..=3 {
            assert_eq!(
                universe.dive(None, 1.0 / WHEEL_FACTOR, DiveMode::Targeted),
                DiveEvent::Moved,
                "outward wheel notch {notch} must stay inside"
            );
        }
        // The same slack spent by a held key: 1/60 s frames at KEY_RATE.
        let mut again = Universe::new(DEMO_SEED);
        for _ in 0..500 {
            if again.dive(Some(marker), WHEEL_FACTOR, DiveMode::Targeted) != DiveEvent::Moved {
                break;
            }
        }
        assert_eq!(again.level().get(), 2);
        assert_eq!(
            again.path.distance_to_center(),
            rest,
            "same journey, same rest"
        );
        let per_frame = KEY_RATE / 60.0;
        let frame = per_frame.exp();
        let mut frames = 0u32;
        while again.dive(None, frame, DiveMode::Targeted) == DiveEvent::Moved {
            frames += 1;
            assert!(frames < 10_000, "a held key must eventually close");
        }
        // Moved frames before the close: the log-distance from the rest to
        // the close distance, spent at KEY_RATE per second.
        let close_distance = 0.5 / CLOSE_ANGLE.sin();
        let expected = ((close_distance / rest).ln() / per_frame).floor();
        assert!(
            (f64::from(frames) - expected).abs() <= 1.0,
            "held-key slack {frames} frames must match the distance slack ({expected})"
        );
        // A deliberate long way out still closes at the entered marker
        // (#403 AC4).
        let mut closed = None;
        for _ in 0..500 {
            if let DiveEvent::Closed(opened) =
                universe.dive(None, 1.0 / WHEEL_FACTOR, DiveMode::Targeted)
            {
                closed = Some(opened);
                break;
            }
        }
        assert_eq!(closed.map(|o| o.marker), Some(marker));
        assert_eq!(universe.level(), Level::MIN);
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
            event = universe.dive(target, WHEEL_FACTOR, DiveMode::Targeted);
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
            if let DiveEvent::Closed(opened) =
                universe.dive(target, 1.0 / WHEEL_FACTOR, DiveMode::Targeted)
            {
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
    fn autopilot_replay_reaches_l14_deterministically() {
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
            assert!(step.alive <= STREAM_CAP, "working set over the cap");
            assert!(
                step.preview_error <= 1e-9,
                "preview drifted: {}",
                step.preview_error
            );
        }
        let (again, _) = replay_autopilot(DEMO_SEED, 1.0 / 60.0, 600.0);
        assert_eq!(steps, again, "journey must be the same every run");
        assert!(
            steps.last().expect("steps").elapsed < 120.0,
            "journey too long"
        );
    }

    #[test]
    fn crossed_portals_walk_the_ray_in_order_without_populations() {
        use crate::r#gen::Point;
        let open = Generated {
            points: vec![
                Point::bare([0.7, 0.0, 0.0], 0.008, MarkerKind::Portal),
                Point::bare([0.6, 0.0, 0.0], 0.008, MarkerKind::Population),
                Point::bare([0.45, 0.0, 0.0], 0.008, MarkerKind::Portal),
                Point::bare([0.0, 0.4, 0.0], 0.008, MarkerKind::Portal),
            ],
            child_constraints: Vec::new(),
        };
        let order = crossed_portals([2.0, 0.0, 0.0], [0.5, 0.0, 0.0], &open, 0.05);
        assert_eq!(order, vec![0, 2], "ray order, populations excluded");
        assert!(
            crossed_portals([0.5, 0.0, 0.0], [0.5, 0.0, 0.0], &open, 0.05).is_empty(),
            "a still camera crosses nothing"
        );
    }

    #[test]
    fn the_open_cell_closes_on_the_angular_rule_alone() {
        // Past the old horizon floor (4.0) but inside the angular close
        // distance a still camera stays in its cell (#403): no second
        // close currency fights the dive.
        let mut universe = Universe::new(DEMO_SEED);
        let marker = universe.autopilot_target().expect("marker");
        assert!(universe.open(MarkerIndex(marker)), "open L2 directly");
        universe.path.set_offset([4.5, 0.0, 0.0]);
        assert_eq!(
            universe.dive(None, 1.0, DiveMode::Passing),
            DiveEvent::Moved,
            "inside the angular close distance nothing closes"
        );
        assert_eq!(universe.level().get(), 2);
        universe
            .path
            .set_offset([0.5 / CLOSE_ANGLE.sin() * 1.01, 0.0, 0.0]);
        match universe.dive(None, 1.0, DiveMode::Passing) {
            DiveEvent::Closed(opened) => assert_eq!(opened.marker, marker),
            DiveEvent::Moved => panic!("the angular rule should have closed the cell"),
            DiveEvent::Opened(_) => panic!("nothing to open while backing out"),
        }
        assert_eq!(universe.level(), Level::MIN);
    }

    #[test]
    fn passthrough_opens_the_crossed_portal_first() {
        // Forced crossing (#152): the camera starts on the far side of one
        // portal, aimed at another, so the segment crosses it exactly. The
        // crossed portal opens first while the stored target value is never
        // touched (it arrives by value and is only read).
        let mut universe = Universe::new(DEMO_SEED);
        let portals: Vec<(u32, [f64; 3])> = universe
            .open
            .points
            .iter()
            .enumerate()
            .filter(|(_, point)| point.kind == MarkerKind::Portal)
            .map(|(index, point)| {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "E-CAST: marker index into a budgeted cell, always fits u32"
                )]
                let marker = index as u32;
                (marker, point.portal_position())
            })
            .collect();
        assert!(portals.len() >= 2, "root cell needs two portals");
        let (crossed, crossed_pos) = portals[0];
        let (target, target_pos) = portals[1];
        let mut axis = [
            target_pos[0] - crossed_pos[0],
            target_pos[1] - crossed_pos[1],
            target_pos[2] - crossed_pos[2],
        ];
        let length = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        assert!(length > 0.0, "portals must not coincide");
        axis = [axis[0] / length, axis[1] / length, axis[2] / length];
        universe.path.set_offset([
            crossed_pos[0] - axis[0] * 2.0,
            crossed_pos[1] - axis[1] * 2.0,
            crossed_pos[2] - axis[2] * 2.0,
        ]);
        let mut opened = None;
        for _ in 0..2000 {
            match universe.dive(Some(target), WHEEL_FACTOR, DiveMode::Passing) {
                DiveEvent::Opened(m) => {
                    opened = Some(m);
                    break;
                }
                DiveEvent::Closed(_) => panic!("backed out while approaching"),
                DiveEvent::Moved => {}
            }
        }
        assert_eq!(opened, Some(crossed), "the crossed portal opens first");
        assert_eq!(universe.level().get(), 2);
    }

    #[test]
    fn root_cannot_rise_past_cap_or_close() {
        let mut universe = Universe::new(DEMO_SEED);
        for _ in 0..100 {
            assert_eq!(
                universe.dive(None, 1.0 / WHEEL_FACTOR, DiveMode::Targeted),
                DiveEvent::Moved
            );
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
            anonymous: false,
            up: crate::frame::IDENTITY_UP,
        };
        let (position, radius) = sibling_in_open_units(entered, ParentUnits([0.2, 0.0, 0.0]));
        assert!((position.0[0] - 10.0).abs() < 1e-9);
        assert_eq!(radius, 0.5);
        assert!(drawn_radius(1e-6, 10.0) > 1e-6);
        assert_eq!(drawn_radius(1.0, 10.0), 1.0);
    }

    #[test]
    fn surface_context_follows_the_l10_ancestor() {
        use crate::nest::autopilot_path;
        let chain = autopilot_path(DEMO_SEED);
        // L1-L10 hold no context.
        let mut universe = Universe::new(DEMO_SEED);
        assert!(universe.surface_context().is_none());
        // Opening to L11-L14 yields a context with a positive radius.
        for &marker in &chain {
            assert!(universe.open(MarkerIndex(marker)), "journey must open");
            let n = universe.level().get();
            if (11..=14).contains(&n) {
                let context = universe
                    .surface_context()
                    .expect("tail levels hold context");
                assert!(context.planet_radius > 0.0);
                if context.air_tint.is_some() {
                    assert!(context.air_thickness > 0.0);
                } else {
                    assert_eq!(context.air_thickness, 0.0);
                }
            }
            if n >= 12 {
                break;
            }
        }
        assert!(universe.level().get() >= 12);
    }
}
