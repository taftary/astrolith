use super::*;
use crate::r#gen::MarkerKind;
use crate::seed::Rng;

/// Test content: each slot holds a marker one time in two, jittered
/// inside its block.
struct Jitter;

impl BlockContent for Jitter {
    fn make(&self, grid: &BlockGrid, seed: u64, key: BlockKey, slot: u32) -> Option<Point> {
        let mut rng = Rng::new(hash_triple(block_seed(grid, seed, key), 1, u64::from(slot)));
        if rng.next_f64() < 0.5 {
            return None;
        }
        let [lx, ly, lz] = grid.corner(key);
        let side = grid.side();
        let at = [
            lx + rng.next_f64() * side,
            ly + rng.next_f64() * side,
            lz + rng.next_f64() * side,
        ];
        Some(Point::bare(at, 0.001, MarkerKind::Portal))
    }
}

const GRID: BlockGrid = BlockGrid {
    per_axis: 10,
    slots: 64,
    base: 5000,
};

#[test]
fn an_id_encodes_its_block_and_slot() {
    for (key, slot) in [
        (BlockKey([0, 0, 0]), 0),
        (BlockKey([9, 0, 3]), 17),
        (BlockKey([9, 9, 9]), 63),
    ] {
        let id = GRID.encode(key, slot).unwrap_or(0);
        assert!(id >= GRID.base);
        assert_eq!(GRID.decode(id), Some((key, slot)));
    }
    assert_eq!(GRID.decode(4999), None, "a fixture ID is not a block ID");
    assert_eq!(GRID.encode(BlockKey([10, 0, 0]), 0), None, "off the grid");
    assert_eq!(GRID.encode(BlockKey([0, 0, 0]), 64), None, "past the slots");
    let last = GRID.encode(BlockKey([9, 9, 9]), 63).unwrap_or(0);
    assert_eq!(GRID.decode(last + 1), None, "past the grid");
}

#[test]
fn a_marker_is_rebuilt_from_its_id_alone() {
    // Saved places: an off-screen marker comes back with the same bits.
    let key = BlockKey([4, 7, 1]);
    let held = block(&Jitter, &GRID, 42, key);
    assert!(!held.is_empty());
    for (id, point) in held {
        assert_eq!(marker(&Jitter, &GRID, 42, id), Some(point));
    }
}

#[test]
fn the_same_blocks_hold_the_same_markers_in_any_order() {
    // Visit order, route and Marker detail never change a block's content.
    let keys = resident(&GRID, [0.1, -0.2, 0.3], 0.15, &[]);
    let (points, ids) = build(&Jitter, &GRID, 42, &keys);
    let mut merged: Vec<(u32, Point)> = keys
        .iter()
        .rev()
        .flat_map(|&key| block(&Jitter, &GRID, 42, key))
        .collect();
    merged.sort_by_key(|(id, _)| *id);
    let mut forward: Vec<(u32, Point)> = ids.into_iter().zip(points).collect();
    forward.sort_by_key(|(id, _)| *id);
    assert_eq!(forward, merged);
    assert!(!forward.is_empty());
}

#[test]
fn another_seed_fills_blocks_differently() {
    let key = BlockKey([5, 5, 5]);
    assert_ne!(block(&Jitter, &GRID, 1, key), block(&Jitter, &GRID, 2, key));
}

#[test]
fn residency_covers_the_load_radius_and_is_sorted() {
    let camera = [0.0, 0.0, 0.0];
    let keys = resident(&GRID, camera, 0.12, &[]);
    assert!(keys.windows(2).all(|w| w[0] < w[1]), "sorted, no repeats");
    for z in 0..10 {
        for y in 0..10 {
            for x in 0..10 {
                let key = BlockKey([x, y, z]);
                let near = GRID.distance_to(key, camera) <= 0.12;
                assert_eq!(near, keys.contains(&key), "{key:?}");
            }
        }
    }
}

#[test]
fn a_still_camera_keeps_its_blocks() {
    // Spec v2 AC9: holding still changes nothing.
    let camera = [0.21, -0.33, 0.05];
    let first = resident(&GRID, camera, 0.15, &[]);
    let again = resident(&GRID, camera, 0.15, &first);
    assert_eq!(first, again);
}

#[test]
fn a_block_edge_does_not_churn() {
    // Hysteresis: a small step back and forth keeps the held blocks; only
    // a move past the drop radius sheds them.
    let load = 0.15;
    let at = [0.0, 0.0, 0.0];
    let held = resident(&GRID, at, load, &[]);
    let nudged = resident(&GRID, [0.02, 0.0, 0.0], load, &held);
    assert!(held.iter().all(|k| nudged.contains(k)), "nothing dropped");
    let back = resident(&GRID, at, load, &nudged);
    assert!(held.iter().all(|k| back.contains(k)));
    let far = resident(&GRID, [0.45, 0.45, 0.45], load, &back);
    assert!(!far.contains(&BlockKey([4, 4, 4])), "far away it drops");
}

#[test]
fn the_camera_outside_the_cell_still_finds_its_nearest_blocks() {
    let keys = resident(&GRID, [0.7, 0.0, 0.0], 0.25, &[]);
    assert!(!keys.is_empty());
    assert!(keys.iter().all(|k| k.0[0] == 9));
}
