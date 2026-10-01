//! Bevy indicators for the nested universe: markers, axes, and the dive camera.
//!
//! No meshes, materials, or textures live here by design (spec v3/v4:
//! indicators only). `UniverseRenderPlugin` owns the camera and the axis
//! indicators; `DivePlugin` (R6, sub-issue #60) owns the nested navigation:
//! the open cell is the render origin, its markers are the next dimension,
//! hovering highlights a marker, clicking targets it, the wheel dives with
//! `v = k * h`, the target opens into its interior at `theta_min`, and
//! diving out closes it back into the marker you came from. The largest
//! markers on screen preview their interior before you enter (R7, #63), so
//! opening and closing change nothing on screen. Every system draws with
//! gizmos only.
//!
//! The pure helpers in this module (navigation math, labels, the headless
//! journey replay) need no window, so the `--verify` mode in `universe-app`
//! reuses them to print exactly what the window would do.

use bevy::camera::{Camera, Projection};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::math::{DVec3, Isometry3d, Vec3};
use bevy::prelude::*;
use universe_core::coords::Level;
use universe_core::nest::{
    MAX_OPEN_LEVEL, MarkerPath, Opened, angular_radius, autopilot_marker, child_ratio,
    child_world_position, children_brightness, generate_cell, marker_position, marker_radius,
    path_seed, preview_set, scale_exponent, shell_brightness,
};
pub use universe_core::nest::{CLOSE_ANGLE, OPEN_ANGLE, PREVIEW_ANGLE, PREVIEW_CAP};
use universe_core::r#gen::Generated;
use universe_core::snapshot::snapshot_generated;

/// Fixed demo root seed `sigma_0` for the window and `--verify`.
///
/// Every cell along every path derives from it, so the universe is the same
/// on every run (determinism invariant).
pub const DEMO_SEED: u64 = 42;

/// Shallowest level: L1 (observable universe), the root cell.
pub const MIN_NAV_LEVEL: u8 = 1;

/// Deepest open level: L11 (planets, M5). Its markers do not open.
pub const MAX_NAV_LEVEL: u8 = MAX_OPEN_LEVEL;

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

/// Pick radius in logical pixels for hover and click.
pub const PICK_PIXELS: f32 = 24.0;

/// Short display label for a level, from the frozen ladder.
pub fn level_label(level: Level) -> &'static str {
    match level.get() {
        1 => "L1 - Observable universe",
        2 => "L2 - Cosmic web",
        3 => "L3 - Superclusters",
        4 => "L4 - Galaxy clusters and groups",
        5 => "L5 - Galaxies",
        6 => "L6 - Galactic structures",
        7 => "L7 - Stellar neighborhood",
        8 => "L8 - Outer solar system",
        9 => "L9 - Planetary system",
        10 => "L10 - Stars",
        11 => "L11 - Planets and moons",
        _ => "Beyond MVP",
    }
}

/// Display order label for `level` (e.g. `"10^21 m"`), rounded from the anchor.
pub fn scale_label(level: Level) -> String {
    format!("10^{} m", scale_exponent(level).round() as i32)
}

/// Anchor object behind the level's exponent, for `--verify` output.
pub fn scale_anchor(level: Level) -> &'static str {
    match level.get() {
        1 => "observable universe, 93 Gly across",
        2 => "Sloan Great Wall, 1.37 Gly",
        3 => "Laniakea, 520 Mly across",
        4 => "Virgo Cluster, 15 Mly across",
        5 => "Milky Way, 100 kly across",
        6 => "molecular-cloud complex, 100 pc",
        7 => "Alpha Centauri, 4.37 ly away",
        8 => "Oort cloud edge, 100,000 AU",
        9 => "heliopause, 120 AU",
        10 => "Sun, 1.39e9 m across (IAU)",
        11 => "Earth, 1.28e7 m across",
        _ => "beyond MVP",
    }
}

/// Window title for the open `level`: app name, ladder label, true scale.
pub fn window_title_for_level(level: Level) -> String {
    format!("Universe MVP - {} | {}", level_label(level), scale_label(level))
}

