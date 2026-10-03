//! Headless `--capture` driver: replay the journey into files.
//!
//! Imports nothing from `universe-render` or `bevy`: everything the gate
//! needs comes from `universe-core` (the dependency-direction CI step
//! asserts this).

use super::verify::{VERIFY_DT, VERIFY_MAX_SECS, milestone_tag};
use crate::protocol::{EXIT_FAIL, EXIT_PASS, capture_fail, capture_ok};
use universe_core::coords::Level;
use universe_core::labels::{level_label, scale_anchor, scale_label};
use universe_core::nav::{DEMO_SEED, MAX_NAV_LEVEL, MIN_NAV_LEVEL, replay_autopilot};
use universe_core::nest::{generate_cell, path_seed};

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
#[must_use]
pub(crate) fn run_capture(dir: &str) -> i32 {
    use std::fmt::Write as _;
    const W: usize = 320;
    const H: usize = 200;
    let root = std::path::Path::new(dir);
    if let Err(e) = std::fs::create_dir_all(root) {
        println!("CAPTURE-FAIL cannot create dir {dir}: {e}");
        return EXIT_FAIL;
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
        println!("{}", capture_ok(files, dir));
        EXIT_PASS
    } else {
        println!("{}", capture_fail(files, dir));
        EXIT_FAIL
    }
}
