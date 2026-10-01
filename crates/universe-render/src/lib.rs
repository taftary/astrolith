//! Bevy indicators for universe levels: grids, vectors, and points.
//!
//! No meshes, materials, or textures live here by design (spec v3/v4:
//! indicators only). `UniverseRenderPlugin` owns the camera plus the shared
//! grid/axis indicators; `LevelNavigationPlugin` (window navigation,
//! sub-issue #43) adds level switching, per-level demo points generated from
//! `universe-core` with a fixed seed, window-title level display, and
//! `Esc`-to-quit. Every system draws with gizmos only.
//!
//! The pure helpers in this module (`demo_*`, `level_*`, `indicator_*`,
//! `canonical_snapshot`) need no window, so the headless `--verify` mode in
//! `universe-app` reuses them to print the same canonical bytes the window
//! draws.

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::math::{Isometry3d, UVec3, Vec3};
use bevy::prelude::*;
use universe_core::astro::GalaxyGenerator;
use universe_core::coords::{CellPos, Level};
use universe_core::density::DensityGenerator;
use universe_core::r#gen::{Constraints, Generated, Generator, UniformGenerator};
use universe_core::seed::hash_cell;
use universe_core::snapshot::snapshot_generated;
use universe_core::terrain::TerrainSampler;

/// Fixed demo root seed for the window and `--verify` snapshots.
///
/// This is a display seed, not the notion's universe root seed: it feeds the
/// cell hash together with each demo cell, so every level regenerates
/// identically on every run.
pub const DEMO_SEED: u64 = 42;

/// Shallowest navigable level: L1 (observable universe).
pub const MIN_NAV_LEVEL: u8 = 1;

/// Deepest navigable level: L11 (planets and moons, M5 terrain).
///
/// L12-L15 (regions, cities, buildings, rooms) are past the M1-M5 scope
/// (spec v3 non-goals), so navigation stops here.
pub const MAX_NAV_LEVEL: u8 = 11;

/// Grid cells per axis for the level indicator grid.
const GRID_CELLS: u32 = 10;

/// Currently displayed ladder level (window navigation state).
///
/// Inserted by `LevelNavigationPlugin`; the grid system reads it through an
/// optional resource so the render plugin also works standalone.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentLevel(Level);

impl CurrentLevel {
    /// Builds navigation state showing `level`, clamped into L1-L11.
    pub fn new(level: Level) -> CurrentLevel {
        let clamped = level.get().clamp(MIN_NAV_LEVEL, MAX_NAV_LEVEL);
        CurrentLevel(Level::new(clamped).unwrap_or(Level::MIN))
    }

    /// Returns the displayed level.
    pub const fn level(self) -> Level {
        self.0
    }

    /// Switches the displayed level, clamping into L1-L11.
    pub fn set(&mut self, level: Level) {
        *self = CurrentLevel::new(level);
    }
}

impl Default for CurrentLevel {
    /// Starts the window on L1 (observable universe).
    fn default() -> CurrentLevel {
        CurrentLevel(Level::MIN)
    }
}

/// Short display label for a navigable level, from the frozen ladder.
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

/// Window title for `level`: the app name plus the current ladder label.
pub fn window_title_for_level(level: Level) -> String {
    format!("Universe MVP - {}", level_label(level))
}

/// Maps navigation keys to levels: `1`-`9` to L1-L9, `0` to L10, `q` to L11.
///
/// Returns `None` for any other key, including `Esc` (quit, not a level).
pub fn level_for_key(code: KeyCode) -> Option<Level> {
    match code {
        KeyCode::Digit1 => Level::new(1),
        KeyCode::Digit2 => Level::new(2),
        KeyCode::Digit3 => Level::new(3),
        KeyCode::Digit4 => Level::new(4),
        KeyCode::Digit5 => Level::new(5),
        KeyCode::Digit6 => Level::new(6),
        KeyCode::Digit7 => Level::new(7),
        KeyCode::Digit8 => Level::new(8),
        KeyCode::Digit9 => Level::new(9),
        KeyCode::Digit0 => Level::new(10),
        KeyCode::KeyQ => Level::new(11),
        _ => None,
    }
}

/// Grid spacing in indicator units for `level`.
///
/// Shallow levels read larger (L1: 1.6) and deep levels finer (L11: 0.4).
/// These are display units, not meters: true ladder sizes span 10^27-10^7 m
/// and cannot fit an `f32` view (see the coordinates module in
/// `universe-core`).
pub fn indicator_spacing(level: Level) -> f32 {
    let depth = level.get().clamp(MIN_NAV_LEVEL, MAX_NAV_LEVEL) - MIN_NAV_LEVEL;
    1.6 - 0.12 * f32::from(depth)
}

