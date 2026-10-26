//! Real-press time bar tests (#981 correction round 1): the press goes
//! through `ButtonInput` and the window cursor like a real click, with
//! Simulate touch on and off. Split from `time_bar` under `E-FILE-SIZE`.

use super::*;
use crate::window_tests::headless_app;
use bevy::math::Affine2;

/// Where the play button sits (no UI layout runs headless, so it gets its
/// box by hand).
const PLAY_AT: Vec2 = Vec2::new(640.0, 40.0);

/// A universe app with the play button laid out and the window cursor on
/// it.
fn app_with_cursor_on_play(simulate: bool) -> App {
    let mut app = headless_app();
    app.world_mut()
        .resource_mut::<crate::SettingsStore>()
        .0
        .simulate_touch = simulate;
    app.update();
    let play = app
        .world_mut()
        .query::<(Entity, &TimeButton)>()
        .iter(app.world())
        .find(|(_, which)| **which == TimeButton::Play)
        .map(|(entity, _)| entity)
        .expect("a play button");
    app.world_mut().entity_mut(play).insert((
        ComputedNode {
            size: Vec2::splat(MENU_BUTTON_DIAMETER_PX),
            ..ComputedNode::DEFAULT
        },
        UiGlobalTransform::from(Affine2::from_translation(PLAY_AT)),
    ));
    let mut window = Window::default();
    window.resolution.set(1280.0, 720.0);
    window.set_cursor_position(Some(PLAY_AT));
    app.world_mut().spawn(window);
    app.update();
    app
}

/// One left click: press for a frame, then release. `just_pressed` lasts
/// one frame like the window's input (the headless app never clears it).
fn click(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    buttons.clear();
    buttons.release(MouseButton::Left);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
    app.update();
}

#[test]
fn a_click_on_play_pauses_with_simulate_touch_on_and_off() {
    // #981 R1: with Simulate touch on, the bar's own claim filed the
    // finger as UI and the tap never pressed the button.
    for simulate in [true, false] {
        let mut app = app_with_cursor_on_play(simulate);
        assert!(app.world().resource::<SkyClock>().0.playing);
        click(&mut app);
        assert!(
            !app.world().resource::<SkyClock>().0.playing,
            "one click pauses (simulate touch {simulate})"
        );
        click(&mut app);
        assert!(
            app.world().resource::<SkyClock>().0.playing,
            "a second click plays again (simulate touch {simulate})"
        );
    }
}
