use super::*;
use crate::nav::DEMO_SEED;

#[test]
fn a_straight_free_flight_passes_into_two_levels_and_out_on_its_line() {
    // #998 Spec v2 AC4 (Q4 A): from L3, fly straight at the journey's
    // marker, through it and into its inner marker, then on out the far
    // side: two opens and two closes, and the path in L3 units stays one
    // straight line.
    // Straight to L3 by opens (no magnification milestones on the chain).
    let mut universe = Universe::new(DEMO_SEED);
    for _ in 0..2 {
        let marker = universe.journey_target().expect("a marker on the way");
        assert!(universe.open(MarkerIndex(marker)));
    }
    assert_eq!(universe.level().get(), 3);
    let base = universe.path.chain().len();
    let marker = universe.journey_target().expect("an L3 marker");
    let centre = universe.marker(MarkerIndex(marker)).expect("its place");
    let radius = universe.marker_radius();
    // Aim through the marker's own journey marker one level down: open a
    // copy to find where that sits, in L3 units.
    let mut peek = universe.clone();
    peek.path
        .set_offset([centre[0] - 3.0 * radius, centre[1], centre[2]]);
    assert!(peek.open(MarkerIndex(marker)));
    let entered = peek.path.entered().expect("entered");
    let inner = peek.journey_target().expect("an L4 marker");
    let inner_at = entered.to_parent_point(peek.marker(MarkerIndex(inner)).expect("inner"));
    // A straight line from 30 radii before the inner marker through it.
    let dir = {
        let d = [
            inner_at[0] - centre[0] + radius * 0.5,
            inner_at[1] - centre[1],
            inner_at[2] - centre[2],
        ];
        let n = length3(d);
        [d[0] / n, d[1] / n, d[2] / n]
    };
    let start = [
        inner_at[0] - dir[0] * 30.0 * radius,
        inner_at[1] - dir[1] * 30.0 * radius,
        inner_at[2] - dir[2] * 30.0 * radius,
    ];
    universe.path.set_offset(start);
    let step = radius / 300.0;
    let (mut opens, mut closes, mut worst) = (0, 0, 0.0f64);
    let mut deepest = 3;
    for _ in 0..30_000 {
        // Move one step along the line in the open cell's units, through
        // every frame between L3 and here.
        let mut d = dir;
        let mut s = step;
        for opened in universe.path.chain().iter().skip(base) {
            d = opened.to_child_direction(d);
            s /= opened.ratio;
        }
        let here = universe.path.offset();
        universe
            .path
            .set_offset([here[0] + d[0] * s, here[1] + d[1] * s, here[2] + d[2] * s]);
        match universe.free_pass() {
            DiveEvent::Opened(_) => opens += 1,
            DiveEvent::Closed(_) => closes += 1,
            DiveEvent::Moved => {}
        }
        deepest = deepest.max(universe.level().get());
        // Back to L3 units: how far off the line.
        let mut p = universe.path.offset();
        for opened in universe.path.chain().iter().skip(base).rev() {
            p = opened.to_parent_point(p);
        }
        let rel = [p[0] - start[0], p[1] - start[1], p[2] - start[2]];
        let along = rel[0] * dir[0] + rel[1] * dir[1] + rel[2] * dir[2];
        let off = length3([
            rel[0] - dir[0] * along,
            rel[1] - dir[1] * along,
            rel[2] - dir[2] * along,
        ]);
        worst = worst.max(off / radius);
        if universe.level().get() == 3 && closes >= 2 {
            break;
        }
    }
    assert!(deepest >= 5, "went two levels down: L{deepest}");
    assert!(opens >= 2 && closes >= 2, "opens {opens}, closes {closes}");
    assert_eq!(universe.level().get(), 3);
    assert!(worst < 1e-9, "off the line by {worst} marker radii");
}
