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

#[test]
fn player_stands_on_the_drawn_ground_at_the_planet() {
    // #779 T4: at L10, key 1 drops the body onto the drawn surface the
    // window uses once its builds are done; it stays there, feet within a
    // centimetre.
    use crate::patches::GroundSurface;
    use crate::player_state::PlayerState;
    let mut app = opened(9);
    assert_eq!(app.world().resource::<Universe>().level().get(), 10);
    crate::player::player_here(&mut app);
    tap(&mut app, KeyCode::Digit1);
    let mut settled = false;
    for _ in 0..600 {
        app.update();
        let state = app.world().resource::<PlayerState>();
        if state.active && !state.ground_pending {
            settled = true;
            break;
        }
    }
    assert!(settled, "the body never reached the drawn ground");
    // Measure once the builds stay done for a whole frame: the body steps
    // before the patches sync, so it has stood on this very surface.
    let mut idle_frames = 0;
    for _ in 0..600 {
        app.update();
        let idle = app.world().resource::<GroundSurface>().1;
        idle_frames = if idle { idle_frames + 1 } else { 0 };
        if idle_frames >= 2 {
            break;
        }
    }
    assert!(idle_frames >= 2, "the patch builds never finished");
    let state = app.world().resource::<PlayerState>().clone();
    assert!(!state.body.flying, "standing, not flying");
    let surface = app
        .world()
        .resource::<GroundSurface>()
        .0
        .clone()
        .expect("drawn surface at L10");
    let g = surface.ground_at(state.body.position).expect("ground");
    let frame = surface.frame();
    let p = frame.from_cell(state.body.position).0;
    let q = frame.from_cell(g.point).0;
    let gap = (p[0] - q[0]) * g.up[0] + (p[1] - q[1]) * g.up[1] + (p[2] - q[2]) * g.up[2];
    assert!(gap.abs() < 0.01, "feet {gap} m off the drawn ground");
}

#[test]
fn player_hidden_until_ground_ready() {
    // #919 T1 AC1-AC3: at L10 the figure hides while it would render under
    // the drawn ground, then stands exactly on it.
    use crate::figure_draw::FigureMesh;
    use crate::input::{Flight, FlightMode};
    use crate::patches::GroundSurface;
    use crate::player_state::PlayerState;
    use universe_core::player::PlayerSpot;
    let mut app = opened(9);
    assert_eq!(app.world().resource::<Universe>().level().get(), 10);
    crate::player::player_here(&mut app);
    tap(&mut app, KeyCode::Digit1);
    let figures = |app: &mut App| {
        app.world_mut()
            .query_filtered::<Entity, With<FigureMesh>>()
            .iter(app.world())
            .count()
    };
    let settled = |app: &mut App| {
        let state = app.world().resource::<PlayerState>().clone();
        state.active && !state.ground_pending
    };
    // The drop lands: the figure stands on the drawn ground.
    let mut ok = false;
    for _ in 0..600 {
        app.update();
        if settled(&mut app) {
            ok = true;
            break;
        }
    }
    assert!(ok, "the body never reached the drawn ground");
    assert!(figures(&mut app) > 0, "the figure shows on ready ground");
    // Waiting figure as on a glide arrival: park the standing body as the
    // spot (the follow logic keeps it via the saved player) and go
    // inactive in dive mode. The spot never steps, so it stays put while
    // the ground under it is checked.
    let park = |app: &mut App, at: [f64; 3]| {
        let (indices, yaw, root) = {
            let universe = app.world().resource::<Universe>();
            let state = app.world().resource::<PlayerState>();
            (universe.path.indices(), state.body.yaw, state.root)
        };
        let spot = PlayerSpot::new(&indices, at, yaw).expect("spot");
        {
            let mut state = app.world_mut().resource_mut::<PlayerState>();
            state.active = false;
            state.spot = spot;
            state.root = root;
        }
        app.world_mut().resource_mut::<SettingsStore>().0.player = Some((root, spot));
        app.world_mut().resource_mut::<Flight>().mode = FlightMode::Dive;
    };
    let feet0 = app.world().resource::<PlayerState>().body.position;
    park(&mut app, feet0);
    app.update();
    assert!(
        figures(&mut app) > 0,
        "waiting figure shows on ready ground"
    );
    // ... then hide it while the ground is gone, as if placed before it
    // was built: the pending flag plus no drawable surface. The
    // in-flight build queues are left running, so the real rebuild
    // returns the figure below.
    {
        let mut state = app.world_mut().resource_mut::<PlayerState>();
        state.ground_pending = true;
    }
    {
        let mut ground = app.world_mut().resource_mut::<GroundSurface>();
        ground.0 = None;
        ground.1 = false;
        ground.2 = false;
    }
    app.update();
    assert_eq!(figures(&mut app), 0, "no figure while the ground builds");
    app.world_mut().resource_mut::<PlayerState>().ground_pending = false;
    let mut back = false;
    for _ in 0..1800 {
        app.update();
        if figures(&mut app) > 0 {
            back = true;
            break;
        }
    }
    assert!(back, "the figure never returns");
}

