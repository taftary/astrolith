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
use universe_core::flight::{ease_heading, nearest_surface_distance, orthogonal_up};
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
/// Driven by `E` (dim) and `Shift+E` (brighten) through the settings store
/// ([`crate::settings_ui`]); applied to the camera [`Exposure`] by
/// [`sync_exposure`]. Render-only state: it never touches snapshots or the
/// golden files.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub(crate) struct ExposureLevel(pub f32);

impl Default for ExposureLevel {
    fn default() -> ExposureLevel {
        ExposureLevel(1.0)
    }
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
        camera_projection(),
        // Filmic curve: brights desaturate instead of clipping to white.
        FILMIC_TONEMAPPING,
        Bloom::NATURAL,
        Exposure { ev100: 0.0 },
        Transform::from_translation(to_vec3(START_OFFSET)).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// The camera's perspective: the 70° vertical field of view every pixel
/// measure and the LOD detail rule assume (`precision::FOV_DEG`, #778 F1 A),
/// on Bevy's infinite reverse-Z projection (#778 Q2 A).
#[must_use]
pub(crate) fn camera_projection() -> Projection {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: render-domain narrowing of the field of view, intended"
    )]
    let fov = universe_core::precision::FOV_DEG.to_radians() as f32;
    Projection::Perspective(PerspectiveProjection { fov, ..default() })
}

/// The eyes' exposure offset in EV at the planet (#908): eased toward
/// [`universe_core::light::adapted_ev_offset`] of the view's light; zero off
/// the planet and with Eye adaptation off. Negative brightens.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct EyeAdaptation(pub f64);

/// The camera's `ev100` for the manual `level` (B / Shift+B) plus the eyes'
/// `offset` (#908 AC3): the manual step stays on top of the adaptation.
/// Pure.
#[must_use]
pub(crate) fn exposure_ev(level: f32, offset: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: an exposure offset narrowed for the GPU, intended"
    )]
    let offset = if offset.is_finite() {
        offset as f32
    } else {
        0.0
    };
    ev100_for(level) + offset
}

/// Eases the eyes toward the view's light at the planet (#908 Q1 C, Q2 B,
/// Q3 A): L10 and its ground levels only, from the sun's height over the
/// rendered camera (the L10 cell is planet-centred, so the camera position
/// is its up). Off the planet, or with Eye adaptation off, the offset is
/// zero at once, so L1-L9 look exactly as before.
pub(crate) fn adapt_eyes(
    time: Res<Time>,
    universe: Res<Universe>,
    origin: Res<crate::style::RenderOrigin>,
    store: Res<crate::SettingsStore>,
    mut eyes: ResMut<EyeAdaptation>,
) {
    let sun = universe_core::light::sun_direction(&universe);
    let (Some(sun), true) = (sun, store.0.eye_adaptation) else {
        if eyes.0 != 0.0 {
            eyes.0 = 0.0;
        }
        return;
    };
    // The view's light from the moonlight model (#981 round 2): the fill,
    // the sun's height and the moons, so a moonlit night is brighter for
    // the eyes than a moonless one.
    let night = crate::moon_lights::night_light(Some(&store));
    let share = crate::moon_lights::sky_now(&universe, origin.camera, night).map_or_else(
        || universe_core::light::view_light_share(sun, origin.camera),
        |sky| universe_core::moonlight::view_share(sun, origin.camera, &sky),
    );
    let target = universe_core::light::adapted_ev_offset(share);
    let next = universe_core::light::ease_ev(eyes.0, target, f64::from(time.delta_secs()));
    if next != eyes.0 {
        eyes.0 = next;
    }
}

