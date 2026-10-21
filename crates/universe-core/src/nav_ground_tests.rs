//! Planet-to-ground dive tests (#892 T1): no sea level, the real-ground
//! stop and the slow-down near the ground.

use crate::dive_orbit::{GROUND_CLEARANCE_M, ground_radius_cells, lowest_ground_avoid_radius};
use crate::nav::{DEMO_SEED, DiveMode, Universe, planet_frame, planet_relief, replay_autopilot};
use crate::planet_frame::PlanetFrame;
use crate::relief::Relief;

/// The L10 home planet after the seeded journey, with its frame and relief.
fn at_the_planet() -> (Universe, PlanetFrame, Relief) {
    let (_, universe) = replay_autopilot(DEMO_SEED, 1.0 / 60.0, 600.0);
    assert_eq!(universe.level().get(), 10);
    let frame = planet_frame(&universe).expect("L10 frame");
    let relief = planet_relief(&universe).expect("L10 relief").clone();
    (universe, frame, relief)
}

/// A unit direction whose ground lies at least `below_m` under the mean
/// radius (an old sea floor), found by a fixed scan.
fn basin_direction(relief: &Relief, below_m: f64) -> [f64; 3] {
    for i in 0..4000u32 {
        let t = f64::from(i) * 0.618_033_988_75;
        let z = 1.0 - 2.0 * (f64::from(i) + 0.5) / 4000.0;
        let r = (1.0 - z * z).max(0.0).sqrt();
        let a = t * std::f64::consts::TAU;
        let dir = [r * a.cos(), z, r * a.sin()];
        if relief.height_m(dir) <= -below_m {
            return dir;
        }
    }
    panic!("no basin {below_m} m deep on the home planet");
}

/// A unit direction whose ground lies above the mean radius (land).
fn land_direction(relief: &Relief) -> [f64; 3] {
    for i in 0..4000u32 {
        let t = f64::from(i) * 0.618_033_988_75;
        let z = 1.0 - 2.0 * (f64::from(i) + 0.5) / 4000.0;
        let r = (1.0 - z * z).max(0.0).sqrt();
        let a = t * std::f64::consts::TAU;
        let dir = [r * a.cos(), z, r * a.sin()];
        if relief.height_m(dir) >= 500.0 {
            return dir;
        }
    }
    panic!("no land on the home planet");
}

/// Camera cells `above_m` over the ground in unit direction `dir`.
fn above(frame: &PlanetFrame, relief: &Relief, dir: [f64; 3], above_m: f64) -> [f64; 3] {
    let r = (frame.radius_m() + relief.height_m(dir) + above_m) / frame.metres_per_cell();
    [dir[0] * r, dir[1] * r, dir[2] * r]
}

/// Height of `camera` above the ground under it, in metres.
fn height(frame: &PlanetFrame, relief: &Relief, camera: [f64; 3]) -> f64 {
    relief.height_above_m(frame.from_cell(camera))
}

#[test]
fn no_sea_level_for_the_ground() {
    // C1 (#892 T1, Q1 D, Q2 A): the old sea floors are ground. The
    // no-pass-through ball sits below the lowest possible ground, so a dive
    // over a basin 3 km under the old sea level comes down to that floor.
    let (mut universe, frame, relief) = at_the_planet();
    let avoid = lowest_ground_avoid_radius(&frame, &relief);
    let mean = frame.radius_m() / frame.metres_per_cell();
    assert!(
        avoid > 0.0 && avoid < mean,
        "avoid ball below the old sea level"
    );
    let dir = basin_direction(&relief, 3_000.0);
    assert!(
        ground_radius_cells(above(&frame, &relief, dir, 0.0), &frame, &relief) > avoid,
        "the deepest basin floor is still outside the avoid ball"
    );
    universe.path.set_offset(above(&frame, &relief, dir, 100.0));
    for _ in 0..600 {
        universe.dive(None, 0.8, DiveMode::Targeted);
    }
    let camera = universe.path.offset();
    let rest = height(&frame, &relief, camera);
    assert!(
        (rest - GROUND_CLEARANCE_M).abs() < 1e-3,
        "rests 3 m above the basin floor, not at the old sea level: {rest} m"
    );
    let from_mean_m = crate::nav::length3(camera) * frame.metres_per_cell() - frame.radius_m();
    assert!(
        from_mean_m < -2_900.0,
        "the camera is under the old sea level: {from_mean_m} m"
    );
}

#[test]
fn dive_stops_above_the_real_ground() {
    // C2 (#892 T1, Q2 A): zooming in toward the planet's centre rests 3 m
    // above the real ground, over land and over the old sea floor, and is
    // never inside the ground on the way.
    let (mut universe, frame, relief) = at_the_planet();
    for dir in [land_direction(&relief), basin_direction(&relief, 1_000.0)] {
        universe
            .path
            .set_offset(above(&frame, &relief, dir, 10_000.0));
        for _ in 0..1200 {
            universe.dive(None, 0.9, DiveMode::Targeted);
            let h = height(&frame, &relief, universe.path.offset());
            assert!(h >= GROUND_CLEARANCE_M - 1e-6, "never inside: {h} m");
        }
        let rest = height(&frame, &relief, universe.path.offset());
        assert!(
            (rest - GROUND_CLEARANCE_M).abs() < 1e-3,
            "rests 3 m above the ground: {rest} m"
        );
    }
}

#[test]
fn dive_slows_near_the_ground() {
    // C5 (#892 T1, Q3 B): holding the dive-in arrow from 10 km, every step
    // is shorter than the one before as the ground gets closer, and the
    // camera comes to rest at 3 m without overshooting.
    let (mut universe, frame, relief) = at_the_planet();
    let dir = land_direction(&relief);
    universe
        .path
        .set_offset(above(&frame, &relief, dir, 10_000.0));
    let mut last_h = height(&frame, &relief, universe.path.offset());
    let mut last_step = f64::INFINITY;
    for _ in 0..400 {
        universe.dive(None, 0.95, DiveMode::Targeted);
        let h = height(&frame, &relief, universe.path.offset());
        assert!(h >= GROUND_CLEARANCE_M - 1e-6, "no overshoot: {h} m");
        let step = last_h - h;
        assert!(step >= -1e-9, "never climbs while diving in: {step} m");
        assert!(
            step <= last_step + 1e-9,
            "slows near the ground: {step} m after {last_step} m"
        );
        last_step = step;
        last_h = h;
    }
    assert!(
        (last_h - GROUND_CLEARANCE_M).abs() < 1e-3,
        "rests at 3 m: {last_h} m"
    );
}
