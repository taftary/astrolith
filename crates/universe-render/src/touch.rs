//! Touch gestures for the game (#561).
//!
//! Every frame [`read_touches`] turns Bevy [`TouchInput`] messages into one
//! [`TouchState`] that the input systems read next to the key map. With
//! Settings > Developer > Simulate touch on, the left mouse button and the
//! cursor become one finger, so both paths share this code (owner answer
//! Q9 B). Each finger takes one role when it goes down: the stick, a round
//! button, the mode reminder, or the free screen (one free finger looks or
//! taps, two pinch). The layout and gesture math live in
//! [`universe_core::touch`]. Everything here is `pub(crate)`.

use crate::input::{Flight, FlightMode};
use crate::settings_ui::SettingsStore;
use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use universe_core::touch::{
    Control, Layout, Shown, StickInput, TouchButton, distance, is_tap, pinch_log_factor, stick,
};

/// Most fingers followed at once (fixed slots, `E-HOT-NOALLOC`).
pub(crate) const MAX_FINGERS: usize = 5;

/// Finger id the simulated (mouse) finger uses.
pub(crate) const SIMULATED_ID: u64 = u64::MAX;

/// Window size used when no window exists (headless tests).
const FALLBACK_SIZE: [f32; 2] = [1280.0, 720.0];

/// What a finger does, chosen when it goes down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    /// Works the on-screen stick.
    Stick,
    /// Holds one round button.
    Button(TouchButton),
    /// Went down on the mode reminder (a tap cycles the mode).
    Reminder,
    /// Free screen: look, tap, or pinch.
    Free,
    /// Went down on another screen control (the menu button): ignored.
    Ui,
}

/// One finger on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Finger {
    /// Toolkit finger id.
    pub id: u64,
    /// Where it went down (logical px).
    pub start: [f32; 2],
    /// Where it is now.
    pub at: [f32; 2],
    /// Farthest it has been from `start`.
    pub travel: f32,
    /// Seconds since it went down.
    pub age: f64,
    /// What it does.
    pub role: Role,
}

/// This frame's touch input, read by the game's input systems.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub(crate) struct TouchState {
    /// Fingers on the screen.
    pub fingers: [Option<Finger>; MAX_FINGERS],
    /// Directions the stick presses.
    pub stick: StickInput,
    /// Look drag this frame (logical px, like mouse motion).
    pub look: [f32; 2],
    /// Dive log factor from a pinch this frame.
    pub pinch: f64,
    /// A free-screen tap this frame, at this point.
    pub tap: Option<[f32; 2]>,
    /// Buttons held, in [`TouchButton::ALL`] order.
    pub held: [bool; 4],
    /// Buttons pressed this frame.
    pub pressed: [bool; 4],
    /// The mode reminder was tapped this frame.
    pub reminder_tapped: bool,
    /// A touch came after the last key or mouse press (automatic show).
    pub seen: bool,
}

impl TouchState {
    /// Whether `button` is held.
    #[must_use]
    pub(crate) fn held(&self, button: TouchButton) -> bool {
        self.held.get(index(button)).copied().unwrap_or(false)
    }

    /// Whether `button` went down this frame.
    #[must_use]
    pub(crate) fn pressed(&self, button: TouchButton) -> bool {
        self.pressed.get(index(button)).copied().unwrap_or(false)
    }

    /// Current positions of the free fingers (at most two matter).
    fn free(&self) -> [Option<(u64, [f32; 2])>; 2] {
        let mut out = [None; 2];
        let mut n = 0;
        for finger in self.fingers.iter().flatten() {
            if finger.role == Role::Free {
                if let Some(slot) = out.get_mut(n) {
                    *slot = Some((finger.id, finger.at));
                }
                n += 1;
            }
        }
        if n > 2 { [None; 2] } else { out }
    }
}

fn index(button: TouchButton) -> usize {
    TouchButton::ALL
        .iter()
        .position(|b| *b == button)
        .unwrap_or(0)
}

/// Which controls the current mode shows.
#[must_use]
pub(crate) fn shown(mode: FlightMode, flying: bool) -> Shown {
    match mode {
        FlightMode::Normal => Shown::normal(flying),
        FlightMode::Free => Shown::free(),
        FlightMode::Dive => Shown::dive(),
    }
}

/// One touch event, from the toolkit or simulated from the mouse.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Event {
    phase: TouchPhase,
    id: u64,
    at: [f32; 2],
}

/// Whether a node of `query` covers `at` (logical px; nodes are physical).
fn contains<F: bevy::ecs::query::QueryFilter>(
    query: &Query<(&ComputedNode, &UiGlobalTransform), F>,
    at: [f32; 2],
    scale: f32,
) -> bool {
    let point = Vec2::new(at[0], at[1]) * scale;
    query
        .iter()
        .any(|(node, transform)| node.contains_point(*transform, point))
}

