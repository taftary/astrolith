//! Era colors, render-boundary conversions, and the pick radius.
//!
//! Thin reader over [`crate::tokens`]: every visual value lives in the
//! tokens module by role, this module only maps levels through the era
//! tables. Everything here is `pub(crate)`: only this crate uses it.

use crate::tokens::{ERA_SIBLING_TINTS, ERA_TINTS, SHELL_FALLBACK_TINT};
use bevy::color::Color;
use bevy::ecs::component::Component;
use bevy::ecs::resource::Resource;
use bevy::ecs::system::{Query, Res};
use bevy::math::Vec3;
use bevy::transform::components::Transform;
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
/// keep readable color at every brightness curve. Dark reds map through the
/// contrast display tint (#446) so protan vision never reads them as black;
/// snapshots keep the sampler value, only drawing maps.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "E-CAST: tint channels narrowed for the GPU, intended"
)]
pub(crate) fn tint_color(tint: ColorInfo) -> Color {
    let shown = universe_core::contrast::display_tint(tint);
    Color::srgb(shown.red as f32, shown.green as f32, shown.blue as f32)
}

/// Camera-relative drawing origin (#558, ADR 0026).
///
/// The camera's own position in open-cell units. Every drawn position is
/// taken relative to it in `f64` before narrowing to `f32`, so things near
/// the camera keep full precision at every level (a 1.8 m body on a whole
/// planet included) and the camera itself sits at the render origin.
/// [`crate::camera::sync_camera`] sets it each frame before drawing; it
/// stays at the cell centre where no window runs (headless tests), so
/// relative and absolute positions agree there.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct RenderOrigin(pub [f64; 3]);

impl RenderOrigin {
    /// `position` relative to the origin, still in `f64`.
    #[must_use]
    pub(crate) fn rel(&self, position: [f64; 3]) -> [f64; 3] {
        [
            position[0] - self.0[0],
            position[1] - self.0[1],
            position[2] - self.0[2],
        ]
    }

    /// `position` relative to the origin, narrowed for the GPU.
    #[must_use]
    pub(crate) fn at(&self, position: [f64; 3]) -> Vec3 {
        to_vec3(self.rel(position))
    }

    /// The open cell's centre relative to the origin (what `Vec3::ZERO`
    /// meant before camera-relative drawing).
    #[must_use]
    pub(crate) fn cell(&self) -> Vec3 {
        self.at([0.0; 3])
    }

    /// `form` with its position-valued parameters (thread endpoints)
    /// taken relative to the origin; directions and sizes are untouched.
    #[must_use]
    pub(crate) fn form(&self, form: universe_core::r#gen::Form) -> universe_core::r#gen::Form {
        match form {
            universe_core::r#gen::Form::Thread { to, via } => universe_core::r#gen::Form::Thread {
                to: self.rel(to),
                via: self.rel(via),
            },
            other => other,
        }
    }
}

/// A mesh entity placed at a cell-space point, re-placed relative to the
/// [`RenderOrigin`] every frame by [`place_anchored`] (#558).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub(crate) struct Anchored(pub [f64; 3]);

/// Moves every [`Anchored`] entity to its point relative to the origin.
pub(crate) fn place_anchored(
    origin: Res<RenderOrigin>,
    mut anchored: Query<(&Anchored, &mut Transform)>,
) {
    for (anchor, mut transform) in &mut anchored {
        let at = origin.at(anchor.0);
        if transform.translation != at {
            transform.translation = at;
        }
    }
}

/// Converts a cell-local `f64` vector (a direction, or a position already
/// taken relative to the [`RenderOrigin`]) to a render `Vec3`.
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

    #[test]
    fn camera_relative_drawing_keeps_a_true_size_body_on_a_planet() {
        // ADR 0026: at L10 one cell unit spans about 18,000 km, so a 1.8 m
        // body is about 1e-7 units tall, standing on the planet surface
        // about 0.35 units from the cell centre.
        let unit_m = 10f64.powf(universe_core::nest::LADDER_EXPONENTS[9]);
        let height = 1.8 / unit_m;
        let foot = [0.0, universe_core::terrain::PLANET_RADIUS_CELL, 0.0];
        let head = [foot[0], foot[1] + height, foot[2]];
        // Camera a few body heights behind, as the third-person view sits.
        let camera = [3.0 * height, foot[1] + 1.5 * height, 0.0];
        let origin = RenderOrigin(camera);
        let drawn = f64::from((origin.at(head) - origin.at(foot)).length());
        assert!(
            ((drawn - height) / height).abs() < 1e-3,
            "relative drawing keeps the body: {drawn} vs {height}"
        );
        // Drawn from the cell centre the same body loses its shape.
        let absolute = f64::from((to_vec3(head) - to_vec3(foot)).length());
        assert!(
            ((absolute - height) / height).abs() > 0.05,
            "absolute drawing was the problem: {absolute} vs {height}"
        );
        assert_eq!(origin.at(camera), Vec3::ZERO, "the camera is the origin");
    }
}
