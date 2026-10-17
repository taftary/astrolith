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

/// Portal-mark ring radius as a fraction of the marker dot.
pub(crate) const PORTAL_RING_SCALE: f32 = 0.6;

/// Portal-mark ring minimum diameter on screen in pixels.
pub(crate) const PORTAL_RING_MIN_PX: f32 = 6.0;

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

/// Label text tint.
pub(crate) const LABEL_TINT: Color = Color::WHITE;

/// Selected-label halo behind the text.
pub(crate) const LABEL_HALO: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);

/// Label rule threshold and frame cap, aliased from the core pick rule.
pub(crate) use universe_core::pick::{LABEL_CAP, LABEL_MIN_PX};

/// Touch pick diameters and the cycle-order cap, aliased from the core rule
/// (touch input itself is a later notion; the sizes are fixed here).
pub(crate) use universe_core::pick::CYCLE_CAP;

/// Touch pick diameter in points, defined for when touch input arrives.
pub(crate) const PICK_TOUCH_PT: f32 = 44.0;

/// Touch pick diameter in density-independent pixels.
pub(crate) const PICK_TOUCH_DP: f32 = 48.0;

const _: () = assert!(PICK_TOUCH_PT >= PICK_MOUSE_PX);
const _: () = assert!(PICK_TOUCH_DP >= PICK_MOUSE_PX);

/// Base tone of the planet ground (#729, ADR 0028): the detail window and
/// the far ball share it, so the crossfade swaps no hues.
pub(crate) const SURFACE_TONE: Color = Color::srgb(0.32, 0.30, 0.28);

/// Label offset from its portal in logical pixels.
pub(crate) const LABEL_OFFSET_PX: f32 = 10.0;

/// Menu and screen tokens (#473): every screen size and tint named by role.
/// The screens cover the window with a dark, nearly opaque background; all
/// buttons are at least 48 px tall (touch guide R-TCH-1); text meets the
/// 4.5:1 contrast floor on the dark background (checked in `panel`, T4).
/// Round menu button diameter in logical pixels (top-right corner).
pub(crate) const MENU_BUTTON_DIAMETER_PX: f32 = 48.0;

/// Screen button height in logical pixels (at least 48 px per spec).
pub(crate) const SCREEN_BUTTON_HEIGHT_PX: f32 = 56.0;

/// Screen text tint (white on the dark background).
pub(crate) const MENU_TEXT_TINT: Color = Color::WHITE;

/// Unselected list text tint (dimmer than the selection, still past 4.5:1).
pub(crate) const MENU_DIM_TINT: Color = Color::srgb(0.72, 0.74, 0.8);

/// Selected-item bar tint (shape carries the state, never hue alone).
pub(crate) const MENU_SELECT_TINT: Color = Color::srgb(0.3, 0.8, 1.0);

/// Window width below which screens collapse to list-then-detail.
pub(crate) const NARROW_BREAK_PX: f32 = 720.0;

/// Screen title text size, logical px (#473 Spec v3).
pub(crate) const TEXT_TITLE_PX: f32 = 18.0;

/// Button and body text size, logical px.
pub(crate) const TEXT_BODY_PX: f32 = 14.0;

/// Category list item text size, logical px.
pub(crate) const TEXT_LIST_PX: f32 = 13.0;

/// Small note text size ("Coming soon"), logical px.
pub(crate) const TEXT_NOTE_PX: f32 = 11.0;

/// Lucide icon glyph size, logical px.
pub(crate) const TEXT_ICON_PX: f32 = 18.0;

/// Gap between a button's icon and its label, logical px.
pub(crate) const MENU_ICON_GAP_PX: f32 = 10.0;

/// Instrument-panel card fill, preview `--card` #0d1017 (#473 Spec v3).
pub(crate) const CARD_TINT: Color = Color::srgb(0.051, 0.063, 0.090);

/// Card and button outer border, preview `--line` #2a3442.
pub(crate) const CARD_LINE_TINT: Color = Color::srgb(0.165, 0.204, 0.259);

