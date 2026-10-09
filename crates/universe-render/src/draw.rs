//! Gizmo drawing of axes, cells, previews, siblings, and room outlines.
//!
//! Everything here is `pub(crate)`: only this crate uses it.

use crate::PreviewCache;
use crate::Universe;
use crate::forms::{FORM_RESOLUTION, draw_form, gate_radius, map_form, turn_form_normals};
use crate::input::Navigation;
use crate::style::{point_color_for_level, scaled, sibling_color_for_level, tint_color, to_vec3};
use bevy::math::{DVec3, Isometry3d, Quat, bounding::Aabb3d};
use bevy::prelude::*;
use universe_core::coords::{Level, ParentUnits};
use universe_core::r#gen::MarkerKind;
use universe_core::nav::{MarkerIndex, drawn_radius, sibling_in_open_units};
use universe_core::nest::{
    FORM_ANGLE, IDENTITY_UP, angular_radius, child_ratio, child_to_parent,
    child_world_position_oriented, children_brightness, parent_to_child, shell_brightness,
};

/// Draws the open cell's RGB axes (orientation cue) at half-cell length.
///
/// Off by default; the `X` key toggles them (`Navigation::show_axes`).
pub(crate) fn draw_axes(mut gizmos: Gizmos, nav: Res<Navigation>) {
    if !nav.show_axes {
        return;
    }
    gizmos.line(Vec3::ZERO, Vec3::X * 0.5, Color::srgb(1.0, 0.0, 0.0));
    gizmos.line(Vec3::ZERO, Vec3::Y * 0.5, Color::srgb(0.0, 1.0, 0.0));
    gizmos.line(Vec3::ZERO, Vec3::Z * 0.5, Color::srgb(0.0, 0.5, 1.0));
}

/// Draws the L1 graticule: equator, two parallels, two meridians.
///
/// Level scenery, not generated content: the observable sphere's grid,
/// drawn faint in the level color.
fn draw_graticule(gizmos: &mut Gizmos, color: Color) {
    let faint = scaled(color, 0.3);
    gizmos
        .circle(Isometry3d::IDENTITY, 0.5, faint)
        .resolution(FORM_RESOLUTION);
    for height in [0.25f32, -0.25f32] {
        let ring = (0.25 - height * height).sqrt();
        let at = Isometry3d::from_translation(Vec3::Y * height);
        gizmos.circle(at, ring, faint).resolution(FORM_RESOLUTION);
    }
    for normal in [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0]] {
        let tilted = Isometry3d::new(
            Vec3::ZERO,
            Quat::from_rotation_arc(Vec3::Z, to_vec3(normal)),
        );
        gizmos
            .circle(tilted, 0.5, faint)
            .resolution(FORM_RESOLUTION);
    }
}

/// Furniture outline half-size as a multiple of the indicator radius (#375).
///
/// Room points carry radius 0.01; outlines at four times that read as small
/// boxes in the one-unit room cell without touching each other.
pub(crate) const FURNITURE_OUTLINE_SCALE: f32 = 4.0;

/// Draws furniture outlines for the open L14 room (#375, ADR 0016).
///
/// Every L14 dot point is furniture (the room generator emits populations
/// only): each gets an axis-aligned box outline in the room tint, sized by
/// its indicator radius. The room shell draws through its own `Box` form in
/// the open-cell pass, so it is skipped here. Returns immediately at any
/// other level. No meshes: `E-RENDER-NO-MESH` holds at every rung.
pub(crate) fn draw_room_outlines(mut gizmos: Gizmos, universe: Res<Universe>) {
    if universe.level().get() != 14 {
        return;
    }
    let color = point_color_for_level(universe.level());
    for point in &universe.open.points {
        if point.form != universe_core::r#gen::Form::Dot {
            continue;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: cell-unit radius narrowed for the GPU, intended"
        )]
        let half = point.radius as f32 * FURNITURE_OUTLINE_SCALE;
        gizmos.aabb_3d(
            Aabb3d::new(to_vec3(point.position), Vec3::splat(half)),
            Isometry3d::IDENTITY,
            color,
        );
    }
}

