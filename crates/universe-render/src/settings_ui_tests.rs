//! Settings UI widget tests (split from `settings_ui.rs` under `E-FILE-SIZE`, #884).
//!
//! Unit tests for the Settings content live here so `settings_ui.rs` stays
//! under the file-size guard. Pure widget behaviour through the headless app.

use super::settings_ui::{
    PendingWindow, SettingReset, SettingSection, SettingStep, SettingToggle, SettingsStore,
    StepField, ToggleField,
};
use crate::camera::ExposureLevel;
use crate::input::{Flight, Navigation, SavedSlots};
use crate::menu::Screen;
use bevy::asset::AssetPlugin;
use bevy::gizmos::{
    GizmoAsset, GizmoHandles,
    config::{DefaultGizmoConfigGroup, GizmoConfigStore},
};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::mesh::Mesh;
use bevy::pbr::StandardMaterial;
use bevy::prelude::*;
use bevy::ui::widget::Text;
use bevy::window::WindowMode;
use universe_core::settings::Category;

/// Headless app with the full dive pipeline (store included).
fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.add_message::<AppExit>();
    app.add_message::<MouseWheel>();
    app.add_message::<MouseMotion>();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.init_resource::<GizmoConfigStore>();
    app.init_gizmo_group::<DefaultGizmoConfigGroup>();
    app.init_asset::<GizmoAsset>();
    app.init_asset::<Mesh>();
    app.init_asset::<StandardMaterial>();
    app.init_resource::<GizmoHandles>();
    app.add_plugins(crate::DivePlugin);
    app
}

/// Taps `key` for exactly one update (plus the state-transition frame).
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

/// Holds `held` while tapping `tap_key` for one update.
fn chord(app: &mut App, held: KeyCode, tap_key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(held);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(tap_key);
    app.update();
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    input.release(tap_key);
    input.release(held);
    input.clear();
    app.update();
}

#[test]
fn only_the_selected_section_keeps_layout_space() {
    use crate::panel::PanelState;
    let mut app = headless_app();
    app.update();
    app.world_mut()
        .resource_mut::<NextState<Screen>>()
        .set(Screen::Settings);
    app.update();
    app.update();
    for leaf in [
        "scene",
        "window",
        "flight",
        "keys",
        "views",
        "accessibility",
        "developer",
    ] {
        app.world_mut().resource_mut::<PanelState>().select(leaf);
        app.update();
        let mut shown = 0;
        for (section, node) in app
            .world_mut()
            .query::<(&SettingSection, &Node)>()
            .iter(app.world())
        {
            if section.leaves.contains(&leaf) {
                assert_eq!(node.display, Display::Flex, "{leaf} shows its section");
                shown += 1;
            } else {
                assert_eq!(node.display, Display::None, "{leaf} collapses the rest");
            }
        }
        assert_eq!(shown, 1, "{leaf} shows exactly one section");
    }
}

#[test]
fn store_defaults_match_the_spec_table() {
    let store = SettingsStore::default();
    assert_eq!(store.0.exposure.get(), 1.0);
    assert!(store.0.labels);
    assert_eq!(store.0.label_limit.get(), 24);
    assert!(!store.0.fullscreen);
    assert!(store.0.vsync);
    assert_eq!(store.0.speed.get(), 5);
    assert!(store.0.views.iter().all(Option::is_none));
    assert!(!store.0.debug_axes);
}

#[test]
fn keys_write_through_the_store_and_stay_in_sync() {
    let mut app = headless_app();
    app.update();
    // `B` dims the store exposure (#884 T3), and the camera level follows.
    tap(&mut app, KeyCode::KeyB);
    assert!(
        app.world().resource::<SettingsStore>().0.exposure.get() < 1.0,
        "B dims the store exposure"
    );
    assert_eq!(
        app.world().resource::<ExposureLevel>().0,
        app.world().resource::<SettingsStore>().0.exposure.get(),
        "the camera level follows the store"
    );
    // `Shift+7` steps the store speed, and flight follows.
    chord(&mut app, KeyCode::ShiftLeft, KeyCode::Digit7);
    assert_eq!(app.world().resource::<SettingsStore>().0.speed.get(), 7);
    assert_eq!(app.world().resource::<Flight>().step, 7);
    // `X` flips the store axes, and navigation follows.
    tap(&mut app, KeyCode::KeyX);
    assert!(app.world().resource::<SettingsStore>().0.debug_axes);
    assert!(app.world().resource::<Navigation>().show_axes);
    // `Ctrl+3` saves slot 3 in the store and the live slots.
    chord(&mut app, KeyCode::ControlLeft, KeyCode::Digit3);
    assert!(
        app.world().resource::<SettingsStore>().0.views[2].is_some(),
        "slot 3 saved in the store"
    );
    assert!(
        app.world().resource::<SavedSlots>().slots[2].is_some(),
        "slot 3 saved live"
    );
}

