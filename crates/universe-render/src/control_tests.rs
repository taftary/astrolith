//! Headless tests for the mode keys, the toggle controls, and the mode
//! reminder (#557), split from `lib.rs` under `E-FILE-SIZE`.

use crate::SettingsStore;
use crate::Universe;
use crate::input::Latches;
use crate::window_tests::headless_app;
use bevy::prelude::*;

/// Presses `key` for one update.
fn tap(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    input.release(key);
    input.clear();
}

#[test]
fn mode_reminder_follows_the_mode_and_the_setting() {
    use crate::hud::ModeReminderText;
    let mut app = headless_app();
    app.update();
    let read = |app: &mut App| {
        app.world_mut()
            .query_filtered::<(&Text, &Visibility), With<ModeReminderText>>()
            .single(app.world())
            .map(|(text, visibility)| (text.0.clone(), *visibility))
            .expect("one reminder line")
    };
    let (text, visibility) = read(&mut app);
    assert!(text.starts_with("Dive"), "{text}");
    assert_eq!(visibility, Visibility::Inherited);
    tap(&mut app, KeyCode::Digit2);
    app.update();
    assert!(read(&mut app).0.starts_with("Free"));
    app.world_mut()
        .resource_mut::<SettingsStore>()
        .0
        .mode_reminder = false;
    app.update();
    assert_eq!(read(&mut app).1, Visibility::Hidden, "off hides the line");
}

#[test]
fn keys_two_and_three_switch_free_and_dive() {
    use crate::input::{Flight, FlightMode};
    let mut app = headless_app();
    app.update();
    tap(&mut app, KeyCode::Digit2);
    assert_eq!(app.world().resource::<Flight>().mode, FlightMode::Free);
    tap(&mut app, KeyCode::Digit2);
    assert_eq!(
        app.world().resource::<Flight>().mode,
        FlightMode::Free,
        "2 again stays free"
    );
    tap(&mut app, KeyCode::Digit3);
    assert_eq!(app.world().resource::<Flight>().mode, FlightMode::Dive);
    tap(&mut app, KeyCode::KeyF);
    assert_eq!(
        app.world().resource::<Flight>().mode,
        FlightMode::Free,
        "F still switches"
    );
    tap(&mut app, KeyCode::Digit1);
    assert_eq!(
        app.world().resource::<Flight>().mode,
        FlightMode::Free,
        "1 changes no mode yet"
    );
}

#[test]
fn dive_toggle_keeps_diving_after_one_press() {
    let mut app = headless_app();
    app.update();
    app.world_mut()
        .resource_mut::<SettingsStore>()
        .0
        .dive_toggle = true;
    let before = app.world().resource::<Universe>().path.offset();
    tap(&mut app, KeyCode::ArrowUp);
    for _ in 0..10 {
        app.update();
    }
    let after = app.world().resource::<Universe>().path.offset();
    assert_ne!(after, before, "one press keeps the dive going");
    assert_eq!(app.world().resource::<Latches>().dive, 1);
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(
        app.world().resource::<Latches>().dive,
        0,
        "a second press stops it"
    );
    let stopped = app.world().resource::<Universe>().path.offset();
    app.update();
    assert_eq!(app.world().resource::<Universe>().path.offset(), stopped);
}
