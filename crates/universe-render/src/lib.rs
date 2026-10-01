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

/// Axis vector length in indicator units; tracks the grid spacing.
pub fn indicator_axis_length(level: Level) -> f32 {
    indicator_spacing(level) * 5.0
}

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

/// Marker radius in indicator units (exaggerated for visibility, not a
/// physical size); tracks the grid spacing.
pub fn point_marker_radius(level: Level) -> f32 {
    indicator_spacing(level) * 0.06
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
            .add_systems(Startup, setup_demo_content)
            .add_systems(Update, (handle_level_keys, draw_demo_points));
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

/// Draws the level indicator grid plus RGB axes, sized per level.
///
/// Reads the navigation level when present (standalone fallback: L1).
fn draw_level_indicators(mut gizmos: Gizmos, current: Option<Res<CurrentLevel>>) {
    let level = match current {
        Some(res) => res.level(),
        None => Level::MIN,
    };
    let spacing = indicator_spacing(level);
    let axis = indicator_axis_length(level);
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

/// Draws the cached demo points for the current level as gizmo spheres.
///
/// Core generator positions are cell-local (`[-0.5, 0.5)`) and are mapped to
/// span the full grid width. Skips silently before startup content exists.
fn draw_demo_points(
    mut gizmos: Gizmos,
    current: Option<Res<CurrentLevel>>,
    content: Option<Res<DemoContent>>,
) {
    let (Some(current), Some(content)) = (current, content) else {
        return;
    };
    let level = current.level();
    let index = usize::from(level.get().clamp(MIN_NAV_LEVEL, MAX_NAV_LEVEL) - MIN_NAV_LEVEL);
    let Some(generated) = content.levels.get(index) else {
        return;
    };
    let color = point_color_for_level(level);
    let radius = point_marker_radius(level);
    let extent = GRID_CELLS as f32 * indicator_spacing(level);
    for point in &generated.points {
        let [px, py, pz] = point.position;
        let pos = Vec3::new(px as f32 * extent, py as f32 * extent, pz as f32 * extent);
        gizmos.sphere(Isometry3d::from_translation(pos), radius, color);
    }
}

/// Switches levels on `1`-`9`/`0`/`q`, refreshes the window title, quits on `Esc`.
///
/// Title writes happen only on an actual switch, never per frame.
fn handle_level_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut current: ResMut<CurrentLevel>,
    mut windows: Query<&mut Window>,
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
        if keys.just_pressed(code)
            && let Some(level) = level_for_key(code)
            && current.level() != level
        {
            current.set(level);
            let title = window_title_for_level(level);
            for mut window in &mut windows {
                window.title = title.clone();
            }
        }
    }
}
