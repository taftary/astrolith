//! The Auto quality controller (#1022 T5, Spec v4 points 4 and 7-9, R1).
//!
//! A pure, deterministic state machine fed one [`crate::quality_window::FrameSample`] per frame of
//! active play. It closes one-second windows ([`crate::quality_window`]);
//! after three overloaded windows, at least three seconds after the last
//! reduction, it steps one control down; after ten windows of spare
//! capacity it steps one up. Every step is a trial: after a two-second
//! settle it watches three windows, keeps a useful reduction and rolls a
//! useless one back, and rolls an upgrade back under sustained overload.
//! A failed control and direction cools down for 60, 120, then 240 seconds
//! of active play. No allocation per frame (`E-HOT-NOALLOC`); time comes
//! only from the samples (`E-TEST-HERMETIC`).

use crate::quality::{Bottleneck, Control, Direction, EffectiveQuality, Limits};
use crate::quality_window::{FrameSample, Window, WindowStats};
use crate::settings_quality::{TargetFps, Tier};

/// Overloaded windows in a row before a reduction (point 7).
pub const OVERLOAD_WINDOWS: u32 = 3;
/// Seconds at least between two reductions (point 7).
pub const REDUCE_GAP_S: f64 = 3.0;
/// Spare windows in a row before a recovery step (point 7).
pub const SPARE_WINDOWS: u32 = 10;
/// Seconds after a change, resume or resize before windows count.
pub const SETTLE_S: f64 = 2.0;
/// Windows a trial watches after its settle (R1).
pub const TRIAL_WINDOWS: usize = 3;
/// Seconds after which a trial without its windows rolls back (R1).
pub const TRIAL_TIMEOUT_S: f64 = 10.0;
/// Cooldowns after one, two, and three or more failures in a row (R1).
pub const COOLDOWNS_S: [f64; 3] = [60.0, 120.0, 240.0];

/// What a reduction trial compared (R1: record which metric was used).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Metric {
    /// p95 active work (reliable timing).
    Work,
    /// p95 frame interval (frame delivery).
    Delivery,
}

/// How a trial ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// The step helped, or an upgrade held: kept.
    Kept,
    /// The step did not help, or the upgrade overloaded: rolled back.
    RolledBack,
    /// No verdict within the time-out: rolled back.
    Insufficient,
    /// The comparison was voided (scene change, pause, resize); the step
    /// stays.
    Discarded,
}

/// A change the controller made to the effective quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Change {
    /// The control stepped.
    pub control: Control,
    /// The way it went.
    pub dir: Direction,
    /// A corrective rollback of a failed trial, not a new step.
    pub rollback: bool,
}

/// The last finished trial, for logs and evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TrialRecord {
    /// The control tried.
    pub control: Control,
    /// The way it went.
    pub dir: Direction,
    /// How it ended.
    pub outcome: Outcome,
    /// The metric a reduction compared (`None` for an upgrade).
    pub metric: Option<Metric>,
}

/// What holds the quality below its ceiling (S9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Restriction {
    /// Nothing: Auto is at its ceiling.
    None,
    /// The ground's memory protection.
    Memory,
    /// Performance.
    Performance,
    /// Both.
    Both,
}

/// One control and direction's failures in a row and cooldown left.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Cooldown {
    failures: u8,
    left_s: f64,
}

/// The trial in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Trial {
    control: Control,
    dir: Direction,
    metric: Metric,
    baseline: [f64; TRIAL_WINDOWS],
    baseline_delivered: bool,
    results: [f64; TRIAL_WINDOWS],
    delivered: u32,
    overloaded: u32,
    seen: usize,
    age_s: f64,
}

/// The Auto controller.
#[derive(Debug, Clone)]
pub struct Controller {
    quality: EffectiveQuality,
    limits: Limits,
    target: TargetFps,
    bottleneck: Bottleneck,
    window: Window,
    recent: [Option<WindowStats>; TRIAL_WINDOWS],
    overload_streak: u32,
    spare_streak: u32,
    delivered_streak: u32,
    since_reduce_s: f64,
    settle_s: f64,
    paused: bool,
    trial: Option<Trial>,
    cooldowns: [Cooldown; 14],
    limited: bool,
    changes: u32,
    rollbacks: u32,
    floor_s: f64,
    last: Option<TrialRecord>,
}

