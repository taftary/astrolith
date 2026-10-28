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
use crate::tokens::{FORM_RESOLUTION, ORBIT_PATH_BRIGHTNESS};
use bevy::math::{Quat, Vec3};
use bevy::prelude::*;
use universe_core::r#gen::Form;

/// Segments of an arm spiral polyline (fixed stack array, no allocation).
const ARM_SEGMENTS: usize = 25;

/// Jittered offsets of a clump's dots, in units of the body radius.
const CLUMP_OFFSETS: [[f32; 3]; 5] = [
    [0.0, 0.0, 0.0],
    [0.5, 0.2, 0.1],
    [-0.4, 0.3, -0.2],
    [0.1, -0.5, 0.3],
    [-0.2, -0.1, 0.5],
];

/// Unit render vector for a cell-unit normal, defaulting to `+Z`.
pub(crate) fn unit(normal: [f64; 3]) -> Vec3 {
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

/// Maps a form's anchors from its own cell frame to open units (#384).
///
/// `map` re-expresses positions (thread endpoints); `scale` multiplies
/// linear sizes (ring edges, rect halves, box heights, grid curvature: a
/// curvature radius is a length, so previews carry the true curvature at
/// their scale, #430). Normals, phases, spans, and flags cross unchanged:
/// callers rotate normals separately through [`turn_form_normals`] when the
/// frame is oriented (#394).
/// Previews map through `child_world_position`, siblings through
/// `sibling_in_open_units`; the open cell draws unmapped.
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
        Form::Box { height } => Form::Box {
            height: height * scale,
        },
        other => other,
    }
}

/// Rotates a form's normals through an oriented frame (#394).
///
/// `turn` maps direction vectors (never positions): previews turn child
/// normals into parent units, siblings turn parent normals into open units.
/// Thread endpoints are positions and cross untouched (the caller maps them
/// through `map_form`).
#[must_use]
pub(crate) fn turn_form_normals(form: Form, turn: impl Fn([f64; 3]) -> [f64; 3]) -> Form {
    match form {
        Form::Disk { normal, barred } => Form::Disk {
            normal: turn(normal),
            barred,
        },
        Form::Ring {
            normal,
            inner,
            outer,
        } => Form::Ring {
            normal: turn(normal),
            inner,
            outer,
        },
        Form::Orbit { normal } => Form::Orbit {
            normal: turn(normal),
        },
        Form::Arc { normal, span } => Form::Arc {
            normal: turn(normal),
            span,
        },
        Form::Arm { normal, phase } => Form::Arm {
            normal: turn(normal),
            phase,
        },
        Form::Sheet { normal } => Form::Sheet {
            normal: turn(normal),
        },
        Form::Patch { normal } => Form::Patch {
            normal: turn(normal),
        },
        other => other,
    }
}

/// Corners of a ground rectangle in cell units (#430).
///
/// `body` is the rect centre, `half` its half extents, `up` the ground
/// normal in the drawn frame: corners lie in the plane perpendicular to
/// `up`, spanning `±half[0]` along the tangent and `±half[1]` along the
/// binormal, so previewed rects follow their patch orientation while open
/// cells (identity `up`) draw flat as before. Degenerate `up` falls back
/// to `+Y`, mirroring [`unit`]. Pure with fixed arrays only
/// (`E-HOT-NOALLOC`).
#[must_use]
pub(crate) fn rect_corners(body: [f64; 3], half: [f64; 2], up: [f64; 3]) -> [[f64; 3]; 4] {
    let length = (up[0] * up[0] + up[1] * up[1] + up[2] * up[2]).sqrt();
    let n = if length.is_finite() && length > 0.0 {
        [up[0] / length, up[1] / length, up[2] / length]
    } else {
        [0.0, 1.0, 0.0]
    };
    // Reference for the tangent basis: world `+Y` unless the normal is
    // near-vertical, when `+Z` keeps the first extent on world `+X`
    // (matching the flat open-cell rect draw for identity `up`).
    let reference = if n[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [0.0, 0.0, 1.0]
    };
    // Tangent = n × reference, binormal = n × tangent (same handedness as
    // the gizmo `tangent` helper).
    let mut t = [
        n[1] * reference[2] - n[2] * reference[1],
        n[2] * reference[0] - n[0] * reference[2],
        n[0] * reference[1] - n[1] * reference[0],
    ];
    let t_length = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt();
    if t_length.is_finite() && t_length > 0.0 {
        t = [t[0] / t_length, t[1] / t_length, t[2] / t_length];
    } else {
        t = [1.0, 0.0, 0.0];
    }
    let b = [
        n[1] * t[2] - n[2] * t[1],
        n[2] * t[0] - n[0] * t[2],
        n[0] * t[1] - n[1] * t[0],
    ];
    [
        [
            body[0] + t[0] * half[0] + b[0] * half[1],
            body[1] + t[1] * half[0] + b[1] * half[1],
            body[2] + t[2] * half[0] + b[2] * half[1],
        ],
        [
            body[0] - t[0] * half[0] + b[0] * half[1],
            body[1] - t[1] * half[0] + b[1] * half[1],
            body[2] - t[2] * half[0] + b[2] * half[1],
        ],
        [
            body[0] - t[0] * half[0] - b[0] * half[1],
            body[1] - t[1] * half[0] - b[1] * half[1],
            body[2] - t[2] * half[0] - b[2] * half[1],
        ],
        [
            body[0] + t[0] * half[0] - b[0] * half[1],
            body[1] + t[1] * half[0] - b[1] * half[1],
            body[2] + t[2] * half[0] - b[2] * half[1],
        ],
    ]
}

