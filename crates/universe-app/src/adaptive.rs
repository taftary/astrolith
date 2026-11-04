//! Adaptive frames in flight (#1085 T6, prototype part `adaptive`).
//!
//! #1085 T4 found that one frame in flight (`inflight1`) removes the
//! VSync-on 40 ms lock (frames alternating two and three refreshes) but is
//! slower with VSync off, and slower than two frames whenever the baseline
//! runs in its good state. So this controller keeps two frames in flight
//! with VSync off, and with VSync on it compares the two limits now and
//! then and keeps the faster:
//!
//! - it measures the mean frame interval over a window of
//!   [`WINDOW_FRAMES`] frames, after [`SETTLE_FRAMES`] frames that let a
//!   change of limit take effect;
//! - every [`PROBE_EVERY_MS`] of holding, it tries the other limit for one
//!   window;
//! - it keeps the probed limit only when its mean is at least
//!   [`KEEP_SHARE`] lower than the held one's last window, else it goes
//!   back.
//!
//! It is pure: [`Adaptive::frame`] takes one frame interval and returns the
//! limit for the next frame. No allocation per frame (`E-HOT-NOALLOC`).

/// The baseline limit (#819).
pub(crate) const TWO: usize = 2;
/// The other limit `adaptive` tries.
pub(crate) const ONE: usize = 1;
/// Frames ignored after a change of limit.
pub(crate) const SETTLE_FRAMES: u32 = 10;
/// Frames in one measuring window.
pub(crate) const WINDOW_FRAMES: u32 = 45;
/// Time spent holding a limit before the next probe, in milliseconds.
pub(crate) const PROBE_EVERY_MS: f64 = 20_000.0;
/// How much faster a probed limit must be to be kept.
pub(crate) const KEEP_SHARE: f64 = 0.03;

/// What the controller is doing.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    /// Holding the current limit; `held_ms` of frames so far.
    Hold { held_ms: f64 },
    /// Trying the current limit against `other`, whose last mean was
    /// `other_mean`.
    Probe { other: usize, other_mean: f64 },
}

/// The adaptive frames-in-flight controller.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Adaptive {
    /// Frames allowed in flight now.
    limit: usize,
    phase: Phase,
    /// Frames still ignored after the last change.
    settle: u32,
    /// Frames and their sum in the current window.
    count: u32,
    sum_ms: f64,
    /// Mean of the last full window at the held limit.
    last_mean: Option<f64>,
    /// Limit changes so far.
    pub(crate) changes: u32,
}

impl Default for Adaptive {
    fn default() -> Self {
        Self {
            limit: TWO,
            // The first window at two is measured, then compared at once.
            phase: Phase::Hold {
                held_ms: PROBE_EVERY_MS,
            },
            settle: SETTLE_FRAMES,
            count: 0,
            sum_ms: 0.0,
            last_mean: None,
            changes: 0,
        }
    }
}

impl Adaptive {
    /// Frames allowed in flight now.
    #[must_use]
    pub(crate) const fn limit(&self) -> usize {
        self.limit
    }

    /// Switches to `limit` and starts settling.
    fn switch(&mut self, limit: usize) {
        if limit != self.limit {
            self.changes += 1;
        }
        self.limit = limit;
        self.settle = SETTLE_FRAMES;
        self.count = 0;
        self.sum_ms = 0.0;
    }

