//! Planet levels L11-L14 as shells around the planet (#632, Spec v2).
//!
//! End to end through the public API: the shown level follows the height
//! above the ground anywhere on the planet, nothing opens below it, the
//! dive comes to rest in L14 above its target point, and climbing returns
//! through every shell to L10 and out of the planet.

use universe_core::r#gen::{Form, MarkerKind};
use universe_core::nav::{DEMO_SEED, DiveEvent, MarkerIndex, Universe, WHEEL_FACTOR};
use universe_core::nest::autopilot_path;
use universe_core::player::PlayerSpot;
use universe_core::shell::metres_per_unit;
use universe_core::terrain::surface_radius;

/// The demo journey opened down to its planet (L10).
fn on_planet() -> Universe {
    let mut universe = Universe::new(DEMO_SEED);
    // `if let` instead of `expect` outside `#[test]` fns (`E-NO-PANIC`).
    let chain = autopilot_path(DEMO_SEED);
    assert!(chain.len() >= 9, "the journey reaches a planet");
    for &marker in chain.iter().take(9) {
        assert!(universe.open(MarkerIndex(marker)), "journey opens");
    }
    assert_eq!(universe.level().get(), 10);
    assert_eq!(
        universe.path.planet_metres_per_unit(),
        Some(metres_per_unit(1.0)),
        "the demo planet is Earth"
    );
    universe
}

/// The point `metres` above Earth's ground under `direction`, open-cell
/// units.
fn above(direction: [f64; 3], metres: f64) -> [f64; 3] {
    let [x, y, z] = direction;
    let reach = (x * x + y * y + z * z).sqrt();
    let unit = [x / reach, y / reach, z / reach];
    let radius = surface_radius(unit) + metres / metres_per_unit(1.0);
    unit.map(|c| c * radius)
}

/// Places the camera as on a fresh arrival (no history): the planet level
/// re-reads the shell straight from L10.
fn place(universe: &mut Universe, at: [f64; 3]) -> u8 {
    universe.path.set_offset([0.0, 0.0, 3.0]);
    universe.path.set_offset(at);
    universe.shown_level().get()
}

#[test]
fn the_level_is_the_same_anywhere_on_the_planet() {
    // C4 / AC3: equator, both poles and the far side read the same shells.
    let mut universe = on_planet();
    let spots = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [-0.6, 0.1, -0.8],
        [0.3, -0.5, 0.8],
    ];
    for direction in spots {
        for (metres, want) in [
            (2.0e6, 10u8),
            (5.0e5, 11),
            (5.0e4, 12),
            (500.0, 13),
            (5.0, 14),
        ] {
            assert_eq!(
                place(&mut universe, above(direction, metres)),
                want,
                "{metres} m over {direction:?}"
            );
        }
    }
}

#[test]
fn climbing_above_l11_returns_to_l10() {
    // C6 / AC4: the regions shell holds until 1.5 times its height.
    let mut universe = on_planet();
    let up = [0.0, 1.0, 0.0];
    assert_eq!(place(&mut universe, above(up, 9.0e5)), 11);
    universe.path.set_offset(above(up, 1.4e6));
    assert_eq!(universe.shown_level().get(), 11, "held under 1,500 km");
    universe.path.set_offset(above(up, 1.6e6));
    assert_eq!(universe.shown_level().get(), 10, "back to the planet level");
    assert_eq!(universe.level().get(), 10, "the same open planet");
}

#[test]
fn the_shell_levels_hold_only_the_planet() {
    // C9 / AC7: nothing to enter below the planet: the open planet holds
    // its record, its moons and its scenery, never a portal, and the
    // scenery (equator ring, spin axis) is gone below the regions shell.
    let mut universe = on_planet();
    assert!(
        universe
            .open
            .points
            .iter()
            .all(|point| point.kind == MarkerKind::Population),
        "no portal on the planet"
    );
    assert!(universe.autopilot_target().is_none(), "nothing opens");
    assert!(!universe.open(MarkerIndex(0)), "nothing opens");
    for point in &universe.open.points {
        let kind = if point.surface.is_some() {
            "record"
        } else if point.moon.is_some() {
            "moon"
        } else {
            match point.form {
                Form::Ring { .. } | Form::Thread { .. } => "scenery",
                other => panic!("unexpected {other:?} on the planet"),
            }
        };
        assert!(!kind.is_empty());
    }
    let up = [0.0, 1.0, 0.0];
    place(&mut universe, above(up, 2.0e6));
    assert_eq!(universe.scenery_fade(), 1.0, "scenery shows from orbit");
    place(&mut universe, above(up, 5.0e5));
    assert_eq!(universe.scenery_fade(), 0.0, "and is gone in the shells");
}

