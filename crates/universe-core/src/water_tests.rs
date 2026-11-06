//! Water model tests (#1162 T2, Spec v2 criteria 1, 5 and 6).

use super::*;
use crate::relief::Relief;
use crate::water_places::fibonacci_dir;

/// The #1161 fixture worlds by type.
const HOME: [u32; 9] = [7, 2, 7, 1, 2, 0, 0, 0, 5];
const MERCURY: [u32; 9] = [7, 2, 7, 1, 2, 0, 0, 0, 0];
const DESERT: [u32; 9] = [7, 2, 7, 2, 3781, 1, 0, 0, 0];
const IRON: [u32; 9] = [7, 2, 7, 9, 1294, 1, 0, 0, 1];
const ICE: [u32; 9] = [7, 2, 7, 1, 303, 1, 0, 0, 1];
const LAVA: [u32; 9] = [7, 2, 7, 5, 4149, 1, 0, 0, 0];

fn relief(chain: &[u32; 9]) -> Relief {
    Relief::for_planet(crate::nav::DEMO_SEED, chain)
}

#[test]
fn each_type_holds_its_water() {
    // Q1 A: liquid on Earth-like, frozen on ice, dry on the rest.
    assert_eq!(
        WaterKind::for_type(WorldType::EarthLike),
        Some(WaterKind::Liquid)
    );
    assert_eq!(WaterKind::for_type(WorldType::Ice), Some(WaterKind::Frozen));
    for dry in [
        WorldType::Desert,
        WorldType::Iron,
        WorldType::Lava,
        WorldType::Barren,
    ] {
        assert_eq!(WaterKind::for_type(dry), None, "{dry:?}");
    }
    assert_eq!(
        relief(&HOME).water().map(|w| w.kind),
        Some(WaterKind::Liquid)
    );
    assert_eq!(
        relief(&ICE).water().map(|w| w.kind),
        Some(WaterKind::Frozen)
    );
    for chain in [MERCURY, DESERT, IRON, LAVA] {
        assert!(relief(&chain).water().is_none(), "{chain:?}");
    }
}

#[test]
fn dry_worlds_stand_on_their_terrain_bit_for_bit() {
    // Criterion 6: saved positions on dry worlds do not move.
    for chain in [MERCURY, DESERT, IRON, LAVA] {
        let r = relief(&chain);
        for i in 0..2_000 {
            let dir = fibonacci_dir(i, 2_000);
            let ground = r.ground(dir);
            assert_eq!(
                r.height_m(dir).to_bits(),
                r.terrain_m(dir).to_bits(),
                "{chain:?}"
            );
            assert_eq!(ground, GroundSample::dry(r.terrain_m(dir)), "{chain:?}");
        }
    }
}

#[test]
fn the_sea_is_a_flat_floor_over_the_ocean_floor() {
    // Q3 A: under the sea the drawn ground is the sea surface; the ocean
    // floor keeps its depth below it.
    for chain in [HOME, ICE] {
        let r = relief(&chain);
        let (mut sea, mut land) = (0, 0);
        for i in 0..4_000 {
            let dir = fibonacci_dir(i, 4_000);
            let g = r.ground(dir);
            assert_eq!(g.surface_m.to_bits(), r.height_m(dir).to_bits());
            if g.terrain_m < SEA_LEVEL_M {
                sea += 1;
                assert_eq!(g.surface_m, SEA_LEVEL_M, "{chain:?}");
                assert!(g.code.is_water(), "{chain:?}");
                let depth = SEA_LEVEL_M - g.terrain_m;
                let back = g.code.depth_m();
                let want = depth.min(CODE_DEPTH_M);
                assert!((back - want).abs() <= 0.03 * want + 1.5, "{depth} {back}");
            } else {
                land += 1;
                assert_eq!(g, GroundSample::dry(g.terrain_m), "{chain:?}");
            }
        }
        assert!(
            sea > 1_000 && land > 400,
            "{chain:?}: sea {sea}, land {land}"
        );
    }
}

#[test]
fn the_same_seed_gives_the_same_water() {
    for chain in [HOME, ICE] {
        let (a, b) = (relief(&chain), relief(&chain));
        assert_eq!(a, b);
        for i in 0..1_000 {
            let dir = fibonacci_dir(i, 1_000);
            assert_eq!(a.ground(dir), b.ground(dir));
        }
    }
}