/// Axis scale factor applied to the (continuous) spacing for RGB axes.
pub const AXIS_SCALE: f32 = 5.0;

/// Marker radius scale factor applied to the (continuous) spacing.
pub const MARKER_SCALE: f32 = 0.06;

/// Baseline generator count for `level`, per content era.
///
/// L1-L4 scatter cluster points (M3), L5-L10 star points (M4 octree), L11
/// terrain points (M5); levels past navigation default to a sparse count.
pub const fn demo_base_count(level: Level) -> u32 {
    match level.get() {
        1..=4 => 48,
        5..=10 => 32,
        11 => 24,
        _ => 16,
    }
}

/// Fixed demo budget for `level`: full density, at least 8 points.
///
/// Valid by construction (density 1.0, `8 <= max`, extents of half a cell).
pub fn demo_constraints_for_level(level: Level) -> Constraints {
    let constraints = Constraints {
        density_multiplier: 1.0,
        min_count: 8,
        max_count: demo_base_count(level).max(8),
        allowed_extent: [0.5, 0.5, 0.5],
    };
    debug_assert!(constraints.is_valid());
    constraints
}

/// Fixed demo cell for `level`: the origin cell of that rung.
pub const fn demo_cell_for_level(level: Level) -> CellPos {
    CellPos::new(level, 0, 0, 0)
}

/// Derives the demo cell seed from the fixed root seed and the cell.
///
/// Same inputs replay the same stream, so window points and `--verify`
/// snapshots agree byte for byte.
pub fn demo_cell_seed(cell: CellPos) -> u64 {
    hash_cell(DEMO_SEED, cell.level.get(), cell.x, cell.y, cell.z)
}

/// Era generator selected for `level`'s demo content.
///
/// M3 density field (L1-L4), M4 galaxies (L5-L10), M5 terrain sampler (L11);
/// uniform scatter past the MVP scope. The window and `--verify` share this,
/// so both show exactly what the milestone generators produce.
#[derive(Debug)]
pub enum DemoGenerator {
    /// L1-L4 cluster points from the M3 density field.
    Density(DensityGenerator),
    /// L5-L10 star points from the M4 galaxy generator.
    Galaxy(GalaxyGenerator),
    /// L11 heightmap points from the M5 terrain sampler.
    Terrain(TerrainSampler),
    /// Uniform scatter for levels past the MVP scope.
    Uniform(UniformGenerator),
}

impl Generator for DemoGenerator {
    /// Generates the demo cell with the era generator for the selected level.
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        match self {
            DemoGenerator::Density(generator) => generator.generate(seed, parent),
            DemoGenerator::Galaxy(generator) => generator.generate(seed, parent),
            DemoGenerator::Terrain(generator) => generator.generate(seed, parent),
            DemoGenerator::Uniform(generator) => generator.generate(seed, parent),
        }
    }
}

/// Builds the era generator for `level`'s demo cell.
pub fn demo_generator_for_level(level: Level) -> DemoGenerator {
    match level.get() {
        1..=4 => {
            let cell = demo_cell_for_level(level);
            DemoGenerator::Density(DensityGenerator::new(cell.level, cell.x, cell.y, cell.z))
        }
        5..=10 => DemoGenerator::Galaxy(GalaxyGenerator::new(demo_base_count(level))),
        11 => DemoGenerator::Terrain(TerrainSampler::new()),
        _ => DemoGenerator::Uniform(UniformGenerator::new(demo_base_count(level))),
    }
}

/// Generates the demo indicator content for `level` from the fixed seed.
pub fn demo_generated_for_level(level: Level) -> Generated {
    let cell = demo_cell_for_level(level);
    let constraints = demo_constraints_for_level(level);
    demo_generator_for_level(level).generate(demo_cell_seed(cell), &constraints)
}

/// Canonical snapshot bytes for `level`.
///
/// Uses the core canonical form ([`snapshot_generated`]), exactly the bytes
/// the determinism check byte-compares, so a snapshot printed by `--verify`
/// proves what the check compared.
pub fn canonical_snapshot(level: Level) -> String {
    snapshot_generated(&demo_generated_for_level(level))
}

/// Indicator color for `level`'s points: cyan clusters (L1-L4, M3), warm
/// stars (L5-L10, M4), green terrain (L11, M5).
pub fn point_color_for_level(level: Level) -> Color {
    match level.get() {
        1..=4 => Color::srgb(0.3, 0.8, 1.0),
        5..=10 => Color::srgb(1.0, 0.85, 0.4),
        11 => Color::srgb(0.4, 1.0, 0.5),
        _ => Color::WHITE,
    }
}

/// Root plugin for universe level indicators (grid plus axis vectors).
///
/// Owns the camera and the grid/axis drawing; per-level demo points and input
/// live in `LevelNavigationPlugin` beside this plugin, never inside it.
pub struct UniverseRenderPlugin;

