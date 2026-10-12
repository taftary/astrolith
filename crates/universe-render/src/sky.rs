//! Horizon rings, sky arcs, and ground marks for the tail surface levels
//! (#394, #543).
//!
//! Indicators only: the curved ground meets the sky in a gizmo circle at
//! the true horizon distance, and air reads as a tinted sky circle (plus
//! haze bands at L11) at the air height; airless worlds show a faint gray
//! circle overhead instead. Ground guides (horizon ticks, a centre cross,
//! and a ruler of a round real length) sit on the ground at L11-L13 behind
//! the Ground guides setting, and a room shows one compass tick on a wall.
//! Everything here is `pub(crate)`: only this crate uses it.

use crate::Universe;
use crate::forms::{draw_form, map_form, turn_form_normals, unit};
use crate::style::{point_color_for_level, scaled, to_vec3};
use crate::tokens::SHELL_FALLBACK_TINT;
use bevy::math::Isometry3d;
use bevy::prelude::*;
use universe_core::coords::ParentUnits;
use universe_core::frame::{
    angular_radius, child_world_position_oriented, children_brightness, horizon_distance,
    parent_to_child,
};
use universe_core::nav::{MarkerIndex, preview_surface_context, sibling_in_open_units};
use universe_core::nest::child_ratio;

/// Brightness of the horizon ring (constant faint ground line, #394).
const HORIZON_BRIGHTNESS: f32 = 0.35;

/// Faint brightness floor of parent context and tail siblings (#394, #430).
///
/// Below [`universe_core::nest::SHELL_FLOOR`] (0.15): the level you came
/// from stays readable but never competes. Kept context rests here while
/// diving deeper; at entries and exits it meets the marker's own shell
/// curve instead (see [`parent_context_brightness`]).
pub(crate) const CONTEXT_FLOOR: f32 = 0.08;

/// Brightness of the sky circle (#394).
const SKY_BRIGHTNESS: f32 = 0.5;

/// Brightness of the airless overhead circle (#543): half the sky circle.
const AIRLESS_SKY_BRIGHTNESS: f32 = SKY_BRIGHTNESS * 0.5;

/// Height and radius of the airless overhead circle, in cell units (#543).
const AIRLESS_SKY_HEIGHT: f32 = 0.3;
/// Radius of the airless overhead circle, in cell units.
const AIRLESS_SKY_RADIUS: f32 = 0.1;

/// Half length of a horizon tick as a share of the horizon radius (#543).
const HORIZON_TICK_SHARE: f32 = 0.04;

/// Half length of each arm of the ground centre cross, cell units (#543).
const CROSS_HALF: f32 = 0.02;

/// Ruler start offset from the cell centre along `+Z`, cell units (#543).
const RULER_OFFSET: f32 = 0.04;

/// Ruler tick height as a share of the ruler length (#543).
const RULER_TICK_SHARE: f32 = 0.08;

/// Compass tick length as a share of the room half width (#543).
const COMPASS_SHARE: f32 = 0.15;

/// Segments of the ground guides: four horizon ticks, two cross arms, the
/// ruler, and its three ticks.
pub(crate) const GROUND_GUIDE_SEGMENTS: usize = 10;

/// Brightness steps of the three L11 haze bands, lowest first (#394).
const HAZE_BRIGHTNESS: [f32; 3] = [0.15, 0.25, 0.35];

/// Brightness of kept parent context around the open cell (#430).
///
/// The entered marker's own shell curve from the mapped camera, floored at
/// [`CONTEXT_FLOOR`]: at the open instant the mapped camera is the parent
/// camera, so this equals the marker's brightness (`SHELL_FLOOR`) and
/// entries and exits change nothing; deeper inside it rests faint so the
/// previous level stays readable but never competes. Pure.
#[must_use]
pub(crate) fn parent_context_brightness(
    form: universe_core::r#gen::Form,
    position: [f64; 3],
    radius: f64,
    mapped_camera: [f64; 3],
    entered_position: [f64; 3],
) -> f32 {
    use crate::forms::gate_radius;
    use universe_core::frame::{angular_radius, shell_brightness};
    let gate = gate_radius(form, to_vec3(position), Vec3::ZERO, radius);
    let distance = ((mapped_camera[0] - entered_position[0]).powi(2)
        + (mapped_camera[1] - entered_position[1]).powi(2)
        + (mapped_camera[2] - entered_position[2]).powi(2))
    .sqrt();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: render-domain narrowing of a brightness, intended"
    )]
    let curved = shell_brightness(angular_radius(gate, distance)) as f32;
    curved.max(CONTEXT_FLOOR)
}

