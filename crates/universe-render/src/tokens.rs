//! Design tokens: every visual value named by role (#446).
//!
//! Single definition site for the marker design system. Drawing code reads
//! these names and holds no numbers of its own (`E-RENDER-SCOPE`): `style.rs`
//! maps levels through the era tables, `draw.rs`/`forms.rs` size through the
//! floors, and the gizmo width is applied explicitly from [`LINE_MIN_PX`].
//! Numeric display rules (contrast floor, ladder steps, pick sizes) alias the
//! pure core rules so the two crates never drift.

use bevy::prelude::*;

/// Pick radius in logical pixels for hover and click (mouse, Spec v1 AC12).
pub(crate) const PICK_MOUSE_PX: f32 = 24.0;

/// Minimum gizmo line width in pixels: every marker line draws at least this.
pub(crate) const LINE_MIN_PX: f32 = 2.0;

/// Minimum actionable dot diameter on screen in pixels (Spec v1 AC2).
pub(crate) const DOT_MIN_PX: f32 = 12.0;

/// Screen-angle reference: radians per pixel at 1080p under a 45-degree
/// vertical field, used only to turn pixel floors into world sizes.
pub(crate) const PX_RAD: f64 = 0.000727;

/// Actionable brightness floor, aliased from the core contrast rule so the
/// drawers and the pure checks never drift (Spec v1 AC3).
pub(crate) use universe_core::contrast::ACTIONABLE_BRIGHTNESS_FLOOR;

/// Previewed-children ladder step, just under the shell (Spec v1 AC5).
pub(crate) use universe_core::contrast::LADDER_CHILD;

/// World-space radius of the dot floor at `distance` from the camera.
#[must_use]
pub(crate) fn dot_floor_world(distance: f64) -> f32 {
    if !distance.is_finite() || distance <= 0.0 {
        return 0.0;
    }
    let angle = f64::from(DOT_MIN_PX / 2.0) * PX_RAD;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: pixel-floor world size narrowed for the GPU, intended"
    )]
    let floor = (distance * angle.tan()) as f32;
    floor.max(0.0)
}

/// Brightness of an open-cell portal marker: the shell curve clamped to the
/// actionable floor (Spec v1 AC3). Populations and background context keep
/// the curve (AC5 ladder, below allowed).
#[must_use]
pub(crate) fn actionable_brightness(brightness: f32) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: core floor narrowed for the GPU, intended"
    )]
    let floor = ACTIONABLE_BRIGHTNESS_FLOOR as f32;
    brightness.max(floor)
}

/// Brightness of previewed children: the preview curve scaled one ladder
/// step under the shell (Spec v1 AC5).
#[must_use]
pub(crate) fn child_brightness(brightness: f32) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: core ladder step narrowed for the GPU, intended"
    )]
    let step = LADDER_CHILD as f32;
    brightness * step
}

/// Circle resolution for form outlines.
pub(crate) const FORM_RESOLUTION: u32 = 24;

/// Selection outline radius as a multiple of the marker dot.
pub(crate) const SELECTION_RING_SCALE: f32 = 1.6;

/// Brightness floor of the selected marker (magenta at this floor keeps 3:1).
pub(crate) const TARGET_BRIGHTNESS_FLOOR: f32 = 0.7;

/// Brightness floor of the hovered marker.
pub(crate) const HOVER_BRIGHTNESS_FLOOR: f32 = 0.4;

/// Population dot size as a fraction of the portal dot.
pub(crate) const POPULATION_SIZE_SCALE: f32 = 0.5;

/// Population brightness as a fraction of the portal brightness.
pub(crate) const POPULATION_BRIGHTNESS_SCALE: f32 = 0.5;

/// Faint scale of the L1 graticule against the level tint.
pub(crate) const GRATICULE_BRIGHTNESS: f32 = 0.3;

/// Fallback shell tint above the root (no shallower era exists).
pub(crate) const SHELL_FALLBACK_TINT: Color = Color::srgb(0.6, 0.6, 0.6);

/// Axis orientation cues (debug axes only, never markers).
pub(crate) const AXIS_X_TINT: Color = Color::srgb(1.0, 0.0, 0.0);
/// Axis orientation cues (debug axes only, never markers).
pub(crate) const AXIS_Y_TINT: Color = Color::srgb(0.0, 1.0, 0.0);
/// Axis orientation cues (debug axes only, never markers).
pub(crate) const AXIS_Z_TINT: Color = Color::srgb(0.0, 0.5, 1.0);

/// Selection outline tint (shape carries the state, never hue alone).
pub(crate) const OUTLINE_TINT: Color = Color::srgb(1.0, 0.0, 1.0);

/// Selection shell tint behind the outline ring (full magenta keeps 3:1).
pub(crate) const OUTLINE_SHELL_TINT: Color = Color::srgb(1.0, 0.0, 1.0);

/// Hover tint: a brighter variant of the same selection shape.
pub(crate) const FOCUS_TINT: Color = Color::WHITE;

