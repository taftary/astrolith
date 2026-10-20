//! Frame timing for `--screens <dir> --shots <file> --timing` (#819).
//!
//! After a shot settles, the sequencer records [`TIMING_FRAMES`] frame
//! times at its view before taking the picture. [`summarise`] turns them
//! into the mean, the 95th percentile, the worst frame, and how often a
//! long frame follows a short one (the alternation #819 is about). Pure.

use bevy::prelude::*;

/// Frames timed per shot.
pub const TIMING_FRAMES: usize = 180;

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

/// The `TIMING` line for shot `name`.
#[must_use]
pub fn timing_line(name: &str, s: &FrameSummary) -> String {
    format!(
        "TIMING {name} mean_ms={:.2} p95_ms={:.2} worst_ms={:.2} alt_pct={:.1}",
        s.mean_ms, s.p95_ms, s.worst_ms, s.alt_pct
    )
}

/// Timing state for a `--timing` run: present only then.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct TimingRun {
    /// Frame times recorded for the current shot, in milliseconds.
    pub frames_ms: Vec<f64>,
    /// One `TIMING` line per finished shot.
    pub lines: Vec<String>,
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
            timing_line("orbit", &a),
            "TIMING orbit mean_ms=52.50 p95_ms=90.00 worst_ms=90.00 alt_pct=52.6"
        );
    }
}
