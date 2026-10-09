//! Headless `--verify` report builder: replay, checks, snapshots.
//!
//! The orchestration moved here from `universe-app` (#86) so every binary
//! that needs the gate shares one implementation instead of duplicating
//! logic (Q6). This module builds the whole stdout report as a `String`
//! and returns the exit code; binaries print the string verbatim, so all
//! programs emit byte-identical output (`E-CORE-HEADLESS`,
//! `E-STDOUT-PROTOCOL`, `E-ONE-PRINTER`).

use crate::coords::Level;
use crate::flight::replay_free_leg;
use crate::r#gen::MarkerKind;
use crate::labels::{level_label, scale_anchor, scale_label};
use crate::nav::{
    DEMO_SEED, JourneyStep, MAX_NAV_LEVEL, MIN_NAV_LEVEL, START_OFFSET, Universe, replay_autopilot,
};
use crate::nest::{
    CLOSE_ANGLE, OPEN_ANGLE, PREVIEW_ANGLE, PREVIEW_CAP, child_ratio, children_brightness,
    generate_cell, level_budget, path_seed, shell_brightness,
};
use crate::snapshot::snapshot_generated;
use crate::stream::STREAM_CAP;

/// Headless replay step, matching a 60 Hz frame.
pub const VERIFY_DT: f64 = 1.0 / 60.0;

/// Replay time budget before the journey counts as stalled.
pub const VERIFY_MAX_SECS: f64 = 600.0;

/// Exit code: every check passed.
pub const EXIT_PASS: i32 = 0;

/// Exit code: a check failed.
pub const EXIT_FAIL: i32 = 1;

/// Exit code: the command line itself was wrong (Q7).
pub const EXIT_USAGE: i32 = 2;

/// Machine-protocol marker: every headless check passed.
pub const VERIFY_OK: &str = "VERIFY-OK";

/// Machine-protocol marker: a headless check failed.
pub const VERIFY_FAIL: &str = "VERIFY-FAIL";

/// Whether the home L4 cell's journey portal sits off the body centre (#384).
///
/// Regenerates the L4 prefix of the fixed journey and checks the picked
/// portal carries an off-centre portal offset: opening a galaxy lands in
/// its disk lane, not at its centre. Pure; the `HOME-PORTAL` verify line
/// prints it in T7.
#[must_use]
pub fn home_portal_off_centre(root: u64) -> bool {
    use crate::nest::{autopilot_candidates, autopilot_marker, autopilot_path};
    let chain = autopilot_path(root);
    let Some(home) = chain.get(..3) else {
        return false;
    };
    let level = Level::new(4).unwrap_or(Level::MIN);
    let seed = path_seed(root, home);
    let cell = generate_cell(root, home);
    let candidates = autopilot_candidates(level, seed, &cell.points);
    autopilot_marker(seed, &candidates).is_some_and(|marker| {
        cell.points
            .get(marker as usize)
            .is_some_and(|point| point.kind == MarkerKind::Portal && point.portal.is_some())
    })
}

/// Maps a level to its milestone tag.
#[must_use]
pub fn milestone_tag(level: Level) -> &'static str {
    match level.get() {
        1..=3 => "M3-cluster",
        4..=9 => "M4-stars",
        10 => "M5-terrain",
        11..=14 => "M6-room",
        _ => "beyond-MVP",
    }
}

/// Formats a check outcome as `PASS`/`FAIL`.
fn flag(passed: bool) -> &'static str {
    if passed { "PASS" } else { "FAIL" }
}

