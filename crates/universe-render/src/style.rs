//! Era colors, render-boundary conversions, and the pick radius.
//!
//! Thin reader over [`crate::tokens`]: every visual value lives in the
//! tokens module by role, this module only maps levels through the era
//! tables. Everything here is `pub(crate)`: only this crate uses it.

use crate::tokens::{ERA_SIBLING_TINTS, ERA_TINTS, SHELL_FALLBACK_TINT};
use bevy::color::Color;
use bevy::math::Vec3;
use universe_core::coords::Level;
use universe_core::r#gen::ColorInfo;

/// Marker color per content era, read from the tokens table.
#[must_use]
pub(crate) fn point_color_for_level(level: Level) -> Color {
    let n = level.get();
    for &(lo, hi, tint) in &ERA_TINTS {
        if n >= lo && n <= hi {
            return tint;
        }
    }
    SHELL_FALLBACK_TINT
}

/// Dimmed era color for the parent cell's siblings, read from the tokens table.
#[must_use]
pub(crate) fn sibling_color_for_level(level: Level) -> Color {
    let n = level.get();
    for &(lo, hi, tint) in &ERA_SIBLING_TINTS {
        if n >= lo && n <= hi {
            return tint;
        }
    }
    SHELL_FALLBACK_TINT
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tail_rung_has_its_own_tint() {
        let mut seen = std::collections::HashSet::new();
        for n in [11u8, 12, 13, 14] {
            let level = Level::new(n).expect("tail rung");
            let linear = point_color_for_level(level).to_linear();
            let key = (
                linear.red.to_bits(),
                linear.green.to_bits(),
                linear.blue.to_bits(),
            );
            assert!(seen.insert(key), "tail tints must differ: L{n}");
            let dim = sibling_color_for_level(level).to_linear();
            assert!(
                dim.red <= linear.red && dim.green <= linear.green && dim.blue <= linear.blue,
                "sibling tint stays dimmer: L{n}"
            );
        }
    }
}
