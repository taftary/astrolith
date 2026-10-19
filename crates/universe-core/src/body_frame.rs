//! The body's local tangent frame near the ground (#779 Q2 A, Q3 A).
//!
//! Below [`LOCAL_BELOW_M`] above the ground the body steps in a
//! [`LocalFrame`](crate::planet_frame::LocalFrame): metres, origin a ground
//! point under the body, axes east, north and up. The frame re-anchors on
//! the ground under the body once it is [`REANCHOR_M`] from the origin.
//! Above that height, or with no ground, the body steps in open-cell units
//! as before. Position, velocity, pull and the ground are moved into the
//! frame and back; the heading turns by the angle between the two heading
//! references, so a yaw means the same direction in both. Gravity is the
//! pull toward the planet's centre, and the body stands along the ground's
//! (radial) up.
//!
//! Pure and deterministic (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use crate::body::{Body, BodyInput, Ground, Turning, body_step, body_step_scaled, heading_basis};
use crate::coords::Level;
use crate::planet_frame::{LocalFrame, LocalPos, PlanetFrame};

/// Height above the ground below which the body steps in its local frame.
pub const LOCAL_BELOW_M: f64 = 100.0;

/// Distance from the frame origin at which the frame re-anchors (metres).
pub const REANCHOR_M: f64 = 1_000.0;

/// The body's current local frame, if it is near the ground.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BodyAnchor(pub Option<LocalFrame>);

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// A planet-frame vector in the local axes.
fn to_local_vec(frame: &LocalFrame, v: [f64; 3]) -> [f64; 3] {
    [dot(v, frame.east), dot(v, frame.north), dot(v, frame.up)]
}

/// A local vector in the planet axes.
fn to_planet_vec(frame: &LocalFrame, v: [f64; 3]) -> [f64; 3] {
    [
        frame.east[0] * v[0] + frame.north[0] * v[1] + frame.up[0] * v[2],
        frame.east[1] * v[0] + frame.north[1] * v[1] + frame.up[1] * v[2],
        frame.east[2] * v[0] + frame.north[2] * v[1] + frame.up[2] * v[2],
    ]
}

/// Angle to add to a planet-frame yaw to get the same heading in the
/// local frame, at `up` (planet axes).
fn yaw_shift(frame: &LocalFrame, up: [f64; 3]) -> f64 {
    let (ahead_planet, _) = heading_basis(up, 0.0);
    let up_local = to_local_vec(frame, up);
    let (ahead_local, _) = heading_basis(up_local, 0.0);
    let side_local = cross(ahead_local, up_local);
    let a = to_local_vec(frame, ahead_planet);
    // `forward(yaw) = ahead cos(yaw) - side sin(yaw)`, so the planet's zero
    // heading sits at local yaw `atan2(-a · side, a · ahead)`.
    (-dot(a, side_local)).atan2(dot(a, ahead_local))
}

/// One body step (as [`body_step`]) that runs in the local tangent frame
/// while the body is less than [`LOCAL_BELOW_M`] above `ground`, keeping
/// `anchor` across steps. `frame` is the open planet's frame.
#[allow(
    clippy::too_many_arguments,
    reason = "one step's physical inputs plus the frame and its anchor"
)]
#[must_use]
pub fn step_near_ground(
    body: Body,
    input: BodyInput,
    level: Level,
    frame: &PlanetFrame,
    anchor: &mut BodyAnchor,
    ground: impl Fn([f64; 3]) -> Option<Ground>,
    pull: [f64; 3],
    turning: Turning,
    speed: f64,
    inertia: bool,
    camera_yaw: f64,
    third_person: bool,
    dt: f64,
) -> Body {
    let mpc = frame.metres_per_cell();
    let here = frame.from_cell(body.position);
    let under = ground(body.position);
    let height = under.map(|g| dot(sub(here.0, frame.from_cell(g.point).0), g.up) * 1.0);
    let Some(g) = under.filter(|_| {
        height.is_some_and(|h| h.is_finite() && h < LOCAL_BELOW_M) && mpc.is_finite() && mpc > 0.0
    }) else {
        anchor.0 = None;
        return body_step(
            body,
            input,
            level,
            ground,
            pull,
            turning,
            speed,
            inertia,
            camera_yaw,
            third_person,
            dt,
        );
    };
    // Anchor (or re-anchor) on the ground under the body.
    let ground_point = frame.from_cell(g.point);
    let far = anchor.0.is_none_or(|a| {
        let d = sub(here.0, a.origin.0);
        dot(d, d).sqrt() > REANCHOR_M
    });
    if far {
        anchor.0 = Some(LocalFrame::at(ground_point));
    }
    let Some(local) = anchor.0 else {
        return body;
    };
    let shift = yaw_shift(&local, g.up);
    let local_body = Body {
        position: local.to_local(here).0,
        velocity: to_local_vec(&local, scale(body.velocity, mpc)),
        yaw: body.yaw + shift,
        ..body
    };
    let local_ground = |point: [f64; 3]| {
        let planet = local.to_planet(LocalPos(point));
        ground(frame.to_cell(planet)).map(|g| Ground {
            point: local.to_local(frame.from_cell(g.point)).0,
            up: to_local_vec(&local, g.up),
        })
    };
    let stepped = body_step_scaled(
        local_body,
        input,
        1.0,
        local_ground,
        to_local_vec(&local, scale(pull, mpc)),
        turning,
        speed,
        inertia,
        camera_yaw + shift,
        third_person,
        dt,
    );
    Body {
        position: frame.to_cell(local.to_planet(LocalPos(stepped.position))),
        velocity: scale(to_planet_vec(&local, stepped.velocity), 1.0 / mpc),
        yaw: stepped.yaw - shift,
        ..stepped
    }
}
