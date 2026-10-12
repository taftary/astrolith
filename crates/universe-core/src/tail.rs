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

    /// Box height of the building at portal `ordinal` of the L13 cell
    /// seeded `seed` (one lane per portal, so a room can read the height
    /// of the building it was entered from without generating the cell).
    #[must_use]
    pub fn building_height(seed: u64, ordinal: u32) -> f64 {
        let mut lane = Rng::new(hash_triple(seed, TAIL_STREAM_TAG, u64::from(ordinal)));
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
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "E-CAST: point index below base_count (8)"
                    )]
                    let ordinal = index as u32;
                    Form::Box {
                        height: Self::building_height(seed, ordinal),
                    }
                } else {
                    Form::Dot
                };
                Point {
                    portal: Some([0.0; 3]),
                    form,
                    ..Point::bare(position, TAIL_POINT_RADIUS, MarkerKind::Portal)
                }
            } else {
                Point::bare(position, TAIL_POINT_RADIUS, MarkerKind::Population)
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
///
/// The shell takes the entered building's shape (#543): its height to
/// width ratio is the building box's, fitted inside the cell sphere. The
/// furniture stands on the shell's floor inside its walls, each piece sized
/// by its kind ([`FURNITURE_KINDS`]) and scaled with the room width so it
/// always fits. Positions come from one seeded lane per piece.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoomGenerator {
    /// Baseline point count at full density (12 for L14).
    base_count: u32,
    /// Entered building box height over its width, when known.
    building_ratio: Option<f64>,
}

/// Decorrelation lane for room furniture placement.
const ROOM_STREAM_TAG: u64 = 0x2007_F0E5_2007_F0E5;

/// Half width of a tail point's footprint (the L13 building box), cell units.
pub const TAIL_POINT_RADIUS: f64 = 0.01;

/// Room shell corner distance from the cell centre (inside the 0.5 sphere).
const ROOM_REACH: f64 = 0.48;

/// Height over width of the room when no building is known (a wide room).
const DEFAULT_ROOM_RATIO: f64 = 0.375;

/// Furniture kinds in placement order with their marker radius in a room
/// of half width [`REFERENCE_ROOM_HALF`] (the drawn outline is four times
/// the radius): bed and sofa near 2 m, tables and desk near 1.2 m, chairs
/// near 0.5 m in a 4 m room (`L14/furniture.md`).
pub const FURNITURE_KINDS: [(&str, f64); 11] = [
    ("bed", 0.05),
    ("sofa", 0.05),
    ("table", 0.03),
    ("chair", 0.0125),
    ("chair", 0.0125),
    ("desk", 0.03),
    ("chair", 0.0125),
    ("table", 0.03),
    ("chair", 0.0125),
    ("sofa", 0.05),
    ("chair", 0.0125),
];

/// Room half width the furniture radii are given for.
const REFERENCE_ROOM_HALF: f64 = 0.4;

/// Drawn outline half size over the marker radius (matches the renderer).
const FURNITURE_OUTLINE: f64 = 4.0;

/// Furniture size share of the reference sizes, so eleven pieces fit the
/// floor of a narrow tower room as well as a wide one.
const FURNITURE_FIT: f64 = 0.5;

/// Seeded placement tries per furniture piece before it may overlap.
const FURNITURE_TRIES: u32 = 48;

impl RoomGenerator {
    /// Creates a room generator emitting up to `base_count` points.
    #[must_use]
    pub const fn new(base_count: u32) -> RoomGenerator {
        RoomGenerator {
            base_count,
            building_ratio: None,
        }
    }

    /// The same room shaped like the entered building (height over width).
    #[must_use]
    pub const fn shaped(self, building_ratio: f64) -> RoomGenerator {
        RoomGenerator {
            building_ratio: Some(building_ratio),
            ..self
        }
    }

    /// Shell half width and full height for this room.
    ///
    /// Height over full width is the building's ratio; the corner
    /// `(w, h/2, w)` reaches exactly [`ROOM_REACH`], so the shell fits the
    /// cell sphere for every ratio.
    #[must_use]
    pub fn shell(&self) -> (f64, f64) {
        let ratio = self
            .building_ratio
            .filter(|ratio| ratio.is_finite() && *ratio > 0.0)
            .unwrap_or(DEFAULT_ROOM_RATIO);
        let half = ROOM_REACH / (2.0 + ratio * ratio).sqrt();
        (half, 2.0 * half * ratio)
    }
}

