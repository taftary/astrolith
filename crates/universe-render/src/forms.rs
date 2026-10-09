//! Form-to-gizmo mapping: every marker body drawn as what it is (#384).
//!
//! Everything here is `pub(crate)`: only this crate uses it. [`draw_form`]
//! maps a core [`Form`](universe_core::gen::Form) to gizmo primitives at a
//! body centre; the level-of-detail decision (dot below [`FORM_ANGLE`],
//! form above) lives with the callers in [`crate::draw`]. All geometry is
//! `f32` computed from render inputs: nothing here feeds back into core
//! (`E-CORE-NO-BEVY`, `E-HOT-NOALLOC`: fixed stack arrays only, no
//! per-frame allocation).

use crate::style::to_vec3;
use bevy::math::{Quat, Vec3};
use bevy::prelude::*;
use universe_core::r#gen::Form;

/// Circle resolution for form outlines (small: forms stay cheap on the dev PC).
pub(crate) const FORM_RESOLUTION: u32 = 24;

/// Segments of an arm spiral polyline (fixed stack array, no allocation).
const ARM_SEGMENTS: usize = 25;

/// Half-lines each way of a ground grid (fixed loop, no allocation).
const GRID_HALF: i32 = 4;

/// Jittered offsets of a clump's dots, in units of the body radius.
const CLUMP_OFFSETS: [[f32; 3]; 5] = [
    [0.0, 0.0, 0.0],
    [0.5, 0.2, 0.1],
    [-0.4, 0.3, -0.2],
    [0.1, -0.5, 0.3],
    [-0.2, -0.1, 0.5],
];

/// Unit render vector for a cell-unit normal, defaulting to `+Z`.
fn unit(normal: [f64; 3]) -> Vec3 {
    to_vec3(normal).try_normalize().unwrap_or(Vec3::Z)
}

/// Isometry at `at` facing `normal` (circles and ellipses face `+Z`).
fn facing(at: Vec3, normal: [f64; 3]) -> Isometry3d {
    Isometry3d::new(at, Quat::from_rotation_arc(Vec3::Z, unit(normal)))
}

/// A tangent perpendicular to `normal` for bar lines.
fn tangent(normal: [f64; 3]) -> Vec3 {
    let n = unit(normal);
    let axis = if n.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    n.cross(axis).try_normalize().unwrap_or(Vec3::X)
}

/// Draws one arm spiral from `centre` in `normal`'s plane.
///
/// A spiral of one and a half turns from `0.1 * size` out to `size`,
/// starting at `phase`, so arms end at the disk edge. Fixed stack array, so
/// no allocation per frame.
fn draw_arm(
    gizmos: &mut Gizmos,
    centre: Vec3,
    normal: [f64; 3],
    phase: f64,
    size: f32,
    color: Color,
) {
    let n = unit(normal);
    let t = tangent(normal);
    let b = n.cross(t);
    let mut strip = [Vec3::ZERO; ARM_SEGMENTS];
    for (i, slot) in strip.iter_mut().enumerate() {
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: segment index below 32, exactly representable"
        )]
        let step = i as f32 / (ARM_SEGMENTS as f32 - 1.0);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a spiral angle, intended"
        )]
        let angle = (phase as f32) + step * 3.0 * std::f32::consts::PI;
        let radius = size * (0.1 + 0.9 * step);
        *slot = centre + t * angle.cos() * radius + b * angle.sin() * radius;
    }
    gizmos.linestrip(strip, color);
}

