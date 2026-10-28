//! Trace tests of the Auto quality controller (#1022 T5): synthetic frame
//! traces drive [`Controller`] through every rule of Spec v4 points 4 and
//! 7-9 and R1. Time comes only from the traces (`E-TEST-HERMETIC`).

use crate::quality::{Bottleneck, Control, Direction, EffectiveQuality, Limits};
use crate::quality_controller::{COOLDOWNS_S, Change, Controller, Metric, Outcome, Restriction};
use crate::quality_window::FrameSample;
use crate::settings_quality::{TargetFps, Tier};

/// The 60 FPS target interval.
const T60: f64 = 1000.0 / 60.0;

/// A frame that does `work_ms` of reliable work under a 60 FPS cap.
fn steady(work_ms: f64) -> FrameSample {
    FrameSample {
        interval_ms: work_ms.max(T60),
        work_ms: Some(work_ms),
    }
}

/// A 1600 x 900 window.
fn limits(ceiling: Tier) -> Limits {
    Limits::new(ceiling, 900)
}

/// The best quality `limits` permit.
fn top(limits: &Limits) -> EffectiveQuality {
    let mut q = EffectiveQuality::floor(limits);
    for c in Control::ALL {
        while q.can_step(c, Direction::Up, limits) {
            q.step(c, Direction::Up);
        }
    }
    q
}

/// A controller fed frame by frame, with every change and its time.
struct Sim {
    c: Controller,
    t: f64,
    log: Vec<(f64, Change)>,
}

impl Sim {
    fn new(c: Controller) -> Sim {
        Sim {
            c,
            t: 0.0,
            log: Vec::new(),
        }
    }

    /// Runs `secs` seconds of frames from `load`.
    fn run(&mut self, secs: f64, load: impl Fn(&EffectiveQuality, &Limits) -> FrameSample) {
        let end = self.t + secs;
        while self.t < end {
            let frame = load(self.c.quality(), self.c.limits());
            self.t += frame.interval_ms / 1000.0;
            if let Some(change) = self.c.observe(frame) {
                self.log.push((self.t, change));
            }
        }
    }

    /// The new steps (not rollbacks) going `dir`, with their times.
    fn steps(&self, dir: Direction) -> Vec<(f64, Control)> {
        self.log
            .iter()
            .filter(|(_, c)| !c.rollback && c.dir == dir)
            .map(|(t, c)| (*t, c.control))
            .collect()
    }
}

#[test]
fn three_overloaded_windows_before_a_reduction_and_three_seconds_between() {
    // Point 7: after the start's two-second settle, the third overloaded
    // window brings the first reduction; reductions stay three seconds
    // apart.
    let l = limits(Tier::Normal);
    let mut sim = Sim::new(Controller::new(TargetFps::Fps60, l).with_quality(top(&l)));
    sim.run(120.0, |q, l| steady(10.0 + 20.0 * q.scale(l).powi(2)));
    let downs = sim.steps(Direction::Down);
    let (first, control) = *downs.first().expect("a reduction");
    assert!((4.9..5.2).contains(&first), "{first}");
    assert_eq!(
        control,
        Control::Resolution,
        "the unknown order starts there"
    );
    for pair in downs.windows(2) {
        let [(a, _), (b, _)] = pair else { continue };
        assert!(b - a >= 3.0, "{a} then {b}");
    }
}

#[test]
fn an_isolated_spike_changes_nothing() {
    // R1: one very slow frame a window is not an overload.
    let l = limits(Tier::Normal);
    let mut sim = Sim::new(Controller::new(TargetFps::Fps60, l).with_quality(top(&l)));
    let frame_no = std::cell::Cell::new(0u32);
    sim.run(60.0, |_, _| {
        frame_no.set(frame_no.get() + 1);
        if frame_no.get().is_multiple_of(90) {
            steady(300.0)
        } else {
            steady(15.0)
        }
    });
    assert!(sim.log.is_empty(), "{:?}", sim.log);
}

