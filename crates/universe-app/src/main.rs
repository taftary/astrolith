//! Binary entry point: windowed nested dive plus headless `--verify`.
//!
//! Without flags this opens the `Universe MVP` window: hover a marker, click
//! to target it, wheel or arrows to dive in and out, Spacebar for the
//! autopilot journey L1 -> L11, `Esc` quits. With `--verify` it runs headless
//! instead: no window is created; the autopilot journey is replayed from the
//! fixed root seed, every opened cell is regenerated twice and byte-compared,
//! the open/close inverse and the pre-entry preview are checked, canonical
//! snapshots print, and the process exits 0 on `VERIFY-OK` or 1 on
//! `VERIFY-FAIL`. Flag parsing uses
//! `std::env` only.

use bevy::prelude::*;
use universe_core::coords::Level;
use universe_core::nest::{
    child_ratio, children_brightness, generate_cell, level_budget, path_seed, shell_brightness,
};
use universe_core::snapshot::snapshot_generated;
use universe_render::{
    CLOSE_ANGLE, DEMO_SEED, DivePlugin, MAX_NAV_LEVEL, MIN_NAV_LEVEL, OPEN_ANGLE, PREVIEW_ANGLE,
    PREVIEW_CAP, START_OFFSET, UniverseRenderPlugin, level_label, replay_autopilot, scale_anchor,
    scale_label, window_title_for_level,
};

/// Headless replay step, matching a 60 Hz frame.
const VERIFY_DT: f64 = 1.0 / 60.0;

/// Replay time budget before the journey counts as stalled.
const VERIFY_MAX_SECS: f64 = 600.0;

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
        .add_plugins((UniverseRenderPlugin, DivePlugin))
        .run();
}

/// Maps a level to its MVP milestone tag.
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

/// Replays the autopilot journey, checks every opened cell, prints snapshots.
///
/// For each level L1-L11 along the journey: the cell regenerates twice from
/// its path and must be byte-identical, the marker count must fit the level
/// budget, and the opened marker must exist in the parent. Then the path is
/// closed all the way back and the camera must land on the start offset
/// (open/close inverse). Prints one `LEVEL` and one `SNAPSHOT` line per
/// level, one `JOURNEY` line per opening, `RATIO` lines, then
/// `VERIFY-OK`/`VERIFY-FAIL`.
///
/// Returns the process exit code: 0 when everything passes, 1 otherwise.
fn run_verify() -> i32 {
    println!(
        "universe --verify: nested dive L1-L11, root seed {DEMO_SEED} (ladder: docs/universe/ladder.md)"
    );
    let (steps, mut universe) = replay_autopilot(DEMO_SEED, VERIFY_DT, VERIFY_MAX_SECS);
    let mut ok = true;
    let mut chain: Vec<u32> = Vec::new();
    for n in MIN_NAV_LEVEL..=MAX_NAV_LEVEL {
        let level = Level::new(n).unwrap_or(Level::MIN);
        let step = steps.iter().find(|step| step.level == level);
        if n > MIN_NAV_LEVEL {
            match step {
                Some(step) => chain.push(step.marker),
                None => {
                    ok = false;
                    println!("LEVEL {} FAIL: journey never opened it", level_label(level));
                    continue;
                }
            }
        }
        let first = generate_cell(DEMO_SEED, &chain);
        let second = generate_cell(DEMO_SEED, &chain);
        let determinism = snapshot_generated(&first) == snapshot_generated(&second);
        let budget = level_budget(level);
        let in_budget = !first.points.is_empty() && first.points.len() <= budget.max_count as usize;
        let marker_exists = chain.last().is_none_or(|&marker| {
            let parent = generate_cell(DEMO_SEED, &chain[..chain.len() - 1]);
            (marker as usize) < parent.points.len()
        });
        let snapshot_matches = step.is_none_or(|step| step.snapshot == snapshot_generated(&first));
        let passed = determinism && in_budget && marker_exists && snapshot_matches;
        ok &= passed;
        println!(
            "LEVEL {} [{}] scale={} anchor=\"{}\" path={:?} seed={} markers={} determinism={} in-budget={} marker-exists={} replay-match={} {}",
            level_label(level),
            milestone_tag(level),
            scale_label(level),
            scale_anchor(level),
            chain,
            path_seed(DEMO_SEED, &chain),
            first.points.len(),
            flag(determinism),
            flag(in_budget),
            flag(marker_exists),
            flag(snapshot_matches),
            flag(passed),
        );
        println!(
            "SNAPSHOT {} {}",
            level_label(level),
            snapshot_generated(&first)
        );
    }
    for step in &steps {
        println!(
            "JOURNEY opened={} marker={} at={:.2}s",
            level_label(step.level),
            step.marker,
            step.elapsed
        );
    }
    ok &= verify_preview(&steps);
    let reached = universe.level().get() == MAX_NAV_LEVEL;
    ok &= reached;
    println!(
        "JOURNEY reached={} levels-opened={} {}",
        level_label(universe.level()),
        steps.len(),
        flag(reached)
    );
    ok &= verify_ratios();
    ok &= verify_inverse(&mut universe);
    if ok {
        println!("VERIFY-OK");
        0
    } else {
        println!("VERIFY-FAIL");
        1
    }
}