#[test]
fn player_sits_exactly_on_drawn_ground() {
    // #919 T3 AC5: at L10 the first settled frame already stands exactly
    // on the drawn ground: feet within a centimetre, standing not flying.
    // (The drop itself is #779's; this locks its exactness for #919.)
    use crate::patches::GroundSurface;
    use crate::player_state::PlayerState;
    let mut app = opened(9);
    assert_eq!(app.world().resource::<Universe>().level().get(), 10);
    crate::player::player_here(&mut app);
    tap(&mut app, KeyCode::Digit1);
    let mut ok = false;
    for _ in 0..600 {
        app.update();
        let state = app.world().resource::<PlayerState>().clone();
        if state.active && !state.ground_pending {
            ok = true;
            break;
        }
    }
    assert!(ok, "the body never reached the drawn ground");
    let state = app.world().resource::<PlayerState>().clone();
    assert!(!state.body.flying, "standing, not flying");
    let surface = app
        .world()
        .resource::<GroundSurface>()
        .0
        .clone()
        .expect("drawn surface at L10");
    let g = surface.ground_at(state.body.position).expect("ground");
    let frame = surface.frame();
    let p = frame.from_cell(state.body.position).0;
    let q = frame.from_cell(g.point).0;
    let gap = (p[0] - q[0]) * g.up[0] + (p[1] - q[1]) * g.up[1] + (p[2] - q[2]) * g.up[2];
    assert!(gap.abs() < 0.01, "feet {gap} m off the drawn ground");
}

fn dive_distance(app: &App) -> (f64, [f64; 3], Option<[f64; 3]>) {
    use universe_core::nav::MarkerIndex;
    let universe = app.world().resource::<Universe>();
    let nav = app.world().resource::<crate::input::Navigation>();
    let center = nav
        .target
        .and_then(|m| universe.marker(MarkerIndex(m)))
        .unwrap_or([0.0; 3]);
    let camera = universe.path.offset();
    let dx = camera[0] - center[0];
    let dy = camera[1] - center[1];
    let dz = camera[2] - center[2];
    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
    (distance, camera, Some(center))
}