#[test]
fn recovery_waits_for_ten_spare_seconds_then_climbs_to_the_ceiling() {
    // Points 4 and 7: Auto starts at the floor; each recovery step needs
    // ten spare windows in a row, in the order of point 8.
    let l = limits(Tier::Normal);
    let mut sim = Sim::new(Controller::new(TargetFps::Fps60, l));
    assert_eq!(*sim.c.quality(), EffectiveQuality::floor(&l));
    sim.run(400.0, |_, _| steady(8.0));
    let ups = sim.steps(Direction::Up);
    let (first, control) = *ups.first().expect("a recovery");
    assert!((11.9..12.3).contains(&first), "{first}");
    assert_eq!(control, Control::Resolution);
    for pair in ups.windows(2) {
        let [(a, _), (b, _)] = pair else { continue };
        assert!(b - a >= 11.9, "{a} then {b}");
    }
    let order: Vec<Control> = ups.iter().map(|(_, c)| *c).collect();
    assert_eq!(
        order,
        [
            Control::Resolution,
            Control::Resolution,
            Control::TerrainWork,
            Control::Terrain,
            Control::Markers,
            // No atmosphere step: Auto starts it at Normal (decision A).
            Control::Shadows,
            Control::Bloom,
        ]
    );
    assert_eq!(*sim.c.quality(), top(&l));
    assert_eq!(sim.c.restriction(), Restriction::None);
}

#[test]
fn a_useful_reduction_is_kept_and_a_useless_one_rolled_back() {
    // R1: resolution cuts the work, so its trials are kept (on the work
    // metric); the loading work changes nothing, so its trial rolls back
    // and that step cools down.
    let l = limits(Tier::Normal);
    let mut sim = Sim::new(Controller::new(TargetFps::Fps60, l).with_quality(top(&l)));
    sim.run(30.0, |q, l| steady(15.0 + 20.0 * q.scale(l).powi(2)));
    let first: Vec<Change> = sim.log.iter().map(|(_, c)| *c).take(4).collect();
    let down = |control| Change {
        control,
        dir: Direction::Down,
        rollback: false,
    };
    assert_eq!(
        first,
        [
            down(Control::Resolution),
            down(Control::Resolution),
            down(Control::TerrainWork),
            Change {
                control: Control::TerrainWork,
                dir: Direction::Up,
                rollback: true
            },
        ]
    );
    assert_eq!(sim.c.quality().res_steps, l.res_steps_max());
    let last = sim.c.last_trial().expect("a trial");
    assert_eq!(last.outcome, Outcome::RolledBack);
    assert_eq!(last.metric, Some(Metric::Work));
    assert!(sim.c.cooldown_s(Control::TerrainWork, Direction::Down) > 50.0);
}

#[test]
fn a_failed_upgrade_cools_down_60_then_120_then_240_seconds() {
    // R1: only resolution can climb (Low ceiling, memory pressure); native
    // overloads, so every try at it rolls back, each further apart.
    let l = Limits {
        memory_pressure: true,
        ..limits(Tier::Low)
    };
    let mut sim = Sim::new(Controller::new(TargetFps::Fps60, l));
    sim.run(1200.0, |q, _| {
        steady(if q.res_steps == 0 { 25.0 } else { 8.0 })
    });
    let rollbacks: Vec<f64> = sim
        .log
        .iter()
        .filter(|(_, c)| c.rollback)
        .map(|(t, _)| *t)
        .collect();
    let tries: Vec<f64> = sim
        .steps(Direction::Up)
        .into_iter()
        .skip(1)
        .map(|(t, _)| t)
        .collect();
    assert!(rollbacks.len() >= 4, "{rollbacks:?}");
    for (k, (rollback, next)) in rollbacks.iter().zip(tries.iter().skip(1)).enumerate() {
        let wait = COOLDOWNS_S.get(k).copied().unwrap_or(240.0);
        assert!(next - rollback >= wait, "try {k}: {rollback} then {next}");
        assert!(
            next - rollback < wait + 15.0,
            "try {k}: {rollback} then {next}"
        );
    }
}

#[test]
fn a_fixed_workload_settles_without_alternating() {
    // Under a fixed synthetic workload (native resolution falls off a
    // cliff) the controller climbs, fails native once, then holds still:
    // no quality changes for the last twenty-five minutes.
    let l = limits(Tier::Normal);
    let mut sim = Sim::new(Controller::new(TargetFps::Fps60, l));
    let cost = |q: &EffectiveQuality, l: &Limits| {
        let tier = |t: Tier| f64::from(t as u8);
        3.0 + 12.0 * q.scale(l).powi(2)
            + if q.res_steps == 0 { 6.0 } else { 0.0 }
            + 2.0 * tier(q.terrain)
            + tier(q.markers)
            + tier(q.atmosphere)
            + if q.shadows { 2.0 } else { 0.0 }
            + if q.bloom { 1.0 } else { 0.0 }
    };
    sim.run(1800.0, |q, l| steady(cost(q, l)));
    assert!(sim.c.rollbacks() >= 1, "native was tried and failed");
    let late: Vec<_> = sim.log.iter().filter(|(t, _)| *t > 300.0).collect();
    assert!(late.is_empty(), "{late:?}");
    let q = sim.c.quality();
    assert!(q.res_steps > 0 && q.terrain > Tier::Low, "{q:?}");
}

