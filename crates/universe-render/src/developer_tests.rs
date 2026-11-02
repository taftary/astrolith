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
