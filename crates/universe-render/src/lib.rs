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
//! The pure navigation math, labels, and headless journey replay used to live
//! here; in M3 (#85) they moved to `universe-core` (`nav`, `labels`), and
//! this crate keeps thin resource newtypes over them plus the temporary
//! re-exports below.

use bevy::camera::{Camera, Projection};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::math::{DVec3, Isometry3d, Vec3};
use bevy::prelude::*;
use universe_core::coords::Level;
pub use universe_core::nest::{CLOSE_ANGLE, OPEN_ANGLE, PREVIEW_ANGLE, PREVIEW_CAP};
use universe_core::nest::{
    angular_radius, child_ratio, child_world_position, children_brightness, shell_brightness,
};

/// Pick radius in logical pixels for hover and click.
pub const PICK_PIXELS: f32 = 24.0;

/// Window-side handle to the navigation state.
///
/// Moved to [`universe_core::nav`] in M3 (#85); this newtype derefs to it so
/// system bodies keep working unchanged.
#[derive(Resource, Deref, DerefMut, Debug, Clone, PartialEq)]
pub struct Universe(pub universe_core::nav::Universe);

/// Window-side handle to the pre-entry preview cache.
///
/// Moved to [`universe_core::nav`] in M3 (#85); this newtype derefs to it so
/// system bodies keep working unchanged.
#[derive(Resource, Deref, DerefMut, Debug, Clone, PartialEq, Default)]
pub struct PreviewCache(pub universe_core::nav::PreviewCache);

// Temporary re-exports of the moved items so already-written code keeps
// resolving while the migration is in flight. `universe-app` imports the
// moved items from `universe-core` directly; these re-exports are removed
// in M5.
pub use universe_core::labels::{level_label, scale_anchor, scale_label, window_title_for_level};
pub use universe_core::nav::{
    AUTOPILOT_RATE, DEMO_SEED, DiveEvent, JourneyStep, KEY_RATE, MAX_NAV_LEVEL, MIN_MARKER_ANGLE,
    MIN_NAV_LEVEL, MarkerIndex, ROOT_MAX_DISTANCE, START_OFFSET, Seed, WHEEL_FACTOR, dive_step,
    drawn_radius, open_marker_radius, preview_error, preview_positions, replay_autopilot,
    should_close, should_open, sibling_in_open_units,
};

/// Marker color per content era: cyan clusters (L1-L4), warm stars
/// (L5-L10), green terrain (L11).
#[must_use]
pub fn point_color_for_level(level: Level) -> Color {
    match level.get() {
        1..=4 => Color::srgb(0.3, 0.8, 1.0),
        5..=10 => Color::srgb(1.0, 0.85, 0.4),
        11 => Color::srgb(0.4, 1.0, 0.5),
        _ => Color::WHITE,
    }
}

/// Dimmed era color for the parent cell's siblings behind the camera.
#[must_use]
pub fn sibling_color_for_level(level: Level) -> Color {
    match level.get() {
        1..=4 => Color::srgb(0.12, 0.32, 0.4),
        5..=10 => Color::srgb(0.4, 0.34, 0.16),
        11 => Color::srgb(0.16, 0.4, 0.2),
        _ => Color::srgb(0.4, 0.4, 0.4),
    }
}

/// Converts a cell-local `f64` position to a render `Vec3`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "E-CAST: f64 to f32 at the render boundary, the only such site in this crate"
)]
fn to_vec3(position: [f64; 3]) -> Vec3 {
    Vec3::new(position[0] as f32, position[1] as f32, position[2] as f32)
}

/// Scales a color's RGB by `brightness` (alpha untouched).
fn scaled(color: Color, brightness: f32) -> Color {
    let s = color.to_srgba();
    Color::srgb(
        s.red * brightness,
        s.green * brightness,
        s.blue * brightness,
    )
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
        app.insert_resource(Universe(universe_core::nav::Universe::new(DEMO_SEED)))
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
    let (Ok(window), Some((camera, camera_transform))) = (windows.single(), cameras.iter().next())
    else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let mut best: Option<(u32, f32)> = None;
    for (index, point) in universe.open.points.iter().enumerate() {
        let Ok(screen) = camera.world_to_viewport(camera_transform, to_vec3(point.position)) else {
            continue;
        };
        let distance = screen.distance(cursor);
        if distance <= PICK_PIXELS && best.is_none_or(|(_, d)| distance < d) {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: hovered marker index into a budgeted cell, always fits u32"
            )]
            let picked = index as u32;
            best = Some((picked, distance));
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
        .and_then(|m| universe.marker(MarkerIndex(m)))
        .map_or(Vec3::ZERO, to_vec3);
    let ease = 1.0 - (-6.0 * time.delta_secs()).exp();
    nav.look = nav.look.lerp(desired, ease);
    let gap = nav
        .target
        .and_then(|m| universe.marker(MarkerIndex(m)))
        .map_or(DVec3::from_array(camera).length(), |position| {
            (DVec3::from_array(camera) - DVec3::from_array(position)).length()
                - universe.marker_radius()
        })
        .max(1e-6);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: render-domain narrowing of a clamped gap, intended"
    )]
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
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: render-domain narrowing of a brightness, intended"
    )]
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
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a radius, intended"
        )]
        let drawn = drawn_radius(radius, distance) as f32;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a brightness, intended"
        )]
        let brightness = shell_brightness(angular_radius(radius, distance)) as f32;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: drawn marker index into a budgeted cell, always fits u32"
        )]
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
        let Some(marker_pos) = universe.marker(MarkerIndex(*marker)) else {
            continue;
        };
        let marker_distance = (camera - DVec3::from_array(marker_pos)).length();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a brightness, intended"
        )]
        let brightness = children_brightness(angular_radius(radius, marker_distance)) as f32;
        if brightness <= 0.0 {
            continue;
        }
        let lit = scaled(color, brightness);
        for point in &content.points {
            let world = child_world_position(marker_pos, ratio, point.position);
            let distance = (camera - DVec3::from_array(world)).length();
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: render-domain narrowing of a radius, intended"
            )]
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
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: sibling marker index into a budgeted cell, always fits u32"
        )]
        let sibling_marker = index as u32;
        if sibling_marker == entered.marker {
            continue;
        }
        let (position, radius) = sibling_in_open_units(entered, point.position);
        let distance = (camera - DVec3::from_array(position)).length();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a radius, intended"
        )]
        let drawn = drawn_radius(radius, distance) as f32;
        gizmos.sphere(
            Isometry3d::from_translation(to_vec3(position)),
            drawn,
            color,
        );
    }
}
