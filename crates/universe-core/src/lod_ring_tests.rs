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

#[test]
fn player_patch_builds_first() {
    // #919 T2 AC4: the wanted leaf covering the player leads the build
    // list, so his patch builds first even when the camera is elsewhere.
    use crate::lod::{PatchKey, patch_centre_dir};
    use crate::lod_range::LevelRanges;
    use crate::relief::Relief;
    use crate::settings::PlanetDetail;
    let journey = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
    let chain: &[u32] = journey.get(..9).unwrap_or(&[]);
    let relief = Relief::for_planet(crate::nav::DEMO_SEED, chain);
    let quad_px = PlanetDetail::Normal.quad_px();
    let camera = PlanetPos([0.0, EARTH_RADIUS_M + 1_000.0, 0.0]);
    let mut tree = QuadTree::new(EARTH_RADIUS_M)
        .expect("tree")
        .with_ranges(LevelRanges::new(&relief, EARTH_RADIUS_M, quad_px));
    tree.set_ring_m(RING_M);
    let mut scratch = crate::lod_select::UpdateScratch::default();
    tree.update_culled_reused(camera, quad_px, &|_| None, None, &mut scratch);
    let mut build = Vec::new();
    tree.append_wanted_to(&mut build);
    assert!(build.len() > 1, "selection worth ordering: {build:?}");
    // The player stands 2 m over the farthest wanted leaf: production
    // order is camera-first, so his leaf does not lead.
    let far = *build.last().expect("wanted");
    let player = PlanetPos(
        patch_centre_dir(far)
            .expect("centre")
            .map(|c| c * (EARTH_RADIUS_M + 2.0)),
    );
    assert_ne!(build[0], far, "camera order leaves the player leaf behind");
    let mut raw = build.clone();
    raw.sort();
    crate::lod_ring::promote_player_leaf(&mut build, player, EARTH_RADIUS_M);
    assert_eq!(build[0], far, "the player's patch leads");
    let mut done = build.clone();
    done.sort();
    assert_eq!(done, raw, "reorder only, nothing added or lost");
    // Empty and singleton lists are no-ops, never panics (`E-NO-PANIC`).
    let mut empty: Vec<PatchKey> = Vec::new();
    crate::lod_ring::promote_player_leaf(&mut empty, player, EARTH_RADIUS_M);
    assert!(empty.is_empty());
    let mut one = vec![far];
    crate::lod_ring::promote_player_leaf(&mut one, player, EARTH_RADIUS_M);
    assert_eq!(one, vec![far]);
}

#[test]
fn the_leaf_holding_the_feet_leads_over_a_coarser_tie() {
    // #981 R1: a coarse leaf's bounding sphere also reads zero distance
    // from the feet, and the key tie-break put it first; the leaf that
    // holds the feet leads instead.
    use crate::cube_sphere::Face;
    use crate::lod::{PatchKey, patch_centre_dir};
    let fine = PatchKey::new(Face::PosZ, 3, 7, 2).expect("key");
    let coarse = PatchKey::new(Face::PosY, 1, 1, 0).expect("key");
    let feet = PlanetPos(
        patch_centre_dir(fine)
            .expect("centre")
            .map(|c| c * (EARTH_RADIUS_M + 2.0)),
    );
    assert!(fine.holds(feet) && !coarse.holds(feet));
    assert!(!PatchKey::root(Face::NegZ).holds(feet));
    assert!(
        !fine.holds(PlanetPos([0.0; 3])),
        "the centre has no direction"
    );
    let mut build = vec![coarse, fine];
    crate::lod_ring::promote_player_leaf(&mut build, feet, EARTH_RADIUS_M);
    assert_eq!(build[0], fine);
    assert_eq!(
        crate::lod_ring::leaf_under(&[coarse, fine], feet),
        Some(fine)
    );
    assert_eq!(crate::lod_ring::leaf_under(&[coarse], feet), None);
}
