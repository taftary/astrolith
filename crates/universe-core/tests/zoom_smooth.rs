//! Fast wheel zoom stays smooth both ways (#915 T1).
//!
//! A fast wheel burst in one frame must cross at most one level boundary
//! and land near it, not jump far away. Slow single-notch steps stay
//! stepwise. Pure core, no window (`E-CORE-NO-BEVY`, `E-TEST-LADDER`).

use universe_core::coords::Level;
use universe_core::nav::{DEMO_SEED, DiveEvent, DiveMode, Universe, WHEEL_FACTOR};
use universe_core::nest::{CLOSE_ANGLE, OPEN_ANGLE, child_ratio};

fn gap(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

#[test]
fn fast_wheel_zoom_out_stays_smooth() {
    // C1 (AC1): a fast outward burst closes once and lands near the parent
    // boundary instead of jumping far away.
    let mut universe = Universe::new(DEMO_SEED);
    let marker = universe.autopilot_target().expect("root has markers");
    let mut event = DiveEvent::Moved;
    for _ in 0..500 {
        event = universe.dive(Some(marker), WHEEL_FACTOR, DiveMode::Targeted);
        if event != DiveEvent::Moved {
            break;
        }
    }
    assert_eq!(event, DiveEvent::Opened(marker));
    // An eight-notch outward burst in one frame.
    let factor = (1.0 / WHEEL_FACTOR).powi(8);
    let event = universe.dive(None, factor, DiveMode::Targeted);
    match event {
        DiveEvent::Closed(opened) => {
            assert!(!opened.anonymous, "named close");
            let offset = universe.path.offset();
            let landed = gap(offset, opened.position);
            let ratio = child_ratio(Level::MIN).expect("L1 ratio");
            let close_d = 0.5 / CLOSE_ANGLE.sin();
            let expected = close_d * ratio;
            assert!(
                landed < expected * 1.5,
                "far jump out: gap {landed} expected near {expected}"
            );
        }
        other => panic!("expected one close, got {other:?}"),
    }
}

#[test]
fn fast_wheel_zoom_in_stays_smooth() {
    // C2 (AC2): a fast inward burst opens once and lands near the child
    // entry instead of jumping deep inside.
    let mut universe = Universe::new(DEMO_SEED);
    let marker = universe.autopilot_target().expect("root has markers");
    let mut event = DiveEvent::Moved;
    for _ in 0..500 {
        event = universe.dive(Some(marker), WHEEL_FACTOR, DiveMode::Targeted);
        if event != DiveEvent::Moved {
            break;
        }
    }
    assert_eq!(event, DiveEvent::Opened(marker));
    // Back out slowly to the parent, just past the close boundary.
    let mut closed = false;
    for _ in 0..500 {
        match universe.dive(None, 1.0 / WHEEL_FACTOR, DiveMode::Targeted) {
            DiveEvent::Closed(opened) if !opened.anonymous => {
                closed = true;
                break;
            }
            DiveEvent::Moved => {}
            _ => {}
        }
    }
    assert!(closed, "slow steps close back to the parent");
    // An eight-notch inward burst in one frame.
    let factor = WHEEL_FACTOR.powi(8);
    let event = universe.dive(Some(marker), factor, DiveMode::Targeted);
    match event {
        DiveEvent::Opened(opened) => {
            assert_eq!(opened, marker, "opens the target");
            let distance = universe.path.distance_to_center();
            let entry = 0.5 / OPEN_ANGLE.sin();
            assert!(
                distance > entry * 0.5 && distance < entry * 1.5,
                "far jump in: distance {distance} expected near entry {entry}"
            );
        }
        other => panic!("expected one open, got {other:?}"),
    }
}

#[test]
fn slow_wheel_steps_stay_stepwise() {
    // C3 (AC3): single-notch steps stay stepwise both ways.
    let mut universe = Universe::new(DEMO_SEED);
    let marker = universe.autopilot_target().expect("root has markers");
    let mut event = DiveEvent::Moved;
    for _ in 0..500 {
        event = universe.dive(Some(marker), WHEEL_FACTOR, DiveMode::Targeted);
        if event != DiveEvent::Moved {
            break;
        }
    }
    assert_eq!(event, DiveEvent::Opened(marker));
    let rest = universe.path.distance_to_center();
    // One notch out stays inside.
    assert_eq!(
        universe.dive(None, 1.0 / WHEEL_FACTOR, DiveMode::Targeted),
        DiveEvent::Moved,
        "one notch out stays inside"
    );
    let out = universe.path.distance_to_center();
    assert!(out > rest, "outward step moves out");
    // One notch back in stays inside.
    assert_eq!(
        universe.dive(None, WHEEL_FACTOR, DiveMode::Targeted),
        DiveEvent::Moved,
        "one notch back in stays inside"
    );
    let back = universe.path.distance_to_center();
    assert!(back < out, "inward step moves back in");
}
