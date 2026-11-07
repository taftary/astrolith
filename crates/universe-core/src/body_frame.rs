//! The body's local tangent frame near the ground (#779 Q2 A, Q3 A).
//!
//! Below [`LOCAL_BELOW_M`](crate::body_frame::LOCAL_BELOW_M) above the ground the body steps in a
//! [`LocalFrame`](crate::planet_frame::LocalFrame): metres, origin a ground
//! point under the body, axes east, north and up. The frame re-anchors on
//! the ground under the body once it is [`REANCHOR_M`](crate::body_frame::REANCHOR_M) from the origin.
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
use crate::frame_diagnostic::{
    CellDirection, FramePath, FrameStepDiagnostic, FrozenConversion, LocalDirection,
};
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
pub(crate) fn to_planet_vec(frame: &LocalFrame, v: [f64; 3]) -> [f64; 3] {
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
    step_near_ground_observed(
        body,
        input,
        level,
        frame,
        anchor,
        ground,
        pull,
        turning,
        speed,
        inertia,
        camera_yaw,
        third_person,
        dt,
        None,
    )
}

/// The same production step, optionally retaining its actual coordinate encodings.
/// Observations do not alter the step or compare different moments in time.
#[allow(
    clippy::too_many_arguments,
    reason = "E-NEWTYPE: existing physical step inputs plus optional read-only diagnostic output"
)]
#[must_use]
pub fn step_near_ground_observed(
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
    mut diagnostic: Option<&mut FrameStepDiagnostic>,
) -> Body {
    if let Some(out) = diagnostic.as_deref_mut() {
        *out = FrameStepDiagnostic {
            previous_anchor: anchor.0,
            ..Default::default()
        };
    }
    let mpc = frame.metres_per_cell();
    let here = frame.from_cell(body.position);
    let under = ground(body.position);
    let height = under.map(|g| dot(sub(here.0, frame.from_cell(g.point).0), g.up) * 1.0);
    let Some(g) = under.filter(|_| {
        height.is_some_and(|h| h.is_finite() && h < LOCAL_BELOW_M) && mpc.is_finite() && mpc > 0.0
    }) else {
        if let Some(out) = diagnostic {
            out.path = if anchor.0.is_some() {
                FramePath::Left
            } else {
                FramePath::Cell
            };
        }
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
    if let Some(out) = diagnostic.as_deref_mut() {
        out.path = match (out.previous_anchor, far) {
            (None, _) => FramePath::Entered,
            (Some(_), true) => FramePath::Reanchored,
            _ => FramePath::Retained,
        };
        out.anchor = Some(local);
        out.input = FrozenConversion::from_encoding(
            frame,
            &local,
            crate::coords::OpenUnits(body.position),
            LocalPos(local_body.position),
            CellDirection(heading_basis(g.up, body.yaw).0),
            LocalDirection(heading_basis(to_local_vec(&local, g.up), local_body.yaw).0),
        );
    }
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
    let result = Body {
        position: frame.to_cell(local.to_planet(LocalPos(stepped.position))),
        velocity: scale(to_planet_vec(&local, stepped.velocity), 1.0 / mpc),
        yaw: stepped.yaw - shift,
        ..stepped
    };
    if let Some(out) = diagnostic {
        out.output = FrozenConversion::from_encoding(
            frame,
            &local,
            crate::coords::OpenUnits(result.position),
            LocalPos(stepped.position),
            CellDirection(heading_basis(g.up, result.yaw).0),
            LocalDirection(heading_basis(to_local_vec(&local, g.up), stepped.yaw).0),
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_do_not_change_the_production_step_and_record_real_frame_switches() {
        let frame = PlanetFrame::for_radius_earth(1.0);
        let ground = |point: [f64; 3]| {
            let length = crate::nav::length3(point);
            if length <= 0.0 || !length.is_finite() {
                return None;
            }
            let up = point.map(|v| v / length);
            Some(Ground {
                point: up.map(|v| v * frame.radius_cells()),
                up,
            })
        };
        let mut ordinary = BodyAnchor::default();
        let mut observed = BodyAnchor::default();
        let mut body = Body::standing([frame.radius_cells(), 0.0, 0.0], 0.7);
        let mut trace = FrameStepDiagnostic::default();
        let radius = frame.radius_m();
        let reanchored = [1.0_f64, 0.001, 0.0];
        let length = crate::nav::length3(reanchored);
        for (position, expected) in [
            ([radius, 0.0, 0.0], FramePath::Entered),
            ([radius + 1.0, 0.0, 0.0], FramePath::Retained),
            (
                reanchored.map(|v| v * radius / length),
                FramePath::Reanchored,
            ),
            ([radius + 200.0, 0.0, 0.0], FramePath::Left),
        ] {
            body.position = frame.to_cell(crate::planet_frame::PlanetPos(position));
            let a = step_near_ground(
                body,
                BodyInput::default(),
                Level::MAX,
                &frame,
                &mut ordinary,
                ground,
                [0.0; 3],
                Turning::Drone,
                1.0,
                true,
                0.3,
                true,
                1.0 / 60.0,
            );
            let b = step_near_ground_observed(
                body,
                BodyInput::default(),
                Level::MAX,
                &frame,
                &mut observed,
                ground,
                [0.0; 3],
                Turning::Drone,
                1.0,
                true,
                0.3,
                true,
                1.0 / 60.0,
                Some(&mut trace),
            );
            assert_eq!(a, b);
            assert_eq!(ordinary, observed);
            assert_eq!(trace.path, expected);
            if expected != FramePath::Left {
                let input = trace.input.unwrap();
                assert!(input.cell_round_trip.0 < 1e-6);
                assert!(input.direction_error.0 < 1e-6);
            } else {
                assert!(trace.input.is_none() && trace.output.is_none());
            }
            body = a;
        }
    }
}
