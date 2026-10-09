//! Sparse tail generators for L11-L14: regions, cities, buildings, room (#375).
//!
//! The ladder ends the way it begins: deterministic scatters with no stored
//! state. [`TailGenerator`](crate::tail::TailGenerator) serves L11-L13 (half
//! portals that open the next rung, half populations that decorate it);
//! [`RoomGenerator`](crate::tail::RoomGenerator) serves L14 (populations
//! only, so nothing opens past the room). Both reuse
//! [`UniformGenerator`](crate::gen::UniformGenerator) for positions, counts,
//! and child budgets, then set the portal/population split.

use crate::r#gen::{Constraints, Generated, Generator, MarkerKind, UniformGenerator};

/// Scatter generator for one tail cell at L11, L12, or L13.
///
/// Emits the uniform scatter for `base_count`, then marks the first half of
/// the points portals (they open the next rung) and the rest populations.
/// Emission order is the uniform stream order, so portals and populations
/// interleave spatially with no layout promise beyond the count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TailGenerator {
    /// Baseline point count at full density (8 for L11-L13).
    base_count: u32,
}

impl TailGenerator {
    /// Creates a tail generator emitting up to `base_count` points.
    #[must_use]
    pub const fn new(base_count: u32) -> TailGenerator {
        TailGenerator { base_count }
    }
}

impl Generator for TailGenerator {
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        let mut out = UniformGenerator::new(self.base_count).generate(seed, parent);
        let portals = out.points.len() / 2;
        for (index, point) in out.points.iter_mut().enumerate() {
            if index >= portals {
                point.kind = MarkerKind::Population;
            }
        }
        out
    }
}

/// Furniture generator for the L14 room: populations only.
///
/// The room never opens, so every point is a population the renderer draws
/// as a furniture outline (#380). Counts and budgets follow the uniform
/// scatter for `base_count` (12 for L14).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomGenerator {
    /// Baseline point count at full density (12 for L14).
    base_count: u32,
}

impl RoomGenerator {
    /// Creates a room generator emitting up to `base_count` points.
    #[must_use]
    pub const fn new(base_count: u32) -> RoomGenerator {
        RoomGenerator { base_count }
    }
}

impl Generator for RoomGenerator {
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        let mut out = UniformGenerator::new(self.base_count).generate(seed, parent);
        for point in &mut out.points {
            point.kind = MarkerKind::Population;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r#gen::respects;
    use crate::snapshot::snapshot_generated;

    /// Builds the M6 tail budget (mirrors `level_budget` for L11-L13).
    fn tail_budget() -> Constraints {
        Constraints::new(1.0, 4, 8, [0.5, 0.5, 0.5]).expect("valid test budget")
    }

    /// Builds the room budget (mirrors `level_budget` for L14).
    fn room_budget() -> Constraints {
        Constraints::new(1.0, 4, 12, [0.5, 0.5, 0.5]).expect("valid test budget")
    }

    #[test]
    fn tail_splits_half_portals_half_populations() {
        let out = TailGenerator::new(8).generate(42, &tail_budget());
        assert_eq!(out.points.len(), 8);
        let portals = out
            .points
            .iter()
            .filter(|point| point.kind == MarkerKind::Portal)
            .count();
        assert_eq!(portals, 4, "half the scatter opens the next rung");
        for point in &out.points {
            assert!(point.radius > 0.0);
            for (axis, extent) in [0.5; 3].iter().enumerate() {
                assert!(
                    point.position[axis].abs() <= *extent,
                    "point outside allowed extent"
                );
            }
        }
    }

    #[test]
    fn tail_always_offers_a_portal() {
        let tight = Constraints::new(0.0, 4, 4, [0.5, 0.5, 0.5]).expect("valid");
        let out = TailGenerator::new(8).generate(7, &tight);
        assert_eq!(out.points.len(), 4);
        assert!(
            out.points
                .iter()
                .any(|point| point.kind == MarkerKind::Portal),
            "even the tightest cell opens onward"
        );
    }

    #[test]
    fn room_never_opens() {
        for seed in [1u64, 7, 42, 999] {
            let out = RoomGenerator::new(12).generate(seed, &room_budget());
            assert_eq!(out.points.len(), 12);
            for point in &out.points {
                assert_eq!(
                    point.kind,
                    MarkerKind::Population,
                    "L14 is the end of the ladder"
                );
            }
        }
    }

    #[test]
    fn tail_and_room_are_deterministic() {
        let tail = TailGenerator::new(8);
        assert_eq!(
            snapshot_generated(&tail.generate(99, &tail_budget())),
            snapshot_generated(&tail.generate(99, &tail_budget()))
        );
        let room = RoomGenerator::new(12);
        assert_eq!(
            snapshot_generated(&room.generate(99, &room_budget())),
            snapshot_generated(&room.generate(99, &room_budget()))
        );
        assert_ne!(
            snapshot_generated(&tail.generate(1, &tail_budget())),
            snapshot_generated(&tail.generate(2, &tail_budget()))
        );
    }

    #[test]
    fn tail_and_room_children_respect_parent() {
        let tail = TailGenerator::new(8).generate(7, &tail_budget());
        for child in &tail.child_constraints {
            assert!(respects(child, &tail_budget()));
        }
        let room = RoomGenerator::new(12).generate(7, &room_budget());
        for child in &room.child_constraints {
            assert!(respects(child, &room_budget()));
        }
    }

    #[test]
    fn invalid_parent_yields_empty_tail_and_room() {
        let bad = Constraints {
            density_multiplier: 2.0,
            min_count: 9,
            max_count: 3,
            allowed_extent: [0.5, 0.5, 0.5],
        };
        assert!(!bad.is_valid());
        assert!(TailGenerator::new(8).generate(3, &bad).points.is_empty());
        assert!(RoomGenerator::new(12).generate(3, &bad).points.is_empty());
    }
}
