//! Headless `--patch-bench` driver (#994 T1): times the ground patch builds
//! of the fixed set on one thread and prints one `PATCHBENCH` line.
//!
//! Each pass builds every patch of [`universe_core::patch_bench::bench_keys`]
//! on the home planet exactly as a game worker does (full build with its
//! shared-grid tile). One pass warms up, then [`PASSES`] are timed and the
//! median pass is reported, so one slow pass does not move the number.
//! `ms_per_patch` builds each patch with its parent's fine heights at hand,
//! as a refining game build reads them (#994 T3); `cold_ms_per_patch`
//! builds without them. Imports nothing from `universe-render` or `bevy`.

use std::time::Instant;

use universe_core::grid_tile::build_patch_and_tile_reusing;
use universe_core::lod::PatchKey;
use universe_core::lod_reuse::FineGrid;
use universe_core::patch_bench::{bench_keys, home_relief};
use universe_core::relief::Relief;

use crate::protocol::{EXIT_FAIL, EXIT_PASS};

/// Timed passes over the fixed set.
const PASSES: usize = 5;

/// The median milliseconds per patch over [`PASSES`] passes (after one
/// warm-up) building `keys`, each with its entry of `parents`; `None` when
/// a patch fails to build.
fn median_ms(
    keys: &[PatchKey],
    parents: &[Option<FineGrid>],
    relief: &Relief,
    radius_m: f64,
) -> Option<f64> {
    let mut pass_ms = Vec::with_capacity(PASSES);
    for pass in 0..=PASSES {
        let start = Instant::now();
        for (key, parent) in keys.iter().zip(parents) {
            build_patch_and_tile_reusing(*key, radius_m, relief, true, parent.as_ref())?;
        }
        if pass > 0 {
            pass_ms.push(start.elapsed().as_secs_f64() * 1_000.0);
        }
    }
    pass_ms.sort_by(f64::total_cmp);
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: sixty patches, exactly representable"
    )]
    let per_patch = pass_ms.get(PASSES / 2).copied()? / keys.len() as f64;
    Some(per_patch)
}

/// Times the fixed set with and without the parents' fine heights and
/// prints the `PATCHBENCH` line. Returns the process exit code: 1 when a
/// patch fails to build.
#[must_use]
pub(crate) fn run_patch_bench() -> i32 {
    let (relief, radius_m) = home_relief();
    let keys = bench_keys();
    // The parents' grids, built once outside the timing.
    let parents: Vec<Option<FineGrid>> = keys
        .iter()
        .map(|key| {
            let parent = PatchKey::new(key.face, key.level.checked_sub(1)?, key.x / 2, key.y / 2)?;
            build_patch_and_tile_reusing(parent, radius_m, &relief, true, None)?.2
        })
        .collect();
    let none: Vec<Option<FineGrid>> = keys.iter().map(|_| None).collect();
    let warm = median_ms(&keys, &parents, &relief, radius_m);
    let cold = median_ms(&keys, &none, &relief, radius_m);
    let (Some(warm), Some(cold)) = (warm, cold) else {
        println!("PATCHBENCH-FAIL a patch of the fixed set did not build");
        return EXIT_FAIL;
    };
    println!(
        "{}",
        crate::protocol::patch_bench_line(keys.len(), PASSES, warm, cold)
    );
    EXIT_PASS
}
