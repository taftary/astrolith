//! The editable Keys page, Settings > Controls > Keys (#560 T3).
//!
//! Every action is one row: its name and a button showing its key. Clicking
//! the button waits for the next key or mouse press and binds it; a key
//! another action holds swaps the two (owner answer Q2 A). Esc cancels the
//! wait. Reset keys restores the defaults. Everything here is `pub(crate)`.

use crate::bindings::{binding_of_button, binding_of_key};
use crate::chrome;
use crate::fonts::TextRole;
use crate::menu::Screen;
use crate::settings_rows::spawn_row;
use crate::settings_ui::SettingsStore;
use crate::tokens::{
    KEY_LABEL_WIDTH_PX, MENU_DIM_TINT, MENU_TEXT_TINT, RESET_BUTTON_WIDTH_PX,
    SCREEN_BUTTON_HEIGHT_PX,
};
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use universe_core::keymap::{Action, Keymap};

/// Text a key button shows while it waits for a key.
pub(crate) const WAITING_TEXT: &str = "Press a key...";

/// The action waiting for its new key, if any.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyCapture {
    /// The action the next key or mouse press binds.
    pub waiting: Option<Action>,
    /// A press was just captured this frame (it is not also a click).
    pub just_bound: bool,
}

/// Marks the key button of one action.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyButton(pub Action);

/// Marks the key text inside one action's button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyText(pub Action);

/// Marks the Reset keys button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeysReset;

/// The Keys page systems.
pub(crate) struct KeysUiPlugin;

impl Plugin for KeysUiPlugin {
    /// Registers the capture before the menu's Esc rule, so the key that
    /// ends a wait (Esc included) does nothing else.
    fn build(&self, app: &mut App) {
        app.init_resource::<KeyCapture>().add_systems(
            Update,
            (
                capture_key,
                key_button_click,
                keys_reset_click,
                sync_key_text,
            )
                .chain()
                .before(crate::menu::esc_rule),
        );
    }
}

/// The fixed keys the page cannot change, shown above the list.
pub(crate) const FIXED_KEYS_NOTE: &str = "Hold Run (Shift by default) and press 1-9 to set the flight speed. The mouse wheel also dives in and rises out.";

/// Spawns the note, one row per action, then the Reset keys button.
pub(crate) fn spawn_keys_page(rows: &mut ChildSpawnerCommands) {
    rows.spawn((
        Text::new(FIXED_KEYS_NOTE),
        TextRole::Note,
        TextColor(MENU_DIM_TINT),
    ));
    for action in Action::ALL {
        spawn_row(rows, |row| {
            row.spawn((
                Text::new(action.label()),
                TextRole::Body,
                TextColor(MENU_TEXT_TINT),
                Node {
                    width: Val::Px(KEY_LABEL_WIDTH_PX),
                    ..default()
                },
            ));
            row.spawn((
                KeyButton(action),
                Name::new(format!("key-{}", action.id())),
                chrome::button(
                    Val::Px(RESET_BUTTON_WIDTH_PX),
                    Val::Px(SCREEN_BUTTON_HEIGHT_PX),
                    false,
                ),
            ))
            .with_children(|button| {
                button
                    .spawn(chrome::button_face(false))
                    .with_children(|face| {
                        face.spawn((
                            KeyText(action),
                            Text::new(""),
                            TextRole::Body,
                            TextColor(MENU_TEXT_TINT),
                        ));
                    });
            });
        });
    }
    rows.spawn((
        KeysReset,
        Name::new("reset-keys"),
        chrome::button(
            Val::Px(RESET_BUTTON_WIDTH_PX),
            Val::Px(SCREEN_BUTTON_HEIGHT_PX),
            false,
        ),
    ))
    .with_children(|button| {
        button
            .spawn(chrome::button_face(false))
            .with_children(|face| {
                face.spawn((
                    Text::new("Reset keys"),
                    TextRole::Body,
                    TextColor(MENU_TEXT_TINT),
                ));
            });
    });
}

/// Binds the next key or mouse press to the waiting action (Esc cancels),
/// and consumes that press so nothing else acts on it this frame.
pub(crate) fn capture_key(
    mut capture: ResMut<KeyCapture>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut buttons: ResMut<ButtonInput<MouseButton>>,
    mut store: ResMut<SettingsStore>,
    screen: Res<State<Screen>>,
) {
    if capture.just_bound {
        capture.just_bound = false;
    }
    let Some(action) = capture.waiting else {
        return;
    };
    if *screen.get() != Screen::Settings {
        capture.waiting = None;
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        keys.clear_just_pressed(KeyCode::Escape);
        capture.waiting = None;
        capture.just_bound = true;
        return;
    }
    let key = keys
        .get_just_pressed()
        .copied()
        .find(|key| binding_of_key(*key).is_some());
    if let Some(key) = key {
        keys.clear_just_pressed(key);
        if let Some(binding) = binding_of_key(key) {
            store.0.keymap.bind(action, binding);
        }
    } else {
        let button = buttons
            .get_just_pressed()
            .copied()
            .find(|button| binding_of_button(*button).is_some());
        let Some(button) = button else {
            return;
        };
        buttons.clear_just_pressed(button);
        if let Some(binding) = binding_of_button(button) {
            store.0.keymap.bind(action, binding);
        }
    }
    capture.waiting = None;
    capture.just_bound = true;
}

