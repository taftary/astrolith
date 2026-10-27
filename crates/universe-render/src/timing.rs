//! Frame timing for `--screens <dir> --shots <file> --timing` (#819).
//!
//! After a shot settles, the sequencer records [`TIMING_FRAMES`] frame
//! times at its view before taking the picture. [`summarise`] turns them
//! into the mean, the 95th percentile, the worst frame, and how often a
//! long frame follows a short one (the alternation #819 is about), and the
//! same for the frame's CPU work without its waits (#1022 T1,
//! [`crate::frame_work`]). [`timing_header`] opens `timing.txt` with the
//! run's real window and graphics, so no header is written by hand. Pure.

use bevy::prelude::*;

/// Frames timed per shot.
pub const TIMING_FRAMES: usize = 180;

/// Window size of a timing run: the #819 target (Q2 A), not the screenshot
/// size; `--size` sets another (#1022 T1).
pub const TIMING_SIZE: (u32, u32) = (1600, 900);

/// A frame is "short" below this share of the mean and "long" above its
/// inverse, for the alternation count.
const SHORT_SHARE: f64 = 0.6;

/// One shot's frame times, summarised.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FrameSummary {
    /// Mean frame time in milliseconds.
    pub mean_ms: f64,
    /// 95th percentile (nearest rank) in milliseconds.
    pub p95_ms: f64,
    /// Worst frame in milliseconds.
    pub worst_ms: f64,
    /// Share of frame pairs where a short frame is followed by a long one,
    /// in percent.
    pub alt_pct: f64,
}

/// Summarises `frames_ms` (empty gives zeros).
#[must_use]
pub fn summarise(frames_ms: &[f64]) -> FrameSummary {
    if frames_ms.is_empty() {
        return FrameSummary::default();
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: a frame count as a float divisor, intended"
    )]
    let count = frames_ms.len() as f64;
    let mean_ms = frames_ms.iter().sum::<f64>() / count;
    let mut sorted = frames_ms.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = (sorted.len() * 95).div_ceil(100).max(1) - 1;
    let p95_ms = sorted.get(rank).copied().unwrap_or(0.0);
    let worst_ms = sorted.last().copied().unwrap_or(0.0);
    let (short, long) = (mean_ms * SHORT_SHARE, mean_ms / SHORT_SHARE);
    let pairs = frames_ms
        .iter()
        .zip(frames_ms.iter().skip(1))
        .filter(|(a, b)| **a < short && **b > long)
        .count();
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: pair counts as floats for a percentage, intended"
    )]
    let alt_pct = if frames_ms.len() > 1 {
        pairs as f64 * 100.0 / (frames_ms.len() - 1) as f64
    } else {
        0.0
    };
    FrameSummary {
        mean_ms,
        p95_ms,
        worst_ms,
        alt_pct,
    }
}

/// The `TIMING` line for shot `name`: the frame intervals `s`, the CPU
/// work without waits `work`, and the mean main-world, render and wait
/// times `means` (#1022 T1).
///
/// `ready` holds the times from the shot's placement until the ground's
/// ring was built (`ring_ms`, #918 AC3) and until every wanted leaf was
/// (`ready_ms`), each `none` when it never got there.
#[must_use]
pub fn timing_line(
    name: &str,
    s: &FrameSummary,
    work: &FrameSummary,
    means: crate::frame_work::WorkSample,
    ready: (Option<f64>, Option<f64>),
) -> String {
    let ms = |t: Option<f64>| t.map_or_else(|| "none".to_string(), |ms| format!("{ms:.0}"));
    format!(
        "TIMING {name} mean_ms={:.2} p95_ms={:.2} worst_ms={:.2} alt_pct={:.1} work_mean_ms={:.2} work_p95_ms={:.2} main_mean_ms={:.2} render_mean_ms={:.2} wait_mean_ms={:.2} ring_ms={} ready_ms={}",
        s.mean_ms,
        s.p95_ms,
        s.worst_ms,
        s.alt_pct,
        work.mean_ms,
        work.p95_ms,
        means.main_ms,
        means.render_ms,
        means.wait_ms,
        ms(ready.0),
        ms(ready.1)
    )
}