impl Plugin for UniverseRenderPlugin {
    /// Registers the indicator camera and the gizmo drawing systems.
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_indicator_camera)
            .add_systems(Update, draw_level_indicators);
    }
}

/// Window navigation polish (sub-issue #43).
///
/// Owns the `CurrentLevel` resource, the cached demo content, key switching
/// (`1`-`9`, `0`, `q`), window-title display, `Esc`-to-quit, and per-level
/// point drawing. Added beside `UniverseRenderPlugin`.
pub struct LevelNavigationPlugin;

impl Plugin for LevelNavigationPlugin {
    /// Registers navigation state, demo content setup, and input/draw systems.
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentLevel>()
            .init_resource::<ZoomState>()
            .init_resource::<FlightState>()
            .add_systems(Startup, setup_demo_content)
            .add_systems(
                Update,
                (
                    handle_level_keys,
                    apply_flight,
                    apply_zoom_travel,
                    sync_level_camera,
                    draw_demo_points,
                    draw_target_beacon,
                )
                    .chain(),
            );
    }
}

/// Cached demo generations for L1-L11.
///
/// Built once at startup so the hot draw system iterates instead of
/// allocating (stack budget: no per-frame allocations in hot systems).
/// Indicator counts stay small (at most 48 points per level), so the
/// gizmo-only frame easily fits the noted 60 fps budget (budget noted, not
/// yet proven, per the spec test plan).
#[derive(Resource, Debug)]
struct DemoContent {
    /// `levels[i]` holds the generation for L(`i + 1`).
    levels: Vec<Generated>,
}

/// Spawns the single 3D camera used to view level indicators.
///
/// Tonemapping is explicitly `None`: gizmo indicators need no filmic curve,
/// and this avoids the `tonemapping_luts` feature the default TonyMcMapFace
/// requires (minimal-features pin, see docs/universe/stack.md).
fn spawn_indicator_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Tonemapping::None,
        Transform::from_xyz(8.0, 8.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Rung (zoom position) displaying `level` exactly.
///
/// Rungs sit at integers (`11 - level`): L1 at 10.0, L11 at 0.0.
pub fn rung_for_level(level: Level) -> f32 {
    11.0 - f32::from(level.get().clamp(MIN_NAV_LEVEL, MAX_NAV_LEVEL))
}

/// Displayed level at continuous zoom `position`.
///
/// Altitude grows with position, so the level derives from log altitude per
/// the notion section 7: each integer rung shows one ladder level.
pub fn level_at_position(position: f32) -> Level {
    let rung = position.clamp(0.0, 10.0).round() as u8;
    Level::new(11 - rung).unwrap_or(Level::MIN)
}

/// Interpolated indicator spacing at continuous zoom `position`.
///
/// Log-interpolates between the rung spacings, so the grid grows smoothly
/// while diving instead of jumping at rung boundaries.
pub fn spacing_at(position: f32) -> f32 {
    let clamped = position.clamp(0.0, 10.0);
    let low = clamped.floor();
    let fraction = clamped - low;
    let high = (low + 1.0).min(10.0);
    let low_spacing = indicator_spacing(level_at_position(low));
    let high_spacing = indicator_spacing(level_at_position(high));
    (low_spacing.ln() + (high_spacing.ln() - low_spacing.ln()) * fraction).exp()
}

/// Zoom travel state: continuous position across the ladder rungs.
///
/// Starts at 10.0 (L1). Wheel-up and ArrowUp dive deeper (position falls);
/// wheel-down and ArrowDown rise. Linear steps in rung space are exponential
/// steps in altitude, so travel feels constant per the notion camera model
/// (`v = k_speed · h`).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ZoomState {
    /// Zoom position in rungs, always within `[0.0, 10.0]`.
    pub position: f32,
}

impl Default for ZoomState {
    /// Starts the window at L1 (observable universe).
    fn default() -> ZoomState {
        ZoomState { position: 10.0 }
    }
}

/// Wheel notches per rung of travel.
const WHEEL_STEP: f32 = 0.5;

/// Rungs per second for held zoom keys.
const KEY_RATE: f32 = 3.0;

/// Seconds for a full L1 to L11 test flight.
pub const FLIGHT_DURATION_SECS: f32 = 20.0;

/// Flight-to-planet test journey (the owner-defined test, round 4, redefined
/// look: fixed-world fly-through).
///
/// Spacebar starts a visible flight from L1 to a deterministic fixed target
/// planet site at L11: the cosmic-web volumes stay fixed in world space and
/// the camera genuinely closes from the L1 framing distance to the L11
/// framing distance while sweeping half an orbit, so surrounding points
/// stream past. As the altitude crosses each rung the child dimension loads
/// and crossfades in while the parent stays visible; a fixed-size magenta
/// beacon marks the locked target and the flight ends stopped at the planet.
/// The target is "random" the way the universe is random — drawn from the
/// fixed demo seed stream, so every run flies the same journey
/// (determinism invariant, owner decision: fixed target).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub enum FlightState {
    /// No flight in progress; manual zoom owns the camera.
    Idle,
    /// Flying; `elapsed` seconds since Spacebar.
    Flying {
        /// Seconds since the flight started.
        elapsed: f32,
    },
}