/// Brightness of the L10 limb preview around the targeted region (#430).
///
/// Fades in by the target's angular size times the kept-context level, so
/// at the open instant it equals the post-entry limb brightness
/// (`SHELL_FLOOR`) and the mesh swap shows no pop; invisible while the
/// region is a point. Pure.
#[must_use]
pub(crate) fn limb_preview_brightness(target_angular: f64) -> f32 {
    use universe_core::frame::{children_brightness, shell_brightness};
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: render-domain narrowing of a brightness, intended"
    )]
    let faded =
        children_brightness(target_angular) as f32 * shell_brightness(target_angular) as f32;
    faded
}

/// Horizontal-circle rotation (gizmo circles face `+Z` by default).
fn flat_rotation() -> Quat {
    Quat::from_rotation_arc(Vec3::Z, Vec3::Y)
}

/// Circle rotation facing the marker frame's up (#430).
///
/// Identity `up` matches [`flat_rotation`]; a patch normal tilts the
/// circle into the tangent plane, so previewed horizons and skies sit flat
/// on their patch.
fn facing_rotation(up: [f64; 3]) -> Quat {
    Quat::from_rotation_arc(Vec3::Z, unit(up))
}

/// Draws the next level's horizon, sky, and haze inside its markers (#430).
///
/// What the child draws around its content is visible before entry,
/// exactly like previewed children: the region's horizon and sky inside
/// the planet's region patches, the city's inside the region, the horizon
/// inside the city. Each marker fades its context in by
/// [`children_brightness`] of its angular size, at the true child scale
/// (lengths times the child ratio, positions through the oriented preview
/// map), with the same level rules the open cell uses (horizon at L11-L13,
/// sky at L11-L12 with haze bands at L11, nothing airless, no sky at L13,
/// room shells come through forms). Leaving fades them out the same way.
/// Allocates nothing per frame (`E-HOT-NOALLOC`).
pub(crate) fn draw_preview_context(
    mut gizmos: Gizmos,
    universe: Res<Universe>,
    previews: Res<crate::PreviewCache>,
) {
    let (Some(ratio), Some(child_level)) =
        (child_ratio(universe.level()), universe.level().deeper())
    else {
        return;
    };
    let child = child_level.get();
    if child == 14 {
        return;
    }
    let Some(context) = preview_surface_context(&universe) else {
        return;
    };
    if !context.planet_radius.is_finite() || context.planet_radius <= 0.0 {
        return;
    }
    let camera = universe.path.offset();
    if !camera.iter().all(|component| component.is_finite()) {
        return;
    }
    if !ratio.is_finite() || ratio <= 0.0 {
        return;
    }
    let marker_radius = universe.marker_radius();
    let ground_color = point_color_for_level(child_level);
    for (marker, _) in previews.entries() {
        let Some(marker_pos) = universe.marker(MarkerIndex(*marker)) else {
            continue;
        };
        let marker_up = universe
            .open
            .points
            .get(*marker as usize)
            .and_then(|point| match point.form {
                universe_core::r#gen::Form::Patch { normal } => Some(normal),
                _ => None,
            })
            .unwrap_or(universe_core::frame::IDENTITY_UP);
        let marker_distance = ((camera[0] - marker_pos[0]).powi(2)
            + (camera[1] - marker_pos[1]).powi(2)
            + (camera[2] - marker_pos[2]).powi(2))
        .sqrt();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a brightness, intended"
        )]
        let brightness = children_brightness(angular_radius(marker_radius, marker_distance)) as f32;
        if brightness <= 0.0 {
            continue;
        }
        // Camera in child units through the oriented preview map.
        let relative = [
            camera[0] - marker_pos[0],
            camera[1] - marker_pos[1],
            camera[2] - marker_pos[2],
        ];
        let turned = parent_to_child(marker_up, relative);
        let child_camera = [turned[0] / ratio, turned[1] / ratio, turned[2] / ratio];
        if !child_camera.iter().all(|component| component.is_finite()) {
            continue;
        }
        let rotation = facing_rotation(marker_up);
        // Horizon ring on the child ground under the camera, at the true
        // child horizon distance, mapped back at the true child size.
        if (11..=13).contains(&child) {
            let horizon = horizon_distance(context.planet_radius, child_camera[1].abs());
            if horizon.is_finite() && horizon > 0.0 {
                let centre = child_world_position_oriented(
                    ParentUnits(marker_pos),
                    ratio,
                    [child_camera[0], 0.0, child_camera[2]],
                    marker_up,
                );
                let at = to_vec3(centre.0);
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "E-CAST: cell-unit horizon narrowed for the GPU, intended"
                )]
                let wide = (horizon * ratio) as f32;
                gizmos.circle(
                    Isometry3d::new(at, rotation),
                    wide,
                    scaled(ground_color, brightness * HORIZON_BRIGHTNESS),
                );
            }
        }
        // Sky circle (and haze bands for regions) over the child ground.
        if (child == 11 || child == 12)
            && let Some(tint) = context.air_tint
            && context.air_thickness.is_finite()
            && context.air_thickness > 0.0
        {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: air tint narrowed for the GPU, intended"
            )]
            let air = Color::srgb(tint[0] as f32, tint[1] as f32, tint[2] as f32);
            let height = context.air_thickness;
            let sky_at = |fraction: f64| {
                child_world_position_oriented(
                    ParentUnits(marker_pos),
                    ratio,
                    [0.0, height * fraction, 0.0],
                    marker_up,
                )
            };
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: cell-unit sky narrowed for the GPU, intended"
            )]
            let wide = (0.5 * ratio) as f32;
            gizmos.circle(
                Isometry3d::new(to_vec3(sky_at(1.0).0), rotation),
                wide,
                scaled(air, brightness * SKY_BRIGHTNESS),
            );
            if child == 11 {
                for (fraction, haze) in [0.25f64, 0.5, 0.75].iter().zip(HAZE_BRIGHTNESS.iter()) {
                    gizmos.circle(
                        Isometry3d::new(to_vec3(sky_at(*fraction).0), rotation),
                        wide,
                        scaled(air, brightness * *haze),
                    );
                }
            }
        }
    }
}

