//! Gizmo drawing of axes, cells, previews, and siblings.
//!
//! Everything here is `pub(crate)`: only this crate uses it.

use crate::PreviewCache;
use crate::Universe;
use crate::input::Navigation;
use crate::style::{point_color_for_level, scaled, sibling_color_for_level, to_vec3};
use bevy::math::{DVec3, Isometry3d};
use bevy::prelude::*;
use universe_core::coords::Level;
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

/// Keeps the preview cache in line with the camera (regenerates only when
/// the set of previewed markers changes).
pub(crate) fn sync_previews(universe: Res<Universe>, mut previews: ResMut<PreviewCache>) {
    previews.sync(&universe);
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
        let marker_color = if nav.target == Some(index) {
            scaled(Color::srgb(1.0, 0.0, 1.0), brightness.max(0.5))
        } else if nav.hover == Some(index) {
            scaled(Color::WHITE, brightness.max(0.4))
        } else {
            scaled(color, brightness)
        };
        let isometry = Isometry3d::from_translation(to_vec3(point.position));
        if brightness > 0.0 {
            gizmos.sphere(isometry, drawn, marker_color);
        }
        if nav.target == Some(index) {
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
        let lit = scaled(color, brightness);
        for point in &content.points {
            let world = child_world_position(marker_pos, ratio, point.position);
            let distance = (camera - DVec3::from_array(world)).length();
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: render-domain narrowing of a radius, intended"
            )]
            let drawn = drawn_radius(child_radius, distance) as f32;
            gizmos.sphere(Isometry3d::from_translation(to_vec3(world)), drawn, lit);
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
        let (position, radius) = sibling_in_open_units(entered, point.position);
        let distance = (camera - DVec3::from_array(position)).length();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a radius, intended"
        )]
        let drawn = drawn_radius(radius, distance) as f32;
        gizmos.sphere(
            Isometry3d::from_translation(to_vec3(position)),
            drawn,
            color,
        );
    }
}
