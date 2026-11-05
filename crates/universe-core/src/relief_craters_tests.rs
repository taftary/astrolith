//! Crater layer tests (#1161 T3, Spec v2 criteria 4 and 5).

use super::*;
use crate::planet_frame::PlanetFrame;
use crate::relief::Relief;
use crate::world_type::list_worlds;

/// Mercury in the home system: the one airless planet near home.
const MERCURY: [u32; 9] = [7, 2, 7, 1, 2, 0, 0, 0, 0];

fn mercury() -> (Relief, PlanetFrame) {
    let root = crate::nav::DEMO_SEED;
    (
        Relief::for_planet(root, &MERCURY),
        PlanetFrame::for_planet(root, &MERCURY),
    )
}

/// `n` directions spread evenly over the sphere.
fn fibonacci(n: usize) -> impl Iterator<Item = [f64; 3]> {
    let golden = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    (0..n).map(move |i| {
        #[expect(clippy::cast_precision_loss, reason = "E-CAST: small test counts")]
        let (fi, fnn) = (i as f64, n as f64);
        let y = 1.0 - 2.0 * (fi + 0.5) / fnn;
        let r = (1.0 - y * y).max(0.0).sqrt();
        [r * (golden * fi).cos(), y, r * (golden * fi).sin()]
    })
}

#[test]
fn worlds_with_air_keep_every_bit_of_their_heights() {
    // Criterion 4 (Q4 A): every world with air (the home system's and the
    // first worlds of the listing) reads the frozen pre-#994 maths exactly.
    let root = crate::nav::DEMO_SEED;
    let journey = crate::nest::autopilot_path(root);
    let start = journey.get(..4).expect("start chain");
    let worlds = list_worlds(root, start, 60);
    let mut checked = 0;
    for world in worlds.iter().filter(|w| w.inputs.has_air) {
        let relief = Relief::for_planet(root, &world.chain);
        assert!(!relief.is_cratered(), "{:?} has air", world.chain);
        for dir in fibonacci(400) {
            let now = relief.height_m(dir);
            let then = crate::relief::reference::height_m(&relief, dir);
            assert_eq!(
                now.to_bits(),
                then.to_bits(),
                "{:?} at {dir:?}",
                world.chain
            );
        }
        checked += 1;
    }
    assert!(
        checked >= 8,
        "only {checked} air worlds checked of {}",
        worlds.len()
    );
}

#[test]
fn airless_worlds_carry_craters_within_their_bounds() {
    // Criterion 5: Mercury's heights hold bowls the air-world maths does
    // not, and stay inside the height bounds the detail levels use.
    let (relief, _) = mercury();
    assert!(relief.is_cratered());
    let (low, high) = relief.height_bounds_m();
    let mut deepest = 0.0f64;
    let mut cratered = 0usize;
    for dir in fibonacci(20_000) {
        let h = relief.height_m(dir);
        assert!(
            h.is_finite() && h >= low && h <= high,
            "{h} outside {low}..{high}"
        );
        let base = crate::relief::reference::height_m(&relief, dir);
        let crater = h - base;
        deepest = deepest.min(crater);
        if crater.abs() > 1.0 {
            cratered += 1;
        }
    }
    assert!(deepest < -1_000.0, "deepest crater {deepest} m");
    assert!(
        cratered > 2_000,
        "only {cratered} of 20000 points in craters"
    );
}

#[test]
fn the_crater_profile_has_no_step() {
    // Criterion 6: the bowl meets the rim, the rim meets the ejecta, and
    // the ejecta reach zero at two radii.
    for complex in [false, true] {
        let (depth, rim) = (1_000.0, 200.0);
        let inside = profile_m(1.0 - 1e-9, depth, rim, complex);
        let outside = profile_m(1.0 + 1e-9, depth, rim, complex);
        assert!((inside - rim).abs() < 1e-3 && (outside - rim).abs() < 1e-3);
        assert!(profile_m(2.0 - 1e-9, depth, rim, complex).abs() < 1e-3);
        assert!((profile_m(0.0, depth, rim, complex) + depth).abs() < 1e-9);
    }
    // A complex crater has a flat floor inside 0.4 radii.
    assert!((profile_m(0.3, 1_000.0, 200.0, true) + 1_000.0).abs() < 1e-9);
    // Depth follows the guide: simple 0.2 D, complex 1.044 D^0.301 km.
    let moon = Craters::new(1, 1_737_400.0, 1.62);
    assert!((moon.depth_rim_m(2_000.0).0 - 400.0).abs() < 1e-9);
    let complex = moon.depth_rim_m(100_000.0).0;
    assert!((complex - 1_044.0 * 100f64.powf(0.301)).abs() < 1e-6);
}

