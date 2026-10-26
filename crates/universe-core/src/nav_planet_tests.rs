//! Planet dive tests: arrival, close, no pass-through and planet-centre target.

use crate::nav::{
    DEMO_SEED, DiveMode, MAX_NAV_LEVEL, WHEEL_FACTOR, l10_entry_rest, length3, planet_frame,
    planet_relief, planet_space, planet_space_for_chain, replay_autopilot,
    should_close_with_radius, sub3,
};
use crate::nest::CLOSE_ANGLE;

#[test]
fn arrival_and_close_follow_the_planet() {
    // AC5 / C5 (T3, Q7 A + review item 4): the floor is `3 x` the radius
    // only, a giant arrives at least `3 x` away, and a giant closes farther
    // out than Earth.
    use crate::nest::autopilot_path;
    let entry = l10_entry_rest();
    // Earth is the journey home cell.
    let journey = autopilot_path(DEMO_SEED);
    let home_chain: &[u32] = journey.get(..9).unwrap_or(&[]);
    let earth = planet_space_for_chain(DEMO_SEED, home_chain).expect("Earth space");
    assert!(
        (earth.radius_cells - crate::planet_frame::earth_radius_cells()).abs() < 1e-12,
        "Earth radius {}",
        earth.radius_cells
    );
    // The floor is `3 x`; the journey entry sits above it, so no push.
    assert!(
        (earth.arrival - 3.0 * earth.radius_cells).abs() < 1e-12,
        "Earth floor 3x: {}",
        earth.arrival
    );
    assert!(entry > earth.arrival, "journey clears the floor");
    // #995 T8: the journey arrives at the planet's gravity zone (about
    // 920,000 km for Earth), far above the floor, with no push-out and no
    // turn: the open is an exact origin shift.
    let (_, universe) = replay_autopilot(DEMO_SEED, 1.0 / 60.0, 600.0);
    assert_eq!(universe.level().get(), 10);
    let at = universe.path.distance_to_center();
    assert!(
        at >= earth.arrival,
        "Earth journey arrival {at} under the 3x floor {}",
        earth.arrival
    );
    // A giant: `11 Earth radii` from the sampler's class draws.
    let giant_radius = crate::planet_frame::radius_cells_for(11.0);
    let giant_floor = 3.0 * giant_radius;
    assert!(
        giant_floor > entry,
        "giant floor {giant_floor} must sit beyond Earth's entry {entry}"
    );
    // Find a real giant L10 chain so `planet_space_for_chain` is
    // exercised, not only the formula.
    let mut giant_chain: Option<Vec<u32>> = None;
    for probe in 0..200u32 {
        let chain = vec![1, 1, 1, 1, 1, 1, 1, 1, probe];
        if crate::home::is_home_planet_cell(DEMO_SEED, &chain) {
            continue;
        }
        let frame = crate::planet_frame::PlanetFrame::for_planet(DEMO_SEED, &chain);
        if (frame.radius_cells() - giant_radius).abs() < 1e-12 {
            giant_chain = Some(chain);
            break;
        }
    }
    let chain = giant_chain.expect("a giant L10 chain exists");
    let giant = planet_space_for_chain(DEMO_SEED, &chain).expect("giant space");
    assert!(
        (giant.radius_cells - giant_radius).abs() < 1e-12,
        "giant radius {}",
        giant.radius_cells
    );
    assert!(
        giant.arrival >= 3.0 * giant.radius_cells,
        "giant arrives above 3x"
    );
    // Leaving: Earth closes as today, giants farther out.
    let earth_close = 0.5f64.max(earth.radius_cells) / CLOSE_ANGLE.sin();
    let giant_close = 0.5f64.max(giant.radius_cells) / CLOSE_ANGLE.sin();
    assert!(
        (earth_close - 0.5 / CLOSE_ANGLE.sin()).abs() < 1e-12,
        "Earth closes as today"
    );
    assert!(
        giant_close > earth_close,
        "giant closes {giant_close} beyond Earth {earth_close}"
    );
    assert!(!should_close_with_radius(
        earth_close * 0.99,
        earth.radius_cells.max(0.5)
    ));
    assert!(should_close_with_radius(
        earth_close * 1.01,
        earth.radius_cells.max(0.5)
    ));
    assert!(!should_close_with_radius(
        giant_close * 0.99,
        giant.radius_cells.max(0.5)
    ));
    assert!(should_close_with_radius(
        giant_close * 1.01,
        giant.radius_cells.max(0.5)
    ));
}