impl Default for FlightState {
    /// Starts parked: no flight until the owner presses Spacebar.
    fn default() -> FlightState {
        FlightState::Idle
    }
}

/// Journey progress `0.0..=1.0` at `elapsed` seconds.
///
/// Smoothstep easing: gentle departure and arrival, steady mid-flight.
pub fn flight_progress(elapsed: f32) -> f32 {
    let unit = (elapsed / FLIGHT_DURATION_SECS).clamp(0.0, 1.0);
    unit * unit * (3.0 - 2.0 * unit)
}

/// Zoom position along the flight: 10.0 (L1) down to 0.0 (L11).
pub fn flight_position(elapsed: f32) -> f32 {
    10.0 - 10.0 * flight_progress(elapsed)
}

/// Orbit angle (radians) swept during the flight.
///
/// Half an orbit over the journey: the camera circles while diving so the
/// point fields visibly stream past instead of scaling in place.
pub fn flight_orbit_angle(elapsed: f32) -> f32 {
    flight_progress(elapsed) * std::f32::consts::PI
}

/// Camera direction during the flight at `elapsed` seconds.
///
/// Diagonal view rotating half an orbit around the vertical axis, always
/// looking at the target site where the beacon waits. Unit length by
/// construction; [`flight_camera_pose`] sets the closing radius.
pub fn flight_direction(elapsed: f32) -> Vec3 {
    let angle = flight_orbit_angle(elapsed);
    Vec3::new(angle.cos() + 1.2, 1.0, angle.sin()).normalize()
}

/// Fixed world size of `level`'s indicator volume.
///
/// Unlike [`spacing_at`] (which rescales one grid with the camera for manual
/// zoom), the flight draws every level at its own fixed size: shallow
/// volumes are big, deep volumes sit small. The camera genuinely moves
/// through them instead of watching one grid breathe.
pub fn world_extent(level: Level) -> f32 {
    GRID_CELLS as f32 * indicator_spacing(level)
}

/// Unit-space spread of the target site around the origin.
const FLIGHT_TARGET_SPREAD: f32 = 0.25;

/// Deterministic target planet site for the test flight (owner decision:
/// fixed target, same journey every run).
///
/// Drawn from the demo seed stream via [`hash_cell`] (the same seed family
/// as the generators, so it is "random" the way the universe is random),
/// then mapped into L1 world units. It is a fixed site rather than a live
/// index into the L11 points so the journey stays stable across generator
/// versions. Pure and allocation-free: visuals and `--verify` share it.
pub fn flight_target() -> Vec3 {
    let l1 = world_extent(Level::new(MIN_NAV_LEVEL).unwrap_or(Level::MIN));
    let lane = |salt: i64| -> f32 {
        let hash = hash_cell(
            DEMO_SEED,
            MAX_NAV_LEVEL,
            salt,
            salt.wrapping_mul(31),
            salt.wrapping_mul(101),
        );
        let unit = (hash >> 32) as u32 as f32 / u32::MAX as f32;
        (unit - 0.5) * 2.0 * FLIGHT_TARGET_SPREAD * l1
    };
    Vec3::new(lane(7), lane(13) * 0.5, lane(29))
}

/// Levels visible at continuous zoom `position` during the flight.
///
/// Returns the parent (shallower) level, the child dimension loading in (if
/// inside a transition band), and the child's blend weight `0.0..=1.0`. At
/// integer rungs only the parent shows; between rungs the parent stays while
/// the child fades in (owner decision: crossfade, never a hard switch).
pub fn flight_crossfade(position: f32) -> (Level, Option<Level>, f32) {
    let depth = (11.0 - position.clamp(0.0, 10.0)).clamp(1.0, 11.0);
    let parent_num = depth.floor() as u8;
    let parent =
        Level::new(parent_num.clamp(MIN_NAV_LEVEL, MAX_NAV_LEVEL)).unwrap_or(Level::MIN);
    let fraction = depth - depth.floor();
    if fraction < 0.02 || parent_num >= MAX_NAV_LEVEL {
        return (parent, None, 0.0);
    }
    let child = Level::new(parent_num + 1).unwrap_or(Level::MIN);
    let weight = fraction * fraction * (3.0 - 2.0 * fraction);
    (parent, Some(child), weight)
}

