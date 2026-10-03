//! Property tests for `universe-core` (M2 for #85).
//!
//! Each test name states its property as a sentence. `proptest` is a
//! dev-dependency only, so it does not count against "std only"
//! (`E-PROTEST`). Regressions, if proptest ever writes them, are committed
//! under `proptest-regressions/`.

use proptest::prelude::*;
use universe_core::nest::{MarkerPath, generate_cell};
use universe_core::snapshot::snapshot_generated;
use universe_core::verify::check_border;

proptest! {
    /// Regenerating a cell twice from the same seed and chain gives
    /// identical snapshots.
    #[test]
    fn regenerating_gives_identical_snapshots(
        root in any::<u64>(),
        chain in prop::collection::vec(0u32..64, 0..4),
    ) {
        let first = snapshot_generated(&generate_cell(root, &chain));
        let second = snapshot_generated(&generate_cell(root, &chain));
        prop_assert_eq!(first, second);
    }

    /// Opening a marker and closing it returns the camera offset within
    /// 1e-12 (relative, floor 1.0).
    #[test]
    fn open_then_close_returns_the_offset_within_1e12(
        offset in (-10.0f64..10.0, -10.0f64..10.0, -10.0f64..10.0),
        marker in 0u32..128,
        position in (-0.5f64..0.5, -0.5f64..0.5, -0.5f64..0.5),
    ) {
        let mut path = MarkerPath::root([offset.0, offset.1, offset.2]);
        let before = path.offset();
        prop_assume!(path.open(marker, [position.0, position.1, position.2]));
        path.close();
        let after = path.offset();
        for (a, b) in before.iter().zip(after.iter()) {
            prop_assert!(
                (a - b).abs() <= 1e-12 * b.abs().max(1.0),
                "offset drifted: {before:?} vs {after:?}",
            );
        }
    }

    /// `snapshot_generated` ignores point order: reversing and rotating the
    /// points changes nothing.
    #[test]
    fn snapshot_generated_ignores_point_order(
        root in any::<u64>(),
        chain in prop::collection::vec(0u32..64, 0..4),
        rotate in 0usize..32,
    ) {
        let cell = generate_cell(root, &chain);
        let mut permuted = cell.points.clone();
        permuted.reverse();
        if !permuted.is_empty() {
            let shift = rotate % permuted.len();
            permuted.rotate_left(shift);
        }
        let mut shuffled = cell.clone();
        shuffled.points = permuted;
        prop_assert_eq!(
            snapshot_generated(&cell),
            snapshot_generated(&shuffled)
        );
    }

    /// `check_border(a, b)` equals `check_border(b, a)` for any sample
    /// sets and any positive epsilon.
    #[test]
    fn check_border_reads_the_same_both_ways(
        a_points in prop::collection::vec(
            (-1.0f64..1.0, -1.0f64..1.0, -1.0f64..1.0),
            0..8,
        ),
        b_points in prop::collection::vec(
            (-1.0f64..1.0, -1.0f64..1.0, -1.0f64..1.0),
            0..8,
        ),
        epsilon in 1e-12f64..1e-3,
    ) {
        let a: Vec<[f64; 3]> = a_points
            .into_iter()
            .map(|(x, y, z)| [x, y, z])
            .collect();
        let b: Vec<[f64; 3]> = b_points
            .into_iter()
            .map(|(x, y, z)| [x, y, z])
            .collect();
        prop_assert_eq!(
            check_border(&a, &b, epsilon),
            check_border(&b, &a, epsilon)
        );
    }
}
