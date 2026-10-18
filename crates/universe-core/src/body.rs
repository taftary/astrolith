//! The player's body in normal mode (#558): walk, jump, fly, turn.
//!
//! Pure and deterministic (`E-CORE-NO-BEVY`, `E-DET-TIERS`): one
//! [`body_step`](crate::body::body_step) advances a [`Body`](crate::body::Body) by `dt` seconds from a [`BodyInput`](crate::body::BodyInput)
//! and a [`Ground`](crate::body::Ground) reader. The body is true size, about
//! [`BODY_HEIGHT_M`](crate::body::BODY_HEIGHT_M) tall in every level's own units. Its walking, running
//! and flying speeds are real metres per second, the same at every level,
//! so walking into a place keeps the same pace (#558 correction round 1,
//! Spec v3). Vertical motion under gravity (jumps, falls) also uses real
//! metres and seconds, scaled into the level's units, so a jump looks the
//! same at every level.
//!
//! Gravity is real (#559): [`crate::gravity::pull`] sums the pull of every
//! massive body, and the body falls under it with its momentum, drifts far
//! from everything, and falls around a body when moving fast enough
//! sideways. The ground ([`level_ground`](crate::body::level_ground))
//! exists on the whole planet (L10).

use crate::coords::Level;
use crate::motion::{
    BODY_TURN_RATE, FLY_S, RUN_S, WALK_DOWN_S, WALK_UP_S, approach, inertia_rate, turn_toward,
};
use crate::nest::scale_exponent;

/// Body height in metres (eye height is [`EYE_SHARE`] of it).
pub const BODY_HEIGHT_M: f64 = 1.8;

/// Eye height as a share of the body height.
pub const EYE_SHARE: f64 = 0.93;

/// Body radius as a share of the body height (shoulder half width).
pub const RADIUS_SHARE: f64 = 0.15;

/// Walking speed in metres per second at the default speed step.
pub const WALK_M_PER_S: f64 = 1.4;

/// Running speed (Shift held while walking), metres per second.
pub const RUN_M_PER_S: f64 = 4.0;

/// Flying speed in metres per second at the default speed step.
pub const FLY_M_PER_S: f64 = 10.0;

/// Boosted flying speed (Shift held while flying), metres per second.
pub const BOOST_M_PER_S: f64 = 30.0;

/// Gravity in metres per second squared (scaled into level units).
pub const GRAVITY_M: f64 = 9.81;

/// Jump take-off speed in metres per second (about half a metre high).
pub const JUMP_M_PER_S: f64 = 3.2;

/// Longest gap between two Space presses that counts as a double-tap.
pub const DOUBLE_TAP_S: f64 = 0.35;

/// Plane-style roll rate in radians per second, and the largest bank.
pub const ROLL_RATE: f64 = 1.5;
/// Largest plane-style bank, radians.
pub const ROLL_LIMIT: f64 = 1.0;
/// Plane-style turn rate per unit sine of bank, radians per second.
pub const BANK_TURN_RATE: f64 = 1.2;

/// How flying turns (owner answer C, setting S1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Turning {
    /// Turn and nose up and down only; A and D slide sideways.
    #[default]
    Drone,
    /// A and D roll, and the bank turns the heading.
    Plane,
}

/// Keys held or pressed this frame for the body.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BodyInput {
    /// W held.
    pub forward: bool,
    /// S held.
    pub back: bool,
    /// A held.
    pub left: bool,
    /// D held.
    pub right: bool,
    /// Shift held: run or boost.
    pub fast: bool,
    /// Space pressed this frame.
    pub space_pressed: bool,
    /// Space held (climb while flying).
    pub space_held: bool,
    /// C held (descend while flying).
    pub down_held: bool,
}

/// The ground under the body: its height along `up` and the `up` itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ground {
    /// The ground point under the body, in open-cell units.
    pub point: [f64; 3],
    /// Unit vector pointing away from the ground (against gravity).
    pub up: [f64; 3],
}

/// The body's state in open-cell units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body {
    /// Feet position, open-cell units.
    pub position: [f64; 3],
    /// Velocity in open-cell units per second while in the air (jumps,
    /// falls, drifts, orbits, and the flying velocity). Walking eases it
    /// along the ground with inertia on and rests it at zero without it
    /// (#559, #756).
    pub velocity: [f64; 3],
    /// Heading around the local up, radians.
    pub yaw: f64,
    /// Plane-style bank, radians (zero for drone turning and walking).
    pub roll: f64,
    /// Flying (true) or under gravity (false).
    pub flying: bool,
    /// Standing on the ground this step.
    pub on_ground: bool,
    /// Seconds since the last Space press, for the double-tap.
    pub since_space: f64,
    /// A Space press was seen; the next press counts only after a release,
    /// so holding Space never toggles again (#756 C5).
    pub awaiting_release: bool,
}

