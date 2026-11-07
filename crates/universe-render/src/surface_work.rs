//! One shared ground/proxy application interval, with real indivisible units.

use crate::loading_diagnostic::{LoadingEvent, Producer};
use crate::patches::{PatchTasks, may_apply};

#[must_use]
pub(crate) fn may_prepare(tasks: &PatchTasks) -> bool {
    tasks.apply_started.is_some_and(|started| {
        may_apply(
            tasks.apply_units,
            tasks.apply_uploads,
            started.elapsed().as_secs_f64() * 1000.0,
            tasks.apply_budget_ms,
        )
    })
}

pub(crate) fn record_proxy(
    tasks: &mut PatchTasks,
    started: std::time::Instant,
    witness: Option<&Producer>,
) {
    let unit_ms = started.elapsed().as_secs_f64() * 1000.0;
    tasks.apply_units += 1;
    tasks.apply_worst_ms = tasks.apply_worst_ms.max(unit_ms);
    if unit_ms > tasks.apply_budget_ms {
        tasks.apply_overruns = tasks.apply_overruns.saturating_add(1);
    }
    if let Some(witness) = witness {
        witness.emit(LoadingEvent::ProxyApplied {
            unit_ms,
            budget_ms: tasks.apply_budget_ms,
        });
        witness.emit(LoadingEvent::ApplyBatch {
            units: 1,
            uploads: 0,
            elapsed_ms: unit_ms,
            budget_ms: tasks.apply_budget_ms,
            work_share: tasks.apply_work_share,
        });
    }
}
