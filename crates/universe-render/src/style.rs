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

/// Camera-relative drawing origin (#558, ADR 0026; planet frame #774).
///
/// The camera's own position in open-cell units (`camera`). Every drawn
/// position is taken relative to it in `f64` before narrowing to `f32`, so
/// things near the camera keep full precision at every level (a 1.8 m body on
/// a whole planet included) and the camera itself sits at the render origin.
/// [`crate::camera::sync_camera`] sets it each frame before drawing; it
/// stays at the cell centre where no window runs (headless tests), so
/// relative and absolute positions agree there.
///
/// At L10 the origin also carries the planet frame (`l10`, recomputed only
/// when the open cell changes): `at`, `cell` and `form` then route every L10
/// position through the one planet-frame conversion (Q4 A, review #774
/// item 2), so no call site can miss it. A missing planet yields no
/// conversion (`None`), never an Earth fallback (review #774 item 4).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct RenderOrigin {
    /// Camera position in open-cell units.
    pub camera: [f64; 3],
    /// Planet frame for the open L10 cell, if any.
    pub l10: Option<universe_core::planet_frame::PlanetFrame>,
    /// Open-cell seed `l10` was built for (cell-change detection).
    pub l10_seed: Option<u64>,
}

impl RenderOrigin {
    /// `position` relative to the origin, still in `f64`.
    ///
    /// The direct primitive for non-L10 levels and non-positional use;
    /// L10 positions go through [`RenderOrigin::at`] instead.
    #[must_use]
    pub(crate) fn rel(&self, position: [f64; 3]) -> [f64; 3] {
        [
            position[0] - self.camera[0],
            position[1] - self.camera[1],
            position[2] - self.camera[2],
        ]
    }

    /// `position` relative to the origin, narrowed for the GPU.
    ///
    /// At L10 this is the exact render path measured headlessly
    /// (`l10_drawn_f32` in core); elsewhere it is the direct
    /// camera-relative narrowing.
    #[must_use]
    pub(crate) fn at(&self, position: [f64; 3]) -> Vec3 {
        match self.l10 {
            Some(frame) => {
                let drawn =
                    universe_core::planet_frame::l10_drawn_f32(position, self.camera, &frame);
                Vec3::new(drawn[0], drawn[1], drawn[2])
            }
            None => to_vec3(self.rel(position)),
        }
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
            universe_core::r#gen::Form::Thread { to, via } => {
                let (to, via) = match self.l10 {
                    Some(frame) => (
                        universe_core::planet_frame::l10_camera_offset(to, self.camera, &frame),
                        universe_core::planet_frame::l10_camera_offset(via, self.camera, &frame),
                    ),
                    None => (self.rel(to), self.rel(via)),
                };
                universe_core::r#gen::Form::Thread { to, via }
            }
            other => other,
        }
    }
}

/// A mesh entity placed at a cell-space point, re-placed relative to the
/// [`RenderOrigin`] every frame by [`place_anchored`] (#558).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub(crate) struct Anchored(pub [f64; 3]);

/// Moves every [`Anchored`] entity to its point relative to the origin.
///
/// L10 anchoring routes through the origin's planet frame automatically
/// (`RenderOrigin::at`, #774 Q4 A).
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
    reason = "E-CAST: f64 directions and non-L10 positions narrowed for the GPU, intended (L10 narrows in core)"
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
        // #770: tail removed; L1-L10 still have tints (not necessarily distinct).
        for n in [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10] {
            let level = Level::new(n).expect("rung");
            let linear = point_color_for_level(level).to_linear();
            assert!(
                linear.red.is_finite() && linear.green.is_finite() && linear.blue.is_finite(),
                "L{n} tint"
            );
            let dim = sibling_color_for_level(level).to_linear();
            assert!(
                dim.red <= linear.red && dim.green <= linear.green && dim.blue <= linear.blue,
                "sibling tint stays dimmer: L{n}"
            );
        }
    }

    #[test]
    fn camera_relative_drawing_keeps_a_true_size_body_on_a_planet() {
        // ADR 0026 + #774 Q7 A: at L10 one cell unit spans 10^7.11 m (about
        // 12,900 km), so a 1.8 m body is about 1e-7 units tall, standing on
        // the true surface (Earth about 0.49 units from the cell centre).
        let unit_m = 10f64.powf(universe_core::nest::LADDER_EXPONENTS[9]);
        let height = 1.8 / unit_m;
        let foot = [0.0, universe_core::frame::l10_earth_radius_cells(), 0.0];
        let head = [foot[0], foot[1] + height, foot[2]];
        // Camera a few body heights behind, as the third-person view sits.
        let camera = [3.0 * height, foot[1] + 1.5 * height, 0.0];
        let origin = RenderOrigin {
            camera,
            l10: None,
            l10_seed: None,
        };
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
        // At L10 the same body routes through the planet frame and matches
        // (review #774 item 2: the routing lives inside `at`).
        let frame = universe_core::planet_frame::PlanetFrame::for_radius_earth(1.0);
        let routed = RenderOrigin {
            camera,
            l10: Some(frame),
            l10_seed: None,
        };
        let drawn_l10 = f64::from((routed.at(head) - routed.at(foot)).length());
        assert!(
            ((drawn_l10 - height) / height).abs() < 1e-3,
            "L10 routing keeps the body: {drawn_l10} vs {height}"
        );
        assert_eq!(routed.at(camera), Vec3::ZERO, "routed camera is the origin");
    }
}
