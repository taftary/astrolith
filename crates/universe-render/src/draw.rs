//! Gizmo drawing of axes, cells, previews, siblings, and room outlines.
//!
//! Everything here is `pub(crate)`: only this crate uses it.

use crate::PreviewCache;
use crate::Universe;
use crate::input::Navigation;
use crate::style::{point_color_for_level, scaled, sibling_color_for_level, tint_color, to_vec3};
use bevy::math::{DVec3, Isometry3d, bounding::Aabb3d};
use bevy::prelude::*;
use universe_core::coords::{Level, ParentUnits};
use universe_core::r#gen::MarkerKind;
use universe_core::nav::{MarkerIndex, drawn_radius, open_marker_radius, sibling_in_open_units};
use universe_core::nest::{
    angular_radius, child_ratio, child_world_position, children_brightness, shell_brightness,
};

/// Draws the open cell's RGB axes (orientation cue) at half-cell length.
pub(crate) fn draw_axes(mut gizmos: Gizmos) {
    gizmos.line(Vec3::ZERO, Vec3::X * 0.5, Color::srgb(1.0, 0.0, 0.0));
    gizmos.line(Vec3::ZERO, Vec3::Y * 0.5, Color::srgb(0.0, 1.0, 0.0));
    gizmos.line(Vec3::ZERO, Vec3::Z * 0.5, Color::srgb(0.0, 0.5, 1.0));
}

/// Furniture outline half-size as a multiple of the indicator radius (#375).
///
/// Room points carry radius 0.01; outlines at four times that read as small
/// boxes in the one-unit room cell without touching each other.
pub(crate) const FURNITURE_OUTLINE_SCALE: f32 = 4.0;

/// Draws furniture outlines for the open L14 room (#375, ADR 0016).
///
/// Every L14 point is furniture (the room generator emits populations
/// only): each gets an axis-aligned box outline in the room tint, sized by
/// its indicator radius. Returns immediately at any other level. No meshes:
/// `E-RENDER-NO-MESH` holds at every rung.
pub(crate) fn draw_room_outlines(mut gizmos: Gizmos, universe: Res<Universe>) {
    if universe.level().get() != 14 {
        return;
    }
    let color = point_color_for_level(universe.level());
    for point in &universe.open.points {
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
/// Markers draw at the true child size with the impostor clamp, dimmed by
/// [`shell_brightness`](universe_core::nest::shell_brightness) as their interior resolves. The open cell's own
/// shell (radius 0.5) uses the same curve in the parent's era color, so the
/// marker you entered and the cell you are in are one continuous object.
/// The hovered marker is white, the target magenta.
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
    let radius = universe.marker_radius();
    for (index, point) in universe.open.points.iter().enumerate() {
        let position = DVec3::from_array(point.position);
        let distance = (camera - position).length();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a radius, intended"
        )]
        let drawn = drawn_radius(radius, distance) as f32;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a brightness, intended"
        )]
        let brightness = shell_brightness(angular_radius(radius, distance)) as f32;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: drawn marker index into a budgeted cell, always fits u32"
        )]
        let index = index as u32;
        // Populations render at half size and half brightness and never
        // highlight: only portals take the target or the hover (#151).
        // Tinted points (stars, galaxies) read their core hue (#157).
        let portal = point.kind == MarkerKind::Portal;
        let drawn = if portal { drawn } else { drawn * 0.5 };
        let brightness = if portal { brightness } else { brightness * 0.5 };
        let base = point.tint.map_or(color, tint_color);
        let marker_color = if portal && nav.target == Some(index) {
            scaled(Color::srgb(1.0, 0.0, 1.0), brightness.max(0.5))
        } else if portal && nav.hover == Some(index) {
            scaled(Color::WHITE, brightness.max(0.4))
        } else {
            scaled(base, brightness)
        };
        let isometry = Isometry3d::from_translation(to_vec3(point.position));
        if brightness > 0.0 {
            gizmos.sphere(isometry, drawn, marker_color);
        }
        if portal && nav.target == Some(index) {
            gizmos.sphere(isometry, drawn * 1.6, Color::srgb(0.6, 0.0, 0.6));
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
    let child_radius = open_marker_radius(child_level, 0.01) * ratio;
    let color = point_color_for_level(child_level);
    for (marker, content) in previews.entries() {
        let Some(marker_pos) = universe.marker(MarkerIndex(*marker)) else {
            continue;
        };
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
            let world = child_world_position(ParentUnits(marker_pos), ratio, point.position);
            let distance = (camera - DVec3::from_array(world.0)).length();
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: render-domain narrowing of a radius, intended"
            )]
            let preview_drawn = drawn_radius(child_radius, distance) as f32;
            let base = point.tint.map_or(color, tint_color);
            let lit = scaled(base, brightness);
            let (preview_drawn, preview_lit) = if point.kind == MarkerKind::Portal {
                (preview_drawn, lit)
            } else {
                (preview_drawn * 0.5, scaled(base, brightness * 0.5))
            };
            gizmos.sphere(
                Isometry3d::from_translation(to_vec3(world.0)),
                preview_drawn,
                preview_lit,
            );
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
    let color = sibling_color_for_level(level);
    for (index, point) in parent.points.iter().enumerate() {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: sibling marker index into a budgeted cell, always fits u32"
        )]
        let sibling_marker = index as u32;
        if sibling_marker == entered.marker {
            continue;
        }
        let (position, radius) = sibling_in_open_units(entered, ParentUnits(point.position));
        let distance = (camera - DVec3::from_array(position.0)).length();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a radius, intended"
        )]
        let drawn = drawn_radius(radius, distance) as f32;
        gizmos.sphere(
            Isometry3d::from_translation(to_vec3(position.0)),
            drawn,
            color,
        );
    }
}
