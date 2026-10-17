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
use bevy::prelude::*;
use universe_core::body::{
    Body, BodyInput, RADIUS_SHARE, Turning, WalkEvent, body_height, body_step, eye_of,
    heading_basis, level_ground, resolve_collisions, walk_levels, yaw_of,
};
use universe_core::flight::free_speed_multiplier;
use universe_core::keymap::Action;
use universe_core::touch::TouchButton;

/// Third-person camera distance behind the eye, in body heights.
const THIRD_BACK: f64 = 3.0;

/// Third-person camera rise above the eye, in body heights.
const THIRD_UP: f64 = 0.6;

/// The body and its camera while normal mode runs.
#[derive(Resource, Debug, Clone, PartialEq)]
pub(crate) struct NormalState {
    /// The body, in the open cell's units.
    pub body: Body,
    /// Third person (true) or first person (false).
    pub third_person: bool,
    /// Whether the body was placed since normal mode was entered.
    pub active: bool,
    /// Strongest pull on the body this frame, m/s2 (#559).
    pub pull_strength: f64,
    /// Unit direction toward the strongest pull, open-cell units.
    pub pull_toward: [f64; 3],
    /// Name of the body pulling hardest.
    pub pull_name: String,
}

impl Default for NormalState {
    fn default() -> NormalState {
        NormalState {
            body: Body::standing([0.0; 3], 0.0),
            third_person: true,
            active: false,
            pull_strength: 0.0,
            pull_toward: [0.0; 3],
            pull_name: String::new(),
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
    let mut body = Body::standing(feet, yaw_of(up, forward));
    // With no ground under it the body starts flying, so the keys move it;
    // a double-tap lets it drift and fall under the real pull (#559).
    body.flying = ground.is_none();
    body
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
    buttons: Res<ButtonInput<MouseButton>>,
    mut universe: ResMut<Universe>,
    mut flight: ResMut<Flight>,
    mut nav: ResMut<Navigation>,
    mut store: ResMut<crate::SettingsStore>,
    mut state: ResMut<NormalState>,
    touch: Res<crate::touch::TouchState>,
    mut player: Option<ResMut<crate::player::Player>>,
) {
    if flight.mode != FlightMode::Normal {
        if state.active
            && let Some(player) = player.as_deref_mut()
        {
            // The body stays where you leave it: its new spot, saved (#607,
            // Q5 A, S2).
            player.leave(&universe, state.body.position, state.body.yaw, &mut store);
        }
        state.active = false;
        return;
    }
    if !state.active {
        let eye = universe.path.offset();
        state.body = match player.as_deref() {
            // Near the body, 1 takes it over where it stands (#607, Q7 A).
            Some(player) if player.spot.is_near(&universe.path) => {
                let mut body = Body::standing(player.spot.position, player.spot.yaw);
                body.flying = level_ground(&universe, player.spot.position).is_none();
                body
            }
            _ => place_body(&universe, eye, nav.forward),
        };
        state.third_person = store.0.start_third_person;
        state.active = true;
        flight.yaw = state.body.yaw;
        flight.pitch = 0.0;
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

/// Body outline proportions as shares of the body height: head centre and
/// radius, shoulder and hip heights, shoulder and hip half widths, hand
/// height and spread, and the facing tick length (#558).
const HEAD_AT: f64 = 0.88;
const HEAD_RADIUS: f64 = 0.065;
const SHOULDER_AT: f64 = 0.78;
const HIP_AT: f64 = 0.5;
const SHOULDER_HALF: f64 = 0.17;
const HAND_AT: f64 = 0.48;
const HAND_HALF: f64 = 0.2;
const FOOT_HALF: f64 = 0.09;
const FACING_TICK: f64 = 0.1;

/// Line segments of the outline person standing at `feet` with `up`,
/// facing `forward`, `height` tall (#558): spine, shoulders, two arms, two
/// legs, and a facing tick from the head. Fixed array, no allocation.
#[must_use]
pub(crate) fn body_segments(
    feet: [f64; 3],
    up: [f64; 3],
    forward: [f64; 3],
    right: [f64; 3],
    height: f64,
) -> [([f64; 3], [f64; 3]); 8] {
    let at = |rise: f64, side: f64, ahead: f64| {
        [
            feet[0] + (up[0] * rise + right[0] * side + forward[0] * ahead) * height,
            feet[1] + (up[1] * rise + right[1] * side + forward[1] * ahead) * height,
            feet[2] + (up[2] * rise + right[2] * side + forward[2] * ahead) * height,
        ]
    };
    [
        (at(HEAD_AT - HEAD_RADIUS, 0.0, 0.0), at(HIP_AT, 0.0, 0.0)),
        (
            at(SHOULDER_AT, -SHOULDER_HALF, 0.0),
            at(SHOULDER_AT, SHOULDER_HALF, 0.0),
        ),
        (
            at(SHOULDER_AT, -SHOULDER_HALF, 0.0),
            at(HAND_AT, -HAND_HALF, 0.0),
        ),
        (
            at(SHOULDER_AT, SHOULDER_HALF, 0.0),
            at(HAND_AT, HAND_HALF, 0.0),
        ),
        (at(HIP_AT, 0.0, 0.0), at(0.0, -FOOT_HALF, 0.0)),
        (at(HIP_AT, 0.0, 0.0), at(0.0, FOOT_HALF, 0.0)),
        (at(HIP_AT, -FOOT_HALF, 0.0), at(HIP_AT, FOOT_HALF, 0.0)),
        (at(HEAD_AT, 0.0, 0.0), at(HEAD_AT, 0.0, FACING_TICK)),
    ]
}

/// Draws the outline person in third person (#558, Q1: a marker figure).
///
/// Hidden in first person (you look through its eyes). Outside normal mode
/// the body waits at the player's spot and draws there while its cell is
/// open (#607, Q5 A). True size, drawn relative to the camera (ADR 0026).
pub(crate) fn draw_body(
    mut gizmos: Gizmos,
    universe: Res<Universe>,
    flight: Res<Flight>,
    state: Res<NormalState>,
    store: Res<crate::SettingsStore>,
    origin: Res<crate::style::RenderOrigin>,
    player: Option<Res<crate::player::Player>>,
) {
    if flight.mode != FlightMode::Normal || !state.active {
        if let Some(player) = player
            && player.spot.is_near(&universe.path)
        {
            let up =
                level_ground(&universe, player.spot.position).map_or([0.0, 1.0, 0.0], |g| g.up);
            let (forward, right) = heading_basis(up, player.spot.yaw);
            let height = body_height(universe.level());
            outline_person(
                &mut gizmos,
                &origin,
                player.spot.position,
                up,
                forward,
                right,
                height,
            );
        }
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
    if !state.third_person {
        return;
    }
    outline_person(
        &mut gizmos,
        &origin,
        state.body.position,
        up,
        forward,
        right,
        height,
    );
}

/// The outline person at `feet`: its segments and its head.
fn outline_person(
    gizmos: &mut Gizmos,
    origin: &crate::style::RenderOrigin,
    feet: [f64; 3],
    up: [f64; 3],
    forward: [f64; 3],
    right: [f64; 3],
    height: f64,
) {
    let color = crate::tokens::FOCUS_TINT;
    for (from, to) in body_segments(feet, up, forward, right, height) {
        gizmos.line(origin.at(from), origin.at(to), color);
    }
    let head = [
        feet[0] + up[0] * HEAD_AT * height,
        feet[1] + up[1] * HEAD_AT * height,
        feet[2] + up[2] * HEAD_AT * height,
    ];
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: body-scale radius narrowed for the GPU, intended"
    )]
    let radius = (HEAD_RADIUS * height) as f32;
    gizmos.sphere(Isometry3d::from_translation(origin.at(head)), radius, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_outline_person_stands_on_its_feet_at_true_height() {
        let feet = [0.1, 0.2, 0.3];
        let segments = body_segments(
            feet,
            [0.0, 1.0, 0.0],
            [0.0, 0.0, -1.0],
            [1.0, 0.0, 0.0],
            2.0,
        );
        let lowest = segments
            .iter()
            .flat_map(|(a, b)| [a[1], b[1]])
            .fold(f64::INFINITY, f64::min);
        let highest = segments
            .iter()
            .flat_map(|(a, b)| [a[1], b[1]])
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(
            (lowest - feet[1]).abs() < 1e-12,
            "the feet touch the ground"
        );
        assert!(highest < feet[1] + 2.0, "nothing above the head");
        assert!(highest > feet[1] + 1.5, "the figure is body tall");
        let (_, tip) = segments[7];
        assert!(tip[2] < feet[2], "the facing tick points forward");
    }
}