/// Draws the open cell: its shell, its markers, hover and target.
///
/// Dots draw at the true child size with the impostor clamp, dimmed by
/// [`shell_brightness`](universe_core::nest::shell_brightness) as their interior resolves; bodies
/// with a form draw the form at the point's own radius once past
/// [`FORM_ANGLE`]. The open cell's own shell (radius 0.5) uses the same
/// curve in the parent's era color, so the marker you entered and the cell
/// you are in are one continuous object. The hovered marker is white, the
/// target magenta. L10 surface samples stay in the data but are not drawn
/// (the mesh shows them, #384).
pub(crate) fn draw_open_cell(mut gizmos: Gizmos, universe: Res<Universe>, nav: Res<Navigation>) {
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
    if universe.level().get() == 1 {
        draw_graticule(&mut gizmos, color);
    }
    // Ground grids draw only from the generated grid point (#394): the tail
    // generator emits one per surface cell with the true curvature, so no
    // level special-case remains here.
    for (index, point) in universe.open.points.iter().enumerate() {
        if point.surface.is_some() {
            continue;
        }
        let position = DVec3::from_array(point.position);
        let distance = (camera - position).length();
        // Every marker draws at its own size and gates on its own visual
        // extent; both brightness curves read 1.0 at the gate (it sits below
        // PREVIEW_ANGLE), so the dot-to-form handoff never pops.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a radius, intended"
        )]
        let own = point.radius as f32;
        let at = to_vec3(point.position);
        let gate = gate_radius(point.form, at, Vec3::ZERO, f64::from(own.max(1e-6)));
        let angular = angular_radius(gate, distance);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a radius, intended"
        )]
        let dot = drawn_radius(f64::from(own.max(1e-6)), distance) as f32;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a brightness, intended"
        )]
        let brightness = shell_brightness(angular) as f32;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: drawn marker index into a budgeted cell, always fits u32"
        )]
        let index = index as u32;
        // Populations render at half size and half brightness and never
        // highlight: only portals take the target or the hover (#151).
        // Tinted points (stars, galaxies) read their core hue (#157).
        let portal = point.kind == MarkerKind::Portal;
        let dot = if portal { dot } else { dot * 0.5 };
        let brightness = if portal { brightness } else { brightness * 0.5 };
        let base = point.tint.map_or(color, tint_color);
        let marker_color = if portal && nav.target == Some(index) {
            scaled(Color::srgb(1.0, 0.0, 1.0), brightness.max(0.5))
        } else if portal && nav.hover == Some(index) {
            scaled(Color::WHITE, brightness.max(0.4))
        } else {
            scaled(base, brightness)
        };
        let isometry = Isometry3d::from_translation(at);
        if brightness > 0.0 {
            if angular < FORM_ANGLE {
                gizmos.sphere(isometry, dot, marker_color);
            } else {
                draw_form(
                    &mut gizmos,
                    point.form,
                    at,
                    Vec3::ZERO,
                    own.max(1e-6),
                    marker_color,
                );
            }
        }
        if portal && nav.target == Some(index) {
            gizmos.sphere(isometry, dot * 1.6, Color::srgb(0.6, 0.0, 0.6));
        }
    }
}