/// The median of three values.
fn median(v: [f64; 3]) -> f64 {
    let [a, b, c] = v;
    a.max(b).min(a.min(b).max(c))
}

/// Twice the median absolute deviation of three values (R1).
fn twice_mad(v: [f64; 3]) -> f64 {
    let m = median(v);
    2.0 * median(v.map(|x| (x - m).abs()))
}

impl Controller {
    /// A controller starting at the floor (point 4: each launch in Auto).
    #[must_use]
    pub fn new(target: TargetFps, limits: Limits) -> Controller {
        Controller {
            quality: EffectiveQuality::floor(&limits),
            limits,
            target,
            bottleneck: Bottleneck::Unknown,
            window: Window::default(),
            recent: [None; TRIAL_WINDOWS],
            overload_streak: 0,
            spare_streak: 0,
            delivered_streak: 0,
            since_reduce_s: REDUCE_GAP_S,
            settle_s: SETTLE_S,
            paused: false,
            trial: None,
            cooldowns: [Cooldown::default(); 14],
            limited: false,
            changes: 0,
            rollbacks: 0,
            floor_s: 0.0,
            last: None,
        }
    }

    /// The same controller from `quality` (tests).
    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_quality(mut self, quality: EffectiveQuality) -> Controller {
        self.quality = quality.clamped(&self.limits);
        self
    }

    /// What is drawn now.
    #[must_use]
    pub const fn quality(&self) -> &EffectiveQuality {
        &self.quality
    }

    /// The limits in force.
    #[must_use]
    pub const fn limits(&self) -> &Limits {
        &self.limits
    }

    /// Steps taken (rollbacks not counted).
    #[must_use]
    pub const fn changes(&self) -> u32 {
        self.changes
    }

    /// Rollbacks made.
    #[must_use]
    pub const fn rollbacks(&self) -> u32 {
        self.rollbacks
    }

    /// Seconds of active play spent at the floor.
    #[must_use]
    pub const fn floor_s(&self) -> f64 {
        self.floor_s
    }

    /// The last finished trial.
    #[must_use]
    pub const fn last_trial(&self) -> Option<TrialRecord> {
        self.last
    }

    /// Every useful reduction is used up or cooling down while overloaded:
    /// the quiet performance-limit status (R1).
    #[must_use]
    pub const fn performance_limited(&self) -> bool {
        self.limited
    }

    /// What holds the quality below its ceiling (S9).
    #[must_use]
    pub fn restriction(&self) -> Restriction {
        let memory = self.limits.memory_pressure
            && (self.quality.terrain < self.limits.ceiling
                || self.quality.terrain_work < Tier::High);
        let performance = self.limited || self.quality.below_ceiling(&self.limits);
        match (memory, performance) {
            (false, false) => Restriction::None,
            (true, false) => Restriction::Memory,
            (false, true) => Restriction::Performance,
            (true, true) => Restriction::Both,
        }
    }

    /// Seconds of cooldown left for `control` going `dir`.
    #[must_use]
    pub fn cooldown_s(&self, control: Control, dir: Direction) -> f64 {
        self.cooldown(control, dir).left_s
    }

    fn slot(control: Control, dir: Direction) -> usize {
        control.index() * 2 + usize::from(dir == Direction::Up)
    }

    fn cooldown(&self, control: Control, dir: Direction) -> Cooldown {
        self.cooldowns
            .get(Self::slot(control, dir))
            .copied()
            .unwrap_or_default()
    }

    fn cooldown_mut(&mut self, control: Control, dir: Direction) -> Option<&mut Cooldown> {
        self.cooldowns.get_mut(Self::slot(control, dir))
    }

    fn target_ms(&self) -> f64 {
        self.target.interval_ms()
    }

    /// Forgets observations, the trial and the streaks, and waits
    /// `settle_s` before counting again.
    fn forget(&mut self, settle_s: f64) {
        self.window.clear();
        self.recent = [None; TRIAL_WINDOWS];
        self.overload_streak = 0;
        self.spare_streak = 0;
        self.delivered_streak = 0;
        self.settle_s = self.settle_s.max(settle_s);
        if let Some(t) = self.trial.take() {
            self.record(t.control, t.dir, Outcome::Discarded, t);
        }
    }

    /// Also clears the trial history: cooldowns and failure counts (point
    /// 9, R1: a target, mode or ceiling change resets it).
    fn reset_history(&mut self) {
        self.trial = None;
        self.forget(SETTLE_S);
        self.cooldowns = [Cooldown::default(); 14];
        self.limited = false;
        self.since_reduce_s = REDUCE_GAP_S;
    }