/// Marker color per content era: cyan clusters (L1-L4), warm stars
/// (L5-L10), green terrain (L11).
pub fn point_color_for_level(level: Level) -> Color {
    match level.get() {
        1..=4 => Color::srgb(0.3, 0.8, 1.0),
        5..=10 => Color::srgb(1.0, 0.85, 0.4),
        11 => Color::srgb(0.4, 1.0, 0.5),
        _ => Color::WHITE,
    }
}

/// Dimmed era color for the parent cell's siblings behind the camera.
pub fn sibling_color_for_level(level: Level) -> Color {
    match level.get() {
        1..=4 => Color::srgb(0.12, 0.32, 0.4),
        5..=10 => Color::srgb(0.4, 0.34, 0.16),
        11 => Color::srgb(0.16, 0.4, 0.2),
        _ => Color::srgb(0.4, 0.4, 0.4),
    }
}

/// Radius markers of the open `level` are drawn at, in open-cell units.
///
/// The true child size (`docs/universe/ladder.md` ratios); L15 falls back to
/// the indicator radius since nothing is deeper.
pub fn open_marker_radius(level: Level, indicator_radius: f64) -> f64 {
    marker_radius(level).unwrap_or(indicator_radius)
}

/// Drawn radius after the impostor clamp: at least [`MIN_MARKER_ANGLE`].
pub fn drawn_radius(true_radius: f64, distance: f64) -> f64 {
    true_radius.max(distance * MIN_MARKER_ANGLE.tan())
}

/// One dive step toward (`factor < 1`) or away from (`factor > 1`) a sphere.
///
/// `h` is the gap between the camera and the sphere surface; the step keeps
/// `h' = factor * h` along the line of sight, which is `v = k * h`
/// integrated. The camera never crosses the surface. A camera sitting
/// exactly on the center steps along `+z`.
pub fn dive_step(camera: [f64; 3], center: [f64; 3], radius: f64, factor: f64) -> [f64; 3] {
    let cam = DVec3::from_array(camera);
    let target = DVec3::from_array(center);
    let to_camera = cam - target;
    let distance = to_camera.length();
    let direction = if distance > 0.0 {
        to_camera / distance
    } else {
        DVec3::Z
    };
    let gap = (distance - radius).max(0.0);
    let next_gap = (gap * factor).max(radius * 1e-3);
    (target + direction * (radius + next_gap)).to_array()
}

/// Returns `true` when a marker of `radius` at `distance` should open.
pub fn should_open(radius: f64, distance: f64) -> bool {
    angular_radius(radius, distance) > OPEN_ANGLE
}

/// Returns `true` when the open cell (radius 0.5) at `distance_to_center`
/// should close back into its marker.
pub fn should_close(distance_to_center: f64) -> bool {
    angular_radius(0.5, distance_to_center) < CLOSE_ANGLE
}

/// Position and radius of a parent-cell sibling marker in open-cell units.
///
/// The open cell sits at `entered.position` in parent units with size ratio
/// `entered.ratio`, so a sibling at `sibling` lands at
/// `(sibling - position) / ratio` and every sibling has radius 0.5.
pub fn sibling_in_open_units(entered: Opened, sibling: [f64; 3]) -> ([f64; 3], f64) {
    let mut out = [0.0; 3];
    for axis in 0..3 {
        out[axis] = (sibling[axis] - entered.position[axis]) / entered.ratio;
    }
    (out, 0.5)
}

/// Converts a cell-local `f64` position to a render `Vec3`.
fn to_vec3(position: [f64; 3]) -> Vec3 {
    Vec3::new(position[0] as f32, position[1] as f32, position[2] as f32)
}

/// The nested universe as seen by the observer: the open cell and its parent.
///
/// Exactly two generations live at a time (section 8 bounded memory). Both
/// regenerate from the path when it changes, so nothing is stored.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct Universe {
    /// Root seed every cell derives from.
    pub root: u64,
    /// Observer path and camera offset.
    pub path: MarkerPath,
    /// Content of the open cell.
    pub open: Generated,
    /// Content of the parent cell, if the open cell is not the root.
    pub parent: Option<Generated>,
}