/// Replays the autopilot journey, checks every opened cell, snapshots it.
///
/// Same content `--verify` always printed, now returned instead of printed:
/// one `LEVEL` and one `SNAPSHOT` line per level L1-L14, one `JOURNEY` line
/// per opening, `RATIO` lines, then `VERIFY-OK`/`VERIFY-FAIL`. Returns the
/// full stdout bytes plus the process exit code (`EXIT_PASS` when everything
/// passes, `EXIT_FAIL` otherwise).
#[must_use]
pub fn verify_report() -> (String, i32) {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "universe --verify: nested dive L1-L14, root seed {DEMO_SEED} (ladder: docs/universe/ladder.md)"
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
                    let _ = writeln!(
                        out,
                        "LEVEL {} FAIL: journey never opened it",
                        level_label(level)
                    );
                    continue;
                }
            }
        }
        let first = generate_cell(DEMO_SEED, &chain);
        let second = generate_cell(DEMO_SEED, &chain);
        let determinism = snapshot_generated(&first) == snapshot_generated(&second);
        let budget = level_budget(level);
        let in_budget = !first.points.is_empty() && first.points.len() <= budget.max_count as usize;
        let marker_exists = chain.split_last().is_none_or(|(&marker, parent)| {
            let parent_cell = generate_cell(DEMO_SEED, parent);
            (marker as usize) < parent_cell.points.len()
        });
        let snapshot_matches = step.is_none_or(|step| step.snapshot == snapshot_generated(&first));
        let passed = determinism && in_budget && marker_exists && snapshot_matches;
        ok &= passed;
        let _ = writeln!(
            out,
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
        let _ = writeln!(
            out,
            "SNAPSHOT {} {}",
            level_label(level),
            snapshot_generated(&first)
        );
    }
    for step in &steps {
        let _ = writeln!(
            out,
            "JOURNEY opened={} marker={} at={:.2}s",
            level_label(step.level),
            step.marker,
            step.elapsed
        );
    }
    ok &= verify_preview(&steps, &mut out);
    let reached = universe.level().get() == MAX_NAV_LEVEL;
    ok &= reached;
    let _ = writeln!(
        out,
        "JOURNEY reached={} levels-opened={} {}",
        level_label(universe.level()),
        steps.len(),
        flag(reached)
    );
    ok &= verify_ratios(&mut out);
    ok &= verify_inverse(&mut universe, &mut out);
    ok &= verify_free_leg(&steps, &mut out);
    if ok {
        let _ = writeln!(out, "{VERIFY_OK}");
        (out, EXIT_PASS)
    } else {
        let _ = writeln!(out, "{VERIFY_FAIL}");
        (out, EXIT_FAIL)
    }
}

