//! One opt-in monotonic clock/main-frame identity shared by input and loading.

use bevy::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[derive(Debug)]
struct Shared {
    started: Instant,
    frame: AtomicU64,
    begin_ns: AtomicU64,
}

#[derive(Resource, Debug, Clone)]
pub(crate) struct ObservationClock(Arc<Shared>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Moment {
    pub frame: u64,
    pub elapsed_ns: u64,
}

impl Default for ObservationClock {
    fn default() -> Self {
        Self(Arc::new(Shared {
            started: Instant::now(),
            frame: AtomicU64::new(0),
            begin_ns: AtomicU64::new(0),
        }))
    }
}

impl ObservationClock {
    pub(crate) fn now(&self) -> Moment {
        Moment {
            frame: self.0.frame.load(Ordering::Relaxed),
            elapsed_ns: u64::try_from(self.0.started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        }
    }

    pub(crate) fn begin(&self) -> (Moment, Option<u64>) {
        let now = self.now().elapsed_ns;
        let previous = self.0.begin_ns.swap(now, Ordering::Relaxed);
        let frame = self
            .0
            .frame
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        (
            Moment {
                frame,
                elapsed_ns: now,
            },
            (frame > 1).then_some(now.saturating_sub(previous)),
        )
    }

    pub(crate) fn install(&self, app: &mut App) {
        if !app.is_plugin_added::<ObservationClockPlugin>() {
            app.insert_resource(self.clone())
                .add_plugins(ObservationClockPlugin);
        }
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ObservationSystems {
    Begins,
}

fn begin(
    clock: Res<ObservationClock>,
    measurement: Option<Res<crate::scenario_measure::MeasureDiagnostics>>,
) {
    let (moment, interval_ns) = clock.begin();
    if let Some(measurement) = measurement {
        measurement.begin(moment, interval_ns);
    }
}

struct ObservationClockPlugin;

impl Plugin for ObservationClockPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, begin.in_set(ObservationSystems::Begins));
    }
}
