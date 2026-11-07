//! Opt-in bounded records at actual production loading boundaries (#1238 T5).
//!
//! Producers keep no asset handles and never supply a loading decision. Workers
//! retain their original epoch even after a body/atlas change. Formatting and
//! draining happen on the consumer, not on worker threads. This is a journal,
//! not a claim that the resource-lifetime/profiling capability is complete.

use bevy::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use universe_core::lod::PatchKey;
use universe_core::moon_catalog::PlanetIdentity;
use universe_core::surface_observation::SurfaceIdentity;

/// Maximum undrained loading events, including worker events between frames.
pub const LOADING_EVENT_LIMIT: usize = 8_192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Epoch(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Job(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Source {
    root: u64,
    open: u64,
    pub(super) host: Option<PlanetIdentity>,
    epoch: Option<Epoch>,
    surface: Option<SurfaceIdentity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QueueStage {
    Collected,
    Dispatched,
    Applied,
}

/// Scalar records only: no mesh copies, strings or lifetime-extending handles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum LoadingEvent {
    Resource(super::resource_diagnostic::ResourceEvent),
    Epoch,
    Abandoned {
        pending: usize,
        completed: usize,
    },
    Queued {
        job: u64,
        key: PatchKey,
        full: bool,
        synchronous: bool,
    },
    Started {
        job: u64,
        key: PatchKey,
    },
    Built {
        job: u64,
        key: PatchKey,
        cpu_bytes: Option<usize>,
        epoch_active: bool,
    },
    Cancelled {
        job: u64,
        key: PatchKey,
    },
    Taken {
        key: PatchKey,
        succeeded: bool,
    },
    Applied {
        key: PatchKey,
        replacement: bool,
        upload: bool,
        unit_ms: f64,
        budget_ms: f64,
    },
    ApplyBatch {
        units: usize,
        uploads: usize,
        elapsed_ms: f64,
        budget_ms: f64,
        work_share: f64,
    },
    Cache {
        entries: usize,
        info_bytes: usize,
        cpu_bytes: usize,
        tile_bytes: usize,
        total_bytes: usize,
    },
    Packed {
        key: PatchKey,
        replaced: bool,
        entries: usize,
        bytes: usize,
    },
    TileTaken {
        key: PatchKey,
        accepted: bool,
        actual_seed: u64,
        actual_radius_bits: u64,
        entries: usize,
        bytes: usize,
    },
    InboxCleared {
        entries: usize,
        bytes: usize,
    },
    HeightCache {
        entries: usize,
        cap: usize,
        logical_bytes: usize,
    },
    Queues {
        stage: QueueStage,
        pending: usize,
        completed: usize,
        wanted: usize,
        build_candidates: usize,
        missing_wanted: Option<usize>,
        cap_bytes: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Record {
    sequence: u64,
    elapsed_ns: u64,
    observed_main_frame: u64,
    source: Source,
    event: LoadingEvent,
}

#[derive(Debug)]
struct Journal {
    pending: Vec<Record>,
    next: u64,
    lost: u64,
    limit: usize,
    closed: bool,
    active_epoch: Option<Epoch>,
}

impl Journal {
    fn new(limit: usize) -> Self {
        Self {
            pending: Vec::with_capacity(limit),
            next: 0,
            lost: 0,
            limit,
            closed: false,
            active_epoch: None,
        }
    }

    fn push(&mut self, mut record: Record) {
        if self.closed {
            return; // Outside the explicitly sealed measurement interval.
        }
        match &mut record.event {
            LoadingEvent::Epoch => self.active_epoch = record.source.epoch,
            LoadingEvent::Abandoned { .. } if self.active_epoch == record.source.epoch => {
                self.active_epoch = None
            }
            LoadingEvent::Built { epoch_active, .. } => {
                *epoch_active =
                    record.source.epoch.is_some() && self.active_epoch == record.source.epoch
            }
            _ => (),
        }
        record.sequence = self.next;
        self.next = self.next.saturating_add(1);
        if self.pending.len() == self.limit {
            self.lost = self.lost.saturating_add(1);
        } else {
            self.pending.push(record);
        }
    }
}

#[derive(Debug)]
pub(super) struct Shared {
    journal: Mutex<Journal>,
    drain: Mutex<Vec<Record>>,
    started: Instant,
    frame: AtomicU64,
    epoch: AtomicU64,
    job: AtomicU64,
    pub(super) resources: Mutex<super::resource_diagnostic::Registry>,
}

/// Explicitly installed loading journal. Ordinary startup never creates it.
#[derive(Resource, Debug, Clone)]
pub struct LoadingDiagnostics(pub(super) Arc<Shared>);

/// End of the observed interval, not a promise that every worker/device is idle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadingBoundary {
    /// Last main-frame identity admitted to this interval.
    pub main_frame: u64,
    /// Sequence numbers issued before sealing (including any dropped records).
    pub records: u64,
    /// End relative to the same monotonic clock used by every loading event.
    pub elapsed_ns: u64,
}

impl Default for LoadingDiagnostics {
    fn default() -> Self {
        Self(Arc::new(Shared {
            journal: Mutex::new(Journal::new(LOADING_EVENT_LIMIT)),
            drain: Mutex::new(Vec::with_capacity(LOADING_EVENT_LIMIT)),
            started: Instant::now(),
            frame: AtomicU64::new(0),
            epoch: AtomicU64::new(0),
            job: AtomicU64::new(0),
            resources: Mutex::new(super::resource_diagnostic::Registry::default()),
        }))
    }
}

impl LoadingDiagnostics {
    /// Sticky loss/poison status. Draining cannot make a lost-record run complete.
    #[must_use]
    pub fn problem(&self) -> Option<String> {
        let journal = match self.0.journal.lock() {
            Ok(journal) if journal.lost == 0 => None,
            Ok(journal) => Some(format!(
                "loading diagnostic buffer overflow: {} required records dropped",
                journal.lost
            )),
            Err(_) => Some("loading diagnostic journal lock poisoned".into()),
        };
        journal.or_else(|| match self.0.resources.lock() {
            Ok(registry) => registry.problem.clone(),
            Err(_) => Some("loading resource registry lock poisoned".into()),
        })
    }

    /// Drains raw records into a reusable, empty output buffer, including on loss.
    /// I/O belongs to the app; a consumer must retain these lines before reuse.
    ///
    /// # Errors
    /// Refuses an undrained output buffer or a poisoned/lossy journal. Loss keeps
    /// the retained partial records and remains sticky after this call.
    pub fn drain_lines(&self, output: &mut Vec<String>) -> Result<(), String> {
        if !output.is_empty() {
            return Err("loading diagnostic output buffer must be empty before draining".into());
        }
        let mut batch = self
            .0
            .drain
            .lock()
            .map_err(|_| "loading diagnostic drain lock poisoned")?;
        let lost = {
            let mut journal = self
                .0
                .journal
                .lock()
                .map_err(|_| "loading diagnostic journal lock poisoned")?;
            std::mem::swap(&mut journal.pending, &mut batch);
            journal.lost
        };
        // No production/worker thread waits for path/float/string formatting.
        output.extend(batch.drain(..).map(record::line));
        if lost != 0 {
            output.push(format!("LOAD-LOSS\tversion=1\tdropped={lost}"));
            return Err(format!(
                "loading diagnostic buffer overflow: {lost} required records dropped"
            ));
        }
        Ok(())
    }

    pub(crate) fn epoch(&self, universe: &universe_core::nav::Universe) -> Producer {
        let surface = PlanetIdentity::from_open_cell(universe)
            .ok()
            .map(SurfaceIdentity::Planet);
        self.surface_epoch(universe, surface)
    }

    /// The existing production patch tree supplies its owner, never the chosen
    /// fixture or nearest proxy. Future real moon ground uses this same journal.
    pub(crate) fn surface_epoch(
        &self,
        universe: &universe_core::nav::Universe,
        surface: Option<SurfaceIdentity>,
    ) -> Producer {
        let mut producer = self.context(universe);
        producer.source.surface = surface;
        producer.source.epoch = Some(Epoch(self.0.epoch.fetch_add(1, Ordering::Relaxed)));
        producer.emit(LoadingEvent::Epoch);
        producer
    }

    /// Seals the session after its real app runner exits. Later worker teardown
    /// is outside this interval, not evidence of GPU release or a completed job.
    /// Retained records can still be drained, and sticky problems remain visible.
    ///
    /// # Errors
    /// Refuses a poisoned journal. This never clears a measurement problem.
    pub fn seal(&self) -> Result<LoadingBoundary, String> {
        let mut journal = self
            .0
            .journal
            .lock()
            .map_err(|_| "loading diagnostic journal lock poisoned")?;
        journal.closed = true;
        Ok(LoadingBoundary {
            main_frame: self.frame(),
            records: journal.next,
            elapsed_ns: u64::try_from(self.0.started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        })
    }

    pub(crate) fn context(&self, universe: &universe_core::nav::Universe) -> Producer {
        Producer {
            diagnostics: self.clone(),
            source: Source {
                root: universe.root.0,
                open: universe.open_seed(),
                host: PlanetIdentity::from_open_cell(universe).ok(),
                epoch: None,
                surface: None,
            },
        }
    }

    pub(crate) fn frame(&self) -> u64 {
        self.0.frame.load(Ordering::Relaxed)
    }

    pub(super) fn emit(&self, source: Source, event: LoadingEvent) {
        let shared = &self.0;
        if let Ok(mut journal) = shared.journal.lock() {
            journal.push(Record {
                sequence: 0,
                elapsed_ns: u64::try_from(shared.started.elapsed().as_nanos()).unwrap_or(u64::MAX),
                observed_main_frame: shared.frame.load(Ordering::Relaxed),
                source,
                event,
            });
        }
    }
}

/// Immutable provenance handed to worker clones; never follows later placement.
#[derive(Debug, Clone)]
pub(crate) struct Producer {
    pub(super) diagnostics: LoadingDiagnostics,
    pub(super) source: Source,
}

impl Producer {
    pub(crate) fn emit(&self, event: LoadingEvent) {
        self.diagnostics.emit(self.source, event);
    }

    pub(crate) fn job(&self, key: PatchKey, full: bool, synchronous: bool) -> Ticket {
        let job = Job(self.diagnostics.0.job.fetch_add(1, Ordering::Relaxed));
        self.emit(LoadingEvent::Queued {
            job: job.0,
            key,
            full,
            synchronous,
        });
        Ticket {
            producer: self.clone(),
            key,
            job,
            finished: false,
        }
    }

    pub(crate) fn cache(&self, cache: &crate::patch_cache::PatchCache) {
        let (info_bytes, cpu_bytes, tile_bytes) = cache.diagnostic_parts();
        self.emit(LoadingEvent::Cache {
            entries: cache.map.len(),
            info_bytes,
            cpu_bytes,
            tile_bytes,
            total_bytes: cache.bytes(),
        });
    }

    pub(crate) fn queues(
        &self,
        tasks: &crate::patches::PatchTasks,
        stage: QueueStage,
        missing_wanted: Option<usize>,
    ) {
        self.emit(LoadingEvent::Queues {
            stage,
            pending: tasks.pending.len(),
            completed: tasks.completed.len(),
            wanted: tasks.wanted.len(),
            build_candidates: tasks.build.len(),
            missing_wanted,
            cap_bytes: tasks.cap_bytes,
        });
    }
}

/// A real queued job. Dropping an unpolled/cancelled future is not completion.
#[derive(Debug)]
pub(crate) struct Ticket {
    producer: Producer,
    key: PatchKey,
    job: Job,
    finished: bool,
}

impl Ticket {
    pub(crate) fn start(&mut self) {
        self.producer.emit(LoadingEvent::Started {
            job: self.job.0,
            key: self.key,
        });
    }

    pub(crate) fn finish(mut self, result: Option<&universe_core::lod_mesh::PatchMesh>) {
        self.producer.emit(LoadingEvent::Built {
            job: self.job.0,
            key: self.key,
            cpu_bytes: result.map(crate::patch_cache::cpu_bytes),
            epoch_active: false, // Filled atomically from the journal's real epoch order.
        });
        self.finished = true;
    }
}

impl Drop for Ticket {
    fn drop(&mut self) {
        if !self.finished {
            self.producer.emit(LoadingEvent::Cancelled {
                job: self.job.0,
                key: self.key,
            });
        }
    }
}

#[path = "loading_record.rs"]
mod record;

fn begin(diagnostics: Res<LoadingDiagnostics>) {
    diagnostics.0.frame.fetch_add(1, Ordering::Relaxed);
}

/// Installs the opt-in journal, frame and main/render scalar lifetime witnesses.
/// No env switches. Public streaming is the app consumer, never a worker action.
pub struct LoadingDiagnosticsPlugin;

impl Plugin for LoadingDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadingDiagnostics>()
            .add_systems(First, begin);
        super::resource_diagnostic::install(app);
    }
}

#[cfg(test)]
#[path = "loading_diagnostic_tests.rs"]
mod tests;
