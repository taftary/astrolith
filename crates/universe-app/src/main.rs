//! Binary entry point: windowed nested dive plus headless `--verify` and `--capture`.
//!
//! Without flags this opens the `Universe MVP` window: hover a marker, click
//! to target it, wheel or arrows to dive in and out, Spacebar for the
//! autopilot journey L1 -> L11, `Esc` quits. With `--verify` it runs headless
//! instead: no window is created; the autopilot journey is replayed from the
//! fixed root seed, every opened cell is regenerated twice and byte-compared,
//! the open/close inverse and the pre-entry preview are checked, canonical
//! snapshots print, and the process exits 0 on `VERIFY-OK` or 1 on
//! `VERIFY-FAIL`. With `--capture <dir>` it replays the same scripted journey
//! headless and writes one `level-L<N>.txt` snapshot plus one `frame-L<N>.ppm`
//! plot per level into `<dir>` (visual state the window would draw, as
//! assertable files), exiting 0 on `CAPTURE-OK` or 1 on `CAPTURE-FAIL`.
//! Flag parsing uses `std::env` only.

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

/// Starts the windowed app, `--verify` headless verification, or
/// `--capture <dir>` headless capture (no window in either headless mode).
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|arg| arg == "--capture") {
        match args.get(i + 1) {
            Some(dir) => std::process::exit(run_capture(dir)),
            None => {
                println!("CAPTURE-FAIL missing <dir> after --capture");
                std::process::exit(1);
            }
        }
    }
    if args.iter().any(|arg| arg == "--verify") {
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
        let marker_exists = chain.split_last().is_none_or(|(&marker, parent)| {
            let parent_cell = generate_cell(DEMO_SEED, parent);
            (marker as usize) < parent_cell.points.len()
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

/// Replays the scripted journey headless and writes per-level capture files.
///
/// Same inputs as `--verify` (autopilot journey L1-L11 from the fixed root
/// seed), but the visual state the window would draw is written to `<dir>` as
/// assertable files: one `level-L<N>.txt` snapshot (scale, anchor, seed,
/// marker count, every marker position + radius at fixed precision) and one
/// `frame-L<N>.ppm` plot (marker x/y as dots, deterministic P3 text), plus a
/// `capture.log` manifest. Prints `CAPTURE-OK files=<n> dir=<dir>`.
///
/// Returns the process exit code: 0 when every journey level produced both
/// files, 1 otherwise.
fn run_capture(dir: &str) -> i32 {
    use std::fmt::Write as _;
    const W: usize = 320;
    const H: usize = 200;
    let root = std::path::Path::new(dir);
    if let Err(e) = std::fs::create_dir_all(root) {
        println!("CAPTURE-FAIL cannot create dir {dir}: {e}");
        return 1;
    }
    let (steps, _universe) = replay_autopilot(DEMO_SEED, VERIFY_DT, VERIFY_MAX_SECS);
    let mut ok = true;
    let mut files = 0;
    let mut chain: Vec<u32> = Vec::new();
    for n in MIN_NAV_LEVEL..=MAX_NAV_LEVEL {
        let level = Level::new(n).unwrap_or(Level::MIN);
        if n > MIN_NAV_LEVEL {
            match steps.iter().find(|step| step.level == level) {
                Some(step) => chain.push(step.marker),
                None => {
                    ok = false;
                    continue;
                }
            }
        }
        let cell = generate_cell(DEMO_SEED, &chain);
        let mut snap = String::new();
        let _ = writeln!(
            snap,
            "level={} milestone={} scale={} anchor=\"{}\" seed={} markers={}",
            level_label(level),
            milestone_tag(level),
            scale_label(level),
            scale_anchor(level),
            path_seed(DEMO_SEED, &chain),
            cell.points.len()
        );
        for (i, p) in cell.points.iter().enumerate() {
            let _ = writeln!(
                snap,
                "marker={i} x={:.6} y={:.6} z={:.6} r={:.6}",
                p.position[0], p.position[1], p.position[2], p.radius
            );
        }
        let mut px = vec![0u8; W * H * 3];
        let mut span = 0.0f64;
        for p in &cell.points {
            span = span.max(p.position[0].abs()).max(p.position[1].abs());
        }
        let span = if span > 0.0 { span } else { 1.0 };
        for p in &cell.points {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: normalized to [0, W-1] by construction"
            )]
            #[expect(
                clippy::cast_sign_loss,
                reason = "E-CAST: normalized to [0, W-1] by construction"
            )]
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: W is 320, exactly representable"
            )]
            let cx = ((p.position[0] / span * 0.5 + 0.5) * (W as f64 - 1.0)) as usize;
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: normalized to [0, H-1] by construction"
            )]
            #[expect(
                clippy::cast_sign_loss,
                reason = "E-CAST: normalized to [0, H-1] by construction"
            )]
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: H is 200, exactly representable"
            )]
            let cy = ((p.position[1] / span * 0.5 + 0.5) * (H as f64 - 1.0)) as usize;
            for dy in 0..2 {
                for dx in 0..2 {
                    let (x, y) = (
                        cx.saturating_add(dx).min(W - 1),
                        cy.saturating_add(dy).min(H - 1),
                    );
                    let o = (y * W + x) * 3;
                    if let Some(pixel) = px.get_mut(o..o + 3) {
                        pixel.fill(255);
                    }
                }
            }
        }
        let mut ppm = format!(
            "P3\n# {} markers={}\n{W} {H}\n255\n",
            level_label(level),
            cell.points.len()
        );
        for row in px.chunks_exact(W * 3) {
            for pix in row.chunks_exact(3) {
                let &[r, g, b] = pix else { continue };
                let _ = write!(ppm, "{r} {g} {b} ");
            }
            ppm.push('\n');
        }
        let label = level_label(level).replace(' ', "");
        if std::fs::write(root.join(format!("level-{label}.txt")), snap).is_err() {
            ok = false;
            continue;
        }
        if std::fs::write(root.join(format!("frame-{label}.ppm")), ppm).is_err() {
            ok = false;
            continue;
        }
        files += 2;
    }
    let manifest = format!(
        "levels={} files={files} seed={DEMO_SEED}\n",
        MAX_NAV_LEVEL - MIN_NAV_LEVEL + 1
    );
    if std::fs::write(root.join("capture.log"), manifest).is_err() {
        ok = false;
    } else {
        files += 1;
    }
    if ok {
        println!("CAPTURE-OK files={files} dir={dir}");
        0
    } else {
        println!("CAPTURE-FAIL files={files} dir={dir}");
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
