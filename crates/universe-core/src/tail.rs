//! Sparse tail generators for L11-L14: regions, cities, buildings, room (#375).
//!
//! The ladder ends the way it begins: deterministic scatters with no stored
//! state. [`TailGenerator`](crate::tail::TailGenerator) serves L11-L13 (half
//! portals that open the next rung, half populations that decorate it), each
//! level with its own forms: region and city outlines, building boxes,
//! river and road threads, lake rings, on a curved ground at L11 and flat
//! grounds below (#384). [`RoomGenerator`](crate::tail::RoomGenerator)
//! serves L14 (populations only, so nothing opens past the room) with a room
//! shell box around the furniture. Both reuse
//! [`UniformGenerator`](crate::gen::UniformGenerator) for positions, counts,
//! and child budgets, then set the portal/population split.

use crate::coords::Level;
use crate::r#gen::{
    Constraints, Form, Generated, Generator, MarkerKind, Point, UniformGenerator, clamp_to_sphere,
};
use crate::seed::{Rng, hash_triple};

/// Curvature of the L11 ground: the region wraps its planet (#384).
///
/// The grid sags by `d^2 / (2 * curvature)` at distance `d` from the
/// centre, so a cell-wide patch dips a visible but gentle arc.
pub const L11_CURVATURE: f64 = 4.0;

/// Decorrelation lane for tail per-point draws (rect halves, jitter).
const TAIL_STREAM_TAG: u64 = 0x7A11_6E15_709A_11E5;

/// Scatter generator for one tail cell at L11, L12, or L13.
///
/// Emits the uniform scatter for `base_count`, then marks the first half of
/// the points portals (they open the next rung) and the rest populations.
/// Portals wear their level's outline (regions and cities as rectangles,
/// buildings as boxes); populations alternate threads (rivers, roads,
/// streets linked to the next two populations) and rings (lakes). Points
/// settle onto the ground: the curved patch at L11, flat planes at L12 and
/// L13. L11 appends its curved-grid scenery behind the portal prefix and
/// trims populations to the same total. Emission order is the uniform
/// stream order, so portals and populations interleave spatially.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TailGenerator {
    /// Level this generator is fixed to (drives forms and grounds).
    level: Level,
    /// Baseline point count at full density (8 for L11-L13).
    base_count: u32,
}

impl TailGenerator {
    /// Creates a tail generator for `level` emitting up to `base_count` points.
    #[must_use]
    pub const fn new(level: Level, base_count: u32) -> TailGenerator {
        TailGenerator { level, base_count }
    }

    /// Rect half extents for a portal at this level, in cell units.
    fn portal_half(&self, lane: &mut Rng) -> [f64; 2] {
        let scale = if self.level.get() == 11 { 1.5 } else { 1.0 };
        [
            scale * (0.02 + 0.03 * lane.next_f64()),
            scale * (0.015 + 0.02 * lane.next_f64()),
        ]
    }

    /// Box height for an L13 portal from its lane.
    fn portal_height(lane: &mut Rng) -> f64 {
        0.02 + 0.06 * lane.next_f64()
    }
}

impl Generator for TailGenerator {
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        let mut out = UniformGenerator::new(self.base_count).generate(seed, parent);
        let portals = out.points.len() / 2;
        // Pass one: kinds, portal outlines, radii, and the ground. Points
        // settle onto the curved patch at L11 (sagging with the planet) or
        // flat planes below, each with a breath of jitter.
        for (index, point) in out.points.iter_mut().enumerate() {
            let mut lane = Rng::new(hash_triple(seed, TAIL_STREAM_TAG, index as u64));
            if index >= portals {
                point.kind = MarkerKind::Population;
            } else if self.level.get() == 13 {
                point.form = Form::Box {
                    height: Self::portal_height(&mut lane),
                };
            } else {
                point.form = Form::Rect {
                    half: self.portal_half(&mut lane),
                };
            }
            let jitter = (lane.next_f64() * 2.0 - 1.0) * 0.005;
            if self.level.get() == 11 {
                let sag = (point.position[0] * point.position[0]
                    + point.position[2] * point.position[2])
                    / (2.0 * L11_CURVATURE);
                point.position[1] = -sag + jitter;
            } else {
                point.position[1] = jitter;
            }
            point.position = clamp_to_sphere(point.position);
        }
        // Pass two: population forms read the settled scatter, so thread
        // endpoints land exactly on fellow populations. The immutable borrow
        // ends with the copies, so the assignment below never aliases.
        let population_order: Vec<usize> = (portals..out.points.len()).collect();
        for (rank, &index) in population_order.iter().enumerate() {
            let mut lane = Rng::new(hash_triple(seed, TAIL_STREAM_TAG, index as u64));
            let current = out.points.get(index).map_or(0.01, |point| point.radius);
            let (form, radius) = if self.level.get() == 13 {
                if rank % 2 == 0 {
                    let (to, via) = thread_links(&out.points, &population_order, rank);
                    (Form::Thread { to, via }, current)
                } else {
                    (
                        Form::Box {
                            height: 0.01 + 0.03 * lane.next_f64(),
                        },
                        0.01,
                    )
                }
            } else if rank % 2 == 0 {
                let (to, via) = thread_links(&out.points, &population_order, rank);
                (Form::Thread { to, via }, current)
            } else {
                (
                    Form::Ring {
                        normal: [0.0, 1.0, 0.0],
                        inner: 0.014,
                        outer: 0.02,
                    },
                    0.02,
                )
            };
            if let Some(point) = out.points.get_mut(index) {
                point.form = form;
                point.radius = radius;
            }
        }
        // L11 appends its curved-grid scenery behind the portal prefix and
        // trims populations to the same total, so counts and the journey
        // hold while the ground reads.
        if self.level.get() == 11 && !out.points.is_empty() {
            let total = out.points.len();
            out.points.push(Point {
                position: [0.0, 0.0, 0.0],
                radius: 0.5 / 3.0,
                kind: MarkerKind::Population,
                form: Form::Grid {
                    normal: [0.0, 1.0, 0.0],
                    curvature: L11_CURVATURE,
                },
                ..Point::bare([0.0; 3], 0.01, MarkerKind::Population)
            });
            out.points.truncate(total);
        }
        out
    }
}

