//! The Auto quality controller in the headless check (#1022 T10).
//!
//! Three `QUALITY` lines for `--verify`: a heavy fixed workload from the
//! best quality, a calm one from the start (the floor), and the heavy run
//! repeated byte for byte. The workload is synthetic: work grows with the
//! picture's pixels and each detail tier. Pure (`E-CORE-NO-BEVY`,
//! `E-DET-TIERS`).

use crate::quality::{Control, Direction, EffectiveQuality, Limits};
use crate::quality_controller::{Controller, Restriction};
use crate::quality_window::FrameSample;
use crate::settings_quality::{TargetFps, Tier};
use std::fmt::Write as _;

/// Seconds each run lasts.
const RUN_S: f64 = 600.0;

/// The last stretch of a run, where a settled controller only retries a
/// failed step once its cooldown (at most 240 s) has passed.
const LATE_S: f64 = 300.0;

/// Changes allowed in the last stretch: two retries, each a step and its
/// rollback.
const LATE_CHANGES_MAX: u32 = 4;

/// A 1600 x 900 window under a Normal ceiling.
fn limits() -> Limits {
    Limits::new(Tier::Normal, 900)
}

/// The best quality `limits` permit.
fn best(limits: &Limits) -> EffectiveQuality {
    let mut q = EffectiveQuality::floor(limits);
    for c in Control::ALL {
        while q.can_step(c, Direction::Up, limits) {
            q.step(c, Direction::Up);
        }
    }
    q
}

/// Milliseconds of work for `q` at `load` (pure).
fn work_ms(q: &EffectiveQuality, l: &Limits, load: f64) -> f64 {
    let tier = |t: Tier| f64::from(t as u8);
    let effects = if q.shadows { 2.0 } else { 0.0 } + if q.bloom { 1.0 } else { 0.0 };
    load * (4.0
        + 14.0 * q.scale(l).powi(2)
        + 2.0 * tier(q.terrain)
        + tier(q.markers)
        + tier(q.atmosphere)
        + effects)
}

/// What one run did.
#[derive(Debug, Clone, PartialEq)]
struct Run {
    steps: u32,
    rollbacks: u32,
    late_changes: u32,
    quality: EffectiveQuality,
    restriction: Restriction,
}

/// Runs `RUN_S` seconds at `load` from `start` (60 FPS, vsync paced).
fn run(start: Option<EffectiveQuality>, load: f64) -> Run {
    let l = limits();
    let mut c = Controller::new(TargetFps::Fps60, l);
    if let Some(q) = start {
        c.set_quality(q);
    }
    let target = TargetFps::Fps60.interval_ms();
    let (mut t, mut late) = (0.0, 0_u32);
    while t < RUN_S {
        let work = work_ms(c.quality(), c.limits(), load);
        let frame = FrameSample {
            interval_ms: work.max(target),
            work_ms: Some(work),
        };
        t += frame.interval_ms / 1000.0;
        if c.observe(frame).is_some() && t > RUN_S - LATE_S {
            late += 1;
        }
    }
    Run {
        steps: c.changes(),
        rollbacks: c.rollbacks(),
        late_changes: late,
        quality: *c.quality(),
        restriction: c.restriction(),
    }
}

/// The quality in one word per control.
fn describe(q: &EffectiveQuality) -> String {
    let on = |b: bool| if b { "on" } else { "off" };
    format!(
        "res_steps={} terrain={} markers={} atmosphere={} shadows={} bloom={} loading={}",
        q.res_steps,
        q.terrain.id(),
        q.markers.id(),
        q.atmosphere.id(),
        on(q.shadows),
        on(q.bloom),
        q.terrain_work.id()
    )
}

/// Appends the three `QUALITY` lines; `true` when all pass.
pub fn verify_quality(out: &mut String) -> bool {
    let l = limits();
    let heavy = run(Some(best(&l)), 1.6);
    // Heavy: quality comes down, then only cooldown retries.
    let heavy_ok = heavy.quality != best(&l) && heavy.late_changes <= LATE_CHANGES_MAX;
    let calm = run(None, 0.5);
    // Calm: from the floor all the way to the ceiling.
    let calm_ok = calm.quality == best(&l) && calm.restriction == Restriction::None;
    let again = run(Some(best(&l)), 1.6);
    let repeat_ok = again == heavy;
    let pass = |ok: bool| if ok { "PASS" } else { "FAIL" };
    for (name, r, ok) in [("heavy", &heavy, heavy_ok), ("calm", &calm, calm_ok)] {
        let _ = writeln!(
            out,
            "QUALITY load={name} steps={} rollbacks={} late_changes={} {} {}",
            r.steps,
            r.rollbacks,
            r.late_changes,
            describe(&r.quality),
            pass(ok)
        );
    }
    let _ = writeln!(
        out,
        "QUALITY repeat={} {}",
        if repeat_ok { "identical" } else { "different" },
        pass(repeat_ok)
    );
    heavy_ok && calm_ok && repeat_ok
}
