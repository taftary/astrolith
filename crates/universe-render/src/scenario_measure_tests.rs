//! Hermetic bounded clock/journal tests, never real-window performance evidence.

use super::*;

#[test]
fn timing_only_has_no_loading_state_or_work_collector_and_normal_start_has_none() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    assert!(!app.world().contains_resource::<MeasureDiagnostics>());
    assert!(!app.world().contains_resource::<crate::LoadingDiagnostics>());
    let scenario =
        crate::parse_scenario("scenario-v2\nseed 42\nsetup loaded\ntimeout 20\nhold 1").unwrap();
    app.insert_resource(crate::ScenarioRun::new(scenario))
        .add_plugins(ScenarioTimingPlugin);
    app.update();
    let diagnostics = app.world().resource::<MeasureDiagnostics>();
    assert!(diagnostics.problem().is_none());
    let mut rows = Vec::new();
    diagnostics.drain_lines(&mut rows).unwrap();
    assert!(rows.iter().any(|row| row.contains("kind=frame-begin")));
    assert!(rows.iter().all(|row| !row.contains("kind=state")
        && !row.contains("kind=main-work")
        && !row.contains("kind=render-work")));
    assert!(!app.world().contains_resource::<crate::LoadingDiagnostics>());
}

#[test]
fn input_work_records_keep_exact_measured_frame_and_loss_is_sticky() {
    let diagnostics = MeasureDiagnostics::with_clock(default());
    diagnostics.begin(
        Moment {
            frame: 4,
            elapsed_ns: 100,
        },
        Some(30),
    );
    diagnostics.work(2, WorkStage::Render, 40, 80);
    let mut rows = Vec::new();
    diagnostics.drain_lines(&mut rows).unwrap();
    assert!(
        rows.iter()
            .any(|row| row.contains("kind=render-work\tmeasured_frame=2\twork_ns=40\twait_ns=80"))
    );
    rows.clear();
    for _ in 0..=MEASURE_EVENT_LIMIT {
        diagnostics.work(4, WorkStage::Main, 10, 0);
    }
    assert!(diagnostics.drain_lines(&mut rows).is_err());
    assert!(rows.last().unwrap().starts_with("MEASURE-LOSS"));
    rows.clear();
    assert!(diagnostics.drain_lines(&mut rows).is_err());
    assert!(
        diagnostics
            .seal_line(true)
            .unwrap()
            .contains("complete=false")
    );
}

#[test]
fn sealed_input_interval_never_accepts_later_teardown_as_evidence() {
    let diagnostics = MeasureDiagnostics::with_clock(default());
    diagnostics.work(1, WorkStage::Main, 10, 0);
    let end = diagnostics.seal_line(true).unwrap();
    diagnostics.work(2, WorkStage::Render, 10, 0);
    let mut rows = Vec::new();
    diagnostics.drain_lines(&mut rows).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(end.contains("\trecords=1\t"));
    assert!(diagnostics.seal_line(true).is_err());
}

#[test]
fn input_and_loading_share_one_clock_without_adding_normal_gameplay_resources() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, crate::LoadingDiagnosticsPlugin));
    let scenario =
        crate::parse_scenario("scenario-v2\nseed 42\nsetup loaded\ntimeout 20\nhold 1").unwrap();
    app.insert_resource(crate::ScenarioRun::new(scenario));
    // The full collector needs real window/render state. Missing required timing
    // clocks must fail; this hermetic test exercises only clock/link installation.
    let clock = app
        .world()
        .resource::<crate::LoadingDiagnostics>()
        .0
        .clock
        .clone();
    app.world_mut()
        .resource_mut::<crate::ScenarioRun>()
        .observation_clock = Some(clock.clone());
    let diagnostics = MeasureDiagnostics::with_clock(clock);
    crate::frame_work::install_witness(&mut app, &diagnostics);
    assert!(diagnostics.problem().unwrap().contains("FrameWork"));
    app.update();
    let frame = app.world().resource::<crate::LoadingDiagnostics>().frame();
    let mut run = app.world_mut().resource_mut::<crate::ScenarioRun>();
    run.trace("ACTION\t0\tHold(1.0)".into());
    assert!(run.pending[0].contains(&format!("\tmain_frame={frame}\t")));
    assert!(!app.world().contains_resource::<crate::Universe>());
}
