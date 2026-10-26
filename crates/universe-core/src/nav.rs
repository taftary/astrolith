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

/// Deepest open level: L10 (planet, #729). Its markers do not open.
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
/// The true child size (`docs/universes/ladder.md` ratios); L10 falls back to
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
    should_close_with_radius(distance_to_center, 0.5)
}

/// Returns `true` when an open cell of `radius` at `distance_to_center`
/// should close (#775, Q7 A).
///
/// At L10 the cell radius is `max(0.5, planet radius)`, so Earth and small
/// planets close as today while giants close farther out.
#[must_use]
pub fn should_close_with_radius(distance_to_center: f64, radius: f64) -> bool {
    angular_radius(radius, distance_to_center) < CLOSE_ANGLE
}

/// The planet's own space at L10 (Q7 A, #775).
///
/// The planet's size sets its space, not the cell: arrival sits above the
/// true surface at `3 x` the radius, the dive closes far away relative to
/// the planet, and the headless in-sphere check measures against the reach.
/// Earth's journey entry (~3.58 cells) already sits above `3 x`, so it is
/// unchanged (review item 4: no entry floor, so off-centre window entries
/// never jump outward).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanetSpace {
    /// Planet radius in L10 open-cell units (true radius from its record).
    pub radius_cells: f64,
    /// Arrival floor from the planet centre: `3 x` the radius.
    pub arrival: f64,
    /// Headless in-sphere radius at L10: `max(0.5, 4 x radius)`.
    pub reach: f64,
}

/// Entry rest distance in open-cell units: `0.5 / sin(OPEN_ANGLE)` (~3.58).
#[must_use]
pub fn l10_entry_rest() -> f64 {
    0.5 / OPEN_ANGLE.sin()
}

/// The planet's space for an L10 chain under `root` (Q7 A).
///
/// Returns `None` unless `chain` reaches L10 (9 markers). Pure and
/// deterministic.
#[must_use]
pub fn planet_space_for_chain(root: u64, chain: &[u32]) -> Option<PlanetSpace> {
    if crate::nest::path_level(chain).get() != 10 {
        return None;
    }
    if chain.len() != 9 {
        return None;
    }
    let frame = crate::planet_frame::PlanetFrame::for_planet(root, chain);
    let radius_cells = frame.radius_cells();
    if !(radius_cells.is_finite() && radius_cells > 0.0) {
        return None;
    }
    // Review item 4: the floor is `3 x` the radius only. Earth's journey
    // entry (~3.58) already clears it, so the journey is unchanged and
    // off-centre window entries never jump out to the old entry rest.
    let arrival = 3.0 * radius_cells;
    let reach = 0.5f64.max(4.0 * radius_cells);
    if !(arrival.is_finite() && reach.is_finite()) {
        return None;
    }
    Some(PlanetSpace {
        radius_cells,
        arrival,
        reach,
    })
}

/// The planet's space for the open L10 cell, if any (Q7 A).
///
/// Cached in [`Universe::reload`]; no allocation on the dive path.
#[must_use]
pub fn planet_space(universe: &Universe) -> Option<PlanetSpace> {
    universe.cached_space
}

/// The planet frame for the open L10 cell, if any (#873 T4).
///
/// Cached in [`Universe::reload`]; no allocation or rebuild on the dive
/// path.
#[must_use]
pub fn planet_frame(universe: &Universe) -> Option<crate::planet_frame::PlanetFrame> {
    universe.cached_frame
}

/// The relief for the open L10 cell, if any (#873 T4).
///
/// Cached in [`Universe::reload`]; no allocation or rebuild on the dive
/// path.
#[must_use]
pub fn planet_relief(universe: &Universe) -> Option<&crate::relief::Relief> {
    universe.cached_relief.as_ref()
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
    /// Autopilot and window dives: only the stored target opens, so the fixed
    /// seeded journey flies exactly as before (#770: landing bend removed).
    Targeted,
    /// The journey (autopilot): [`DiveMode::Targeted`], and in a planetary
    /// system it flies to the planet's morning side first, so it arrives in
    /// daylight with no turn at the open (#995 T8, #780 F1 A).
    Journey,
}

