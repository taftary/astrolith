//! Normal mode: the player's body in the window (#558).
//!
//! Key 1 enters it. The body (`universe_core::body`) stands on the ground
//! under the camera, walks with W A S D, runs with Shift, jumps with Space,
//! takes off and lands on a Space double-tap, climbs and descends with
//! Space and C while flying, and turns drone or plane style (Flying turns
//! setting). Mouse look shares free flight's yaw and pitch. The body walks
//! on the planet ground under the camera. V switches between first person
//! (through the eyes) and third person (behind and above); the Starting
//! camera setting picks the first one. The path offset is the body's eye,
//! so every level rule (opening, closing, previews) reads the body.

use crate::Universe;
use crate::input::{Flight, FlightMode, Navigation};
use crate::player_state::PlayerState;
use bevy::prelude::*;
use universe_core::body::{
    Body, BodyInput, RADIUS_SHARE, Turning, WalkEvent, body_height, body_step, eye_of,
    heading_basis, level_ground, resolve_collisions, walk_levels, yaw_of,
};
use universe_core::flight::free_speed_multiplier;
use universe_core::keymap::Action;
use universe_core::touch::TouchButton;

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
    let mut body = Body::standing(feet, yaw_of(up, forward));
    // With no ground under it the body starts flying, so the keys move it;
    // a double-tap lets it drift and fall under the real pull (#559).
    body.flying = ground.is_none();
    body
}

/// Moves the body one frame while normal mode runs (#558).
///
/// Runs with [`crate::player_state::enter_player`], which owns placing the
/// body; leaving and following live there too (leave, follow, enter order,
/// #754). The handover ease advances here. Allocates nothing per frame
/// (`E-HOT-NOALLOC`).
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn normal_mode_step(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut universe: ResMut<Universe>,
    mut flight: ResMut<Flight>,
    mut nav: ResMut<Navigation>,
    store: Res<crate::SettingsStore>,
    mut state: ResMut<PlayerState>,
    touch: Res<crate::touch::TouchState>,
) {
    if flight.mode != FlightMode::Normal || !state.active {
        return;
    }
    // The handover ease advances here; entering sets it to 0, so the first
    // body frame still matches the arrival (#754 AC2).
    if state.handover_from.is_some() {
        let dt = f64::from(time.delta_secs());
        state.handover_t += dt;
        if state.handover_t >= crate::player_camera::HANDOVER_S {
            state.handover_from = None;
            state.handover_t = 0.0;
        }
    }
    // Every key comes from the saved key map (#560).
    let actions = crate::bindings::Actions {
        map: store.0.keymap,
        keys: &keys,
        buttons: &buttons,
    };
    if actions.just_pressed(Action::Camera) || touch.pressed(TouchButton::Camera) {
        state.third_person = !state.third_person;
    }
    let input = BodyInput {
        forward: actions.pressed(Action::Forward) || touch.stick.forward,
        back: actions.pressed(Action::Back) || touch.stick.back,
        left: actions.pressed(Action::Left) || touch.stick.left,
        right: actions.pressed(Action::Right) || touch.stick.right,
        fast: actions.pressed(Action::Run) || touch.stick.run,
        space_pressed: actions.just_pressed(Action::SpaceAction)
            || touch.pressed(TouchButton::Jump),
        space_held: actions.pressed(Action::SpaceAction) || touch.held(TouchButton::Jump),
        down_held: actions.pressed(Action::Descend) || touch.held(TouchButton::Down),
    };
    let turning = if store.0.plane_turning {
        Turning::Plane
    } else {
        Turning::Drone
    };
    let level = universe.level();
    let dt = f64::from(time.delta_secs());
    // The real pull of every massive body (#559), kept for the arrow and
    // the readout line.
    let pull = universe_core::gravity::pull(&universe, state.body.position);
    state.pull_strength = pull.strongest;
    state.pull_toward = pull.toward;
    if state.pull_name != pull.name {
        state.pull_name.clone_from(&pull.name);
    }
    // Mouse look owns the heading; plane banking turns it further.
    let mut body = state.body;
    body.yaw = flight.yaw;
    body = body_step(
        body,
        input,
        level,
        |point| level_ground(&universe, point),
        pull.accel,
        turning,
        free_speed_multiplier(store.0.speed.get()),
        store.0.movement_inertia,
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

/// Pull arrow length in body heights (#559).
const PULL_ARROW: f64 = 0.8;
/// Pull arrow brightness (context, under the actionable floor).
const PULL_BRIGHTNESS: f32 = 0.45;

fn add3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale3(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// Draws the pull arrow in normal mode (#559, Q8 A).
///
/// The figure itself lives in `figure_draw`: this system keeps only the pull
/// arrow beside the body (third person) or in front of the eyes (first
/// person). Outside normal mode it draws nothing; the waiting figure draws
/// through the same figure code.
pub(crate) fn draw_body(
    mut gizmos: Gizmos,
    universe: Res<Universe>,
    flight: Res<Flight>,
    state: Res<PlayerState>,
    store: Res<crate::SettingsStore>,
    origin: Res<crate::style::RenderOrigin>,
) {
    if flight.mode != FlightMode::Normal || !state.active {
        return;
    }
    let up = level_ground(&universe, state.body.position).map_or([0.0, 1.0, 0.0], |g| g.up);
    let (forward, right) = heading_basis(up, state.body.yaw);
    let height = body_height(universe.level());
    if store.0.show_pull && state.pull_strength >= universe_core::gravity::FAINT_PULL {
        // A faint arrow toward the strongest pull (#559): beside the body
        // at shoulder height in third person (so a pull straight down stays
        // above the ground), in front of the eyes in first person.
        let start = if state.third_person {
            add3(
                state.body.position,
                add3(scale3(up, 0.9 * height), scale3(right, 0.45 * height)),
            )
        } else {
            add3(
                eye_of(&universe, &state.body).0,
                scale3(forward, 2.0 * height),
            )
        };
        let end = add3(start, scale3(state.pull_toward, PULL_ARROW * height));
        gizmos.arrow(
            origin.at(start),
            origin.at(end),
            crate::style::scaled(crate::tokens::FOCUS_TINT, PULL_BRIGHTNESS),
        );
    }
}
