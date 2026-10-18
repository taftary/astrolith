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
            body.yaw,
            false,
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
            body.yaw,
            false,
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
            body.yaw,
            false,
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
            body.yaw,
            false,
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
            turning.yaw,
            false,
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
            turning.yaw,
            false,
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
            run.yaw,
            false,
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
                flying.yaw,
                false,
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
        still.yaw,
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
            body.yaw,
            false,
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
                body.yaw,
                false,
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
                body.yaw,
                false,
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

#[test]
fn smoothing_and_inertia_settings() {
    // #756 C6 (S1, S2): Camera smoothing (Off, Low, Medium, High; default
    // Low) and Movement inertia (default on) are saved, restored and reset
    // with Controls; Off gives today's behaviour.
    use universe_core::flight::{
        LOOK_TAU_HIGH, LOOK_TAU_LOW, LOOK_TAU_MEDIUM, LOOK_TAU_OFF, smooth_look,
    };
    use universe_core::settings::{CameraSmoothing, Category, Settings};
    assert_eq!(Settings::defaults().camera_smoothing, CameraSmoothing::Low);
    assert!(Settings::defaults().movement_inertia);
    assert_eq!(CameraSmoothing::from_id("off"), Some(CameraSmoothing::Off));
    assert_eq!(CameraSmoothing::from_id("low"), Some(CameraSmoothing::Low));
    assert_eq!(
        CameraSmoothing::from_id("medium"),
        Some(CameraSmoothing::Medium)
    );
    assert_eq!(
        CameraSmoothing::from_id("high"),
        Some(CameraSmoothing::High)
    );
    assert_eq!(CameraSmoothing::from_id("turbo"), None);
    assert_eq!(CameraSmoothing::Off.tau(), LOOK_TAU_OFF);
    assert_eq!(CameraSmoothing::Low.tau(), LOOK_TAU_LOW);
    assert_eq!(CameraSmoothing::Medium.tau(), LOOK_TAU_MEDIUM);
    assert_eq!(CameraSmoothing::High.tau(), LOOK_TAU_HIGH);
    // Off snaps to the target: today's raw mouse.
    assert_eq!(
        smooth_look(0.7, 0.2, CameraSmoothing::Off.tau(), 1.0 / 60.0),
        0.7
    );
    // Saved, restored and reset with Controls.
    let mut s = Settings::defaults();
    s.camera_smoothing = CameraSmoothing::High;
    s.movement_inertia = false;
    let text = s.format();
    assert!(text.contains("camera_smoothing = high"), "{text}");
    assert!(text.contains("movement_inertia = off"), "{text}");
    assert_eq!(Settings::parse(&text), s);
    let reset = s.reset(Category::Controls);
    assert_eq!(reset.camera_smoothing, CameraSmoothing::Low);
    assert!(reset.movement_inertia);
    // Damaged entries keep their defaults, and an old file keeps them too.
    let damaged = Settings::parse("camera_smoothing = turbo\nmovement_inertia = maybe\n");
    assert_eq!(damaged.camera_smoothing, CameraSmoothing::Low);
    assert!(damaged.movement_inertia);
    let old = Settings::parse("exposure = 2\nlabels = off\nspeed = 3\n");
    assert_eq!(old.camera_smoothing, CameraSmoothing::Low);
    assert!(old.movement_inertia);
}