/// Fills [`TouchState`] from this frame's touches (#561).
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn read_touches(
    time: Res<Time>,
    mut touches: MessageReader<TouchInput>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    store: Res<SettingsStore>,
    flight: Res<Flight>,
    normal: Res<crate::normal_mode::NormalState>,
    reminder: Query<(&ComputedNode, &UiGlobalTransform), With<crate::hud::ModeReminderText>>,
    menu: Query<(&ComputedNode, &UiGlobalTransform), With<crate::menu::MenuButton>>,
    mut state: ResMut<TouchState>,
) {
    let window = windows.iter().next();
    let size = window.map_or(FALLBACK_SIZE, |w| [w.width(), w.height()]);
    let scale = window.map_or(1.0, Window::scale_factor);
    let layout = Layout::new(size[0], size[1], store.0.left_hand);
    let flying = flight.mode == FlightMode::Normal && normal.active && normal.body.flying;
    let shown = shown(flight.mode, flying);

    state.look = [0.0; 2];
    state.pinch = 0.0;
    state.tap = None;
    state.pressed = [false; 4];
    state.reminder_tapped = false;
    // A key or (real) mouse press hides automatic touch controls.
    let simulate = store.0.simulate_touch;
    if keys.get_just_pressed().next().is_some()
        || (!simulate && buttons.get_just_pressed().next().is_some())
    {
        state.seen = false;
    }
    let before = state.free();

    let mut events: [Option<Event>; 16] = [None; 16];
    let mut count = 0;
    let mut push = |event: Event| {
        if let Some(slot) = events.get_mut(count) {
            *slot = Some(event);
            count += 1;
        }
    };
    for touch in touches.read() {
        push(Event {
            phase: touch.phase,
            id: touch.id,
            at: [touch.position.x, touch.position.y],
        });
    }
    if simulate && let Some(cursor) = window.and_then(Window::cursor_position) {
        let at = [cursor.x, cursor.y];
        let down = state
            .fingers
            .iter()
            .flatten()
            .any(|finger| finger.id == SIMULATED_ID);
        if buttons.just_pressed(MouseButton::Left) && !down {
            push(Event {
                phase: TouchPhase::Started,
                id: SIMULATED_ID,
                at,
            });
        }
        if buttons.just_released(MouseButton::Left) {
            push(Event {
                phase: TouchPhase::Ended,
                id: SIMULATED_ID,
                at,
            });
        } else if buttons.pressed(MouseButton::Left) && down {
            push(Event {
                phase: TouchPhase::Moved,
                id: SIMULATED_ID,
                at,
            });
        }
    }

    for event in events.iter().flatten() {
        match event.phase {
            TouchPhase::Started => {
                state.seen = true;
                let role = if contains(&menu, event.at, scale) {
                    Role::Ui
                } else if contains(&reminder, event.at, scale) {
                    Role::Reminder
                } else {
                    match layout.hit(event.at, shown) {
                        Some(Control::Stick) => Role::Stick,
                        Some(Control::Button(button)) => {
                            if let Some(pressed) = state.pressed.get_mut(index(button)) {
                                *pressed = true;
                            }
                            Role::Button(button)
                        }
                        None => Role::Free,
                    }
                };
                let finger = Finger {
                    id: event.id,
                    start: event.at,
                    at: event.at,
                    travel: 0.0,
                    age: 0.0,
                    role,
                };
                if let Some(slot) = state.fingers.iter_mut().find(|slot| slot.is_none()) {
                    *slot = Some(finger);
                }
            }
            TouchPhase::Moved => {
                if let Some(finger) = state
                    .fingers
                    .iter_mut()
                    .flatten()
                    .find(|finger| finger.id == event.id)
                {
                    finger.at = event.at;
                    finger.travel = finger.travel.max(distance(finger.start, event.at));
                }
            }
            TouchPhase::Ended | TouchPhase::Canceled => {
                let Some(slot) = state
                    .fingers
                    .iter_mut()
                    .find(|slot| slot.is_some_and(|finger| finger.id == event.id))
                else {
                    continue;
                };
                let Some(mut finger) = slot.take() else {
                    continue;
                };
                finger.travel = finger.travel.max(distance(finger.start, event.at));
                let tapped = event.phase == TouchPhase::Ended && is_tap(finger.travel, finger.age);
                match finger.role {
                    Role::Free if tapped => state.tap = Some(event.at),
                    Role::Reminder if tapped => state.reminder_tapped = true,
                    _ => {}
                }
            }
        }
    }

    // One free finger drags the look; two pinch.
    let after = state.free();
    match (before, after) {
        ([Some((a, from)), None], [Some((b, to)), None]) if a == b => {
            state.look = [to[0] - from[0], to[1] - from[1]];
        }
        ([Some((a0, a)), Some((b0, b))], [Some((a1, c)), Some((b1, d))])
            if a0 == a1 && b0 == b1 =>
        {
            state.pinch = pinch_log_factor(distance(a, b), distance(c, d));
        }
        _ => {}
    }

    let dt = f64::from(time.delta_secs());
    let mut held = [false; 4];
    let mut stick_input = StickInput::default();
    for finger in state.fingers.iter_mut().flatten() {
        finger.age += dt;
        match finger.role {
            Role::Stick => {
                stick_input = stick([
                    finger.at[0] - layout.stick[0],
                    finger.at[1] - layout.stick[1],
                ]);
            }
            Role::Button(button) => {
                if let Some(hold) = held.get_mut(index(button)) {
                    *hold = true;
                }
            }
            Role::Reminder | Role::Free | Role::Ui => {}
        }
    }
    state.held = held;
    state.stick = stick_input;
}

