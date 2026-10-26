//! The time bar (#981 T4, Spec v2 goal 1): play or pause the sky, slower,
//! the speed, faster, and the home calendar readout, top middle.
//!
//! It shows in every mode and level and hides with the mode button's
//! Display switch (Display > Scene > Mode reminder). Presses resolve by
//! cursor math like the mode chooser (#955): a click or tap on a button is
//! the bar's, never the game's, through the shared
//! [`ModeSwitchClaim`](crate::mode_switch::ModeSwitchClaim). The keys
//! (T, [ and ], #981 S1) come from the saved key map. Pause stops only the
//! sky (Q4 A). Everything here is `pub(crate)`.

use crate::bindings::Actions;
use crate::chrome;
use crate::fonts::{Icon, TextRole};
use crate::sky::SkyClock;
use crate::tokens::{
    LAYER_UNIVERSE, MENU_BUTTON_DIAMETER_PX, MENU_BUTTON_GAP_PX, MENU_MARGIN_PX, MENU_TEXT_TINT,
    NARROW_BREAK_PX, TOUCH_FILL,
};
use bevy::prelude::*;
use bevy::ui::{Node, PositionType, Val, widget::Text};
use universe_core::clock::{readout, speed_label};
use universe_core::keymap::Action;

/// The home planet's calendar day in hours (Q7 A: the readout counts
/// Earth days).
const HOME_DAY_HOURS: f64 = 24.0;

/// Width of the speed plate, wide enough for `10,000×`.
const SPEED_PLATE_PX: f32 = 76.0;

/// One time bar button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimeButton {
    /// Play or pause the sky.
    Play,
    /// One speed step slower.
    Slower,
    /// One speed step faster.
    Faster,
}

/// The whole bar (its visibility follows the Display switch).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TimeBar;

/// One text the bar keeps up to date.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimeText {
    /// The play or pause glyph.
    Play,
    /// The speed between the buttons.
    Speed,
    /// The calendar readout.
    Readout,
}

/// What one press does to the clock.
pub(crate) fn press(clock: &mut SkyClock, button: TimeButton) {
    match button {
        TimeButton::Play => clock.0.toggle(),
        TimeButton::Slower => clock.0.slower(),
        TimeButton::Faster => clock.0.faster(),
    }
}

/// The glyph the play button shows: pause while playing, play while
/// paused (pure).
#[must_use]
pub(crate) fn play_icon(playing: bool) -> Icon {
    if playing { Icon::Pause } else { Icon::Resume }
}

/// A text plate: the bar's speed and readout sit on a soft fill so they
/// read over any sky.
fn plate(width: Val) -> impl Bundle {
    (
        Node {
            width,
            height: Val::Px(MENU_BUTTON_DIAMETER_PX),
            padding: UiRect::horizontal(Val::Px(MENU_BUTTON_GAP_PX)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(Val::Px(crate::tokens::BUTTON_RADIUS_PX)),
            ..default()
        },
        BackgroundColor(TOUCH_FILL),
    )
}

/// Spawns the bar, top middle.
pub(crate) fn spawn_time_bar(mut commands: Commands) {
    commands
        .spawn((
            TimeBar,
            Name::new("time-bar"),
            GlobalZIndex(LAYER_UNIVERSE),
            // A full-width row that only centres its content: it has no
            // `Interaction`, so presses beside the buttons reach the game.
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(MENU_MARGIN_PX),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(MENU_BUTTON_GAP_PX / 2.0),
                ..default()
            },
        ))
        .with_children(|bar| {
            let square = Val::Px(MENU_BUTTON_DIAMETER_PX);
            let button = |bar: &mut ChildSpawnerCommands, which: TimeButton, name: &str| {
                bar.spawn((
                    which,
                    Name::new(name.to_string()),
                    chrome::button(square, square, false),
                ))
                .with_children(|button| {
                    button
                        .spawn(chrome::button_face(false))
                        .with_children(|face| match which {
                            TimeButton::Play => {
                                let (text, role) = play_icon(true).text();
                                face.spawn((TimeText::Play, text, role, TextColor(MENU_TEXT_TINT)));
                            }
                            TimeButton::Slower => {
                                face.spawn((
                                    Text::new("÷"),
                                    TextRole::Title,
                                    TextColor(MENU_TEXT_TINT),
                                ));
                            }
                            TimeButton::Faster => {
                                face.spawn((
                                    Text::new("×"),
                                    TextRole::Title,
                                    TextColor(MENU_TEXT_TINT),
                                ));
                            }
                        });
                });
            };
            button(bar, TimeButton::Play, "time-play");
            button(bar, TimeButton::Slower, "time-slower");
            bar.spawn(plate(Val::Px(SPEED_PLATE_PX)))
                .with_children(|plate| {
                    plate.spawn((
                        TimeText::Speed,
                        Text::new(speed_label(1.0)),
                        TextRole::Body,
                        TextColor(MENU_TEXT_TINT),
                    ));
                });
            button(bar, TimeButton::Faster, "time-faster");
            bar.spawn(plate(Val::Auto)).with_children(|plate| {
                plate.spawn((
                    TimeText::Readout,
                    Text::new(readout(0.0, HOME_DAY_HOURS)),
                    TextRole::Body,
                    TextColor(MENU_TEXT_TINT),
                ));
            });
        });
}

