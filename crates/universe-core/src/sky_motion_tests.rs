//! Sky motion tests (#981 T2), split under `E-FILE-SIZE`.

use crate::nav::{DEMO_SEED, DiveEvent, MarkerIndex, Universe, l8_planet_zone, length3, sub3};

/// The demo journey opened down to the planetary system (L8).
fn system() -> Universe {
    let mut universe = Universe::new(DEMO_SEED);
    for &marker in crate::nest::autopilot_path(DEMO_SEED)
        .get(..7)
        .expect("journey reaches L8")
    {
        assert!(universe.open(MarkerIndex(marker)));
    }
    universe
}

/// The camera's place in L8 units, closing every deeper level on a copy.
fn in_system(universe: &Universe) -> [f64; 3] {
    let mut path = universe.path.clone();
    while path.level().get() > 8 {
        path.close().expect("closes");
    }
    path.offset()
}

#[test]
fn time_zero_moves_nothing() {
    // #981: the journey and its goldens run at time zero, unchanged.
    let mut universe = system();
    let before = universe.open.clone();
    universe.advance(0.0);
    assert_eq!(universe.open, before);
}

#[test]
fn planets_go_around_their_star_with_their_period() {
    // #981 Q1 C: a quarter period turns Earth a quarter turn on its orbit.
    let mut universe = system();
    let earth = universe.journey_target().expect("Earth");
    let info = universe.open.points[earth as usize].planet.expect("planet");
    let start = universe.marker(MarkerIndex(earth)).expect("Earth");
    universe.advance(info.period_days * crate::clock::DAY_S / 4.0);
    let now = universe.marker(MarkerIndex(earth)).expect("Earth");
    let r = |p: [f64; 3]| p[0].hypot(p[2]);
    assert!((r(now) - r(start)).abs() < 1e-12 * r(start), "same orbit");
    let turn = (start[0] * now[0] + start[2] * now[2]) / (r(start) * r(now));
    assert!(turn.abs() < 1e-9, "a quarter turn: cos {turn}");
    assert_eq!(now[1], start[1], "in the ecliptic");
}

#[test]
fn inside_a_planet_the_camera_rides_along() {
    // #981 Q5, Q6: the camera inside a planet's cell keeps its place on the
    // planet while the planet moves around the star.
    let mut universe = system();
    let earth = universe.journey_target().expect("Earth");
    let at = universe.marker(MarkerIndex(earth)).expect("Earth");
    universe.path.set_offset([at[0], at[1] + 1e-6, at[2]]);
    assert!(universe.open_planet(earth));
    let before = universe.path.offset();
    let place_before = in_system(&universe);
    universe.advance(30.0 * crate::clock::DAY_S);
    assert_eq!(
        universe.path.offset(),
        before,
        "the camera keeps its place on the planet"
    );
    let place_after = in_system(&universe);
    assert!(
        length3(sub3(place_after, place_before)) > 1e-4,
        "and moved with the planet through the system"
    );
}

#[test]
fn moons_go_around_their_planet() {
    // #981 Q1 C: the Moon turns once per period around Earth.
    let mut universe = system();
    let earth = universe.journey_target().expect("Earth");
    let at = universe.marker(MarkerIndex(earth)).expect("Earth");
    universe.path.set_offset([at[0], at[1] + 1e-6, at[2]]);
    assert!(universe.open_planet(earth));
    let (index, moon) = universe
        .open
        .points
        .iter()
        .enumerate()
        .find_map(|(i, p)| p.moon.map(|m| (i, m)))
        .expect("the Moon");
    let start = universe.open.points[index].position;
    universe.advance(moon.period_days * crate::clock::DAY_S / 2.0);
    // The planet's cell turned with its ground (#981 T3): undo the spin.
    let spin = universe.path.entered().expect("entry").spin;
    let half = crate::frame::turn_y(universe.open.points[index].position, spin);
    assert!(
        (half[0] + start[0]).abs() < 1e-6 && (half[1] + start[1]).abs() < 1e-6,
        "half a turn"
    );
}

