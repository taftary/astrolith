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
use universe_core::destination::Way;
use universe_core::flight::{
    ease_heading, free_look_direction, nearest_surface_distance, orthogonal_up,
};
use universe_core::frame::local_up;
use universe_core::labels::window_title_for_level;
use universe_core::nav::{MarkerIndex, START_OFFSET};

/// Filmic tonemap that works without the `tonemapping_luts` feature (#157 R1).
///
/// `TonyMcMapface`, `AgX`, and `BlenderFilmic` all require LUT assets the
/// minimal pin does not ship (the window logs an error and falls back);
/// `AcesFitted` is the filmic ACES curve with no LUT needed.
pub(crate) const FILMIC_TONEMAPPING: Tonemapping = Tonemapping::AcesFitted;

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
/// Filmic (`AcesFitted`) tonemapping carries HDR brights into screen range
/// with no LUT feature; [`Bloom`] glows the brightest sources, and
/// [`Exposure`] opens at the pre-#157 look. The near plane is rewritten
/// every frame by `sync_camera` from the dive distance.
pub(crate) fn spawn_indicator_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("indicator camera"),
        Camera3d::default(),
        // Filmic curve: brights desaturate instead of clipping to white.
        FILMIC_TONEMAPPING,
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

/// Turn rate of the heading toward a locked target, per second (#403).
///
/// `1 - exp(-rate * dt)` of the remaining angle per frame: about half a
/// second to settle on a new target, never a snap.
pub(crate) const LOOK_RATE: f64 = 6.0;

/// Settle rate of the camera up toward the open frame's `+Y`, per second.
///
/// Slower than the look so an oriented entry (L10 region patches, #394)
/// rolls the horizon level over about a second and a half instead of
/// snapping it on the entry frame (#403).
pub(crate) const UP_RATE: f64 = 2.0;

/// Next heading for one frame of dive mode (#403). Pure.
///
/// With a target the heading eases toward it at [`LOOK_RATE`]; without
/// one it holds, so a glide after a transition never turns toward the
/// cell center on its own. `dt` is the frame time in seconds.
#[must_use]
pub(crate) fn next_heading(
    forward: [f64; 3],
    camera: [f64; 3],
    target: Option<[f64; 3]>,
    dt: f64,
) -> [f64; 3] {
    let Some(position) = target else {
        return forward;
    };
    let desired = [
        position[0] - camera[0],
        position[1] - camera[1],
        position[2] - camera[2],
    ];
    ease_heading(forward, desired, 1.0 - (-LOOK_RATE * dt).exp())
}

/// Point the dive camera looks at (#423). Pure.
///
/// The marker on the way to the destination when there is one; on arrival
/// the room itself, the open cell's center. `None` only when neither holds
/// (free flight, or before the first input frame), so the heading holds.
#[must_use]
pub(crate) fn look_point(on_the_way: Option<[f64; 3]>, arrived: bool) -> Option<[f64; 3]> {
    on_the_way.or(arrived.then_some([0.0; 3]))
}

/// Next camera up for one frame: eased toward the local up, then made
/// perpendicular to `forward` so the roll is well defined (#403, #430). Pure.
///
/// The target is the open frame's `+Y` everywhere except near the L10
/// planet, where [`local_up`] blends to the ground normal by altitude. `dt`
/// is the frame time in seconds; one frame never rolls more than the
/// `UP_RATE` bound, including when looking straight down (the previous up
/// is kept, never flipped).
#[must_use]
pub(crate) fn next_up(forward: [f64; 3], up: [f64; 3], target: [f64; 3], dt: f64) -> [f64; 3] {
    let settled = ease_heading(up, target, 1.0 - (-UP_RATE * dt).exp());
    orthogonal_up(forward, settled)
}