/// Draws a ground grid through `body` in `normal`'s plane.
///
/// `2 * GRID_HALF + 1` lines each way spanning `±3 * size`; curved grids sag
/// along `-normal` by `d² / (2 * curvature)` for distance `d` from the
/// centre (the L11 ground wrapping the planet, #384). One `line` call per
/// grid line: no allocation.
pub(crate) fn draw_grid(
    gizmos: &mut Gizmos,
    body: Vec3,
    normal: [f64; 3],
    curvature: f64,
    size: f32,
    color: Color,
) {
    let n = unit(normal);
    let t = tangent(normal);
    let b = n.cross(t);
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: grid index below 8, exactly representable"
    )]
    let span = |k: i32| size * 3.0 * k as f32 / GRID_HALF as f32;
    let sag = |along: f32, across: f32| {
        if curvature > 0.0 {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: render-domain narrowing of a curvature sag, intended"
            )]
            let sag = ((along * along + across * across) / (2.0 * curvature as f32)).min(size);
            -n * sag
        } else {
            Vec3::ZERO
        }
    };
    for k in -GRID_HALF..=GRID_HALF {
        let across = span(k);
        let ends = [span(-GRID_HALF), span(GRID_HALF)];
        gizmos.line(
            body + t * across + b * ends[0] + sag(across, ends[0]),
            body + t * across + b * ends[1] + sag(across, ends[1]),
            color,
        );
        gizmos.line(
            body + b * across + t * ends[0] + sag(ends[0], across),
            body + b * across + t * ends[1] + sag(ends[1], across),
            color,
        );
    }
}

/// Maps a form's anchors from its own cell frame to open units (#384).
///
/// `map` re-expresses positions (thread endpoints); `scale` multiplies
/// linear sizes (ring edges, rect halves, box heights). Normals, phases,
/// spans, and flags cross unchanged. Previews map through
/// `child_world_position`, siblings through `sibling_in_open_units`; the
/// open cell draws unmapped.
#[must_use]
pub(crate) fn map_form(form: Form, map: impl Fn([f64; 3]) -> [f64; 3], scale: f64) -> Form {
    match form {
        Form::Ring {
            normal,
            inner,
            outer,
        } => Form::Ring {
            normal,
            inner: inner * scale,
            outer: outer * scale,
        },
        Form::Thread { to, via } => Form::Thread {
            to: map(to),
            via: map(via),
        },
        Form::Rect { half } => Form::Rect {
            half: [half[0] * scale, half[1] * scale],
        },
        Form::Box { height } => Form::Box {
            height: height * scale,
        },
        other => other,
    }
}

/// Visual extent of one form for the level-of-detail gate (#384).
///
/// Every marker gates on its own visual size: dots and bodies on the point
/// radius, rings on their outer edge, orbits on the orbit radius about
/// `centre`, rectangles and boxes on their largest side. The level radius
/// survives only in navigation math, never in drawing.
#[must_use]
pub(crate) fn gate_radius(form: Form, body: Vec3, centre: Vec3, point_radius: f64) -> f64 {
    match form {
        Form::Ring { outer, .. } => outer,
        Form::Shell | Form::Orbit { .. } => f64::from(body.distance(centre).max(1e-6)),
        Form::Rect { half } => half[0].max(half[1]),
        Form::Box { height } => point_radius.max(height),
        _ => point_radius,
    }
}