    /// Starts again at the floor: a launch in Auto, a switch from Manual,
    /// or Display Reset (points 4 and 10).
    pub fn restart(&mut self) {
        self.quality = EffectiveQuality::floor(&self.limits);
        self.reset_history();
    }

    /// A new FPS target (point 9).
    pub fn set_target(&mut self, target: TargetFps) {
        if target != self.target {
            self.target = target;
            self.reset_history();
        }
    }

    /// A new Auto ceiling: a lower one applies at once, a higher one only
    /// permits gradual recovery (point 9).
    pub fn set_ceiling(&mut self, ceiling: Tier) {
        if ceiling != self.limits.ceiling {
            self.limits.ceiling = ceiling;
            self.quality = self.quality.clamped(&self.limits);
            self.reset_history();
        }
    }

    /// The viewport's shorter side is now `native_short` pixels (point 7:
    /// settle after a resize; the comparison is void).
    pub fn resize(&mut self, native_short: u32) {
        self.limits.floor_scale = crate::quality::floor_scale(native_short);
        self.quality = self.quality.clamped(&self.limits);
        self.forget(SETTLE_S);
    }

    /// The ground's memory pressure began or ended (R2).
    pub const fn set_memory_pressure(&mut self, on: bool) {
        self.limits.memory_pressure = on;
    }

    /// The diagnosed bottleneck (R1).
    pub const fn set_bottleneck(&mut self, bottleneck: Bottleneck) {
        self.bottleneck = bottleneck;
    }

    /// Adaptation pauses in menus and while minimised or in the background;
    /// it settles for two seconds after (point 7).
    pub fn set_paused(&mut self, paused: bool) {
        if paused == self.paused {
            return;
        }
        self.paused = paused;
        self.forget(if paused { 0.0 } else { SETTLE_S });
    }

    /// A level or planet change, a settings change or a changed loading
    /// state: the observations and the comparison are void (R1).
    pub fn discard(&mut self) {
        self.forget(0.0);
    }

    fn record(&mut self, control: Control, dir: Direction, outcome: Outcome, t: Trial) {
        self.last = Some(TrialRecord {
            control,
            dir,
            outcome,
            metric: (dir == Direction::Down).then_some(t.metric),
        });
    }

    /// One frame of active play; returns the change made, if any.
    pub fn observe(&mut self, frame: FrameSample) -> Option<Change> {
        if self.paused {
            return None;
        }
        let dt = frame.interval_ms.max(0.0) / 1000.0;
        for c in &mut self.cooldowns {
            c.left_s = (c.left_s - dt).max(0.0);
        }
        self.since_reduce_s += dt;
        if self.quality.at_floor(&self.limits) {
            self.floor_s += dt;
        }
        if let Some(t) = self.trial.as_mut() {
            t.age_s += dt;
            if t.age_s > TRIAL_TIMEOUT_S {
                return self.finish(Outcome::Insufficient);
            }
        }
        if self.settle_s > 0.0 {
            self.settle_s -= dt;
            return None;
        }
        let stats = self.window.push(frame, self.target_ms())?;
        self.on_window(stats)
    }

    fn on_window(&mut self, stats: WindowStats) -> Option<Change> {
        let target_ms = self.target_ms();
        self.recent.rotate_left(1);
        if let Some(last) = self.recent.last_mut() {
            *last = Some(stats);
        }
        let overloaded = stats.overloaded();
        self.overload_streak = if overloaded {
            self.overload_streak + 1
        } else {
            0
        };
        self.spare_streak = if stats.spare(target_ms) {
            self.spare_streak + 1
        } else {
            0
        };
        self.delivered_streak = if stats.delivered() && !overloaded {
            self.delivered_streak + 1
        } else {
            0
        };
        if !overloaded {
            self.limited = false;
        }
        if self.trial.is_some() {
            return self.watch(stats);
        }
        if self.overload_streak >= OVERLOAD_WINDOWS && self.since_reduce_s >= REDUCE_GAP_S {
            return self.reduce();
        }
        if self.spare_streak >= SPARE_WINDOWS {
            return self.recover();
        }
        // Without reliable work timing, steady delivery allows a bounded
        // recovery probe, judged as any upgrade (R1).
        if stats.p95_work_ms.is_none() && self.delivered_streak >= SPARE_WINDOWS {
            return self.recover();
        }
        None
    }