/// Draws the horizon ring on the ground at L11-L13 (#394).
///
/// The ring sits on the ground plane (`y = 0`) under the camera at the
/// true horizon distance for the camera's height above the ground. Beyond
/// the cell on entry, closing in on descent. Allocates nothing per frame
/// (`E-HOT-NOALLOC`).
pub(crate) fn draw_horizon(mut gizmos: Gizmos, universe: Res<Universe>) {
    let level = universe.level().get();
    if !(11..=13).contains(&level) {
        return;
    }
    let Some(surface) = universe.surface_context() else {
        return;
    };
    if !surface.planet_radius.is_finite() || surface.planet_radius <= 0.0 {
        return;
    }
    let camera = universe.path.offset();
    if !camera.iter().all(|component| component.is_finite()) {
        return;
    }
    let height = camera[1].abs();
    let horizon = horizon_distance(surface.planet_radius, height);
    if !horizon.is_finite() || horizon <= 0.0 {
        return;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cell-unit camera narrowed for the GPU, intended"
    )]
    let centre = Vec3::new(camera[0] as f32, 0.0, camera[2] as f32);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cell-unit horizon narrowed for the GPU, intended"
    )]
    let radius = horizon as f32;
    let color = scaled(point_color_for_level(universe.level()), HORIZON_BRIGHTNESS);
    gizmos.circle(Isometry3d::new(centre, flat_rotation()), radius, color);
}