impl Body {
    /// A body standing at `position` with heading `yaw`.
    #[must_use]
    pub fn standing(position: [f64; 3], yaw: f64) -> Body {
        Body {
            position,
            velocity: [0.0; 3],
            yaw,
            roll: 0.0,
            flying: false,
            on_ground: false,
            since_space: f64::INFINITY,
            awaiting_release: false,
        }
    }
}

/// One metre in `level`'s open-cell units (a cell spans `10^e` metres).
#[must_use]
pub fn metre(level: Level) -> f64 {
    10f64.powf(-scale_exponent(level))
}

/// Body height in `level`'s open-cell units.
///
/// Always 1.8 m: `metre` carries the level scale, so the body keeps one
/// true size everywhere (#742).
#[must_use]
pub fn body_height(level: Level) -> f64 {
    BODY_HEIGHT_M * metre(level)
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let length = dot(a, a).sqrt();
    (length.is_finite() && length > 0.0).then(|| scale(a, 1.0 / length))
}

/// Forward and right on the plane perpendicular to `up` for heading `yaw`.
///
/// Heading zero faces the projection of `-Z` (or of `+X` when `up` is
/// along `Z`); positive yaw turns left, as in free flight. Pure.
#[must_use]
pub fn heading_basis(up: [f64; 3], yaw: f64) -> ([f64; 3], [f64; 3]) {
    let up = unit(up).unwrap_or([0.0, 1.0, 0.0]);
    let reference = if dot(up, [0.0, 0.0, 1.0]).abs() > 0.99 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 0.0, -1.0]
    };
    let ahead = unit(sub(reference, scale(up, dot(reference, up)))).unwrap_or([1.0, 0.0, 0.0]);
    let side = cross(ahead, up);
    let (sin, cos) = yaw.sin_cos();
    let forward = add(scale(ahead, cos), scale(side, -sin));
    let right = cross(forward, up);
    (forward, right)
}

/// The ground under `position` in the open cell of `universe` (#558).
///
/// The whole planet (L10) pulls toward its centre onto its flat faces
/// (`terrain::surface_radius`, #593). `None` elsewhere.
#[must_use]
pub fn level_ground(universe: &crate::nav::Universe, position: [f64; 3]) -> Option<Ground> {
    match universe.level().get() {
        10 => {
            let up = unit(position)?;
            let radius = crate::terrain::surface_radius(up);
            Some(Ground {
                point: scale(up, radius),
                up,
            })
        }
        _ => None,
    }
}

/// Heading of `forward` around `up`, the inverse of [`heading_basis`].
#[must_use]
pub fn yaw_of(up: [f64; 3], forward: [f64; 3]) -> f64 {
    let (ahead, right) = heading_basis(up, 0.0);
    // `heading_basis` turns left with yaw: forward = ahead cos - right sin,
    // with `right` taken at heading zero.
    let yaw = (-dot(forward, right)).atan2(dot(forward, ahead));
    // `-0.0` reads as `0.0`, so headings compare by value.
    if yaw == 0.0 { 0.0 } else { yaw }
}

/// Eye height as a share of the body height, in `level` units.
#[must_use]
pub fn eye_height(level: Level) -> f64 {
    body_height(level) * EYE_SHARE
}

/// The body's eye point and its up, in the open cell's units.
#[must_use]
pub fn eye_of(universe: &crate::nav::Universe, body: &Body) -> ([f64; 3], [f64; 3]) {
    let up = level_ground(universe, body.position).map_or([0.0, 1.0, 0.0], |g| g.up);
    (
        add(body.position, scale(up, eye_height(universe.level()))),
        up,
    )
}

/// What [`walk_levels`] did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WalkEvent {
    /// Same cell as before.
    Stayed,
    /// The portal `marker` opened around the body.
    Opened(u32),
    /// The open cell closed back into its parent.
    Closed,
}

