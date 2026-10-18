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
    crate::player::player_here(&mut app);
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
    use crate::player_state::PlayerState;
    use universe_core::body::{body_height, level_ground};
    let mut app = opened(9);
    crate::player::player_here(&mut app);
    tap(&mut app, KeyCode::Digit1);
    app.update();
    let state = app.world().resource::<PlayerState>().clone();
    assert!(state.active, "key 1 places the body");
    assert!(state.third_person, "third person by default (S2)");
    // #770: no ground; the body floats where placed.
    let (feet, height) = {
        let universe = app.world().resource::<Universe>();
        assert!(
            level_ground(universe, state.body.position).is_none(),
            "no ground until the rebuild"
        );
        (state.body.position, 0.0)
    };
    assert!(height < 1e-9, "floating, no ground: {height}");
    hold(&mut app, KeyCode::KeyW, 30);
    let walked = app.world().resource::<PlayerState>().body.position;
    assert_ne!(walked, feet, "W walks");
    let level = app.world().resource::<Universe>().level();
    let size = body_height(level);
    // #770: no ground; the path is the eye.
    let _size = size;
    let _eye = app.world().resource::<Universe>().path.offset();
    tap(&mut app, KeyCode::KeyV);
    assert!(
        !app.world().resource::<PlayerState>().third_person,
        "V switches to first person"
    );
    tap(&mut app, KeyCode::Digit3);
    assert!(
        !app.world().resource::<PlayerState>().active,
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

#[test]
fn starting_camera_setting_picks_the_first_camera() {
    use crate::player_state::PlayerState;
    let mut app = opened(9);
    app.world_mut()
        .resource_mut::<SettingsStore>()
        .0
        .start_third_person = false;
    crate::player::player_here(&mut app);
    tap(&mut app, KeyCode::Digit1);
    app.update();
    assert!(
        !app.world().resource::<PlayerState>().third_person,
        "Starting camera set to first person starts in first person (AC10)"
    );
}

#[test]
fn the_readout_names_the_pull_on_the_planet_and_show_pull_hides_it() {
    use crate::hud::HudText;
    let mut app = opened(9);
    crate::player::player_here(&mut app);
    tap(&mut app, KeyCode::Digit1);
    app.update();
    app.update();
    let read = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&Text, With<HudText>>()
            .single(app.world())
            .map(|text| text.0.clone())
            .expect("one readout")
    };
    let text = read(&mut app);
    // The body starts a little above the ground and falls: about 9.7-9.8.
    assert!(text.contains("pull: Earth 9."), "{text}");
    app.world_mut().resource_mut::<SettingsStore>().0.show_pull = false;
    app.update();
    assert!(
        !read(&mut app).contains("pull:"),
        "Show pull off hides the line"
    );
    tap(&mut app, KeyCode::Digit2);
    app.world_mut().resource_mut::<SettingsStore>().0.show_pull = true;
    app.update();
    assert!(!read(&mut app).contains("pull:"), "free mode feels no pull");
}

#[test]
fn rebound_keys_work_in_the_game_and_the_old_ones_do_not() {
    use crate::input::{Flight, FlightMode};
    use crate::menu::Screen;
    use universe_core::keymap::{Action, Binding};
    let mut app = headless_app();
    app.update();
    {
        let mut store = app.world_mut().resource_mut::<SettingsStore>();
        let map = &mut store.0.keymap;
        map.bind(Action::FreeMode, Binding::named("KeyN").expect("bindable"));
        map.bind(Action::Menu, Binding::named("KeyM").expect("bindable"));
    }
    tap(&mut app, KeyCode::Digit2);
    assert_eq!(
        app.world().resource::<Flight>().mode,
        FlightMode::Dive,
        "2 no longer enters free mode"
    );
    tap(&mut app, KeyCode::KeyN);
    assert_eq!(app.world().resource::<Flight>().mode, FlightMode::Free);
    tap(&mut app, KeyCode::Escape);
    app.update();
    assert_eq!(
        *app.world().resource::<State<Screen>>().get(),
        Screen::Universe,
        "Esc no longer opens the menu"
    );
    tap(&mut app, KeyCode::KeyM);
    app.update();
    assert_eq!(*app.world().resource::<State<Screen>>().get(), Screen::Menu);
}

/// Sends one touch event (#561).
fn finger(app: &mut App, phase: bevy::input::touch::TouchPhase, id: u64, at: [f32; 2]) {
    app.world_mut().write_message(TouchInput {
        phase,
        position: Vec2::new(at[0], at[1]),
        window: Entity::PLACEHOLDER,
        force: None,
        id,
    });
}

/// The touch layout of the headless app (no window: 1280 x 720, right hand).
fn touch_layout() -> universe_core::touch::Layout {
    universe_core::touch::Layout::new(1280.0, 720.0, false)
}