/// Draws the sky circle (and haze bands at L11) over the ground (#394).
///
/// L11 draws a sky circle at the air height plus three haze bands below it;
/// L12 draws the sky circle alone; L13 draws nothing. Airless worlds draw a
/// faint gray circle overhead at L11-L12 instead (#543). Allocates nothing
/// per frame (`E-HOT-NOALLOC`).
pub(crate) fn draw_sky(mut gizmos: Gizmos, universe: Res<Universe>) {
    let level = universe.level().get();
    if level != 11 && level != 12 {
        return;
    }
    let Some(surface) = universe.surface_context() else {
        return;
    };
    let airless = surface.air_tint.is_none()
        || !surface.air_thickness.is_finite()
        || surface.air_thickness <= 0.0;
    if airless {
        // A world without air still gets a faint gray mark overhead (#543),
        // never as bright as the sky circle of a world with air.
        gizmos.circle(
            Isometry3d::new(Vec3::new(0.0, AIRLESS_SKY_HEIGHT, 0.0), flat_rotation()),
            AIRLESS_SKY_RADIUS,
            scaled(SHELL_FALLBACK_TINT, AIRLESS_SKY_BRIGHTNESS),
        );
        return;
    }
    let Some(tint) = surface.air_tint else {
        return;
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: air tint narrowed for the GPU, intended"
    )]
    let air = Color::srgb(tint[0] as f32, tint[1] as f32, tint[2] as f32);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cell-unit air height narrowed for the GPU, intended"
    )]
    let height = surface.air_thickness as f32;
    // Sky dome rim at the cell edge, at the air height.
    gizmos.circle(
        Isometry3d::new(Vec3::new(0.0, height, 0.0), flat_rotation()),
        0.5,
        scaled(air, SKY_BRIGHTNESS),
    );
    if level == 11 {
        for (fraction, brightness) in [0.25f32, 0.5, 0.75].iter().zip(HAZE_BRIGHTNESS.iter()) {
            gizmos.circle(
                Isometry3d::new(Vec3::new(0.0, height * fraction, 0.0), flat_rotation()),
                0.5,
                scaled(air, *brightness),
            );
        }
    }
}

/// Draws the entered parent marker's body in the open frame (#394, T5).
///
/// The level you came from stays faintly around you at the true ratio
/// until the next open: at L11 the planet limb circle plus its air rim
/// (centre `[0, -R, 0]` from the surface context); at L14 the building
/// walls (the entered box mapped through the open frame like a sibling);
/// at L12-L13 the entered spot's footprint as a flat ring on the ground
/// (#543). Brightness follows the
/// entered marker's own shell curve from the mapped camera (#430), so
/// entries and exits change nothing, floored at [`CONTEXT_FLOOR`] so the
/// context persists while diving deeper. Allocates nothing per frame
/// (`E-HOT-NOALLOC`).
pub(crate) fn draw_parent_context(mut gizmos: Gizmos, universe: Res<Universe>) {
    let open = universe.level().get();
    if !(11..=14).contains(&open) {
        return;
    }
    let (Some(parent), Some(entered)) = (&universe.parent, universe.path.entered()) else {
        return;
    };
    let Some(dot) = parent.points.get(entered.marker as usize) else {
        return;
    };
    let parent_level = universe.level().shallower().unwrap_or(universe.level());
    // Kept-context brightness (#430): the entered marker's own shell curve
    // from the mapped camera, so entries and exits change nothing.
    let mapped = entered.to_parent_point(universe.path.offset());
    let kept =
        parent_context_brightness(dot.form, dot.position, dot.radius, mapped, entered.position);
    if open == 11 {
        // Planet limb and air around the region, from the surface context.
        let Some(surface) = universe.surface_context() else {
            return;
        };
        if !surface.planet_radius.is_finite() || surface.planet_radius <= 0.0 {
            return;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: cell-unit planet frame narrowed for the GPU, intended"
        )]
        let centre = Vec3::new(0.0, -(surface.planet_radius as f32), 0.0);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: cell-unit planet radius narrowed for the GPU, intended"
        )]
        let radius = surface.planet_radius as f32;
        gizmos.circle(
            Isometry3d::new(centre, flat_rotation()),
            radius,
            scaled(point_color_for_level(parent_level), kept),
        );
        if let Some(tint) = surface.air_tint
            && surface.air_thickness.is_finite()
            && surface.air_thickness > 0.0
        {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: air tint narrowed for the GPU, intended"
            )]
            let air = Color::srgb(tint[0] as f32, tint[1] as f32, tint[2] as f32);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: cell-unit air thickness narrowed for the GPU, intended"
            )]
            let rim = radius + surface.air_thickness as f32;
            gizmos.circle(
                Isometry3d::new(centre, flat_rotation()),
                rim,
                scaled(air, kept),
            );
        }
        return;
    }
    // L12-L14: the entered body mapped into the open frame at 1/ratio with
    // the open orientation. The building shell fades toward the context
    // floor as the room is approached (#430).
    if !entered.ratio.is_finite() || entered.ratio <= 0.0 {
        return;
    }
    let scale = 1.0 / entered.ratio;
    let map = |local: [f64; 3]| sibling_in_open_units(entered, ParentUnits(local)).0.0;
    let unturn = |normal: [f64; 3]| parent_to_child(entered.up, normal);
    let world = map(dot.position);
    let centre = map(dot.portal_position());
    let form = turn_form_normals(map_form(dot.form, map, scale), unturn);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: render-domain narrowing of a ratio, intended"
    )]
    let scale_f = scale as f32;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: render-domain narrowing of a radius, intended"
    )]
    let own = dot.radius as f32 * scale_f;
    if keeps_footprint_ring(form) {
        // An entered dot (a region or city spot) keeps a dim flat ring on
        // the ground around you: the footprint of the place you came from
        // (#543), on the same kept-context curve as every other body.
        gizmos.circle(
            Isometry3d::new(to_vec3(world), flat_rotation()),
            own.max(1e-6),
            scaled(point_color_for_level(parent_level), kept),
        );
        return;
    }
    draw_form(
        &mut gizmos,
        form,
        to_vec3(world),
        to_vec3(centre),
        own.max(1e-6),
        scaled(point_color_for_level(parent_level), kept),
    );
}