/// Era tint table: marker colour per content era.
pub(crate) const ERA_TINTS: [(u8, u8, Color); 7] = [
    (1, 3, Color::srgb(0.3, 0.8, 1.0)),
    (4, 9, Color::srgb(1.0, 0.85, 0.4)),
    (10, 10, Color::srgb(0.4, 1.0, 0.5)),
    (11, 11, Color::srgb(0.7, 0.5, 1.0)),
    (12, 12, Color::srgb(1.0, 0.6, 0.2)),
    (13, 13, Color::srgb(0.3, 0.9, 0.8)),
    (14, 14, Color::WHITE),
];

/// Dimmed era tint table for parent-cell siblings behind the camera.
pub(crate) const ERA_SIBLING_TINTS: [(u8, u8, Color); 7] = [
    (1, 3, Color::srgb(0.12, 0.32, 0.4)),
    (4, 9, Color::srgb(0.4, 0.34, 0.16)),
    (10, 10, Color::srgb(0.16, 0.4, 0.2)),
    (11, 11, Color::srgb(0.28, 0.2, 0.4)),
    (12, 12, Color::srgb(0.4, 0.24, 0.08)),
    (13, 13, Color::srgb(0.12, 0.36, 0.32)),
    (14, 14, Color::srgb(0.4, 0.4, 0.4)),
];

/// Applies the line-width floor to the default gizmo group.
///
/// Missing group is a no-op return, never a panic.
pub(crate) fn apply_gizmo_width(mut store: ResMut<GizmoConfigStore>) {
    use bevy::gizmos::config::DefaultGizmoConfigGroup;
    let Some((config, _)) = store.get_config_mut::<DefaultGizmoConfigGroup>() else {
        return;
    };
    config.line.width = LINE_MIN_PX;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Source of `style.rs` next to this module.
    const STYLE_SOURCE: &str = include_str!("style.rs");
    /// Source of `draw.rs` next to this module.
    const DRAW_SOURCE: &str = include_str!("draw.rs");
    /// Source of `forms.rs` next to this module.
    const FORMS_SOURCE: &str = include_str!("forms.rs");

    #[test]
    fn no_numeric_colour_literal_remains_outside_tokens() {
        for (name, source) in [
            ("style.rs", STYLE_SOURCE),
            ("draw.rs", DRAW_SOURCE),
            ("forms.rs", FORMS_SOURCE),
        ] {
            for marker in ["Color::srgb(0", "Color::srgb(1", "Color::srgba(0"] {
                assert!(
                    !source.contains(marker),
                    "{name} holds a numeric colour outside tokens.rs: {marker}"
                );
            }
            assert!(
                !source.contains("Color::WHITE"),
                "{name} holds a named white outside tokens.rs"
            );
        }
    }

    #[test]
    fn no_visual_size_const_remains_outside_tokens() {
        for (name, source) in [
            ("style.rs", STYLE_SOURCE),
            ("draw.rs", DRAW_SOURCE),
            ("forms.rs", FORMS_SOURCE),
        ] {
            for marker in [
                "const PICK_",
                "const LINE_",
                "const DOT_",
                "const LABEL_",
                "PICK_PIXELS",
            ] {
                assert!(
                    !source.contains(marker),
                    "{name} holds a visual size outside tokens.rs: {marker}"
                );
            }
        }
    }

    #[test]
    fn gizmo_width_comes_from_the_token() {
        use bevy::ecs::system::RunSystemOnce;
        use bevy::gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore};
        const {
            assert!(LINE_MIN_PX >= 2.0);
        }
        let mut app = App::new();
        app.init_resource::<GizmoConfigStore>();
        app.init_gizmo_group::<DefaultGizmoConfigGroup>();
        let _ = app.world_mut().run_system_once(apply_gizmo_width);
        let store = app.world().resource::<GizmoConfigStore>();
        let (config, _) = store.config::<DefaultGizmoConfigGroup>();
        assert_eq!(config.line.width, LINE_MIN_PX);
    }

    #[test]
    fn pick_token_matches_the_spec_size() {
        assert_eq!(PICK_MOUSE_PX, 24.0);
    }

    #[test]
    fn dot_floor_holds_twelve_pixels_and_actionable_never_dims() {
        // At ten cell units the floor world size reads back to the pixel
        // floor through the screen-angle reference.
        let world = dot_floor_world(10.0);
        let angle = (f64::from(world) / 10.0).atan();
        let diameter_px = angle / PX_RAD * 2.0;
        assert!((diameter_px - f64::from(DOT_MIN_PX)).abs() < 0.05);
        assert_eq!(dot_floor_world(f64::NAN), 0.0);
        assert_eq!(dot_floor_world(0.0), 0.0);
        // Actionable markers never drop below the contrast floor; context
        // keeps the curve.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: test floor narrowed for the GPU, intended"
        )]
        let floor = ACTIONABLE_BRIGHTNESS_FLOOR as f32;
        assert_eq!(actionable_brightness(0.15), floor);
        assert_eq!(actionable_brightness(1.0), 1.0);
        assert!(child_brightness(1.0) < 1.0, "children sit under the shell");
    }
}