/// Syncs camera pose, near plane, and window title from the universe.
///
/// In dive mode the camera sits at the path offset and eases its heading
/// toward the marker on the way to the destination, or the room itself on
/// arrival (#423). In free flight it
/// faces the drag-look direction with the near plane on the nearest
/// surface. The up settles toward the open frame's `+Y`. The near plane
/// follows the gap to the target (or surface) so tiny deep markers are
/// never clipped (infinite reverse-Z projection, so no far plane is
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
    let dt = f64::from(time.delta_secs());
    let gap = match flight.mode {
        FlightMode::Dive => {
            let arrived = nav
                .destination
                .is_some_and(|held| held.on_the_way(&universe.path) == Way::Arrived);
            let target = look_point(
                nav.target.and_then(|m| universe.marker(MarkerIndex(m))),
                arrived,
            );
            nav.forward = next_heading(nav.forward, camera, target, dt);
            target
                .map_or(DVec3::from_array(camera).length(), |position| {
                    (DVec3::from_array(camera) - DVec3::from_array(position)).length()
                        - universe.marker_radius()
                })
                .max(1e-6)
        }
        FlightMode::Free => {
            nav.forward = free_look_direction(flight.yaw, flight.pitch);
            nearest_surface_distance(camera, &universe.open, universe.marker_radius()).max(1e-6)
        }
    };
    nav.up = next_up(nav.forward, nav.up, local_up(level, camera), dt);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: render-domain narrowing of a clamped gap, intended"
    )]
    let near = (gap * 0.05).clamp(1e-7, 0.1) as f32;
    let position = to_vec3(camera);
    let forward = to_vec3(nav.forward);
    let up = to_vec3(nav.up);
    for (mut transform, mut projection) in &mut cameras {
        *transform = Transform::from_translation(position).looking_to(forward, up);
        if let Projection::Perspective(perspective) = &mut *projection {
            perspective.near = near;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filmic_tonemap_needs_no_lut_feature() {
        // TonyMcMapface, AgX, and BlenderFilmic require `tonemapping_luts`
        // (the window logs an error and falls back); only the LUT-free
        // filmic curve may ship here.
        assert!(
            matches!(FILMIC_TONEMAPPING, Tonemapping::AcesFitted),
            "filmic choice must stay LUT-free"
        );
    }

    #[test]
    fn exposure_opens_at_the_pre_change_look() {
        assert_eq!(ExposureLevel::default(), ExposureLevel(1.0));
        assert_eq!(ev100_for(1.0), 0.0);
    }

    #[test]
    fn a_glide_without_a_target_keeps_its_heading_instead_of_turning_to_the_center() {
        // After a transition with nothing ahead the camera sits off-center
        // looking away from the origin; the heading must hold (#403 R2).
        let forward = [1.0, 0.0, 0.0];
        let camera = [0.0, 0.0, 5.0];
        assert_eq!(next_heading(forward, camera, None, 1.0 / 60.0), forward);
        assert_eq!(next_heading(forward, camera, None, 10.0), forward);
    }

    #[test]
    fn the_camera_looks_at_the_marker_on_the_way_else_the_room_on_arrival() {
        // #423: the marker on the way wins; on arrival the room's center.
        let marker = Some([0.3, 0.1, -0.2]);
        assert_eq!(look_point(marker, false), marker);
        assert_eq!(look_point(None, true), Some([0.0; 3]));
        assert_eq!(look_point(None, false), None, "nothing to aim at holds");
    }

    #[test]
    fn a_locked_target_turns_the_heading_without_snapping() {
        let forward = [0.0, 0.0, -1.0];
        let camera = [0.0; 3];
        let target = Some([1.0, 0.0, 0.0]);
        let turned = next_heading(forward, camera, target, 1.0 / 60.0);
        let cosine = turned[0] * forward[0] + turned[1] * forward[1] + turned[2] * forward[2];
        assert!(cosine > 0.99, "one frame turns only a few degrees");
        assert!(turned[0] > 0.0, "and turns toward the target");
        let mut heading = forward;
        for _ in 0..600 {
            heading = next_heading(heading, camera, target, 1.0 / 60.0);
        }
        assert!(heading[0] > 0.999, "ten seconds settle on the target");
    }

    #[test]
    fn up_settles_to_frame_y_and_stays_perpendicular_to_forward() {
        use universe_core::frame::IDENTITY_UP;
        let forward = [0.0, 0.0, -1.0];
        let tilted = [0.6, 0.8, 0.0];
        let one_frame = next_up(forward, tilted, IDENTITY_UP, 1.0 / 60.0);
        assert!(
            one_frame[0] < tilted[0] && one_frame[0] > 0.5,
            "rolls gently"
        );
        let mut up = tilted;
        for _ in 0..600 {
            up = next_up(forward, up, IDENTITY_UP, 1.0 / 60.0);
        }
        assert!((up[1] - 1.0).abs() < 1e-6, "settles upright");
        let leaning = next_up([0.0, 1.0, 0.0], [0.0, 0.8, -0.6], IDENTITY_UP, 0.0);
        assert!(
            leaning[1].abs() < 1e-12,
            "looking straight up, the up is pushed out of the view axis"
        );
    }

    #[test]
    fn up_rate_bounds_one_frame_and_looking_down_keeps_up() {
        use universe_core::frame::IDENTITY_UP;
        let forward = [0.0, 0.0, -1.0];
        let up = [0.6, 0.8, 0.0];
        let dt = 1.0 / 60.0;
        let stepped = next_up(forward, up, IDENTITY_UP, dt);
        let dot = (stepped[0] * up[0] + stepped[1] * up[1] + stepped[2] * up[2]).clamp(-1.0, 1.0);
        let angle = dot.acos();
        let bound = UP_RATE * dt + 1e-9;
        assert!(angle <= bound, "one frame rolls {angle} past the {bound} bound");
        let down = [0.0, -1.0, 0.0];
        let carried = [1.0, 0.0, 0.0];
        let kept = next_up(down, carried, IDENTITY_UP, dt);
        let kept_dot = (kept[0] * carried[0] + kept[1] * carried[1] + kept[2] * carried[2])
            .clamp(-1.0, 1.0);
        assert!(kept_dot.acos() <= bound, "looking down never snaps");
        assert!(kept[0] > 0.9, "looking down keeps the previous up");
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