/// The bar's top edge: beside the corner buttons, or on a second row
/// under them on a narrow (phone) window so they never overlap (pure).
#[must_use]
pub(crate) fn bar_top_px(window_width: f32) -> f32 {
    if window_width < NARROW_BREAK_PX {
        MENU_MARGIN_PX + MENU_BUTTON_DIAMETER_PX + MENU_BUTTON_GAP_PX
    } else {
        MENU_MARGIN_PX
    }
}

/// Refreshes the bar: hidden with the Display switch, placed for the
/// window's width, the play glyph, the speed and the readout. Writes only
/// on a change.
pub(crate) fn sync_time_bar(
    store: Res<crate::SettingsStore>,
    clock: Res<SkyClock>,
    windows: Query<&Window>,
    mut bar: Query<(&mut Visibility, &mut Node), With<TimeBar>>,
    mut texts: Query<(&TimeText, &mut Text)>,
) {
    let want = if store.0.mode_reminder {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let top = Val::Px(bar_top_px(
        windows.iter().next().map_or(f32::MAX, Window::width),
    ));
    for (mut visibility, mut node) in &mut bar {
        if *visibility != want {
            *visibility = want;
        }
        if node.top != top {
            node.top = top;
        }
    }
    for (which, mut text) in &mut texts {
        let want = match which {
            TimeText::Play => play_icon(clock.0.playing).glyph().to_string(),
            TimeText::Speed => speed_label(clock.0.speed()),
            TimeText::Readout => readout(clock.0.game_s, HOME_DAY_HOURS),
        };
        if text.0 != want {
            text.0 = want;
        }
    }
}

/// Presses the bar's buttons by cursor math (mouse) or a touch tap, and
/// the time keys. A press on a button is claimed, so the game does not
/// pick behind it. Runs after the mode chooser's click, which clears the
/// claim each frame.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn time_bar_click(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    touch: Res<crate::touch::TouchState>,
    store: Res<crate::SettingsStore>,
    nodes: Query<(&TimeButton, &ComputedNode, &UiGlobalTransform)>,
    mut clock: ResMut<SkyClock>,
    mut claim: ResMut<crate::mode_switch::ModeSwitchClaim>,
) {
    let actions = Actions {
        map: store.0.keymap,
        keys: &keys,
        buttons: &buttons,
    };
    for (action, which) in [
        (Action::TimePlay, TimeButton::Play),
        (Action::TimeSlower, TimeButton::Slower),
        (Action::TimeFaster, TimeButton::Faster),
    ] {
        if actions.just_pressed(action) {
            press(&mut clock, which);
        }
    }
    if !store.0.mode_reminder {
        return;
    }
    let window = windows.iter().next();
    let scale = window.map_or(1.0, Window::scale_factor);
    let hit = |at: [f32; 2]| {
        let point = Vec2::new(at[0], at[1]) * scale;
        nodes
            .iter()
            .find(|(_, node, transform)| node.contains_point(**transform, point))
            .map(|(which, _, _)| *which)
    };
    // A mouse press on a button is the bar's. With Simulate touch the left
    // button is the finger: its press only claims, its tap presses.
    if buttons.just_pressed(MouseButton::Left)
        && let Some(which) = window
            .and_then(Window::cursor_position)
            .and_then(|cursor| hit([cursor.x, cursor.y]))
    {
        claim.0 = true;
        if !store.0.simulate_touch {
            press(&mut clock, which);
        }
    }
    if let Some(which) = touch.time_tap.and_then(hit) {
        claim.0 = true;
        press(&mut clock, which);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presses_step_the_clock() {
        // #981 Q3 A, Q4 A.
        let mut clock = SkyClock::default();
        assert!(clock.0.playing, "the game starts playing");
        assert_eq!(play_icon(clock.0.playing), Icon::Pause);
        press(&mut clock, TimeButton::Play);
        assert!(!clock.0.playing);
        assert_eq!(play_icon(clock.0.playing), Icon::Resume);
        press(&mut clock, TimeButton::Faster);
        press(&mut clock, TimeButton::Faster);
        assert_eq!(speed_label(clock.0.speed()), "100×");
        press(&mut clock, TimeButton::Slower);
        assert_eq!(speed_label(clock.0.speed()), "10×");
    }

    #[test]
    fn a_phone_window_puts_the_bar_under_the_corner_buttons() {
        // #981 Q8 A: tap-friendly on touch, never over the corner buttons.
        assert_eq!(bar_top_px(1280.0), MENU_MARGIN_PX);
        assert!(bar_top_px(400.0) >= MENU_MARGIN_PX + MENU_BUTTON_DIAMETER_PX);
    }
}
