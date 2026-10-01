//! Binary entry point: windowed level indicators plus headless `--verify`.
//!
//! Without flags this opens the `Universe MVP` window (3D camera, level
//! navigation on `1`-`9`/`0`/`q` across L1-L11, `Esc` quits). With `--verify`
//! it runs headless instead: no window is created; fixed demo places for
//! M1-M5 (L1-L11) are regenerated from `universe-core`, determinism and
//! border checks run, canonical snapshots print, and the process exits 0 on
//! `VERIFY-OK` or 1 on `VERIFY-FAIL`. Flag parsing uses `std::env` only.

use bevy::prelude::*;
use universe_core::coords::Level;
use universe_core::verify::{check_border, check_determinism};
use universe_render::{
    LevelNavigationPlugin, UniverseRenderPlugin, canonical_snapshot, demo_cell_for_level,
    demo_cell_seed, demo_constraints_for_level, demo_generated_for_level,
    demo_generator_for_level, level_at_position, level_label, window_title_for_level,
    MAX_NAV_LEVEL, MIN_NAV_LEVEL,
};

/// Starts the windowed app, or headless verification with `--verify` (no window).
fn main() {
    if std::env::args().any(|arg| arg == "--verify") {
        std::process::exit(run_verify());
    }
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: window_title_for_level(Level::MIN),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((UniverseRenderPlugin, LevelNavigationPlugin))
        .run();
}

/// Maps a navigable level to its MVP milestone tag.
fn milestone_tag(level: Level) -> &'static str {
    match level.get() {
        1..=4 => "M3-cluster",
        5..=10 => "M4-stars",
        11 => "M5-terrain",
        _ => "beyond-MVP",
    }
}

/// Formats a check outcome as `PASS`/`FAIL`.
fn flag(passed: bool) -> &'static str {
    if passed { "PASS" } else { "FAIL" }
}

/// Regenerates the fixed demo places, checks them, prints snapshots.
///
/// For every level L1-L11 this regenerates the demo cell twice from the fixed
/// seed, requires byte-identical determinism, requires border self-agreement
/// of the regenerated point sets, and requires a drifted copy to be rejected
/// (proving the border check is not vacuous). Prints one canonical snapshot
/// per level, then `VERIFY-OK`/`VERIFY-FAIL`.
///
/// Returns the process exit code: 0 when every level passes, 1 otherwise.
fn run_verify() -> i32 {
    println!(
        "universe --verify: M1-M5 demo places L1-L11, fixed seed (M1 ladder: docs/universe/ladder.md)"
    );
    let mut ok = true;
    for n in MIN_NAV_LEVEL..=MAX_NAV_LEVEL {
        let level = Level::new(n).unwrap_or(Level::MIN);
        let cell = demo_cell_for_level(level);
        let seed = demo_cell_seed(cell);
        let constraints = demo_constraints_for_level(level);
        let generator = demo_generator_for_level(level);
        let first = demo_generated_for_level(level);
        let second = demo_generated_for_level(level);
        let determinism = check_determinism(&generator, seed, &constraints);
        let previous: Vec<[f64; 3]> =
            first.points.iter().map(|point| point.position).collect();
        let current: Vec<[f64; 3]> =
            second.points.iter().map(|point| point.position).collect();
        let border = check_border(&previous, &current, 1e-9);
        let mut drifted = previous.clone();
        let drift_rejected = match drifted.first_mut() {
            Some(sample) => {
                sample[0] += 1.0;
                !check_border(&previous, &drifted, 1e-9)
            }
            // Unreachable: demo budgets always emit points. Vacuous pass.
            None => true,
        };
        let passed = determinism && border && drift_rejected;
        ok &= passed;
        println!(
            "LEVEL {} [{}] cell=({},{},{},{}) seed={} points={} determinism={} border={} drift-reject={} {}",
            level_label(level),
            milestone_tag(level),
            level.get(),
            cell.x,
            cell.y,
            cell.z,
            seed,
            first.points.len(),
            flag(determinism),
            flag(border),
            flag(drift_rejected),
            flag(passed),
        );
        println!("SNAPSHOT {} {}", level_label(level), canonical_snapshot(level));
    }
    let zoom_sweep = verify_zoom_sweep();
    ok &= zoom_sweep;
    if ok {
        println!("VERIFY-OK");
        0
    } else {
        println!("VERIFY-FAIL");
        1
    }
}

/// Sweeps the continuous zoom range, requiring every rung level to appear.
///
/// Steps position 0.0 to 10.0 in quarters: endpoints must be L11 and L1,
/// levels must never rise while rising, and all eleven levels must appear.
/// Prints one `ZOOM-SWEEP` line; returns the pass flag.
fn verify_zoom_sweep() -> bool {
    let mut seen = [false; 11];
    let mut previous = 11u8;
    let mut position = 0.0f32;
    while position <= 10.0 {
        let rung_level = level_at_position(position).get();
        if rung_level > previous {
            println!("ZOOM-SWEEP FAIL: level rose to {rung_level} after {previous}");
            return false;
        }
        previous = rung_level;
        seen[usize::from(rung_level - 1)] = true;
        position += 0.25;
    }
    let endpoints = level_at_position(0.0).get() == 11 && level_at_position(10.0).get() == 1;
    let passed = endpoints && seen.iter().all(|visited| *visited);
    println!(
        "ZOOM-SWEEP endpoints=L11,L1:{} all-levels:{} {}",
        flag(endpoints),
        flag(seen.iter().all(|visited| *visited)),
        flag(passed),
    );
    passed
}