    /// Takes one frame's interval (ms) and whether VSync is on; returns the
    /// limit for the next frame (pure).
    pub(crate) fn frame(&mut self, interval_ms: f64, vsync: bool) -> usize {
        if !vsync {
            // #1085 T4: one frame in flight lost everywhere with VSync off.
            if self.limit != TWO {
                self.switch(TWO);
            }
            self.phase = Phase::Hold {
                held_ms: PROBE_EVERY_MS,
            };
            self.last_mean = None;
            return self.limit;
        }
        if !interval_ms.is_finite() || interval_ms <= 0.0 {
            return self.limit;
        }
        if let Phase::Hold { held_ms } = &mut self.phase {
            *held_ms += interval_ms;
        }
        if self.settle > 0 {
            self.settle -= 1;
            return self.limit;
        }
        self.count += 1;
        self.sum_ms += interval_ms;
        if self.count < WINDOW_FRAMES {
            return self.limit;
        }
        let mean = self.sum_ms / f64::from(self.count);
        self.count = 0;
        self.sum_ms = 0.0;
        match self.phase {
            Phase::Hold { held_ms } => {
                self.last_mean = Some(mean);
                if held_ms >= PROBE_EVERY_MS {
                    let other = if self.limit == TWO { ONE } else { TWO };
                    let held = self.limit;
                    self.switch(other);
                    self.phase = Phase::Probe {
                        other: held,
                        other_mean: mean,
                    };
                }
            }
            Phase::Probe { other, other_mean } => {
                if mean > other_mean * (1.0 - KEEP_SHARE) {
                    self.switch(other);
                    // Re-measure the limit we went back to before holding.
                    self.last_mean = Some(other_mean);
                } else {
                    self.last_mean = Some(mean);
                }
                self.phase = Phase::Hold { held_ms: 0.0 };
            }
        }
        self.limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs `frames` frames where each limit gives its own interval.
    fn run(c: &mut Adaptive, frames: u32, at_one: f64, at_two: f64, vsync: bool) {
        for _ in 0..frames {
            let ms = if c.limit() == ONE { at_one } else { at_two };
            c.frame(ms, vsync);
        }
    }

    /// Frames for one settle plus one window.
    const STEP: u32 = SETTLE_FRAMES + WINDOW_FRAMES;

    #[test]
    fn starts_at_the_baseline() {
        assert_eq!(Adaptive::default().limit(), TWO);
    }

    #[test]
    fn a_locked_baseline_moves_to_one_frame() {
        // T4 Low ground: 39.9 ms at two, 25.0 ms at one.
        let mut c = Adaptive::default();
        run(&mut c, STEP, 25.0, 39.9, true);
        assert_eq!(c.limit(), ONE, "probes one after the first window");
        run(&mut c, STEP, 25.0, 39.9, true);
        assert_eq!(c.limit(), ONE, "keeps the faster limit");
        assert_eq!(c.changes, 1);
    }

    #[test]
    fn a_good_baseline_stays_at_two() {
        // T4 Normal ground in the good state: 25.7 ms at two, 29.3 at one.
        let mut c = Adaptive::default();
        run(&mut c, 2 * STEP, 29.3, 25.7, true);
        assert_eq!(c.limit(), TWO, "the probe lost, so back to two");
        assert_eq!(c.changes, 2);
    }

    #[test]
    fn a_tie_keeps_the_held_limit() {
        // Less than KEEP_SHARE faster is not enough.
        let mut c = Adaptive::default();
        run(&mut c, 2 * STEP, 39.0, 40.0, true);
        assert_eq!(c.limit(), TWO);
    }

    #[test]
    fn it_probes_again_after_holding() {
        let mut c = Adaptive::default();
        run(&mut c, 2 * STEP, 25.0, 39.9, true);
        assert_eq!(c.limit(), ONE);
        // The scene changes: two is now faster. Within the probe interval
        // plus one step, the controller finds it.
        let frames_to_probe = 700 + 2 * STEP; // 700 frames at 29.3 ms is over PROBE_EVERY_MS;
        run(&mut c, frames_to_probe, 29.3, 25.7, true);
        assert_eq!(c.limit(), TWO);
    }

    #[test]
    fn it_does_not_probe_before_the_interval() {
        let mut c = Adaptive::default();
        run(&mut c, 2 * STEP, 25.0, 39.9, true);
        let changes = c.changes;
        // 400 frames at 25 ms is half of PROBE_EVERY_MS.
        run(&mut c, 400, 25.0, 39.9, true);
        assert_eq!(c.changes, changes, "no probe yet");
    }

    #[test]
    fn vsync_off_always_keeps_two() {
        let mut c = Adaptive::default();
        run(&mut c, 2 * STEP, 25.0, 39.9, true);
        assert_eq!(c.limit(), ONE);
        assert_eq!(c.frame(20.0, false), TWO, "VSync off goes back at once");
        run(&mut c, 10 * STEP, 5.0, 40.0, false);
        assert_eq!(c.limit(), TWO, "never probes with VSync off");
    }

    #[test]
    fn bad_intervals_are_ignored() {
        let mut c = Adaptive::default();
        for ms in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
            assert_eq!(c.frame(ms, true), TWO);
        }
        assert_eq!(c, Adaptive::default());
    }
}
