//! The player's body in normal mode (#558): walk, jump, fly, turn.
//!
//! Pure and deterministic (`E-CORE-NO-BEVY`, `E-DET-TIERS`): one
//! [`body_step`] advances a [`Body`] by `dt` seconds from a [`BodyInput`]
//! and a [`Ground`] reader. The body is true size, about
//! [`BODY_HEIGHT_M`] tall in every level's own units, while its walking and
//! flying speeds are given in cells per second, so crossing a region takes
//! about as long as crossing a room (owner answer Q4). Vertical motion under
//! gravity (jumps, falls) uses real metres and seconds, scaled into the
//! level's units, so a jump looks the same at every level.
//!
//! Gravity in this step (owner answer Q2): toward the planet's centre on
//! the whole planet (L10), straight down onto the ground from the region to
//! the room (L11-L14), and nothing elsewhere, where the body floats and
//! flies. Gravity from every body everywhere is step 3 (#559).

use crate::coords::Level;
use crate::nest::scale_exponent;

/// Body height in metres (eye height is [`EYE_SHARE`] of it).
pub const BODY_HEIGHT_M: f64 = 1.8;

/// Eye height as a share of the body height.
pub const EYE_SHARE: f64 = 0.93;

/// Body radius as a share of the body height (shoulder half width).
pub const RADIUS_SHARE: f64 = 0.15;

/// Walking speed in cells per second at the default speed step.
pub const WALK_CELLS_PER_S: f64 = 0.08;

/// Running multiplier (Shift held while walking).
pub const RUN_FACTOR: f64 = 2.5;

/// Flying speed in cells per second at the default speed step.
pub const FLY_CELLS_PER_S: f64 = 0.2;

/// Boost multiplier (Shift held while flying).
pub const BOOST_FACTOR: f64 = 3.0;

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
    /// Velocity along the local up, open-cell units per second (jumps,
    /// falls, climbs). Horizontal motion is applied directly each step.
    pub vertical: f64,
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
}

impl Body {
    /// A body standing at `position` with heading `yaw`.
    #[must_use]
    pub fn standing(position: [f64; 3], yaw: f64) -> Body {
        Body {
            position,
            vertical: 0.0,
            yaw,
            roll: 0.0,
            flying: false,
            on_ground: false,
            since_space: f64::INFINITY,
        }
    }
}

/// One metre in `level`'s open-cell units (a cell spans `10^e` metres).
#[must_use]
pub fn metre(level: Level) -> f64 {
    10f64.powf(-scale_exponent(level))
}

/// Body height in `level`'s open-cell units.
#[must_use]
pub fn body_height(level: Level) -> f64 {
    BODY_HEIGHT_M * metre(level)
}

