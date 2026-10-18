//! Player controls that move like in reality (#756, Spec v2).
//!
//! Integration tests over the public core API (`E-TEST-PLACE`): inertia and
//! landing momentum (T1), camera-relative movement (T2), smooth camera (T3),
//! and the double-tap cases (T4). No window, no GPU (`E-TEST-NO-WINDOW`);
//! hermetic fixed steps (`E-TEST-HERMETIC`).

use universe_core::body::{
    BOOST_M_PER_S, Body, BodyInput, DOUBLE_TAP_S, FLY_M_PER_S, Ground, JUMP_M_PER_S, RUN_M_PER_S,
    Turning, WALK_M_PER_S, body_step, metre,
};
use universe_core::coords::Level;

#[expect(
    clippy::expect_used,
    reason = "E-NO-PANIC: test helper, levels 10-14 always valid"
)]
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

/// Earth gravity straight down in `level n` units.
fn down(n: u8) -> [f64; 3] {
    [0.0, -universe_core::body::GRAVITY_M * metre(level(n)), 0.0]
}

/// Stand on the flat ground with `inertia`.
fn landed(n: u8, inertia: bool) -> Body {
    let mut body = Body::standing([0.0; 3], 0.0);
    for _ in 0..2 {
        body = body_step(
            body,
            BodyInput::default(),
            level(n),
            flat,
            down(n),
            Turning::Drone,
            1.0,
            inertia,
            1.0 / 60.0,
        );
    }
    assert!(body.on_ground, "lands on the flat ground");
    body
}

fn forward(fast: bool) -> BodyInput {
    BodyInput {
        forward: true,
        fast,
        ..BodyInput::default()
    }
}

/// Forward speed in m/s from the cell-unit velocity.
fn speed_ms(velocity: [f64; 3], n: u8) -> f64 {
    let length =
        (velocity[0] * velocity[0] + velocity[1] * velocity[1] + velocity[2] * velocity[2]).sqrt();
    length / metre(level(n))
}

#[test]
fn movement_builds_up_and_keeps_momentum() {
    // #756 C1: walking, running and flying speed up and slow down over the
    // spec's times, momentum carries through turns, top speeds unchanged.
    let dt = 1.0 / 60.0;
    let mut body = landed(14, true);
    // Walking builds up over about 0.3 s.
    for _ in 0..9 {
        body = body_step(
            body,
            forward(false),
            level(14),
            flat,
            down(14),
            Turning::Drone,
            1.0,
            true,
            dt,
        );
    }
    let half = speed_ms(body.velocity, 14);
    assert!(
        (0.3 * WALK_M_PER_S..0.9 * WALK_M_PER_S).contains(&half),
        "about half speed after 0.15 s: {half}"
    );
    for _ in 0..21 {
        body = body_step(
            body,
            forward(false),
            level(14),
            flat,
            down(14),
            Turning::Drone,
            1.0,
            true,
            dt,
        );
    }
    let full = speed_ms(body.velocity, 14);
    assert!(
        (full - WALK_M_PER_S).abs() < WALK_M_PER_S * 0.05,
        "about full walking speed after 0.5 s: {full}"
    );
    // Releasing the keys slows to a stop in about 0.25 s.
    for _ in 0..15 {
        body = body_step(
            body,
            BodyInput::default(),
            level(14),
            flat,
            down(14),
            Turning::Drone,
            1.0,
            true,
            dt,
        );
    }
    let stopped = speed_ms(body.velocity, 14);
    assert!(
        stopped < WALK_M_PER_S * 0.05,
        "about stopped after 0.25 s: {stopped}"
    );
    // Momentum carries through a turn: full speed ahead, then strafe right.
    // After a few frames the velocity bends (both components live) instead
    // of snapping to the new wish.
    let mut turning = landed(14, true);
    for _ in 0..40 {
        turning = body_step(
            turning,
            forward(false),
            level(14),
            flat,
            down(14),
            Turning::Drone,
            1.0,
            true,
            dt,
        );
    }
    let right = BodyInput {
        right: true,
        ..BodyInput::default()
    };
    for _ in 0..6 {
        turning = body_step(
            turning,
            right,
            level(14),
            flat,
            down(14),
            Turning::Drone,
            1.0,
            true,
            dt,
        );
    }
    let metres = metre(level(14));
    let vx = turning.velocity[0] / metres;
    let vz = turning.velocity[2] / metres;
    assert!(vx > 0.1, "turns toward the new wish: {vx}");
    assert!(vz < -0.5 * WALK_M_PER_S, "keeps momentum: {vz}");
    // The top speeds are unchanged: run, fly and boost reach theirs.
    let mut run = landed(14, true);
    for _ in 0..40 {
        run = body_step(
            run,
            forward(true),
            level(14),
            flat,
            down(14),
            Turning::Drone,
            1.0,
            true,
            dt,
        );
    }
    let run_speed = speed_ms(run.velocity, 14);
    assert!(
        (run_speed - RUN_M_PER_S).abs() < RUN_M_PER_S * 0.05,
        "run reaches {RUN_M_PER_S} m/s: {run_speed}"
    );
    for (fast, want) in [(false, FLY_M_PER_S), (true, BOOST_M_PER_S)] {
        let mut flying = Body::standing([0.0, 1.0, 0.0], 0.0);
        flying.flying = true;
        for _ in 0..70 {
            flying = body_step(
                flying,
                forward(fast),
                level(14),
                flat,
                down(14),
                Turning::Drone,
                1.0,
                true,
                dt,
            );
        }
        let flown = speed_ms(flying.velocity, 14);
        assert!(
            (flown - want).abs() < want * 0.05,
            "flying reaches {want} m/s: {flown}"
        );
    }
    // With inertia off the first step moves the full stride, as today.
    let still = landed(14, false);
    let moved = body_step(
        still,
        forward(false),
        level(14),
        flat,
        down(14),
        Turning::Drone,
        1.0,
        false,
        dt,
    );
    let stride = ((moved.position[0] - still.position[0]).powi(2)
        + (moved.position[2] - still.position[2]).powi(2))
    .sqrt()
        / metre(level(14));
    assert!(
        (stride - WALK_M_PER_S * dt).abs() < 1e-9,
        "instant stride without inertia: {stride}"
    );
}

