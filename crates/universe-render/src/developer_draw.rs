//! Gizmo drawing of the developer layer's planned parts (#1128 T3).
//!
//! Reads [`crate::developer::DeveloperLayer::planned`]: every decision
//! (stage, crossfade, view weight, budget) was made in core; this module
//! only maps a part's stage and shape family to gizmo lines at its true
//! anchor, in its semantic colour and line pattern (`E-RENDER-NO-MESH`).
//! Today's markers are drawn by `draw` and never here (Spec v3 B).

use bevy::math::Quat;
use bevy::prelude::*;
use universe_core::coords::Level;
use universe_core::r#gen::Form;
use universe_core::semantic::{ColourRole, PartRecord, ShapeFamily};
use universe_core::semantic_display::{LinePattern, Stage, drawn_extent, palette};

use crate::Universe;
use crate::developer::DeveloperLayer;
use crate::forms::{Paths, draw_form};
use crate::style::{RenderOrigin, point_color_for_level, scaled, tint_color};

/// Segments of a drawn developer circle; patterns count in these.
const CIRCLE_SEGMENTS: usize = 48;

/// Share of a part's drawn extent used by its landmark cross and glyph.
const GLYPH_SHARE: f32 = 0.5;

/// Brightness of muted owner colours (orbits, arcs and paths).
const MUTED: f32 = 0.6;

/// Colour of a part: its palette colour, or its owner's own colour (star
/// and galaxy classification, the level's era colour) or a muted owner.
fn part_color(universe: &Universe, record: &PartRecord) -> (Color, LinePattern) {
    let spec = record.spec();
    let (rgb, pattern) = palette(spec.colour);
    if let Some([r, g, b]) = rgb {
        return (Color::srgb(r, g, b), pattern);
    }
    let level = Level::new(record.id.level).unwrap_or(Level::MIN);
    let owner = match record.parent {
        universe_core::semantic::Parent::Marker(id) => universe
            .open
            .point_by_id(id)
            .and_then(|point| point.tint)
            .map_or_else(|| point_color_for_level(level), tint_color),
        universe_core::semantic::Parent::Cell => point_color_for_level(level),
    };
    let color = if spec.colour == ColourRole::OwnerMuted {
        scaled(owner, MUTED)
    } else {
        owner
    };
    (color, pattern)
}

/// A normal in open-cell units turned into render space (the planet frame
/// at L10, #774 Q4 A).
fn turned(origin: &RenderOrigin, normal: [f64; 3]) -> Vec3 {
    match origin.form(Form::Orbit { normal }) {
        Form::Orbit { normal } => crate::forms::unit(normal),
        _ => crate::forms::unit(normal),
    }
}

/// A circle of `radius` about `centre` in the plane with `normal`, drawn in
/// `pattern`.
fn pattern_circle(
    gizmos: &mut Gizmos,
    centre: Vec3,
    normal: Vec3,
    radius: f32,
    color: Color,
    pattern: LinePattern,
) {
    let rotation = Quat::from_rotation_arc(Vec3::Z, normal.normalize_or(Vec3::Z));
    let point = |index: usize| {
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: a segment index below 48, exact in f32"
        )]
        let angle = index as f32 / CIRCLE_SEGMENTS as f32 * std::f32::consts::TAU;
        centre + rotation * Vec3::new(angle.cos() * radius, angle.sin() * radius, 0.0)
    };
    let (on, off) = pattern.runs().unwrap_or((CIRCLE_SEGMENTS, 0));
    for index in 0..CIRCLE_SEGMENTS {
        if index % (on + off) < on {
            gizmos.line(point(index), point(index + 1), color);
        }
    }
}

/// A short three-axis cross: the landmark (R1).
fn cross(gizmos: &mut Gizmos, at: Vec3, size: f32, color: Color) {
    for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
        gizmos.line(at - axis * size, at + axis * size, color);
    }
}

