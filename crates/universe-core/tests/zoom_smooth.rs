//! Fast wheel zoom stays smooth both ways (#915 R1).
//!
//! Maximum dive speed: one call moves at most one wheel notch, so a fast
//! scroll crosses one boundary per frame and lands near it instead of
//! jumping far away. Slow single-notch steps stay stepwise. Pure core, no
//! window (`E-CORE-NO-BEVY`, `E-TEST-LADDER`).

use universe_core::coords::Level;
use universe_core::nav::{DEMO_SEED, DiveEvent, DiveMode, Universe, WHEEL_FACTOR};
use universe_core::nest::{CLOSE_ANGLE, OPEN_ANGLE, child_ratio};

fn gap(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

fn open_child(marker: u32) -> Universe {
    let mut universe = Universe::new(DEMO_SEED);
    let mut event = DiveEvent::Moved;
    for _ in 0..500 {
        event = universe.dive(Some(marker), WHEEL_FACTOR, DiveMode::Targeted);
        if event != DiveEvent::Moved {
            break;
        }
    }
    assert_eq!(event, DiveEvent::Opened(marker));
    universe
}

#[test]
fn fast_wheel_zoom_out_stays_smooth() {
    // C1 (AC1): fast outward scroll is capped to one notch per frame and the
    // close lands near the parent boundary instead of jumping far away.
    let mut universe = {
        let marker = Universe::new(DEMO_SEED)
            .autopilot_target()
            .expect("root has markers");
        open_child(marker)
    };
    let rest = universe.path.distance_to_center();
    // A large burst in one call moves only one notch (maximum dive speed).
    let burst = (1.0 / WHEEL_FACTOR).powi(8);
    assert_eq!(
        universe.dive(None, burst, DiveMode::Targeted),
        DiveEvent::Moved,
        "capped burst stays inside on the first frame"
    );
    let first = universe.path.distance_to_center();
    let single = rest * (1.0 / WHEEL_FACTOR);
    assert!(
        (first - single).abs() < 1e-9,
        "burst moves one notch: {first} expected {single}"
    );
    // Keep scrolling fast until the level closes; the landing sits near the
    // boundary (single-notch overshoot eases to it).
    let mut closed = None;
    for _ in 0..20 {
        match universe.dive(None, burst, DiveMode::Targeted) {
            DiveEvent::Closed(opened) if !opened.anonymous => {
                closed = Some(opened);
                break;
            }
            DiveEvent::Moved => {}
            other => panic!("expected move or close, got {other:?}"),
        }
    }
    let opened = closed.expect("fast scroll closes within 20 frames");
    let landed = gap(universe.path.offset(), opened.position);
    let ratio = child_ratio(Level::MIN).expect("L1 ratio");
    let expected = (0.5 / CLOSE_ANGLE.sin()) * ratio;
    assert!(
        landed < expected * 1.1,
        "far jump out: gap {landed} expected near {expected}"
    );
}

#[test]
fn fast_wheel_zoom_in_stays_smooth() {
    // C2 (AC2): fast inward scroll is capped the same way and the open lands
    // near the child entry instead of jumping deep inside.
    let marker = Universe::new(DEMO_SEED)
        .autopilot_target()
        .expect("root has markers");
    let mut universe = open_child(marker);
    // Back out to the parent with capped fast scroll; the landing sits near
    // the parent boundary.
    let burst_out = (1.0 / WHEEL_FACTOR).powi(8);
    let mut closed = false;
    for _ in 0..20 {
        match universe.dive(None, burst_out, DiveMode::Targeted) {
            DiveEvent::Closed(opened) if !opened.anonymous => {
                closed = true;
                break;
            }
            DiveEvent::Moved => {}
            other => panic!("expected move or close, got {other:?}"),
        }
    }
    assert!(closed, "fast scroll closes back to the parent");
    // Fast inward scroll until the level opens.
    let burst_in = WHEEL_FACTOR.powi(8);
    let mut opened_marker = None;
    for _ in 0..20 {
        match universe.dive(Some(marker), burst_in, DiveMode::Targeted) {
            DiveEvent::Opened(opened) => {
                opened_marker = Some(opened);
                break;
            }
            DiveEvent::Moved => {}
            other => panic!("expected move or open, got {other:?}"),
        }
    }
    assert_eq!(
        opened_marker,
        Some(marker),
        "fast scroll reopens the target"
    );
    let distance = universe.path.distance_to_center();
    let entry = 0.5 / OPEN_ANGLE.sin();
    assert!(
        distance > entry * 0.9 && distance < entry * 1.1,
        "far jump in: distance {distance} expected near entry {entry}"
    );
}

#[test]
fn slow_wheel_steps_stay_stepwise() {
    // C3 (AC3): single-notch steps stay stepwise both ways.
    let marker = Universe::new(DEMO_SEED)
        .autopilot_target()
        .expect("root has markers");
    let mut universe = open_child(marker);
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