/// Applies the manual [`ExposureLevel`] (#157) plus the eyes' offset
/// ([`EyeAdaptation`], #908) to the camera.
///
/// Skipped when there is not exactly one 3D camera.
pub(crate) fn sync_exposure(
    exposure: Res<ExposureLevel>,
    eyes: Res<EyeAdaptation>,
    mut cameras: Query<&mut Exposure, With<Camera3d>>,
) {
    let ev100 = exposure_ev(exposure.0, eyes.0);
    for mut slot in &mut cameras {
        if slot.ev100 != ev100 {
            slot.ev100 = ev100;
        }
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

/// Absolute slew cap of the camera up per frame, in radians (#458).
///
/// Proportional easing alone steps `fraction * angle`: near-polar region
/// approaches swing the L10 blend target through almost 180 degrees and
/// step about 0.09 rad in one frame. The cap keeps every frame an order
/// below a radian (the window feel bound is 0.05); small errors ease
/// exactly as before, large ones take more frames to converge.
pub(crate) const UP_STEP_MAX: f64 = 0.03;

/// Dot product of two direction vectors (render-local camera math).
fn dot_units(a: [f64; 3], b: [f64; 3]) -> f64 {
    let [ax, ay, az] = a;
    let [bx, by, bz] = b;
    ax * bx + ay * by + az * bz
}

/// Moves unit `from` toward unit `to` by at most `max_step` radians.
///
/// Slerp interpolation, so the travelled angle is exact; degenerate or
/// non-finite inputs hold `from`, never a panic. Render-only camera math:
/// replays never call it, so the headless journey stays byte-identical.
#[must_use]
fn slew_toward(from: [f64; 3], to: [f64; 3], max_step: f64) -> [f64; 3] {
    if !from.iter().all(|c| c.is_finite()) || !to.iter().all(|c| c.is_finite()) {
        return from;
    }
    if !(max_step.is_finite() && max_step > 0.0) {
        return from;
    }
    let cosine = dot_units(from, to).clamp(-1.0, 1.0);
    let angle = cosine.acos();
    if !angle.is_finite() || angle <= max_step {
        return to;
    }
    let sine = angle.sin();
    if !sine.is_finite() || sine <= 1e-12 {
        return from;
    }
    let travelled = max_step / angle;
    let a = ((1.0 - travelled) * angle).sin() / sine;
    let b = (travelled * angle).sin() / sine;
    let [fx, fy, fz] = from;
    let [tx, ty, tz] = to;
    [fx * a + tx * b, fy * a + ty * b, fz * a + tz * b]
}

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
/// the surface itself, the open cell's center. `None` only when neither holds
/// (free flight, or before the first input frame), so the heading holds.
#[must_use]
pub(crate) fn look_point(on_the_way: Option<[f64; 3]>, arrived: bool) -> Option<[f64; 3]> {
    on_the_way.or(arrived.then_some([0.0; 3]))
}

/// Next camera up for one frame: eased toward the local up, made
/// perpendicular to `forward`, slew-capped, so the roll is well defined
/// and bounded (#403, #430, #458).
///
/// The target is the open frame's `+Y` everywhere except near the L10
/// planet, where [`local_up`] blends to the ground normal by altitude. `dt`
/// is the frame time in seconds. The returned up never rolls more than
/// [`UP_STEP_MAX`] past the incoming `up` in one frame: small errors ease
/// exactly as before, while near-polar approaches (the blend target swings
/// wide) and fast heading turns (the perpendicular projection swings with
/// them) spread over frames instead of snapping. Looking straight down
/// keeps the previous up, never flipped.
#[must_use]
pub(crate) fn next_up(forward: [f64; 3], up: [f64; 3], target: [f64; 3], dt: f64) -> [f64; 3] {
    let settled = ease_heading(up, target, 1.0 - (-UP_RATE * dt).exp());
    let perp = orthogonal_up(forward, settled);
    slew_toward(up, perp, UP_STEP_MAX)
}

/// Syncs camera pose, near plane, and window title from the universe.
///
/// In dive mode the camera sits at the path offset and eases its heading
/// toward the marker on the way to the destination, or the surface itself on
/// arrival (#423). In free flight it
/// faces the drag-look direction with the near plane on the nearest
/// surface. The up settles toward the open frame's `+Y`. The near plane
/// follows the gap to the target (or surface) so tiny deep markers are
/// never clipped (infinite reverse-Z projection, so no far plane is
/// needed). Skipped when there is not exactly one window.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input; the camera reads every mode's pose"
)]
pub(crate) fn sync_camera(
    time: Res<Time>,
    universe: Res<Universe>,
    mut nav: ResMut<Navigation>,
    flight: Res<Flight>,
    normal: Res<crate::player_state::PlayerState>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Camera3d>>,
    mut origin: ResMut<crate::style::RenderOrigin>,
    patch_tree: Option<Res<crate::patches::PatchTree>>,
    window: Single<&mut Window>,
) {
    let level = universe.level();
    if nav.titled != Some(level) {
        nav.titled = Some(level);
        window.into_inner().title = window_title_for_level(level);
    }
    let mut camera = universe.path.offset();
    let dt = f64::from(time.delta_secs());
    let gap = match flight.mode {
        FlightMode::Normal => {
            // One function poses the player's camera (#754 AC3): first,
            // third, or the handover ease from the arrival. The general sync
            // only applies it; the up is the ground's up, unsmoothed.
            use crate::player_camera::{CameraCase, HANDOVER_S, player_camera};
            use universe_core::body::{body_height, eye_of, heading_basis};
            let (eye, up) = eye_of(&universe, &normal.body);
            // The camera follows its own heading (`Flight.yaw`); in third
            // person A/D turn the body and the heading eases back behind it
            // (#899 Spec v5).
            let (ahead, _) = heading_basis(up, flight.yaw);
            let height = body_height(level);
            let case = if normal.handover_from.is_some() && normal.handover_t < HANDOVER_S {
                CameraCase::Handover
            } else if normal.third_person {
                CameraCase::Third
            } else {
                CameraCase::First
            };
            let handover_from = normal.handover_from.unwrap_or(eye);
            let pose = player_camera(
                case,
                eye,
                ahead,
                up,
                height,
                flight.pitch,
                normal.third_person,
                handover_from,
                normal.handover_t,
                [0.0; 3],
                [0.0; 3],
            );
            if let Some((position, forward, up_pose)) = pose {
                camera = position;
                nav.forward = forward;
                nav.up = up_pose;
            }
            height
        }
        FlightMode::Dive => {
            // Released: dive keeps its own rules (#754 AC3).
            let _ = crate::player_camera::player_camera(
                crate::player_camera::CameraCase::Released,
                [0.0; 3],
                [0.0; 3],
                [0.0; 3],
                1.0,
                0.0,
                true,
                [0.0; 3],
                0.0,
                [0.0; 3],
                [0.0; 3],
            );
            let arrived = nav
                .destination
                .is_some_and(|held| held.on_the_way(&universe.path) == Way::Arrived);
            // The player's body wins once the dive reaches its cell (#607).
            let target = nav.focus.or_else(|| {
                look_point(
                    nav.target.and_then(|m| universe.marker(MarkerIndex(m))),
                    arrived,
                )
            });
            nav.forward = next_heading(nav.forward, camera, target, dt);
            target
                .map_or(DVec3::from_array(camera).length(), |position| {
                    let reach = if nav.focus.is_some() {
                        0.0
                    } else {
                        universe.marker_radius()
                    };
                    (DVec3::from_array(camera) - DVec3::from_array(position)).length() - reach
                })
                .max(1e-6)
        }
        FlightMode::Free => {
            // Released: free keeps its own rules (#754 AC3). Spaceship
            // (#884 T2): the view keeps its own forward and up with no
            // gravity easing and no pitch clamp.
            let _ = crate::player_camera::player_camera(
                crate::player_camera::CameraCase::Released,
                [0.0; 3],
                [0.0; 3],
                [0.0; 3],
                1.0,
                0.0,
                true,
                [0.0; 3],
                0.0,
                [0.0; 3],
                [0.0; 3],
            );
            nav.forward = flight.free_forward;
            nav.up = flight.free_up;
            nearest_surface_distance(camera, &universe.open, universe.marker_radius()).max(1e-6)
        }
    };
    if flight.mode == FlightMode::Dive {
        // Dive turn (#884 T1, Q1 A): the base up eases toward the local up
        // as today, but the drawn roll is derived from it without writing
        // back. The stored `roll` is the dragged target; `roll_drawn`
        // eases toward it and stops exactly, so release stops the turn.
        let base = next_up(nav.forward, nav.up, local_up(level, camera), dt);
        nav.up = base;
        nav.roll_drawn = universe_core::dive_orbit::ease_roll(nav.roll_drawn, nav.roll, dt);
    }
    // Camera-relative drawing (ADR 0026): the camera is the render origin.
    // At L10 the origin also carries the planet frame, recomputed only when
    // the open cell changes (seed-keyed, #774 Q4 A, review #774 item 4): one
    // source (`PlanetFrame::for_planet` from the path), no per-frame rebuild,
    // and a missing planet yields no conversion, never an Earth fallback.
    if universe.level().get() == 10 {
        let seed = universe.open_seed();
        if origin.l10_seed != Some(seed) {
            let chain = universe.path.indices();
            let l10: &[u32] = chain.get(..9).unwrap_or(&[]);
            origin.l10 = if l10.len() == 9 {
                Some(universe_core::planet_frame::PlanetFrame::for_planet(
                    universe.root.0,
                    l10,
                ))
            } else {
                None
            };
            origin.l10_seed = Some(seed);
        }
    } else if origin.l10_seed.is_some() {
        origin.l10 = None;
        origin.l10_seed = None;
    }
    // The open planet's relief, kept seed-keyed by `sync_patches` (#799).
    let relief = patch_tree
        .as_ref()
        .filter(|tree| tree.seed == origin.l10_seed)
        .and_then(|tree| tree.relief.as_ref());
    // #995 T9: in a planetary system the nearest body bounds the near plane
    // too, so a planet passed close by is never clipped.
    let gap = if universe.level().get() == 8 {
        gap.min(universe_core::flight::nearest_body_distance(camera, &universe.open).max(1e-6))
    } else {
        gap
    };
    let near = near_plane_world(gap, camera, origin.l10.as_ref(), relief);
    origin.camera = camera;
    let position = Vec3::ZERO;
    let forward = to_vec3(nav.forward);
    // The drawn up carries the eased roll; the stored `nav.up` stays the
    // unrolled base (#884 T1).
    let drawn = if flight.mode == FlightMode::Dive {
        universe_core::dive_orbit::roll_up(nav.forward, nav.up, nav.roll_drawn)
    } else {
        nav.up
    };
    let up = to_vec3(drawn);
    for (mut transform, mut projection) in &mut cameras {
        *transform = Transform::from_translation(position).looking_to(forward, up);
        if let Projection::Perspective(perspective) = &mut *projection {
            perspective.near = near;
        }
    }
}