/// Whether an entered body is kept as a flat footprint ring (#543).
///
/// Entered dots (region and city spots at L12-L13) have no outline of
/// their own, so the place you came from stays as a dim ring on the
/// ground; every other form draws itself at the kept brightness.
#[must_use]
pub(crate) fn keeps_footprint_ring(form: universe_core::r#gen::Form) -> bool {
    matches!(form, universe_core::r#gen::Form::Dot)
}

/// Segments of the ground guides for a horizon of radius `horizon` around
/// `centre` (the camera foot, `y = 0`), a ground height reader, and a ruler
/// `ruler` cell units long (#543).
///
/// Four radial ticks across the horizon ring at `+X`, `-X`, `+Z`, `-Z`; a
/// centre cross on the ground at the cell centre; the ruler along `+X` from
/// just off the centre on the ground, with ticks at its start, middle, and
/// end. Fixed array, no allocation (`E-HOT-NOALLOC`).
#[must_use]
pub(crate) fn ground_guide_segments(
    centre: Vec3,
    horizon: f32,
    ground: impl Fn(f32, f32) -> f32,
    ruler: f32,
) -> [(Vec3, Vec3); GROUND_GUIDE_SEGMENTS] {
    let tick = horizon * HORIZON_TICK_SHARE;
    let radial = |direction: Vec3| {
        (
            centre + direction * (horizon - tick),
            centre + direction * (horizon + tick),
        )
    };
    let at = |x: f32, z: f32| Vec3::new(x, ground(x, z), z);
    let start = at(0.0, RULER_OFFSET);
    let middle = at(ruler * 0.5, RULER_OFFSET);
    let end = at(ruler, RULER_OFFSET);
    let up = Vec3::Y * ruler * RULER_TICK_SHARE;
    [
        radial(Vec3::X),
        radial(Vec3::NEG_X),
        radial(Vec3::Z),
        radial(Vec3::NEG_Z),
        (at(-CROSS_HALF, 0.0), at(CROSS_HALF, 0.0)),
        (at(0.0, -CROSS_HALF), at(0.0, CROSS_HALF)),
        (start, end),
        (start, start + up),
        (middle, middle + up),
        (end, end + up),
    ]
}

/// Draws the ground guides at L11-L13 when the setting is on (#543).
///
/// Horizon ticks, the centre cross, and the ruler of
/// [`universe_core::labels::ground_ruler`], all at the horizon ring's
/// brightness: faint context, never as bright as anything enterable.
pub(crate) fn draw_ground_guides(
    mut gizmos: Gizmos,
    universe: Res<Universe>,
    store: Res<crate::SettingsStore>,
) {
    if !store.0.ground_guides {
        return;
    }
    let Some((ruler, _)) = universe_core::labels::ground_ruler(universe.level()) else {
        return;
    };
    let Some(surface) = universe.surface_context() else {
        return;
    };
    let camera = universe.path.offset();
    if !camera.iter().all(|component| component.is_finite()) {
        return;
    }
    let horizon = horizon_distance(surface.planet_radius, camera[1].abs());
    if !horizon.is_finite() || horizon <= 0.0 {
        return;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cell-unit guides narrowed for the GPU, intended"
    )]
    let (centre, horizon, ruler) = (
        Vec3::new(camera[0] as f32, 0.0, camera[2] as f32),
        horizon as f32,
        ruler as f32,
    );
    let frame = universe.frame.as_ref();
    let ground = |x: f32, z: f32| {
        frame.map_or(0.0, |frame| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: cell-unit ground height narrowed for the GPU, intended"
            )]
            let height =
                universe_core::ground::ground_height(frame, f64::from(x), f64::from(z)) as f32;
            height
        })
    };
    let color = scaled(point_color_for_level(universe.level()), HORIZON_BRIGHTNESS);
    for (from, to) in ground_guide_segments(centre, horizon, ground, ruler) {
        gizmos.line(from, to, color);
    }
}