/// Draws a box standing on the ground normal (#430).
///
/// Base and lid rectangles from [`rect_corners`] plus four corner posts:
/// the same twelve edges as the axis-aligned box for identity `up`,
/// following the patch frame otherwise. `footprint` is the half width
/// (the point radius, as in [`draw_form`]), `height` the full height. No
/// allocation.
pub(crate) fn draw_oriented_box(
    gizmos: &mut Gizmos,
    body: [f64; 3],
    footprint: f32,
    height: f64,
    up: [f64; 3],
    color: Color,
) {
    let half = f64::from(footprint);
    let length = (up[0] * up[0] + up[1] * up[1] + up[2] * up[2]).sqrt();
    let n = if length.is_finite() && length > 0.0 {
        [up[0] / length, up[1] / length, up[2] / length]
    } else {
        [0.0, 1.0, 0.0]
    };
    let half_extents = [half, half];
    let [a, b, c, d] = rect_corners(body, half_extents, n).map(to_vec3);
    let lid_body = [
        body[0] + n[0] * height,
        body[1] + n[1] * height,
        body[2] + n[2] * height,
    ];
    let [e, f, g, h] = rect_corners(lid_body, half_extents, n).map(to_vec3);
    gizmos.line(a, b, color);
    gizmos.line(b, c, color);
    gizmos.line(c, d, color);
    gizmos.line(d, a, color);
    gizmos.line(a, e, color);
    gizmos.line(b, f, color);
    gizmos.line(c, g, color);
    gizmos.line(d, h, color);
    gizmos.line(e, f, color);
    gizmos.line(f, g, color);
    gizmos.line(g, h, color);
    gizmos.line(h, e, color);
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
        Form::Box { height } => point_radius.max(height),
        _ => point_radius,
    }
}

/// The colour of an orbit path (#1056 Spec v1, owner Q2): the marker's own
/// `color`, lifted so its brightest channel reaches at least
/// [`ORBIT_PATH_BRIGHTNESS`] (a light line, never a dark crack across the
/// sky), with the height `weight` ([`universe_core::labels::orbit_path_weight`])
/// as its alpha. `None` when nothing draws: no weight, or a black marker.
#[must_use]
pub(crate) fn orbit_path_color(color: Color, weight: f64) -> Option<Color> {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: a 0..=1 weight narrowed to an alpha, intended"
    )]
    let alpha = weight.clamp(0.0, 1.0) as f32;
    let s = color.to_srgba();
    let top = s.red.max(s.green).max(s.blue);
    if alpha <= 0.0 || top <= 0.0 {
        return None;
    }
    let lift = (ORBIT_PATH_BRIGHTNESS / top).max(1.0);
    Some(Color::srgba(
        (s.red * lift).min(1.0),
        (s.green * lift).min(1.0),
        (s.blue * lift).min(1.0),
        s.alpha * alpha,
    ))
}

/// Sine of 10°: a circle seen this far or more out of its plane draws in
/// full (#1057 Spec v1). A constant: render code takes no `asin`
/// (`E-TRANSCENDENTAL`).
const EDGE_FULL_SIN: f32 = 0.173_648_18;

/// Sine of 2°: a circle seen this close to its plane, or closer, is gone
/// (#1057 Spec v1).
const EDGE_GONE_SIN: f32 = 0.034_899_497;