/// Flight camera pose at `elapsed` seconds: position plus look target.
///
/// Fixed-world fly-through (owner decision): volumes stay fixed and the
/// camera genuinely closes from the L1 framing distance to the L11 framing
/// distance while orbiting half a turn, always looking at the target site.
/// The radius strictly decreases, so the approach never stalls mid-journey.
pub fn flight_camera_pose(elapsed: f32) -> (Vec3, Vec3) {
    let target = flight_target();
    let start = world_extent(Level::new(MIN_NAV_LEVEL).unwrap_or(Level::MIN)) * 1.4;
    let end = world_extent(Level::new(MAX_NAV_LEVEL).unwrap_or(Level::MIN)) * 1.1;
    let radius = start + (end - start) * flight_progress(elapsed);
    (target + flight_direction(elapsed) * radius, target)
}

/// Camera distance to the target site at `elapsed` seconds.
pub fn flight_camera_distance(elapsed: f32) -> f32 {
    flight_camera_pose(elapsed).0.distance(flight_target())
}

/// Advances or cancels the test flight.
///
/// Spacebar starts the flight (or cancels it mid-flight); number keys hand
/// control back to manual zoom; wheel and arrow zoom also cancel. While
/// flying, the zoom position follows the journey and the camera orbits.
fn apply_flight(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut flight: ResMut<FlightState>,
    mut zoom: ResMut<ZoomState>,
    mut wheel: MessageReader<MouseWheel>,
) {
    if keys.just_pressed(KeyCode::Space) {
        match *flight {
            FlightState::Idle => {
                *flight = FlightState::Flying { elapsed: 0.0 };
            }
            FlightState::Flying { .. } => {
                *flight = FlightState::Idle;
                return;
            }
        }
    }
    let flying = match *flight {
        FlightState::Idle => return,
        FlightState::Flying { elapsed } => elapsed,
    };
    let manual_override = !wheel.is_empty()
        || keys.pressed(KeyCode::ArrowUp)
        || keys.pressed(KeyCode::ArrowDown)
        || keys.pressed(KeyCode::Equal)
        || keys.pressed(KeyCode::Minus)
        || keys.just_pressed(KeyCode::Digit1)
        || keys.just_pressed(KeyCode::Digit2)
        || keys.just_pressed(KeyCode::Digit3)
        || keys.just_pressed(KeyCode::Digit4)
        || keys.just_pressed(KeyCode::Digit5)
        || keys.just_pressed(KeyCode::Digit6)
        || keys.just_pressed(KeyCode::Digit7)
        || keys.just_pressed(KeyCode::Digit8)
        || keys.just_pressed(KeyCode::Digit9)
        || keys.just_pressed(KeyCode::Digit0)
        || keys.just_pressed(KeyCode::KeyQ);
    wheel.clear();
    if manual_override {
        *flight = FlightState::Idle;
        return;
    }
    let elapsed = flying + time.delta_secs();
    if elapsed >= FLIGHT_DURATION_SECS {
        *flight = FlightState::Idle;
        zoom.position = 0.0;
    } else {
        *flight = FlightState::Flying { elapsed };
        zoom.position = flight_position(elapsed);
    }
}

/// Draws the locked-target beacon while flying.
///
/// A fixed-size magenta marker at the target site plus a sight line from the
/// camera, so the destination stays visible for the whole journey. The size
/// is constant in world units, so the beacon visibly grows on approach
/// instead of holding a fixed screen size.
fn draw_target_beacon(
    mut gizmos: Gizmos,
    flight: Res<FlightState>,
    cameras: Query<&Transform, With<Camera3d>>,
) {
    if *flight == FlightState::Idle {
        return;
    }
    let target = flight_target();
    let size = world_extent(Level::new(MAX_NAV_LEVEL).unwrap_or(Level::MIN));
    gizmos.sphere(
        Isometry3d::from_translation(target),
        size * 0.09,
        Color::srgb(1.0, 0.0, 1.0),
    );
    if let Some(camera) = cameras.iter().next() {
        gizmos.line(camera.translation, target, Color::srgb(1.0, 0.0, 1.0));
    }
}

/// Applies smooth exponential zoom travel and derives the displayed level.
///
/// Reads wheel and zoom keys, moves the camera along the view diagonal so
/// the current rung's grid fills the view, and syncs `CurrentLevel` plus the
/// window title when the rounded level changes. While a test flight runs,
/// the flight owns the camera and this system only syncs level and title.
fn apply_zoom_travel(
    time: Res<Time>,
    mut wheel: MessageReader<MouseWheel>,
    keys: Res<ButtonInput<KeyCode>>,
    flight: Res<FlightState>,
    mut zoom: ResMut<ZoomState>,
) {
    let mut delta = 0.0;
    if *flight == FlightState::Idle {
        for event in wheel.read() {
            let lines = match event.unit {
                MouseScrollUnit::Line => event.y,
                MouseScrollUnit::Pixel => event.y / 24.0,
            };
            delta -= lines * WHEEL_STEP;
        }
        if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::Equal) {
            delta -= KEY_RATE * time.delta_secs();
        }
        if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::Minus) {
            delta += KEY_RATE * time.delta_secs();
        }
    } else {
        wheel.clear();
    }
    zoom.position = (zoom.position + delta).clamp(0.0, 10.0);
}