/// Opens and closes cells around a walking or flying body (#558, Q8).
///
/// The body's eye is the camera for the open and close rules of the dive:
/// the nearest portal opens once it reaches [`crate::nest::OPEN_ANGLE`]
/// from the eye ([`crate::nav::should_open`]), and the open cell closes on
/// [`crate::nav::should_close`]. Crossing a frame carries the feet, the
/// heading, and the velocity with the frame's own map, so nothing
/// jumps. Sets the path offset to the eye either way.
pub fn walk_levels(universe: &mut crate::nav::Universe, body: &mut Body) -> WalkEvent {
    use crate::nav::{MarkerIndex, should_close, should_open};
    let (eye, up) = eye_of(universe, body);
    universe.path.set_offset(eye);
    let forward = heading_basis(up, body.yaw).0;
    if universe.path.can_open() {
        let radius = universe.marker_radius();
        let nearest = universe
            .open
            .points
            .iter()
            .enumerate()
            .filter(|(_, point)| point.kind == crate::r#gen::MarkerKind::Portal)
            .map(|(index, point)| {
                let to = sub(point.portal_position(), eye);
                (index, dot(to, to).sqrt())
            })
            .filter(|(_, distance)| should_open(radius, *distance))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((index, _)) = nearest
            && let Ok(marker) = u32::try_from(index)
            && universe.open(MarkerIndex(marker))
            && let Some(entered) = universe.path.entered()
        {
            body.position = entered.to_child_point(body.position);
            body.velocity = scale(
                entered.to_child_direction(body.velocity),
                1.0 / entered.ratio,
            );
            let child_forward = entered.to_child_direction(forward);
            let child_up = level_ground(universe, body.position).map_or([0.0, 1.0, 0.0], |g| g.up);
            body.yaw = yaw_of(child_up, child_forward);
            universe.path.set_offset(eye_of(universe, body).0);
            return WalkEvent::Opened(marker);
        }
    }
    if should_close(universe.path.distance_to_center())
        && let Some(left) = universe.close()
    {
        body.position = left.to_parent_point(body.position);
        body.velocity = scale(left.to_parent_direction(body.velocity), left.ratio);
        let parent_forward = left.to_parent_direction(forward);
        let parent_up = level_ground(universe, body.position).map_or([0.0, 1.0, 0.0], |g| g.up);
        body.yaw = yaw_of(parent_up, parent_forward);
        universe.path.set_offset(eye_of(universe, body).0);
        return WalkEvent::Closed;
    }
    WalkEvent::Stayed
}

/// Pushes a body at `feet` out of the solid things of the open cell (#558).
///
/// The body is an upright cylinder `height` tall and `radius` wide. Below
/// L10 nothing opens, so no level holds solid things: the body passes
/// through. Overlaps would resolve along the shallower horizontal axis.
/// Pure; returns `feet` untouched.
#[must_use]
pub fn resolve_collisions(
    level: Level,
    open: &crate::r#gen::Generated,
    feet: [f64; 3],
    height: f64,
    radius: f64,
) -> [f64; 3] {
    let _ = (level, open, height, radius);
    feet
}

/// Advances `body` by `dt` seconds at `level` (#558, #559).
///
/// `ground` returns the ground under a position (or `None` where the level
/// has none) and `pull` is the summed gravity at the body in open-cell
/// units per second squared ([`crate::gravity::pull`]); `speed` is the
/// free-flight speed multiplier (Shift+1-9). On the ground the body walks
/// and its feet follow the ground; Space jumps. In the air it falls under
/// the pull and keeps its momentum, so it drifts, and moving sideways fast
/// enough it falls around a body (an orbit). A double-tap takes off or
/// stops flying; flying hovers against the pull and moves with the keys
/// (Space and C climb and descend), and stopping keeps the last flying
/// velocity. `turning` decides whether A and D slide or roll. With `inertia`
/// the feet ease toward the wish instead of snapping to it, and landing
/// keeps the sideways momentum; without it the assignments stay instant,
/// as today. `camera_yaw` is the camera's heading: in third person
/// (`third_person`) the wish is built from it and the body turns toward its
/// walking direction at [`crate::motion::BODY_TURN_RATE`],
/// while in first person the wish is built from the body's own heading.
/// Pure.
#[allow(
    clippy::too_many_arguments,
    reason = "one step's physical inputs; a struct would only rename them"
)]
#[must_use]
pub fn body_step(
    mut body: Body,
    input: BodyInput,
    level: Level,
    ground: impl Fn([f64; 3]) -> Option<Ground>,
    pull: [f64; 3],
    turning: Turning,
    speed: f64,
    inertia: bool,
    camera_yaw: f64,
    third_person: bool,
    dt: f64,
) -> Body {
    if !dt.is_finite() || dt <= 0.0 {
        return body;
    }
    let metre = metre(level);
    let under = ground(body.position);
    // Up is the ground's up, else against the pull, else +Y.
    let up = under
        .map(|g| g.up)
        .or_else(|| unit(scale(pull, -1.0)))
        .unwrap_or([0.0, 1.0, 0.0]);
    // Double-tap Space toggles flight; a single press on the ground jumps.
    // A press counts only after a release since the last press, so holding
    // Space (or key repeat) never toggles again, and a third quick press
    // after a toggle starts a fresh single instead of toggling back (#756 C5).
    body.since_space += dt;
    if !input.space_held {
        body.awaiting_release = false;
    }
    let tap = input.space_pressed && !body.awaiting_release;
    if input.space_pressed {
        body.awaiting_release = true;
    }
    let double = tap && body.since_space <= DOUBLE_TAP_S;
    if tap {
        body.since_space = if double { f64::INFINITY } else { 0.0 };
    }
    if double {
        body.flying = !body.flying;
        body.roll = 0.0;
        if body.flying {
            body.velocity = [0.0; 3];
        }
    } else if tap && body.on_ground && !body.flying {
        body.velocity = scale(up, JUMP_M_PER_S * metre);
        body.on_ground = false;
    }
    // Heading and bank.
    let steer = f64::from(u8::from(input.right)) - f64::from(u8::from(input.left));
    let (sideways, rolling) = if body.flying && turning == Turning::Plane {
        (0.0, steer)
    } else {
        (steer, 0.0)
    };
    if rolling == 0.0 {
        body.roll -= body.roll.signum() * (ROLL_RATE * dt).min(body.roll.abs());
    } else {
        body.roll = (body.roll + rolling * ROLL_RATE * dt).clamp(-ROLL_LIMIT, ROLL_LIMIT);
    }
    body.yaw -= body.roll.sin() * BANK_TURN_RATE * dt;
    // In third person the wish is built from the camera's heading; in first
    // person the body already faces it, so the body's own heading is used.
    let move_yaw = if third_person { camera_yaw } else { body.yaw };
    let (forward, right) = heading_basis(up, move_yaw);
    let ahead = f64::from(u8::from(input.forward)) - f64::from(u8::from(input.back));
    // Real metres per second, scaled into the open level's units.
    let pace = match (body.flying, input.fast) {
        (true, true) => BOOST_M_PER_S,
        (true, false) => FLY_M_PER_S,
        (false, true) => RUN_M_PER_S,
        (false, false) => WALK_M_PER_S,
    } * metre
        * speed;
    let wish = unit(add(scale(forward, ahead), scale(right, sideways))).unwrap_or([0.0; 3]);
    let pushing = ahead != 0.0 || sideways != 0.0;
    // In third person the body turns toward its walking direction at a capped
    // rate (#756 C2); plane banking keeps driving the heading while flying.
    if third_person && pushing && !(body.flying && turning == Turning::Plane) {
        body.yaw = turn_toward(body.yaw, yaw_of(up, wish), BODY_TURN_RATE, dt);
    }
    let walking = body.on_ground && !body.flying;
    if walking {
        if inertia {
            // Gradual starts and stops (#756 C1): the feet keep a
            // velocity along the ground, easing toward the wish, so
            // turns bend instead of snapping. Braking from a run (or
            // a flight landing) uses the pace it carries.
            let along = dot(body.velocity, up);
            let planar = sub(body.velocity, scale(up, along));
            let carried = dot(planar, planar).sqrt();
            let running = input.fast || carried > WALK_M_PER_S * metre * speed;
            let (top_mps, up_time, down_time) = if running {
                (RUN_M_PER_S, RUN_S, RUN_S)
            } else {
                (WALK_M_PER_S, WALK_UP_S, WALK_DOWN_S)
            };
            let reference = if pushing {
                pace
            } else {
                (top_mps * metre * speed).max(carried)
            };
            let response = if pushing {
                inertia_rate(reference, up_time)
            } else {
                inertia_rate(reference, down_time)
            };
            let velocity = approach(planar, scale(wish, pace), response, dt);
            body.velocity = velocity;
            body.position = add(body.position, scale(velocity, dt));
        } else {
            // Feet move along the ground; the pull holds them there.
            body.velocity = [0.0; 3];
            body.position = add(body.position, scale(wish, pace * dt));
        }
    } else if body.flying {
        // Thrust cancels the pull: the keys set the velocity.
        let climb = f64::from(u8::from(input.space_held)) - f64::from(u8::from(input.down_held));
        let target = add(scale(wish, pace), scale(up, climb * pace));
        if inertia {
            let target_speed = dot(target, target).sqrt();
            let carried = dot(body.velocity, body.velocity).sqrt();
            let reference = if target_speed > 0.0 {
                pace
            } else {
                pace.max(carried)
            };
            body.velocity = approach(body.velocity, target, inertia_rate(reference, FLY_S), dt);
        } else {
            body.velocity = target;
        }
        body.position = add(body.position, scale(body.velocity, dt));
    } else {
        body.velocity = add(body.velocity, scale(pull, dt));
        body.position = add(body.position, scale(body.velocity, dt));
    }
    // Keep the feet on (never under) the ground. A walking body follows
    // the ground down slopes and around a planet: within one step (or one
    // body height) above it, it stays glued instead of floating.
    let glue = if walking {
        (pace * dt).max(body_height(level))
    } else {
        0.0
    };
    body.on_ground = false;
    if let Some(ground) = ground(body.position) {
        let height = dot(sub(body.position, ground.point), ground.up);
        if height <= glue {
            body.position = ground.point;
            // Flying or falling into the ground lands.
            body.flying = false;
            body.roll = 0.0;
            if inertia {
                // Landing keeps the sideways momentum (#756 C4): only
                // the into-ground part stops.
                let into = dot(body.velocity, ground.up);
                body.velocity = sub(body.velocity, scale(ground.up, into.min(0.0)));
            } else {
                body.velocity = [0.0; 3];
            }
            body.on_ground = true;
        }
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(n: u8) -> Level {
        Level::new(n).expect("level")
    }

    /// Earth gravity straight down in `level n` units.
    fn down(n: u8) -> [f64; 3] {
        [0.0, -GRAVITY_M * metre(level(n)), 0.0]
    }

    /// Earth gravity toward the cell centre (a planet of the test sphere).
    fn toward_centre(position: [f64; 3]) -> [f64; 3] {
        scale(
            unit(position).expect("off centre"),
            -GRAVITY_M * metre(level(10)),
        )
    }

    /// Flat ground at `y = 0` with `+Y` up.
    fn flat(position: [f64; 3]) -> Option<Ground> {
        Some(Ground {
            point: [position[0], 0.0, position[2]],
            up: [0.0, 1.0, 0.0],
        })
    }

    fn run(mut body: Body, input: BodyInput, n: u8, steps: usize) -> Body {
        for _ in 0..steps {
            body = body_step(
                body,
                input,
                level(n),
                flat,
                down(n),
                Turning::Drone,
                1.0,
                false,
                body.yaw,
                false,
                1.0 / 60.0,
            );
        }
        body
    }

    #[test]
    fn body_keeps_one_true_size_everywhere() {
        for n in 10..=14u8 {
            let height = body_height(level(n));
            let expected = 1.8 / 10f64.powf(scale_exponent(level(n)));
            assert!((height - expected).abs() <= expected * 1e-12, "L{n}");
        }
        assert!(body_height(level(10)) < 1e-6, "a speck on a whole planet");
        assert!(body_height(level(14)) > 0.3, "a third of a cell");
    }

    /// Metres covered in one second at level `n` with `input`, speed step
    /// multiplier `speed`, starting `flying` or standing on the ground.
    fn metres_in_a_second(n: u8, input: BodyInput, flying: bool, speed: f64) -> f64 {
        let mut body = if flying {
            let mut body = Body::standing([0.0, 1.0, 0.0], 0.0);
            body.flying = true;
            body
        } else {
            // One idle step lands the body.
            run(Body::standing([0.0; 3], 0.0), BodyInput::default(), n, 1)
        };
        let start = body.position;
        for _ in 0..60 {
            body = body_step(
                body,
                input,
                level(n),
                flat,
                down(n),
                Turning::Drone,
                speed,
                false,
                body.yaw,
                false,
                1.0 / 60.0,
            );
        }
        assert_eq!(body.flying, flying, "L{n} stays in its mode");
        let moved = sub(body.position, start);
        dot(moved, moved).sqrt() / metre(level(n))
    }

    #[test]
    fn walking_running_and_flying_are_real_speeds_at_every_level() {
        let walk = BodyInput {
            forward: true,
            ..BodyInput::default()
        };
        let fast = BodyInput { fast: true, ..walk };
        for n in 10..=14u8 {
            for (input, flying, expected) in [
                (walk, false, WALK_M_PER_S),
                (fast, false, RUN_M_PER_S),
                (walk, true, FLY_M_PER_S),
                (fast, true, BOOST_M_PER_S),
            ] {
                let metres = metres_in_a_second(n, input, flying, 1.0);
                assert!(
                    (metres - expected).abs() < expected * 1e-6,
                    "L{n} flying={flying} fast={}: {metres} m/s, want {expected}",
                    input.fast
                );
            }
        }
        assert_eq!(
            (WALK_M_PER_S, RUN_M_PER_S, FLY_M_PER_S, BOOST_M_PER_S),
            (1.4, 4.0, 10.0, 30.0),
            "the approved speeds (Spec v3, Q3 A)"
        );
    }

    #[test]
    fn the_speed_steps_multiply_every_speed() {
        let walk = BodyInput {
            forward: true,
            ..BodyInput::default()
        };
        for step in 1..=9u32 {
            let multiplier = crate::flight::free_speed_multiplier(step);
            let walked = metres_in_a_second(12, walk, false, multiplier);
            let flown = metres_in_a_second(12, walk, true, multiplier);
            assert!(
                (walked - WALK_M_PER_S * multiplier).abs() < 1e-6,
                "step {step}"
            );
            assert!(
                (flown - FLY_M_PER_S * multiplier).abs() < 1e-6,
                "step {step}"
            );
        }
        assert_eq!(crate::flight::free_speed_multiplier(1), 1.0 / 16.0);
        assert_eq!(crate::flight::free_speed_multiplier(5), 1.0);
        assert_eq!(crate::flight::free_speed_multiplier(9), 16.0);
    }

    #[test]
    fn a_jump_rises_and_lands_back_on_the_ground() {
        let mut body = run(Body::standing([0.0; 3], 0.0), BodyInput::default(), 14, 2);
        assert!(body.on_ground);
        let jump = BodyInput {
            space_pressed: true,
            ..BodyInput::default()
        };
        body = body_step(
            body,
            jump,
            level(14),
            flat,
            down(14),
            Turning::Drone,
            1.0,
            false,
            body.yaw,
            false,
            1.0 / 60.0,
        );
        let mut peak: f64 = 0.0;
        for _ in 0..120 {
            body = body_step(
                body,
                BodyInput::default(),
                level(14),
                flat,
                down(14),
                Turning::Drone,
                1.0,
                false,
                body.yaw,
                false,
                1.0 / 60.0,
            );
            peak = peak.max(body.position[1]);
        }
        let peak_m = peak / metre(level(14));
        assert!((0.3..0.8).contains(&peak_m), "about half a metre: {peak_m}");
        assert!(
            body.on_ground && body.position[1] == 0.0,
            "back on the ground"
        );
    }

    #[test]
    fn double_space_takes_off_and_lands() {
        let tap = BodyInput {
            space_pressed: true,
            ..BodyInput::default()
        };
        let idle = BodyInput::default();
        let step = |body: Body, input: BodyInput| {
            body_step(
                body,
                input,
                level(12),
                flat,
                down(12),
                Turning::Drone,
                1.0,
                false,
                body.yaw,
                false,
                0.1,
            )
        };
        let mut body = run(Body::standing([0.0; 3], 0.0), idle, 12, 2);
        body = step(body, tap);
        body = step(body, tap);
        assert!(body.flying, "a double-tap takes off");
        let climb = BodyInput {
            space_held: true,
            ..BodyInput::default()
        };
        for _ in 0..5 {
            body = step(body, climb);
        }
        assert!(body.position[1] > 0.0, "Space climbs while flying");
        body = step(body, idle);
        let hover = body.position[1];
        body = step(body, idle);
        assert_eq!(body.position[1], hover, "flying holds its height");
        body = step(
            body,
            BodyInput {
                down_held: true,
                ..BodyInput::default()
            },
        );
        assert!(body.position[1] < hover, "C descends");
        for _ in 0..30 {
            body = step(body, idle);
        }
        body = step(body, tap);
        body = step(body, tap);
        assert!(!body.flying, "another double-tap drops to land");
        for _ in 0..200 {
            body = step(body, idle);
        }
        assert!(body.on_ground, "and the fall lands on the ground");
    }

    #[test]
    fn plane_turning_rolls_and_drone_slides() {
        let right = BodyInput {
            right: true,
            ..BodyInput::default()
        };
        let mut drone = Body::standing([0.0, 0.5, 0.0], 0.0);
        drone.flying = true;
        let mut plane = drone;
        for _ in 0..30 {
            drone = body_step(
                drone,
                right,
                level(12),
                flat,
                down(12),
                Turning::Drone,
                1.0,
                false,
                drone.yaw,
                false,
                1.0 / 60.0,
            );
            plane = body_step(
                plane,
                right,
                level(12),
                flat,
                down(12),
                Turning::Plane,
                1.0,
                false,
                plane.yaw,
                false,
                1.0 / 60.0,
            );
        }
        assert_eq!(drone.roll, 0.0, "a drone never rolls");
        assert!(drone.position[0] > 0.0, "D slides a drone right");
        assert!(
            plane.roll > 0.0 && plane.roll <= ROLL_LIMIT,
            "D banks a plane"
        );
        assert!(plane.yaw < 0.0, "the bank turns the heading right");
    }

    #[test]
    fn on_the_whole_planet_gravity_pulls_to_the_centre() {
        let radius = 0.35;
        let sphere = |position: [f64; 3]| {
            let up = unit(position)?;
            Some(Ground {
                point: scale(up, radius),
                up,
            })
        };
        let start = [radius * 0.6, radius * 0.8, 0.0];
        let mut body = Body::standing(start, 0.0);
        let forward = BodyInput {
            forward: true,
            ..BodyInput::default()
        };
        for _ in 0..120 {
            body = body_step(
                body,
                forward,
                level(10),
                sphere,
                toward_centre(body.position),
                Turning::Drone,
                1.0,
                false,
                body.yaw,
                false,
                1.0 / 60.0,
            );
            let r = dot(body.position, body.position).sqrt();
            assert!(
                (r - radius).abs() < 1e-9,
                "walking stays on the sphere: {r}"
            );
        }
        let moved = sub(body.position, start);
        let metres = dot(moved, moved).sqrt() / metre(level(10));
        assert!(
            (metres - 2.0 * WALK_M_PER_S).abs() < 0.05,
            "it walked about two seconds (the first step lands) at real speed: {metres} m"
        );
    }

    #[test]
    fn without_ground_or_pull_the_body_drifts_at_its_own_speed() {
        // Far from everything (no ground, no pull) the body keeps its
        // momentum (#559): it neither stops nor falls.
        let mut body = Body::standing([0.0, 0.2, 0.0], 0.0);
        body.velocity = [0.01, 0.0, 0.0];
        for _ in 0..60 {
            body = body_step(
                body,
                BodyInput::default(),
                level(9),
                |_| None,
                [0.0; 3],
                Turning::Drone,
                1.0,
                false,
                body.yaw,
                false,
                1.0 / 60.0,
            );
        }
        assert!(!body.on_ground);
        assert!(
            (body.position[0] - 0.01).abs() < 1e-12,
            "drifted one second: {:?}",
            body.position
        );
        assert_eq!(body.position[1], 0.2, "no pull, no fall");
    }

    #[test]
    fn level_ground_follows_each_level() {
        let chain = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
        let mut universe = crate::nav::Universe::new(crate::nav::DEMO_SEED);
        for (depth, &marker) in chain.iter().enumerate() {
            assert!(
                universe.open(crate::nav::MarkerIndex(marker)),
                "journey opens"
            );
            let n = universe.level().get();
            let probe = [0.01, 0.2, -0.02];
            let ground = level_ground(&universe, probe);
            match n {
                10 => {
                    let ground = ground.expect("ground on the planet");
                    assert!((dot(ground.up, ground.up) - 1.0).abs() < 1e-12, "L{n} up");
                }
                _ => assert!(ground.is_none(), "L{n} has no ground (depth {depth})"),
            }
        }
    }

    #[test]
    fn walking_up_to_a_place_stays_on_the_surface() {
        let mut universe = opened(9);
        assert_eq!(universe.level().get(), 10);
        // Stand on the ground a little way from the first region portal.
        let portal = universe
            .open
            .points
            .iter()
            .find(|point| point.kind == crate::r#gen::MarkerKind::Portal)
            .copied()
            .expect("a region portal");
        let target = portal.portal_position();
        let start = [target[0] + 0.3, 0.0, target[2]];
        let ground = level_ground(&universe, start).expect("planet ground");
        let mut body = Body::standing(ground.point, 0.0);
        // Face the portal and walk at it: below L10 nothing can open.
        let toward = sub(target, start);
        body.yaw = yaw_of([0.0, 1.0, 0.0], unit(toward).expect("away from the portal"));
        let forward = BodyInput {
            forward: true,
            ..BodyInput::default()
        };
        // Real walking speed (Spec v3): the top speed step and long steps
        // cross the region's kilometres in few steps.
        let (speed, dt) = (16.0, 10.0);
        let step = |universe: &crate::nav::Universe, body: Body| {
            body_step(
                body,
                forward,
                universe.level(),
                |p| level_ground(universe, p),
                crate::gravity::pull(universe, body.position).accel,
                Turning::Drone,
                speed,
                false,
                body.yaw,
                false,
                dt,
            )
        };
        // Metres a step covers, measured in the level it is taken in.
        let metres = |universe: &crate::nav::Universe, from: Body, to: Body| {
            let moved = sub(to.position, from.position);
            dot(moved, moved).sqrt() / metre(universe.level())
        };
        let mut before = 0.0;
        for _ in 0..2000 {
            let next = step(&universe, body);
            before = metres(&universe, body, next);
            body = next;
            assert!(
                !matches!(walk_levels(&mut universe, &mut body), WalkEvent::Opened(_)),
                "walking up to the region opens nothing"
            );
        }
        assert_eq!(universe.level().get(), 10);
        // Slopes add a little climb, so each step is at least the level
        // pace.
        let expected = WALK_M_PER_S * speed * dt;
        assert!(
            (expected..expected * 1.1).contains(&before),
            "walking pace: {before} m in a step, want about {expected}"
        );
        let ground = level_ground(&universe, body.position).expect("planet ground");
        let height = dot(sub(body.position, ground.point), ground.up);
        assert!(
            height.abs() < body_height(universe.level()),
            "the feet stay on the ground: {height}"
        );
    }

    fn opened(depth: usize) -> crate::nav::Universe {
        let chain = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
        let mut universe = crate::nav::Universe::new(crate::nav::DEMO_SEED);
        for &marker in chain.get(..depth).expect("journey depth") {
            assert!(universe.open(crate::nav::MarkerIndex(marker)));
        }
        universe
    }

    #[test]
    fn a_body_high_above_the_planet_falls_back_and_lands() {
        let universe = opened(9);
        let radius = crate::terrain::PLANET_RADIUS_CELL;
        // Well above the surface, at rest: it falls faster and faster.
        let mut body = Body::standing([0.0, radius * 1.3, 0.0], 0.0);
        let mut speeds = Vec::new();
        for _ in 0..20_000 {
            let pull = crate::gravity::pull(&universe, body.position).accel;
            body = body_step(
                body,
                BodyInput::default(),
                level(10),
                |p| level_ground(&universe, p),
                pull,
                Turning::Drone,
                1.0,
                false,
                body.yaw,
                false,
                1.0,
            );
            speeds.push(dot(body.velocity, body.velocity).sqrt());
            if body.on_ground {
                break;
            }
        }
        assert!(body.on_ground, "the fall lands on the planet");
        assert!(
            speeds.windows(2).take(50).all(|w| w[1] >= w[0]),
            "it speeds up while falling"
        );
    }

    #[test]
    fn moving_sideways_fast_enough_falls_around_the_planet() {
        let universe = opened(9);
        let r = crate::terrain::PLANET_RADIUS_CELL * 2.0;
        let start = [r, 0.0, 0.0];
        let accel = crate::gravity::pull(&universe, start).accel;
        let g = dot(accel, accel).sqrt();
        // Circular speed v = sqrt(g r), sideways.
        let mut body = Body::standing(start, 0.0);
        body.velocity = [0.0, 0.0, (g * r).sqrt()];
        let period = 2.0 * std::f64::consts::PI * (r / g).sqrt();
        let dt = period / 2000.0;
        let mut lowest = f64::INFINITY;
        let mut highest: f64 = 0.0;
        for _ in 0..1000 {
            let pull = crate::gravity::pull(&universe, body.position).accel;
            body = body_step(
                body,
                BodyInput::default(),
                level(10),
                |p| level_ground(&universe, p),
                pull,
                Turning::Drone,
                1.0,
                false,
                body.yaw,
                false,
                dt,
            );
            let distance = dot(body.position, body.position).sqrt();
            lowest = lowest.min(distance);
            highest = highest.max(distance);
        }
        assert!(!body.on_ground, "it never lands");
        assert!(
            lowest > r * 0.95 && highest < r * 1.05,
            "half an orbit at its radius: {lowest}..{highest}"
        );
        assert!(body.position[0] < 0.0, "half way round the planet");
    }

    #[test]
    fn falling_onto_a_region_stays_on_the_surface() {
        let mut universe = opened(9);
        let portal = universe
            .open
            .points
            .iter()
            .find(|point| point.kind == crate::r#gen::MarkerKind::Portal)
            .copied()
            .expect("a region portal");
        let at = portal.portal_position();
        let up = unit(at).expect("off centre");
        // Above the region, falling straight down onto it.
        let mut body = Body::standing(add(at, scale(up, 0.05)), 0.0);
        for _ in 0..20_000 {
            let pull = crate::gravity::pull(&universe, body.position).accel;
            let n = universe.level();
            body = body_step(
                body,
                BodyInput::default(),
                n,
                |p| level_ground(&universe, p),
                pull,
                Turning::Drone,
                1.0,
                false,
                body.yaw,
                false,
                1.0,
            );
            assert!(
                !matches!(walk_levels(&mut universe, &mut body), WalkEvent::Opened(_)),
                "falling onto the region opens nothing"
            );
        }
        assert_eq!(universe.level().get(), 10);
        let ground = level_ground(&universe, body.position).expect("planet ground");
        let height = dot(sub(body.position, ground.point), ground.up);
        assert!(
            height.abs() < body_height(universe.level()),
            "the fall lands on the ground: {height}"
        );
    }

    #[test]
    fn heading_basis_is_orthonormal_and_turns_left_with_yaw() {
        for up in [[0.0, 1.0, 0.0], [0.6, 0.8, 0.0], [0.0, 0.0, 1.0]] {
            let (forward, right) = heading_basis(up, 0.3);
            assert!((dot(forward, forward) - 1.0).abs() < 1e-12);
            assert!(dot(forward, up).abs() < 1e-12 && dot(right, up).abs() < 1e-12);
        }
        for yaw in [0.0, 0.7, -2.0] {
            let up = [0.6, 0.8, 0.0];
            let (forward, _) = heading_basis(up, yaw);
            assert!(
                (yaw_of(up, forward) - yaw).abs() < 1e-12,
                "yaw_of inverts heading_basis"
            );
        }
        let (zero, _) = heading_basis([0.0, 1.0, 0.0], 0.0);
        let (left, _) = heading_basis([0.0, 1.0, 0.0], 0.5);
        assert!((zero[2] + 1.0).abs() < 1e-12, "heading zero faces -Z");
        assert!(left[0] < 0.0, "positive yaw turns left");
    }
}