/// How much of a circle about `centre` with plane `normal` and `radius`
/// draws for an `eye`, `0..=1` (#1057 Spec v2, owner Q1 A and Q2): the
/// eye's angle out of the circle's plane, seen from the centre and from
/// the circle's nearest point, the smaller of the two; full from 10°, gone
/// under 2°, a smooth ease between. Edge-on, a big circle is a straight
/// line across the screen (the line through the Sun at L7); a circle
/// passing close by the eye sweeps across it too (the dark bands); both
/// thin away instead. An eye at the centre is in the plane. Pure.
#[must_use]
pub(crate) fn edge_on_weight(normal: Vec3, centre: Vec3, radius: f32, eye: Vec3) -> f32 {
    let n = normal.normalize_or_zero();
    if n == Vec3::ZERO {
        return 1.0;
    }
    let d = eye - centre;
    let len = d.length();
    if !len.is_finite() || len <= 0.0 {
        return 0.0;
    }
    let height = n.dot(d);
    let from_centre = (height / len).abs();
    // The circle's nearest point: towards the eye's foot in the plane;
    // over the centre every point is as near, so any in-plane direction.
    let foot = d - n * height;
    let dir = foot
        .try_normalize()
        .unwrap_or_else(|| n.any_orthonormal_vector());
    let e = d - dir * radius.max(0.0);
    let l = e.length();
    let from_near = if l > 0.0 { (height / l).abs() } else { 0.0 };
    let s = from_centre.min(from_near);
    let t = ((s - EDGE_GONE_SIN) / (EDGE_FULL_SIN - EDGE_GONE_SIN)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// What decides how much of a marker's big circles draws: the orbit-path
/// height weight (#1056) and the eye they are seen from (#1057).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Paths {
    /// [`universe_core::labels::orbit_path_weight`] for the camera.
    pub height: f64,
    /// The camera in render space.
    pub eye: Vec3,
}

/// Within this many radii of a sphere's centre the eye sees its outline
/// circles as big circles that can lie edge-on across the screen (#1057).
const BIG_SPHERE_REACH: f32 = 4.0;

/// Draws a sphere's three axis circles. Near and inside a big sphere
/// (within [`BIG_SPHERE_REACH`] radii) each circle fades by its own
/// [`edge_on_weight`], so none draws a straight line across the screen
/// (the line through the Sun at L7, #1057); farther away it is the plain
/// gizmo sphere, as before.
fn sphere(gizmos: &mut Gizmos, centre: Vec3, radius: f32, color: Color, eye: Vec3) {
    if eye.distance(centre) > BIG_SPHERE_REACH * radius {
        gizmos.sphere(Isometry3d::from_translation(centre), radius, color);
        return;
    }
    for normal in [Vec3::X, Vec3::Y, Vec3::Z] {
        let Some(line) = faded(color, edge_on_weight(normal, centre, radius, eye)) else {
            continue;
        };
        let plane = Isometry3d::new(centre, Quat::from_rotation_arc(Vec3::Z, normal));
        gizmos
            .circle(plane, radius, line)
            .resolution(FORM_RESOLUTION);
    }
}

/// `color` with its alpha scaled by `weight`; `None` at zero.
fn faded(color: Color, weight: f32) -> Option<Color> {
    (weight > 0.0).then(|| color.with_alpha(color.alpha() * weight))
}

/// Draws the body at `body` with radius `size`.
///
/// An `Orbit` circle draws in [`orbit_path_color`] at the `paths` height
/// weight times its [`edge_on_weight`] for the `paths` eye; `Ring` circles and
/// each of a `Shell`'s three great circles fade by their own edge-on
/// weight. Bodies keep `color`.
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
    paths: Paths,
) {
    let Paths { height: path, eye } = paths;
    match form {
        Form::Dot | Form::Body => sphere(gizmos, body, size, color, eye),
        Form::Shell => {
            // Three great circles about the centre at the body's distance:
            // a reservoir shell has no body of its own, so members keep
            // small radii and the shell reads from their orbits. A fraction
            // of a wireframe sphere's line count.
            let radius = body.distance(centre).max(1e-6);
            for normal in [Vec3::X, Vec3::Y, Vec3::Z] {
                let Some(line) = faded(color, edge_on_weight(normal, centre, radius, eye)) else {
                    continue;
                };
                let plane = Isometry3d::new(centre, Quat::from_rotation_arc(Vec3::Z, normal));
                gizmos
                    .circle(plane, radius, line)
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
            if let Some(line) = faded(color, edge_on_weight(unit(normal), centre, outer, eye)) {
                gizmos
                    .circle(facing(centre, normal), outer.max(1e-6), line)
                    .resolution(FORM_RESOLUTION);
            }
            if inner > 1e-6
                && let Some(line) = faded(color, edge_on_weight(unit(normal), centre, inner, eye))
            {
                gizmos
                    .circle(facing(centre, normal), inner, line)
                    .resolution(FORM_RESOLUTION);
            }
            gizmos.sphere(Isometry3d::from_translation(body), size * 0.3, color);
        }
        Form::Orbit { normal } => {
            let radius = body.distance(centre).max(1e-6);
            let edge = f64::from(edge_on_weight(unit(normal), centre, radius, eye));
            if let Some(line) = orbit_path_color(color, path * edge) {
                gizmos
                    .circle(facing(centre, normal), radius, line)
                    .resolution(FORM_RESOLUTION);
            }
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

    #[test]
    fn circles_fade_when_seen_edge_on() {
        // #1057 Spec v1 (owner Q1 A): full from 10° out of the plane, gone
        // under 2°, smooth between; the eye at the centre is in the plane.
        let at = |deg: f32| Vec3::new(deg.to_radians().cos(), deg.to_radians().sin(), 0.0) * 50.0;
        let n = Vec3::Y;
        assert_eq!(
            edge_on_weight(n, Vec3::ZERO, 1.0, at(0.0)),
            0.0,
            "in the plane"
        );
        assert_eq!(edge_on_weight(n, Vec3::ZERO, 1.0, at(1.9)), 0.0);
        assert_eq!(edge_on_weight(n, Vec3::ZERO, 1.0, at(10.1)), 1.0);
        assert_eq!(
            edge_on_weight(n, Vec3::ZERO, 1.0, at(90.0)),
            1.0,
            "from above"
        );
        assert_eq!(
            edge_on_weight(-n, Vec3::ZERO, 1.0, at(-30.0)),
            1.0,
            "from below"
        );
        assert_eq!(
            edge_on_weight(n, Vec3::ZERO, 1.0, Vec3::ZERO),
            0.0,
            "at the centre"
        );
        let mut last = 0.0;
        for i in 0..=100_u8 {
            let w = edge_on_weight(n, Vec3::ZERO, 1.0, at(2.0 + 0.08 * f32::from(i)));
            assert!(w >= last, "rises steadily: {w} after {last}");
            last = w;
        }
        assert!((last - 1.0).abs() < 1e-3, "{last}");
        // A shell's three great circles fade apart: from an eye on the X
        // axis, the X-normal circle is seen face-on, the other two edge-on.
        let eye = Vec3::X * 10.0;
        assert_eq!(edge_on_weight(Vec3::X, Vec3::ZERO, 1.0, eye), 1.0);
        assert_eq!(edge_on_weight(Vec3::Y, Vec3::ZERO, 1.0, eye), 0.0);
        assert_eq!(edge_on_weight(Vec3::Z, Vec3::ZERO, 1.0, eye), 0.0);
        assert_eq!(faded(Color::BLACK, 0.0), None);
        // Spec v2 (Q2): a big circle all round the eye, seen from just
        // above its plane, is a band across the screen near its nearest
        // part: gone, though from its centre the eye is straight above.
        let big = 1_000.0;
        assert_eq!(edge_on_weight(n, Vec3::ZERO, big, Vec3::Y * 20.0), 0.0);
        // Well above the same circle (more than 10° from every point), it
        // shows in full.
        assert_eq!(edge_on_weight(n, Vec3::ZERO, big, Vec3::Y * 400.0), 1.0);
        // Just above the edge of the same circle: from its centre the eye is
        // about 3° out of the plane (the far side lies along the horizon),
        // and the smaller angle decides, so it has mostly faded.
        assert!(edge_on_weight(n, Vec3::ZERO, big, Vec3::new(990.0, 50.0, 0.0)) < 0.1);
    }

    #[test]
    fn orbit_paths_are_light_in_their_own_hue_and_fade_by_height() {
        // #1056 Spec v1: a dim moon's path is lifted to the floor in its
        // own hue, never a dark line; the weight fades it; none at zero.
        // A blue tint from the tokens, dimmed as a far moon is.
        let tint = crate::tokens::MENU_SELECT_TINT;
        let base = tint.to_srgba();
        let dim = crate::style::scaled(tint, 0.05);
        let path = orbit_path_color(dim, 1.0).expect("drawn").to_srgba();
        let top = path.red.max(path.green).max(path.blue);
        assert!((top - ORBIT_PATH_BRIGHTNESS).abs() < 1e-5, "{path:?}");
        assert!(
            (path.red / path.blue - base.red / base.blue).abs() < 1e-4
                && (path.green / path.blue - base.green / base.blue).abs() < 1e-4,
            "same hue"
        );
        assert!((path.alpha - 1.0).abs() < 1e-6);
        let bright = tint;
        let kept = orbit_path_color(bright, 0.5).expect("drawn").to_srgba();
        assert!(
            (kept.blue - base.blue).abs() < 1e-5,
            "a bright path keeps its colour"
        );
        assert!((kept.alpha - 0.5).abs() < 1e-6, "half faded");
        assert_eq!(orbit_path_color(bright, 0.0), None, "gone near the ground");
        assert_eq!(orbit_path_color(Color::BLACK, 1.0), None);
    }

    /// Form helper math stays total on degenerate input.
    #[test]
    fn degenerate_normals_fall_back_to_axes() {
        assert_eq!(unit([0.0, 0.0, 0.0]), Vec3::Z);
        assert_eq!(unit([f64::NAN, 0.0, 0.0]), Vec3::Z);
        let t = tangent([0.0, 1.0, 0.0]);
        assert!(t.x.abs() + t.z.abs() > 0.9, "tangent must leave +Y: {t:?}");
    }

    #[test]
    fn identity_up_lays_rects_flat() {
        let body = [0.2, -0.1, 0.3];
        let half = [0.03, 0.02];
        let corners = rect_corners(body, half, [0.0, 1.0, 0.0]);
        for corner in corners {
            assert!(
                (corner[1] - body[1]).abs() < 1e-12,
                "flat in the ground plane"
            );
        }
        let mut xs = [0.0; 4];
        let mut zs = [0.0; 4];
        for (i, corner) in corners.iter().enumerate() {
            xs[i] = corner[0];
            zs[i] = corner[2];
        }
        xs.sort_by(|a, b| a.total_cmp(b));
        zs.sort_by(|a, b| a.total_cmp(b));
        assert!((xs[0] - (body[0] - half[0])).abs() < 1e-12);
        assert!((xs[3] - (body[0] + half[0])).abs() < 1e-12);
        assert!((zs[0] - (body[2] - half[1])).abs() < 1e-12);
        assert!((zs[3] - (body[2] + half[1])).abs() < 1e-12);
    }

    #[test]
    fn tilted_up_turns_rects_into_the_patch_plane() {
        // A planet-tilted patch normal: corners stay centred on the body,
        // perpendicular to the normal, with true edge lengths.
        let body = [0.1, 0.2, -0.1];
        let half = [0.03, 0.02];
        let length = (0.3f64 * 0.3 + 0.8 * 0.8 + 0.5 * 0.5).sqrt();
        let up = [0.3 / length, 0.8 / length, 0.5 / length];
        let corners = rect_corners(body, half, up);
        let mut centroid = [0.0; 3];
        for corner in corners {
            for axis in 0..3 {
                centroid[axis] += corner[axis] / 4.0;
            }
            let off = [
                corner[0] - body[0],
                corner[1] - body[1],
                corner[2] - body[2],
            ];
            assert!(
                (off[0] * up[0] + off[1] * up[1] + off[2] * up[2]).abs() < 1e-12,
                "corner leaves the patch plane"
            );
        }
        for axis in 0..3 {
            assert!(
                (centroid[axis] - body[axis]).abs() < 1e-12,
                "centred on the body"
            );
        }
        for side in 0..4 {
            let a = corners[side];
            let b = corners[(side + 1) % 4];
            let edge =
                ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
            let want = if side % 2 == 0 {
                2.0 * half[0]
            } else {
                2.0 * half[1]
            };
            assert!((edge - want).abs() < 1e-12, "edge keeps its length");
        }
    }

    #[test]
    fn degenerate_up_falls_back_to_flat() {
        let body = [0.1, 0.2, 0.3];
        let half = [0.03, 0.02];
        assert_eq!(
            rect_corners(body, half, [0.0; 3]),
            rect_corners(body, half, [0.0, 1.0, 0.0])
        );
        assert_eq!(
            rect_corners(body, half, [f64::NAN, 0.0, 0.0]),
            rect_corners(body, half, [0.0, 1.0, 0.0])
        );
    }
}
