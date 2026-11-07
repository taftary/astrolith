//! Explicit loading-sidecar v1 field names, not Rust Debug output or stdout.

use super::{LoadingEvent, QueueStage, Record};
use universe_core::lod::PatchKey;

fn patch(key: PatchKey) -> String {
    format!("{}/{}/{}/{}", key.face_index(), key.level, key.x, key.y)
}

fn payload(event: LoadingEvent) -> String {
    match event {
        LoadingEvent::Resource(event) => crate::resource_record::payload(event),
        LoadingEvent::Epoch => "kind=epoch-start".into(),
        LoadingEvent::Abandoned { pending, completed } => format!(
            "kind=epoch-abandoned\tpending_discarded={pending}\tcompleted_discarded={completed}"
        ),
        LoadingEvent::Queued {
            job,
            key,
            full,
            synchronous,
        } => format!(
            "kind=job-queued\tjob={job}\tpatch={}\tfull={full}\tsynchronous={synchronous}",
            patch(key)
        ),
        LoadingEvent::Started { job, key } => {
            format!("kind=job-started\tjob={job}\tpatch={}", patch(key))
        }
        LoadingEvent::Built {
            job,
            key,
            cpu_bytes,
            epoch_active,
        } => format!(
            "kind=job-finished\tjob={job}\tpatch={}\tsucceeded={}\tmesh_logical_cpu_bytes={}\tepoch_active_at_finish={epoch_active}",
            patch(key),
            cpu_bytes.is_some(),
            cpu_bytes.map_or_else(|| "unavailable".into(), |n| n.to_string())
        ),
        LoadingEvent::Cancelled { job, key } => {
            format!("kind=job-cancelled\tjob={job}\tpatch={}", patch(key))
        }
        LoadingEvent::Taken { key, succeeded } => format!(
            "kind=result-collected\tpatch={}\tsucceeded={succeeded}",
            patch(key)
        ),
        LoadingEvent::Applied {
            key,
            replacement,
            upload,
            unit_ms,
            budget_ms,
        } => format!(
            "kind=apply-unit\tpatch={}\tcpu_replacement={replacement}\tupload_returned_gpu={upload}\tunit_ms={unit_ms}\tbudget_ms={budget_ms}\tsingle_unit_overrun={}",
            patch(key),
            unit_ms > budget_ms
        ),
        LoadingEvent::ProxyApplied { unit_ms, budget_ms } => format!(
            "kind=proxy-apply-unit\tunit_ms={unit_ms}\tbudget_ms={budget_ms}\tsingle_unit_overrun={}",
            unit_ms > budget_ms
        ),
        LoadingEvent::ApplyBatch {
            units,
            uploads,
            elapsed_ms,
            budget_ms,
            work_share,
        } => format!(
            "kind=apply-batch\tunits={units}\tnew_patch_upload_attempts={uploads}\telapsed_ms={elapsed_ms}\tbudget_ms={budget_ms}\twork_share={work_share}\tincludes_synchronous_roots=false"
        ),
        LoadingEvent::Cache {
            entries,
            info_bytes,
            cpu_bytes,
            tile_bytes,
            total_bytes,
        } => format!(
            "kind=ground-cache\tentries={entries}\tinfo_accounted_bytes={info_bytes}\tcpu_accounted_bytes={cpu_bytes}\ttile_accounted_bytes={tile_bytes}\ttotal_accounted_bytes={total_bytes}\ttotal_gpu_allocation=false"
        ),
        LoadingEvent::Packed {
            key,
            replaced,
            entries,
            bytes,
        } => format!(
            "kind=inbox-insert\tpatch={}\treplaced={replaced}\tentries={entries}\tlogical_texel_bytes={bytes}\tcount_scope=shared-inbox-all-epochs",
            patch(key)
        ),
        LoadingEvent::TileTaken {
            key,
            accepted,
            actual_seed,
            actual_radius_bits,
            entries,
            bytes,
        } => format!(
            "kind=inbox-take\tpatch={}\taccepted={accepted}\tactual_relief_seed={actual_seed}\tactual_radius_bits={actual_radius_bits}\tentries={entries}\tlogical_texel_bytes={bytes}\tcount_scope=shared-inbox-all-epochs",
            patch(key)
        ),
        LoadingEvent::InboxCleared { entries, bytes } => format!(
            "kind=inbox-clear\tdiscarded_entries={entries}\tdiscarded_logical_texel_bytes={bytes}\tcount_scope=shared-inbox-all-epochs"
        ),
        LoadingEvent::HeightCache {
            entries,
            cap,
            logical_bytes,
        } => format!(
            "kind=height-cache\tentries={entries}\tentry_cap={cap}\tlogical_grid_bytes={logical_bytes}\tcount_scope=shared-cache-all-epochs\texternal_readers_included=false"
        ),
        LoadingEvent::Queues {
            stage,
            pending,
            completed,
            wanted,
            build_candidates,
            missing_wanted,
            cap_bytes,
        } => {
            let stage = match stage {
                QueueStage::Collected => "collected",
                QueueStage::Dispatched => "dispatched",
                QueueStage::Applied => "applied",
            };
            format!(
                "kind=queues\tstage={stage}\tpending={pending}\tcompleted={completed}\twanted={wanted}\tbuild_candidates={build_candidates}\tmissing_wanted_at_dispatch={}\tground_cap_bytes={cap_bytes}",
                missing_wanted
                    .map_or_else(|| "unavailable-before-dispatch".into(), |n| n.to_string())
            )
        }
    }
}

pub(super) fn line(record: Record) -> String {
    let source = record.source;
    let surface = match source.surface {
        Some(universe_core::surface_observation::SurfaceIdentity::Planet(_)) => "planet".into(),
        Some(universe_core::surface_observation::SurfaceIdentity::Moon(moon)) => {
            format!("moon:{}", moon.marker().0)
        }
        None => "not-a-surface-epoch".into(),
    };
    let host = source.host.map_or_else(
        || "unavailable-not-open-planet".into(),
        |p| {
            p.chain()
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(".")
        },
    );
    format!(
        "LOAD-EVENT\tversion=1\tsequence={}\telapsed_ns={}\tobserved_main_frame={}\troot_seed={}\topen_seed={}\tepoch={}\thost_path={host}\tsurface_owner={surface}\t{}",
        record.sequence,
        record.elapsed_ns,
        record.observed_main_frame,
        source.root,
        source.open,
        source
            .epoch
            .map_or_else(|| "not-a-patch-epoch".into(), |e| e.0.to_string()),
        payload(record.event)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_build_is_not_a_zero_byte_success_and_queue_stage_is_explicit() {
        let key = PatchKey::root(universe_core::cube_sphere::Face::PosX);
        let text = payload(LoadingEvent::Built {
            job: 4,
            key,
            cpu_bytes: None,
            epoch_active: false,
        });
        assert_eq!(
            text,
            "kind=job-finished\tjob=4\tpatch=0/0/0/0\tsucceeded=false\tmesh_logical_cpu_bytes=unavailable\tepoch_active_at_finish=false"
        );
        let text = payload(LoadingEvent::Queues {
            stage: QueueStage::Collected,
            pending: 1,
            completed: 2,
            wanted: 3,
            build_candidates: 4,
            missing_wanted: None,
            cap_bytes: 8_388_608,
        });
        assert!(text.contains("\tstage=collected\tpending=1\tcompleted=2"));
        assert!(text.contains(
            "\tmissing_wanted_at_dispatch=unavailable-before-dispatch\tground_cap_bytes=8388608"
        ));
    }
}