fn hold_key(app: &mut App, key: KeyCode, frames: usize) {
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

fn send_motion(app: &mut App, dx: f32, dy: f32) {
    use bevy::input::mouse::MouseMotion;
    app.world_mut().write_message(MouseMotion {
        delta: Vec2::new(dx, dy),
    });
}

#[test]
fn dive_wasd_orbits_at_same_distance() {
    // #873 T2/Q1 AC1: W S A D orbit in dive mode at the same distance.
    let mut app = headless_app();
    app.update();
    let (before, camera_before, _) = dive_distance(&app);
    assert!(before > 0.0);
    hold_key(&mut app, KeyCode::KeyA, 60);
    let (after, camera_after, _) = dive_distance(&app);
    let rel = (after - before).abs() / before;
    assert!(
        rel < 1e-3,
        "WASD orbit keeps the distance within 0.1 %: {before} -> {after}"
    );
    assert_ne!(camera_after, camera_before, "A moves the camera");
    hold_key(&mut app, KeyCode::KeyW, 60);
    let (third, _, _) = dive_distance(&app);
    let rel2 = (third - before).abs() / before;
    assert!(rel2 < 1e-3, "W keeps the distance too: {before} -> {third}");
}

#[test]
fn dive_left_drag_orbits_and_click_still_picks() {
    // #873 T2 AC1: a left-drag orbits; a plain click without dragging still
    // chooses a target (threshold, so clicks are not swallowed).
    use crate::input::CursorPortals;
    use crate::tokens::CYCLE_CAP;
    let mut app = headless_app();
    app.update();
    // Stage a ranked order so the click picks deterministically.
    let way = app.world().resource::<crate::input::Navigation>().target;
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
    // Plain click (no motion) picks.
    app.world_mut()
        .resource_mut::<CursorPortals>()
        .clone_from(&CursorPortals {
            pos: [100.0, 100.0],
            order: {
                let mut o = [0; CYCLE_CAP];
                o[0] = other;
                o
            },
            len: 1,
            valid: true,
            ..CursorPortals::default()
        });
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    {
        let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        buttons.release(MouseButton::Left);
        buttons.clear();
    }
    app.update();
    assert_eq!(
        app.world().resource::<crate::input::Navigation>().target,
        Some(other),
        "a click without dragging still chooses a target"
    );
    // Left-drag orbits at the same distance.
    let (before, _, _) = dive_distance(&app);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    // One large drag past the click threshold.
    send_motion(&mut app, 60.0, 0.0);
    app.update();
    send_motion(&mut app, 60.0, 10.0);
    app.update();
    {
        let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        buttons.release(MouseButton::Left);
        buttons.clear();
    }
    app.update();
    let (after, camera_after, _) = dive_distance(&app);
    let rel = (after - before).abs() / before;
    assert!(
        rel < 1e-3,
        "left-drag orbit keeps the distance: {before} -> {after}"
    );
    assert!(
        camera_after != app.world().resource::<Universe>().path.offset() || rel < 1e-3,
        "drag moved the camera"
    );
}

#[test]
fn dive_right_drag_turns_in_place() {
    // #873 T2/Q2 AC3: right-drag turns the view around the line to the
    // target. The camera never moves, the turn stays.
    let mut app = headless_app();
    app.update();
    let camera_before = app.world().resource::<Universe>().path.offset();
    let roll_before = app.world().resource::<crate::input::Navigation>().roll;
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Right);
    send_motion(&mut app, 60.0, 0.0);
    app.update();
    {
        let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        buttons.release(MouseButton::Right);
        buttons.clear();
    }
    app.update();
    let camera_after = app.world().resource::<Universe>().path.offset();
    let roll_after = app.world().resource::<crate::input::Navigation>().roll;
    assert_eq!(camera_after, camera_before, "turn never moves the camera");
    assert_ne!(roll_after, roll_before, "turn stays where left");
    // The drawn roll eases toward the dragged roll and then holds (#884 T1):
    // release stops the turn. Fixed 16 ms steps so the 50 ms easing settles
    // deterministically. A window is spawned so `sync_camera` (which owns the
    // easing and skips without exactly one window) runs headless.
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;
    app.world_mut().spawn((Window::default(),));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        16,
    )));
    for _ in 0..600 {
        app.update();
    }
    let drawn = app
        .world()
        .resource::<crate::input::Navigation>()
        .roll_drawn;
    let target = app.world().resource::<crate::input::Navigation>().roll;
    assert!(
        (drawn - target).abs() < 1e-9,
        "the drawn roll settles on the dragged angle"
    );
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(
        app.world()
            .resource::<crate::input::Navigation>()
            .roll_drawn,
        drawn,
        "no drift after release"
    );
}

