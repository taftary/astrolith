//! Instrument-panel chrome shared by every screen (#473 Spec v3).
//!
//! One look for the Menu, Settings, Guides, and pop-up cards: the dim veil
//! behind the screens, framed cards (outer border, inner frame line,
//! rounded corners, cyan corner brackets, shadow), and double-border
//! buttons with hover, pressed, and danger states. Every value comes from
//! `tokens.rs` (`E-TOKENS`); screens only call these builders.

use crate::menu::Screen;
use crate::settings_ui::SettingsStore;
use crate::tokens::{
    BRACKET_PX, BRACKET_STROKE_PX, BUTTON_FILL, BUTTON_HOVER_FILL, BUTTON_INNER_INSET_PX,
    BUTTON_INNER_RADIUS_PX, BUTTON_PRESSED_FILL, BUTTON_RADIUS_PX, CARD_INNER_INSET_PX,
    CARD_INNER_RADIUS_PX, CARD_INNER_TINT, CARD_LINE_TINT, CARD_PADDING_PX, CARD_RADIUS_PX,
    CARD_SHADOW_BLUR_PX, CARD_SHADOW_TINT, CARD_SHADOW_Y_PX, CARD_TINT, CHROME_CYAN, CHROME_DANGER,
    DIM_VEIL_DEFAULT, DIM_VEIL_RGB, LAYER_DIM, MENU_ICON_GAP_PX,
};
use bevy::prelude::*;

/// The dim veil drawn over the universe behind every screen.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DimVeil;

/// A framed card (holds the trim children).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Card;

/// One cyan corner bracket of a card.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CornerBracket;

/// The thin inner frame line of a card.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InnerFrame;

/// A double-border button; `danger` tints its inner border red.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChromeButton {
    /// Red-tinted inner border (Quit).
    pub danger: bool,
}

/// The inner face of a [`ChromeButton`] (inner border plus fill).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ButtonFace;

/// Veil colour for a darkness in 0..=1 (pure).
#[must_use]
pub(crate) fn veil_color(darkness: f32) -> Color {
    let (r, g, b) = DIM_VEIL_RGB;
    Color::srgba(r, g, b, darkness.clamp(0.0, 1.0))
}

/// Chrome plugin: the dim veil and the button states.
pub(crate) struct ChromePlugin;

impl Plugin for ChromePlugin {
    /// Spawns the veil and registers the veil and button systems.
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_dim_veil)
            .add_systems(Update, (sync_dim_veil, style_chrome_buttons));
    }
}

/// Spawns the full-window dim veil (hidden in the universe).
pub(crate) fn spawn_dim_veil(mut commands: Commands) {
    commands.spawn((
        DimVeil,
        Name::new("dim-veil"),
        GlobalZIndex(LAYER_DIM),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(veil_color(DIM_VEIL_DEFAULT)),
        Visibility::Hidden,
    ));
}

/// Shows the veil whenever a screen is open, at the Background dimming
/// darkness from the settings store (the default without a store).
pub(crate) fn sync_dim_veil(
    screen: Res<State<Screen>>,
    store: Option<Res<SettingsStore>>,
    mut veils: Query<(&mut Visibility, &mut BackgroundColor), With<DimVeil>>,
) {
    let want = if *screen.get() == Screen::Universe {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    let darkness = store.map_or(DIM_VEIL_DEFAULT, |store| store.0.dimming.fraction());
    let tint = veil_color(darkness);
    for (mut visibility, mut fill) in &mut veils {
        if *visibility != want {
            *visibility = want;
        }
        if fill.0 != tint {
            fill.0 = tint;
        }
    }
}

/// Card node: solid fill, 1 px outer border, rounded corners, shadow.
#[must_use]
pub(crate) fn card(width: Val, height: Val) -> impl Bundle {
    (
        Card,
        Node {
            width,
            height,
            flex_direction: FlexDirection::Column,
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(CARD_RADIUS_PX)),
            padding: UiRect::all(Val::Px(CARD_PADDING_PX)),
            ..default()
        },
        BackgroundColor(CARD_TINT),
        BorderColor::all(CARD_LINE_TINT),
        BoxShadow::new(
            CARD_SHADOW_TINT,
            Val::Px(0.0),
            Val::Px(CARD_SHADOW_Y_PX),
            Val::Px(0.0),
            Val::Px(CARD_SHADOW_BLUR_PX),
        ),
    )
}

/// Spawns a card's trim: the inner frame line and the four cyan corner
/// brackets. Absolute and click-through, so content layout is untouched.
pub(crate) fn spawn_card_trim(card: &mut ChildSpawnerCommands) {
    card.spawn((
        InnerFrame,
        Name::new("card-inner-frame"),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(CARD_INNER_INSET_PX),
            right: Val::Px(CARD_INNER_INSET_PX),
            top: Val::Px(CARD_INNER_INSET_PX),
            bottom: Val::Px(CARD_INNER_INSET_PX),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(CARD_INNER_RADIUS_PX)),
            ..default()
        },
        BorderColor::all(CARD_INNER_TINT),
    ));
    let stroke = Val::Px(BRACKET_STROKE_PX);
    let none = Val::Px(0.0);
    let edge = Val::Px(-1.0);
    for (top, left) in [(true, true), (true, false), (false, true), (false, false)] {
        let radius = Val::Px(CARD_RADIUS_PX);
        card.spawn((
            CornerBracket,
            Name::new("card-bracket"),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(BRACKET_PX),
                height: Val::Px(BRACKET_PX),
                top: if top { edge } else { Val::Auto },
                bottom: if top { Val::Auto } else { edge },
                left: if left { edge } else { Val::Auto },
                right: if left { Val::Auto } else { edge },
                border: UiRect {
                    left: if left { stroke } else { none },
                    right: if left { none } else { stroke },
                    top: if top { stroke } else { none },
                    bottom: if top { none } else { stroke },
                },
                border_radius: BorderRadius {
                    top_left: if top && left { radius } else { none },
                    top_right: if top && !left { radius } else { none },
                    bottom_left: if !top && left { radius } else { none },
                    bottom_right: if !top && !left { radius } else { none },
                },
                ..default()
            },
            BorderColor::all(CHROME_CYAN),
        ));
    }
}