#[test]
fn touch_walks_jumps_and_switches_camera_in_normal_mode() {
    use crate::player_state::PlayerState;
    use bevy::input::touch::TouchPhase;
    use universe_core::touch::{STICK_RADIUS_PX, TouchButton};
    let mut app = opened(9);
    crate::player::player_here(&mut app);
    tap(&mut app, KeyCode::Digit1);
    app.update();
    let start = app.world().resource::<PlayerState>().body.position;
    let layout = touch_layout();
    finger(&mut app, TouchPhase::Started, 1, layout.stick);
    app.update();
    let push = [layout.stick[0], layout.stick[1] - 0.6 * STICK_RADIUS_PX];
    finger(&mut app, TouchPhase::Moved, 1, push);
    for _ in 0..30 {
        app.update();
    }
    finger(&mut app, TouchPhase::Ended, 1, push);
    app.update();
    let walked = app.world().resource::<PlayerState>().body.position;
    assert_ne!(walked, start, "the stick walks");
    let third = app.world().resource::<PlayerState>().third_person;
    let camera = layout.button(TouchButton::Camera);
    finger(&mut app, TouchPhase::Started, 2, camera);
    app.update();
    finger(&mut app, TouchPhase::Ended, 2, camera);
    app.update();
    assert_ne!(
        app.world().resource::<PlayerState>().third_person,
        third,
        "Camera switches the view"
    );
    let jump = layout.button(TouchButton::Jump);
    finger(&mut app, TouchPhase::Started, 3, jump);
    app.update();
    finger(&mut app, TouchPhase::Ended, 3, jump);
    app.update();
    assert!(
        !app.world().resource::<PlayerState>().body.on_ground,
        "Jump leaves the ground"
    );
}

#[test]
fn touch_flies_the_autopilot_pinches_and_taps_in_dive_mode() {
    use crate::input::{Autopilot, Navigation};
    use bevy::input::touch::TouchPhase;
    use universe_core::touch::TouchButton;
    let mut app = headless_app();
    app.update();
    let button = touch_layout().button(TouchButton::Autopilot);
    finger(&mut app, TouchPhase::Started, 1, button);
    app.update();
    finger(&mut app, TouchPhase::Ended, 1, button);
    app.update();
    assert_eq!(*app.world().resource::<Autopilot>(), Autopilot::Flying);
    // A pinch is a dive input: it takes over from the autopilot.
    finger(&mut app, TouchPhase::Started, 2, [600.0, 360.0]);
    finger(&mut app, TouchPhase::Started, 3, [680.0, 360.0]);
    app.update();
    finger(&mut app, TouchPhase::Moved, 2, [560.0, 360.0]);
    finger(&mut app, TouchPhase::Moved, 3, [720.0, 360.0]);
    app.update();
    assert_eq!(*app.world().resource::<Autopilot>(), Autopilot::Idle);
    finger(&mut app, TouchPhase::Ended, 2, [560.0, 360.0]);
    finger(&mut app, TouchPhase::Ended, 3, [720.0, 360.0]);
    app.update();
    // A tap chooses the place under the finger (headless: the hover).
    let way = app.world().resource::<Navigation>().target;
    let other = {
        let universe = app.world().resource::<Universe>();
        universe
            .open
            .points
            .iter()
            .enumerate()
            .filter(|(_, p)| p.kind == universe_core::r#gen::MarkerKind::Portal)
            .map(|(i, _)| u32::try_from(i).expect("budgeted"))
            .find(|m| Some(*m) != way)
            .expect("another portal")
    };
    finger(&mut app, TouchPhase::Started, 4, [400.0, 200.0]);
    app.update();
    app.world_mut().resource_mut::<Navigation>().hover = Some(other);
    finger(&mut app, TouchPhase::Ended, 4, [401.0, 200.0]);
    app.update();
    assert_eq!(
        app.world().resource::<Navigation>().target,
        Some(other),
        "a tap chooses the target"
    );
}

#[test]
fn simulate_touch_turns_the_mouse_into_a_finger_only_when_on() {
    use crate::input::{Autopilot, Flight, FlightMode};
    let mut app = headless_app();
    app.update();
    // Off: a left click on the Autopilot spot is an ordinary click.
    let press = |app: &mut App| {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        buttons.release(MouseButton::Left);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
    };
    let button = touch_layout().button(universe_core::touch::TouchButton::Autopilot);
    let mut window = Window::default();
    window.resolution.set(1280.0, 720.0);
    window.set_cursor_position(Some(Vec2::new(button[0], button[1])));
    app.world_mut().spawn(window);
    app.update();
    press(&mut app);
    assert_eq!(*app.world().resource::<Autopilot>(), Autopilot::Idle);
    assert!(!app.world().resource::<crate::touch::TouchState>().seen);
    // On: the same click presses the Autopilot button.
    app.world_mut()
        .resource_mut::<SettingsStore>()
        .0
        .simulate_touch = true;
    press(&mut app);
    assert_eq!(*app.world().resource::<Autopilot>(), Autopilot::Flying);
    assert!(app.world().resource::<crate::touch::TouchState>().seen);
    assert_eq!(app.world().resource::<Flight>().mode, FlightMode::Dive);
}