/// Whether gravity holds the body at `level` in this step (L10-L14).
#[must_use]
pub fn has_gravity(level: Level) -> bool {
    (10..=14).contains(&level.get())
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
/// The whole planet (L10) pulls toward its centre onto the relief
/// (`terrain::surface_radius`); the region, city, and plot cells (L11-L13)
/// stand on their relief surface (`ground::ground_height`); the room (L14)
/// stands on its shell's floor. `None` elsewhere.
#[must_use]
pub fn level_ground(universe: &crate::nav::Universe, position: [f64; 3]) -> Option<Ground> {
    match universe.level().get() {
        10 => {
            let up = unit(position)?;
            let radius = crate::terrain::surface_radius(universe.open_seed(), up);
            Some(Ground {
                point: scale(up, radius),
                up,
            })
        }
        11..=13 => {
            let frame = universe.frame.as_ref()?;
            let height = crate::ground::ground_height(frame, position[0], position[2]);
            Some(Ground {
                point: [position[0], height, position[2]],
                up: [0.0, 1.0, 0.0],
            })
        }
        14 => {
            let shell = universe.open.points.last()?;
            Some(Ground {
                point: [position[0], shell.position[1], position[2]],
                up: [0.0, 1.0, 0.0],
            })
        }
        _ => None,
    }
}

/// Advances `body` by `dt` seconds at `level` (#558).
///
/// `ground` returns the ground under a position (or `None` where the level
/// has none); `speed` is the free-flight speed multiplier (Shift+1-9).
/// Walking slides along the ground and keeps the feet on it; Space jumps,
/// a double-tap takes off or lands; while flying, Space and C climb and
/// descend, and `turning` decides whether A and D slide or roll. Pure.
#[must_use]
pub fn body_step(
    mut body: Body,
    input: BodyInput,
    level: Level,
    ground: impl Fn([f64; 3]) -> Option<Ground>,
    turning: Turning,
    speed: f64,
    dt: f64,
) -> Body {
    if !dt.is_finite() || dt <= 0.0 {
        return body;
    }
    let metre = metre(level);
    let gravity_here = has_gravity(level);
    let under = if gravity_here {
        ground(body.position)
    } else {
        None
    };
    let up = under.map_or([0.0, 1.0, 0.0], |g| g.up);
    // Double-tap Space toggles flight; a single press on the ground jumps.
    body.since_space += dt;
    let double = input.space_pressed && body.since_space <= DOUBLE_TAP_S;
    if input.space_pressed {
        body.since_space = if double { f64::INFINITY } else { 0.0 };
    }
    if double {
        body.flying = !body.flying || !gravity_here;
        body.vertical = 0.0;
        body.roll = 0.0;
    } else if input.space_pressed && body.on_ground && !body.flying {
        body.vertical = JUMP_M_PER_S * metre;
        body.on_ground = false;
    }
    if !gravity_here {
        body.flying = true;
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
    // Horizontal motion along the ground plane, in cells per second.
    let (forward, right) = heading_basis(up, body.yaw);
    let ahead = f64::from(u8::from(input.forward)) - f64::from(u8::from(input.back));
    let rate = if body.flying {
        FLY_CELLS_PER_S * if input.fast { BOOST_FACTOR } else { 1.0 }
    } else {
        WALK_CELLS_PER_S * if input.fast { RUN_FACTOR } else { 1.0 }
    } * speed;
    let mut step = add(scale(forward, ahead), scale(right, sideways));
    if let Some(direction) = unit(step) {
        step = scale(direction, rate * dt);
    }
    body.position = add(body.position, step);
    // Vertical motion: climbing while flying, gravity otherwise.
    if body.flying {
        let climb = f64::from(u8::from(input.space_held)) - f64::from(u8::from(input.down_held));
        body.vertical = climb * rate;
    } else {
        body.vertical -= GRAVITY_M * metre * dt;
    }
    body.position = add(body.position, scale(up, body.vertical * dt));
    // Keep the feet on (never under) the ground. A walking body also
    // follows the ground down slopes and around a planet: within one step
    // (or one body height) above it, it stays glued instead of floating.
    let walking = !body.flying && body.vertical <= 0.0;
    let glue = if walking {
        (rate * dt).max(body_height(level))
    } else {
        0.0
    };
    body.on_ground = false;
    if gravity_here && let Some(ground) = ground(body.position) {
        let height = dot(sub(body.position, ground.point), ground.up);
        if height <= glue {
            body.position = ground.point;
            if body.flying {
                // Flying into the ground lands.
                body.flying = false;
                body.roll = 0.0;
            }
            body.vertical = 0.0;
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

    /// Flat ground at `y = 0` with `+Y` up.
    fn flat(position: [f64; 3]) -> Option<Ground> {
        Some(Ground {
            point: [position[0], 0.0, position[2]],
            up: [0.0, 1.0, 0.0],
        })
    }

    fn run(mut body: Body, input: BodyInput, n: u8, steps: usize) -> Body {
        for _ in 0..steps {
            body = body_step(body, input, level(n), flat, Turning::Drone, 1.0, 1.0 / 60.0);
        }
        body
    }

    #[test]
    fn the_body_is_true_size_at_every_level() {
        for n in 10..=14u8 {
            let height = body_height(level(n));
            let expected = 1.8 / 10f64.powf(scale_exponent(level(n)));
            assert!((height - expected).abs() <= expected * 1e-12, "L{n}");
        }
        assert!(body_height(level(10)) < 1e-6, "a speck on a whole planet");
        assert!(body_height(level(14)) > 0.3, "a third of a room");
    }

    #[test]
    fn walking_crosses_a_cell_in_the_same_time_at_every_level() {
        let forward = BodyInput {
            forward: true,
            ..BodyInput::default()
        };
        let mut covered = Vec::new();
        for n in [11u8, 12, 13, 14] {
            let body = run(Body::standing([0.0; 3], 0.0), forward, n, 60);
            covered.push(body.position[2].abs());
            assert!(body.on_ground, "L{n} walks on the ground");
            assert_eq!(body.position[1], 0.0, "L{n} feet stay on the ground");
        }
        for pair in covered.windows(2) {
            assert!((pair[0] - pair[1]).abs() < 1e-12, "same pace: {covered:?}");
        }
        assert!((covered[0] - WALK_CELLS_PER_S).abs() < 1e-9);
    }

    #[test]
    fn a_jump_rises_and_lands_back_on_the_ground() {
        let mut body = run(Body::standing([0.0; 3], 0.0), BodyInput::default(), 14, 2);
        assert!(body.on_ground);
        let jump = BodyInput {
            space_pressed: true,
            ..BodyInput::default()
        };
        body = body_step(body, jump, level(14), flat, Turning::Drone, 1.0, 1.0 / 60.0);
        let mut peak: f64 = 0.0;
        for _ in 0..120 {
            body = body_step(
                body,
                BodyInput::default(),
                level(14),
                flat,
                Turning::Drone,
                1.0,
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
            body_step(body, input, level(12), flat, Turning::Drone, 1.0, 0.1)
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
                Turning::Drone,
                1.0,
                1.0 / 60.0,
            );
            plane = body_step(
                plane,
                right,
                level(12),
                flat,
                Turning::Plane,
                1.0,
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
                Turning::Drone,
                1.0,
                1.0 / 60.0,
            );
            let r = dot(body.position, body.position).sqrt();
            assert!(
                (r - radius).abs() < 1e-9,
                "walking stays on the sphere: {r}"
            );
        }
        assert!(
            sub(body.position, start).iter().any(|c| c.abs() > 1e-3),
            "it moved"
        );
    }

    #[test]
    fn without_ground_the_body_floats_and_flies() {
        let body = run(
            Body::standing([0.0, 0.2, 0.0], 0.0),
            BodyInput::default(),
            9,
            30,
        );
        assert!(body.flying, "no gravity above the planet in this step");
        assert_eq!(body.position[1], 0.2, "it floats in place");
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
                10..=14 => {
                    let ground = ground.expect("ground from the planet to the room");
                    assert!((dot(ground.up, ground.up) - 1.0).abs() < 1e-12, "L{n} up");
                }
                _ => assert!(ground.is_none(), "L{n} has no ground (depth {depth})"),
            }
        }
    }

    #[test]
    fn heading_basis_is_orthonormal_and_turns_left_with_yaw() {
        for up in [[0.0, 1.0, 0.0], [0.6, 0.8, 0.0], [0.0, 0.0, 1.0]] {
            let (forward, right) = heading_basis(up, 0.3);
            assert!((dot(forward, forward) - 1.0).abs() < 1e-12);
            assert!(dot(forward, up).abs() < 1e-12 && dot(right, up).abs() < 1e-12);
        }
        let (zero, _) = heading_basis([0.0, 1.0, 0.0], 0.0);
        let (left, _) = heading_basis([0.0, 1.0, 0.0], 0.5);
        assert!((zero[2] + 1.0).abs() < 1e-12, "heading zero faces -Z");
        assert!(left[0] < 0.0, "positive yaw turns left");
    }
}