#[test]
fn dive_orbit_follows_the_turn() {
    // #884 T1 review: the orbit runs around the drawn (rolled) up, so after
    // a 90-degree turn W still moves up on the screen instead of sideways.
    // From the same start pose, W with no roll and W with a 90-degree roll
    // move the camera in perpendicular directions.
    use crate::input::Navigation;
    use std::f64::consts::FRAC_PI_2;
    fn displaced(app: &mut App, frames: usize) -> [f64; 3] {
        let before = app.world().resource::<Universe>().path.offset();
        hold_key(app, KeyCode::KeyW, frames);
        let after = app.world().resource::<Universe>().path.offset();
        [
            after[0] - before[0],
            after[1] - before[1],
            after[2] - before[2],
        ]
    }
    fn norm(v: [f64; 3]) -> [f64; 3] {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        assert!(len > 1e-12, "the orbit moves the camera");
        [v[0] / len, v[1] / len, v[2] / len]
    }
    let mut app = headless_app();
    app.update();
    for _ in 0..10 {
        app.update();
    }
    let pose = (
        app.world().resource::<Universe>().path.offset(),
        app.world().resource::<Navigation>().forward,
        app.world().resource::<Navigation>().up,
    );
    let plain = norm(displaced(&mut app, 30));
    // Back to the same pose, then a 90-degree drawn turn (the drag path is
    // covered by `dive_right_drag_turns_in_place`; the orbit reads the drawn
    // roll directly).
    app.world_mut()
        .resource_mut::<Universe>()
        .path
        .set_offset(pose.0);
    app.world_mut().resource_mut::<Navigation>().forward = pose.1;
    app.world_mut().resource_mut::<Navigation>().up = pose.2;
    app.world_mut().resource_mut::<Navigation>().roll = FRAC_PI_2;
    app.world_mut().resource_mut::<Navigation>().roll_drawn = FRAC_PI_2;
    let turned = norm(displaced(&mut app, 30));
    let dot = plain[0] * turned[0] + plain[1] * turned[1] + plain[2] * turned[2];
    assert!(
        dot.abs() < 0.2,
        "a 90-degree turn swaps the screen axes: {dot}"
    );
}

#[test]
fn free_view_ignores_gravity_near_the_planet() {
    // AC2/C2 second half (#884 T2): standing still for 10 s near the planet
    // in free mode never turns the view. `sync_camera` leaves the free pose
    // alone instead of easing it toward the planet's up.
    use crate::input::{Flight, FlightMode, Navigation};
    let mut app = opened(9);
    // A window is spawned so `sync_camera` (which skips without exactly one
    // window) runs headless; without it the test would pass vacuously.
    app.world_mut().spawn((Window::default(),));
    tap(&mut app, KeyCode::Digit2);
    assert_eq!(app.world().resource::<Flight>().mode, FlightMode::Free);
    let flight_before = (
        app.world().resource::<Flight>().free_forward,
        app.world().resource::<Flight>().free_up,
    );
    let nav_before = (
        app.world().resource::<Navigation>().forward,
        app.world().resource::<Navigation>().up,
    );
    for _ in 0..600 {
        app.update();
    }
    assert_eq!(
        (
            app.world().resource::<Flight>().free_forward,
            app.world().resource::<Flight>().free_up
        ),
        flight_before,
        "no input leaves the free pose alone"
    );
    assert_eq!(
        (
            app.world().resource::<Navigation>().forward,
            app.world().resource::<Navigation>().up
        ),
        nav_before,
        "gravity never turns the free view"
    );
}

#[test]
fn touch_drag_orbits_in_dive() {
    // #873 T2/Q1 AC10 C10: one-finger drag in dive mode orbits like the
    // left mouse drag; a tap still picks.
    use bevy::input::touch::TouchPhase;
    let mut app = headless_app();
    app.update();
    let (before, _, _) = dive_distance(&app);
    // Free-screen drag away from the stick and buttons.
    finger(&mut app, TouchPhase::Started, 1, [640.0, 300.0]);
    app.update();
    finger(&mut app, TouchPhase::Moved, 1, [700.0, 300.0]);
    app.update();
    finger(&mut app, TouchPhase::Moved, 1, [760.0, 310.0]);
    app.update();
    finger(&mut app, TouchPhase::Ended, 1, [760.0, 310.0]);
    app.update();
    let (after, camera_after, _) = dive_distance(&app);
    let rel = (after - before).abs() / before;
    assert!(
        rel < 1e-3,
        "touch drag keeps the distance: {before} -> {after}"
    );
    assert_ne!(camera_after, [640.0, 300.0, 0.0], "sanity");
    // Tap still picks (covered by the existing tap test, re-asserted here
    // through the tap resource clearing each frame).
    assert!(
        app.world()
            .resource::<crate::touch::TouchState>()
            .tap
            .is_none()
    );
}