/// Draws the body at `body` with radius `size`.
///
/// `size` is the point's own radius in render units; `Ring` inner/outer
/// radii ride on the form itself. System forms (`Ring` bands, `Shell`
/// reservoirs, `Orbit` circles, `Arm` spirals) centre on `centre`: the cell
/// centre in the open cell, the marker position in previews. Everything
/// else draws at the body.
pub(crate) fn draw_form(
    gizmos: &mut Gizmos,
    form: Form,
    body: Vec3,
    centre: Vec3,
    size: f32,
    color: Color,
) {
    match form {
        Form::Dot | Form::Body => {
            gizmos.sphere(Isometry3d::from_translation(body), size, color);
        }
        Form::Shell => {
            // Three great circles about the centre at the body's distance:
            // a reservoir shell has no body of its own, so members keep
            // small radii and the shell reads from their orbits. A fraction
            // of a wireframe sphere's line count.
            let radius = body.distance(centre).max(1e-6);
            for normal in [Vec3::X, Vec3::Y, Vec3::Z] {
                let plane = Isometry3d::new(centre, Quat::from_rotation_arc(Vec3::Z, normal));
                gizmos
                    .circle(plane, radius, color)
                    .resolution(FORM_RESOLUTION);
            }
        }
        Form::Disk { normal, barred } => {
            gizmos.ellipse(facing(body, normal), Vec2::splat(size), color);
            if barred {
                let t = tangent(normal) * size;
                gizmos.line(body - t, body + t, color);
            }
        }
        Form::Spheroid => {
            gizmos.ellipse(Isometry3d::from_translation(body), Vec2::splat(size), color);
            gizmos.ellipse(
                Isometry3d::from_translation(body),
                Vec2::splat(size * 0.6),
                color,
            );
        }
        Form::Clump => {
            for offset in CLUMP_OFFSETS {
                let at = body + Vec3::from_array(offset) * size;
                gizmos.sphere(Isometry3d::from_translation(at), size * 0.25, color);
            }
        }
        Form::Ring {
            normal,
            inner,
            outer,
        } => {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: cell-unit radii narrowed for the GPU, intended"
            )]
            let (inner, outer) = (inner as f32, outer as f32);
            gizmos
                .circle(facing(centre, normal), outer.max(1e-6), color)
                .resolution(FORM_RESOLUTION);
            if inner > 1e-6 {
                gizmos
                    .circle(facing(centre, normal), inner, color)
                    .resolution(FORM_RESOLUTION);
            }
            gizmos.sphere(Isometry3d::from_translation(body), size * 0.3, color);
        }
        Form::Orbit { normal } => {
            let radius = body.distance(centre).max(1e-6);
            gizmos
                .circle(facing(centre, normal), radius, color)
                .resolution(FORM_RESOLUTION);
            gizmos.sphere(Isometry3d::from_translation(body), size, color);
        }
        Form::Arc { normal, span } => {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: render-domain narrowing of an arc span, intended"
            )]
            let span = (span as f32).clamp(0.1, 2.0 * std::f32::consts::PI);
            gizmos
                .arc_3d(span, size, facing(body, normal), color)
                .resolution(FORM_RESOLUTION);
            gizmos.sphere(Isometry3d::from_translation(body), size, color);
        }
        Form::Arm { normal, phase } => {
            draw_arm(gizmos, centre, normal, phase, size, color);
        }
        Form::Thread { to, via } => {
            gizmos.line(body, to_vec3(to), color);
            gizmos.line(body, to_vec3(via), color);
        }
        Form::Sheet { normal } => {
            gizmos.rect(facing(body, normal), Vec2::splat(size * 2.0), color);
        }
        Form::Patch { normal } => {
            gizmos.rect(facing(body, normal), Vec2::splat(size), color);
        }
        Form::Grid { normal, curvature } => {
            draw_grid(gizmos, body, normal, curvature, size, color);
        }
        Form::Rect { half } => {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: cell-unit extents narrowed for the GPU, intended"
            )]
            let extents = Vec2::new(half[0] as f32 * 2.0, half[1] as f32 * 2.0);
            let flat = Isometry3d::new(body, Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2));
            gizmos.rect(flat, extents, color);
        }
        Form::Box { height } => {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: cell-unit height narrowed for the GPU, intended"
            )]
            let half_height = (height as f32 * 0.5).max(1e-6);
            let center = body + Vec3::Y * half_height;
            gizmos.aabb_3d(
                bevy::math::bounding::Aabb3d::new(center, Vec3::new(size, half_height, size)),
                Isometry3d::IDENTITY,
                color,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Form helper math stays total on degenerate input.
    #[test]
    fn degenerate_normals_fall_back_to_axes() {
        assert_eq!(unit([0.0, 0.0, 0.0]), Vec3::Z);
        assert_eq!(unit([f64::NAN, 0.0, 0.0]), Vec3::Z);
        let t = tangent([0.0, 1.0, 0.0]);
        assert!(t.x.abs() + t.z.abs() > 0.9, "tangent must leave +Y: {t:?}");
    }
}