#[test]
fn a_capped_game_without_work_timing_probes_one_step_at_a_time() {
    // R1: with no reliable work timing, ten seconds meeting the target
    // allow a probe, judged like any upgrade.
    let l = limits(Tier::Normal);
    let mut sim = Sim::new(Controller::new(TargetFps::Fps60, l));
    sim.run(60.0, |q, _| FrameSample {
        interval_ms: if q.res_steps == 0 { 25.0 } else { T60 },
        work_ms: None,
    });
    let ups = sim.steps(Direction::Up);
    assert_eq!(ups.first().map(|(_, c)| *c), Some(Control::Resolution));
    assert!(sim.c.rollbacks() >= 1, "native overloads and rolls back");
    assert!(sim.c.cooldown_s(Control::Resolution, Direction::Up) > 0.0);
    assert_eq!(sim.c.quality().res_steps, 1);
    // With reliable timing, delivery alone is not spare capacity.
    let mut busy = Sim::new(Controller::new(TargetFps::Fps60, l));
    busy.run(60.0, |_, _| steady(15.0));
    assert!(busy.log.is_empty(), "{:?}", busy.log);
}

#[test]
fn a_trial_without_its_windows_times_out_and_rolls_back() {
    let l = limits(Tier::Normal);
    let mut sim = Sim::new(Controller::new(TargetFps::Fps60, l).with_quality(top(&l)));
    sim.run(5.5, |_, _| steady(30.0));
    assert_eq!(sim.log.len(), 1, "the reduction");
    // The work timing then fails: the trial cannot be judged.
    sim.run(12.0, |_, _| FrameSample {
        interval_ms: 30.0,
        work_ms: None,
    });
    let (t, change) = *sim.log.get(1).expect("the rollback");
    assert!(change.rollback);
    assert!((14.9..15.3).contains(&t), "{t}");
    assert_eq!(
        sim.c.last_trial().map(|r| r.outcome),
        Some(Outcome::Insufficient)
    );
    assert_eq!(*sim.c.quality(), top(&l));
}

#[test]
fn the_floor_holds_and_the_limit_shows_quietly() {
    // R1: an overload nothing can help ends in the quiet performance-limit
    // status at the floor, never below it, and useless steps are undone.
    let l = limits(Tier::Normal);
    let floor = EffectiveQuality {
        terrain_work: Tier::Low,
        ..EffectiveQuality::floor(&l)
    };
    let mut sim = Sim::new(Controller::new(TargetFps::Fps60, l).with_quality(floor));
    sim.run(30.0, |_, _| steady(40.0));
    assert!(sim.log.is_empty());
    assert!(sim.c.performance_limited());
    assert_eq!(sim.c.restriction(), Restriction::Performance);
    assert!(sim.c.floor_s() > 29.0);
    // From the start quality, the one possible reduction is useless.
    let mut sim = Sim::new(Controller::new(TargetFps::Fps60, l));
    sim.run(30.0, |_, _| steady(40.0));
    assert_eq!(*sim.c.quality(), EffectiveQuality::floor(&l));
    assert!(sim.c.performance_limited());
}

#[test]
fn ceilings_apply_at_once_down_and_gradually_up() {
    // Point 9.
    let high = limits(Tier::High);
    let mut c = Controller::new(TargetFps::Fps60, high).with_quality(top(&high));
    c.set_ceiling(Tier::Low);
    let q = *c.quality();
    // The atmosphere stays at Auto's floor, Normal (owner decision A).
    assert_eq!(
        (q.terrain, q.markers, q.atmosphere),
        (Tier::Low, Tier::Low, Tier::Normal)
    );
    assert!(!q.shadows && !q.bloom);
    c.set_ceiling(Tier::High);
    assert_eq!(*c.quality(), q, "a raised ceiling only permits recovery");
    // A target change clears the trial history.
    let l = limits(Tier::Normal);
    let mut sim = Sim::new(Controller::new(TargetFps::Fps60, l).with_quality(top(&l)));
    sim.run(30.0, |_, _| steady(25.0));
    assert!(sim.c.cooldown_s(Control::Resolution, Direction::Down) > 0.0);
    sim.c.set_target(TargetFps::Fps30);
    assert_eq!(sim.c.cooldown_s(Control::Resolution, Direction::Down), 0.0);
    // At 30 FPS the same work is no overload.
    let before = sim.log.len();
    sim.run(30.0, |_, _| FrameSample {
        interval_ms: 1000.0 / 30.0,
        work_ms: Some(25.0),
    });
    assert!(
        sim.log
            .iter()
            .skip(before)
            .all(|(_, c)| c.dir == Direction::Up),
        "{:?}",
        sim.log
    );
}