#[test]
fn behind_follow_walking_and_flying_first_unchanged() {
    // #899 AC1/AC3/AC4: in third person an A/D push moves the camera heading
    // (walk and fly); Off and first person leave it exactly as today.
    use crate::input::Flight;
    use crate::player_state::PlayerState;
    use universe_core::settings::BehindFollow;
    fn headings(app: &App) -> (f64, f64, bool, bool) {
        let flight = app.world().resource::<Flight>();
        let state = app.world().resource::<PlayerState>();
        (
            flight.target_yaw,
            state.body.yaw,
            state.third_person,
            state.body.flying,
        )
    }
    fn enter_third(app: &mut App) {
        crate::player::player_here(app);
        tap(app, KeyCode::Digit1);
        app.update();
        assert!(app.world().resource::<PlayerState>().active);
        assert!(app.world().resource::<PlayerState>().third_person);
        // The follow is off during the takeover handover by design; wait
        // for it to end so the frames below do not depend on frame timing
        // (CI flake, #907).
        for _ in 0..600 {
            if app
                .world()
                .resource::<PlayerState>()
                .handover_from
                .is_none()
            {
                break;
            }
            app.update();
        }
        assert!(
            app.world()
                .resource::<PlayerState>()
                .handover_from
                .is_none()
        );
    }
    // Third person walking, Smooth (default): holding A moves the camera.
    let mut app = opened(9);
    enter_third(&mut app);
    let (before, _, _, _) = headings(&app);
    hold(&mut app, KeyCode::KeyA, 30);
    let (after, body_after, _, _) = headings(&app);
    assert_ne!(after, before, "A/D turns pull the camera behind");
    assert_ne!(
        body_after, before,
        "the body turns toward its walking direction"
    );
    // Off keeps today: the camera heading never moves.
    let mut off = opened(9);
    enter_third(&mut off);
    off.world_mut()
        .resource_mut::<SettingsStore>()
        .0
        .behind_follow = BehindFollow::Off;
    let (off_before, _, _, _) = headings(&off);
    hold(&mut off, KeyCode::KeyA, 30);
    assert_eq!(
        headings(&off).0,
        off_before,
        "Off leaves the camera where the mouse left it"
    );
    // First person: A strafes, the heading stays with the look.
    let mut first = opened(9);
    enter_third(&mut first);
    tap(&mut first, KeyCode::KeyV);
    assert!(!first.world().resource::<PlayerState>().third_person);
    let (first_before, _, _, _) = headings(&first);
    hold(&mut first, KeyCode::KeyA, 30);
    assert_eq!(
        headings(&first).0,
        first_before,
        "first person stays as today"
    );
    // Third person flying: the follow applies too.
    let mut air = opened(9);
    enter_third(&mut air);
    air.world_mut().resource_mut::<PlayerState>().body.flying = true;
    let (air_before, _, _, flying) = headings(&air);
    assert!(flying, "test starts airborne");
    hold(&mut air, KeyCode::KeyA, 30);
    assert_ne!(
        headings(&air).0,
        air_before,
        "flying turns pull the camera behind too"
    );
}

#[test]
fn behind_follow_settles_behind() {
    // AC1/AC3 (#899 Spec v5, the PR #905 review): A/D turn the body
    // car-style and the camera turns with it, so holding D never makes it
    // circle on its own; after a mouse offset, walking with W eases the
    // camera back and it settles exactly behind the body.
    use crate::input::Flight;
    use crate::player_state::PlayerState;
    fn wrapped(a: f64) -> f64 {
        let two_pi = 2.0 * std::f64::consts::PI;
        let r = a.rem_euclid(two_pi);
        if r > std::f64::consts::PI {
            r - two_pi
        } else {
            r
        }
    }
    fn gap(app: &App) -> f64 {
        let body = app.world().resource::<PlayerState>().body.yaw;
        let camera = app.world().resource::<Flight>().target_yaw;
        wrapped(body - camera)
    }
    let mut app = opened(9);
    crate::player::player_here(&mut app);
    tap(&mut app, KeyCode::Digit1);
    // Let the handover finish so the follow is live.
    for _ in 0..600 {
        if app
            .world()
            .resource::<PlayerState>()
            .handover_from
            .is_none()
        {
            break;
        }
        app.update();
    }
    assert!(app.world().resource::<PlayerState>().third_person);
    assert!(
        app.world()
            .resource::<PlayerState>()
            .handover_from
            .is_none()
    );
    let start_gap = gap(&app).abs();
    // Hold D for 60 frames: the body turns right and the camera turns the
    // same way every frame, never falling further behind.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyD);
    let body_start = app.world().resource::<PlayerState>().body.yaw;
    let mut last_camera = app.world().resource::<Flight>().target_yaw;
    for _ in 0..60 {
        app.update();
        let camera = app.world().resource::<Flight>().target_yaw;
        assert!(
            camera <= last_camera + 1e-12,
            "the camera turns right with the body"
        );
        last_camera = camera;
        assert!(
            gap(&app).abs() <= start_gap + 1e-9,
            "never falls behind or circles: {} (start {start_gap})",
            gap(&app)
        );
    }
    let body_end = app.world().resource::<PlayerState>().body.yaw;
    assert!(
        body_end < body_start - 1.0,
        "D turned the body: {body_start} -> {body_end}"
    );
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyD);
        keys.clear();
    }
    // A mouse offset of half a radian, then walking with W: the camera
    // eases back and settles exactly behind.
    {
        let mut flight = app.world_mut().resource_mut::<Flight>();
        flight.target_yaw += 0.5;
        flight.yaw += 0.5;
    }
    assert!(gap(&app).abs() > 0.4, "the mouse moved the camera aside");
    hold(&mut app, KeyCode::KeyW, 40);
    assert!(gap(&app).abs() < 1e-9, "settled behind: {}", gap(&app));
}

