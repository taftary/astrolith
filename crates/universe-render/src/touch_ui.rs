//! The on-screen touch controls (#561 T3): the stick and the round buttons.
//!
//! UI nodes drawn from the core layout ([`universe_core::touch::Layout`]);
//! they take no clicks themselves ([`crate::touch`] hit-tests the same
//! layout). They show in the universe view when Touch controls is set to
//! always, or on automatic after a touch until the next key or mouse press
//! (owner answer Q6); Hand left swaps the sides (Q7 A). Down shows only
//! while flying and Autopilot only in dive mode. Everything here is
//! `pub(crate)`.

use crate::fonts::TextRole;
use crate::input::{Flight, FlightMode};
use crate::menu::Screen;
use crate::settings_ui::SettingsStore;
use crate::tokens::{LAYER_UNIVERSE, MENU_TEXT_TINT, TOUCH_EDGE, TOUCH_FILL, TOUCH_PRESSED_FILL};
use crate::touch::{Role, TouchState, shown};
use bevy::prelude::*;
use universe_core::touch::{BUTTON_RADIUS_PX, Layout, STICK_RADIUS_PX, TouchButton, thumb};

/// Thumb radius inside the stick base, logical px.
const THUMB_RADIUS_PX: f32 = 0.45 * STICK_RADIUS_PX;

/// Window size used when no window exists (headless tests).
const FALLBACK_SIZE: [f32; 2] = [1280.0, 720.0];

/// Touch input and the on-screen controls (#561): the state, the
/// controls, the Views page buttons, and forgetting fingers when a screen
/// opens. `read_touches` itself runs first in the `DivePlugin` input chain.
pub(crate) struct TouchPlugin;

impl Plugin for TouchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TouchState>()
            .add_message::<TouchInput>()
            .add_systems(Startup, spawn_touch_ui)
            .add_systems(
                Update,
                (
                    sync_touch_ui.after(crate::DiveSystems::Input),
                    crate::views_ui::view_buttons_click,
                ),
            )
            .add_systems(OnExit(Screen::Universe), crate::touch::clear_touches);
    }
}

/// Marks one on-screen touch control node.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TouchPart {
    /// The stick base.
    Stick,
    /// The stick thumb (child of the base).
    Thumb,
    /// One round button.
    Button(TouchButton),
}

/// A round node `radius` across its half-size.
fn round(radius: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        width: Val::Px(2.0 * radius),
        height: Val::Px(2.0 * radius),
        border: UiRect::all(Val::Px(2.0)),
        border_radius: BorderRadius::MAX,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        display: Display::None,
        ..default()
    }
}

/// Spawns the stick and the buttons, hidden until [`sync_touch_ui`] shows
/// them.
pub(crate) fn spawn_touch_ui(mut commands: Commands) {
    commands
        .spawn((
            TouchPart::Stick,
            Name::new("touch-stick"),
            GlobalZIndex(LAYER_UNIVERSE),
            round(STICK_RADIUS_PX),
            BackgroundColor(TOUCH_FILL),
            BorderColor::all(TOUCH_EDGE),
        ))
        .with_children(|base| {
            base.spawn((
                TouchPart::Thumb,
                Name::new("touch-thumb"),
                Node {
                    display: Display::Flex,
                    ..round(THUMB_RADIUS_PX)
                },
                BackgroundColor(TOUCH_PRESSED_FILL),
                BorderColor::all(TOUCH_EDGE),
            ));
        });
    for button in TouchButton::ALL {
        commands
            .spawn((
                TouchPart::Button(button),
                Name::new(format!("touch-{}", button.label())),
                GlobalZIndex(LAYER_UNIVERSE),
                round(BUTTON_RADIUS_PX),
                BackgroundColor(TOUCH_FILL),
                BorderColor::all(TOUCH_EDGE),
            ))
            .with_children(|face| {
                face.spawn((
                    Text::new(button.label()),
                    TextRole::Body,
                    TextColor(MENU_TEXT_TINT),
                ));
            });
    }
}

/// Whether the touch controls show now.
#[must_use]
pub(crate) fn controls_shown(store: &SettingsStore, touch: &TouchState, screen: Screen) -> bool {
    screen == Screen::Universe && (store.0.touch_always || touch.seen)
}

