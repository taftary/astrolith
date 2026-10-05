//! Indicator camera spawn and per-frame sync.
//!
//! Everything here is `pub(crate)`: only this crate uses it.

use crate::Universe;
use crate::input::{Flight, FlightMode, Navigation};
use crate::style::to_vec3;
use bevy::camera::{Exposure, Projection};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::system::Single;
use bevy::math::{DVec3, Vec3};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use universe_core::flight::{free_look_direction, nearest_surface_distance};
use universe_core::labels::window_title_for_level;
use universe_core::nav::{MarkerIndex, START_OFFSET};

/// Manual exposure multiplier, `1.0` matching the pre-#157 look (#157).
///
/// Driven by `E` (dim) and `Shift+E` (brighten) in [`crate::input`]; applied
/// to the camera [`Exposure`] by [`sync_exposure`]. Render-only state: it
/// never touches snapshots or the golden files.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub(crate) struct ExposureLevel(pub f32);

impl Default for ExposureLevel {
    fn default() -> ExposureLevel {
        ExposureLevel(1.0)
    }
}

/// Exposure step per key tap (`E` divides, `Shift+E` multiplies).
pub(crate) const EXPOSURE_STEP: f32 = 1.25;

/// Dimmest manual exposure (two stops under the default look).
pub(crate) const EXPOSURE_MIN: f32 = 0.25;

/// Brightest manual exposure (two stops over the default look).
pub(crate) const EXPOSURE_MAX: f32 = 4.0;

/// Steps the exposure one tap up (brighter) or down (dimmer), clamped.
#[must_use]
pub(crate) fn step_exposure(level: f32, up: bool) -> f32 {
    (if up {
        level * EXPOSURE_STEP
    } else {
        level / EXPOSURE_STEP
    })
    .clamp(EXPOSURE_MIN, EXPOSURE_MAX)
}

/// Maps an exposure multiplier to the camera `ev100` value.
///
/// `1.0` reads `0.0` (the pre-#157 look); each doubling brightens one stop.
#[must_use]
pub(crate) fn ev100_for(level: f32) -> f32 {
    -level.max(f32::MIN_POSITIVE).log2()
}

/// Spawns the single 3D camera.
///
/// Filmic (`TonyMcMapface`) tonemapping carries HDR brights into screen
/// range, [`Bloom`] glows the brightest sources, and [`Exposure`] opens at
/// the pre-#157 look. The near plane is rewritten every frame by
/// `sync_camera` from the dive distance.
pub(crate) fn spawn_indicator_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("indicator camera"),
        Camera3d::default(),
        // Filmic curve: brights desaturate instead of clipping to white.
        Tonemapping::TonyMcMapface,
        Bloom::NATURAL,
        Exposure { ev100: 0.0 },
        Transform::from_translation(to_vec3(START_OFFSET)).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Applies the manual [`ExposureLevel`] to the camera (#157).
///
/// Skipped when there is not exactly one 3D camera.
pub(crate) fn sync_exposure(
    exposure: Res<ExposureLevel>,
    mut cameras: Query<&mut Exposure, With<Camera3d>>,
) {
    let ev100 = ev100_for(exposure.0);
    for mut slot in &mut cameras {
        slot.ev100 = ev100;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposure_opens_at_the_pre_change_look() {
        assert_eq!(ExposureLevel::default(), ExposureLevel(1.0));
        assert_eq!(ev100_for(1.0), 0.0);
    }

    #[test]
    fn exposure_steps_clamp_at_two_stops() {
        assert_eq!(step_exposure(1.0, true), EXPOSURE_STEP);
        assert_eq!(step_exposure(1.0, false), 1.0 / EXPOSURE_STEP);
        assert_eq!(step_exposure(EXPOSURE_MAX, true), EXPOSURE_MAX);
        assert_eq!(step_exposure(EXPOSURE_MIN, false), EXPOSURE_MIN);
        assert!(ev100_for(EXPOSURE_MAX) < ev100_for(EXPOSURE_MIN));
    }
}