/// Checks the pre-entry preview at every opening of the journey (R7, #63).
///
/// Per opened level: the target was previewed in the frame before opening,
/// at most `PREVIEW_CAP` markers were previewed, the previewed child
/// positions equal the open cell's markers through the frame (relative
/// 1e-9), and the streaming working set held at most `STREAM_CAP` cells
/// (#152). Then the brightness curves are sampled across `PREVIEW_ANGLE`
/// and `OPEN_ANGLE` for continuity. Appends one `PREVIEW` line per opening
/// plus a summary.
fn verify_preview(steps: &[JourneyStep], out: &mut String) -> bool {
    use std::fmt::Write as _;
    let mut passed = true;
    for step in steps {
        let previewed = step.preview_count >= 1;
        let capped = step.preview_count <= PREVIEW_CAP;
        let exact = step.preview_error <= 1e-9;
        let bounded = step.alive <= STREAM_CAP;
        let line_ok = previewed && capped && exact && bounded;
        passed &= line_ok;
        let _ = writeln!(
            out,
            "PREVIEW entering={} previewed={} (cap {}) position-error={:.2e} alive={} (max {}) anon={} {}",
            level_label(step.level),
            step.preview_count,
            PREVIEW_CAP,
            step.preview_error,
            step.alive,
            STREAM_CAP,
            step.anon_depth,
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
    let _ = writeln!(
        out,
        "PREVIEW brightness-continuous:{} endpoints:{} all-openings:{} {}",
        flag(continuous),
        flag(endpoints),
        flag(all_levels),
        flag(passed)
    );
    passed
}

/// Appends the true child ratio per rung and checks each lies in `(0, 1)`.
fn verify_ratios(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let mut passed = true;
    for n in MIN_NAV_LEVEL..MAX_NAV_LEVEL {
        let level = Level::new(n).unwrap_or(Level::MIN);
        let ratio = child_ratio(level).unwrap_or(f64::NAN);
        let valid = ratio.is_finite() && ratio > 0.0 && ratio < 1.0;
        passed &= valid;
        let _ = writeln!(
            out,
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

/// Checks the scripted free-flight leg after the dive (#152).
///
/// Replays the leg twice for rerun identity, matches its starting snapshot
/// against the dive's own L5 snapshot, and proves the working set stayed
/// within `STREAM_CAP`. Positions print at six decimals, the cross-platform
/// byte-identity rule (`E-DET-TIERS`). Appends three `FREE-LEG` lines.
fn verify_free_leg(steps: &[JourneyStep], out: &mut String) -> bool {
    use std::fmt::Write as _;
    let (leg, end) = replay_free_leg(DEMO_SEED, VERIFY_DT, VERIFY_MAX_SECS);
    let (again, _) = replay_free_leg(DEMO_SEED, VERIFY_DT, VERIFY_MAX_SECS);
    let identical = leg == again;
    let level_five = Level::new(5).unwrap_or(Level::MIN);
    let snapshot_match = steps
        .iter()
        .find(|step| step.level == level_five)
        .is_some_and(|step| step.snapshot == leg.start_snapshot);
    let at_level = leg.level == level_five;
    let start_ok = at_level && snapshot_match;
    let _ = writeln!(
        out,
        "FREE-LEG start={} snapshot-match:{} steps={} {}",
        level_label(leg.level),
        flag(snapshot_match),
        leg.points.len(),
        flag(start_ok),
    );
    let [end_x, end_y, end_z] = end.path.offset();
    let [start_x, start_y, start_z] = leg.start_offset;
    let displacement =
        ((end_x - start_x).powi(2) + (end_y - start_y).powi(2) + (end_z - start_z).powi(2)).sqrt();
    let bounded = leg.alive_max <= STREAM_CAP;
    let _ = writeln!(
        out,
        "FREE-LEG end=[{end_x:.6}, {end_y:.6}, {end_z:.6}] displacement={displacement:.6} alive-max={} (max {}) {}",
        leg.alive_max,
        STREAM_CAP,
        flag(bounded),
    );
    let _ = writeln!(
        out,
        "FREE-LEG repeat-identical:{} {}",
        flag(identical),
        flag(identical),
    );
    start_ok && bounded && identical
}

/// Checks the open/close inverse at every depth of the journey.
///
/// Closing one level and reopening the same marker must restore the camera
/// offset (relative 1e-9). Closing everything must land at the root with a
/// finite camera inside the start radius. A full close then reopen cannot
/// restore the deep offset: thirteen ratios multiply to ~1e-27, below
/// float64 resolution in root units, which is exactly why the notion (5.1)
/// keeps the chain. Appends `INVERSE`.
fn verify_inverse(universe: &mut Universe, out: &mut String) -> bool {
    use std::fmt::Write as _;
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
    let _ = writeln!(
        out,
        "INVERSE closed={} at-root:{} one-level-round-trip:{} landed-inside-start:{} {}",
        closed,
        flag(at_root),
        flag(one_level_exact),
        flag(landed),
        flag(passed)
    );
    passed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nest::{autopilot_candidates, autopilot_marker, autopilot_path};
    use crate::sysgen::PORTAL_LANE_FRACTION;

    #[test]
    fn home_portal_sits_off_centre_on_the_fixed_journey() {
        assert!(
            home_portal_off_centre(DEMO_SEED),
            "the home galaxy portal must live in the disk lane"
        );
        // The lane sits at the documented fraction of the body radius.
        let chain = autopilot_path(DEMO_SEED);
        assert!(chain.len() >= 3, "the journey must reach L4");
        let home = &chain[..3];
        let level = Level::new(4).unwrap_or(Level::MIN);
        let seed = path_seed(DEMO_SEED, home);
        let cell = generate_cell(DEMO_SEED, home);
        let candidates = autopilot_candidates(level, seed, &cell.points);
        let pick = autopilot_marker(seed, &candidates).expect("home L4 pick");
        let point = &cell.points[pick as usize];
        let offset = point.portal.expect("the pick carries a portal");
        let lane = (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
        assert!(
            (lane - point.radius * PORTAL_LANE_FRACTION).abs() < 1e-12,
            "lane {lane} must sit at the documented fraction of {}",
            point.radius
        );
    }
}