/// Outer part of a double-border button (the face is its one child).
#[must_use]
pub(crate) fn button(width: Val, height: Val, danger: bool) -> impl Bundle {
    (
        ChromeButton { danger },
        Button,
        Node {
            width,
            height,
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(BUTTON_RADIUS_PX)),
            padding: UiRect::all(Val::Px(BUTTON_INNER_INSET_PX)),
            ..default()
        },
        BorderColor::all(CARD_LINE_TINT),
    )
}

/// Inner face of a button: inner border, fill, and centred content row.
#[must_use]
pub(crate) fn button_face(danger: bool) -> impl Bundle {
    (
        ButtonFace,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::Px(MENU_ICON_GAP_PX),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(BUTTON_INNER_RADIUS_PX)),
            ..default()
        },
        BackgroundColor(BUTTON_FILL),
        BorderColor::all(if danger {
            CHROME_DANGER
        } else {
            CARD_INNER_TINT
        }),
    )
}

/// Outer border and face fill for a button state (pure).
#[must_use]
pub(crate) fn button_state(interaction: Interaction) -> (Color, Color) {
    match interaction {
        Interaction::Pressed => (CHROME_CYAN, BUTTON_PRESSED_FILL),
        Interaction::Hovered => (CHROME_CYAN, BUTTON_HOVER_FILL),
        Interaction::None => (CARD_LINE_TINT, BUTTON_FILL),
    }
}

/// Chrome buttons whose interaction changed this frame.
type ChangedButtons<'w, 's> = Query<
    'w,
    's,
    (
        &'static Interaction,
        &'static mut BorderColor,
        &'static Children,
    ),
    (Changed<Interaction>, With<ChromeButton>),
>;

/// Applies hover (cyan borders, brighter fill) and pressed (darker fill).
pub(crate) fn style_chrome_buttons(
    mut buttons: ChangedButtons,
    mut faces: Query<&mut BackgroundColor, With<ButtonFace>>,
) {
    for (interaction, mut border, children) in &mut buttons {
        let (line, fill) = button_state(*interaction);
        *border = BorderColor::all(line);
        for child in children.iter() {
            if let Ok(mut face) = faces.get_mut(child) {
                face.0 = fill;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_turns_borders_cyan_and_brightens_pressed_darkens() {
        let (rest_line, rest_fill) = button_state(Interaction::None);
        let (hover_line, hover_fill) = button_state(Interaction::Hovered);
        let (_, pressed_fill) = button_state(Interaction::Pressed);
        assert_eq!(rest_line, CARD_LINE_TINT);
        assert_eq!(hover_line, CHROME_CYAN);
        let lum = |c: Color| {
            let l = c.to_linear();
            l.red + l.green + l.blue
        };
        assert!(lum(hover_fill) > lum(rest_fill), "hover brightens");
        assert!(lum(pressed_fill) < lum(rest_fill), "pressed darkens");
    }

    #[test]
    fn veil_alpha_follows_darkness() {
        assert!((veil_color(0.8).alpha() - 0.8).abs() < 1e-6);
        assert!((veil_color(2.0).alpha() - 1.0).abs() < 1e-6);
    }

    /// Alpha of the screen veil.
    fn veil_alpha(app: &mut App) -> f32 {
        app.world_mut()
            .query_filtered::<&BackgroundColor, With<DimVeil>>()
            .single(app.world())
            .expect("one veil")
            .0
            .alpha()
    }

    /// Presses every button carrying `marker` once.
    fn press<C: Component + PartialEq>(app: &mut App, marker: C) {
        let found: Vec<Entity> = app
            .world_mut()
            .query::<(Entity, &C)>()
            .iter(app.world())
            .filter(|(_, held)| **held == marker)
            .map(|(entity, _)| entity)
            .collect();
        assert!(!found.is_empty(), "button exists");
        // A Reset button shows in several sections; one press is enough.
        let entity = found[0];
        *app.world_mut().get_mut::<Interaction>(entity).unwrap() = Interaction::Pressed;
        app.update();
        *app.world_mut().get_mut::<Interaction>(entity).unwrap() = Interaction::None;
        app.update();
    }

    #[test]
    fn background_dimming_drives_the_veil_and_resets_with_display() {
        use crate::settings_ui::{SettingReset, SettingStep, StepField};
        use universe_core::settings::Category;
        let mut app = crate::window_tests::headless_app();
        app.update();
        app.world_mut()
            .resource_mut::<NextState<Screen>>()
            .set(Screen::Settings);
        app.update();
        app.update();
        assert!((veil_alpha(&mut app) - 0.80).abs() < 1e-6, "80% by default");
        let lighter = SettingStep {
            field: StepField::Dimming,
            dir: -1,
        };
        for _ in 0..10 {
            press(&mut app, lighter);
        }
        assert!((veil_alpha(&mut app) - 0.60).abs() < 1e-6, "stops at 60%");
        let darker = SettingStep {
            field: StepField::Dimming,
            dir: 1,
        };
        for _ in 0..10 {
            press(&mut app, darker);
        }
        assert!((veil_alpha(&mut app) - 0.95).abs() < 1e-6, "stops at 95%");
        press(&mut app, SettingReset(Category::Display));
        assert!((veil_alpha(&mut app) - 0.80).abs() < 1e-6, "Display reset");
    }
}