/// Card inner frame and button inner border, preview `--inner` #1b2430.
pub(crate) const CARD_INNER_TINT: Color = Color::srgb(0.106, 0.141, 0.188);

/// Corner brackets, hover borders, and selection, preview `--cyan` #4dccff.
pub(crate) const CHROME_CYAN: Color = Color::srgb(0.302, 0.800, 1.0);

/// Quit's red-tinted inner border, preview `--danger` #ff5c6c at 45 %.
pub(crate) const CHROME_DANGER: Color = Color::srgba(1.0, 0.361, 0.424, 0.45);

/// Quit's red icon, preview #ff5c6c.
pub(crate) const CHROME_DANGER_TEXT: Color = Color::srgb(1.0, 0.361, 0.424);

/// Button face fill at rest, preview #121822.
pub(crate) const BUTTON_FILL: Color = Color::srgb(0.071, 0.094, 0.133);

/// Button face fill on hover (brighter), preview #152030.
pub(crate) const BUTTON_HOVER_FILL: Color = Color::srgb(0.082, 0.125, 0.188);

/// Button face fill while pressed (darker).
pub(crate) const BUTTON_PRESSED_FILL: Color = Color::srgb(0.043, 0.059, 0.086);

/// Shadow under a card, preview `0 12px 40px rgba(0,0,0,.6)`.
pub(crate) const CARD_SHADOW_TINT: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);

/// Shadow vertical offset and blur, logical px.
pub(crate) const CARD_SHADOW_Y_PX: f32 = 12.0;

/// Shadow blur radius, logical px.
pub(crate) const CARD_SHADOW_BLUR_PX: f32 = 40.0;

/// Dim veil colour (preview `--scrim` rgb 5,6,9); alpha comes from the
/// Background dimming setting.
pub(crate) const DIM_VEIL_RGB: (f32, f32, f32) = (0.020, 0.024, 0.035);

/// Dim veil darkness until the setting is read (80 %).
pub(crate) const DIM_VEIL_DEFAULT: f32 = 0.80;

/// Card corner radius, logical px.
pub(crate) const CARD_RADIUS_PX: f32 = 8.0;

/// Card inner frame inset and radius, logical px.
pub(crate) const CARD_INNER_INSET_PX: f32 = 4.0;

/// Card inner frame radius, logical px.
pub(crate) const CARD_INNER_RADIUS_PX: f32 = 5.0;

/// Card padding, logical px.
pub(crate) const CARD_PADDING_PX: f32 = 18.0;

/// Corner bracket arm length and stroke, logical px.
pub(crate) const BRACKET_PX: f32 = 12.0;

/// Corner bracket stroke width, logical px.
pub(crate) const BRACKET_STROKE_PX: f32 = 2.0;

/// Button corner radius, logical px.
pub(crate) const BUTTON_RADIUS_PX: f32 = 6.0;

/// Button inner border inset (outer padding), logical px.
pub(crate) const BUTTON_INNER_INSET_PX: f32 = 3.0;

/// Button inner border radius, logical px.
pub(crate) const BUTTON_INNER_RADIUS_PX: f32 = 4.0;

/// Menu card width, logical px (about 320).
pub(crate) const MENU_CARD_WIDTH_PX: f32 = 320.0;

/// Guides/Settings card share of the window when wide, percent.
pub(crate) const PANEL_CARD_PERCENT: f32 = 90.0;

/// Margin around the Guides/Settings card when narrow, logical px.
pub(crate) const PANEL_NARROW_MARGIN_PX: f32 = 8.0;

/// Gap between Settings controls in one row, logical px.
pub(crate) const ROW_GAP_PX: f32 = 12.0;

/// Width of a Settings row label, logical px.
pub(crate) const SETTING_LABEL_WIDTH_PX: f32 = 200.0;