/// Forgets every finger when the universe view is left (a screen opens):
/// screens take taps through the UI, and no finger outlives the switch.
pub(crate) fn clear_touches(mut state: ResMut<TouchState>) {
    let seen = state.seen;
    *state = TouchState {
        seen,
        ..TouchState::default()
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_tests::headless_app;

    pub(crate) fn touch(app: &mut App, phase: TouchPhase, id: u64, at: [f32; 2]) {
        let window = Entity::PLACEHOLDER;
        app.world_mut().write_message(TouchInput {
            phase,
            position: Vec2::new(at[0], at[1]),
            window,
            force: None,
            id,
        });
    }

    fn state(app: &App) -> TouchState {
        app.world().resource::<TouchState>().clone()
    }

    fn mode(app: &App) -> FlightMode {
        app.world().resource::<Flight>().mode
    }

    #[test]
    fn a_still_short_press_on_the_free_screen_is_a_tap() {
        let mut app = headless_app();
        app.update();
        touch(&mut app, TouchPhase::Started, 1, [640.0, 300.0]);
        app.update();
        assert!(state(&app).seen, "a touch shows automatic controls");
        touch(&mut app, TouchPhase::Ended, 1, [642.0, 301.0]);
        app.update();
        assert_eq!(state(&app).tap, Some([642.0, 301.0]));
        app.update();
        assert_eq!(state(&app).tap, None, "one frame only");
        // A drag is no tap.
        touch(&mut app, TouchPhase::Started, 2, [640.0, 300.0]);
        app.update();
        touch(&mut app, TouchPhase::Moved, 2, [700.0, 300.0]);
        app.update();
        assert_eq!(state(&app).look, [60.0, 0.0], "one finger drags the look");
        touch(&mut app, TouchPhase::Ended, 2, [700.0, 300.0]);
        app.update();
        assert_eq!(state(&app).tap, None);
    }

    #[test]
    fn two_fingers_pinch_apart_to_dive_in() {
        let mut app = headless_app();
        app.update();
        touch(&mut app, TouchPhase::Started, 1, [600.0, 360.0]);
        touch(&mut app, TouchPhase::Started, 2, [680.0, 360.0]);
        app.update();
        touch(&mut app, TouchPhase::Moved, 1, [560.0, 360.0]);
        touch(&mut app, TouchPhase::Moved, 2, [720.0, 360.0]);
        app.update();
        let pinch = state(&app).pinch;
        assert!(pinch < 0.0, "apart dives in: {pinch}");
        assert_eq!(state(&app).look, [0.0, 0.0], "a pinch does not look");
    }

    #[test]
    fn the_stick_and_buttons_take_their_fingers_in_normal_mode() {
        let mut app = headless_app();
        app.update();
        app.world_mut().resource_mut::<Flight>().mode = FlightMode::Normal;
        app.update();
        let layout = Layout::new(FALLBACK_SIZE[0], FALLBACK_SIZE[1], false);
        let centre = layout.stick;
        touch(&mut app, TouchPhase::Started, 1, centre);
        app.update();
        touch(
            &mut app,
            TouchPhase::Moved,
            1,
            [centre[0], centre[1] - universe_core::touch::STICK_RADIUS_PX],
        );
        app.update();
        let s = state(&app);
        assert!(s.stick.forward && s.stick.run, "pushed to the edge runs");
        assert_eq!(s.look, [0.0, 0.0], "the stick finger does not look");
        touch(
            &mut app,
            TouchPhase::Started,
            2,
            layout.button(TouchButton::Camera),
        );
        app.update();
        assert!(state(&app).pressed(TouchButton::Camera));
        assert!(state(&app).held(TouchButton::Camera));
        app.update();
        assert!(!state(&app).pressed(TouchButton::Camera), "pressed once");
        assert!(state(&app).held(TouchButton::Camera), "still held");
        // Left hand moves the stick to the other corner.
        app.world_mut().resource_mut::<SettingsStore>().0.left_hand = true;
        touch(&mut app, TouchPhase::Ended, 1, centre);
        app.update();
        assert_eq!(state(&app).stick, StickInput::default(), "lifted");
        touch(&mut app, TouchPhase::Started, 3, centre);
        app.update();
        assert!(
            state(&app)
                .fingers
                .iter()
                .flatten()
                .any(|f| f.id == 3 && f.role == Role::Free),
            "left hand: the bottom left is free screen"
        );
        assert_eq!(mode(&app), FlightMode::Normal);
    }

    #[test]
    fn a_key_press_hides_automatic_controls_again() {
        let mut app = headless_app();
        app.update();
        touch(&mut app, TouchPhase::Started, 1, [640.0, 300.0]);
        app.update();
        assert!(state(&app).seen);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyQ);
        app.update();
        assert!(!state(&app).seen);
    }
}
