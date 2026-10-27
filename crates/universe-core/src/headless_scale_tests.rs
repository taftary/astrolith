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
    // The speed carries across both ways (#998 Spec v2 AC1: within a
    // third; the study measured up to 14 times going out before B1).
    for h in inward
        .handovers
        .iter()
        .chain(&outward.handovers)
        .filter(|h| h.speed_ratio > 0.0)
    {
        assert!((2.0 / 3.0..4.0 / 3.0).contains(&h.speed_ratio), "{h:?}");
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

#[test]
fn l7_hands_over_into_the_star_system_exactly_and_back() {
    // #998 Spec v2 AC5 (B4): from the Oort shells (L7) into the star-system
    // space (L8) the frame change is exact and the speed carries, in and
    // out; from L8 down to the ground is #995's one space.
    let (inward, outward) = journey_to(8).expect("the journey reaches L8");
    let into = inward
        .handovers
        .iter()
        .find(|h| h.from == 7)
        .expect("L7 to L8");
    assert_eq!(into.to, 8);
    let back = outward
        .handovers
        .iter()
        .find(|h| h.from == 8)
        .expect("L8 to L7");
    for h in [into, back] {
        assert!(h.jump_m < 1e-3 && h.turn_deg < 1e-9, "{h:?}");
    }
    assert!(
        (2.0 / 3.0..4.0 / 3.0).contains(&back.speed_ratio),
        "{back:?}"
    );
}