/// Checks the pre-entry preview at every opening of the journey (R7, #63).
///
/// Per opened level: the target was previewed in the frame before opening,
/// at most `PREVIEW_CAP` markers were previewed, the previewed child
/// positions equal the open cell's markers through the frame (relative
/// 1e-9), and at most `2 + PREVIEW_CAP` generations were alive. Then the
/// brightness curves are sampled across `PREVIEW_ANGLE` and `OPEN_ANGLE`
/// for continuity. Prints one `PREVIEW` line per opening plus a summary.
fn verify_preview(steps: &[universe_render::JourneyStep]) -> bool {
    let mut passed = true;
    for step in steps {
        let previewed = step.preview_count >= 1;
        let capped = step.preview_count <= PREVIEW_CAP;
        let exact = step.preview_error <= 1e-9;
        let bounded = step.alive <= 2 + PREVIEW_CAP;
        let line_ok = previewed && capped && exact && bounded;
        passed &= line_ok;
        println!(
            "PREVIEW entering={} previewed={} (cap {}) position-error={:.2e} alive={} (max {}) {}",
            level_label(step.level),
            step.preview_count,
            PREVIEW_CAP,
            step.preview_error,
            step.alive,
            2 + PREVIEW_CAP,
            flag(line_ok)
        );
    }
    let eps = 1e-6;
    let continuous = [PREVIEW_ANGLE, OPEN_ANGLE, CLOSE_ANGLE].iter().all(|edge| {
        (shell_brightness(edge - eps) - shell_brightness(edge + eps)).abs() < 1e-4
            && (children_brightness(edge - eps) - children_brightness(edge + eps)).abs() < 1e-4
    });
    let endpoints = shell_brightness(PREVIEW_ANGLE) == 1.0
        && children_brightness(PREVIEW_ANGLE) == 0.0
        && children_brightness(OPEN_ANGLE) == 1.0;
    let all_levels = steps.len() == usize::from(MAX_NAV_LEVEL - MIN_NAV_LEVEL);
    passed &= continuous && endpoints && all_levels;
    println!(
        "PREVIEW brightness-continuous:{} endpoints:{} all-openings:{} {}",
        flag(continuous),
        flag(endpoints),
        flag(all_levels),
        flag(passed)
    );
    passed
}

/// Prints the true child ratio per rung and checks each lies in `(0, 1)`.
fn verify_ratios() -> bool {
    let mut passed = true;
    for n in MIN_NAV_LEVEL..MAX_NAV_LEVEL {
        let level = Level::new(n).unwrap_or(Level::MIN);
        let ratio = child_ratio(level).unwrap_or(f64::NAN);
        let valid = ratio.is_finite() && ratio > 0.0 && ratio < 1.0;
        passed &= valid;
        println!(
            "RATIO L{}->L{} child/parent={:.3e} markers<={} {}",
            n,
            n + 1,
            ratio,
            level_budget(level).max_count,
            flag(valid)
        );
    }
    passed
}

/// Checks the open/close inverse at every depth of the journey.
///
/// Closing one level and reopening the same marker must restore the camera
/// offset (relative 1e-9). Closing everything must land at the root with a
/// finite camera inside the start radius. A full close then reopen cannot
/// restore the deep offset: ten ratios multiply to ~1e-20, below float64
/// resolution in root units, which is exactly why the notion (5.1) keeps
/// the chain. Prints `INVERSE`.
fn verify_inverse(universe: &mut universe_render::Universe) -> bool {
    let mut path = universe.path.clone();
    let mut one_level_exact = true;
    let mut closed = 0;
    while let Some(opened) = path.close() {
        closed += 1;
        let mut reopened = path.clone();
        reopened.open(opened.marker, opened.position);
        let mut back = reopened.clone();
        back.close();
        one_level_exact &= back
            .offset()
            .iter()
            .zip(path.offset())
            .all(|(a, b)| (a - b).abs() <= 1e-9 * b.abs().max(1.0));
    }
    let at_root = path.level() == Level::MIN;
    let start_radius = START_OFFSET.iter().map(|c| c * c).sum::<f64>().sqrt();
    let landed = path.offset().iter().all(|c| c.is_finite())
        && path.distance_to_center() <= start_radius + 1e-9;
    let passed = at_root && one_level_exact && landed;
    println!(
        "INVERSE closed={} at-root:{} one-level-round-trip:{} landed-inside-start:{} {}",
        closed,
        flag(at_root),
        flag(one_level_exact),
        flag(landed),
        flag(passed)
    );
    passed
}