#[test]
fn a_zone_that_reaches_a_still_camera_opens_without_a_jump() {
    // #981 Q6: the camera waits ahead of Earth on its orbit; Earth's gravity
    // zone arrives and opens the planet in one exact step.
    let mut universe = system();
    let earth = universe.journey_target().expect("Earth");
    let info = universe.open.points[earth as usize].planet.expect("planet");
    let zone = l8_planet_zone(&universe, earth).expect("zone");
    // A place on Earth's orbit a little ahead of it (planets run toward
    // shrinking angle in `XZ`, #981 T3).
    let at = universe.marker(MarkerIndex(earth)).expect("Earth");
    let r = at[0].hypot(at[2]);
    let ahead = at[2].atan2(at[0]) - 3.0 * zone / r;
    let wait = [r * ahead.cos(), 0.0, r * ahead.sin()];
    universe.path.set_offset(wait);
    let step = info.period_days * crate::clock::DAY_S * (0.1 * zone / r) / std::f64::consts::TAU;
    let mut opened = false;
    for n in 1..200 {
        universe.advance(f64::from(n) * step);
        if universe.level().get() == 8 {
            assert_eq!(
                universe.path.offset(),
                wait,
                "the camera stays still in space"
            );
        }
        if let Some(DiveEvent::Opened(m)) = universe.sync_zones() {
            assert_eq!(m, earth);
            let back = in_system(&universe);
            assert!(
                length3(sub3(back, wait)) * crate::orbits::metres_per_cell(8) < 1e-3,
                "no jump"
            );
            opened = true;
            break;
        }
    }
    assert!(opened, "Earth's zone reaches the camera");
}

/// The demo journey's planet opened from the system (L10).
fn on_planet() -> Universe {
    let mut universe = system();
    let earth = universe.journey_target().expect("Earth");
    let at = universe.marker(MarkerIndex(earth)).expect("Earth");
    universe.path.set_offset([at[0], at[1] + 1e-6, at[2]]);
    assert!(universe.open_planet(earth));
    universe
}

/// Angle between two directions, degrees.
fn angle_deg(a: [f64; 3], b: [f64; 3]) -> f64 {
    let cos = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]) / (length3(a) * length3(b));
    cos.clamp(-1.0, 1.0).acos().to_degrees()
}

#[test]
fn the_sky_turns_and_the_ground_stays() {
    // #981 T3: a quarter day turns the sun a quarter turn about the pole;
    // the camera keeps its place on the ground.
    let mut universe = on_planet();
    let hours = universe.spin_hours.expect("a day length");
    let before = universe.path.offset();
    let sun = crate::light::sun_direction(&universe).expect("sun");
    universe.advance(hours * 3_600.0 / 4.0);
    assert_eq!(universe.path.offset(), before, "the ground stays put");
    let now = crate::light::sun_direction(&universe).expect("sun");
    assert!((now[1] - sun[1]).abs() < 0.01, "about the pole");
    let flat = |v: [f64; 3]| [v[0], 0.0, v[2]];
    let turn = angle_deg(flat(sun), flat(now));
    assert!(
        turn < 90.0 && turn > 89.0,
        "a quarter turn, less the orbit's share: {turn}"
    );
    // Sunrise in the east: the sun moves toward lower longitude, `-Y`
    // turn (`+X` toward `+Z`).
    let cross_y = sun[2] * now[0] - sun[0] * now[2];
    assert!(cross_y < 0.0, "westward across the sky");
}

#[test]
fn opening_a_spun_planet_keeps_the_camera_place() {
    // #981 T3: a planet opened late in the day is already turned; the
    // camera's place in the system does not jump.
    let mut universe = system();
    universe.advance(5.3 * 3_600.0);
    let earth = universe.journey_target().expect("Earth");
    let at = universe.marker(MarkerIndex(earth)).expect("Earth");
    let wait = [at[0] + 2e-6, at[1] + 1e-6, at[2]];
    universe.path.set_offset(wait);
    assert!(universe.open_planet(earth));
    assert!(universe.path.entered().expect("entry").spin != 0.0);
    let back = in_system(&universe);
    assert!(
        length3(sub3(back, wait)) * crate::orbits::metres_per_cell(8) < 1e-3,
        "no jump"
    );
}