/// The room compass tick for a shell of half width `half` and height
/// `height` standing at `floor` (#543): a short line at mid height on the
/// `+X` wall, pointing into the room.
#[must_use]
pub(crate) fn compass_tick(half: f32, height: f32, floor: f32) -> (Vec3, Vec3) {
    let y = floor + height * 0.5;
    (
        Vec3::new(half, y, 0.0),
        Vec3::new(half * (1.0 - COMPASS_SHARE), y, 0.0),
    )
}

/// Draws the room compass tick on the `+X` wall at L14 (#543).
///
/// The room shell is the open cell's last point (a standing box); the tick
/// shows one fixed direction, at the horizon ring's faint brightness.
pub(crate) fn draw_room_compass(mut gizmos: Gizmos, universe: Res<Universe>) {
    if universe.level().get() != 14 {
        return;
    }
    let Some(shell) = universe.open.points.last() else {
        return;
    };
    let universe_core::r#gen::Form::Box { height } = shell.form else {
        return;
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cell-unit room shell narrowed for the GPU, intended"
    )]
    let (half, height, floor) = (shell.radius as f32, height as f32, shell.position[1] as f32);
    let (from, to) = compass_tick(half, height, floor);
    gizmos.line(
        from,
        to,
        scaled(point_color_for_level(universe.level()), HORIZON_BRIGHTNESS),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use universe_core::nest::{CLOSE_ANGLE, OPEN_ANGLE, PREVIEW_ANGLE, SHELL_FLOOR};

    #[test]
    fn ground_guides_sit_on_the_ground_and_the_horizon() {
        let centre = Vec3::new(0.1, 0.0, -0.05);
        let ground = |x: f32, z: f32| 0.01 * x - 0.02 * z;
        let segments = ground_guide_segments(centre, 2.0, ground, 0.2);
        assert_eq!(segments.len(), GROUND_GUIDE_SEGMENTS);
        for (from, to) in &segments[..4] {
            let mid = (*from + *to) * 0.5;
            assert!(
                ((mid - centre).length() - 2.0).abs() < 1e-5,
                "tick off the horizon"
            );
        }
        for (from, to) in &segments[4..7] {
            for end in [from, to] {
                assert!(
                    (end.y - ground(end.x, end.z)).abs() < 1e-6,
                    "guide off the ground"
                );
            }
        }
        let (start, end) = segments[6];
        assert!(((end - start).length() - 0.2).abs() < 1e-3, "ruler length");
        const {
            assert!(
                (HORIZON_BRIGHTNESS as f64) < universe_core::contrast::ACTIONABLE_BRIGHTNESS_FLOOR,
                "guides stay under the actionable floor"
            );
        }
    }

    #[test]
    fn entered_dots_keep_a_footprint_ring() {
        use universe_core::r#gen::Form;
        assert!(
            keeps_footprint_ring(Form::Dot),
            "an entered spot stays as a ring"
        );
        assert!(
            !keeps_footprint_ring(Form::Box { height: 0.05 }),
            "an entered building draws its own walls"
        );
        assert!(!keeps_footprint_ring(Form::Patch {
            normal: [0.0, 1.0, 0.0]
        }));
    }

    #[test]
    fn compass_tick_lies_on_the_wall_and_points_inward() {
        let (from, to) = compass_tick(0.3, 0.4, -0.2);
        assert_eq!(from, Vec3::new(0.3, 0.0, 0.0));
        assert!(to.x < from.x && to.x > 0.0, "tick points into the room");
        assert_eq!(to.y, from.y);
    }

    #[test]
    fn airless_mark_stays_fainter_than_the_sky() {
        const {
            assert!(AIRLESS_SKY_BRIGHTNESS < SKY_BRIGHTNESS);
            assert!(AIRLESS_SKY_HEIGHT < 0.5 && AIRLESS_SKY_RADIUS < 0.5);
        }
    }

    #[test]
    fn kept_context_meets_the_marker_curve_at_entries_and_exits() {
        use universe_core::r#gen::Form;
        // A dot marker at the open distance: the kept brightness equals the
        // marker's own shell brightness, so the open frame swaps nothing.
        let form = Form::Dot;
        let position = [0.3, 0.0, 0.1];
        let radius = 0.01;
        let rest = radius / OPEN_ANGLE.sin();
        let on_normal = [position[0], position[1] + rest, position[2]];
        let kept = parent_context_brightness(form, position, radius, on_normal, position);
        assert!((f64::from(kept) - SHELL_FLOOR).abs() < 1e-6);
        // Deep inside the child the context rests at the floor.
        let deep = [position[0], position[1] + radius * 1.05, position[2]];
        assert_eq!(
            parent_context_brightness(form, position, radius, deep, position),
            CONTEXT_FLOOR
        );
        // At the close distance it equals the marker's brightness again, so
        // backing out swaps nothing either.
        let far = 0.5 / CLOSE_ANGLE.sin() * 0.99;
        let out = [far, 0.0, 0.0];
        let marker = [0.0; 3];
        let kept_far = parent_context_brightness(form, marker, 0.5, out, marker);
        let shell = {
            use universe_core::frame::{angular_radius, shell_brightness};
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: test brightness narrowed for the GPU, intended"
            )]
            let curved = shell_brightness(angular_radius(0.5, far)) as f32;
            curved
        };
        assert!((kept_far - shell.max(CONTEXT_FLOOR)).abs() < 1e-6);
        // Never below the floor, never above full.
        assert!((CONTEXT_FLOOR..=1.0).contains(&kept));
        assert!((CONTEXT_FLOOR..=1.0).contains(&kept_far));
    }

    #[test]
    fn limb_preview_fades_in_and_meets_the_kept_limb_at_open() {
        assert_eq!(limb_preview_brightness(PREVIEW_ANGLE), 0.0);
        assert_eq!(limb_preview_brightness(PREVIEW_ANGLE * 0.5), 0.0);
        assert_eq!(limb_preview_brightness(f64::NAN), 0.0);
        let at_open = limb_preview_brightness(OPEN_ANGLE);
        assert!((f64::from(at_open) - SHELL_FLOOR).abs() < 1e-6);
        let past_open = limb_preview_brightness(OPEN_ANGLE * 1.5);
        assert!(
            past_open <= at_open,
            "the preview never outshines the kept limb"
        );
        assert!(past_open > CONTEXT_FLOOR, "still lit just past open");
        assert!((0.0..=1.0).contains(&at_open));
    }
}