/// Position and radius of a parent-cell sibling marker in open-cell units.
///
/// The open cell sits at `entered.position` in parent units with size ratio
/// `entered.ratio`, so a sibling at `sibling` lands at the inverse-rotated
/// `(sibling - position) / ratio` and every sibling has radius 0.5.
#[must_use]
pub fn sibling_in_open_units(entered: Opened, sibling: ParentUnits) -> (OpenUnits, f64) {
    (OpenUnits(entered.to_child_point(sibling.0)), 0.5)
}

/// A frame is left only past this multiple of the radius it is entered at
/// (#995, the study's band).
pub const ZONE_LEAVE_FACTOR: f64 = crate::system_handover::LEAVE_FACTOR;

/// A planet's gravity zone (sphere of influence, Laplace radius) around
/// `star`, in metres (#995 T8).
#[must_use]
pub fn planet_zone_m(planet: crate::r#gen::PlanetInfo, star: crate::r#gen::StarInfo) -> f64 {
    crate::system_space::laplace_radius_m(
        planet.orbit_au * crate::system_space::AU_M,
        crate::gravity::planet_mass_kg(planet.radius_earth),
        star.mass_solar * crate::gravity::SOLAR_MASS_KG,
    )
}

/// The gravity zone of the L10 planet at `chain` under `root`, in L10 cell
/// units (#995 T9): its moons live inside it.
#[must_use]
pub fn planet_zone_cells_for_chain(root: u64, chain: &[u32]) -> Option<f64> {
    if crate::nest::path_level(chain).get() != 10 {
        return None;
    }
    let (marker, close_up) = chain.split_last()?;
    let system = close_up.split_last()?.1;
    let star = generate_cell(root, system).points.first()?.star?;
    let planet = generate_cell(root, close_up)
        .points
        .get(*marker as usize)?
        .planet?;
    let frame = crate::planet_frame::PlanetFrame::for_planet(root, chain);
    Some(planet_zone_m(planet, star) / frame.metres_per_cell())
}

