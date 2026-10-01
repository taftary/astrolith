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
    FLIGHT_DURATION_SECS, LevelNavigationPlugin, UniverseRenderPlugin, canonical_snapshot,
    demo_cell_for_level, demo_cell_seed, demo_constraints_for_level, demo_generated_for_level,
    demo_generator_for_level, flight_camera_distance, flight_crossfade, flight_orbit_angle,
    flight_position, flight_target, level_at_position, level_label, scale_anchor, scale_label,
    window_title_for_level, world_extent, MAX_NAV_LEVEL, MIN_NAV_LEVEL,
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
            "LEVEL {} [{}] scale={} anchor=\"{}\" cell=({},{},{},{}) seed={} points={} determinism={} border={} drift-reject={} {}",
            level_label(level),
            milestone_tag(level),
            scale_label(level),
            scale_anchor(level),
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
    let flight_sweep = verify_flight();
    ok &= flight_sweep;
    let flight_target_check = verify_flight_target();
    ok &= flight_target_check;
    let crossfade = verify_crossfade();
    ok &= crossfade;
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

/// Simulates the Spacebar test flight headlessly over its full duration.
///
/// Steps the journey in half-second increments: the zoom position must fall
/// monotonically from L1 to L11, every level must appear, the orbit must
/// sweep its half turn, and the camera distance to the fixed target site
/// must strictly close. Prints one `FLIGHT-SWEEP` line; returns the pass flag.
fn verify_flight() -> bool {
    let mut seen = [false; 11];
    let mut previous_position = f32::INFINITY;
    let mut previous_distance = f32::INFINITY;
    let mut elapsed = 0.0f32;
    while elapsed <= FLIGHT_DURATION_SECS {
        let position = flight_position(elapsed);
        if position > previous_position {
            println!("FLIGHT-SWEEP FAIL: rose mid-journey at {elapsed:.1}s");
            return false;
        }
        previous_position = position;
        let distance = flight_camera_distance(elapsed);
        if distance > previous_distance {
            println!("FLIGHT-SWEEP FAIL: camera pulled away at {elapsed:.1}s");
            return false;
        }
        previous_distance = distance;
        seen[usize::from(level_at_position(position).get() - 1)] = true;
        elapsed += 0.5;
    }
    let orbit_ok =
        (flight_orbit_angle(FLIGHT_DURATION_SECS) - std::f32::consts::PI).abs() < 1e-6;
    let dive_ok =
        flight_position(0.0) == 10.0 && flight_position(FLIGHT_DURATION_SECS) == 0.0;
    let approach_ok =
        flight_camera_distance(FLIGHT_DURATION_SECS) < flight_camera_distance(0.0) / 2.0;
    let passed = orbit_ok && dive_ok && approach_ok && seen.iter().all(|visited| *visited);
    println!(
        "FLIGHT-SWEEP dive=10.0->0.0:{} orbit-half-turn:{} approach-closes:{} all-levels:{} {}",
        flag(dive_ok),
        flag(orbit_ok),
        flag(approach_ok),
        flag(seen.iter().all(|visited| *visited)),
        flag(passed),
    );
    passed
}

/// Checks the fixed flight target site: same value on repeat reads and
/// inside the L1 volume. Prints one `FLIGHT-TARGET` line.
fn verify_flight_target() -> bool {
    let target = flight_target();
    let fixed = target == flight_target();
    let half = world_extent(Level::new(MIN_NAV_LEVEL).unwrap_or(Level::MIN)) / 2.0;
    let in_bounds =
        target.x.abs() <= half && target.y.abs() <= half && target.z.abs() <= half;
    let passed = fixed && in_bounds;
    println!(
        "FLIGHT-TARGET site=({:.6},{:.6},{:.6}) fixed:{} in-bounds:{} {}",
        target.x,
        target.y,
        target.z,
        flag(fixed),
        flag(in_bounds),
        flag(passed),
    );
    passed
}

/// Checks the flight crossfade: integer rungs show the parent alone, bands
/// between rungs overlap parent and child with the child directly below the
/// parent, and all ten transitions appear. Prints one `FLIGHT-XFADE` line.
fn verify_crossfade() -> bool {
    let mut rungs_solo = true;
    for rung in 0..=10u8 {
        let (_, child, _) = flight_crossfade(f32::from(rung));
        rungs_solo &= child.is_none();
    }
    let mut transitions = [false; 10];
    let mut bands_overlap = true;
    let mut position = 0.0f32;
    while position <= 10.0 {
        let (parent, child, weight) = flight_crossfade(position);
        if let Some(child) = child {
            bands_overlap &= (0.0..=1.0).contains(&weight) && child.get() == parent.get() + 1;
            transitions[usize::from(parent.get() - 1)] = true;
        }
        position += 0.125;
    }
    let passed =
        rungs_solo && bands_overlap && transitions.iter().all(|seen| *seen);
    println!(
        "FLIGHT-XFADE solo-at-rungs:{} overlap-in-bands:{} all-transitions:{} {}",
        flag(rungs_solo),
        flag(bands_overlap),
        flag(transitions.iter().all(|seen| *seen)),
        flag(passed),
    );
    passed
}
