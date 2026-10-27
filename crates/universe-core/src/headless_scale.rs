//! Continuous scaling study through L1-L7, handovers and precision
//! (#998 T1, study).
//!
//! The cells already nest through exact frames ([`Opened`](crate::frame::Opened)): a point
//! re-expressed in the child and back is the same point, and the dive
//! opens and closes at angle gates. This study flies a continuous camera
//! through them and measures what a seamless zoom would cost, before
//! anything visible changes:
//!
//! - the journey in, L1 to L7, with the autopilot's distance-proportional
//!   speed (FM6), and back out, closing every level;
//! - a free-flight pass straight through an L4 marker, opening and closing
//!   it on the way, whose path must stay one straight line;
//! - at every handover the position error of the frame change, in metres
//!   at that level, the direction error of the frame turn, and the speed
//!   change from the last step before to the first step after;
//! - at every step the `f64` spacing at the camera, in metres and as a share
//!   of the step it takes.
//!
//! Pure and deterministic (`E-CORE-NO-BEVY`, `E-DET-TIERS`,
//! `E-REPLAY-CORE`).

use crate::coords::Level;
use crate::frame::Opened;
use crate::nav::{
    AUTOPILOT_RATE, DEMO_SEED, DiveEvent, DiveMode, MarkerIndex, Universe, should_close,
    should_open,
};

/// Steps at 60 frames a second before a leg gives up.
const MAX_STEPS: usize = 200_000;

/// Deepest level of the study: L7, one star with its Oort shells.
pub const STUDY_DEEPEST: u8 = 7;

/// Metres in one cell unit at `level`.
#[must_use]
pub fn cell_m(level: Level) -> f64 {
    1.0 / crate::body::metre(level)
}

fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn length3(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn angle_deg(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (la, lb) = (length3(a), length3(b));
    if la <= 0.0 || lb <= 0.0 {
        return 0.0;
    }
    let cross = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    length3(cross).atan2(dot).to_degrees()
}

/// The `f64` spacing at `position` (cell units): one step of the largest
/// coordinate.
fn spacing(position: [f64; 3]) -> f64 {
    let largest = position.iter().fold(0.0f64, |m, c| m.max(c.abs()));
    if largest > 0.0 {
        largest * f64::EPSILON
    } else {
        f64::MIN_POSITIVE
    }
}

/// One level handover along a path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Handover {
    /// Level before.
    pub from: u8,
    /// Level after.
    pub to: u8,
    /// Position error of the frame change, metres at the finer level: the
    /// point re-expressed across the frame and back.
    pub jump_m: f64,
    /// Direction error of the frame turn and back, degrees.
    pub turn_deg: f64,
    /// The first step after over the last step before, both in metres.
    pub speed_ratio: f64,
}

/// The worst `f64` spacing at the camera on one level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Precision {
    /// Level.
    pub level: u8,
    /// Spacing in metres.
    pub spacing_m: f64,
    /// Spacing over the step taken that frame.
    pub share_of_step: f64,
}

/// What a leg measured.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Leg {
    /// Every handover, in order.
    pub handovers: Vec<Handover>,
    /// The worst spacing per level.
    pub precision: Vec<Precision>,
    /// Frames flown.
    pub frames: usize,
}

impl Leg {
    fn note_precision(&mut self, level: Level, position: [f64; 3], step: f64) {
        let spacing = spacing(position);
        let row = Precision {
            level: level.get(),
            spacing_m: spacing * cell_m(level),
            share_of_step: if step > 0.0 { spacing / step } else { 0.0 },
        };
        match self.precision.iter_mut().find(|p| p.level == row.level) {
            Some(held) => {
                held.spacing_m = held.spacing_m.max(row.spacing_m);
                held.share_of_step = held.share_of_step.max(row.share_of_step);
            }
            None => self.precision.push(row),
        }
    }
}

/// The frame change's errors at `child` (child units) through `opened`:
/// metres at the child level, and degrees for a direction.
fn frame_errors(opened: &Opened, child: [f64; 3], child_level: Level) -> (f64, f64) {
    let parent = opened.to_parent_point(child);
    let back = opened.to_child_point(parent);
    let jump_m = length3(sub3(back, child)) * cell_m(child_level);
    let dir = [0.6, -0.48, 0.64];
    let turned = opened.to_parent_direction(opened.to_child_direction(dir));
    (jump_m, angle_deg(dir, turned))
}

