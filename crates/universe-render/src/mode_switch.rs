//! Mode toggle button and chooser (#955).
//!
//! The top-left corner carries one 48 px chrome button showing the current
//! mode's icon only. Clicking it opens a chooser with one row per mode
//! (icon plus label, current mode highlighted). The Display > Scene > Mode
//! reminder switch hides both. Everything here is `pub(crate)`.

use crate::chrome;
use crate::fonts::{Icon, TextRole};
use crate::input::{Flight, FlightMode};
use crate::tokens::{
    BUTTON_INNER_INSET_PX, BUTTON_RADIUS_PX, CARD_LINE_TINT, LAYER_UNIVERSE, MENU_BUTTON_DIAMETER_PX,
    MENU_BUTTON_GAP_PX, MENU_MARGIN_PX, MENU_SELECT_TINT, MENU_TEXT_TINT,
};
use bevy::prelude::*;
use bevy::ui::{Node, PositionType, Val, widget::Text};

/// Marks the top-left mode button (visible in the universe only).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModeButton;

/// Icon glyph inside the mode button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModeButtonIcon;

/// Marks the hidden mode chooser card under the button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModeChooser;

/// One chooser row, carrying the mode it picks.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModeRow(pub FlightMode);

/// Whether the chooser is open (closed by default).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ChooserOpen(pub bool);

/// Every mode in chooser order.
pub(crate) const MODES: [FlightMode; 3] = [FlightMode::Normal, FlightMode::Free, FlightMode::Dive];

/// Lucide icon for one mode (pure).
#[must_use]
pub(crate) const fn icon_for_mode(mode: FlightMode) -> Icon {
    match mode {
        FlightMode::Normal => Icon::ModeNormal,
        FlightMode::Free => Icon::ModeFree,
        FlightMode::Dive => Icon::ModeDive,
    }
}

/// Chooser label for one mode (pure).
#[must_use]
pub(crate) const fn label_for_mode(mode: FlightMode) -> &'static str {
    match mode {
        FlightMode::Normal => "Normal",
        FlightMode::Free => "Free",
        FlightMode::Dive => "Dive",
    }
}

/// Mode toggle button, chooser, and their sync.
pub(crate) struct ModeSwitchPlugin;

impl Plugin for ModeSwitchPlugin {
    /// Registers the chooser state, the button UI, and its sync.
    fn build(&self, app: &mut App) {
        app.init_resource::<ChooserOpen>()
            .add_systems(Startup, spawn_mode_switch)
            .add_systems(Update, sync_mode_switch);
    }
}

/// Spawns the mode button and the hidden chooser.
pub(crate) fn spawn_mode_switch(mut commands: Commands) {
    commands
        .spawn((
            ModeButton,
            Name::new("mode-button"),
            GlobalZIndex(LAYER_UNIVERSE),
            chrome::button(
                Val::Px(MENU_BUTTON_DIAMETER_PX),
                Val::Px(MENU_BUTTON_DIAMETER_PX),
                false,
            ),
        ))
        .insert(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(MENU_MARGIN_PX),
            top: Val::Px(MENU_MARGIN_PX),
            width: Val::Px(MENU_BUTTON_DIAMETER_PX),
            height: Val::Px(MENU_BUTTON_DIAMETER_PX),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(BUTTON_RADIUS_PX)),
            padding: UiRect::all(Val::Px(BUTTON_INNER_INSET_PX)),
            ..default()
        })
        .with_children(|button| {
            button
                .spawn(chrome::button_face(false))
                .with_children(|face| {
                    let (text, role) = icon_for_mode(FlightMode::Dive).text();
                    face.spawn((ModeButtonIcon, text, role, TextColor(MENU_TEXT_TINT)));
                });
        });
    commands
        .spawn((
            ModeChooser,
            Name::new("mode-chooser"),
            GlobalZIndex(LAYER_UNIVERSE),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(MENU_MARGIN_PX),
                top: Val::Px(MENU_MARGIN_PX + MENU_BUTTON_DIAMETER_PX + MENU_BUTTON_GAP_PX),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(MENU_BUTTON_GAP_PX),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|chooser| {
            for mode in MODES {
                chooser
                    .spawn((
                        ModeRow(mode),
                        Name::new("mode-row"),
                        chrome::button(Val::Px(200.0), Val::Px(56.0), false),
                    ))
                    .with_children(|row| {
                        row.spawn(chrome::button_face(false)).with_children(|face| {
                            let (icon, role) = icon_for_mode(mode).text();
                            face.spawn((icon, role, TextColor(MENU_TEXT_TINT)));
                            face.spawn((
                                Text::new(label_for_mode(mode)),
                                TextRole::Body,
                                TextColor(MENU_TEXT_TINT),
                            ));
                        });
                    });
            }
        });
}

/// Refreshes the button and chooser: hidden when the Mode reminder switch
/// is off, the chooser shown only while open.
///
/// Writes only on a change; missing nodes are a no-op.
pub(crate) fn sync_mode_switch(
    store: Res<crate::SettingsStore>,
    flight: Res<Flight>,
    open: Res<ChooserOpen>,
    mut button: Query<&mut Visibility, (With<ModeButton>, Without<ModeChooser>)>,
    mut icon: Query<&mut Text, With<ModeButtonIcon>>,
    mut chooser: Query<&mut Visibility, (With<ModeChooser>, Without<ModeButton>)>,
    mut rows: Query<(&ModeRow, &mut BorderColor)>,
) {
    let Ok(mut button_visibility) = button.single_mut() else {
        return;
    };
    let want_button = if store.0.mode_reminder {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *button_visibility != want_button {
        *button_visibility = want_button;
    }
    if let Ok(mut text) = icon.single_mut() {
        let want = icon_for_mode(flight.mode).glyph().to_string();
        if text.0 != want {
            text.0 = want;
        }
    }
    if let Ok(mut chooser_visibility) = chooser.single_mut() {
        let want_chooser = if store.0.mode_reminder && open.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *chooser_visibility != want_chooser {
            *chooser_visibility = want_chooser;
        }
    }
    for (row, mut border) in &mut rows {
        let want = BorderColor::all(if row.0 == flight.mode {
            MENU_SELECT_TINT
        } else {
            CARD_LINE_TINT
        });
        if *border != want {
            *border = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_tests::headless_app;

    #[test]
    fn mode_button_and_chooser_spawn_with_the_off_state() {
        let mut app = headless_app();
        app.update();
        let button = app
            .world_mut()
            .query_filtered::<&Visibility, With<ModeButton>>()
            .single(app.world())
            .copied()
            .expect("one mode button");
        assert_eq!(button, Visibility::Inherited, "button shows by default");
        let rows: Vec<FlightMode> = app
            .world_mut()
            .query::<&ModeRow>()
            .iter(app.world())
            .map(|row| row.0)
            .collect();
        assert_eq!(rows, MODES, "one row per mode in order");
        let chooser = app
            .world_mut()
            .query_filtered::<&Visibility, With<ModeChooser>>()
            .single(app.world())
            .copied()
            .expect("one chooser");
        assert_eq!(chooser, Visibility::Hidden, "chooser starts closed");
        app.world_mut()
            .resource_mut::<crate::SettingsStore>()
            .0
            .mode_reminder = false;
        app.update();
        let button = app
            .world_mut()
            .query_filtered::<&Visibility, With<ModeButton>>()
            .single(app.world())
            .copied()
            .expect("one mode button");
        assert_eq!(button, Visibility::Hidden, "the switch hides the button");
    }
}
