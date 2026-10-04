//! Indicator camera spawn and per-frame sync.
//!
//! Everything here is `pub(crate)`: only this crate uses it.

use crate::Universe;
use crate::input::{Flight, FlightMode, Navigation};
use crate::style::to_vec3;
use bevy::camera::Projection;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::system::Single;
use bevy::math::{DVec3, Vec3};
use bevy::prelude::*;
use universe_core::flight::{free_look_direction, nearest_surface_distance};
use universe_core::labels::window_title_for_level;
use universe_core::nav::{MarkerIndex, START_OFFSET};

/// Spawns the single 3D camera.
///
/// Tonemapping is `None` (no LUT feature in the minimal pin). The near plane
/// is rewritten every frame by `sync_camera` from the dive distance.
pub(crate) fn spawn_indicator_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("indicator camera"),
        Camera3d::default(),
        Tonemapping::None,
        Transform::from_translation(to_vec3(START_OFFSET)).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Syncs camera pose, near plane, and window title from the universe.
///
/// In dive mode the camera sits at the path offset and eases its look point
/// toward the target (or the cell center). In free flight it faces the
/// drag-look direction with the near plane on the nearest surface. The near
/// plane follows the gap to the target (or surface) so tiny deep markers
/// are never clipped (infinite reverse-Z projection, so no far plane is
/// needed). Skipped when there is not exactly one window.
pub(crate) fn sync_camera(
    time: Res<Time>,
    universe: Res<Universe>,
    mut nav: ResMut<Navigation>,
    flight: Res<Flight>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Camera3d>>,
    window: Single<&mut Window>,
) {
    let level = universe.level();
    if nav.titled != Some(level) {
        nav.titled = Some(level);
        window.into_inner().title = window_title_for_level(level);
    }
    let camera = universe.path.offset();
    let gap = match flight.mode {
        FlightMode::Dive => {
            let desired = nav
                .target
                .and_then(|m| universe.marker(MarkerIndex(m)))
                .map_or(Vec3::ZERO, to_vec3);
            let ease = 1.0 - (-6.0 * time.delta_secs()).exp();
            nav.look = nav.look.lerp(desired, ease);
            nav.target
                .and_then(|m| universe.marker(MarkerIndex(m)))
                .map_or(DVec3::from_array(camera).length(), |position| {
                    (DVec3::from_array(camera) - DVec3::from_array(position)).length()
                        - universe.marker_radius()
                })
                .max(1e-6)
        }
        FlightMode::Free => {
            let facing = to_vec3(camera) + to_vec3(free_look_direction(flight.yaw, flight.pitch));
            nav.look = facing;
            nearest_surface_distance(camera, &universe.open, universe.marker_radius()).max(1e-6)
        }
    };
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