#[test]
fn the_dive_rests_in_l14_above_the_target_point() {
    // C10 / AC8: the dive comes down to its target and rests in L14 above
    // the ground: the player's arrival pose, or straight down.
    let spot = PlayerSpot::seeded(DEMO_SEED).expect("a spot");
    let mut universe = Universe::new(DEMO_SEED);
    for &marker in spot.chain() {
        assert!(universe.open(MarkerIndex(marker)), "the way to the player");
    }
    assert_eq!(universe.level().get(), 10, "the player stands on a planet");
    universe.path.set_offset([0.0, 0.0, 3.0]);
    let (pose, _) = spot.arrival_pose();
    let mut seen = Vec::new();
    let mut arrived = false;
    for _ in 0..2000 {
        arrived = universe.approach(pose, WHEEL_FACTOR);
        let height = universe
            .path
            .height_above_ground_m()
            .expect("on the planet");
        assert!(height > 0.0, "never under the ground: {height} m");
        let now = universe.shown_level().get();
        if seen.last() != Some(&now) {
            seen.push(now);
        }
        if arrived {
            break;
        }
    }
    assert!(arrived, "the dive arrives");
    assert_eq!(seen, vec![10, 11, 12, 13, 14], "through every shell");
    let height = universe
        .path
        .height_above_ground_m()
        .expect("on the planet");
    assert!((1.0..10.0).contains(&height), "rests in L14: {height} m");
    // Straight down from where you are when the player is elsewhere.
    let mut universe = on_planet();
    let start = above([0.4, 0.5, -0.6], 3.0e6);
    place(&mut universe, start);
    let rest = universe.ground_rest(start).expect("a rest point");
    while !universe.approach(rest, WHEEL_FACTOR) {}
    assert_eq!(universe.shown_level().get(), 14);
}

#[test]
fn rising_returns_through_the_levels_to_l10() {
    // C11 / AC8: Down arrow climbs by height through every shell, then the
    // planet closes past the close angle, as any cell.
    let mut universe = on_planet();
    place(&mut universe, above([0.0, 1.0, 0.0], 5.0));
    let mut seen = vec![universe.shown_level().get()];
    let mut closed = false;
    for _ in 0..500 {
        let event = universe.ascend(1.0 / WHEEL_FACTOR);
        if let DiveEvent::Closed(_) = event {
            closed = true;
            break;
        }
        let now = universe.shown_level().get();
        if seen.last() != Some(&now) {
            seen.push(now);
        }
    }
    assert_eq!(
        seen,
        vec![14, 13, 12, 11, 10],
        "back up through every shell"
    );
    assert!(closed, "the planet closes into its marker");
    assert_eq!(universe.level().get(), 9);
}

#[test]
fn shells_follow_heights_in_planet_metres() {
    // C3 / AC3 of #648: heights are measured with Earth's real size, so
    // each shell starts at its height in planet metres, not ladder metres.
    let mut universe = on_planet();
    let up = [0.0, 1.0, 0.0];
    for (metres, want) in [
        (1.01e6, 10u8),
        (0.99e6, 11),
        (0.99e5, 12),
        (0.99e3, 13),
        (9.9, 14),
    ] {
        assert_eq!(place(&mut universe, above(up, metres)), want, "{metres} m");
        let read = universe.path.height_above_ground_m().unwrap_or(f64::NAN);
        assert!((read - metres).abs() < 1e-6 * metres, "reads {read} m");
    }
    // 1,000 km in the old ladder metre (10^7.11 m per unit) is 1,413 km in
    // Earth metres: above the regions shell.
    let ladder = 10f64.powf(7.11);
    let old = surface_radius(up) + 1.0e6 / ladder;
    assert_eq!(place(&mut universe, [0.0, old, 0.0]), 10);
    let read = universe.path.height_above_ground_m().unwrap_or(f64::NAN);
    assert!((read / 1.0e6 - metres_per_unit(1.0) / ladder).abs() < 1e-9);
}