#[test]
fn views_clear_and_reset_touch_only_their_slots() {
    let mut app = headless_app();
    app.update();
    chord(&mut app, KeyCode::ControlLeft, KeyCode::Digit2);
    assert!(app.world().resource::<SettingsStore>().0.views[1].is_some());
    // Clear slot 2 through the store (widget path below covers clicks).
    app.world_mut().resource_mut::<SettingsStore>().0.views[1] = None;
    app.update();
    assert!(app.world().resource::<SettingsStore>().0.views[1].is_none());
    assert!(
        app.world().resource::<SavedSlots>().slots[1].is_none(),
        "cleared slots recall to nothing"
    );
    // Resetting Display keeps the speed the keys set.
    chord(&mut app, KeyCode::ShiftLeft, KeyCode::Digit9);
    app.world_mut().resource_mut::<SettingsStore>().0 = app
        .world()
        .resource::<SettingsStore>()
        .0
        .reset(Category::Display);
    app.update();
    let store = app.world().resource::<SettingsStore>().0;
    assert_eq!(store.exposure.get(), 1.0);
    assert_eq!(store.speed.get(), 9, "other categories are untouched");
}

#[test]
fn fullscreen_request_pends_until_the_window_reports_back() {
    let mut app = headless_app();
    app.world_mut().spawn((Window::default(),));
    app.update();
    let count = app.world_mut().query::<&Window>().iter(app.world()).count();
    assert_eq!(count, 1, "exactly one window for the Single param");
    assert!(!app.world().resource::<PendingWindow>().fullscreen);
    app.world_mut().resource_mut::<SettingsStore>().0.fullscreen = true;
    app.update();
    assert!(
        app.world().resource::<PendingWindow>().fullscreen,
        "the switch pends while the window changes"
    );
    app.update();
    assert!(
        !app.world().resource::<PendingWindow>().fullscreen,
        "the loader clears once the window reports back"
    );
    assert!(
        app.world_mut()
            .query::<&Window>()
            .iter(app.world())
            .all(|window| !matches!(window.mode, WindowMode::Windowed)),
        "the window went fullscreen"
    );
}

#[test]
fn accessibility_shows_coming_soon() {
    let mut app = headless_app();
    app.update();
    let texts: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    assert!(
        texts.iter().any(|text| text == "Coming soon"),
        "accessibility shows its note"
    );
    assert!(
        texts
            .iter()
            .any(|text| text == crate::keys_ui::FIXED_KEYS_NOTE),
        "the Keys page names the fixed keys"
    );
}

/// Presses one widget button for one update.
fn press_widget<T: Component>(app: &mut App, matches: impl Fn(&T) -> bool) {
    let entity = app
        .world_mut()
        .query::<(Entity, &T)>()
        .iter(app.world())
        .find(|(_, widget)| matches(widget))
        .map(|(entity, _)| entity)
        .expect("widget exists");
    app.world_mut()
        .entity_mut(entity)
        .insert(Interaction::Pressed);
    app.update();
    app.world_mut().entity_mut(entity).insert(Interaction::None);
    app.update();
}

#[test]
fn widgets_write_through_the_store() {
    let mut app = headless_app();
    app.update();
    // Speed `+` steps the store speed, and flight follows.
    press_widget(&mut app, |step: &SettingStep| {
        step.field == StepField::Speed && step.dir > 0
    });
    assert_eq!(app.world().resource::<SettingsStore>().0.speed.get(), 6);
    assert_eq!(app.world().resource::<Flight>().step, 6);
    // Labels toggle flips the store.
    press_widget(&mut app, |toggle: &SettingToggle| {
        toggle.0 == ToggleField::Labels
    });
    assert!(!app.world().resource::<SettingsStore>().0.labels);
    // #770: Ground guides removed.
    // The new controls rows write through the store too (#557).
    press_widget(&mut app, |step: &SettingStep| {
        step.field == StepField::LookSensitivity && step.dir > 0
    });
    assert_eq!(
        app.world()
            .resource::<SettingsStore>()
            .0
            .look_sensitivity
            .get(),
        6
    );
    for field in [
        ToggleField::InvertLook,
        ToggleField::DiveToggle,
        ToggleField::LookToggle,
        ToggleField::ModeReminder,
    ] {
        press_widget(&mut app, |toggle: &SettingToggle| toggle.0 == field);
    }
    let settings = app.world().resource::<SettingsStore>().0;
    assert!(settings.invert_look && settings.dive_toggle && settings.look_toggle);
    assert!(!settings.mode_reminder, "the reminder turns off");
    // Views reset clears every slot.
    chord(&mut app, KeyCode::ControlLeft, KeyCode::Digit2);
    assert!(
        app.world().resource::<SettingsStore>().0.views[1].is_some(),
        "slot 2 saved first"
    );
    press_widget(&mut app, |reset: &SettingReset| reset.0 == Category::Views);
    assert!(
        app.world()
            .resource::<SettingsStore>()
            .0
            .views
            .iter()
            .all(Option::is_none),
        "reset clears all slots"
    );
    assert!(
        app.world()
            .resource::<SavedSlots>()
            .slots
            .iter()
            .all(Option::is_none),
        "cleared slots recall to nothing"
    );
}