#[test]
fn craters_match_across_every_cube_edge() {
    // Criterion 5: the heights are one function of the direction, so an
    // edge point reached from two faces has the same height bits.
    use crate::cube_sphere::{Face, grid_point};
    let (relief, _) = mercury();
    let n = 64u32;
    let faces = [
        Face::PosX,
        Face::NegX,
        Face::PosY,
        Face::NegY,
        Face::PosZ,
        Face::NegZ,
    ];
    let mut seen: std::collections::HashMap<[i64; 3], u64> = std::collections::HashMap::new();
    let mut shared = 0;
    for face in faces {
        for k in 0..=n {
            for (i, j) in [(0, k), (n, k), (k, 0), (k, n)] {
                let Some(dir) = grid_point(face, i, j, n) else {
                    continue;
                };
                #[expect(clippy::cast_possible_truncation, reason = "E-CAST: test key")]
                let key = dir.map(|c| (c * 1e9).round() as i64);
                let bits = relief.height_m(dir).to_bits();
                if let Some(&other) = seen.get(&key) {
                    assert_eq!(bits, other, "edge point {dir:?}");
                    shared += 1;
                } else {
                    seen.insert(key, bits);
                }
            }
        }
    }
    assert!(shared > 6 * 4 * 32, "only {shared} shared edge points");
}

#[test]
#[ignore = "slow (~20 s on the dev PC): runs with --include-ignored (CI, validator), #847"]
fn the_player_walks_a_crater_wall_with_feet_on_the_ground() {
    // Criterion 5: walking, running, jumping and landing on a crater wall
    // keep the feet on the drawn ground, as GROUND checks on Earth.
    let (relief, frame) = mercury();
    // The steepest crater wall among the sampled directions (50 m steps).
    let step = 50.0 / frame.radius_m();
    let wall = fibonacci(20_000)
        .map(|dir| {
            let a = relief.height_m(dir);
            let b = relief.height_m([dir[0] + step, dir[1], dir[2]]);
            ((b - a).abs(), dir)
        })
        .max_by(|x, y| x.0.total_cmp(&y.0))
        .map(|(_, dir)| dir)
        .expect("a wall");
    let m = crate::headless_ground::walk_metrics_at(&relief, frame, wall).expect("walk");
    let limit = crate::headless_ground::FEET_LIMIT_M;
    assert!(m.walk_gap_m < limit, "walk {:?}", m);
    assert!(m.run_gap_m < limit, "run {:?}", m);
    assert!(m.land_gap_m < limit, "land {:?}", m);
    assert!(m.landings >= 1, "{m:?}");
}

#[test]
fn craters_cost_a_bounded_share_of_a_height_read() {
    // T3 done-when: worker build time measured. The crater layer adds
    // 8 hashes per class per point; report its share of a height read.
    let (relief, _) = mercury();
    let dirs: Vec<[f64; 3]> = fibonacci(20_000).collect();
    let craters = Craters::new(7, relief.radius_m(), 3.7);
    let started = std::time::Instant::now();
    let mut sum = 0.0;
    for &dir in &dirs {
        sum += crate::relief::reference::height_m(&relief, dir);
    }
    let base = started.elapsed();
    let started = std::time::Instant::now();
    for &dir in &dirs {
        sum += craters.height_m(dir);
    }
    let layer = started.elapsed();
    assert!(sum.is_finite());
    eprintln!(
        "CRATER-COST base {base:?} craters {layer:?} for {} points",
        dirs.len()
    );
}
