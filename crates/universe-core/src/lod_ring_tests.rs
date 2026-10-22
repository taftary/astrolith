//! Ground ring and build order tests (#918 T2).
//!
//! Pure and hermetic (`E-TEST-HERMETIC`).

use crate::headless_ring::{RingMetrics, ring_metrics};
use crate::lod_ring::{RING_M, RING_MIN_M, next_ring_m};
use crate::lod_select::QuadTree;
use crate::planet_frame::{EARTH_RADIUS_M, PlanetPos};
use std::sync::OnceLock;

/// One arrival and turn, shared by the tests below.
fn metrics() -> &'static RingMetrics {
    static METRICS: OnceLock<RingMetrics> = OnceLock::new();
    METRICS.get_or_init(ring_metrics)
}

#[test]
fn builds_nearest_first() {
    // C2 (#918 AC2, Q3 A): on arrival the wanted leaves are in distance
    // order, so the leaf under the player is built first.
    let m = metrics();
    assert!(m.settled, "the arrival settles: {m:?}");
    assert!(m.nearest_first, "nearest first: {m:?}");
}

#[test]
fn turning_builds_nothing_in_the_ring() {
    // C4 (#918 AC4, Q2 A, Q5 A): a full turn needs no build inside the
    // ring, and the ring holds ground in every direction.
    let m = metrics();
    assert!(m.ring_patches > 0, "the ring holds ground: {m:?}");
    assert_eq!(m.turn_builds, 0, "{m:?}");
}

#[test]
fn no_pop_in_after_settle() {
    // C11 (#918): after the ground settles, turning neither splits nor
    // merges a leaf near the player, so nothing pops in there.
    let m = metrics();
    assert_eq!(m.ring_changes, 0, "{m:?}");
}

#[test]
fn ring_shrinks_to_fit_the_cap() {
    // C10 (#918 P1 B): over the cap the ring shrinks step by step and turns
    // off below its minimum; with room it grows back to 500 m, never more.
    let cap = 64 << 20;
    let mut ring = RING_M;
    let mut steps = 0;
    while ring > 0.0 {
        let next = next_ring_m(ring, RING_M, cap + 1, cap);
        assert!(next < ring, "shrinks over the cap");
        ring = next;
        steps += 1;
        assert!(steps < 50, "reaches zero");
    }
    assert_eq!(
        next_ring_m(300.0, RING_M, cap, cap),
        300.0,
        "holds at the cap"
    );
    let mut ring = 0.0;
    for _ in 0..50 {
        ring = next_ring_m(ring, RING_M, cap / 2, cap);
    }
    assert_eq!(ring, RING_M, "grows back to the target");
    assert!(next_ring_m(0.0, RING_M, cap / 2, cap) >= RING_MIN_M);
}

#[test]
fn no_ring_keeps_the_old_selection() {
    // #918 T2: with the ring off, the wanted leaves are every leaf without
    // culling, as before.
    let mut tree = QuadTree::new(EARTH_RADIUS_M).expect("tree");
    let camera = PlanetPos([0.0, EARTH_RADIUS_M + 1_000.0, 0.0]);
    let mut scratch = crate::lod_select::UpdateScratch::default();
    tree.update_culled_reused(camera, 8.0, &|_| None, None, &mut scratch);
    let mut wanted = Vec::new();
    tree.append_wanted_to(&mut wanted);
    wanted.sort();
    assert_eq!(wanted, tree.leaves());
    assert_eq!(tree.ring_m(), 0.0);
}
