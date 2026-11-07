//! Production apply boundary and optional loading witnesses (#1238 T5).
//! The existing priority, budget and cache decisions are unchanged.

use crate::loading_diagnostic::{LoadingEvent, Producer};
use crate::patches::{PatchCache, PatchTasks, apply_rank, may_apply};
use bevy::prelude::*;
use universe_core::lod::PatchKey;
use universe_core::planet_frame::PlanetFrame;
use universe_core::relief::Relief;

pub(crate) fn abandon(tasks: &mut PatchTasks) {
    if let Some(witness) = tasks.witness.take() {
        witness.emit(LoadingEvent::Abandoned {
            pending: tasks.pending.len(),
            completed: tasks.completed.len(),
        });
    }
}

/// Old jobs keep their queue slots and original observation provenance until
/// they finish. Only this generation's output may enter the application queue.
pub(crate) fn collect(tasks: &mut PatchTasks) {
    let mut index = 0;
    while index < tasks.pending.len() {
        if !tasks
            .pending
            .get(index)
            .is_some_and(|(_, task)| task.is_finished())
        {
            index += 1;
            continue;
        }
        let (key, pending) = tasks.pending.remove(index);
        let result = bevy::tasks::block_on(pending.task);
        if let Some(witness) = &pending.witness {
            witness.emit(LoadingEvent::Taken {
                key,
                succeeded: result.is_some(),
            });
        }
        if let Some(result) = result
            && crate::terrain_epoch::same_epoch(result.epoch.as_ref(), tasks.epoch.as_ref())
        {
            tasks.completed.push((key, result));
        }
    }
}

/// Root generation is synchronous and outside the asynchronous apply budget.
pub(crate) fn root(
    key: PatchKey,
    radius: f64,
    relief: &Relief,
    witness: Option<&Producer>,
) -> Option<universe_core::lod_mesh::PatchMesh> {
    let mut ticket = witness.map(|w| w.job(key, true, true));
    if let Some(ticket) = &mut ticket {
        ticket.start();
    }
    let result = universe_core::lod_mesh::build_patch(key, radius, relief);
    if let Some(ticket) = ticket {
        ticket.finish(result.as_ref());
    }
    result
}

#[allow(
    clippy::too_many_arguments,
    reason = "E-MOD-DOMAIN: existing apply boundary split from the large patch system"
)]
pub(crate) fn apply(
    tasks: &mut PatchTasks,
    cache: &mut PatchCache,
    grid: &mut crate::patch_grid::GridParams<'_>,
    frame: &PlanetFrame,
    relief: &Relief,
    meshes: &mut Assets<Mesh>,
    budget_ms: f64,
    work_share: f64,
) {
    tasks.completed.sort_by_key(|(key, _)| {
        (
            apply_rank(*key, &tasks.near, &tasks.build),
            std::cmp::Reverse(key.level),
            key.face_index(),
            key.x,
            key.y,
        )
    });
    let started = std::time::Instant::now();
    let (mut units, mut uploads) = (0usize, 0usize);
    while !tasks.completed.is_empty()
        && may_apply(
            units,
            uploads,
            started.elapsed().as_secs_f64() * 1000.0,
            budget_ms,
        )
    {
        let unit = std::time::Instant::now();
        let (_, result) = tasks.completed.remove(0);
        // Defense at the application boundary too: no stale result can replace
        // a CPU patch or upload a tile after owner/quality/resize invalidation.
        if !crate::terrain_epoch::same_epoch(result.epoch.as_ref(), tasks.epoch.as_ref())
            || result
                .epoch
                .as_ref()
                .is_some_and(|epoch| cache.recipe != Some(epoch.recipe))
        {
            units += 1;
            continue;
        }
        let mesh = result.mesh;
        let key = mesh.key;
        let replacement = cache.contains(key);
        let upload = if replacement {
            cache.set_cpu(mesh);
            false
        } else {
            let gpu = crate::patch_grid::upload(
                &mesh,
                frame.radius_m(),
                relief,
                &mut grid.atlas,
                &grid.inbox,
                meshes,
            );
            let uploaded = gpu.is_some();
            // A failed upload is not cover and must not suppress future builds.
            if uploaded {
                cache.insert(mesh, gpu);
            }
            uploads += 1;
            uploaded
        };
        units += 1;
        let unit_ms = unit.elapsed().as_secs_f64() * 1000.0;
        tasks.apply_worst_ms = tasks.apply_worst_ms.max(unit_ms);
        if unit_ms > budget_ms {
            tasks.apply_overruns = tasks.apply_overruns.saturating_add(1);
            debug!("ground apply overran its budget: {unit_ms:.2} ms over {budget_ms:.2} ms");
        }
        if let Some(witness) = &tasks.witness {
            witness.emit(LoadingEvent::Applied {
                key,
                replacement,
                upload,
                unit_ms,
                budget_ms,
            });
            witness.cache(cache);
        }
    }
    if let Some(witness) = &tasks.witness {
        witness.emit(LoadingEvent::ApplyBatch {
            units,
            uploads,
            elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
            budget_ms,
            work_share,
        });
    }
}