/// Draws the planned developer parts.
pub(crate) fn draw_developer_parts(
    mut gizmos: Gizmos,
    universe: Res<Universe>,
    origin: Res<RenderOrigin>,
    layer: Res<DeveloperLayer>,
) {
    if !layer.effective().enabled {
        return;
    }
    let eye_cell = universe.path.offset();
    let eye = origin.at(eye_cell);
    let cell = origin.cell();
    let path = universe_core::labels::orbit_path_weight(crate::hud::camera_height_m(
        &universe,
        origin.camera,
    ));
    for (record, plan) in layer.planned() {
        let (base, pattern) = part_color(&universe, record);
        let color = scaled(base, plan.weight);
        let at = origin.at(record.anchor);
        let offset = [
            eye_cell[0] - record.anchor[0],
            eye_cell[1] - record.anchor[1],
            eye_cell[2] - record.anchor[2],
        ];
        let distance =
            (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
        let size = origin.len(drawn_extent(record.extent, distance));
        let facing = (eye - at).normalize_or(Vec3::Z);
        let normal = record.orientation.map(|n| turned(&origin, n));
        match plan.stage {
            Stage::Hidden => {}
            Stage::Landmark => cross(&mut gizmos, at, size * GLYPH_SHARE, color),
            Stage::Glyph => {
                pattern_circle(&mut gizmos, at, facing, size * GLYPH_SHARE, color, pattern);
            }
            Stage::Outline | Stage::Composite | Stage::Resolved | Stage::Local => {
                outline(
                    &mut gizmos,
                    record,
                    at,
                    normal,
                    facing,
                    size,
                    color,
                    pattern,
                );
                if plan.stage >= Stage::Composite
                    && let Some(normal) = normal
                {
                    // Secondary part: the orientation tick at the anchor.
                    gizmos.line(at, at + normal * size * GLYPH_SHARE, color);
                }
                // Threads keep their true endpoints (filaments).
                if record.spec().shape == ShapeFamily::Thread
                    && let universe_core::semantic::Parent::Marker(id) = record.parent
                    && let Some(point) = universe.open.point_by_id(id)
                    && matches!(point.form, Form::Thread { .. })
                {
                    draw_form(
                        &mut gizmos,
                        origin.form(point.form),
                        at,
                        cell,
                        size,
                        color,
                        Paths { height: path, eye },
                    );
                }
                if record.id.kind == universe_core::semantic::SemanticType::HostArms
                    && let Some([x, y, z]) = record.orientation
                {
                    for phase in [0.0, std::f64::consts::PI] {
                        draw_form(
                            &mut gizmos,
                            origin.form(Form::Arm {
                                normal: [x, y, z],
                                phase,
                            }),
                            at,
                            cell,
                            size,
                            color,
                            Paths { height: path, eye },
                        );
                    }
                }
            }
        }
    }
}

/// The true-oriented outer geometry of one part (R3 and up).
#[expect(
    clippy::too_many_arguments,
    reason = "one part's geometry, colour and pattern for one gizmo call"
)]
fn outline(
    gizmos: &mut Gizmos,
    record: &PartRecord,
    at: Vec3,
    normal: Option<Vec3>,
    facing: Vec3,
    size: f32,
    color: Color,
    pattern: LinePattern,
) {
    match (record.spec().shape, normal) {
        (
            ShapeFamily::Disk | ShapeFamily::Ring | ShapeFamily::Sheet | ShapeFamily::Arc,
            Some(n),
        ) => {
            pattern_circle(gizmos, at, n, size, color, pattern);
        }
        (ShapeFamily::Axis, Some(n)) => {
            // A bar or spin axis lies along the tangent or the axis itself.
            let along = if record.id.kind == universe_core::semantic::SemanticType::SpinAxis {
                n
            } else {
                n.any_orthonormal_vector()
            };
            gizmos.line(at - along * size, at + along * size, color);
        }
        // The arms draw as arms, never as a circle.
        _ if record.id.kind == universe_core::semantic::SemanticType::HostArms => {}
        _ => {
            // Spheres, spheroids, shells, clumps and points: the silhouette.
            pattern_circle(gizmos, at, facing, size, color, pattern);
        }
    }
}