#[test]
fn dive_never_enters_a_planet() {
    // AC4/C4 (#873 T3, Q3 A): at L10 a dive toward the centre, an
    // autopilot-style dive and an orbit never land inside the planet;
    // an off-axis step behind the planet slides around it to the target.
    // #892 Q2 A: "inside" is below the real ground (the old sea floors are
    // ground), and the avoid ball sits below the lowest ground.
    use crate::coords::OpenUnits;
    use crate::dive_orbit::{dive_orbit, lowest_ground_avoid_radius, slide_around_sphere};
    let (_, mut universe) = replay_autopilot(DEMO_SEED, 1.0 / 60.0, 600.0);
    assert_eq!(universe.level().get(), MAX_NAV_LEVEL);
    let space = planet_space(&universe).expect("L10 planet space");
    let frame = planet_frame(&universe).expect("L10 frame");
    let relief = planet_relief(&universe).expect("L10 relief").clone();
    let avoid = lowest_ground_avoid_radius(&frame, &relief);
    assert!(avoid > 0.0 && avoid < space.radius_cells);
    // Dive toward the centre (target None): never below the real ground.
    for _ in 0..500 {
        universe.dive(None, WHEEL_FACTOR, DiveMode::Targeted);
        let above = relief.height_above_m(frame.from_cell(universe.path.offset()));
        assert!(
            above >= 0.0,
            "dive toward the centre went under the ground: {above} m"
        );
    }
    // Autopilot-style dives (targeted, small steps) never enter either.
    let (_, universe) = replay_autopilot(DEMO_SEED, 1.0 / 60.0, 600.0);
    assert!(relief.height_above_m(frame.from_cell(universe.path.offset())) >= 0.0);
    // Orbit around the planet centre at the arrival distance stays out.
    let center = OpenUnits([0.0; 3]);
    let mut camera = OpenUnits(universe.path.offset());
    for _ in 0..120 {
        let (next, _) = dive_orbit(camera, center, [0.0, 1.0, 0.0], 0.05, 0.02);
        let slid = slide_around_sphere(camera.0, next.0, [0.0; 3], avoid);
        let d = length3(sub3(slid, [0.0; 3]));
        assert!(
            d >= avoid * (1.0 - 1e-12),
            "orbit entered the planet: {d} < {avoid}"
        );
        camera = OpenUnits(slid);
    }
    // Off-axis step behind the planet slides around it and still makes
    // progress toward the far-side target instead of sticking.
    let from = [avoid * 3.0, 0.0, 0.0];
    let behind = [-avoid * 3.0, avoid * 0.5, 0.0];
    let mut at = from;
    for _ in 0..2000 {
        let wish = [
            at[0] + (behind[0] - at[0]) * 0.02,
            at[1] + (behind[1] - at[1]) * 0.02,
            at[2] + (behind[2] - at[2]) * 0.02,
        ];
        at = slide_around_sphere(at, wish, [0.0; 3], avoid);
        let d = length3(sub3(at, [0.0; 3]));
        assert!(
            d >= avoid * (1.0 - 1e-12),
            "slide entered the planet: {d} < {avoid}"
        );
    }
    let dx = at[0] - behind[0];
    let dy = at[1] - behind[1];
    let dz = at[2] - behind[2];
    let left = (dx * dx + dy * dy + dz * dz).sqrt();
    let start_dx = from[0] - behind[0];
    let start_dy = from[1] - behind[1];
    let start_left = (start_dx * start_dx + start_dy * start_dy).sqrt();
    assert!(
        left < start_left * 0.1,
        "slide goes around to the far-side target: {left} left of {start_left}"
    );
}