    fn cooled(&self, control: Control, dir: Direction) -> bool {
        self.cooldown(control, dir).left_s > 0.0
    }

    fn start(&mut self, control: Control, dir: Direction, metric: Metric) -> Change {
        let baseline_of = |s: Option<WindowStats>| {
            s.map_or(0.0, |s| match metric {
                Metric::Work => s.p95_work_ms.unwrap_or(s.p95_interval_ms),
                Metric::Delivery => s.p95_interval_ms,
            })
        };
        let trial = Trial {
            control,
            dir,
            metric,
            baseline: self.recent.map(baseline_of),
            baseline_delivered: self.recent.iter().all(|s| s.is_some_and(|s| s.delivered())),
            results: [0.0; TRIAL_WINDOWS],
            delivered: 0,
            overloaded: 0,
            seen: 0,
            age_s: 0.0,
        };
        self.quality.step(control, dir);
        self.changes += 1;
        self.forget(SETTLE_S);
        self.trial = Some(trial);
        Change {
            control,
            dir,
            rollback: false,
        }
    }

    fn reduce(&mut self) -> Option<Change> {
        let metric = if self
            .recent
            .iter()
            .all(|s| s.is_some_and(|s| s.p95_work_ms.is_some()))
        {
            Metric::Work
        } else {
            Metric::Delivery
        };
        let pick = self.bottleneck.reduction_order().into_iter().find(|c| {
            self.quality.can_step(*c, Direction::Down, &self.limits)
                && !self.cooled(*c, Direction::Down)
        });
        let Some(control) = pick else {
            self.limited = true;
            return None;
        };
        self.since_reduce_s = 0.0;
        Some(self.start(control, Direction::Down, metric))
    }

    fn recover(&mut self) -> Option<Change> {
        let pick = Control::RECOVERY.into_iter().find(|c| {
            self.quality.can_step(*c, Direction::Up, &self.limits)
                && !self.cooled(*c, Direction::Up)
        });
        // At the ceiling, or every step cooling down: wait again.
        self.spare_streak = 0;
        self.delivered_streak = 0;
        pick.map(|control| self.start(control, Direction::Up, Metric::Delivery))
    }

    fn watch(&mut self, stats: WindowStats) -> Option<Change> {
        let t = self.trial.as_mut()?;
        // A window without the trial's metric does not count; the
        // time-out ends a trial that cannot finish.
        let value = match (t.dir, t.metric) {
            (Direction::Down, Metric::Work) => stats.p95_work_ms?,
            _ => stats.p95_interval_ms,
        };
        if let Some(slot) = t.results.get_mut(t.seen) {
            *slot = value;
        }
        t.seen += 1;
        t.overloaded += u32::from(stats.overloaded());
        t.delivered += u32::from(stats.delivered());
        if t.seen < TRIAL_WINDOWS {
            return None;
        }
        let t = *t;
        let kept = match t.dir {
            Direction::Down => {
                let base = median(t.baseline);
                let gain = base - median(t.results);
                let needed = (0.05 * base).max(0.5).max(twice_mad(t.baseline));
                gain >= needed || (!t.baseline_delivered && t.delivered == 3)
            }
            Direction::Up => t.overloaded < 3,
        };
        if kept {
            if let Some(c) = self.cooldown_mut(t.control, t.dir) {
                *c = Cooldown::default();
            }
            self.trial = None;
            self.record(t.control, t.dir, Outcome::Kept, t);
            None
        } else {
            self.finish(Outcome::RolledBack)
        }
    }

    /// Rolls the trial's step back and cools that control and direction
    /// down.
    fn finish(&mut self, outcome: Outcome) -> Option<Change> {
        let t = self.trial.take()?;
        self.quality.step(t.control, t.dir.reverse());
        self.rollbacks += 1;
        if let Some(c) = self.cooldown_mut(t.control, t.dir) {
            c.failures = c.failures.saturating_add(1);
            let i = usize::from(c.failures.min(3)) - 1;
            c.left_s = COOLDOWNS_S.get(i).copied().unwrap_or(240.0);
        }
        self.record(t.control, t.dir, outcome, t);
        self.forget(SETTLE_S);
        Some(Change {
            control: t.control,
            dir: t.dir.reverse(),
            rollback: true,
        })
    }
}
