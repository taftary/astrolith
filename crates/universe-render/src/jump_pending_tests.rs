//! A jump made while the ground under the body is still being built is
//! kept when the ground becomes ready (#1191 Spec v1, owner answer A):
//! only the height is corrected. Standing still, the body is placed on the
//! ground as before.
//!
//! The real moment (the ground build landing on the frame after the jump)
//! depends on background threads, so these tests make it exact: they stand
//! on the built ground, jump, then mark the ground as just arriving
//! (`ground_pending`), as the #1158 probe did.

use crate::Universe;
use crate::control_tests::opened;
use crate::player_state::PlayerState;
use bevy::prelude::*;

/// The player standing on the built ground at L10.
fn standing() -> App {
    let mut app = opened(9);
    crate::player::player_here(&mut app);
    press(&mut app, KeyCode::Digit1);
    release(&mut app, KeyCode::Digit1);
    app.update();
    crate::lib_tests::standing_on_built_ground(&mut app);
    assert!(
        app.world().resource::<PlayerState>().body.on_ground,
        "starts standing"
    );
    app
}

fn press(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
}

fn release(app: &mut App, key: KeyCode) {
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    input.release(key);
    input.clear();
}

/// The ground under the body: its point and up.
fn ground(app: &App) -> universe_core::body::Ground {
    let world = app.world();
    let position = world.resource::<PlayerState>().body.position;
    crate::patches::ground_under(
        world.get_resource::<crate::patches::GroundSurface>(),
        world.resource::<Universe>(),
        position,
    )
    .expect("ground under the body")
}

/// Height of the feet above the ground, and the speed along its up.
fn height_and_climb(app: &App) -> (f64, f64) {
    let g = ground(app);
    let body = app.world().resource::<PlayerState>().body;
    let along = |v: [f64; 3]| v[0] * g.up[0] + v[1] * g.up[1] + v[2] * g.up[2];
    let rel = [
        body.position[0] - g.point[0],
        body.position[1] - g.point[1],
        body.position[2] - g.point[2],
    ];
    (along(rel), along(body.velocity))
}

/// Jumps (Space for one frame) and then marks the ground as just arriving.
fn jump_then_ground_arrives(app: &mut App) {
    press(app, KeyCode::Space);
    release(app, KeyCode::Space);
    let (_, climb) = height_and_climb(app);
    assert!(climb > 0.0, "the jump starts upward: {climb}");
    app.world_mut().resource_mut::<PlayerState>().ground_pending = true;
}

#[test]
fn jump_continues_when_the_ground_becomes_ready() {
    // AC1: the jump keeps going, not put back on the ground.
    let mut app = standing();
    jump_then_ground_arrives(&mut app);
    app.update();
    let state = app.world().resource::<PlayerState>();
    assert!(!state.ground_pending, "the ground was taken");
    assert!(!state.body.on_ground, "the jump is not cancelled");
    let (height, climb) = height_and_climb(&app);
    assert!(height > 0.0, "still in the air: {height}");
    assert!(climb > 0.0, "still rising: {climb}");
}

#[test]
fn jump_below_the_real_ground_is_raised_and_kept() {
    // AC2: never left below the real ground; the jump continues from it.
    let mut app = standing();
    jump_then_ground_arrives(&mut app);
    let g = ground(&app);
    let metre = universe_core::body::metre(app.world().resource::<Universe>().level());
    {
        let mut state = app.world_mut().resource_mut::<PlayerState>();
        state.body.position = [
            g.point[0] - g.up[0] * 0.5 * metre,
            g.point[1] - g.up[1] * 0.5 * metre,
            g.point[2] - g.up[2] * 0.5 * metre,
        ];
    }
    app.update();
    let (height, climb) = height_and_climb(&app);
    assert!(height >= -1e-9, "not below the ground: {height}");
    assert!(climb > 0.0, "the jump continues: {climb}");
    assert!(
        !app.world().resource::<PlayerState>().body.on_ground,
        "still jumping"
    );
}

#[test]
fn standing_is_placed_on_the_ground_as_before() {
    // AC3: standing still, the body is placed on the ground.
    let mut app = standing();
    app.world_mut().resource_mut::<PlayerState>().ground_pending = true;
    app.update();
    let state = app.world().resource::<PlayerState>();
    assert!(!state.ground_pending, "the ground was taken");
    assert!(!state.body.flying, "standing, not flying");
    let (height, _) = height_and_climb(&app);
    let metre = universe_core::body::metre(app.world().resource::<Universe>().level());
    assert!(height.abs() < 0.01 * metre, "on the ground: {height}");
}
