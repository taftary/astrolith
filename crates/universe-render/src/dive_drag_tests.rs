//! Dive drag direction tests (#1129, split from `control_tests` under
//! `E-FILE-SIZE`): the scene follows the hand on both axes.

use crate::SettingsStore;
use crate::Universe;
use crate::control_tests::{dive_distance, finger, send_motion};
use crate::window_tests::headless_app;
use bevy::prelude::*;

/// Camera displacement after a dive drag, along the drawn up and the
/// screen right of the view before it, in open-cell units.
fn drag_moves(app: &mut App, invert: bool, drag: impl FnOnce(&mut App)) -> (f64, f64) {
    use crate::input::Navigation;
    app.world_mut()
        .resource_mut::<SettingsStore>()
        .0
        .invert_look = invert;
    app.update();
    let (_, before, target) = dive_distance(app);
    let center = target.unwrap_or([0.0; 3]);
    let up = app.world().resource::<Navigation>().up;
    let f = [
        center[0] - before[0],
        center[1] - before[1],
        center[2] - before[2],
    ];
    // Screen right = forward x up.
    let right = [
        f[1] * up[2] - f[2] * up[1],
        f[2] * up[0] - f[0] * up[2],
        f[0] * up[1] - f[1] * up[0],
    ];
    drag(app);
    let after = app.world().resource::<Universe>().path.offset();
    let d = [
        after[0] - before[0],
        after[1] - before[1],
        after[2] - before[2],
    ];
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    (dot(d, up), dot(d, right))
}

/// A left-drag by `(dx, dy)` pixels past the click threshold.
fn left_drag(app: &mut App, dx: f32, dy: f32) {
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    // Past the click threshold first along the same direction.
    send_motion(app, dx, dy);
    app.update();
    send_motion(app, dx, dy);
    app.update();
    let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    buttons.release(MouseButton::Left);
    buttons.clear();
}

#[test]
fn dive_drag_up_and_right_move_the_scene_with_the_hand() {
    // #1129 AC1: a left-drag up moves the camera down around the target (the
    // scene goes up with the mouse); a drag right moves the camera left (the
    // scene goes right), as before.
    let mut app = headless_app();
    app.update();
    let (rise, side) = drag_moves(&mut app, false, |app| left_drag(app, 0.0, -60.0));
    assert!(rise < 0.0, "drag up moves the camera down: {rise}");
    assert!(side.abs() < rise.abs() * 0.1, "and not sideways: {side}");
    let (rise, side) = drag_moves(&mut app, false, |app| left_drag(app, 60.0, 0.0));
    assert!(side < 0.0, "drag right moves the camera left: {side}");
    assert!(rise.abs() < side.abs() * 0.1, "and not up or down: {rise}");
    let (rise, _) = drag_moves(&mut app, false, |app| left_drag(app, 0.0, 60.0));
    assert!(rise > 0.0, "drag down moves the camera up: {rise}");
}

#[test]
fn invert_look_flips_the_dive_drag_up_and_down() {
    // #1129 AC2: with Invert look on, a drag up moves the camera up; a drag
    // right still moves it left.
    let mut app = headless_app();
    app.update();
    let (rise, _) = drag_moves(&mut app, true, |app| left_drag(app, 0.0, -60.0));
    assert!(rise > 0.0, "inverted: drag up moves the camera up: {rise}");
    let (_, side) = drag_moves(&mut app, true, |app| left_drag(app, 60.0, 0.0));
    assert!(side < 0.0, "left/right unchanged: {side}");
}

#[test]
fn touch_drag_up_in_dive_moves_the_scene_with_the_finger() {
    // #1129 AC3: one finger moving up the screen moves the camera down
    // around the target, like the mouse.
    use bevy::input::touch::TouchPhase;
    let mut app = headless_app();
    app.update();
    let (rise, _) = drag_moves(&mut app, false, |app| {
        finger(app, TouchPhase::Started, 1, [640.0, 400.0]);
        app.update();
        finger(app, TouchPhase::Moved, 1, [640.0, 340.0]);
        app.update();
        finger(app, TouchPhase::Moved, 1, [640.0, 280.0]);
        app.update();
        finger(app, TouchPhase::Ended, 1, [640.0, 280.0]);
        app.update();
    });
    assert!(rise < 0.0, "finger up moves the camera down: {rise}");
}
