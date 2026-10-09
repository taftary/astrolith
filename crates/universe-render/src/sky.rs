//! Horizon rings and sky arcs for the tail surface levels (#394).
//!
//! Indicators only: the curved ground meets the sky in a gizmo circle at
//! the true horizon distance, and air reads as a tinted sky circle (plus
//! haze bands at L11) at the air height. Everything here is `pub(crate)`:
//! only this crate uses it.

use crate::Universe;
use crate::forms::{draw_form, map_form, turn_form_normals};
use crate::style::{point_color_for_level, scaled, to_vec3};
use bevy::math::Isometry3d;
use bevy::prelude::*;
use universe_core::coords::ParentUnits;
use universe_core::frame::{horizon_distance, parent_to_child};
use universe_core::nav::sibling_in_open_units;

/// Brightness of the horizon ring (constant faint ground line, #394).
const HORIZON_BRIGHTNESS: f32 = 0.35;

/// Constant faint brightness of parent context and tail siblings (#394).
///
/// Below [`universe_core::nest::SHELL_FLOOR`] (0.15), held until the next
/// open: the level you came from stays readable but never competes.
pub(crate) const CONTEXT_FLOOR: f32 = 0.08;

/// Brightness of the sky circle (#394).
const SKY_BRIGHTNESS: f32 = 0.5;

/// Brightness steps of the three L11 haze bands, lowest first (#394).
const HAZE_BRIGHTNESS: [f32; 3] = [0.15, 0.25, 0.35];

/// Horizontal-circle rotation (gizmo circles face `+Z` by default).
fn flat_rotation() -> Quat {
    Quat::from_rotation_arc(Vec3::Z, Vec3::Y)
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
/// L12 draws the sky circle alone; L13 draws nothing. Airless worlds draw
/// nothing at any level. Allocates nothing per frame (`E-HOT-NOALLOC`).
pub(crate) fn draw_sky(mut gizmos: Gizmos, universe: Res<Universe>) {
    let level = universe.level().get();
    if level != 11 && level != 12 {
        return;
    }
    let Some(surface) = universe.surface_context() else {
        return;
    };
    let Some(tint) = surface.air_tint else {
        return;
    };
    if !surface.air_thickness.is_finite() || surface.air_thickness <= 0.0 {
        return;
    }
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
/// (centre `[0, -R, 0]` from the surface context); at L12 the region rect,
/// at L13 the city blocks, at L14 the building walls (the entered body
/// mapped through the open frame like a sibling). Brightness is the
/// constant [`CONTEXT_FLOOR`]. Allocates nothing per frame
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
    if matches!(dot.form, universe_core::r#gen::Form::Dot) {
        return;
    }
    let parent_level = universe.level().shallower().unwrap_or(universe.level());
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
            scaled(point_color_for_level(parent_level), CONTEXT_FLOOR),
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
                scaled(air, CONTEXT_FLOOR),
            );
        }
        return;
    }
    // L12-L14: the entered body mapped into the open frame at 1/ratio with
    // the open orientation, drawn at the context floor.
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
    draw_form(
        &mut gizmos,
        form,
        to_vec3(world),
        to_vec3(centre),
        own.max(1e-6),
        scaled(point_color_for_level(parent_level), CONTEXT_FLOOR),
    );
}
