//! Headless developer-marker continuity probe for `--verify` (#1128 T7).
//!
//! Builds the semantic registry of every cell on the demo journey twice,
//! then zooms slowly in and out of every registered part. It counts:
//!
//! - `flickers`: a stage that turns back against the zoom direction;
//! - `duplicates`: an identity registered more than once;
//! - `anchor_jumps`: a part whose anchor or extent differs between two
//!   builds of the same cell (the camera never moves a part);
//! - `colour_jumps`: a part whose colour role differs between builds or
//!   stages.
//!
//! Appends one `MARKER-CONTINUITY` line; passes when all four are zero.

use crate::home::autopilot_path;
use crate::nav::DEMO_SEED;
use crate::nest::{generate_cell, path_level};
use crate::semantic::{PartRecord, classify_cell};
use crate::semantic_display::{Stage, apparent_angle, select_stage};

/// Geometric zoom steps from far to near (and back) per part.
const ZOOM_STEPS: u32 = 400;

/// Parts zoomed per cell, at most (the whole cell is still checked for
/// duplicates and jumps); bounded so the probe stays fast.
const ZOOMED_PER_CELL: usize = 400;

/// Counts stage turn-backs zooming `record` in and then out.
fn flickers(record: &PartRecord) -> u32 {
    let spec = record.spec();
    let extent = record.extent.max(1e-12);
    let mut count = 0;
    for inward in [true, false] {
        let mut stage = Stage::Hidden;
        let mut last = None;
        for step in 0..=ZOOM_STEPS {
            let t = f64::from(step) / f64::from(ZOOM_STEPS);
            let t = if inward { t } else { 1.0 - t };
            // From a million extents to just outside the part.
            let distance = extent * 1.0e6_f64.powf(1.0 - t) * 1.01_f64.powf(t);
            stage = select_stage(&spec, apparent_angle(extent, distance), stage, false);
            if let Some(previous) = last {
                let back = if inward {
                    stage < previous
                } else {
                    stage > previous
                };
                if back {
                    count += 1;
                }
            }
            last = Some(stage);
        }
    }
    count
}

/// Runs the probe and appends its line; returns whether it passed.
pub(crate) fn verify_marker_continuity(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let path = autopilot_path(DEMO_SEED);
    let (mut flicker, mut duplicates, mut anchor_jumps, mut colour_jumps) =
        (0u32, 0u32, 0u32, 0u32);
    let (mut first, mut second) = (Vec::new(), Vec::new());
    for depth in 0..=path.len().min(9) {
        let Some(chain) = path.get(..depth) else {
            break;
        };
        let level = path_level(chain);
        let cell = generate_cell(DEMO_SEED, chain);
        first.clear();
        second.clear();
        classify_cell(level, &cell, None, &mut first);
        classify_cell(level, &generate_cell(DEMO_SEED, chain), None, &mut second);
        duplicates += u32::try_from(
            first
                .windows(2)
                .filter(|pair| matches!(pair, [a, b] if a.id == b.id))
                .count(),
        )
        .unwrap_or(u32::MAX);
        if first.len() != second.len() {
            anchor_jumps += 1;
        }
        for (a, b) in first.iter().zip(&second) {
            if a.id != b.id || a.anchor != b.anchor || a.extent != b.extent {
                anchor_jumps += 1;
            }
            if a.spec().colour != b.spec().colour {
                colour_jumps += 1;
            }
        }
        for record in first.iter().take(ZOOMED_PER_CELL) {
            flicker += flickers(record);
        }
    }
    let passed = flicker == 0 && duplicates == 0 && anchor_jumps == 0 && colour_jumps == 0;
    let _ = writeln!(
        out,
        "MARKER-CONTINUITY flickers={flicker} duplicates={duplicates} anchor_jumps={anchor_jumps} colour_jumps={colour_jumps} {}",
        if passed { "PASS" } else { "FAIL" }
    );
    passed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_journey_markers_are_continuous() {
        let mut out = String::new();
        assert!(verify_marker_continuity(&mut out), "{out}");
        assert_eq!(
            out,
            "MARKER-CONTINUITY flickers=0 duplicates=0 anchor_jumps=0 colour_jumps=0 PASS\n"
        );
    }
}
