use super::*;

#[test]
#[ignore = "study probe (#998): cargo test -p universe-core --lib scale_study_prints -- --ignored --nocapture"]
fn scale_study_prints() {
    let lines = lines();
    for line in &lines {
        println!("{line}");
    }
    assert!(
        lines.iter().all(|l| !l.contains("did not reach")),
        "{lines:?}"
    );
}

#[test]
fn the_journey_hands_over_exactly_through_l1_to_l7_and_back() {
    // #998 AC1, AC2: every frame change in and out is exact to well under
    // a millimetre and 1e-9 degrees, and the f64 spacing at the camera is a
    // tiny share of every step, at every level.
    let (inward, outward) = journey().expect("the journey reaches L7");
    let pairs: Vec<(u8, u8)> = inward.handovers.iter().map(|h| (h.from, h.to)).collect();
    assert_eq!(pairs, [(1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 7)]);
    let back: Vec<(u8, u8)> = outward.handovers.iter().map(|h| (h.from, h.to)).collect();
    assert_eq!(back, [(7, 6), (6, 5), (5, 4), (4, 3), (3, 2), (2, 1)]);
    for h in inward.handovers.iter().chain(&outward.handovers) {
        assert!(h.jump_m < 1e-3, "{h:?}");
        assert!(h.turn_deg < 1e-9, "{h:?}");
    }
    // Inward the speed carries across (the autopilot's distance-scaled
    // speed changes by at most about a third).
    for h in inward.handovers.iter().filter(|h| h.speed_ratio > 0.0) {
        assert!((0.5..2.0).contains(&h.speed_ratio), "{h:?}");
    }
    for p in inward.precision.iter().chain(&outward.precision) {
        assert!(p.share_of_step < 1e-9, "{p:?}");
    }
}

#[test]
fn a_free_pass_through_a_marker_stays_on_its_line() {
    // #998 AC1 (Q4 A): flying straight through an L4 marker opens and
    // closes it, and the path stays one straight line across both
    // handovers.
    let pass = free_pass().expect("the journey reaches L4");
    assert!(pass.crossed, "{pass:?}");
    assert!(pass.off_line_share < 1e-9, "{pass:?}");
}
