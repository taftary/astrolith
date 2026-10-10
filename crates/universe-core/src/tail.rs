//! Sparse tail generators for L11-L14: regions, cities, buildings, room (#375, #458).
//!
//! The ladder ends the way it begins: deterministic scatters with no stored
//! state. [`TailGenerator`](crate::tail::TailGenerator) serves L11-L13 (half
//! portals that open the next rung, half populations that decorate it).
//! Every point is a tile of the one icosphere surface: portals wear the
//! enterable mark for the level (dots on the tile at L11/L12, one building
//! box standing on the slope per enterable L13 plot per Q11) and
//! populations are plain dots; rivers, roads, lakes, and decorative boxes
//! are gone (Q10), and so are the bowl grid, rects, and threads. Points
//! settle onto the true-curvature ground at each level's scale (#394).
//! [`RoomGenerator`](crate::tail::RoomGenerator) serves L14 (populations
//! only, so nothing opens past the room) with a room shell box around the
//! furniture. Both reuse [`UniformGenerator`](crate::gen::UniformGenerator)
//! for positions, counts, and child budgets, then set the portal/population
//! split.

use crate::coords::Level;
use crate::frame::planet_radius_cells;
use crate::r#gen::{
    Constraints, Form, Generated, Generator, MarkerKind, Point, UniformGenerator, clamp_to_sphere,
};
use crate::seed::{Rng, hash_triple};

/// Decorrelation lane for tail per-point draws (rect halves, jitter).
const TAIL_STREAM_TAG: u64 = 0x7A11_6E15_709A_11E5;

/// Scatter generator for one tail cell at L11, L12, or L13.
///
/// Emits the uniform scatter for `base_count`, then marks the first half of
/// the points portals (they open the next rung) and the rest populations.
/// Every point is a tile of the one icosphere surface (#458): portals wear
/// the enterable mark (dots at L11/L12, one building box per enterable L13
/// plot standing on the slope), populations are plain dots. Points settle
/// onto the true-curvature ground as the local sphere patch at each level's
/// scale (#394); portals sit on population tile spots, each carrying its
/// portal offset.
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

    /// Box height for an L13 portal from its lane.
    fn portal_height(lane: &mut Rng) -> f64 {
        0.02 + 0.06 * lane.next_f64()
    }
}

impl Generator for TailGenerator {
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        let mut out = UniformGenerator::new(self.base_count).generate(seed, parent);
        let portals = out.points.len() / 2;
        // True curvature from the planet radius at this level's scale (#394):
        // a gentle bowl at L11, nearly flat at L12, flat to the eye at L13.
        let radius = planet_radius_cells(self.level).unwrap_or(0.0);
        // Pass one: kinds, enterable marks, radii, and the ground. Points
        // settle onto `y = -d^2 / (2R)` with a breath of jitter. Portals
        // wear the enterable mark (dots at L11/L12, one building box per
        // enterable L13 plot standing on the slope); populations are plain
        // dots (#458 Q10/Q11).
        for (index, point) in out.points.iter_mut().enumerate() {
            let mut lane = Rng::new(hash_triple(seed, TAIL_STREAM_TAG, index as u64));
            if index >= portals {
                point.kind = MarkerKind::Population;
                point.form = Form::Dot;
            } else if self.level.get() == 13 {
                point.form = Form::Box {
                    height: Self::portal_height(&mut lane),
                };
            } else {
                point.form = Form::Dot;
            }
            let jitter = (lane.next_f64() * 2.0 - 1.0) * 0.005;
            if radius > 0.0 {
                let sag = (point.position[0] * point.position[0]
                    + point.position[2] * point.position[2])
                    / (2.0 * radius);
                point.position[1] = -sag + jitter;
            } else {
                point.position[1] = jitter;
            }
            point.position = clamp_to_sphere(point.position);
        }
        // Pass two: portals sit on population tile spots (#394, #458). Each
        // portal copies its anchor tile position, so cities, buildings, and
        // rooms all stand on the surface; each carries its portal offset so
        // preview, open, close, and siblings follow it.
        {
            // Anchor vertices from the settled populations (copied, so the
            // assignment below never aliases).
            let anchors: Vec<[f64; 3]> = (portals..out.points.len())
                .filter_map(|index| out.points.get(index))
                .map(|point| point.position)
                .collect();
            for portal_index in 0..portals {
                let Some(point) = out.points.get_mut(portal_index) else {
                    continue;
                };
                if !anchors.is_empty() {
                    let anchor = anchors
                        .get(portal_index % anchors.len())
                        .copied()
                        .unwrap_or([0.0, 0.0, 0.0]);
                    point.position = anchor;
                }
                point.position = clamp_to_sphere(point.position);
                point.portal = Some([0.0; 3]);
            }
        }
        out
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
    fn tail_levels_wear_no_rects_or_patches() {
        use crate::frame::planet_radius_cells;
        for n in [11u8, 12, 13] {
            let out = TailGenerator::new(tail_level(n), 8).generate(42, &tail_budget());
            let mut dots = 0u32;
            let mut boxes = 0u32;
            let expected_radius = planet_radius_cells(tail_level(n)).expect("tail ground");
            assert!(expected_radius > 0.0, "L{n} ground curves truly");
            for point in &out.points {
                match point.form {
                    Form::Dot => {
                        dots += 1;
                    }
                    Form::Box { .. } => {
                        boxes += 1;
                        assert_eq!(
                            point.kind,
                            MarkerKind::Portal,
                            "L{n} boxes open the next rung"
                        );
                    }
                    Form::Rect { .. }
                    | Form::Patch { .. }
                    | Form::Grid { .. }
                    | Form::Thread { .. }
                    | Form::Ring { .. } => {
                        panic!("L{n} wears a separate flat piece: {:?}", point.form);
                    }
                    _ => {
                        dots += 1;
                    }
                }
                // Points settle by the true planet radius at each level.
                assert!(
                    point.position[1].abs() <= 0.06,
                    "L{n} leaves its ground: {}",
                    point.position[1]
                );
            }
            if n == 13 {
                assert!(boxes > 0 && dots > 0, "L13 builds boxes over dots");
            } else {
                assert_eq!(boxes, 0, "L{n} builds no boxes");
                assert!(dots > 0, "L{n} draws dots");
            }
        }
    }

    #[test]
    fn plots_carry_one_building_box_on_the_slope() {
        use crate::frame::planet_radius_cells;
        let level = tail_level(13);
        let radius = planet_radius_cells(level).expect("tail ground");
        let out = TailGenerator::new(level, 8).generate(42, &tail_budget());
        let portals: Vec<_> = out
            .points
            .iter()
            .filter(|point| point.kind == MarkerKind::Portal)
            .collect();
        assert!(!portals.is_empty(), "L13 offers plots");
        for portal in portals {
            let Form::Box { height } = portal.form else {
                panic!("L13 plot wears {:?}", portal.form);
            };
            assert!(height > 0.0, "building box has no height");
            let [x, _, z] = portal.position;
            let bowl = -((x * x + z * z) / (2.0 * radius));
            assert!(
                (portal.position[1] - bowl).abs() <= 0.005 + 1e-9,
                "building box floats off the slope"
            );
            assert!(portal.portal.is_some(), "plot carries no room offset");
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
