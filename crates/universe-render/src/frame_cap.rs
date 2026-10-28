//! The frame cap (#1022 T7, Spec v4 point 6).
//!
//! At the end of the main world's frame, after its work is measured
//! ([`FrameWorkSet::MainEnds`]), the cap sleeps until the next deadline of
//! the chosen target ([`universe_core::pacing`]). The sleep is the
//! standard one, which on Windows uses a high-resolution waitable timer:
//! the CPU idles, it never spins. The sleep is recorded as
//! [`crate::WorkSample::cap_ms`], apart from the work, so the quality
//! controller never counts it. [`PacingStatus`] holds how evenly the
//! display shows the target and the plain-words note when it cannot.
//!
//! Added by the window runs only: headless tests and `--verify` never
//! sleep.

use crate::SettingsStore;
use crate::frame_work::{FrameWork, FrameWorkSet};
use bevy::prelude::*;
use bevy::window::{Monitor, PrimaryMonitor};
use std::time::{Duration, Instant};
use universe_core::pacing::{Pacing, cap_plan, next_deadline, pacing_note};
use universe_core::settings::TargetFps;

/// How the display shows the target, for Settings (S9) and logs.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct PacingStatus {
    /// The target in force.
    pub target: TargetFps,
    /// The primary display's refresh, when known.
    pub refresh_hz: Option<f64>,
    /// How evenly the display shows the target.
    pub pacing: Pacing,
    /// The cap's interval; `None` when vsync paces alone.
    pub interval_ms: Option<f64>,
    /// The plain-words note when the pacing is not even.
    pub note: Option<String>,
}

impl Default for PacingStatus {
    fn default() -> Self {
        PacingStatus {
            target: TargetFps::default(),
            refresh_hz: None,
            pacing: Pacing::Even,
            interval_ms: None,
            note: None,
        }
    }
}

/// The cap's clock: milliseconds since `origin`, and the last deadline.
#[derive(Resource, Debug)]
struct CapClock {
    origin: Instant,
    deadline: Option<f64>,
}

impl Default for CapClock {
    fn default() -> Self {
        CapClock {
            origin: Instant::now(),
            deadline: None,
        }
    }
}

/// Milliseconds since `origin`.
fn ms_since(origin: Instant) -> f64 {
    origin.elapsed().as_secs_f64() * 1000.0
}

/// Sleeps to the target's next deadline (see the module notes).
fn cap_frame(
    store: Option<Res<SettingsStore>>,
    monitors: Query<&Monitor, With<PrimaryMonitor>>,
    mut status: ResMut<PacingStatus>,
    mut clock: ResMut<CapClock>,
    work: Option<ResMut<FrameWork>>,
) {
    let (target, vsync) = store.map_or((TargetFps::default(), true), |s| {
        (s.0.target_fps, s.0.vsync)
    });
    let refresh = monitors
        .iter()
        .next()
        .and_then(|m| m.refresh_rate_millihertz)
        .map(|mhz| f64::from(mhz) / 1000.0);
    let plan = cap_plan(target, vsync, refresh);
    if status.target != target
        || status.refresh_hz != refresh
        || status.pacing != plan.pacing
        || status.interval_ms != plan.interval_ms
    {
        let note = refresh.and_then(|hz| pacing_note(target, hz, plan.pacing));
        if let Some(note) = &note {
            info!("frame pacing: {note}");
        }
        *status = PacingStatus {
            target,
            refresh_hz: refresh,
            pacing: plan.pacing,
            interval_ms: plan.interval_ms,
            note,
        };
    }
    let slept = match plan.interval_ms {
        None => {
            clock.deadline = None;
            0.0
        }
        Some(interval) => {
            let now = ms_since(clock.origin);
            let (deadline, sleep_ms) = next_deadline(clock.deadline, now, interval);
            clock.deadline = Some(deadline);
            if sleep_ms > 0.0 {
                std::thread::sleep(Duration::from_secs_f64(sleep_ms / 1000.0));
            }
            ms_since(clock.origin) - now
        }
    };
    if let Some(mut work) = work {
        work.set_cap_ms(slept);
    }
}

/// The frame cap for the window runs.
pub struct FrameCapPlugin;

impl Plugin for FrameCapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PacingStatus>()
            .init_resource::<CapClock>()
            .add_systems(Last, cap_frame.after(FrameWorkSet::MainEnds));
    }
}