#[test]
fn smoothing_and_inertia_widgets_write_through_the_store() {
    // #756 C6 (S1, S2): the Flight page steps camera smoothing and
    // toggles movement inertia through the store.
    use universe_core::settings::CameraSmoothing;
    let mut app = headless_app();
    app.update();
    assert_eq!(
        app.world().resource::<SettingsStore>().0.camera_smoothing,
        CameraSmoothing::Low
    );
    assert!(app.world().resource::<SettingsStore>().0.movement_inertia);
    // Both rows exist with their labels.
    let texts: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    for label in [
        "Camera smoothing",
        "Movement inertia",
        "Free-mode roll speed",
    ] {
        assert!(texts.iter().any(|text| text == label), "{label} row exists");
    }
    press_widget(&mut app, |step: &SettingStep| {
        step.field == StepField::CameraSmoothing && step.dir > 0
    });
    press_widget(&mut app, |step: &SettingStep| {
        step.field == StepField::CameraSmoothing && step.dir > 0
    });
    assert_eq!(
        app.world().resource::<SettingsStore>().0.camera_smoothing,
        CameraSmoothing::High
    );
    press_widget(&mut app, |step: &SettingStep| {
        step.field == StepField::CameraSmoothing && step.dir < 0
    });
    assert_eq!(
        app.world().resource::<SettingsStore>().0.camera_smoothing,
        CameraSmoothing::Medium
    );
    press_widget(&mut app, |toggle: &SettingToggle| {
        toggle.0 == ToggleField::MovementInertia
    });
    assert!(!app.world().resource::<SettingsStore>().0.movement_inertia);
    press_widget(&mut app, |toggle: &SettingToggle| {
        toggle.0 == ToggleField::MovementInertia
    });
    assert!(app.world().resource::<SettingsStore>().0.movement_inertia);
    // #884 T5 S1: the Flight page steps the free-mode roll speed through
    // the store, and Controls reset restores 1.0.
    press_widget(&mut app, |step: &SettingStep| {
        step.field == StepField::FreeRollSpeed && step.dir > 0
    });
    assert_eq!(
        app.world()
            .resource::<SettingsStore>()
            .0
            .free_roll_speed
            .get(),
        1.1
    );
    app.world_mut().resource_mut::<SettingsStore>().0 = app
        .world()
        .resource::<SettingsStore>()
        .0
        .reset(Category::Controls);
    app.update();
    let store = app.world().resource::<SettingsStore>().0;
    assert_eq!(store.camera_smoothing, CameraSmoothing::Low);
    assert!(store.movement_inertia);
    assert_eq!(store.free_roll_speed.get(), 1.0);
}

#[test]
fn behind_follow_widget_writes_through_the_store() {
    // #899 T4 S1: the Flight page shows the Behind camera follow row and
    // steps it through the store; Controls reset restores Smooth.
    use universe_core::settings::BehindFollow;
    let mut app = headless_app();
    app.update();
    assert_eq!(
        app.world().resource::<SettingsStore>().0.behind_follow,
        BehindFollow::Smooth
    );
    let texts: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    assert!(
        texts.iter().any(|text| text == "Behind camera follow"),
        "Behind camera follow row exists"
    );
    press_widget(&mut app, |step: &SettingStep| {
        step.field == StepField::BehindFollow && step.dir < 0
    });
    assert_eq!(
        app.world().resource::<SettingsStore>().0.behind_follow,
        BehindFollow::Quick
    );
    press_widget(&mut app, |step: &SettingStep| {
        step.field == StepField::BehindFollow && step.dir < 0
    });
    assert_eq!(
        app.world().resource::<SettingsStore>().0.behind_follow,
        BehindFollow::Off
    );
    press_widget(&mut app, |step: &SettingStep| {
        step.field == StepField::BehindFollow && step.dir > 0
    });
    assert_eq!(
        app.world().resource::<SettingsStore>().0.behind_follow,
        BehindFollow::Quick
    );
}