/// Syncs the displayed level, window title, and camera from the zoom state.
///
/// Manual zoom frames the current rung's grid on the fixed diagonal. During
/// a test flight the camera follows the fixed-world pose instead (genuine
/// closing distance onto the target site); level and title still sync from
/// the flight-driven zoom position.
fn sync_level_camera(
    flight: Res<FlightState>,
    zoom: Res<ZoomState>,
    mut current: ResMut<CurrentLevel>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
    mut windows: Query<&mut Window>,
) {
    let level = level_at_position(zoom.position);
    if current.level() != level {
        current.set(level);
        let title = window_title_for_level(level);
        for mut window in &mut windows {
            window.title = title.clone();
        }
    }
    if let FlightState::Flying { elapsed } = *flight {
        let (position, target) = flight_camera_pose(elapsed);
        for mut transform in &mut cameras {
            *transform = Transform::from_translation(position).looking_at(target, Vec3::Y);
        }
        return;
    }
    let spacing = spacing_at(zoom.position);
    let distance = GRID_CELLS as f32 * spacing;
    let direction = Vec3::new(1.0, 1.0, 1.0).normalize();
    for mut transform in &mut cameras {
        *transform =
            Transform::from_translation(direction * distance).looking_at(Vec3::ZERO, Vec3::Y);
    }
}

/// Draws one level's grid plus RGB axes at its fixed world size.
///
/// Used by the flight crossfade so each volume holds still while the camera
/// moves; manual zoom keeps its continuous rescaling below.
fn draw_fixed_grid(gizmos: &mut Gizmos, level: Level) {
    let spacing = indicator_spacing(level);
    gizmos
        .grid_3d(
            Isometry3d::IDENTITY,
            UVec3::splat(GRID_CELLS),
            Vec3::splat(spacing),
            Color::WHITE,
        )
        .outer_edges();
    let axis = spacing * AXIS_SCALE;
    gizmos.line(Vec3::ZERO, Vec3::X * axis, Color::srgb(1.0, 0.0, 0.0));
    gizmos.line(Vec3::ZERO, Vec3::Y * axis, Color::srgb(0.0, 1.0, 0.0));
    gizmos.line(Vec3::ZERO, Vec3::Z * axis, Color::srgb(0.0, 0.5, 1.0));
}

/// Draws the level indicator grid plus RGB axes.
///
/// Manual zoom follows the continuous zoom position (standalone fallback:
/// L1 rung). During a flight the parent volume plus the loading child are
/// drawn at fixed sizes instead, so the journey reads as motion through
/// space with crossfading dimensions.
fn draw_level_indicators(
    mut gizmos: Gizmos,
    zoom: Option<Res<ZoomState>>,
    flight: Option<Res<FlightState>>,
) {
    let position = zoom.map_or(10.0, |res| res.position);
    if flight.as_deref().is_some_and(|state| *state != FlightState::Idle) {
        let (parent, child, _) = flight_crossfade(position);
        draw_fixed_grid(&mut gizmos, parent);
        if let Some(child) = child {
            draw_fixed_grid(&mut gizmos, child);
        }
        return;
    }
    let spacing = spacing_at(position);
    let axis = spacing * AXIS_SCALE;
    gizmos
        .grid_3d(
            Isometry3d::IDENTITY,
            UVec3::splat(GRID_CELLS),
            Vec3::splat(spacing),
            Color::WHITE,
        )
        .outer_edges();
    gizmos.line(Vec3::ZERO, Vec3::X * axis, Color::srgb(1.0, 0.0, 0.0));
    gizmos.line(Vec3::ZERO, Vec3::Y * axis, Color::srgb(0.0, 1.0, 0.0));
    gizmos.line(Vec3::ZERO, Vec3::Z * axis, Color::srgb(0.0, 0.5, 1.0));
}

/// Builds the cached demo generations for L1-L11 once at startup.
fn setup_demo_content(mut commands: Commands) {
    let mut levels = Vec::with_capacity(usize::from(MAX_NAV_LEVEL - MIN_NAV_LEVEL + 1));
    for n in MIN_NAV_LEVEL..=MAX_NAV_LEVEL {
        let level = Level::new(n).unwrap_or(Level::MIN);
        levels.push(demo_generated_for_level(level));
    }
    commands.insert_resource(DemoContent { levels });
}