/// What a timing run ran on, for [`timing_header`].
#[derive(Debug, Clone, PartialEq)]
pub struct RunFacts {
    /// Window size in physical pixels, read from the window.
    pub size: (u32, u32),
    /// The window's scale factor.
    pub scale: f32,
    /// Graphics adapter name.
    pub adapter: String,
    /// Graphics backend.
    pub backend: String,
    /// Present mode, as the window asks for it.
    pub present: String,
    /// Ground build threads.
    pub workers: usize,
    /// Whether the GPU pass timers ran (`ASTROLITH_GPU_TIMERS=1`).
    pub gpu_timers: bool,
}

/// The `TIMING-RUN` line that opens `timing.txt` (#1022 T1).
#[must_use]
pub fn timing_header(facts: &RunFacts) -> String {
    format!(
        "TIMING-RUN size={}x{} scale={:.2} adapter=\"{}\" backend={} present={} workers={} timer=cpu-work{}",
        facts.size.0,
        facts.size.1,
        facts.scale,
        facts.adapter,
        facts.backend,
        facts.present,
        facts.workers,
        if facts.gpu_timers { "+gpu-passes" } else { "" }
    )
}

/// Timing state for a `--timing` run: present only then.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct TimingRun {
    /// Window size of the run in physical pixels (#1022 T1).
    pub size: (u32, u32),
    /// Frame times recorded for the current shot, in milliseconds.
    pub frames_ms: Vec<f64>,
    /// The same frames' CPU work without waits, in milliseconds (#1022 T1).
    pub work_ms: Vec<f64>,
    /// The same frames' main-world, render and wait times summed, in
    /// milliseconds (#1022 T1).
    pub sums: crate::frame_work::WorkSample,
    /// One `TIMING` line per finished shot.
    pub lines: Vec<String>,
    /// Milliseconds since the current shot was placed (#918).
    pub settle_ms: f64,
    /// When the ground first had nothing left to build, in milliseconds
    /// after placement (#918).
    pub ready_ms: Option<f64>,
    /// When the ground's ring was first all built, in milliseconds after
    /// placement (#918 AC3).
    pub ring_ms: Option<f64>,
}

impl Default for TimingRun {
    fn default() -> Self {
        Self {
            size: TIMING_SIZE,
            frames_ms: Vec::new(),
            work_ms: Vec::new(),
            sums: crate::frame_work::WorkSample::default(),
            lines: Vec::new(),
            settle_ms: 0.0,
            ready_ms: None,
            ring_ms: None,
        }
    }
}

impl TimingRun {
    /// Records one frame: its interval and its work sample.
    pub fn record(&mut self, interval_ms: f64, work: crate::frame_work::WorkSample) {
        self.frames_ms.push(interval_ms);
        self.work_ms.push(work.work_ms());
        self.sums.main_ms += work.main_ms;
        self.sums.render_ms += work.render_ms;
        self.sums.wait_ms += work.wait_ms;
    }