/// Flies the journey in from L1 to L7, then back out to L1. `None` when the
/// journey does not reach L7.
#[must_use]
pub fn journey() -> Option<(Leg, Leg)> {
    let mut universe = Universe::new(DEMO_SEED);
    let factor = (-AUTOPILOT_RATE / 60.0).exp();
    let mut inward = Leg::default();
    let mut target = None;
    let mut last_step_m = 0.0;
    let mut pending: Option<Handover> = None;
    while universe.level().get() < STUDY_DEEPEST {
        inward.frames += 1;
        if inward.frames > MAX_STEPS {
            return None;
        }
        if target.is_none() {
            target = universe.journey_target();
        }
        let level = universe.level();
        let before = universe.path.offset();
        let event = universe.dive(target, factor, DiveMode::Journey);
        match event {
            DiveEvent::Opened(_) => {
                target = None;
                let opened = universe.path.entered()?;
                let child = universe.path.offset();
                let (jump_m, turn_deg) = frame_errors(&opened, child, universe.level());
                // The step that crossed, in parent units.
                last_step_m = length3(sub3(opened.to_parent_point(child), before)) * cell_m(level);
                pending = Some(Handover {
                    from: level.get(),
                    to: universe.level().get(),
                    jump_m,
                    turn_deg,
                    speed_ratio: 0.0,
                });
            }
            DiveEvent::Closed(opened) => target = Some(opened.marker),
            DiveEvent::Moved => {
                let step = length3(sub3(universe.path.offset(), before));
                let step_m = step * cell_m(level);
                if let Some(mut handover) = pending.take() {
                    handover.speed_ratio = if last_step_m > 0.0 {
                        step_m / last_step_m
                    } else {
                        0.0
                    };
                    inward.handovers.push(handover);
                }
                last_step_m = step_m;
                inward.note_precision(level, universe.path.offset(), step);
            }
        }
    }
    // The last handover (into L7) has no step after it on this leg.
    inward.handovers.extend(pending.take());
    // Out: zoom away from the centre, closing every level back to L1.
    let mut outward = Leg::default();
    let mut last_step_m = 0.0;
    let mut pending: Option<Handover> = None;
    while !universe.path.is_at_root() {
        outward.frames += 1;
        if outward.frames > MAX_STEPS {
            return None;
        }
        let level = universe.level();
        let before = universe.path.offset();
        match universe.dive(None, 1.0 / factor, DiveMode::Passing) {
            DiveEvent::Closed(opened) => {
                let parent = universe.path.offset();
                let child = opened.to_child_point(parent);
                let (jump_m, turn_deg) = frame_errors(&opened, child, level);
                last_step_m = length3(sub3(child, before)) * cell_m(level);
                pending = Some(Handover {
                    from: level.get(),
                    to: universe.level().get(),
                    jump_m,
                    turn_deg,
                    speed_ratio: 0.0,
                });
            }
            DiveEvent::Opened(_) => return None,
            DiveEvent::Moved => {
                let step = length3(sub3(universe.path.offset(), before));
                let step_m = step * cell_m(level);
                if let Some(mut handover) = pending.take() {
                    handover.speed_ratio = if last_step_m > 0.0 {
                        step_m / last_step_m
                    } else {
                        0.0
                    };
                    outward.handovers.push(handover);
                }
                last_step_m = step_m;
                outward.note_precision(level, universe.path.offset(), step);
            }
        }
    }
    outward.handovers.extend(pending.take());
    Some((inward, outward))
}

/// A free-flight pass straight through one L4 marker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FreePass {
    /// Whether the pass opened the marker and closed it again.
    pub crossed: bool,
    /// The farthest the path strayed from its straight line, metres at L4.
    pub off_line_m: f64,
    /// The same, as a share of the marker's radius.
    pub off_line_share: f64,
    /// Frames flown.
    pub frames: usize,
}

