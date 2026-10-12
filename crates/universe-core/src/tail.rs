//! Sparse tail generators for L11-L14: regions, cities, buildings, room (#375, #458).
//!
//! The ladder ends the way it begins: deterministic content with no stored
//! state. [`TailGenerator`](crate::tail::TailGenerator) serves L11-L13 from
//! the one icosphere surface (ADR 0024): the cell's tiles are the entered
//! cluster split to the level's depth inside the cell window, grown into
//! clusters; enterable clusters become portals at their anchor tiles (one
//! building box standing on the slope per enterable L13 plot, Q11) and
//! closed clusters plain population dots. Rivers, roads, lakes, decorative
//! boxes, and the bowl scatter are gone (Q10).
//! [`RoomGenerator`](crate::tail::RoomGenerator) serves L14 (populations
//! only, so nothing opens past the room) with a room shell box around the
//! furniture. Both take their counts and child budgets from
//! [`UniformGenerator`](crate::gen::UniformGenerator); the tail places its
//! points straight from the clusters, with no throwaway scatter (#543).

use crate::coords::Level;
use crate::r#gen::{
    Constraints, Form, Generated, Generator, MarkerKind, Point, UniformGenerator, halved_child,
};
use crate::seed::{Rng, hash_triple};
use crate::surface::SurfaceFrame;

/// Decorrelation lane for tail per-point draws (box heights).
const TAIL_STREAM_TAG: u64 = 0x7A11_6E15_709A_11E5;

/// Surface generator for one tail cell at L11, L12, or L13 (#458).
///
/// Counts and child budgets come from the uniform scatter for
/// `base_count`; every point is then a cluster anchor of the cell's
/// [`SurfaceFrame`]: enterable clusters first (portals, so a marker index
/// is its portal ordinal), closed clusters after (population dots). L13
/// portals wear one building box standing on the slope. Without a frame
/// (a chain that never crossed an L10 portal) the cell stays empty.
#[derive(Clone, Debug, PartialEq)]
pub struct TailGenerator {
    /// Level this generator is fixed to (drives forms).
    level: Level,
    /// Baseline point count at full density (8 for L11-L13).
    base_count: u32,
    /// The open surface cell, when the chain reached it through portals.
    frame: Option<SurfaceFrame>,
}

impl TailGenerator {
    /// Creates a tail generator for `level` over the surface cell `frame`.
    #[must_use]
    pub const fn new(level: Level, base_count: u32, frame: Option<SurfaceFrame>) -> TailGenerator {
        TailGenerator {
            level,
            base_count,
            frame,
        }
    }

    /// Box height for an L13 portal from its lane.
    fn portal_height(lane: &mut Rng) -> f64 {
        0.02 + 0.06 * lane.next_f64()
    }
}

impl Generator for TailGenerator {
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        let empty = Generated {
            points: Vec::new(),
            child_constraints: Vec::new(),
        };
        let Some(frame) = &self.frame else {
            return empty;
        };
        if !parent.is_valid() {
            return empty;
        }
        let clusters = frame.clusters();
        let ordered = clusters
            .iter()
            .filter(|cluster| cluster.enterable)
            .chain(clusters.iter().filter(|cluster| !cluster.enterable));
        let wanted = UniformGenerator::new(self.base_count).count_for(parent) as usize;
        let mut points = Vec::with_capacity(wanted);
        for cluster in ordered.take(wanted) {
            // Anchors are member tiles of a grown cluster, so they always
            // have a centre; a malformed one is skipped, never invented.
            let Some(centre) = crate::icosphere::tri_centre(cluster.anchor) else {
                continue;
            };
            // The cell window (radius 0.45) sits inside the 0.5 shell, so
            // anchors land inside the cell with no clamp (surface.rs).
            let position = frame.surface_cell(centre);
            let index = points.len();
            let point = if cluster.enterable {
                let form = if self.level.get() == 13 {
                    let mut lane = Rng::new(hash_triple(seed, TAIL_STREAM_TAG, index as u64));
                    Form::Box {
                        height: Self::portal_height(&mut lane),
                    }
                } else {
                    Form::Dot
                };
                Point {
                    portal: Some([0.0; 3]),
                    form,
                    ..Point::bare(position, 0.01, MarkerKind::Portal)
                }
            } else {
                Point::bare(position, 0.01, MarkerKind::Population)
            };
            points.push(point);
        }
        Generated {
            child_constraints: vec![halved_child(parent); points.len()],
            points,
        }
    }
}

