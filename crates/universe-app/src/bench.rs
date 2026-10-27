//! Headless `--patch-bench` driver (#994 T1): times the ground patch builds
//! of the fixed set on one thread and prints one `PATCHBENCH` line.
//!
//! Each pass builds every patch of [`universe_core::patch_bench::bench_keys`]
//! on the home planet exactly as a game worker does (full build with its
//! shared-grid tile). One pass warms up, then [`PASSES`] are timed and the
//! median pass is reported, so one slow pass does not move the number.
//! Imports nothing from `universe-render` or `bevy`.

use std::time::Instant;

use universe_core::grid_tile::build_patch_and_tile;
use universe_core::patch_bench::{bench_keys, home_relief};

use crate::protocol::{EXIT_FAIL, EXIT_PASS};

/// Timed passes over the fixed set.
const PASSES: usize = 5;

/// Builds the fixed set [`PASSES`] times plus a warm-up and prints the
/// median milliseconds per patch. Returns the process exit code: 1 when a
/// patch fails to build.
#[must_use]
pub(crate) fn run_patch_bench() -> i32 {
    let (relief, radius_m) = home_relief();
    let keys = bench_keys();
    let mut pass_ms = Vec::with_capacity(PASSES);
    for pass in 0..=PASSES {
        let start = Instant::now();
        for key in &keys {
            if build_patch_and_tile(*key, radius_m, &relief, true).is_none() {
                println!("PATCHBENCH-FAIL patch {key:?} did not build");
                return EXIT_FAIL;
            }
        }
        if pass > 0 {
            pass_ms.push(start.elapsed().as_secs_f64() * 1_000.0);
        }
    }
    pass_ms.sort_by(f64::total_cmp);
    let median = pass_ms.get(PASSES / 2).copied().unwrap_or(f64::NAN);
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: sixty patches, exactly representable"
    )]
    let per_patch = median / keys.len() as f64;
    println!(
        "{}",
        crate::protocol::patch_bench_line(keys.len(), PASSES, per_patch)
    );
    EXIT_PASS
}