/// The gravity zone of L8 planet `marker` in L8 cell units, when the open
/// cell is a planetary system and `marker` is one of its planets (#995 T8).
#[must_use]
pub fn l8_planet_zone(universe: &Universe, marker: u32) -> Option<f64> {
    if universe.level().get() != 8 {
        return None;
    }
    let planet = universe.open.points.get(marker as usize)?.planet?;
    let star = universe.open.points.first()?.star?;
    Some(planet_zone_m(planet, star) / crate::orbits::metres_per_cell(8))
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
    /// Cached open-cell seed, refreshed in [`Universe::reload`] (review item
    /// 5: [`Universe::open_seed`] must not allocate per frame,
    /// `E-HOT-NOALLOC`).
    cached_seed: u64,
    /// Cached planet space at L10, refreshed in [`Universe::reload`]
    /// (review item 5: the dive close check must not allocate per step).
    cached_space: Option<PlanetSpace>,
    /// Cached planet frame at L10, refreshed in [`Universe::reload`] (#873
    /// T4: the ground floor must not allocate or rebuild per step,
    /// `E-HOT-NOALLOC`).
    cached_frame: Option<crate::planet_frame::PlanetFrame>,
    /// Cached relief at L10, refreshed in [`Universe::reload`] (#873 T4).
    cached_relief: Option<crate::relief::Relief>,
    /// Cached gravity zone of the open L10 planet in L10 cell units
    /// (#995 T8), refreshed in [`Universe::reload`].
    cached_zone: Option<f64>,
    /// Game time of the sky in seconds (#981): every moving body's place.
    pub(crate) time_s: f64,
    /// The open and parent cells' moving bodies, rebuilt in
    /// [`Universe::reload`] (#981, `E-HOT-NOALLOC`: never per frame).
    pub(crate) movers: Vec<crate::sky_motion::Mover>,
    /// The open planet's day in hours at L10 (#981 T3), else `None`.
    pub(crate) spin_hours: Option<f64>,
    /// The open planet's unit spin axis (#981 T3: its tilt), `+Y` when
    /// none.
    pub(crate) spin_axis: [f64; 3],
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
            cached_seed: path_seed(root, &[]),
            cached_space: None,
            cached_frame: None,
            cached_relief: None,
            cached_zone: None,
            time_s: 0.0,
            movers: Vec::new(),
            spin_hours: None,
            spin_axis: crate::frame::IDENTITY_UP,
        };
        universe.reload();
        universe
    }

    /// Regenerates the open and parent cells from the current path.
    ///
    /// Refreshes the cached seed and planet space from the same indices, so
    /// per-frame readers never allocate. Opens and closes are rare; the one
    /// allocation here is not per frame.
    pub fn reload(&mut self) {
        let mut parent_indices = self.path.indices();
        self.open = generate_cell(self.root.0, &parent_indices);
        self.parent = parent_indices
            .pop()
            .map(|_| generate_cell(self.root.0, &parent_indices));
        let indices = self.path.indices();
        self.cached_seed = path_seed(self.root.0, &indices);
        self.cached_space = if self.path.level().get() == 10 {
            planet_space_for_chain(self.root.0, &indices)
        } else {
            None
        };
        // #873 T4: cache the frame and relief with the space, so the ground
        // floor never rebuilds per step. Opens are rare; the work here is
        // not per frame.
        if self.path.level().get() == 10 && indices.len() == 9 {
            let frame = crate::planet_frame::PlanetFrame::for_planet(self.root.0, &indices);
            let relief = crate::relief::Relief::for_planet(self.root.0, &indices);
            self.cached_frame = Some(frame);
            self.cached_relief = Some(relief);
        } else {
            self.cached_frame = None;
            self.cached_relief = None;
        }
        self.cached_zone = if self.path.level().get() == 10 {
            planet_zone_cells_for_chain(self.root.0, &indices)
        } else {
            None
        };
        // #981: the regenerated cells sit at time zero; move them to now.
        crate::sky_motion::collect_movers(self);
        self.apply_time(true);
    }

    /// Level of the open cell.
    #[must_use]
    pub fn level(&self) -> Level {
        self.path.level()
    }

    /// Seed of the open cell (cached in [`Universe::reload`], no allocation).
    #[must_use]
    pub fn open_seed(&self) -> u64 {
        self.cached_seed
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
    /// (#394). At L10 the entry offset is pushed radially out to the
    /// planet's arrival when closer (Q7 A, #775), and turned around the
    /// planet to its morning side, the same distance out, so you arrive in
    /// daylight (#780 F1 A, [`crate::player::MORNING_RAD`]).
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
        // Q7 A + review item 4: the planet's own size sets its space. Push out
        // only when inside `3 x` the radius; Earth's journey entry already
        // clears it, so the journey is unchanged and off-centre entries never
        // jump. Computed from the new chain (opens are rare, so the one
        // allocation is not per frame); `reload` below refreshes the cache.
        if self.level().get() == 10 {
            let indices = self.path.indices();
            if let Some(space) = planet_space_for_chain(self.root.0, &indices) {
                let d = self.path.distance_to_center();
                if d.is_finite() && d > 0.0 && d < space.arrival {
                    let s = space.arrival / d;
                    let off = self.path.offset();
                    self.path.set_offset([off[0] * s, off[1] * s, off[2] * s]);
                }
            }
        }
        self.reload();
        if self.level().get() == 10
            && let Some(sun) = crate::light::sun_direction(self)
        {
            let d = self.path.distance_to_center();
            if d.is_finite() && d > 0.0 {
                let m = crate::light::from_sun(sun, crate::player::MORNING_RAD);
                self.path.set_offset([m[0] * d, m[1] * d, m[2] * d]);
            }
        }
        true
    }

    /// Enters L8 planet `marker` from its gravity zone (#995 T8): the star
    /// close-up (L9) and the planet (L10) open in one step, both exact
    /// origin shifts, so the camera stays where it is and nothing jumps. No
    /// arrival push and no turn to the morning side: the camera keeps its
    /// place.
    pub fn open_planet(&mut self, marker: u32) -> bool {
        if l8_planet_zone(self, marker).is_none() || marker == 0 {
            return false;
        }
        let Some(star) = self.open.points.first().map(|p| p.portal_position()) else {
            return false;
        };
        let before = self.path.clone();
        if !self.path.open_oriented(0, star, crate::frame::IDENTITY_UP) {
            return false;
        }
        self.reload();
        let inner = marker - 1;
        let Some(position) = self
            .open
            .points
            .get(inner as usize)
            .map(|p| p.portal_position())
        else {
            self.path = before;
            self.reload();
            return false;
        };
        if !self
            .path
            .open_oriented(inner, position, crate::frame::IDENTITY_UP)
        {
            self.path = before;
            self.reload();
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

    /// Where the journey dives in the open cell (#995 T8): in a planetary
    /// system the journey's planet (the star close-up is passed on the way
    /// in, from the planet's gravity zone), elsewhere
    /// [`Universe::autopilot_target`]. The journey's planet is the close-up's
    /// autopilot pick, one up as an L8 marker.
    #[must_use]
    pub fn journey_target(&self) -> Option<u32> {
        if self.level().get() != 8 {
            return self.autopilot_target();
        }
        let mut chain = self.path.indices();
        chain.push(0);
        let close_up = generate_cell(self.root.0, &chain);
        let seed = path_seed(self.root.0, &chain);
        let level = crate::nest::path_level(&chain);
        let candidates = autopilot_candidates(level, seed, &close_up.points);
        autopilot_marker(seed, &candidates).map(|m| m + 1)
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
        // #915 R1: maximum dive speed. One call moves at most one wheel
        // notch, so a fast scroll crosses one boundary per frame and lands
        // near it instead of jumping far. Held keys, the autopilot and
        // single notches sit inside the range and keep today's path.
        // Small autopilot steps (about 2 percent) never reach the ease
        // below, so the journey and goldens hold (`E-GOLDEN`).
        let factor = if factor.is_finite() {
            factor.clamp(WHEEL_FACTOR, 1.0 / WHEEL_FACTOR)
        } else {
            factor
        };
        // #995 T8: in a planetary system a planet target is a true-size body
        // that opens from its gravity zone, and nothing opens by angle.
        let zone = target.and_then(|m| l8_planet_zone(self, m));
        let in_system = self.level().get() == 8;
        let (center, radius) = match target.and_then(|m| self.marker(MarkerIndex(m))) {
            Some(position) if in_system => (
                OpenUnits(position),
                target
                    .and_then(|m| self.open.points.get(m as usize))
                    .map_or(self.marker_radius(), |p| p.radius),
            ),
            Some(position) => (OpenUnits(position), self.marker_radius()),
            None => (OpenUnits([0.0; 3]), 0.0),
        };
        // #995 T8 (#780 F1 A): the journey still arrives in daylight. With no
        // turn at the open, the autopilot first flies to a waypoint on the
        // planet's morning side, twice its gravity zone out, then straight
        // down once lined up with it (within about 11 degrees), so it enters
        // the zone and lands on the morning side.
        let (center, radius) = match (mode, zone) {
            (DiveMode::Journey, Some(zone)) => {
                let to_star = sub3([0.0; 3], center.0);
                let sun = div3(to_star, length3(to_star).max(f64::MIN_POSITIVE));
                let morning = crate::light::from_sun(sun, crate::player::MORNING_RAD);
                let waypoint = add3(center.0, mul3(morning, 2.0 * zone));
                let out = sub3(from.0, center.0);
                let lined_up = dot3(out, morning) > 0.98 * length3(out);
                if !lined_up && length3(out) > zone {
                    (OpenUnits(waypoint), 0.0)
                } else {
                    (center, radius)
                }
            }
            _ => (center, radius),
        };
        // At L10 the planet (its frame and relief) decides the ground.
        let ground = if self.level().get() == 10 {
            planet_frame(self).zip(planet_relief(self))
        } else {
            None
        };
        // #892 T1 (Q3 B): diving toward the planet's centre scales the
        // height above the real ground, so the dive slows near it and rests
        // at the 3 m stop. Other targets keep the marker-surface step.
        let mut next = match (target, ground) {
            (None, Some((frame, relief))) => {
                let ground_r = crate::dive_orbit::ground_radius_cells(from.0, &frame, relief);
                let floor = crate::dive_orbit::GROUND_CLEARANCE_M / frame.metres_per_cell();
                if ground_r > 0.0 {
                    OpenUnits(crate::dive_orbit::ground_dive_step(
                        from.0, ground_r, factor, floor,
                    ))
                } else {
                    dive_step(from, center, radius, factor)
                }
            }
            _ => dive_step(from, center, radius, factor),
        };
        // #873 T3 (Q3 A), #892 T1 (Q2 A): never enter the L10 planet's core;
        // the avoid ball sits below the lowest ground (no sea level), and the
        // clamp below does the real stop above the ground.
        if let Some((frame, relief)) = ground {
            let avoid = crate::dive_orbit::lowest_ground_avoid_radius(&frame, relief);
            if avoid > 0.0 {
                next = OpenUnits(crate::dive_orbit::slide_around_sphere(
                    from.0, next.0, [0.0; 3], avoid,
                ));
            }
        }
        // #873 T4 (Q7 A, Q8 A): never closer than 3 m above the drawn
        // terrain at L10; zooming in stops there in dive mode.
        if let Some((frame, relief)) = ground {
            next = OpenUnits(crate::dive_orbit::clamp_above_ground(
                next.0,
                &frame,
                relief,
                crate::dive_orbit::GROUND_CLEARANCE_M,
            ));
        }
        if self.path.is_at_root() {
            let distance = length3(next.0);
            if distance > ROOT_MAX_DISTANCE {
                next = OpenUnits(mul3(next.0, ROOT_MAX_DISTANCE / distance));
            }
        }
        // #915 R1: smoother dimension conversion. A crossing more than 5
        // percent past the boundary lands just past it instead, in both
        // directions, for marker and L10 ground paths, so the remap
        // continues near instead of far away.
        if factor.is_finite() && factor > 1.0 {
            let close_d = match (planet_space(self), self.cached_zone) {
                (_, Some(zone)) if self.level().get() == 10 => zone * ZONE_LEAVE_FACTOR,
                (Some(space), None) if self.level().get() == 10 => {
                    0.5f64.max(space.radius_cells) / CLOSE_ANGLE.sin()
                }
                _ => 0.5 / CLOSE_ANGLE.sin(),
            };
            let current_d = length3(from.0);
            let next_d = length3(next.0);
            if close_d.is_finite() && current_d < close_d && next_d > close_d * 1.05 && next_d > 0.0
            {
                let s = close_d * (1.0 + 1e-6) / next_d;
                next = OpenUnits(mul3(next.0, s));
            }
        }
        if factor.is_finite()
            && factor < 1.0
            && target.is_some()
            && self.path.can_open()
            && !in_system
        {
            let radius = self.marker_radius();
            let open_d = radius / OPEN_ANGLE.sin();
            let current_m = length3(sub3(from.0, center.0));
            let next_m = length3(sub3(next.0, center.0));
            if open_d.is_finite()
                && open_d > 0.0
                && current_m > open_d
                && next_m < open_d * 0.95
                && next_m > 0.0
            {
                let dir = div3(sub3(next.0, center.0), next_m);
                next = OpenUnits(add3(center.0, mul3(dir, open_d * (1.0 - 1e-6))));
            }
        }
        self.path.set_offset(next.0);
        // #995 T8: inside a planet's gravity zone its cell opens (the star
        // close-up and the planet in one exact step).
        if let (Some(marker), Some(zone)) = (target, zone)
            && let Some(planet) = self.marker(MarkerIndex(marker))
            && length3(sub3(next.0, planet)) < zone
            && self.open_planet(marker)
        {
            return DiveEvent::Opened(marker);
        }
        // No stored target means a center dive: nothing opens, as before.
        // With a target, the span run-up follows the stored target while
        // the opening itself goes to the nearest crossed portal in ray
        // order (pass-through, #152). The stored value is only read.
        if target.is_some() && self.path.can_open() && !in_system {
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
            if matches!(mode, DiveMode::Targeted | DiveMode::Journey) {
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
        // #995 T8: past the planet's gravity zone (and its band) the planet
        // and the star close-up close in one exact step, back into the
        // system; the planet becomes the target again.
        if self.level().get() == 10
            && let Some(zone) = self.cached_zone
        {
            if self.path.distance_to_center() > zone * ZONE_LEAVE_FACTOR
                && let Some(planet) = self.close()
                && let Some(star) = self.close()
            {
                // The star close-up's frame is identity: the planet's spin
                // is the whole turn back to the system (#981 T3).
                return DiveEvent::Closed(Opened {
                    marker: planet.marker + 1,
                    spin: planet.spin,
                    axis: planet.axis,
                    ..star
                });
            }
            return DiveEvent::Moved;
        }
        if planet_aware_should_close(self) {
            // Pop exactly one entry; milestones unwind silently (their content
            // never changed, so no regeneration) and never report an event.
            match self.close() {
                Some(opened) if !opened.anonymous => {
                    return DiveEvent::Closed(opened);
                }
                _ => {}
            }
        }
        DiveEvent::Moved
    }
}

/// Planet-aware close rule: at L10 the cell radius is `max(0.5, planet
/// radius)` (Q7 A, #775), so Earth and small planets close as today while
/// giants close farther out. Everywhere else this is [`should_close`].
#[must_use]
fn planet_aware_should_close(universe: &Universe) -> bool {
    let distance = universe.path.distance_to_center();
    if universe.level().get() == 10
        && let Some(space) = planet_space(universe)
    {
        let cell_r = 0.5f64.max(space.radius_cells);
        should_close_with_radius(distance, cell_r)
    } else {
        should_close(distance)
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
/// [`AUTOPILOT_RATE`], open at [`OPEN_ANGLE`], repeat until L10 or
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
            target = universe.journey_target();
            if target.is_none() {
                break;
            }
        }
        previews.sync(&universe);
        // Working-set accounting for the bound proof (#152, #742): every
        // open-chain prefix at zero plus one chain per previewed marker
        // at its camera gap.
        let chains = working_chains(
            &universe.path,
            universe.path.offset(),
            &universe.open.points,
            &previews,
        );
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
        let event = universe.dive(target, factor, DiveMode::Journey);
        match event {
            DiveEvent::Opened(marker) => {
                target = None;
                let opened = universe.path.entered().unwrap_or(Opened {
                    marker,
                    position: [0.0; 3],
                    ratio: 1.0,
                    anonymous: false,
                    up: crate::frame::IDENTITY_UP,
                    spin: 0.0,
                    axis: crate::frame::IDENTITY_UP,
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
#[path = "nav_tests.rs"]
mod tests;