/// Draws one level's cached points at their fixed world size.
///
/// `size_weight` scales the marker radius so a loading child visibly grows
/// in; the parent always draws full-size. Reads the startup cache, so the
/// hot path iterates without allocating.
fn draw_fixed_points(
    gizmos: &mut Gizmos,
    content: &DemoContent,
    level: Level,
    size_weight: f32,
) {
    let index = usize::from(level.get().clamp(MIN_NAV_LEVEL, MAX_NAV_LEVEL) - MIN_NAV_LEVEL);
    let Some(generated) = content.levels.get(index) else {
        return;
    };
    let color = point_color_for_level(level);
    let extent = world_extent(level);
    let radius = extent * 0.012 * size_weight.max(0.0);
    for point in &generated.points {
        let [px, py, pz] = point.position;
        let pos = Vec3::new(px as f32 * extent, py as f32 * extent, pz as f32 * extent);
        gizmos.sphere(Isometry3d::from_translation(pos), radius, color);
    }
}

/// Draws the cached demo points for the current level as gizmo spheres.
///
/// Core generator positions are cell-local (`[-0.5, 0.5)`) and are mapped to
/// span the full grid width. Manual zoom draws the single current level with
/// the continuous spacing; during a flight the parent volume stays while the
/// loading child blends in at its fixed size. Skips silently before startup
/// content exists.
fn draw_demo_points(
    mut gizmos: Gizmos,
    current: Option<Res<CurrentLevel>>,
    content: Option<Res<DemoContent>>,
    zoom: Option<Res<ZoomState>>,
    flight: Option<Res<FlightState>>,
) {
    let (Some(current), Some(content)) = (current, content) else {
        return;
    };
    let position = zoom.map_or(10.0, |res| res.position);
    if flight.as_deref().is_some_and(|state| *state != FlightState::Idle) {
        let (parent, child, weight) = flight_crossfade(position);
        draw_fixed_points(&mut gizmos, &content, parent, 1.0);
        if let Some(child) = child {
            draw_fixed_points(&mut gizmos, &content, child, weight.max(0.35));
        }
        return;
    }
    let level = current.level();
    let index = usize::from(level.get().clamp(MIN_NAV_LEVEL, MAX_NAV_LEVEL) - MIN_NAV_LEVEL);
    let Some(generated) = content.levels.get(index) else {
        return;
    };
    let color = point_color_for_level(level);
    let spacing = spacing_at(position);
    let radius = spacing * MARKER_SCALE;
    let extent = GRID_CELLS as f32 * spacing;
    for point in &generated.points {
        let [px, py, pz] = point.position;
        let pos = Vec3::new(px as f32 * extent, py as f32 * extent, pz as f32 * extent);
        gizmos.sphere(Isometry3d::from_translation(pos), radius, color);
    }
}

/// Switches rungs on `1`-`9`/`0`/`q`, quits on `Esc`.
///
/// Number keys jump the zoom position to the rung; `apply_zoom_travel`
/// syncs the level and title from there. Title writes happen only on an
/// actual level change, never per frame.
fn handle_level_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut zoom: ResMut<ZoomState>,
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
        return;
    }
    for code in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
        KeyCode::Digit0,
        KeyCode::KeyQ,
    ] {
        if keys.just_pressed(code) && let Some(level) = level_for_key(code) {
            zoom.position = rung_for_level(level);
        }
    }
}

#[cfg(test)]
mod zoom_tests {
    use super::*;

    #[test]
    fn rungs_hit_endpoints_and_every_level() {
        assert_eq!(level_at_position(10.0), Level::new(1).expect("L1"));
        assert_eq!(level_at_position(0.0), Level::new(11).expect("L11"));
        for rung in 0..=10u8 {
            let level = level_at_position(f32::from(rung));
            assert_eq!(level, Level::new(11 - rung).expect("rung level"));
            assert_eq!(rung_for_level(level), f32::from(rung));
        }
    }

    #[test]
    fn sweep_visits_all_levels_monotonically() {
        let mut seen = [false; 11];
        let mut previous = 11u8;
        let mut position = 0.0f32;
        while position <= 10.0 {
            let level = level_at_position(position).get();
            assert!(level <= previous, "level rose while rising: {level} after {previous}");
            previous = level;
            seen[usize::from(level - 1)] = true;
            position += 0.25;
        }
        assert!(seen.iter().all(|visited| *visited), "sweep missed a level");
    }

