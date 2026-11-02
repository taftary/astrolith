//! Developer layer tests (#1128 T2: C1, C8, C13).

use super::DeveloperLayer;
use crate::input::Navigation;
use crate::lib_tests::{click, headless_app, other_root_portal};
use crate::{SettingsStore, Universe};
use bevy::prelude::*;
use universe_core::settings::{DeveloperMarkers, DeveloperPart};

/// Saved choices with the master `enabled` and every filter `filters`.
fn markers(enabled: bool, filters: bool) -> DeveloperMarkers {
    let mut saved = DeveloperMarkers::default();
    saved.enabled = enabled;
    for part in DeveloperPart::ALL {
        saved.set_filter(part, filters);
    }
    saved
}

/// Runs the start, a click on another portal and one more frame under the
/// saved `choice`; returns navigation, the open cell's snapshot and the
/// number of developer parts shown at the end.
fn journey_under(choice: DeveloperMarkers) -> (Navigation, String, usize) {
    let mut app = headless_app();
    app.world_mut()
        .resource_mut::<SettingsStore>()
        .0
        .developer_markers = choice;
    app.update();
    let start = app.world().resource::<Navigation>().destination;
    let first = start.and_then(|held| held.markers().first().copied());
    let (other, _) = other_root_portal(&app, first.expect("journey"));
    click(&mut app, other);
    app.update();
    let nav = *app.world().resource::<Navigation>();
    let snapshot = app.world().resource::<Universe>().0.snapshot();
    let shown = app.world().resource::<DeveloperLayer>().shown_count();
    (nav, snapshot, shown)
}

#[test]
fn navigation_survives_all_developer_filters_off() {
    // C8/C13: hover, target, destination and the universe are the same with
    // the layer off, on with every filter off, and on with every filter on.
    let (fresh_nav, fresh_universe, fresh_shown) = journey_under(DeveloperMarkers::default());
    assert_eq!(
        fresh_shown, 0,
        "C1: a fresh profile shows no developer parts"
    );
    assert!(fresh_nav.target.is_some(), "the click targets a portal");
    let (off_nav, off_universe, off_shown) = journey_under(markers(true, false));
    assert_eq!(off_shown, 0, "every filter off lets nothing through");
    assert_eq!(off_nav, fresh_nav, "navigation ignores the filters");
    assert_eq!(
        off_universe, fresh_universe,
        "the universe ignores the filters"
    );
    let (on_nav, on_universe, on_shown) = journey_under(markers(true, true));
    assert_eq!(on_nav, fresh_nav, "navigation ignores the layer");
    assert_eq!(
        on_universe, fresh_universe,
        "the universe ignores the layer"
    );
    // Release forcibly excludes the layer (AC2); debug shows it.
    assert_eq!(on_shown > 0, cfg!(debug_assertions));
    let layer = DeveloperLayer::default();
    assert!(!layer.effective().enabled, "the layer starts off");
}

#[test]
fn each_filter_lets_through_only_its_own_parts() {
    if !cfg!(debug_assertions) {
        return;
    }
    let mut app = headless_app();
    for part in DeveloperPart::ALL {
        let mut only = markers(true, false);
        only.set_filter(part, true);
        app.world_mut()
            .resource_mut::<SettingsStore>()
            .0
            .developer_markers = only;
        app.update();
        let layer = app.world().resource::<DeveloperLayer>();
        assert!(
            layer.shown().all(|record| record.spec().class == part),
            "{part:?} alone"
        );
    }
}

#[test]
fn the_layer_adds_only_on_top_of_todays_markers_within_budgets() {
    // Spec v3 B: baseline parts (today's dots and shapes) get developer
    // detail only from the outline up; every group stays under its cap.
    use universe_core::semantic_display::Stage;
    if !cfg!(debug_assertions) {
        return;
    }
    let mut app = headless_app();
    app.world_mut()
        .resource_mut::<SettingsStore>()
        .0
        .developer_markers = markers(true, true);
    for _ in 0..3 {
        app.update();
    }
    let layer = app.world().resource::<DeveloperLayer>();
    let mut groups = std::collections::BTreeMap::new();
    for (record, plan) in layer.planned() {
        let spec = record.spec();
        if spec.baseline {
            assert!(plan.stage >= Stage::Outline, "{:?}", record.id.kind);
        }
        assert!(plan.weight > 0.0 && plan.weight <= 1.0);
        let count = groups.entry(format!("{:?}", spec.budget)).or_insert(0usize);
        *count += 1;
        if let Some(cap) = spec.budget.cap() {
            assert!(*count <= cap, "{:?} under its cap", spec.budget);
        }
    }
}

#[test]
fn the_labels_filter_never_touches_navigation_labels() {
    // C10: navigation labels are the same with developer labels on or off.
    use crate::labels::PortalLabel;
    let visible = |choice: DeveloperMarkers| {
        let mut app = headless_app();
        app.world_mut()
            .resource_mut::<SettingsStore>()
            .0
            .developer_markers = choice;
        for _ in 0..3 {
            app.update();
        }
        let mut query = app
            .world_mut()
            .query_filtered::<&Visibility, With<PortalLabel>>();
        query
            .iter(app.world())
            .filter(|visibility| **visibility != Visibility::Hidden)
            .count()
    };
    let mut labels_off = markers(true, true);
    labels_off.set_filter(DeveloperPart::Labels, false);
    assert_eq!(visible(markers(true, true)), visible(labels_off));
    assert_eq!(visible(DeveloperMarkers::default()), visible(labels_off));
}

/// Opens the whole journey one marker at a time under `choice`; returns
/// what each step left: the open cell's snapshot, its seed, and navigation.
fn journey_steps(choice: DeveloperMarkers) -> Vec<(String, u64, Navigation)> {
    use universe_core::nav::MarkerIndex;
    let mut app = headless_app();
    // The sky holds still, so moving bodies sit at the same place in
    // every run (#981); only the developer choice differs.
    app.insert_resource(crate::sky::SkyClock(universe_core::clock::Clock {
        playing: false,
        ..universe_core::clock::Clock::default()
    }));
    app.world_mut()
        .resource_mut::<SettingsStore>()
        .0
        .developer_markers = choice;
    app.update();
    let way = app
        .world()
        .resource::<Navigation>()
        .destination
        .expect("journey");
    let mut out = Vec::new();
    for &marker in way.markers() {
        assert!(
            app.world_mut()
                .resource_mut::<Universe>()
                .open(MarkerIndex(marker))
        );
        app.update();
        let universe = &app.world().resource::<Universe>().0;
        out.push((
            universe.snapshot(),
            universe.open_seed(),
            *app.world().resource::<Navigation>(),
        ));
    }
    out
}

#[test]
fn marker_visibility_never_changes_universe_or_navigation() {
    // C13: the same journey opens the same cells with the same content,
    // seeds and navigation whatever the developer choices.
    let fresh = journey_steps(DeveloperMarkers::default());
    assert!(fresh.len() >= 9, "the journey reaches the planet");
    let mut paths_only = markers(true, false);
    paths_only.set_filter(DeveloperPart::Paths, true);
    for choice in [markers(true, true), markers(true, false), paths_only] {
        let other = journey_steps(choice);
        assert_eq!(other.len(), fresh.len());
        for (step, (a, b)) in fresh.iter().zip(&other).enumerate() {
            assert_eq!(a.0, b.0, "step {step}: same content");
            assert_eq!(a.1, b.1, "step {step}: same seed");
            assert_eq!(a.2, b.2, "step {step}: same navigation");
        }
    }
}
