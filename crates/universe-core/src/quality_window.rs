//! One-second observation windows of the quality controller (#1022 T5,
//! Spec v4 point 7 and R1). Pure, with fixed-size buffers: a frame never
//! allocates (`E-HOT-NOALLOC`).

/// The length of a window in milliseconds of active play.
pub const WINDOW_MS: f64 = 1000.0;

/// Frames a window keeps for its percentiles; frames past it still count
/// as slow or not (one second above 1000 frames a second).
pub const WINDOW_FRAMES: usize = 1024;

/// A slow frame takes longer than this share of the target interval (R1).
pub const SLOW_SHARE: f64 = 1.1;

/// One frame as the controller sees it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FrameSample {
    /// Time since the previous frame, in milliseconds.
    pub interval_ms: f64,
    /// The frame's critical-path work without its intentional waits, when
    /// that timing is reliable (R1); `None` otherwise.
    pub work_ms: Option<f64>,
}

/// The facts of one closed window.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WindowStats {
    /// Frames in the window.
    pub frames: u32,
    /// Frames over 110 % of the target interval.
    pub slow: u32,
    /// The 95th percentile frame interval.
    pub p95_interval_ms: f64,
    /// The 95th percentile work, when every frame's work was reliable.
    pub p95_work_ms: Option<f64>,
}

impl WindowStats {
    /// At least 20 % of the frames are slow and at least two are, so one
    /// very slow frame alone never counts (R1).
    #[must_use]
    pub const fn overloaded(&self) -> bool {
        self.slow >= 2 && self.slow * 5 >= self.frames
    }

    /// At least 95 % of the frames are within 110 % of the target interval
    /// (the frame-delivery target, R1).
    #[must_use]
    pub const fn delivered(&self) -> bool {
        (self.frames - self.slow) * 20 >= self.frames * 19
    }

    /// Reliable spare capacity: p95 work under 85 % of `target_ms` and not
    /// overloaded (R1).
    #[must_use]
    pub fn spare(&self, target_ms: f64) -> bool {
        !self.overloaded() && self.p95_work_ms.is_some_and(|w| w < 0.85 * target_ms)
    }
}

/// Nearest-rank 95th percentile of `values` (0 when empty); reorders them.
fn p95(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let rank = (values.len() * 95).div_ceil(100).max(1) - 1;
    let (_, v, _) = values.select_nth_unstable_by(rank, f64::total_cmp);
    *v
}

/// The window being filled.
#[derive(Debug, Clone)]
pub struct Window {
    intervals: [f64; WINDOW_FRAMES],
    work: [f64; WINDOW_FRAMES],
    kept: usize,
    frames: u32,
    slow: u32,
    elapsed_ms: f64,
    reliable: bool,
}

impl Default for Window {
    fn default() -> Self {
        Window {
            intervals: [0.0; WINDOW_FRAMES],
            work: [0.0; WINDOW_FRAMES],
            kept: 0,
            frames: 0,
            slow: 0,
            elapsed_ms: 0.0,
            reliable: true,
        }
    }
}

impl Window {
    /// Drops the partial window (point 7: partial windows are discarded).
    pub fn clear(&mut self) {
        self.kept = 0;
        self.frames = 0;
        self.slow = 0;
        self.elapsed_ms = 0.0;
        self.reliable = true;
    }

    /// Adds `frame` against a target interval of `target_ms`; returns the
    /// window's facts when this frame closes it, and starts the next.
    pub fn push(&mut self, frame: FrameSample, target_ms: f64) -> Option<WindowStats> {
        if let (Some(i), Some(w)) = (
            self.intervals.get_mut(self.kept),
            self.work.get_mut(self.kept),
        ) {
            *i = frame.interval_ms;
            *w = frame.work_ms.unwrap_or(0.0);
            self.kept += 1;
        }
        self.reliable &= frame.work_ms.is_some();
        self.frames += 1;
        if frame.interval_ms > SLOW_SHARE * target_ms {
            self.slow += 1;
        }
        self.elapsed_ms += frame.interval_ms;
        if self.elapsed_ms < WINDOW_MS {
            return None;
        }
        let kept = self.kept;
        let reliable = self.reliable;
        let stats = WindowStats {
            frames: self.frames,
            slow: self.slow,
            p95_interval_ms: self.intervals.get_mut(..kept).map_or(0.0, p95),
            p95_work_ms: reliable.then(|| self.work.get_mut(..kept).map_or(0.0, p95)),
        };
        self.clear();
        Some(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T60: f64 = 1000.0 / 60.0;

    fn frame(ms: f64) -> FrameSample {
        FrameSample {
            interval_ms: ms,
            work_ms: Some(ms * 0.5),
        }
    }

    #[test]
    fn a_window_closes_after_one_second() {
        let mut w = Window::default();
        let mut closed = None;
        let mut n = 0;
        while closed.is_none() {
            closed = w.push(frame(T60), T60);
            n += 1;
        }
        let stats = closed.expect("closed");
        assert_eq!(stats.frames, n);
        assert!((60..=61).contains(&n), "{n}");
        assert_eq!(stats.slow, 0);
        assert!(stats.delivered() && !stats.overloaded());
        assert!((stats.p95_work_ms.expect("reliable") - T60 * 0.5).abs() < 1e-9);
        assert!(stats.spare(T60));
    }

    #[test]
    fn one_slow_frame_is_not_an_overload() {
        // An isolated spike: one 300 ms frame in a second.
        let stats = WindowStats {
            frames: 43,
            slow: 1,
            p95_interval_ms: 17.0,
            p95_work_ms: None,
        };
        assert!(!stats.overloaded());
        // Two slow frames out of ten are.
        let stats = WindowStats {
            frames: 10,
            slow: 2,
            ..stats
        };
        assert!(stats.overloaded() && !stats.delivered());
        // Without reliable work there is no spare capacity.
        assert!(!WindowStats { slow: 0, ..stats }.spare(T60));
    }

    #[test]
    fn unreliable_work_spoils_the_window_and_clear_drops_it() {
        let mut w = Window::default();
        assert!(
            w.push(
                FrameSample {
                    interval_ms: 500.0,
                    work_ms: None
                },
                T60
            )
            .is_none()
        );
        let stats = w.push(frame(500.0), T60).expect("closed");
        assert_eq!(stats.p95_work_ms, None);
        assert_eq!(stats.slow, 2);
        w.push(frame(600.0), T60);
        w.clear();
        assert!(w.push(frame(600.0), T60).is_none(), "the part was dropped");
    }
}