/// Near plane in world units (#778 Q1 A): open cells, metres at L10 (#780
/// F2 A).
///
/// Everywhere: 5% of the gap to the target or nearest surface, in
/// `[1e-12, 0.1]` cells (tiny deep markers are never clipped). At L10 also
/// at most the planet rule, 5% of the height above the drawn terrain with a
/// 1 mm floor, in every mode, so the ground is never cut away.
fn near_plane_world(
    gap: f64,
    camera: [f64; 3],
    frame: Option<&universe_core::planet_frame::PlanetFrame>,
    relief: Option<&universe_core::relief::Relief>,
) -> f32 {
    let mut near = (gap * 0.05).clamp(1e-12, 0.1);
    if let (Some(frame), Some(relief)) = (frame, relief) {
        let height = relief.height_above_m(frame.from_cell(camera));
        let terrain = universe_core::planet_frame::near_plane_m(height) / frame.metres_per_cell();
        if terrain.is_finite() && terrain > 0.0 {
            near = near.min(terrain.max(1e-12));
        }
    }
    // World units are metres at the planet.
    let near = near * frame.map_or(1.0, |f| f.metres_per_cell());
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: render-domain narrowing of a clamped near plane, intended"
    )]
    let near = near as f32;
    near
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_exposure_stays_on_top() {
        // C3 (#908 AC3): the manual level (B / Shift+B) shifts the exposure
        // by the same step with or without the eyes' offset, and the offset
        // brightens (lowers ev100) on top of it.
        for offset in [0.0, -1.1, 0.4] {
            let step = exposure_ev(2.0, offset) - exposure_ev(1.0, offset);
            assert!((step - (ev100_for(2.0) - ev100_for(1.0))).abs() < 1e-6);
        }
        assert!(
            exposure_ev(1.0, -1.0) < exposure_ev(1.0, 0.0),
            "the night offset brightens"
        );
        assert_eq!(
            exposure_ev(1.0, f64::NAN),
            ev100_for(1.0),
            "a bad offset is ignored"
        );
    }

    #[test]
    fn eyes_rest_off_the_planet() {
        // C2 (#908 Q3 A): off the planet the eyes' offset is zero, so L1-L9
        // look exactly as before.
        let mut app = crate::window_tests::headless_app();
        app.world_mut().resource_mut::<EyeAdaptation>().0 = -1.0;
        app.update();
        assert_eq!(app.world().resource::<Universe>().level().get(), 1);
        assert_eq!(app.world().resource::<EyeAdaptation>().0, 0.0);
    }

    #[test]
    fn projection_is_infinite_reverse_z() {
        // The camera keeps Bevy's default perspective (#778 Q2 A): infinite
        // reverse-Z, so depth is `near / distance` and nothing is far-clipped.
        let Projection::Perspective(mut perspective) = camera_projection() else {
            panic!("camera projection is not perspective");
        };
        // The field of view the pixel measures use (#778 F1 A).
        let want = universe_core::precision::FOV_DEG.to_radians();
        assert!(
            (f64::from(perspective.fov) - want).abs() < 1e-6,
            "fov {}",
            perspective.fov
        );
        perspective.near = 1e-3;
        let clip_from_view = bevy::camera::CameraProjection::get_clip_from_view(&perspective);
        for distance in [0.01f32, 1.0, 1.0e6, 1.0e12] {
            let clip = clip_from_view * Vec4::new(0.0, 0.0, -distance, 1.0);
            let depth = clip.z / clip.w;
            let want = 1e-3 / distance;
            assert!(
                (depth - want).abs() <= want * 1e-5,
                "depth {depth} at {distance}, want {want}"
            );
        }
    }

    #[test]
    fn l10_near_plane_follows_the_terrain() {
        use universe_core::planet_frame::PlanetFrame;
        let journey = universe_core::nest::autopilot_path(universe_core::nav::DEMO_SEED);
        let l10 = journey.get(..9).unwrap_or(&[]);
        let frame = PlanetFrame::for_planet(universe_core::nav::DEMO_SEED, l10);
        let relief = universe_core::relief::Relief::for_planet(universe_core::nav::DEMO_SEED, l10);
        let up = [0.6f64, -0.2, 0.77];
        let len = (up[0] * up[0] + up[1] * up[1] + up[2] * up[2]).sqrt();
        let up = [up[0] / len, up[1] / len, up[2] / len];
        let ground = frame.radius_m() + relief.height_m(up);
        for height in [0.01f64, 1.0, 100.0, 1.0e6] {
            let r = ground + height;
            let camera = frame.to_cell(universe_core::planet_frame::PlanetPos([
                up[0] * r,
                up[1] * r,
                up[2] * r,
            ]));
            // A far gap (the cell centre) must not win over the ground.
            let near = f64::from(near_plane_world(0.49, camera, Some(&frame), Some(&relief)));
            let want = universe_core::planet_frame::near_plane_m(height);
            assert!(
                (near - want).abs() <= want * 1e-3,
                "near {near} m at {height} m, want {want} m"
            );
        }
    }

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
    fn the_camera_looks_at_the_marker_on_the_way_else_the_surface_on_arrival() {
        // #423: the marker on the way wins; on arrival the surface center.
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
        let first = [0.0, 0.8, -0.6];
        let pushed = next_up([0.0, 1.0, 0.0], first, IDENTITY_UP, 0.0);
        let push = (pushed[0] * first[0] + pushed[1] * first[1] + pushed[2] * first[2])
            .clamp(-1.0, 1.0)
            .acos();
        assert!(
            push <= UP_STEP_MAX + 1e-9,
            "looking straight up, the first frame never snaps: {push}"
        );
        let mut leaning = first;
        for _ in 0..120 {
            leaning = next_up([0.0, 1.0, 0.0], leaning, IDENTITY_UP, 0.0);
        }
        assert!(
            leaning[1].abs() < 1e-6,
            "looking straight up, the up leaves the view axis over frames"
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
        assert!(
            angle <= bound,
            "one frame rolls {angle} past the {bound} bound"
        );
        let down = [0.0, -1.0, 0.0];
        let carried = [1.0, 0.0, 0.0];
        let kept = next_up(down, carried, IDENTITY_UP, dt);
        let kept_dot =
            (kept[0] * carried[0] + kept[1] * carried[1] + kept[2] * carried[2]).clamp(-1.0, 1.0);
        assert!(kept_dot.acos() <= bound, "looking down never snaps");
        assert!(kept[0] > 0.9, "looking down keeps the previous up");
    }

    #[test]
    fn brighter_exposure_reads_a_lower_ev100() {
        use universe_core::settings::{EXPOSURE_MAX, EXPOSURE_MIN};
        assert!(ev100_for(EXPOSURE_MAX) < ev100_for(EXPOSURE_MIN));
    }
}