impl Universe {
    /// Starts at the root cell with the camera at [`START_OFFSET`].
    pub fn new(root: u64) -> Universe {
        let mut universe = Universe {
            root,
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
        let indices = self.path.indices();
        self.open = generate_cell(self.root, &indices);
        self.parent = if indices.is_empty() {
            None
        } else {
            Some(generate_cell(self.root, &indices[..indices.len() - 1]))
        };
    }

    /// Level of the open cell.
    pub fn level(&self) -> Level {
        self.path.level()
    }

    /// Seed of the open cell.
    pub fn open_seed(&self) -> u64 {
        path_seed(self.root, &self.path.indices())
    }

    /// Drawn-size radius of the open cell's markers (true child size).
    pub fn marker_radius(&self) -> f64 {
        open_marker_radius(self.level(), 0.01)
    }

    /// Position of marker `marker` in the open cell.
    pub fn marker(&self, marker: u32) -> Option<[f64; 3]> {
        marker_position(&self.open, marker)
    }

    /// Opens `marker`: the camera stays put while the origin descends.
    ///
    /// Returns `false`, untouched, when the marker is missing or the level
    /// cannot open.
    pub fn open(&mut self, marker: u32) -> bool {
        let Some(position) = self.marker(marker) else {
            return false;
        };
        if !self.path.open(marker, position) {
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
    pub fn autopilot_target(&self) -> Option<u32> {
        autopilot_marker(self.open_seed(), self.open.points.len())
    }

    /// Canonical snapshot of the open cell.
    pub fn snapshot(&self) -> String {
        snapshot_generated(&self.open)
    }

    /// Moves the camera one dive step relative to `target` (or the cell
    /// center when `None`), opening or closing as thresholds are crossed.
    ///
    /// Returns the navigation event that happened, so callers (window and
    /// `--verify`) can re-target identically.
    pub fn dive(&mut self, target: Option<u32>, factor: f64) -> DiveEvent {
        let camera = self.path.offset();
        let (center, radius) = match target.and_then(|m| self.marker(m)) {
            Some(position) => (position, self.marker_radius()),
            None => ([0.0; 3], 0.0),
        };
        let mut next = dive_step(camera, center, radius, factor);
        if self.path.chain().is_empty() {
            let distance = DVec3::from_array(next).length();
            if distance > ROOT_MAX_DISTANCE {
                next = (DVec3::from_array(next) * (ROOT_MAX_DISTANCE / distance)).to_array();
            }
        }
        self.path.set_offset(next);
        if let Some(marker) = target
            && self.path.can_open()
            && let Some(position) = self.marker(marker)
        {
            let distance = (DVec3::from_array(next) - DVec3::from_array(position)).length();
            if should_open(self.marker_radius(), distance) && self.open(marker) {
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

/// Scales a color's RGB by `brightness` (alpha untouched).
fn scaled(color: Color, brightness: f32) -> Color {
    let s = color.to_srgba();
    Color::srgb(s.red * brightness, s.green * brightness, s.blue * brightness)
}

/// Interiors of the open cell's largest-on-screen markers, drawn before entry
/// (R7, #63).
///
/// Holds at most [`PREVIEW_CAP`] generations, keyed by the open path and the
/// marker index. [`PreviewCache::sync`] regenerates only when the preview set
/// changes, so the draw path never allocates per frame (section 8 bounded
/// memory: `2 + PREVIEW_CAP` generations alive at most).
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct PreviewCache {
    path: Vec<u32>,
    entries: Vec<(u32, Generated)>,
    /// Total cell generations performed by this cache (test/verify counter).
    pub regenerations: u64,
}

impl PreviewCache {
    /// Brings the cache in line with the camera: returns the current preview
    /// set and `true` when any cell was (re)generated.
    pub fn sync(&mut self, universe: &Universe) -> bool {
        let indices = universe.path.indices();
        if self.path != indices {
            self.path = indices;
            self.entries.clear();
        }
        let wanted = if universe.path.can_open() {
            preview_set(
                universe.path.offset(),
                &universe.open.points,
                universe.marker_radius(),
            )
        } else {
            Vec::new()
        };
        let before = self.entries.len();
        self.entries.retain(|(marker, _)| wanted.contains(marker));
        let mut changed = self.entries.len() != before;
        for marker in wanted {
            if self.entries.iter().any(|(m, _)| *m == marker) {
                continue;
            }
            let mut chain = self.path.clone();
            chain.push(marker);
            self.entries.push((marker, generate_cell(universe.root, &chain)));
            self.regenerations += 1;
            changed = true;
        }
        if changed {
            self.entries.sort_by_key(|(marker, _)| *marker);
        }
        changed
    }

    /// Number of previewed markers.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` when nothing is previewed.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Previewed content of `marker`, if cached.
    pub fn get(&self, marker: u32) -> Option<&Generated> {
        self.entries
            .iter()
            .find(|(m, _)| *m == marker)
            .map(|(_, generated)| generated)
    }

    /// Previewed markers with their content, by marker index.
    pub fn entries(&self) -> &[(u32, Generated)] {
        &self.entries
    }
}

/// World positions (open-cell units) of `marker`'s previewed children.
///
/// Empty when the marker is missing or the level cannot open.
pub fn preview_positions(universe: &Universe, marker: u32, content: &Generated) -> Vec<[f64; 3]> {
    let (Some(marker_pos), Some(ratio)) =
        (universe.marker(marker), child_ratio(universe.level()))
    else {
        return Vec::new();
    };
    content
        .points
        .iter()
        .map(|point| child_world_position(marker_pos, ratio, point.position))
        .collect()
}

/// Largest relative error between `previewed` (pre-open, parent units,
/// re-expressed through `opened`) and the open cell's marker positions.
pub fn preview_error(previewed: &[[f64; 3]], opened: Opened, open: &Generated) -> f64 {
    let mut worst = 0.0f64;
    if previewed.len() != open.points.len() {
        return f64::INFINITY;
    }
    for (world, point) in previewed.iter().zip(&open.points) {
        for ((w, anchor), want) in world.iter().zip(opened.position).zip(point.position) {
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
/// [`AUTOPILOT_RATE`], open at [`OPEN_ANGLE`], repeat until L11 or
/// `max_secs`. Returns the opened levels in order and the final universe.
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
            .and_then(|m| previews.get(m).map(|c| preview_positions(&universe, m, c)))
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

/// Root plugin: the indicator camera plus the open cell's RGB axes.
pub struct UniverseRenderPlugin;

impl Plugin for UniverseRenderPlugin {
    /// Registers the camera and the axis drawing system.
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_indicator_camera)
            .add_systems(Update, draw_axes);
    }
}

/// Nested dive navigation (R6, sub-issue #60).
///
/// Owns the `Universe`, hover and target state, the autopilot, wheel and
/// key diving, open/close, camera sync, and marker drawing. Added beside
/// `UniverseRenderPlugin`.
pub struct DivePlugin;

impl Plugin for DivePlugin {
    /// Registers navigation state and the input, dive, camera, draw chain.
    fn build(&self, app: &mut App) {
        app.insert_resource(Universe::new(DEMO_SEED))
            .init_resource::<Navigation>()
            .init_resource::<Autopilot>()
            .init_resource::<PreviewCache>()
            .add_systems(
                Update,
                (
                    handle_quit,
                    pick_hover,
                    handle_input,
                    sync_previews,
                    sync_camera,
                    draw_open_cell,
                    draw_previews,
                    draw_parent_siblings,
                )
                    .chain(),
            );
    }
}

/// Hover, target, and smoothed look point.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct Navigation {
    /// Marker under the cursor, if any.
    pub hover: Option<u32>,
    /// Locked target marker, if any.
    pub target: Option<u32>,
    /// Current look point in open-cell units (eased toward the target).
    pub look: Vec3,
    /// Level the title was last written for.
    pub titled: Option<Level>,
}

/// Spacebar autopilot: targets the seeded marker per level and dives.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Autopilot {
    /// Manual control.
    #[default]
    Idle,
    /// Diving automatically until L11.
    Flying,
}

/// Spawns the single 3D camera.
///
/// Tonemapping is `None` (no LUT feature in the minimal pin). The near plane
/// is rewritten every frame by `sync_camera` from the dive distance.
fn spawn_indicator_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Tonemapping::None,
        Transform::from_translation(to_vec3(START_OFFSET)).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Quits on `Esc`.
fn handle_quit(keys: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

/// Finds the marker nearest the cursor within [`PICK_PIXELS`].
fn pick_hover(
    universe: Res<Universe>,
    mut nav: ResMut<Navigation>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    nav.hover = None;
    if !universe.path.can_open() {
        return;
    }
    let (Ok(window), Some((camera, camera_transform))) =
        (windows.single(), cameras.iter().next())
    else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let mut best: Option<(u32, f32)> = None;
    for (index, point) in universe.open.points.iter().enumerate() {
        let Ok(screen) = camera.world_to_viewport(camera_transform, to_vec3(point.position))
        else {
            continue;
        };
        let distance = screen.distance(cursor);
        if distance <= PICK_PIXELS && best.is_none_or(|(_, d)| distance < d) {
            best = Some((index as u32, distance));
        }
    }
    nav.hover = best.map(|(index, _)| index);
}

/// Handles click, wheel, keys, and the autopilot.
///
/// Click locks the hovered marker. Wheel and arrows dive with the target
/// (or the cell center). Spacebar toggles the autopilot; any manual input
/// cancels it. Open clears the target; close re-targets the closed marker.
fn handle_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    mut universe: ResMut<Universe>,
    mut nav: ResMut<Navigation>,
    mut autopilot: ResMut<Autopilot>,
) {
    let mut notches = 0.0f64;
    for event in wheel.read() {
        notches += match event.unit {
            MouseScrollUnit::Line => f64::from(event.y),
            MouseScrollUnit::Pixel => f64::from(event.y) / 24.0,
        };
    }
    let dt = f64::from(time.delta_secs());
    let mut log_factor = notches * WHEEL_FACTOR.ln();
    if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::Equal) {
        log_factor -= KEY_RATE * dt;
    }
    if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::Minus) {
        log_factor += KEY_RATE * dt;
    }
    let clicked = buttons.just_pressed(MouseButton::Left);
    let manual = clicked || log_factor != 0.0;
    if keys.just_pressed(KeyCode::Space) {
        *autopilot = match *autopilot {
            Autopilot::Idle => Autopilot::Flying,
            Autopilot::Flying => Autopilot::Idle,
        };
    } else if manual {
        *autopilot = Autopilot::Idle;
    }
    if clicked && let Some(hover) = nav.hover {
        nav.target = Some(hover);
    }
    if *autopilot == Autopilot::Flying {
        if !universe.path.can_open() {
            *autopilot = Autopilot::Idle;
        } else {
            if nav.target.is_none() {
                nav.target = universe.autopilot_target();
            }
            log_factor = -AUTOPILOT_RATE * dt;
        }
    }
    if log_factor == 0.0 {
        return;
    }
    match universe.dive(nav.target, log_factor.exp()) {
        DiveEvent::Moved => {}
        DiveEvent::Opened(_) => {
            nav.target = None;
            nav.look = Vec3::ZERO;
        }
        DiveEvent::Closed(opened) => {
            nav.target = Some(opened.marker);
            nav.look = to_vec3(opened.position);
        }
    }
}

/// Syncs camera pose, near plane, and window title from the universe.
///
/// The camera sits at the path offset and eases its look point toward the
/// target (or the cell center). The near plane follows the gap to the
/// target so tiny deep markers are never clipped (infinite reverse-Z
/// projection, so no far plane is needed).
fn sync_camera(
    time: Res<Time>,
    universe: Res<Universe>,
    mut nav: ResMut<Navigation>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Camera3d>>,
    mut windows: Query<&mut Window>,
) {
    let level = universe.level();
    if nav.titled != Some(level) {
        nav.titled = Some(level);
        let title = window_title_for_level(level);
        for mut window in &mut windows {
            window.title = title.clone();
        }
    }
    let camera = universe.path.offset();
    let desired = nav
        .target
        .and_then(|m| universe.marker(m))
        .map_or(Vec3::ZERO, to_vec3);
    let ease = 1.0 - (-6.0 * time.delta_secs()).exp();
    nav.look = nav.look.lerp(desired, ease);
    let gap = nav
        .target
        .and_then(|m| universe.marker(m))
        .map_or(DVec3::from_array(camera).length(), |position| {
            (DVec3::from_array(camera) - DVec3::from_array(position)).length()
                - universe.marker_radius()
        })
        .max(1e-6);
    let near = (gap * 0.05).clamp(1e-7, 0.1) as f32;
    let position = to_vec3(camera);
    let look = if nav.look.distance_squared(position) < 1e-12 {
        Vec3::ZERO
    } else {
        nav.look
    };
    for (mut transform, mut projection) in &mut cameras {
        *transform = Transform::from_translation(position).looking_at(look, Vec3::Y);
        if let Projection::Perspective(perspective) = &mut *projection {
            perspective.near = near;
        }
    }
}

/// Draws the open cell's RGB axes (orientation cue) at half-cell length.
fn draw_axes(mut gizmos: Gizmos) {
    gizmos.line(Vec3::ZERO, Vec3::X * 0.5, Color::srgb(1.0, 0.0, 0.0));
    gizmos.line(Vec3::ZERO, Vec3::Y * 0.5, Color::srgb(0.0, 1.0, 0.0));
    gizmos.line(Vec3::ZERO, Vec3::Z * 0.5, Color::srgb(0.0, 0.5, 1.0));
}

/// Keeps the preview cache in line with the camera (regenerates only when
/// the set of previewed markers changes).
fn sync_previews(universe: Res<Universe>, mut previews: ResMut<PreviewCache>) {
    previews.sync(&universe);
}

/// Draws the open cell: its shell, its markers, hover and target.
///
/// Markers draw at the true child size with the impostor clamp, dimmed by
/// [`shell_brightness`] as their interior resolves. The open cell's own
/// shell (radius 0.5) uses the same curve in the parent's era color, so the
/// marker you entered and the cell you are in are one continuous object.
/// The hovered marker is white, the target magenta.
fn draw_open_cell(mut gizmos: Gizmos, universe: Res<Universe>, nav: Res<Navigation>) {
    let camera = DVec3::from_array(universe.path.offset());
    let shell = shell_brightness(angular_radius(0.5, camera.length())) as f32;
    if shell > 0.0 {
        let base = universe
            .level()
            .shallower()
            .map_or(Color::srgb(0.6, 0.6, 0.6), point_color_for_level);
        gizmos.sphere(Isometry3d::IDENTITY, 0.5, scaled(base, shell));
    }
    let color = point_color_for_level(universe.level());
    let radius = universe.marker_radius();
    for (index, point) in universe.open.points.iter().enumerate() {
        let position = DVec3::from_array(point.position);
        let distance = (camera - position).length();
        let drawn = drawn_radius(radius, distance) as f32;
        let brightness = shell_brightness(angular_radius(radius, distance)) as f32;
        let index = index as u32;
        let marker_color = if nav.target == Some(index) {
            scaled(Color::srgb(1.0, 0.0, 1.0), brightness.max(0.5))
        } else if nav.hover == Some(index) {
            scaled(Color::WHITE, brightness.max(0.4))
        } else {
            scaled(color, brightness)
        };
        let isometry = Isometry3d::from_translation(to_vec3(point.position));
        if brightness > 0.0 {
            gizmos.sphere(isometry, drawn, marker_color);
        }
        if nav.target == Some(index) {
            gizmos.sphere(isometry, drawn * 1.6, Color::srgb(0.6, 0.0, 0.6));
        }
    }
}

/// Draws the previewed interiors inside the largest-on-screen markers.
///
/// Children sit at [`child_world_position`] at their true size (the next
/// level's marker radius scaled by the ratio) with the impostor clamp, lit by
/// [`children_brightness`] of their parent marker: invisible while the marker
/// is a point, fully lit by the time it opens, so entering changes nothing.
fn draw_previews(mut gizmos: Gizmos, universe: Res<Universe>, previews: Res<PreviewCache>) {
    let (Some(ratio), Some(child_level)) =
        (child_ratio(universe.level()), universe.level().deeper())
    else {
        return;
    };
    let camera = DVec3::from_array(universe.path.offset());
    let radius = universe.marker_radius();
    let child_radius = open_marker_radius(child_level, 0.01) * ratio;
    let color = point_color_for_level(child_level);
    for (marker, content) in previews.entries() {
        let Some(marker_pos) = universe.marker(*marker) else {
            continue;
        };
        let marker_distance = (camera - DVec3::from_array(marker_pos)).length();
        let brightness = children_brightness(angular_radius(radius, marker_distance)) as f32;
        if brightness <= 0.0 {
            continue;
        }
        let lit = scaled(color, brightness);
        for point in &content.points {
            let world = child_world_position(marker_pos, ratio, point.position);
            let distance = (camera - DVec3::from_array(world)).length();
            let drawn = drawn_radius(child_radius, distance) as f32;
            gizmos.sphere(Isometry3d::from_translation(to_vec3(world)), drawn, lit);
        }
    }
}

/// Draws the parent cell's sibling markers around the open cell.
///
/// Siblings sit `1 / ratio` cells away at radius 0.5, so what you came from
/// stays behind you while you dive. The marker you entered is the shell.
fn draw_parent_siblings(mut gizmos: Gizmos, universe: Res<Universe>) {
    let (Some(parent), Some(entered)) = (&universe.parent, universe.path.entered()) else {
        return;
    };
    let camera = DVec3::from_array(universe.path.offset());
    let level = universe.level().shallower().unwrap_or(Level::MIN);
    let color = sibling_color_for_level(level);
    for (index, point) in parent.points.iter().enumerate() {
        if index as u32 == entered.marker {
            continue;
        }
        let (position, radius) = sibling_in_open_units(entered, point.position);
        let distance = (camera - DVec3::from_array(position)).length();
        let drawn = drawn_radius(radius, distance) as f32;
        gizmos.sphere(
            Isometry3d::from_translation(to_vec3(position)),
            drawn,
            color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dive_step_closes_and_never_crosses_surface() {
        let mut camera = [0.0, 0.0, 4.0];
        let center = [0.0, 0.0, 0.0];
        let radius = 0.2;
        let mut previous = 4.0;
        for step in 0..200 {
            camera = dive_step(camera, center, radius, WHEEL_FACTOR);
            let distance = DVec3::from_array(camera).length();
            if step < 20 {
                assert!(distance < previous, "did not approach at step {step}");
            } else {
                assert!(distance <= previous, "pulled away at step {step}");
            }
            assert!(distance > radius, "crossed the surface");
            previous = distance;
        }
        let away = dive_step(camera, center, radius, 1.0 / WHEEL_FACTOR);
        assert!(DVec3::from_array(away).length() > previous);
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
        assert_eq!(closed.map(|o| o.marker), Some(marker), "close must re-target");
        assert_eq!(universe.level(), Level::MIN);
    }

    #[test]
    fn autopilot_replay_reaches_l11_deterministically() {
        let (steps, universe) = replay_autopilot(DEMO_SEED, 1.0 / 60.0, 600.0);
        assert_eq!(universe.level().get(), MAX_NAV_LEVEL);
        assert_eq!(steps.len(), usize::from(MAX_NAV_LEVEL - 1));
        for (index, step) in steps.iter().enumerate() {
            assert_eq!(usize::from(step.level.get()), index + 2);
            assert!(step.preview_count >= 1, "target was not previewed before opening");
            assert!(step.preview_count <= PREVIEW_CAP);
            assert!(step.preview_error <= 1e-9, "preview drifted: {}", step.preview_error);
            assert!(step.alive <= 2 + PREVIEW_CAP);
        }
        let (again, _) = replay_autopilot(DEMO_SEED, 1.0 / 60.0, 600.0);
        assert_eq!(steps, again, "journey must be the same every run");
        assert!(steps.last().expect("steps").elapsed < 120.0, "journey too long");
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
        assert!(cache.entries().iter().all(|(m, _)| universe.marker(*m).is_some()));
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
        let (position, radius) = sibling_in_open_units(entered, [0.2, 0.0, 0.0]);
        assert!((position[0] - 10.0).abs() < 1e-9);
        assert_eq!(radius, 0.5);
        assert!(drawn_radius(1e-6, 10.0) > 1e-6);
        assert_eq!(drawn_radius(1.0, 10.0), 1.0);
    }

    #[test]
    fn titles_carry_scale_readout() {
        for n in MIN_NAV_LEVEL..=MAX_NAV_LEVEL {
            let level = Level::new(n).expect("level");
            assert!(window_title_for_level(level).contains(&scale_label(level)));
        }
        assert_eq!(scale_label(Level::MIN), "10^27 m");
    }
}
