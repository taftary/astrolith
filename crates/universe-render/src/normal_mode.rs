//! Normal mode: the player's body in the window (#558).
//!
//! Key 1 enters it. The body (`universe_core::body`) stands on the ground
//! under the camera, walks with W A S D, runs with Shift, jumps with Space,
//! takes off and lands on a Space double-tap, climbs and descends with
//! Space and C while flying, and turns drone or plane style (Flying turns
//! setting). Mouse look shares free flight's yaw and pitch. Buildings, room
//! walls and furniture are solid, and places open and close around the
//! body as it walks (`walk_levels`). V switches between first person
//! (through the eyes) and third person (behind and above); the Starting
//! camera setting picks the first one. The path offset is the body's eye,
//! so every level rule (opening, closing, previews) reads the body.

use crate::Universe;
use crate::input::{Flight, FlightMode, Navigation};
use bevy::prelude::*;
use universe_core::body::{
    Body, BodyInput, RADIUS_SHARE, Turning, WalkEvent, body_height, body_step, eye_of,
    heading_basis, level_ground, resolve_collisions, walk_levels, yaw_of,
};
use universe_core::flight::free_speed_multiplier;

/// Third-person camera distance behind the eye, in body heights.
const THIRD_BACK: f64 = 3.0;

/// Third-person camera rise above the eye, in body heights.
const THIRD_UP: f64 = 0.6;

/// The body and its camera while normal mode runs.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub(crate) struct NormalState {
    /// The body, in the open cell's units.
    pub body: Body,
    /// Third person (true) or first person (false).
    pub third_person: bool,
    /// Whether the body was placed since normal mode was entered.
    pub active: bool,
}

impl Default for NormalState {
    fn default() -> NormalState {
        NormalState {
            body: Body::standing([0.0; 3], 0.0),
            third_person: true,
            active: false,
        }
    }
}

/// Places the body under `eye`: on the ground when the level has one,
/// floating at `eye` otherwise, facing `forward` (#558).
#[must_use]
pub(crate) fn place_body(
    universe: &universe_core::nav::Universe,
    eye: [f64; 3],
    forward: [f64; 3],
) -> Body {
    let ground = level_ground(universe, eye);
    let up = ground.map_or([0.0, 1.0, 0.0], |g| g.up);
    let feet = ground.map_or(eye, |g| g.point);
    Body::standing(feet, yaw_of(up, forward))
}

/// Camera pose for the body: `(position, forward, up)` in open-cell units.
///
/// First person looks from the eye; third person sits [`THIRD_BACK`] body
/// heights behind and [`THIRD_UP`] above it, looking the same way. `pitch`
/// tilts the look toward the local up.
#[must_use]
pub(crate) fn camera_pose(
    universe: &universe_core::nav::Universe,
    state: &NormalState,
    pitch: f64,
) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let (eye, up) = eye_of(universe, &state.body);
    let (ahead, _) = heading_basis(up, state.body.yaw);
    let (sin, cos) = pitch.sin_cos();
    let forward = [
        ahead[0] * cos + up[0] * sin,
        ahead[1] * cos + up[1] * sin,
        ahead[2] * cos + up[2] * sin,
    ];
    if !state.third_person {
        return (eye, forward, up);
    }
    let size = body_height(universe.level());
    let position = [
        eye[0] - forward[0] * THIRD_BACK * size + up[0] * THIRD_UP * size,
        eye[1] - forward[1] * THIRD_BACK * size + up[1] * THIRD_UP * size,
        eye[2] - forward[2] * THIRD_BACK * size + up[2] * THIRD_UP * size,
    ];
    (position, forward, up)
}

/// Moves the body one frame while normal mode runs (#558).
///
/// Runs after `handle_input`, which owns the mode keys and mouse look.
/// Leaving normal mode parks the state; entering places the body under the
/// camera. Allocates nothing per frame (`E-HOT-NOALLOC`).
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn normal_mode_step(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut universe: ResMut<Universe>,
    mut flight: ResMut<Flight>,
    mut nav: ResMut<Navigation>,
    store: Res<crate::SettingsStore>,
    mut state: ResMut<NormalState>,
) {
    if flight.mode != FlightMode::Normal {
        state.active = false;
        return;
    }
    if !state.active {
        let eye = universe.path.offset();
        state.body = place_body(&universe, eye, nav.forward);
        state.third_person = store.0.start_third_person;
        state.active = true;
        flight.yaw = state.body.yaw;
        flight.pitch = 0.0;
    }
    if keys.just_pressed(KeyCode::KeyV) {
        state.third_person = !state.third_person;
    }
    let input = BodyInput {
        forward: keys.pressed(KeyCode::KeyW),
        back: keys.pressed(KeyCode::KeyS),
        left: keys.pressed(KeyCode::KeyA),
        right: keys.pressed(KeyCode::KeyD),
        fast: keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight),
        space_pressed: keys.just_pressed(KeyCode::Space),
        space_held: keys.pressed(KeyCode::Space),
        down_held: keys.pressed(KeyCode::KeyC),
    };
    let turning = if store.0.plane_turning {
        Turning::Plane
    } else {
        Turning::Drone
    };
    let level = universe.level();
    let dt = f64::from(time.delta_secs());
    // Mouse look owns the heading; plane banking turns it further.
    let mut body = state.body;
    body.yaw = flight.yaw;
    body = body_step(
        body,
        input,
        level,
        |point| level_ground(&universe, point),
        turning,
        free_speed_multiplier(store.0.speed.get()),
        dt,
    );
    let size = body_height(level);
    body.position = resolve_collisions(
        level,
        &universe.open,
        body.position,
        size,
        size * RADIUS_SHARE,
    );
    if walk_levels(&mut universe, &mut body) != WalkEvent::Stayed {
        // The new cell may arrive rotated: nothing to carry but the heading,
        // which `walk_levels` already mapped.
        nav.target = None;
    }
    flight.yaw = body.yaw;
    state.body = body;
}
