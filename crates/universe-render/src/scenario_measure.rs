//! Bounded, explicitly installed real-input timing journal (#1238 T6).
//! Work keeps its actual main/extracted-render frame; latest unrelated samples
//! are never summed or called the same frame. This collector makes no decisions.

use crate::observation_clock::{Moment, ObservationClock};
use bevy::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum undrained measurement records. Full files are bounded by the app.
pub const MEASURE_EVENT_LIMIT: usize = 8_192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkStage {
    Main,
    Render,
    Cap,
}

#[derive(Debug)]
enum Event {
    Begin {
        begin_ns: u64,
        interval_ns: Option<u64>,
    },
    ClockSync {
        before_ns: u64,
        unix_ns: u128,
        after_ns: u64,
    },
    Work {
        frame: u64,
        stage: WorkStage,
        work_ns: u64,
        wait_ns: u64,
    },
    State(String),
}

#[derive(Debug)]
struct Record {
    sequence: u64,
    moment: Moment,
    event: Event,
}

#[derive(Debug)]
struct Journal {
    pending: Vec<Record>,
    next: u64,
    lost: u64,
    closed: bool,
    problem: Option<String>,
    last_sync_ns: Option<u64>,
}

impl Default for Journal {
    fn default() -> Self {
        Self {
            pending: Vec::with_capacity(MEASURE_EVENT_LIMIT),
            next: 0,
            lost: 0,
            closed: false,
            problem: None,
            last_sync_ns: None,
        }
    }
}

impl Journal {
    fn push(&mut self, moment: Moment, event: Event) {
        if self.closed {
            return;
        }
        let sequence = self.next;
        self.next = self.next.saturating_add(1);
        if self.pending.len() >= MEASURE_EVENT_LIMIT {
            self.lost = self.lost.saturating_add(1);
        } else {
            self.pending.push(Record {
                sequence,
                moment,
                event,
            });
        }
    }

    fn problem(&self) -> Option<String> {
        self.problem.clone().or_else(|| {
            (self.lost > 0).then(|| {
                format!(
                    "input measurement journal overflow: {} required records dropped",
                    self.lost
                )
            })
        })
    }
}

#[derive(Debug)]
struct Shared {
    clock: ObservationClock,
    journal: Mutex<Journal>,
    drain: Mutex<Vec<Record>>,
}

/// Opt-in shared input measurement journal, never installed by an environment flag.
#[derive(Resource, Debug, Clone)]
pub struct MeasureDiagnostics(Arc<Shared>);

impl MeasureDiagnostics {
    fn with_clock(clock: ObservationClock) -> Self {
        Self(Arc::new(Shared {
            clock,
            journal: Mutex::new(default()),
            drain: Mutex::new(Vec::with_capacity(MEASURE_EVENT_LIMIT)),
        }))
    }

    pub(crate) fn clock(&self) -> ObservationClock {
        self.0.clock.clone()
    }

    pub(crate) fn begin(&self, moment: Moment, interval_ns: Option<u64>) {
        if let Ok(mut journal) = self.0.journal.lock() {
            journal.push(
                self.0.clock.now(),
                Event::Begin {
                    begin_ns: moment.elapsed_ns,
                    interval_ns,
                },
            );
            if journal
                .last_sync_ns
                .is_none_or(|last| moment.elapsed_ns.saturating_sub(last) >= 1_000_000_000)
            {
                let before = self.0.clock.now();
                let wall = SystemTime::now().duration_since(UNIX_EPOCH);
                let after = self.0.clock.now();
                if let Ok(wall) = wall {
                    journal.push(
                        after,
                        Event::ClockSync {
                            before_ns: before.elapsed_ns,
                            unix_ns: wall.as_nanos(),
                            after_ns: after.elapsed_ns,
                        },
                    );
                    journal.last_sync_ns = Some(after.elapsed_ns);
                } else {
                    journal.problem.get_or_insert_with(|| {
                        "required process-memory clock synchronization unavailable".into()
                    });
                }
            }
        }
    }

    pub(crate) fn work(&self, frame: u64, stage: WorkStage, work_ns: u64, wait_ns: u64) {
        if let Ok(mut journal) = self.0.journal.lock() {
            journal.push(
                self.0.clock.now(),
                Event::Work {
                    frame,
                    stage,
                    work_ns,
                    wait_ns,
                },
            );
        }
    }

    pub(crate) fn state(&self, payload: String) {
        if payload.len() > 16_000 {
            self.fail("input measurement state exceeds its 16000-byte limit");
            return;
        }
        if let Ok(mut journal) = self.0.journal.lock() {
            journal.push(self.0.clock.now(), Event::State(payload));
        }
    }

    pub(crate) fn comparison_state(&self, payload: String) {
        if payload.len() > 16_000 {
            self.fail("comparison state exceeds its 16000-byte limit");
        } else if let Ok(mut journal) = self.0.journal.lock() {
            journal.push(
                self.0.clock.now(),
                Event::State(format!("comparison=true\t{payload}")),
            );
        }
    }

    pub(crate) fn fail(&self, reason: &str) {
        if let Ok(mut journal) = self.0.journal.lock() {
            journal.problem.get_or_insert_with(|| reason.into());
        }
    }