impl Generator for RoomGenerator {
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        if !parent.is_valid() {
            return Generated {
                points: Vec::new(),
                child_constraints: Vec::new(),
            };
        }
        let total = UniformGenerator::new(self.base_count).count_for(parent) as usize;
        let (half, height) = self.shell();
        let floor = -height / 2.0;
        let scale = half / REFERENCE_ROOM_HALF * FURNITURE_FIT;
        let pieces = total.saturating_sub(1).min(FURNITURE_KINDS.len());
        // Largest pieces find their spots first, so a crowded tower room
        // still fits its bed and sofas; the output keeps the kinds order.
        let mut order: Vec<(usize, f64)> = FURNITURE_KINDS
            .iter()
            .take(pieces)
            .map(|(_, radius)| *radius)
            .enumerate()
            .collect();
        order.sort_by(|a, b| b.1.total_cmp(&a.1));
        let mut placed: Vec<Option<Point>> = vec![None; pieces];
        for (index, base) in order {
            let radius = base * scale;
            let outline = radius * FURNITURE_OUTLINE;
            let room = (half - outline).max(0.0);
            let mut lane = Rng::new(hash_triple(seed, ROOM_STREAM_TAG, index as u64));
            // A few seeded tries for a spot clear of the pieces already
            // placed; a crowded room keeps the last try.
            let mut spot = [0.0, 0.0];
            for _ in 0..FURNITURE_TRIES {
                spot = [
                    (lane.next_f64() * 2.0 - 1.0) * room,
                    (lane.next_f64() * 2.0 - 1.0) * room,
                ];
                let clear = placed.iter().flatten().all(|other: &Point| {
                    let reach = other.radius * FURNITURE_OUTLINE + outline;
                    (other.position[0] - spot[0]).abs() >= reach
                        || (other.position[2] - spot[1]).abs() >= reach
                });
                if clear {
                    break;
                }
            }
            if let Some(slot) = placed.get_mut(index) {
                *slot = Some(Point::bare(
                    [spot[0], floor + outline, spot[1]],
                    radius,
                    MarkerKind::Population,
                ));
            }
        }
        let mut points: Vec<Point> = placed.into_iter().flatten().collect();
        if total > 0 {
            // The room shell: a box standing on the floor, centred on the cell.
            points.push(Point {
                form: Form::Box { height },
                ..Point::bare([0.0, floor, 0.0], half, MarkerKind::Population)
            });
        }
        Generated {
            points,
            child_constraints: vec![halved_child(parent); 8],
        }
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
        let (half, height) = RoomGenerator::new(12).shell();
        assert_eq!(
            shell.form,
            Form::Box { height },
            "the room shell closes the cell"
        );
        assert_eq!(shell.radius, half);
        assert_eq!(out.points.len(), 12, "eleven pieces plus the shell");
    }

    #[test]
    fn room_takes_the_building_shape_inside_the_cell() {
        for ratio in [1.0, 2.5, 4.0] {
            let (half, height) = RoomGenerator::new(12).shaped(ratio).shell();
            assert!(
                (height / (2.0 * half) - ratio).abs() < 1e-12,
                "room ratio {} vs building {ratio}",
                height / (2.0 * half)
            );
            let corner = (2.0 * half * half + (height / 2.0) * (height / 2.0)).sqrt();
            assert!(corner <= 0.5, "room corner leaves the cell: {corner}");
        }
        let wide = RoomGenerator::new(12).shell();
        let tower = RoomGenerator::new(12).shaped(4.0).shell();
        assert!(
            tower.1 / tower.0 > wide.1 / wide.0,
            "towers make tall rooms"
        );
    }

    #[test]
    fn furniture_stands_on_the_floor_inside_the_walls() {
        for (seed, ratio) in [(7u64, None), (42, Some(1.0)), (9, Some(4.0))] {
            let room = match ratio {
                Some(ratio) => RoomGenerator::new(12).shaped(ratio),
                None => RoomGenerator::new(12),
            };
            let (half, height) = room.shell();
            let floor = -height / 2.0;
            let out = room.generate(seed, &room_budget());
            let furniture = &out.points[..out.points.len() - 1];
            assert_eq!(furniture.len(), FURNITURE_KINDS.len());
            for (piece, (kind, _)) in furniture.iter().zip(FURNITURE_KINDS) {
                let outline = piece.radius * FURNITURE_OUTLINE;
                let [x, y, z] = piece.position;
                assert!(
                    (y - outline - floor).abs() < 1e-12,
                    "{kind} floats: base {} vs floor {floor}",
                    y - outline
                );
                assert!(
                    x.abs() + outline <= half + 1e-12 && z.abs() + outline <= half + 1e-12,
                    "{kind} pokes through a wall"
                );
            }
            for (i, a) in furniture.iter().enumerate() {
                for b in &furniture[i + 1..] {
                    let reach = (a.radius + b.radius) * FURNITURE_OUTLINE;
                    let apart = (a.position[0] - b.position[0]).abs() >= reach - 1e-12
                        || (a.position[2] - b.position[2]).abs() >= reach - 1e-12;
                    assert!(
                        apart,
                        "furniture overlaps in room {seed}: {:?} r{} vs {:?} r{} half {half}",
                        a.position, a.radius, b.position, b.radius
                    );
                }
            }
            let bed = furniture[0].radius;
            let chair = furniture[3].radius;
            assert!(bed > chair, "a bed outsizes a chair");
        }
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
