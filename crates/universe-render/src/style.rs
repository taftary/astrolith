//! Era colors, render-boundary conversions, and the pick radius.
//!
//! Everything here is `pub(crate)`: only this crate uses it.

use bevy::color::Color;
use bevy::math::Vec3;
use universe_core::coords::Level;
use universe_core::r#gen::ColorInfo;

/// Pick radius in logical pixels for hover and click.
pub(crate) const PICK_PIXELS: f32 = 24.0;

/// Marker color per content era: cyan clusters (L1-L3), warm stars
/// (L4-L9), green terrain (L10).
#[must_use]
pub(crate) fn point_color_for_level(level: Level) -> Color {
    match level.get() {
        1..=3 => Color::srgb(0.3, 0.8, 1.0),
        4..=9 => Color::srgb(1.0, 0.85, 0.4),
        10 => Color::srgb(0.4, 1.0, 0.5),
        _ => Color::WHITE,
    }
}

/// Dimmed era color for the parent cell's siblings behind the camera.
#[must_use]
pub(crate) fn sibling_color_for_level(level: Level) -> Color {
    match level.get() {
        1..=3 => Color::srgb(0.12, 0.32, 0.4),
        4..=9 => Color::srgb(0.4, 0.34, 0.16),
        10 => Color::srgb(0.16, 0.4, 0.2),
        _ => Color::srgb(0.4, 0.4, 0.4),
    }
}

/// Per-object display hue from a core tint (#157).
///
/// Hue only: emissive strength lives in the billboard quads, so gizmo dots
/// keep readable color at every brightness curve.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "E-CAST: tint channels narrowed for the GPU, intended"
)]
pub(crate) fn tint_color(tint: ColorInfo) -> Color {
    Color::srgb(tint.red as f32, tint.green as f32, tint.blue as f32)
}

/// Converts a cell-local `f64` position to a render `Vec3`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "E-CAST: f64 to f32 at the render boundary, the only such site in this crate"
)]
pub(crate) fn to_vec3(position: [f64; 3]) -> Vec3 {
    Vec3::new(position[0] as f32, position[1] as f32, position[2] as f32)
}

/// Scales a color's RGB by `brightness` (alpha untouched).
pub(crate) fn scaled(color: Color, brightness: f32) -> Color {
    let s = color.to_srgba();
    Color::srgb(
        s.red * brightness,
        s.green * brightness,
        s.blue * brightness,
    )
}
