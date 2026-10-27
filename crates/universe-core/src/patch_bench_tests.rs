use super::*;
use crate::cube_sphere::grid_point;
use crate::lod::{PATCH_QUADS, patch_centre_dir};
use crate::relief::reference;

/// Every direction a build of `key` reads on its own grids: the 33 x 33
/// vertices with a two-point ring (inside the face), and the 65 x 65
/// double-resolution error grid.
fn read_dirs(key: PatchKey) -> Vec<[f64; 3]> {
    let mut dirs = Vec::new();
    let n = key.per_edge().expect("per edge") * PATCH_QUADS;
    let (i0, j0) = (key.x * PATCH_QUADS, key.y * PATCH_QUADS);
    for dj in 0..=PATCH_QUADS + 4 {
        for di in 0..=PATCH_QUADS + 4 {
            let (Some(gi), Some(gj)) = ((i0 + di).checked_sub(2), (j0 + dj).checked_sub(2)) else {
                continue;
            };
            if let Some(dir) = grid_point(key.face, gi, gj, n) {
                dirs.push(dir);
            }
        }
    }
    for dj in 0..=PATCH_QUADS * 2 {
        for di in 0..=PATCH_QUADS * 2 {
            if let Some(dir) = grid_point(key.face, i0 * 2 + di, j0 * 2 + dj, n * 2) {
                dirs.push(dir);
            }
        }
    }
    dirs
}

#[test]
fn the_bench_set_is_fixed_and_spread() {
    let keys = bench_keys();
    assert_eq!(keys.len(), 60);
    assert_eq!(keys, bench_keys(), "the same set every run");
    let mut sorted = keys.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), keys.len(), "no patch twice");
    let (relief, radius_m) = home_relief();
    assert!(!relief.is_giant() && radius_m > 1.0e6);
    let heights: Vec<f64> = keys
        .iter()
        .map(|key| relief.height_m(patch_centre_dir(*key).expect("centre")))
        .collect();
    assert!(heights.iter().any(|h| *h > 0.0), "some land");
    assert!(heights.iter().any(|h| *h < 0.0), "some sea");
}

#[test]
fn the_ground_heights_match_the_frozen_reference() {
    // #994 AC1: within 1 mm of the maths before the speed work, at every
    // direction the fixed set's builds read.
    let (relief, _) = home_relief();
    let mut worst = 0.0f64;
    let mut count = 0usize;
    for key in bench_keys() {
        for dir in read_dirs(key) {
            let gap = (relief.height_m(dir) - reference::height_m(&relief, dir)).abs();
            worst = worst.max(gap);
            count += 1;
        }
    }
    assert!(count > 300_000, "{count} directions");
    assert!(worst <= 1.0e-3, "worst gap {worst} m");
}

#[test]
#[ignore = "probe (#994): cargo test --release -p universe-core --lib probe_build -- --ignored --nocapture"]
fn probe_build_time_by_level() {
    // Best of five passes per depth of the fixed set, one thread.
    use std::time::Instant;
    let (relief, radius_m) = home_relief();
    let keys = bench_keys();
    for level in BENCH_LEVELS {
        let set: Vec<PatchKey> = keys.iter().copied().filter(|k| k.level == level).collect();
        let mut best = f64::INFINITY;
        for _ in 0..5 {
            let t = Instant::now();
            for key in &set {
                let _ = crate::grid_tile::build_patch_and_tile(*key, radius_m, &relief, true);
            }
            #[expect(clippy::cast_precision_loss, reason = "twelve patches")]
            let per_patch = t.elapsed().as_secs_f64() * 1e3 / set.len() as f64;
            best = best.min(per_patch);
        }
        println!("PROBE level={level} best_ms_per_patch={best:.3}");
    }
}