#[test]
fn planet_center_is_the_dive_target() {
    // AC5/C5 (#873 T4 Q4 A, Q7 A, Q8 A): from L10 on the default target
    // is the planet centre (L11-L14 stay numbered but unreachable, #742);
    // zooming in stops 3 m above the drawn terrain in dive mode, and
    // orbiting keeps the distance to the centre but never drops below
    // 3 m. Pure, headless.
    use crate::coords::OpenUnits;
    use crate::dive_orbit::{GROUND_CLEARANCE_M, clamp_above_ground, dive_orbit};
    let (_, mut universe) = replay_autopilot(DEMO_SEED, 1.0 / 60.0, 600.0);
    assert_eq!(universe.level().get(), MAX_NAV_LEVEL);
    // Default target at L10: no marker on the way, a centre dive.
    assert!(
        !universe.path.can_open(),
        "L10 holds no portals: the dive is a centre dive"
    );
    let frame = planet_frame(&universe).expect("L10 frame");
    let relief = planet_relief(&universe).expect("L10 relief").clone();
    // Zooming in never drops below 3 m above the terrain.
    for _ in 0..2000 {
        universe.dive(None, WHEEL_FACTOR, DiveMode::Targeted);
        let camera = universe.path.offset();
        let above = relief.height_above_m(frame.from_cell(camera));
        assert!(
            above >= GROUND_CLEARANCE_M - 1e-6,
            "zoom stops 3 m above the ground: {above} m"
        );
    }
    // Stopped: further dives hold the same pose (at the floor, or at the
    // bare surface when the drawn ground dips below it, #873 T3+T4).
    let camera = universe.path.offset();
    let above = relief.height_above_m(frame.from_cell(camera));
    assert!(
        above >= GROUND_CLEARANCE_M - 1e-6,
        "never below the floor: {above} m"
    );
    let held = camera;
    for _ in 0..10 {
        universe.dive(None, WHEEL_FACTOR, DiveMode::Targeted);
    }
    let again = universe.path.offset();
    let moved = length3(sub3(again, held));
    assert!(moved < 1e-9, "zoom stops: {moved} cells of drift");
    // Orbiting at the arrival distance keeps the distance to the centre
    // (within 0.1 %) and stays above the floor.
    let (_, universe) = replay_autopilot(DEMO_SEED, 1.0 / 60.0, 600.0);
    let frame = planet_frame(&universe).expect("frame");
    let relief = planet_relief(&universe).expect("relief").clone();
    let center = OpenUnits([0.0; 3]);
    let mut camera = OpenUnits(universe.path.offset());
    let before = length3(sub3(camera.0, [0.0; 3]));
    for _ in 0..120 {
        let (next, _) = dive_orbit(camera, center, [0.0, 1.0, 0.0], 0.05, 0.02);
        let floored = clamp_above_ground(next.0, &frame, &relief, GROUND_CLEARANCE_M);
        let after = length3(sub3(floored, [0.0; 3]));
        let rel = (after - before).abs() / before;
        assert!(rel < 1e-3, "orbit keeps the distance: {before} -> {after}");
        let above = relief.height_above_m(frame.from_cell(floored));
        assert!(
            above >= GROUND_CLEARANCE_M - 1e-6,
            "orbit never drops below 3 m: {above} m"
        );
        camera = OpenUnits(floored);
    }
    // The floor lifts a below-floor point to exactly 3 m.
    let dir = [0.0, 1.0, 0.0];
    let h = relief.height_m(dir);
    let mpc = frame.metres_per_cell();
    let radius_m = frame.radius_m();
    let low_m = radius_m + h + 1.0;
    let low_cells = [
        dir[0] * low_m / mpc,
        dir[1] * low_m / mpc,
        dir[2] * low_m / mpc,
    ];
    let lifted = clamp_above_ground(low_cells, &frame, &relief, GROUND_CLEARANCE_M);
    let lifted_above = relief.height_above_m(frame.from_cell(lifted));
    assert!(
        (lifted_above - GROUND_CLEARANCE_M).abs() < 1e-6,
        "the floor lifts 1 m to 3 m: {lifted_above}"
    );
}