    /// Sticky loss/missing-data status, unaffected by draining or sealing.
    #[must_use]
    pub fn problem(&self) -> Option<String> {
        self.0.journal.lock().map_or_else(
            |_| Some("input measurement journal lock poisoned".into()),
            |j| j.problem(),
        )
    }

    /// Drains into an empty reusable buffer, retaining the prefix even on loss.
    ///
    /// # Errors
    /// Refuses undrained output, poison, overflow or a required observation failure.
    pub fn drain_lines(&self, output: &mut Vec<String>) -> Result<(), String> {
        if !output.is_empty() {
            return Err("input measurement output must be empty before draining".into());
        }
        let mut batch = self
            .0
            .drain
            .lock()
            .map_err(|_| "input measurement drain lock poisoned")?;
        let problem = {
            let mut journal = self
                .0
                .journal
                .lock()
                .map_err(|_| "input measurement journal lock poisoned")?;
            std::mem::swap(&mut journal.pending, &mut batch);
            journal.problem()
        };
        output.extend(batch.drain(..).map(line));
        if let Some(problem) = problem {
            output.push("MEASURE-LOSS\tversion=1\trequired_evidence_missing=true".into());
            return Err(problem);
        }
        Ok(())
    }

    /// Seals the observed interval, not worker idleness or physical GPU release.
    ///
    /// # Errors
    /// Refuses a poisoned journal. Required observation failures remain sticky.
    pub fn seal_line(&self, scenario_ok: bool) -> Result<String, String> {
        let mut journal = self
            .0
            .journal
            .lock()
            .map_err(|_| "input measurement journal lock poisoned")?;
        if journal.closed {
            return Err("input measurement already sealed".into());
        }
        journal.closed = true;
        let moment = self.0.clock.now();
        Ok(format!(
            "MEASURE-END\tversion=1\tcomplete={}\tmain_frame={}\trecords={}\telapsed_ns={}\tshutdown_idle_or_gpu_release_claim=false",
            scenario_ok && journal.problem().is_none(),
            moment.frame,
            journal.next,
            moment.elapsed_ns
        ))
    }
}

fn line(record: Record) -> String {
    let payload = match record.event {
        Event::Begin {
            begin_ns,
            interval_ns,
        } => format!(
            "kind=frame-begin\tbegin_ns={begin_ns}\tinterval_ns={}\tinterval_scope=First-to-First",
            interval_ns.map_or_else(|| "unavailable-first-frame".into(), |n| n.to_string())
        ),
        Event::ClockSync {
            before_ns,
            unix_ns,
            after_ns,
        } => format!(
            "kind=clock-sync\tbefore_ns={before_ns}\tunix_ns={unix_ns}\tafter_ns={after_ns}\tscope=bracketed-wall-observation-not-shared-process-monotonic"
        ),
        Event::Work {
            frame,
            stage,
            work_ns,
            wait_ns,
        } => {
            let kind = match stage {
                WorkStage::Main => "main-work",
                WorkStage::Render => "render-work",
                WorkStage::Cap => "cap-wait",
            };
            format!(
                "kind={kind}\tmeasured_frame={frame}\twork_ns={work_ns}\twait_ns={wait_ns}\tgpu_execution=unavailable-not-enabled"
            )
        }
        Event::State(payload) => {
            if let Some(payload) = payload.strip_prefix("comparison=true\t") {
                format!("kind=comparison-state\t{payload}")
            } else {
                format!("kind=state\t{payload}")
            }
        }
    };
    format!(
        "MEASURE-EVENT\tversion=1\tsequence={}\telapsed_ns={}\tobserved_main_frame={}\t{payload}",
        record.sequence, record.moment.elapsed_ns, record.moment.frame
    )
}

/// Installs real-input measurements only for the explicit measurement launch.
pub struct ScenarioMeasurePlugin;

impl Plugin for ScenarioMeasurePlugin {
    fn build(&self, app: &mut App) {
        install(app, true);
    }
}

/// Only common interval/clock/settings evidence, without the full collector.
/// Used on both sides of collection-overhead comparisons, never normal startup.
pub struct ScenarioTimingPlugin;

impl Plugin for ScenarioTimingPlugin {
    fn build(&self, app: &mut App) {
        install(app, false);
    }
}

fn install(app: &mut App, full: bool) {
    let clock = app
        .world()
        .get_resource::<crate::LoadingDiagnostics>()
        .map_or_else(ObservationClock::default, |d| d.0.clock.clone());
    clock.install(app);
    let diagnostics = MeasureDiagnostics::with_clock(clock.clone());
    if let Some(mut run) = app.world_mut().get_resource_mut::<crate::ScenarioRun>() {
        run.observation_clock = Some(clock);
    }
    if full {
        crate::frame_work::install_witness(app, &diagnostics);
    }
    app.insert_resource(diagnostics).add_systems(
        Last,
        crate::scenario_measure_state::compare
            .after(crate::ScenarioSystems::Observe)
            .before(crate::FrameWorkSet::MainEnds),
    );
    if full {
        app.add_systems(
            Last,
            crate::scenario_measure_state::collect
                .after(crate::ScenarioSystems::Observe)
                .before(crate::FrameWorkSet::MainEnds),
        );
    }
}

#[cfg(test)]
#[path = "scenario_measure_tests.rs"]
mod tests;