/// Thread endpoints for the `rank`-th population: the next two populations
/// by order, wrapping within the population list (#384).
///
/// Chains of lines read as rivers, roads, and streets. A lone population
/// links to itself (a zero-length thread the renderer draws as a dot).
fn thread_links(points: &[Point], order: &[usize], rank: usize) -> ([f64; 3], [f64; 3]) {
    if order.is_empty() {
        return ([0.0; 3], [0.0; 3]);
    }
    let at = |k: usize| {
        order
            .get((rank + k) % order.len())
            .and_then(|index| points.get(*index))
            .map_or([0.0; 3], |point| point.position)
    };
    (at(1), at(2))
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

    #[test]
    fn tail_splits_half_portals_half_populations() {
        let out = TailGenerator::new(tail_level(12), 8).generate(42, &tail_budget());
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
    fn tail_wears_outlines_threads_and_rings_on_grounds() {
        for n in [11u8, 12, 13] {
            let out = TailGenerator::new(tail_level(n), 8).generate(42, &tail_budget());
            let mut rects = 0u32;
            let mut boxes = 0u32;
            let mut threads = 0u32;
            let mut rings = 0u32;
            for point in &out.points {
                match point.form {
                    Form::Rect { .. } => {
                        rects += 1;
                        assert_eq!(point.kind, MarkerKind::Portal, "L{n} rects open");
                    }
                    Form::Box { .. } => {
                        boxes += 1;
                    }
                    Form::Thread { to, via } => {
                        threads += 1;
                        assert_eq!(point.kind, MarkerKind::Population);
                        // Endpoints land on fellow populations.
                        for end in [to, via] {
                            assert!(
                                out.points.iter().any(|other| other.position == end),
                                "L{n} thread leaves the scatter"
                            );
                        }
                    }
                    Form::Ring { .. } => {
                        rings += 1;
                        assert_eq!(point.kind, MarkerKind::Population);
                    }
                    Form::Grid { curvature, .. } => {
                        assert_eq!(n, 11, "only L11 carries its ground");
                        assert_eq!(curvature, L11_CURVATURE);
                    }
                    other => panic!("L{n} wears a wrong form: {other:?}"),
                }
                // Grounds: flat below, curved at L11.
                assert!(
                    point.position[1].abs() <= 0.06,
                    "L{n} leaves its ground: {}",
                    point.position[1]
                );
            }
            if n == 13 {
                assert!(boxes > 0 && rects == 0, "L13 builds boxes");
            } else {
                assert!(rects > 0 && boxes == 0, "L{n} draws outlines");
            }
            assert!(
                (threads > 0 && rings > 0) || n == 13,
                "L{n} threads and rings"
            );
        }
    }

    #[test]
    fn tail_always_offers_a_portal() {
        let tight = Constraints::new(0.0, 4, 4, [0.5, 0.5, 0.5]).expect("valid");
        let out = TailGenerator::new(tail_level(12), 8).generate(7, &tight);
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
        let tail = TailGenerator::new(tail_level(11), 8);
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
        let tail = TailGenerator::new(tail_level(12), 8).generate(7, &tail_budget());
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
        assert!(
            TailGenerator::new(tail_level(13), 8)
                .generate(3, &bad)
                .points
                .is_empty()
        );
        assert!(RoomGenerator::new(12).generate(3, &bad).points.is_empty());
    }
}