/// Clicking an action's key button starts waiting for its new key.
pub(crate) fn key_button_click(
    mut capture: ResMut<KeyCapture>,
    buttons: Query<(&Interaction, &KeyButton), Changed<Interaction>>,
) {
    for (interaction, button) in &buttons {
        // The click that ended a wait does not start another.
        if *interaction == Interaction::Pressed && !capture.just_bound {
            capture.waiting = Some(button.0);
        }
    }
}

/// Reset keys restores the default key map.
pub(crate) fn keys_reset_click(
    mut store: ResMut<SettingsStore>,
    mut capture: ResMut<KeyCapture>,
    buttons: Query<&Interaction, (Changed<Interaction>, With<KeysReset>)>,
) {
    if buttons.iter().any(|i| *i == Interaction::Pressed) {
        store.0.keymap = Keymap::default();
        capture.waiting = None;
    }
}

/// Rewrites every key text from the store and the wait.
pub(crate) fn sync_key_text(
    store: Res<SettingsStore>,
    capture: Res<KeyCapture>,
    added: Query<(), Added<KeyText>>,
    mut texts: Query<(&KeyText, &mut Text)>,
) {
    if !store.is_changed() && !capture.is_changed() && added.is_empty() {
        return;
    }
    for (key, mut text) in &mut texts {
        let want = if capture.waiting == Some(key.0) {
            WAITING_TEXT.to_string()
        } else {
            store.0.keymap.binding(key.0).label()
        };
        if text.0 != want {
            text.0 = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_tests::headless_app;
    use universe_core::keymap::Binding;

    fn settings_app() -> App {
        let mut app = headless_app();
        app.update();
        app.world_mut()
            .resource_mut::<NextState<Screen>>()
            .set(Screen::Settings);
        app.update();
        app.update();
        app
    }

    fn click(app: &mut App, action: Action) {
        let entity = app
            .world_mut()
            .query::<(Entity, &KeyButton)>()
            .iter(app.world())
            .find(|(_, button)| button.0 == action)
            .map(|(entity, _)| entity)
            .expect("one key button per action");
        app.world_mut()
            .entity_mut(entity)
            .insert(Interaction::Pressed);
        app.update();
        app.world_mut().entity_mut(entity).insert(Interaction::None);
        app.update();
    }

    fn tap(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release(key);
        input.clear();
        app.update();
    }

    fn text_of(app: &mut App, action: Action) -> String {
        app.world_mut()
            .query::<(&KeyText, &Text)>()
            .iter(app.world())
            .find(|(key, _)| key.0 == action)
            .map(|(_, text)| text.0.clone())
            .expect("one key text per action")
    }

    fn bound(app: &App, action: Action) -> &'static str {
        app.world()
            .resource::<SettingsStore>()
            .0
            .keymap
            .binding(action)
            .name()
    }

    fn screen(app: &App) -> Screen {
        *app.world().resource::<State<Screen>>().get()
    }

    #[test]
    fn every_action_has_a_row_showing_its_key() {
        let mut app = settings_app();
        assert_eq!(text_of(&mut app, Action::Forward), "W");
        assert_eq!(text_of(&mut app, Action::DiveIn), "Up arrow");
        assert_eq!(text_of(&mut app, Action::Pick), "Left mouse button");
        let rows = app
            .world_mut()
            .query::<&KeyButton>()
            .iter(app.world())
            .count();
        assert_eq!(rows, universe_core::keymap::ACTIONS);
    }

    #[test]
    fn clicking_an_action_captures_the_next_key_and_a_taken_key_swaps() {
        let mut app = settings_app();
        click(&mut app, Action::Forward);
        assert_eq!(text_of(&mut app, Action::Forward), WAITING_TEXT);
        tap(&mut app, KeyCode::KeyI);
        assert_eq!(bound(&app, Action::Forward), "KeyI");
        assert_eq!(text_of(&mut app, Action::Forward), "I");
        // S belongs to Back: binding it to Forward gives Back the old I.
        click(&mut app, Action::Forward);
        tap(&mut app, KeyCode::KeyS);
        assert_eq!(bound(&app, Action::Forward), "KeyS");
        assert_eq!(bound(&app, Action::Back), "KeyI");
        assert_eq!(screen(&app), Screen::Settings);
    }

    #[test]
    fn a_mouse_press_binds_and_esc_cancels_without_leaving() {
        let mut app = settings_app();
        click(&mut app, Action::Camera);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Middle);
        app.update();
        {
            let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            buttons.release(MouseButton::Middle);
            buttons.clear();
        }
        app.update();
        assert_eq!(bound(&app, Action::Camera), "MouseMiddle");
        click(&mut app, Action::Run);
        tap(&mut app, KeyCode::Escape);
        assert_eq!(bound(&app, Action::Run), "Shift", "Esc keeps the key");
        assert_eq!(text_of(&mut app, Action::Run), "Shift");
        assert_eq!(screen(&app), Screen::Settings, "Esc only cancels the wait");
    }

    #[test]
    fn reset_keys_restores_the_defaults() {
        let mut app = settings_app();
        app.world_mut()
            .resource_mut::<SettingsStore>()
            .0
            .keymap
            .bind(Action::Menu, Binding::named("KeyM").expect("bindable"));
        let entity = app
            .world_mut()
            .query_filtered::<Entity, With<KeysReset>>()
            .single(app.world())
            .expect("one Reset keys button");
        app.world_mut()
            .entity_mut(entity)
            .insert(Interaction::Pressed);
        app.update();
        assert_eq!(
            app.world().resource::<SettingsStore>().0.keymap,
            Keymap::default()
        );
        assert_eq!(text_of(&mut app, Action::Menu), "Esc");
    }
}
