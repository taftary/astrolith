//! The continuous zoom in the headless check (#998 build T7, AC9).
//!
//! Three lines for `--verify`: the dive's speed across every handover in
//! and out (AC1), the frame change's position and direction errors at
//! every handover (AC2), and the drawn set across the zoom in to the
//! star-system space and back: how much the picture changes at a handover
//! against an ordinary frame, and the most markers drawn against the
//! Marker detail budget (AC3). Pure (`E-CORE-NO-BEVY`, `E-DET-TIERS`).

use crate::drawn_set::{
    LayerFades, MarkerDetail, budget_cutoff, drawn_set, leaving_on_open, shown, size_fade,
};
use crate::frame::angular_radius;
use crate::headless_scale::journey_to;
use crate::nav::{AUTOPILOT_RATE, DEMO_SEED, DiveEvent, DiveMode, Universe};
use crate::preview::PreviewCache;

/// Frame time of the walk, seconds (60 frames a second).
const DT: f64 = 1.0 / 60.0;

/// Deepest level of the walk: the star-system space (#998 B4).
const WALK_DEEPEST: u8 = 8;

/// The speed ratios a handover may show: within a third (AC1).
const SPEED_BAND: std::ops::Range<f64> = 2.0 / 3.0..4.0 / 3.0;

/// What the zoom walk measured.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ZoomWalk {
    /// Handovers, in and out.
    pub handovers: usize,
    /// Largest change of the summed fade between two frames at a handover.
    pub worst_handover: f64,
    /// Largest change between two ordinary frames.
    pub worst_plain: f64,
    /// Most markers wanting to draw (fade above zero) in one frame.
    pub most_wanted: usize,
    /// Most markers drawn in one frame under the Normal budget.
    pub most_drawn: usize,
}

/// Walks the journey in to L8 and back out to L1 at 60 frames a second,
/// with the layer fades, the way the window does.
#[must_use]
pub fn zoom_walk() -> ZoomWalk {
    let mut universe = Universe::new(DEMO_SEED);
    let mut previews = PreviewCache::default();
    let mut fades = LayerFades::settled();
    let mut set = Vec::new();
    let factor = (-AUTOPILOT_RATE * DT).exp();
    let mut target = None;
    let mut walk = ZoomWalk::default();
    let budget = MarkerDetail::default().budget();
    let mut angles = Vec::new();
    let mut inward = true;
    for _ in 0..40_000 {
        if inward && universe.level().get() >= WALK_DEEPEST {
            inward = false;
        }
        if !inward && universe.path.is_at_root() {
            break;
        }
        previews.sync(&universe);
        drawn_set(&universe, &previews, &fades, &mut set);
        let before = shown(&set, universe.path.offset());
        let drawn_before = set.clone();
        let event = if inward {
            if target.is_none() {
                target = universe.journey_target();
            }
            universe.dive(target, factor, DiveMode::Journey)
        } else {
            universe.dive(None, 1.0 / factor, DiveMode::Passing)
        };
        let handover = match event {
            DiveEvent::Opened(_) => {
                target = None;
                if let Some(entered) = universe.path.entered() {
                    fades.on_open(&leaving_on_open(&drawn_before, entered), entered);
                }
                true
            }
            DiveEvent::Closed(opened) => {
                if inward {
                    target = Some(opened.marker);
                }
                fades.on_close();
                true
            }
            DiveEvent::Moved => false,
        };
        fades.tick(DT);
        previews.sync(&universe);
        drawn_set(&universe, &previews, &fades, &mut set);
        let camera = universe.path.offset();
        let change = (shown(&set, camera) - before).abs();
        if handover {
            walk.handovers += 1;
            walk.worst_handover = walk.worst_handover.max(change);
        } else {
            walk.worst_plain = walk.worst_plain.max(change);
        }
        let cutoff = budget_cutoff(&set, camera, budget, &mut angles);
        let (mut wanted, mut drawn) = (0, 0);
        for d in &set {
            let v = [
                d.at[0] - camera[0],
                d.at[1] - camera[1],
                d.at[2] - camera[2],
            ];
            let distance = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            if d.weight * size_fade(d.radius, distance) > 0.0 {
                wanted += 1;
                if angular_radius(d.radius, distance) >= cutoff {
                    drawn += 1;
                }
            }
        }
        walk.most_wanted = walk.most_wanted.max(wanted);
        walk.most_drawn = walk.most_drawn.max(drawn);
    }
    walk
}

/// Appends the `SCALE-SPEED`, `SCALE-JUMP` and `SCALE-DRAWN` lines;
/// `true` when all pass.
pub fn verify_continuity(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let Some((inward, outward)) = journey_to(WALK_DEEPEST) else {
        let _ = writeln!(out, "SCALE-SPEED FAIL: the journey did not reach L8");
        return false;
    };
    let mut ok = true;
    for (way, leg) in [("in", &inward), ("out", &outward)] {
        let ratios = leg
            .handovers
            .iter()
            .map(|h| h.speed_ratio)
            .filter(|r| *r > 0.0);
        let low = ratios.clone().fold(f64::INFINITY, f64::min);
        let high = ratios.fold(0.0, f64::max);
        let passed = SPEED_BAND.contains(&low) && SPEED_BAND.contains(&high);
        ok &= passed;
        let _ = writeln!(
            out,
            "SCALE-SPEED way={way} handovers={} ratio_min={low:.3} ratio_max={high:.3} {}",
            leg.handovers.len(),
            pass_word(passed)
        );
    }
    let all = inward.handovers.iter().chain(&outward.handovers);
    let jump_mm = all.clone().map(|h| h.jump_m * 1e3).fold(0.0, f64::max);
    let turn_deg = all.map(|h| h.turn_deg).fold(0.0, f64::max);
    let passed = jump_mm < 1.0 && turn_deg < 1e-9;
    ok &= passed;
    let _ = writeln!(
        out,
        "SCALE-JUMP levels=L1-L8 worst_jump_mm={jump_mm:.3e} worst_turn_deg={turn_deg:.3e} {}",
        pass_word(passed)
    );
    let walk = zoom_walk();
    let budget = MarkerDetail::default().budget();
    let passed = walk.handovers == 14
        && walk.worst_plain <= 2.0
        && walk.worst_handover <= walk.worst_plain + 1.0
        && walk.most_drawn <= budget;
    ok &= passed;
    let _ = writeln!(
        out,
        "SCALE-DRAWN handovers={} worst_handover={:.2} worst_plain={:.2} most_wanted={} most_drawn={} budget={budget} {}",
        walk.handovers,
        walk.worst_handover,
        walk.worst_plain,
        walk.most_wanted,
        walk.most_drawn,
        pass_word(passed)
    );
    ok
}

fn pass_word(passed: bool) -> &'static str {
    if passed { "PASS" } else { "FAIL" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_continuity_lines_pass() {
        let mut out = String::new();
        let ok = verify_continuity(&mut out);
        println!("{out}");
        assert!(ok, "{out}");
        assert_eq!(out.lines().count(), 4, "{out}");
    }
}