/// Places, shows, and hides the touch controls every frame.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn sync_touch_ui(
    store: Res<SettingsStore>,
    touch: Res<TouchState>,
    flight: Res<Flight>,
    normal: Res<crate::player_state::PlayerState>,
    screen: Res<State<Screen>>,
    windows: Query<&Window>,
    mut parts: Query<(&TouchPart, &mut Node, &mut BackgroundColor)>,
) {
    let window = windows.iter().next();
    let size = window.map_or(FALLBACK_SIZE, |w| [w.width(), w.height()]);
    let layout = Layout::new(size[0], size[1], store.0.left_hand);
    let on = controls_shown(&store, &touch, *screen.get());
    let flying = flight.mode == FlightMode::Normal && normal.active && normal.body.flying;
    let shown = shown(flight.mode, flying);
    let stick_finger = touch
        .fingers
        .iter()
        .flatten()
        .find(|finger| finger.role == Role::Stick)
        .map(|finger| {
            [
                finger.at[0] - layout.stick[0],
                finger.at[1] - layout.stick[1],
            ]
        });
    for (part, mut node, mut fill) in &mut parts {
        let (visible, centre, radius, pressed) = match *part {
            TouchPart::Stick => (on && shown.stick, layout.stick, STICK_RADIUS_PX, false),
            TouchPart::Thumb => {
                // Relative to the base: its centre plus the held offset.
                let offset = stick_finger.map_or([0.0, 0.0], thumb);
                let centre = [STICK_RADIUS_PX + offset[0], STICK_RADIUS_PX + offset[1]];
                (true, centre, THUMB_RADIUS_PX, stick_finger.is_some())
            }
            TouchPart::Button(button) => (
                on && shown.has(button),
                layout.button(button),
                BUTTON_RADIUS_PX,
                touch.held(button),
            ),
        };
        let display = if visible {
            Display::Flex
        } else {
            Display::None
        };
        let left = Val::Px(centre[0] - radius);
        let top = Val::Px(centre[1] - radius);
        if node.display != display {
            node.display = display;
        }
        if node.left != left {
            node.left = left;
        }
        if node.top != top {
            node.top = top;
        }
        let want = if pressed {
            TOUCH_PRESSED_FILL
        } else {
            TOUCH_FILL
        };
        if *part != TouchPart::Thumb && fill.0 != want {
            fill.0 = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_tests::headless_app;
    use bevy::input::touch::{TouchInput, TouchPhase};

    fn parts(app: &mut App) -> Vec<(TouchPart, Display, Val)> {
        app.world_mut()
            .query::<(&TouchPart, &Node)>()
            .iter(app.world())
            .map(|(part, node)| (*part, node.display, node.left))
            .collect()
    }

    fn showing(app: &mut App, want: TouchPart) -> bool {
        parts(app)
            .iter()
            .any(|(part, display, _)| *part == want && *display == Display::Flex)
    }

    #[test]
    fn automatic_shows_after_a_touch_and_hides_after_a_key() {
        let mut app = headless_app();
        app.update();
        let autopilot = TouchPart::Button(TouchButton::Autopilot);
        assert!(!showing(&mut app, autopilot), "hidden before any touch");
        app.world_mut().write_message(TouchInput {
            phase: TouchPhase::Started,
            position: Vec2::new(640.0, 300.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: 1,
        });
        app.update();
        app.update();
        assert!(
            showing(&mut app, autopilot),
            "a touch shows them (dive: Autopilot)"
        );
        assert!(
            !showing(&mut app, TouchPart::Stick),
            "no stick in dive mode"
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyQ);
        app.update();
        app.update();
        assert!(!showing(&mut app, autopilot), "a key hides them");
    }

    #[test]
    fn always_shows_the_mode_controls_and_hand_swaps_sides() {
        let mut app = headless_app();
        app.world_mut()
            .resource_mut::<SettingsStore>()
            .0
            .touch_always = true;
        app.world_mut().resource_mut::<Flight>().mode = FlightMode::Free;
        app.update();
        app.update();
        assert!(showing(&mut app, TouchPart::Stick), "free: the stick");
        assert!(!showing(&mut app, TouchPart::Button(TouchButton::Jump)));
        let left_of = |app: &mut App| {
            parts(app)
                .into_iter()
                .find(|(part, _, _)| *part == TouchPart::Stick)
                .map(|(_, _, left)| left)
        };
        let right_hand = left_of(&mut app);
        assert!(
            matches!(right_hand, Some(Val::Px(x)) if x < 640.0),
            "bottom left"
        );
        app.world_mut().resource_mut::<SettingsStore>().0.left_hand = true;
        app.update();
        let left_hand = left_of(&mut app);
        assert!(
            matches!(left_hand, Some(Val::Px(x)) if x > 640.0),
            "swapped: bottom right"
        );
        // A screen hides them.
        app.world_mut()
            .resource_mut::<NextState<Screen>>()
            .set(Screen::Settings);
        app.update();
        app.update();
        assert!(!showing(&mut app, TouchPart::Stick), "screens hide them");
    }
}