#[test]
fn pauses_drop_their_frames_and_settle_after() {
    // Point 7: paused frames and partial windows do not count; two seconds
    // settle after the resume.
    let l = limits(Tier::Normal);
    let mut sim = Sim::new(Controller::new(TargetFps::Fps60, l));
    sim.run(11.5, |_, _| steady(8.0));
    assert!(sim.log.is_empty());
    sim.c.set_paused(true);
    sim.run(30.0, |_, _| steady(100.0));
    assert!(sim.log.is_empty(), "paused");
    sim.c.set_paused(false);
    let resumed = sim.t;
    sim.run(20.0, |_, _| steady(8.0));
    let (first, _) = *sim.log.first().expect("a recovery");
    assert!(first - resumed >= 11.9, "{resumed} then {first}");
}

#[test]
fn memory_pressure_holds_terrain_and_says_so() {
    // R2 and S9: no terrain recovery and no more loading work under
    // memory pressure; the rest still recovers.
    let l = Limits {
        memory_pressure: true,
        ..limits(Tier::Normal)
    };
    let mut sim = Sim::new(Controller::new(TargetFps::Fps60, l));
    sim.run(300.0, |_, _| steady(8.0));
    let q = *sim.c.quality();
    assert_eq!((q.terrain, q.terrain_work), (Tier::Low, Tier::Normal));
    assert!(q.shadows && q.bloom && q.res_steps == 0);
    assert_eq!(sim.c.restriction(), Restriction::Memory);
    sim.c.set_memory_pressure(false);
    sim.run(60.0, |_, _| steady(8.0));
    assert!(sim.c.quality().terrain_work > Tier::Normal);
}

#[test]
fn auto_never_lowers_the_atmosphere_below_normal() {
    // Owner decision A (#1022, after the T8 picture check).
    let l = limits(Tier::High);
    let mut sim = Sim::new(Controller::new(TargetFps::Fps60, l).with_quality(top(&l)));
    sim.run(600.0, |_, _| steady(40.0));
    assert!(sim.c.quality().atmosphere >= Tier::Normal);
    assert_eq!(EffectiveQuality::floor(&l).atmosphere, Tier::Normal);
    assert!(!EffectiveQuality::floor(&l).can_step(Control::Atmosphere, Direction::Down, &l));
}

#[test]
fn a_scene_change_voids_the_comparison() {
    // R1: a level change mid-trial keeps the step without a verdict.
    let l = limits(Tier::Normal);
    let mut sim = Sim::new(Controller::new(TargetFps::Fps60, l).with_quality(top(&l)));
    sim.run(5.5, |_, _| steady(30.0));
    let stepped = *sim.c.quality();
    sim.c.discard();
    assert_eq!(
        sim.c.last_trial().map(|r| r.outcome),
        Some(Outcome::Discarded)
    );
    assert_eq!(*sim.c.quality(), stepped);
    assert_ne!(stepped, top(&l));
}

#[test]
fn a_bottleneck_hint_picks_the_first_reduction() {
    let l = limits(Tier::Normal);
    for (hint, want) in [
        (Bottleneck::Markers, Control::Markers),
        (Bottleneck::TerrainProcessing, Control::TerrainWork),
        (Bottleneck::Pixels, Control::Resolution),
    ] {
        let mut c = Controller::new(TargetFps::Fps60, l).with_quality(top(&l));
        c.set_bottleneck(hint);
        let mut sim = Sim::new(c);
        sim.run(5.5, |_, _| steady(30.0));
        assert_eq!(
            sim.steps(Direction::Down).first().map(|(_, c)| *c),
            Some(want)
        );
    }
}