/// Furniture generator for the L14 room: populations only.
///
/// The room never opens, so every point is a population the renderer draws
/// as a furniture outline (#380). Counts and budgets follow the uniform
/// scatter for `base_count` (12 for L14): eleven furniture pieces plus the
/// room shell box appended last, so the total never moves.
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
        // The room shell around the furniture: a box centred on the cell so
        // its faces sit at ±0.4 by ±0.15. One furniture piece trims from the
        // tail to keep the total; the room never opens, so no journey moves.
        if !out.points.is_empty() {
            let total = out.points.len();
            out.points.truncate(total.saturating_sub(1));
            out.points.push(Point {
                position: [0.0, -0.15, 0.0],
                radius: 0.4,
                kind: MarkerKind::Population,
                form: Form::Box { height: 0.3 },
                ..Point::bare([0.0; 3], 0.4, MarkerKind::Population)
            });
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

    fn tail_level(n: u8) -> Level {
        Level::new(n).expect("tail rung")
    }

    /// The demo journey's surface frame at tail level `n`.
    fn frame(n: u8) -> SurfaceFrame {
        let chain = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
        let named = chain
            .get(..usize::from(n) - 1)
            .expect("journey reaches the tail");
        SurfaceFrame::for_path(crate::nav::DEMO_SEED, named).expect("tail frame")
    }

    fn tail(n: u8) -> TailGenerator {
        TailGenerator::new(tail_level(n), 8, Some(frame(n)))
    }

    #[test]
    fn tail_splits_half_portals_half_populations() {
        for n in [11u8, 12, 13] {
            let out = tail(n).generate(42, &tail_budget());
            assert_eq!(out.points.len(), 8, "L{n} keeps today's count");
            let kinds: Vec<MarkerKind> = out.points.iter().map(|point| point.kind).collect();
            assert!(
                kinds.iter().take(4).all(|kind| *kind == MarkerKind::Portal),
                "L{n} portals come first"
            );
            assert!(
                kinds
                    .iter()
                    .skip(4)
                    .all(|kind| *kind == MarkerKind::Population),
                "L{n} closed clusters are populations"
            );
            for point in &out.points {
                assert!(point.radius > 0.0);
                let length = point.position.iter().map(|c| c * c).sum::<f64>().sqrt();
                assert!(length < 0.5, "L{n} point outside the cell shell");
            }
        }
    }

    #[test]
    fn tail_points_are_cluster_anchors_on_the_surface() {
        for n in [11u8, 12, 13] {
            let surface = frame(n);
            let out = tail(n).generate(42, &tail_budget());
            let clusters = surface.clusters();
            for point in &out.points {
                let anchored = clusters.iter().any(|cluster| {
                    crate::icosphere::tri_centre(cluster.anchor)
                        .map(|centre| surface.surface_cell(centre))
                        == Some(point.position)
                });
                assert!(anchored, "L{n} point is not a cluster anchor");
                let ground = surface.ground_height(point.position[0], point.position[2]);
                assert!(
                    (point.position[1] - ground).abs() < 1e-6,
                    "L{n} point floats off the surface"
                );
            }
            assert_eq!(
                surface.portal_positions(),
                out.points
                    .iter()
                    .filter(|point| point.kind == MarkerKind::Portal)
                    .map(|point| point.position)
                    .collect::<Vec<_>>(),
                "L{n} frame and generator agree on portals"
            );
        }
    }

    #[test]
    fn tail_levels_wear_no_rects_or_patches() {
        for n in [11u8, 12, 13] {
            let out = tail(n).generate(42, &tail_budget());
            for point in &out.points {
                match point.form {
                    Form::Dot => {}
                    Form::Box { height } => {
                        assert_eq!(n, 13, "L{n} builds no boxes");
                        assert_eq!(point.kind, MarkerKind::Portal, "boxes open the room");
                        assert!(height > 0.0, "building box has no height");
                        assert!(point.portal.is_some(), "plot carries no room offset");
                    }
                    other => panic!("L{n} wears a separate piece: {other:?}"),
                }
            }
            let boxes = out
                .points
                .iter()
                .filter(|point| matches!(point.form, Form::Box { .. }))
                .count();
            if n == 13 {
                assert_eq!(boxes, 4, "one building box per enterable plot (Q11)");
            }
        }
    }

    #[test]
    fn tail_always_offers_a_portal() {
        let tight = Constraints::new(0.0, 4, 4, [0.5, 0.5, 0.5]).expect("valid");
        let out = tail(12).generate(7, &tight);
        assert_eq!(out.points.len(), 4);
        assert!(
            out.points
                .iter()
                .any(|point| point.kind == MarkerKind::Portal),
            "even the tightest cell opens onward"
        );
    }

    #[test]
    fn tail_without_a_surface_stays_empty() {
        let out = TailGenerator::new(tail_level(11), 8, None).generate(7, &tail_budget());
        assert!(out.points.is_empty());
        assert!(out.child_constraints.is_empty());
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
    fn room_shell_closes_the_cell() {
        let out = RoomGenerator::new(12).generate(42, &room_budget());
        let shell = out.points.last().expect("the shell appends last");
        assert_eq!(
            shell.form,
            Form::Box { height: 0.3 },
            "the room shell closes the cell"
        );
        assert_eq!(out.points.len(), 12, "the shell trims one piece");
    }

    #[test]
    fn tail_and_room_are_deterministic() {
        let region = tail(11);
        assert_eq!(
            snapshot_generated(&region.generate(99, &tail_budget())),
            snapshot_generated(&region.generate(99, &tail_budget()))
        );
        let room = RoomGenerator::new(12);
        assert_eq!(
            snapshot_generated(&room.generate(99, &room_budget())),
            snapshot_generated(&room.generate(99, &room_budget()))
        );
        assert_ne!(
            snapshot_generated(&region.generate(1, &tail_budget())),
            snapshot_generated(&tail(12).generate(1, &tail_budget()))
        );
    }

    #[test]
    fn tail_and_room_children_respect_parent() {
        let city = tail(12).generate(7, &tail_budget());
        for child in &city.child_constraints {
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
        assert!(tail(13).generate(3, &bad).points.is_empty());
        assert!(RoomGenerator::new(12).generate(3, &bad).points.is_empty());
    }
}