    /// Ends the current shot: its `TIMING` line, then a clean slate.
    pub fn finish_shot(&mut self, name: &str) {
        let summary = summarise(&self.frames_ms);
        let work = summarise(&self.work_ms);
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: a frame count as a float divisor, intended"
        )]
        let count = self.frames_ms.len().max(1) as f64;
        let means = crate::frame_work::WorkSample {
            main_ms: self.sums.main_ms / count,
            render_ms: self.sums.render_ms / count,
            wait_ms: self.sums.wait_ms / count,
        };
        let ready = (self.ring_ms, self.ready_ms);
        self.lines
            .push(timing_line(name, &summary, &work, means, ready));
        self.frames_ms.clear();
        self.work_ms.clear();
        self.sums = crate::frame_work::WorkSample::default();
        self.settle_ms = 0.0;
        self.ready_ms = None;
        self.ring_ms = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_summarises_frame_times() {
        // C1 (#819 AC1): mean, nearest-rank 95th percentile, worst, and the
        // share of short-then-long pairs.
        let steady = [10.0; 20];
        let s = summarise(&steady);
        assert_eq!(
            (s.mean_ms, s.p95_ms, s.worst_ms, s.alt_pct),
            (10.0, 10.0, 10.0, 0.0)
        );
        // 15 / 90 ms alternation, as measured in #777: every other pair is
        // short-then-long.
        let alternating: Vec<f64> = (0..20)
            .map(|i| if i % 2 == 0 { 15.0 } else { 90.0 })
            .collect();
        let a = summarise(&alternating);
        assert!((a.mean_ms - 52.5).abs() < 1e-9);
        assert_eq!(a.p95_ms, 90.0);
        assert_eq!(a.worst_ms, 90.0);
        assert!(
            (a.alt_pct - 10.0 * 100.0 / 19.0).abs() < 1e-9,
            "{}",
            a.alt_pct
        );
        // Percentile by nearest rank on 1..=100: the 95th value.
        let ramp: Vec<f64> = (1..=100).map(f64::from).collect();
        assert_eq!(summarise(&ramp).p95_ms, 95.0);
        assert_eq!(summarise(&[]), FrameSummary::default());
        assert_eq!(
            timing_line(
                "orbit",
                &a,
                &s,
                crate::frame_work::WorkSample {
                    main_ms: 10.0,
                    render_ms: 4.0,
                    wait_ms: 1.5
                },
                (None, None)
            ),
            "TIMING orbit mean_ms=52.50 p95_ms=90.00 worst_ms=90.00 alt_pct=52.6 work_mean_ms=10.00 work_p95_ms=10.00 main_mean_ms=10.00 render_mean_ms=4.00 wait_mean_ms=1.50 ring_ms=none ready_ms=none"
        );
        // #918 AC3: the ring's and the whole ground's ready times, whole
        // milliseconds.
        assert_eq!(
            timing_line(
                "ground",
                &a,
                &s,
                crate::frame_work::WorkSample {
                    main_ms: 10.0,
                    render_ms: 4.0,
                    wait_ms: 0.0
                },
                (Some(640.2), Some(812.4))
            ),
            "TIMING ground mean_ms=52.50 p95_ms=90.00 worst_ms=90.00 alt_pct=52.6 work_mean_ms=10.00 work_p95_ms=10.00 main_mean_ms=10.00 render_mean_ms=4.00 wait_mean_ms=0.00 ring_ms=640 ready_ms=812"
        );
    }

    #[test]
    fn a_shot_records_work_apart_from_its_waits() {
        // #1022 T1: the work is the longer of the two threads, the waits
        // are kept apart, and finishing a shot clears it for the next.
        let mut run = TimingRun::default();
        assert_eq!(run.size, TIMING_SIZE);
        for _ in 0..4 {
            run.record(
                40.0,
                crate::frame_work::WorkSample {
                    main_ms: 6.0,
                    render_ms: 9.0,
                    wait_ms: 28.0,
                },
            );
        }
        run.finish_shot("calm");
        assert_eq!(
            run.lines,
            [
                "TIMING calm mean_ms=40.00 p95_ms=40.00 worst_ms=40.00 alt_pct=0.0 work_mean_ms=9.00 work_p95_ms=9.00 main_mean_ms=6.00 render_mean_ms=9.00 wait_mean_ms=28.00 ring_ms=none ready_ms=none"
            ]
        );
        assert!(run.frames_ms.is_empty() && run.work_ms.is_empty());
        assert_eq!(run.sums, crate::frame_work::WorkSample::default());
    }

    #[test]
    fn the_header_names_the_real_run() {
        let facts = RunFacts {
            size: (1600, 900),
            scale: 1.0,
            adapter: "Intel(R) UHD Graphics 620".into(),
            backend: "Vulkan".into(),
            present: "AutoVsync".into(),
            workers: 6,
            gpu_timers: false,
        };
        assert_eq!(
            timing_header(&facts),
            "TIMING-RUN size=1600x900 scale=1.00 adapter=\"Intel(R) UHD Graphics 620\" backend=Vulkan present=AutoVsync workers=6 timer=cpu-work"
        );
        let gpu = RunFacts {
            gpu_timers: true,
            ..facts
        };
        assert!(timing_header(&gpu).ends_with("timer=cpu-work+gpu-passes"));
    }
}
