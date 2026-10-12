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
        FlightMode::Normal,
        "1 enters normal mode (#558)"
    );
    tap(&mut app, KeyCode::KeyF);
    assert_eq!(
        app.world().resource::<Flight>().mode,
        FlightMode::Free,
        "F leaves normal mode for free mode"
    );
}

/// Headless app opened down the fixed journey to `depth` markers.
fn opened(depth: usize) -> App {
    use universe_core::nav::{DEMO_SEED, MarkerIndex};
    use universe_core::nest::autopilot_path;
    let mut app = headless_app();
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        for &marker in autopilot_path(DEMO_SEED)
            .get(..depth)
            .expect("journey reaches its depth")
        {
            assert!(universe.open(MarkerIndex(marker)), "journey marker opens");
        }
        universe.path.set_offset([0.05, 0.3, 0.05]);
    }
    app.update();
    app
}

/// Holds `key` for `frames` updates.
fn hold(app: &mut App, key: KeyCode, frames: usize) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    for _ in 0..frames {
        app.update();
    }
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    input.release(key);
    input.clear();
}

#[test]
fn normal_mode_stands_walks_and_switches_camera() {
    use crate::normal_mode::NormalState;
    use universe_core::body::{body_height, level_ground};
    let mut app = opened(10);
    tap(&mut app, KeyCode::Digit1);
    app.update();
    let state = *app.world().resource::<NormalState>();
    assert!(state.active, "key 1 places the body");
    assert!(state.third_person, "third person by default (S2)");
    let (feet, height) = {
        let universe = app.world().resource::<Universe>();
        let ground = level_ground(universe, state.body.position).expect("region ground");
        (
            state.body.position,
            (state.body.position[1] - ground.point[1]).abs(),
        )
    };
    assert!(height < 1e-9, "the body stands on the ground: {height}");
    hold(&mut app, KeyCode::KeyW, 30);
    let walked = app.world().resource::<NormalState>().body.position;
    assert_ne!(walked, feet, "W walks");
    let level = app.world().resource::<Universe>().level();
    let size = body_height(level);
    let eye = app.world().resource::<Universe>().path.offset();
    let universe = app.world().resource::<Universe>();
    let under = level_ground(universe, eye).expect("ground under the eye");
    let above = eye[1] - under.point[1];
    assert!(
        above > 0.5 * size && above < 1.5 * size,
        "the path follows the eye: {above}"
    );
    tap(&mut app, KeyCode::KeyV);
    assert!(
        !app.world().resource::<NormalState>().third_person,
        "V switches to first person"
    );
    tap(&mut app, KeyCode::Digit3);
    assert!(
        !app.world().resource::<NormalState>().active,
        "leaving parks the body"
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