#[test]
fn third_person_moves_with_the_camera() {
    // #756 C2: in third person movement is relative to the camera and the
    // body turns toward its walking direction; in first person the body
    // faces the look direction.
    use std::f64::consts::PI;
    let dt = 1.0 / 60.0;
    // Camera and body both face -Z (yaw 0). Strafe right (D): the body moves
    // +X and turns toward -PI/2 (a half turn takes about 0.33 s).
    let mut body = landed(14, true);
    let right = BodyInput {
        right: true,
        ..BodyInput::default()
    };
    for _ in 0..20 {
        body = body_step(
            body,
            right,
            level(14),
            flat,
            down(14),
            Turning::Drone,
            1.0,
            true,
            0.0,
            true,
            dt,
        );
    }
    let metres = metre(level(14));
    let x = body.position[0] / metres;
    assert!(
        (0.15..0.35).contains(&x),
        "strafes where the camera looks: {x}"
    );
    assert!(
        body.position[2].abs() < 1e-12,
        "no forward drift: {:?}",
        body.position
    );
    assert!(
        (body.yaw + PI / 2.0).abs() < 0.05,
        "turns toward its walking direction: {}",
        body.yaw
    );
    // A turned camera moves the body where it looks, not where the body
    // faces: camera yaw PI/2 faces -X, the body starts facing elsewhere.
    let mut turned = landed(14, true);
    turned.yaw = 2.0;
    for _ in 0..20 {
        turned = body_step(
            turned,
            forward(false),
            level(14),
            flat,
            down(14),
            Turning::Drone,
            1.0,
            true,
            PI / 2.0,
            true,
            dt,
        );
    }
    assert!(
        turned.position[0] < 0.0,
        "walks where the camera looks: {:?}",
        turned.position
    );
    assert!(
        (turned.yaw - PI / 2.0).abs() < 1e-9,
        "faces its walking direction: {}",
        turned.yaw
    );
    // In first person the body keeps facing the look direction.
    let mut first = landed(14, true);
    first.yaw = 0.7;
    for _ in 0..20 {
        first = body_step(
            first,
            forward(false),
            level(14),
            flat,
            down(14),
            Turning::Drone,
            1.0,
            true,
            first.yaw,
            false,
            dt,
        );
    }
    assert_eq!(first.yaw, 0.7, "first person keeps its heading");
    assert!(
        (first.position[0].powi(2) + first.position[2].powi(2)).sqrt() / metres > 0.15,
        "and walks: {:?}",
        first.position
    );
    // Plane turning keeps its steering in third person: D banks the body and
    // the bank turns the heading, while thrust follows the camera forward.
    let mut plane = Body::standing([0.0, 1.0, 0.0], 0.0);
    plane.flying = true;
    let banked = BodyInput {
        forward: true,
        right: true,
        ..BodyInput::default()
    };
    for _ in 0..30 {
        plane = body_step(
            plane,
            banked,
            level(14),
            flat,
            down(14),
            Turning::Plane,
            1.0,
            true,
            0.0,
            true,
            dt,
        );
    }
    assert!(plane.roll > 0.0, "D banks a plane: {}", plane.roll);
    assert!(plane.yaw < 0.0, "the bank turns the heading: {}", plane.yaw);
    assert!(
        plane.position[2] < 0.0,
        "thrust follows the camera: {:?}",
        plane.position
    );
}

#[test]
fn double_tap_toggles_flight_once() {
    // #756 C5: a single press jumps; two presses within 0.35 s toggle flight
    // exactly once; a third quick press never toggles again; a held press
    // never repeats. Touch Jump feeds the same input, so it behaves the same.
    let dt = 1.0 / 60.0;
    // A realistic press holds the key past its pressed frame.
    let press = BodyInput {
        space_pressed: true,
        space_held: true,
        ..BodyInput::default()
    };
    let hold = BodyInput {
        space_held: true,
        ..BodyInput::default()
    };
    let idle = BodyInput::default();
    let step = |body: Body, input: BodyInput| {
        body_step(
            body,
            input,
            level(14),
            flat,
            down(14),
            Turning::Drone,
            1.0,
            true,
            body.yaw,
            false,
            dt,
        )
    };
    // A single press on the ground jumps.
    let mut body = landed(14, true);
    body = step(body, press);
    assert!(!body.on_ground, "a single press jumps");
    assert!(!body.flying, "but does not take off");
    // Two presses within 0.35 s take off exactly once.
    body = step(body, idle);
    body = step(body, press);
    assert!(body.flying, "a double-tap takes off");
    // A third quick press never toggles again.
    body = step(body, idle);
    body = step(body, press);
    assert!(body.flying, "a third quick press keeps flying");
    body = step(body, idle);
    // A fresh double after a pause toggles back.
    for _ in 0..30 {
        body = step(body, idle);
    }
    body = step(body, press);
    body = step(body, idle);
    body = step(body, press);
    assert!(!body.flying, "a fresh double lands");
    // A held press never repeats: holding from the ground jumps once and
    // never takes off.
    let mut held = landed(14, true);
    held = step(held, press);
    for _ in 0..30 {
        held = step(held, hold);
    }
    assert!(!held.on_ground, "the hold jumped once");
    assert!(!held.flying, "the hold never took off");
    // Holding while flying never toggles either.
    let mut cruise = landed(14, true);
    cruise = step(cruise, press);
    cruise = step(cruise, idle);
    cruise = step(cruise, press);
    assert!(cruise.flying, "double-tap takes off");
    for _ in 0..60 {
        cruise = step(cruise, hold);
    }
    assert!(cruise.flying, "the hold never toggles back");
    // Touch Jump behaves the same: the touch button feeds `space_pressed`,
    // so the same double takes off exactly once.
    let mut touch = landed(14, true);
    touch = step(touch, press);
    touch = step(touch, idle);
    touch = step(touch, press);
    assert!(touch.flying, "touch double-tap takes off");
    touch = step(touch, idle);
    touch = step(touch, press);
    assert!(touch.flying, "touch third press keeps flying");
    let _ = DOUBLE_TAP_S;
}