#[test]
fn sea_ice_follows_the_polar_caps_and_ice_worlds_freeze_all() {
    // Q4 A: frozen where the Earth-like caps reach (1 - 0.3 x 0.25 of the
    // sine of the latitude, about 67.7 degrees), liquid elsewhere.
    let home = relief(&HOME);
    let water = home.water().expect("home water");
    let at = |lat: f64| crate::water_places::lat_lon_dir(lat, 10.0);
    assert!(water.frozen_at(at(80.0)) && water.frozen_at(at(-75.0)));
    assert!(!water.frozen_at(at(60.0)) && !water.frozen_at(at(0.0)));
    let ice = relief(&ICE);
    let frozen = ice.water().expect("ice water");
    assert!(frozen.frozen_at(at(0.0)) && frozen.frozen_at(at(45.0)));
}

#[test]
fn the_water_code_keeps_land_ring_and_depth_apart() {
    assert!(!WaterCode::LAND.is_water() && !WaterCode::RING.is_water());
    assert_eq!(WaterCode::water(0.001), WaterCode(2));
    assert_eq!(WaterCode::water(1e6), WaterCode(255));
    let mut last = 0.0;
    for depth in [1.0, 5.0, 20.0, 50.0, 100.0, 300.0] {
        let back = WaterCode::water(depth).depth_m();
        assert!(
            back > last && (back - depth).abs() < 0.1 * depth + 0.1,
            "{depth} {back}"
        );
        last = back;
    }
}

#[test]
fn the_waves_follow_the_game_clock() {
    // Criterion 8 (Q6 A): faster time gives faster waves, a paused sky
    // stops them, and the shader's wave time tracks the clock.
    use crate::clock::{Clock, GAME_PER_REAL};
    let mut clock = Clock {
        game_s: 1_000.0,
        ..Clock::default()
    };
    let one = WaveClock::sync(&clock, 5.0);
    clock.faster();
    let ten = WaveClock::sync(&clock, 5.0);
    assert!(ten.rate > one.rate && one.rate > 0.0, "{one:?} {ten:?}");
    // Two real seconds of play at 10x move the waves as far as the clock.
    let mut later = clock;
    later.tick(2.0);
    let expected = (later.game_s - clock.game_s) / GAME_PER_REAL;
    assert!((ten.at(7.0) - ten.at(5.0) - expected).abs() < 1e-9);
    assert!((wave_time_s(later.game_s) - ten.at(7.0)).abs() < 1e-9);
    clock.toggle();
    let paused = WaveClock::sync(&clock, 5.0);
    assert_eq!(paused.rate, 0.0);
    assert_eq!(paused.at(5.0), paused.at(500.0));
    assert!((0.0..WAVE_PERIOD_S).contains(&wave_time_s(1.0e12)));
}

#[test]
#[ignore = "timing report only (run with --release --ignored --nocapture)"]
fn water_costs_a_bounded_share_of_a_height_read() {
    // #1162 T7: the drawn ground on a wet world (sea and lake lookups) per
    // read, against the terrain alone on the same world.
    let home = relief(&HOME);
    let dirs: Vec<[f64; 3]> = (0..200_000).map(|i| fibonacci_dir(i, 200_000)).collect();
    let mut sum = 0.0;
    let t = std::time::Instant::now();
    for &d in &dirs {
        sum += crate::relief::reference::height_m(&home, d);
    }
    let base = t.elapsed();
    let dry = relief(&HOME).with_water(None);
    let t = std::time::Instant::now();
    for &d in &dirs {
        sum += dry.height_m(d);
    }
    let without = t.elapsed();
    let t = std::time::Instant::now();
    for &d in &dirs {
        sum += home.terrain_m(d);
    }
    let terrain = t.elapsed();
    let t = std::time::Instant::now();
    for &d in &dirs {
        sum += home.height_m(d);
    }
    let ground = t.elapsed();
    assert!(sum.is_finite());
    eprintln!(
        "WATER-COST reference {base:?} without-water {without:?} terrain {terrain:?} ground {ground:?} for {} reads",
        dirs.len()
    );
}
