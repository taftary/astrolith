//! Hermetic journal/provenance tests; these do not prove real-window loading.

use super::*;

fn source(epoch: u64) -> Source {
    Source {
        root: 42,
        open: 7,
        host: None,
        epoch: Some(Epoch(epoch)),
        surface: None,
    }
}

fn record(event: LoadingEvent) -> Record {
    Record {
        sequence: 99,
        elapsed_ns: 123,
        observed_main_frame: 4,
        source: source(0),
        event,
    }
}

#[test]
fn transient_events_are_retained_in_order_and_capacity_never_grows() {
    let mut journal = Journal::new(2);
    let capacity = journal.pending.capacity();
    journal.push(record(LoadingEvent::Epoch));
    journal.push(record(LoadingEvent::InboxCleared {
        entries: 1,
        bytes: 40,
    }));
    assert_eq!(journal.pending.len(), 2);
    assert_eq!(journal.pending[0].sequence, 0);
    assert_eq!(journal.pending[1].sequence, 1);
    journal.push(record(LoadingEvent::Epoch));
    assert_eq!(journal.pending.len(), 2);
    assert_eq!(journal.lost, 1);
    assert_eq!(journal.pending.capacity(), capacity);
    journal.pending.clear();
    journal.push(record(LoadingEvent::Epoch));
    assert_eq!(
        journal.pending[0].sequence, 3,
        "dropped required records leave a sequence gap"
    );
    assert_eq!(journal.lost, 1, "draining never clears loss");
}

#[test]
fn loss_keeps_partial_evidence_and_cannot_be_cleared_by_draining() {
    let diagnostics = LoadingDiagnostics::default();
    {
        let mut journal = diagnostics.0.journal.lock().unwrap();
        *journal = Journal::new(1);
        journal.push(record(LoadingEvent::Epoch));
        journal.push(record(LoadingEvent::Epoch));
    }
    let mut output = Vec::new();
    assert!(diagnostics.drain_lines(&mut output).is_err());
    assert!(output[0].starts_with("LOAD-EVENT\tversion=1\tsequence=0\telapsed_ns=123"));
    assert_eq!(output[1], "LOAD-LOSS\tversion=1\tdropped=1");
    assert!(
        diagnostics
            .problem()
            .unwrap()
            .contains("1 required records dropped")
    );
    assert!(diagnostics.drain_lines(&mut output).is_err());
    assert_eq!(
        output.len(),
        2,
        "an unconsumed output buffer is not overwritten"
    );
    output.clear();
    assert!(diagnostics.drain_lines(&mut output).is_err());
}

#[test]
fn old_workers_keep_original_epoch_and_cancelled_is_not_built() {
    let diagnostics = LoadingDiagnostics::default();
    let first = diagnostics.epoch(&universe_core::nav::Universe::new(42));
    let key = PatchKey::root(universe_core::cube_sphere::Face::PosX);
    let cancelled = first.job(key, true, false);
    let finished = first.job(key, false, false);
    let second = diagnostics.epoch(&universe_core::nav::Universe::new(13));
    assert_ne!(first.source, second.source);
    finished.finish(None);
    drop(cancelled);
    let journal = diagnostics.0.journal.lock().unwrap();
    assert!(matches!(
        journal.pending[4].event,
        LoadingEvent::Built {
            cpu_bytes: None,
            ..
        }
    ));
    assert!(matches!(
        journal.pending[5].event,
        LoadingEvent::Cancelled { .. }
    ));
    assert_eq!(journal.pending[4].source, first.source);
    assert_eq!(journal.pending[5].source, first.source);
    assert!(matches!(
        journal.pending[4].event,
        LoadingEvent::Built {
            epoch_active: false,
            ..
        }
    ));
}

#[test]
fn sealed_interval_does_not_claim_later_worker_teardown_or_clear_any_loss() {
    let diagnostics = LoadingDiagnostics::default();
    let producer = diagnostics.epoch(&universe_core::nav::Universe::new(42));
    let boundary = diagnostics.seal().unwrap();
    assert_eq!(boundary.records, 1);
    producer.emit(LoadingEvent::Epoch);
    let mut lines = Vec::new();
    diagnostics.drain_lines(&mut lines).unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(diagnostics.seal().unwrap().records, 1);
    assert!(diagnostics.problem().is_none());
}

#[test]
fn ordinary_app_has_no_journal_and_opt_in_installs_no_gameplay_state() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.update();
    assert!(!app.world().contains_resource::<LoadingDiagnostics>());
    app.add_plugins(LoadingDiagnosticsPlugin);
    app.update();
    assert_eq!(
        app.world()
            .resource::<LoadingDiagnostics>()
            .0
            .frame
            .load(Ordering::Relaxed),
        1
    );
    assert!(!app.world().contains_resource::<crate::Universe>());
    assert!(
        !app.world()
            .contains_resource::<crate::patches::PatchTasks>()
    );
}