/// Flies straight through the journey's L4 marker, a third of its radius
/// off its centre, from 8 radii before it to 28 after (the close gate is
/// 20 radii out), at a fixed speed in L4 units (carried into the child
/// through the frame and back), opening at the open gate and closing at the
/// close gate. `None` when the journey does not reach L4.
#[must_use]
pub fn free_pass() -> Option<FreePass> {
    let mut universe = Universe::new(DEMO_SEED);
    let factor = (-AUTOPILOT_RATE / 60.0).exp();
    let mut target = None;
    let mut guard = 0;
    while universe.level().get() < 4 {
        guard += 1;
        if guard > MAX_STEPS {
            return None;
        }
        if target.is_none() {
            target = universe.journey_target();
        }
        match universe.dive(target, factor, DiveMode::Journey) {
            DiveEvent::Opened(_) => target = None,
            DiveEvent::Closed(opened) => target = Some(opened.marker),
            DiveEvent::Moved => {}
        }
    }
    let marker = universe.journey_target()?;
    let centre = universe.marker(MarkerIndex(marker))?;
    let radius = universe.marker_radius();
    let start = [
        centre[0] - 8.0 * radius,
        centre[1] + radius / 3.0,
        centre[2],
    ];
    let direction = [1.0, 0.0, 0.0];
    let step = radius / 400.0;
    universe.path.set_offset(start);
    let (mut opened_once, mut crossed, mut worst) = (false, false, 0.0f64);
    let mut frames = 0;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "E-CAST: a small positive frame count"
    )]
    let total = (36.0 * radius / step) as usize;
    for _ in 0..total {
        frames += 1;
        let at_l4 = universe.level().get() == 4;
        if at_l4 {
            let next = [
                universe.path.offset()[0] + direction[0] * step,
                universe.path.offset()[1] + direction[1] * step,
                universe.path.offset()[2] + direction[2] * step,
            ];
            universe.path.set_offset(next);
            // Off the line, in L4 units.
            let rel = sub3(next, start);
            let off = length3([0.0, rel[1], rel[2]]);
            worst = worst.max(off);
            if !opened_once && should_open(radius, length3(sub3(next, centre))) {
                if !universe.open(MarkerIndex(marker)) {
                    return None;
                }
                opened_once = true;
            }
        } else {
            let opened = universe.path.entered()?;
            let here = universe.path.offset();
            let inner_dir = opened.to_child_direction(direction);
            let inner_step = step / opened.ratio;
            let next = [
                here[0] + inner_dir[0] * inner_step,
                here[1] + inner_dir[1] * inner_step,
                here[2] + inner_dir[2] * inner_step,
            ];
            universe.path.set_offset(next);
            let rel = sub3(opened.to_parent_point(next), start);
            worst = worst.max(length3([0.0, rel[1], rel[2]]));
            if should_close(length3(next)) {
                universe.close()?;
                crossed = universe.level().get() == 4;
            }
        }
    }
    let off_line = worst;
    Some(FreePass {
        crossed,
        off_line_m: off_line * cell_m(Level::new(4)?),
        off_line_share: off_line / radius,
        frames,
    })
}

/// The study's lines: `SCALE-HANDOVER`, `SCALE-PRECISION` and `SCALE-FREE`.
#[must_use]
pub fn lines() -> Vec<String> {
    let mut out = Vec::new();
    let Some((inward, outward)) = journey() else {
        out.push("SCALE-HANDOVER journey did not reach L7".to_string());
        return out;
    };
    for (way, leg) in [("in", &inward), ("out", &outward)] {
        for h in &leg.handovers {
            out.push(format!(
                "SCALE-HANDOVER way={way} from=L{} to=L{} jump_m={:.3e} turn_deg={:.3e} speed_ratio={:.3}",
                h.from, h.to, h.jump_m, h.turn_deg, h.speed_ratio
            ));
        }
        for p in &leg.precision {
            out.push(format!(
                "SCALE-PRECISION way={way} level=L{} spacing_m={:.3e} share_of_step={:.3e}",
                p.level, p.spacing_m, p.share_of_step
            ));
        }
        out.push(format!("SCALE-FRAMES way={way} frames={}", leg.frames));
    }
    match free_pass() {
        Some(f) => out.push(format!(
            "SCALE-FREE crossed={} off_line_m={:.3e} off_line_share={:.3e} frames={}",
            f.crossed, f.off_line_m, f.off_line_share, f.frames
        )),
        None => out.push("SCALE-FREE pass did not reach L4".to_string()),
    }
    out
}

#[cfg(test)]
#[path = "headless_scale_tests.rs"]
mod tests;