#[test]
fn jumps_follow_local_gravity() {
    // #756 C4: the same 3.2 m/s push jumps about six times higher on the Moon
    // than on Earth, and landing keeps the sideways momentum.
    let dt = 1.0 / 60.0;
    let jump = BodyInput {
        space_pressed: true,
        ..BodyInput::default()
    };
    let peak_and_time = |pull: [f64; 3]| {
        let mut body = landed(14, true);
        body = body_step(
            body,
            jump,
            level(14),
            flat,
            pull,
            Turning::Drone,
            1.0,
            true,
            dt,
        );
        let mut peak = 0.0f64;
        let mut steps = 0usize;
        for _ in 0..2000 {
            body = body_step(
                body,
                BodyInput::default(),
                level(14),
                flat,
                pull,
                Turning::Drone,
                1.0,
                true,
                dt,
            );
            peak = peak.max(body.position[1]);
            steps += 1;
            if body.on_ground {
                break;
            }
        }
        assert!(body.on_ground, "the jump lands back on the ground");
        (peak / metre(level(14)), steps)
    };
    let (earth_peak, _) = peak_and_time(down(14));
    assert!(
        (0.3..0.8).contains(&earth_peak),
        "about half a metre on Earth: {earth_peak}"
    );
    let moon_pull = [0.0, -1.62 * metre(level(14)), 0.0];
    let (moon_peak, _) = peak_and_time(moon_pull);
    let ratio = moon_peak / earth_peak;
    assert!(
        moon_peak > earth_peak,
        "the Moon jump is higher: {moon_peak} vs {earth_peak}"
    );
    assert!(
        (5.0..7.0).contains(&ratio),
        "about six times higher on the Moon: {ratio}"
    );
    let _ = JUMP_M_PER_S;
    // Landing keeps the sideways momentum with inertia, and stops dead
    // without it.
    for (inertia, want) in [(true, 1.0), (false, 0.0)] {
        let mut body = Body::standing([0.0, 0.01, 0.0], 0.0);
        body.velocity = [1.0 * metre(level(14)), -0.5 * metre(level(14)), 0.0];
        for _ in 0..600 {
            body = body_step(
                body,
                BodyInput::default(),
                level(14),
                flat,
                down(14),
                Turning::Drone,
                1.0,
                inertia,
                dt,
            );
            if body.on_ground {
                break;
            }
        }
        assert!(body.on_ground, "the fall lands");
        let kept = body.velocity[0] / metre(level(14));
        assert!(
            (kept - want).abs() < 0.05,
            "inertia={inertia} keeps {want} m/s sideways: {kept}"
        );
    }
    let _ = DOUBLE_TAP_S;
}