/// Draws the previewed interiors inside the largest-on-screen markers.
///
/// Children sit at [`child_world_position`] at their true size (the next
/// level's marker radius scaled by the ratio) with the impostor clamp, lit by
/// [`children_brightness`] of their parent marker: invisible while the marker
/// is a point, fully lit by the time it opens, so entering changes nothing.
pub(crate) fn draw_previews(
    mut gizmos: Gizmos,
    universe: Res<Universe>,
    previews: Res<PreviewCache>,
) {
    let (Some(ratio), Some(child_level)) =
        (child_ratio(universe.level()), universe.level().deeper())
    else {
        return;
    };
    let camera = DVec3::from_array(universe.path.offset());
    let radius = universe.marker_radius();
    let color = point_color_for_level(child_level);
    for (marker, content) in previews.entries() {
        let Some(marker_pos) = universe.marker(MarkerIndex(*marker)) else {
            continue;
        };
        // Oriented preview (#394): the marker's patch normal turns the
        // child frame, so the previewed region sits where opening lands it.
        // Normals turn with positions; thread endpoints map as positions.
        let marker_up = universe
            .open
            .points
            .get(*marker as usize)
            .and_then(|point| match point.form {
                universe_core::r#gen::Form::Patch { normal } => Some(normal),
                _ => None,
            })
            .unwrap_or(IDENTITY_UP);
        let marker_distance = (camera - DVec3::from_array(marker_pos)).length();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a brightness, intended"
        )]
        let brightness = children_brightness(angular_radius(radius, marker_distance)) as f32;
        if brightness <= 0.0 {
            continue;
        }
        for point in &content.points {
            if point.surface.is_some() {
                continue;
            }
            // Children map into open units through the marker: anchors move,
            // linear sizes scale by the ratio.
            let map = |local: [f64; 3]| {
                child_world_position_oriented(ParentUnits(marker_pos), ratio, local, marker_up).0
            };
            let turn = |normal: [f64; 3]| child_to_parent(marker_up, normal);
            let world = map(point.position);
            let centre = to_vec3(marker_pos);
            let distance = (camera - DVec3::from_array(world)).length();
            let form = turn_form_normals(map_form(point.form, map, ratio), turn);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: render-domain narrowing of a ratio, intended"
            )]
            let ratio_f = ratio as f32;
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: render-domain narrowing of a radius, intended"
            )]
            let own = point.radius as f32 * ratio_f;
            let at = to_vec3(world);
            let gate = gate_radius(form, at, centre, f64::from(own.max(1e-6)));
            let angular = angular_radius(gate, distance);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: render-domain narrowing of a radius, intended"
            )]
            let dot = drawn_radius(f64::from(own.max(1e-6)), distance) as f32;
            let base = point.tint.map_or(color, tint_color);
            let lit = scaled(base, brightness);
            let (dot, lit) = if point.kind == MarkerKind::Portal {
                (dot, lit)
            } else {
                (dot * 0.5, scaled(base, brightness * 0.5))
            };
            if angular < FORM_ANGLE {
                gizmos.sphere(Isometry3d::from_translation(at), dot, lit);
            } else {
                draw_form(&mut gizmos, form, at, centre, own.max(1e-6), lit);
            }
        }
    }
}

/// Draws the parent cell's sibling markers around the open cell.
///
/// Siblings sit `1 / ratio` cells away at radius 0.5, so what you came from
/// stays behind you while you dive. The marker you entered is the shell.
pub(crate) fn draw_parent_siblings(mut gizmos: Gizmos, universe: Res<Universe>) {
    let (Some(parent), Some(entered)) = (&universe.parent, universe.path.entered()) else {
        return;
    };
    let camera = DVec3::from_array(universe.path.offset());
    let level = universe.level().shallower().unwrap_or(Level::MIN);
    // Tail siblings share the parent-context floor (#394, T5): faint and
    // constant until the next open. Shallower levels keep sibling hues.
    let color = if (11..=14).contains(&universe.level().get()) {
        scaled(point_color_for_level(level), crate::sky::CONTEXT_FLOOR)
    } else {
        sibling_color_for_level(level)
    };
    for (index, point) in parent.points.iter().enumerate() {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: sibling marker index into a budgeted cell, always fits u32"
        )]
        let sibling_marker = index as u32;
        if sibling_marker == entered.marker {
            continue;
        }
        // Siblings map into open units through the entered marker exactly
        // like preview children: anchors move, linear sizes scale by the
        // inverse ratio. Siblings sit 1/ratio cells away, so dots rule.
        // Normals unturn through the entered frame (#394).
        let scale = 1.0 / entered.ratio;
        let map = |local: [f64; 3]| sibling_in_open_units(entered, ParentUnits(local)).0.0;
        let unturn = |normal: [f64; 3]| parent_to_child(entered.up, normal);
        let world = map(point.position);
        let centre = map(point.portal_position());
        let distance = (camera - DVec3::from_array(world)).length();
        let form = turn_form_normals(map_form(point.form, map, scale), unturn);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a ratio, intended"
        )]
        let scale_f = scale as f32;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a radius, intended"
        )]
        let own = point.radius as f32 * scale_f;
        let at = to_vec3(world);
        let gate = gate_radius(form, at, to_vec3(centre), f64::from(own.max(1e-6)));
        let angular = angular_radius(gate, distance);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a radius, intended"
        )]
        let dot = drawn_radius(f64::from(own.max(1e-6)), distance) as f32;
        if angular < FORM_ANGLE {
            gizmos.sphere(Isometry3d::from_translation(at), dot, color);
        } else {
            draw_form(&mut gizmos, form, at, to_vec3(centre), own.max(1e-6), color);
        }
    }
}