    #[test]
    fn spacing_matches_rungs_and_grows_smoothly() {
        for rung in 0..=10u8 {
            let at_rung = spacing_at(f32::from(rung));
            let expected = indicator_spacing(level_at_position(f32::from(rung)));
            assert!((at_rung - expected).abs() < 1e-6, "rung {rung} mismatch");
        }
        let mut previous = 0.0f32;
        let mut position = 0.0f32;
        while position <= 10.0 {
            let spacing = spacing_at(position);
            assert!(spacing > 0.0 && spacing >= previous, "spacing shrank while rising");
            previous = spacing;
            position += 0.25;
        }
    }

    #[test]
    fn positions_clamp_into_range() {
        assert_eq!(level_at_position(99.0), Level::new(1).expect("L1"));
        assert_eq!(level_at_position(-99.0), Level::new(11).expect("L11"));
        assert!(spacing_at(99.0) > 0.0 && spacing_at(-99.0) > 0.0);
    }

    #[test]
    fn flight_dives_l1_to_l11_and_stops() {
        assert_eq!(flight_position(0.0), 10.0);
        assert_eq!(flight_position(FLIGHT_DURATION_SECS), 0.0);
        assert_eq!(flight_position(FLIGHT_DURATION_SECS + 100.0), 0.0);
        assert_eq!(level_at_position(flight_position(0.0)).get(), 1);
        assert_eq!(
            level_at_position(flight_position(FLIGHT_DURATION_SECS)).get(),
            11
        );
    }

    #[test]
    fn flight_visits_every_level_monotonically() {
        let mut seen = [false; 11];
        let mut previous_position = f32::INFINITY;
        let mut elapsed = 0.0f32;
        while elapsed <= FLIGHT_DURATION_SECS {
            let position = flight_position(elapsed);
            assert!(position <= previous_position, "flight rose mid-journey");
            previous_position = position;
            let rung_level = level_at_position(position).get();
            seen[usize::from(rung_level - 1)] = true;
            elapsed += 0.5;
        }
        assert!(seen.iter().all(|visited| *visited), "flight skipped a level");
    }

    #[test]
    fn flight_sweeps_half_an_orbit() {
        assert_eq!(flight_orbit_angle(0.0), 0.0);
        let end = flight_orbit_angle(FLIGHT_DURATION_SECS);
        assert!(
            (end - std::f32::consts::PI).abs() < 1e-6,
            "orbit did not complete half turn"
        );
        let direction = flight_direction(0.0);
        assert!((direction.length() - 1.0).abs() < 1e-6, "direction not unit");
    }

    #[test]
    fn flight_target_is_fixed_and_in_bounds() {
        let first = flight_target();
        let second = flight_target();
        assert_eq!(first, second, "target must be fixed across calls");
        let half = world_extent(Level::new(1).expect("L1")) / 2.0;
        assert!(
            first.x.abs() <= half && first.y.abs() <= half && first.z.abs() <= half,
            "target outside the L1 volume: {first}"
        );
        assert!(
            first.length() > 0.05,
            "target degenerately at the origin: {first}"
        );
    }

    #[test]
    fn flight_crossfade_overlaps_parent_and_child() {
        for rung in 0..=10u8 {
            let (parent, child, weight) = flight_crossfade(f32::from(rung));
            assert_eq!(parent, Level::new(11 - rung).expect("rung level"));
            assert!(child.is_none(), "rung {rung} should show the parent alone");
            assert_eq!(weight, 0.0);
        }
        let mut saw_overlap = false;
        let mut position = 0.0f32;
        while position <= 10.0 {
            let (parent, child, weight) = flight_crossfade(position);
            if let Some(child) = child {
                saw_overlap = true;
                assert_eq!(
                    child.get(),
                    parent.get() + 1,
                    "child must be the next level down"
                );
                assert!(
                    (0.0..=1.0).contains(&weight),
                    "weight out of range: {weight}"
                );
            }
            position += 0.125;
        }
        assert!(saw_overlap, "crossfade never overlapped parent and child");
    }

    #[test]
    fn flight_camera_closes_monotonically_onto_target() {
        let target = flight_target();
        let (start_pos, start_look) = flight_camera_pose(0.0);
        let (end_pos, end_look) = flight_camera_pose(FLIGHT_DURATION_SECS);
        assert_eq!(start_look, target);
        assert_eq!(end_look, target);
        assert_eq!(start_pos.distance(target), flight_camera_distance(0.0));
        assert_eq!(end_pos.distance(target), flight_camera_distance(FLIGHT_DURATION_SECS));
        let mut previous = f32::INFINITY;
        let mut elapsed = 0.0f32;
        while elapsed <= FLIGHT_DURATION_SECS {
            let distance = flight_camera_distance(elapsed);
            assert!(distance <= previous, "camera pulled away at {elapsed:.1}s");
            previous = distance;
            elapsed += 0.5;
        }
        assert!(
            flight_camera_distance(FLIGHT_DURATION_SECS) < flight_camera_distance(0.0) / 2.0,
            "approach did not close onto the target"
        );
    }
}
