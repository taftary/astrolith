//! Tail levels stop at L10 (#770 T2).
//!
//! The old L11-L14 surface stack is gone; the journey ends at the planet's
//! cell (L10). These tests prove the levels stop, the labels stop, and the
//! dive never opens below L10.

use universe_core::coords::{Level, MAX_LEVEL};
use universe_core::labels::level_label;
use universe_core::nav::{DEMO_SEED, MAX_NAV_LEVEL, MarkerIndex, Universe};
use universe_core::nest::MAX_OPEN_LEVEL;

#[test]
fn levels_stop_at_l10() {
    // MAX_LEVEL drops from 14 to 10, matching MAX_OPEN_LEVEL.
    assert_eq!(MAX_LEVEL, 10, "levels stop at L10");
    assert_eq!(MAX_OPEN_LEVEL, 10);
    assert_eq!(MAX_NAV_LEVEL, 10);
    assert_eq!(Level::MAX.get(), 10);
    assert!(Level::new(11).is_none(), "no L11");
    assert!(Level::new(14).is_none(), "no L14");
    // Labels stop at L10.
    let l10 = level_label(Level::new(10).unwrap());
    assert!(!l10.is_empty(), "L10 has a label");
    // Journey reaches L10 and never opens below.
    let mut universe = Universe::new(DEMO_SEED);
    let mut steps = 0;
    while universe.level().get() < 10 && steps < 20 {
        let marker = universe.autopilot_target().expect("journey marker");
        assert!(universe.open(MarkerIndex(marker)), "journey opens");
        steps += 1;
    }
    assert_eq!(universe.level().get(), 10, "journey ends at L10");
    assert!(!universe.path.can_open(), "L10 never opens");
    // No portal at L10 opens below (portals removed).
    let portals: Vec<_> = universe
        .open
        .points
        .iter()
        .filter(|pt| pt.kind == universe_core::r#gen::MarkerKind::Portal)
        .collect();
    assert!(portals.is_empty(), "no L11 portals");
    // Snapshot depth: chain never exceeds 9 opens (L10).
    assert!(
        universe.path.indices().len() <= 9,
        "snapshot depth stops at L10"
    );
}