/// Fill of an on-screen touch control (#561): see-through over the sky.
pub(crate) const TOUCH_FILL: Color = Color::srgba(0.071, 0.094, 0.133, 0.55);

/// Fill of a held touch control or the stick thumb (#561).
pub(crate) const TOUCH_PRESSED_FILL: Color = Color::srgba(0.32, 0.36, 0.44, 0.75);

/// Edge of an on-screen touch control (#561).
pub(crate) const TOUCH_EDGE: Color = Color::srgba(0.72, 0.74, 0.8, 0.7);

/// Width of an action name on the Keys page, logical px (#560).
pub(crate) const KEY_LABEL_WIDTH_PX: f32 = 360.0;

/// Width of a per-category Reset button, logical px.
pub(crate) const RESET_BUTTON_WIDTH_PX: f32 = 220.0;

/// Width of a small Settings button (stepper, toggle), logical px.
pub(crate) const SMALL_BUTTON_WIDTH_PX: f32 = 72.0;

/// UI layer of the universe view: HUD readout, portal labels, menu button
/// (#473 Spec v3 layers; `GlobalZIndex`, so spawn order never decides).
pub(crate) const LAYER_UNIVERSE: i32 = 0;

/// UI layer of the Menu, Settings, and Guides cards.
pub(crate) const LAYER_SCREEN: i32 = 20;

/// UI layer of the dim veil behind every screen.
pub(crate) const LAYER_DIM: i32 = 10;

/// UI layer of the second dim veil behind a pop-up (also the click blocker).
pub(crate) const LAYER_POPUP_DIM: i32 = 30;

/// UI layer of pop-up cards (Quit confirmation), above every screen.
pub(crate) const LAYER_POPUP: i32 = 40;

/// Fixed darkness of the pop-up veil (not the Background dimming setting).
pub(crate) const POPUP_VEIL_DARKNESS: f32 = 0.5;

/// Quit pop-up card width, logical px.
pub(crate) const POPUP_CARD_WIDTH_PX: f32 = 320.0;

/// Category-list column width in logical pixels (wide layout).
pub(crate) const PANEL_LIST_WIDTH_PX: f32 = 260.0;

/// Selection-bar width in logical pixels.
pub(crate) const PANEL_BAR_WIDTH_PX: f32 = 4.0;

/// Scrollbar gutter width in logical pixels (panel content thumbs).
pub(crate) const SCROLLBAR_WIDTH_PX: f32 = 8.0;

/// Scrollbar thumb minimum length in logical pixels (stays grabbable).
pub(crate) const SCROLLBAR_THUMB_MIN_PX: f32 = 24.0;

/// Scrollbar thumb tint (light slate on the dark card).
pub(crate) const SCROLLBAR_THUMB_TINT: Color = Color::srgb(0.55, 0.65, 0.75);

/// Gap between stacked screen buttons in logical pixels.
pub(crate) const MENU_BUTTON_GAP_PX: f32 = 12.0;

/// Screen edge margin in logical pixels.
pub(crate) const MENU_MARGIN_PX: f32 = 12.0;
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
    fn label_tokens_match_the_core_rule() {
        use universe_core::pick::{LABEL_CAP as CORE_CAP, LABEL_MIN_PX as CORE_MIN};
        assert_eq!(LABEL_MIN_PX, CORE_MIN);
        assert_eq!(LABEL_CAP, CORE_CAP);
    }

    #[test]
    fn touch_and_cycle_tokens_match_the_core_rule() {
        use universe_core::pick::{
            CYCLE_CAP as CORE_CYCLE, PICK_TOUCH_DP as CORE_DP, PICK_TOUCH_PT as CORE_PT,
        };
        assert_eq!(PICK_TOUCH_PT, CORE_PT);
        assert_eq!(PICK_TOUCH_DP, CORE_DP);
        assert_eq!(CYCLE_CAP, CORE_CYCLE);
        const {
            assert!(PICK_TOUCH_PT == 44.0);
            assert!(PICK_TOUCH_DP == 48.0);
        }
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