/// Holds the look button (right mouse), moves the mouse by `(dx, dy)` and
/// runs one frame (#907 T2).
fn look_drag(app: &mut App, dx: f32, dy: f32) {
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Right);
    send_motion(app, dx, dy);
    app.update();
    let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    buttons.release(MouseButton::Right);
    buttons.clear();
}

#[test]
fn free_mouse_up_looks_up() {
    // C2 (#907 T2, AC2): in free mode moving the mouse up (negative screen
    // dy) turns the view up and down turns it down; Invert look flips both;
    // left and right are unchanged (a drag right turns right).
    use crate::input::{Flight, FlightMode};
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    // How much the view direction rose along the old up during one drag.
    let rise = |app: &mut App, dy: f32| {
        let (fwd0, up0) = {
            let flight = app.world().resource::<Flight>();
            (flight.free_forward, flight.free_up)
        };
        look_drag(app, 0.0, dy);
        dot(app.world().resource::<Flight>().free_forward, up0) - dot(fwd0, up0)
    };
    let mut app = headless_app();
    app.update();
    tap(&mut app, KeyCode::KeyF);
    assert_eq!(app.world().resource::<Flight>().mode, FlightMode::Free);
    assert!(rise(&mut app, -40.0) > 0.0, "mouse up looks up");
    let mut app = headless_app();
    app.update();
    tap(&mut app, KeyCode::KeyF);
    assert!(rise(&mut app, 40.0) < 0.0, "mouse down looks down");
    let mut app = headless_app();
    app.update();
    tap(&mut app, KeyCode::KeyF);
    app.world_mut()
        .resource_mut::<SettingsStore>()
        .0
        .invert_look = true;
    assert!(rise(&mut app, -40.0) < 0.0, "Invert look flips it");
    // Left and right as before: a drag right turns toward the view's right.
    let mut app = headless_app();
    app.update();
    tap(&mut app, KeyCode::KeyF);
    let (fwd, up) = {
        let flight = app.world().resource::<Flight>();
        (flight.free_forward, flight.free_up)
    };
    let right = universe_core::flight::free_right(fwd, up);
    look_drag(&mut app, 40.0, 0.0);
    assert!(
        dot(app.world().resource::<Flight>().free_forward, right) > 0.0,
        "a drag right turns right"
    );
}

#[test]
fn player_mouse_up_looks_up() {
    // C3 (#907, owner check): the player camera already looks up with the
    // mouse up; this pins it so it cannot flip.
    use crate::input::Flight;
    use crate::player_state::PlayerState;
    let mut app = opened(9);
    crate::player::player_here(&mut app);
    tap(&mut app, KeyCode::Digit1);
    app.update();
    assert!(app.world().resource::<PlayerState>().active);
    let before = app.world().resource::<Flight>().target_pitch;
    look_drag(&mut app, 0.0, -40.0);
    let after = app.world().resource::<Flight>().target_pitch;
    assert!(
        after > before,
        "mouse up raises the look: {before} -> {after}"
    );
    look_drag(&mut app, 0.0, 80.0);
    let lower = app.world().resource::<Flight>().target_pitch;
    assert!(lower < after, "mouse down lowers it: {after} -> {lower}");
}
