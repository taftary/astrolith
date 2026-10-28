//! The frame cap and its pacing with vsync (#1022 T7, Spec v4 point 6).
//! Pure: the window side sleeps until the deadline this module computes.
//!
//! The 30 or 60 FPS target limits rendering in both modes. With vsync the
//! display paces the frames itself when its refresh equals the target, so
//! no cap is added; at a whole multiple (120 Hz for 60) the cap takes every
//! other refresh. Any other refresh cannot show the target evenly: the cap
//! still holds the target and the game says why in plain words, never
//! changing the target by itself.

use crate::settings_quality::TargetFps;

/// A refresh this close to a whole multiple of the target counts as one
/// (59.94 Hz paces 60 FPS).
const MULTIPLE_TOLERANCE: f64 = 0.02;

/// How the display can show the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pacing {
    /// Every frame shows for the same time.
    Even,
    /// The refresh is not a whole multiple of the target: some frames show
    /// longer than others.
    Uneven,
    /// The display refreshes more slowly than the target.
    BelowTarget,
}

/// What the cap does for one target, vsync and refresh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CapPlan {
    /// The cap's interval in milliseconds; `None` when vsync paces alone.
    pub interval_ms: Option<f64>,
    /// How evenly the display shows the target.
    pub pacing: Pacing,
}

/// The cap for `target`, with `vsync` on or off, on a display refreshing
/// `refresh_hz` times a second when known (pure).
#[must_use]
pub fn cap_plan(target: TargetFps, vsync: bool, refresh_hz: Option<f64>) -> CapPlan {
    let interval = target.interval_ms();
    let capped = |pacing| CapPlan {
        interval_ms: Some(interval),
        pacing,
    };
    let Some(refresh) = refresh_hz.filter(|r| r.is_finite() && *r > 0.0) else {
        return capped(Pacing::Even);
    };
    if !vsync {
        return capped(Pacing::Even);
    }
    let ratio = refresh / f64::from(target.fps());
    if ratio < 1.0 - MULTIPLE_TOLERANCE {
        return CapPlan {
            interval_ms: None,
            pacing: Pacing::BelowTarget,
        };
    }
    let whole = ratio.round();
    if (ratio - whole).abs() > MULTIPLE_TOLERANCE * whole {
        return capped(Pacing::Uneven);
    }
    if whole <= 1.0 {
        // The display's own pace is the target.
        return CapPlan {
            interval_ms: None,
            pacing: Pacing::Even,
        };
    }
    capped(Pacing::Even)
}

/// The plain-words note for an uneven or too-slow display, `None` when the
/// display shows the target evenly (Spec v4 point 6).
#[must_use]
pub fn pacing_note(target: TargetFps, refresh_hz: f64, pacing: Pacing) -> Option<String> {
    let hz = refresh_hz.round();
    let fps = target.fps();
    match pacing {
        Pacing::Even => None,
        Pacing::Uneven => Some(format!(
            "Your display refreshes {hz} times a second, which does not divide evenly into {fps} FPS: some frames stay on screen longer than others. A display at a multiple of {fps} Hz, or VSync off, gives even pacing."
        )),
        Pacing::BelowTarget => Some(format!(
            "Your display refreshes {hz} times a second, so it cannot show {fps} FPS with VSync on."
        )),
    }
}

/// The next deadline after `previous`, `interval_ms` later, and how long to
/// sleep from `now` (all in milliseconds on one clock). A frame that ran
/// past its deadline starts a new one from `now`, so late frames never
/// cause a burst of catch-up frames (pure).
#[must_use]
pub fn next_deadline(previous: Option<f64>, now: f64, interval_ms: f64) -> (f64, f64) {
    let due = previous.map_or(now, |p| p + interval_ms);
    if due <= now {
        (now, 0.0)
    } else {
        (due, due - now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vsync_alone_paces_a_matching_display() {
        let plan = cap_plan(TargetFps::Fps60, true, Some(60.0));
        assert_eq!(plan.interval_ms, None);
        assert_eq!(plan.pacing, Pacing::Even);
        assert_eq!(
            cap_plan(TargetFps::Fps60, true, Some(59.94)).interval_ms,
            None
        );
        // 30 FPS on 60 Hz: every other refresh, by the cap.
        let plan = cap_plan(TargetFps::Fps30, true, Some(60.0));
        assert!((plan.interval_ms.expect("capped") - 1000.0 / 30.0).abs() < 1e-9);
        assert_eq!(plan.pacing, Pacing::Even);
        // 60 FPS on 120 Hz.
        assert_eq!(
            cap_plan(TargetFps::Fps60, true, Some(120.0)).pacing,
            Pacing::Even
        );
    }

    #[test]
    fn an_uneven_display_keeps_the_target_and_says_so() {
        // 60 FPS on 90 and 144 Hz cannot be even; the target holds.
        for hz in [90.0, 144.0, 75.0] {
            let plan = cap_plan(TargetFps::Fps60, true, Some(hz));
            assert_eq!(plan.pacing, Pacing::Uneven, "{hz}");
            assert!(plan.interval_ms.is_some());
            let note = pacing_note(TargetFps::Fps60, hz, plan.pacing).expect("a note");
            assert!(note.contains("60 FPS"), "{note}");
        }
        let plan = cap_plan(TargetFps::Fps60, true, Some(30.0));
        assert_eq!(plan.pacing, Pacing::BelowTarget);
        assert!(pacing_note(TargetFps::Fps60, 30.0, plan.pacing).is_some());
        assert_eq!(pacing_note(TargetFps::Fps60, 60.0, Pacing::Even), None);
        // Vsync off or an unknown display: the cap alone, even.
        assert_eq!(
            cap_plan(TargetFps::Fps60, false, Some(144.0)).pacing,
            Pacing::Even
        );
        assert_eq!(cap_plan(TargetFps::Fps30, true, None).pacing, Pacing::Even);
    }

    #[test]
    fn deadlines_step_by_the_interval_and_resync_when_late() {
        let (d, sleep) = next_deadline(None, 100.0, 16.0);
        assert_eq!((d, sleep), (100.0, 0.0));
        let (d, sleep) = next_deadline(Some(d), 110.0, 16.0);
        assert_eq!((d, sleep), (116.0, 6.0));
        // A long frame: no catch-up burst.
        let (d, sleep) = next_deadline(Some(d), 170.0, 16.0);
        assert_eq!((d, sleep), (170.0, 0.0));
    }
}
