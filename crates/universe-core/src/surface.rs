//! One icosphere surface from planet to building, per level (#458, ADR 0024).
//!
//! The planet is the core icosphere split once (80 triangles). Each surface
//! level L10-L13 reads the same triangles split to its own depth, chosen so
//! a triangle edge is about a quarter of the open cell under the ladder
//! sizes. Everything here is pure `f64` and deterministic (`E-DET-TIERS`).

use crate::coords::Level;
use crate::frame::{IDENTITY_UP, Opened};
use crate::icosphere::{TriAddr, grow_clusters, normalize, pick_enterable, tri_centre};
use crate::nest::child_ratio;
use crate::terrain::{PLANET_RADIUS_CELL, REGION_PORTALS, RELIEF_RANGE_CELL, surface_radius};

/// Radius of the tile window inside an open tail cell (cell units).
///
/// Inside the cell's `0.5` shell, so every anchor a generator emits passes
/// `clamp_to_sphere` unchanged.
pub const WINDOW_RADIUS: f64 = 0.45;

/// Fixed-point steps of [`SurfaceFrame::ground_height`] (converges to
/// about 1e-9 cell units on the steepest L10 columns the tests probe).
const GROUND_STEPS: usize = 24;

/// Clusters per tail cell (L11-L13): today's tail marker count (Q9).
pub const TAIL_CLUSTERS: u32 = 8;

/// Icosphere depth per surface level L10..=L13 (ADR 0024).
///
/// L10 is the default single split (Q2). L11-L13 follow the R11 ladder
/// sizes: about 5, 11, and 19 splits put a triangle edge near a quarter of
/// the cell (tested against [`child_ratio`]).
pub const SURFACE_DEPTH: [u8; 4] = [1, 5, 11, 19];

/// Edge of a base icosahedron face over its circumradius.
const BASE_EDGE: f64 = 1.051_462_224_238_267_3;

/// Icosphere depth of the surface at `level`, or `None` outside L10-L13.
#[must_use]
pub fn surface_depth(level: Level) -> Option<u8> {
    let index = usize::from(level.get().checked_sub(10)?);
    SURFACE_DEPTH.get(index).copied()
}

/// Open-cell size at `level` in L10 units (`None` outside L10-L13).
///
/// The product of the true ladder ratios from L10 down (one at L10).
#[must_use]
pub fn cell_span_l10(level: Level) -> Option<f64> {
    if !(10..=13).contains(&level.get()) {
        return None;
    }
    let mut span = 1.0;
    for n in 10..level.get() {
        span *= child_ratio(Level::new(n)?)?;
    }
    Some(span)
}

/// Nominal triangle edge at `depth`, in L10 units.
#[must_use]
pub fn edge_l10(depth: u8) -> f64 {
    BASE_EDGE * PLANET_RADIUS_CELL / 2f64.powi(i32::from(depth))
}

/// One cluster of a surface cell: its tiles, whether it opens, its anchor.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceCluster {
    /// Tiles at the cell's surface depth, sorted by address.
    pub tiles: Vec<TriAddr>,
    /// Whether the cluster is enterable (a portal) or closed (a population).
    pub enterable: bool,
    /// The tile whose centre is nearest the cluster mean (C4 anchor).
    pub anchor: TriAddr,
}

/// The open surface cell L10-L13 seen from the planet (#458, ADR 0024).
///
/// Holds the planet seed, the opened steps from L10 down (the same
/// position, ratio, and up the navigation pushes, so the arithmetic
/// matches [`Opened::to_child_point`] exactly), and the entered parent
/// cluster's tiles. Maps planet points to and from cell units, answers the
/// ground height, and grows the cell's clusters. Pure and deterministic.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFrame {
    /// Open level, `10..=13`.
    level: Level,
    /// Seed of the L10 planet cell (the relief reads it).
    planet_seed: u64,
    /// Seed of the open cell (its clusters read it).
    cell_seed: u64,
    /// Opened steps from L10 down to the open cell.
    steps: Vec<Opened>,
    /// Entered parent cluster tiles (empty at L10: the whole sphere).
    parent_tiles: Vec<TriAddr>,
}

impl SurfaceFrame {
    /// The open L10 planet cell for `planet_seed`.
    #[must_use]
    pub fn planet(planet_seed: u64) -> SurfaceFrame {
        SurfaceFrame {
            level: Level::new(10).unwrap_or(Level::MAX),
            planet_seed,
            cell_seed: planet_seed,
            steps: Vec::new(),
            parent_tiles: Vec::new(),
        }
    }

    /// The surface cell reached by `chain` under `root`, or `None` off L10-L13.
    ///
    /// The L10 marker is located among the generated L10 portals (the
    /// `k`-th portal is the `k`-th enterable region cluster); deeper markers
    /// are tail portals, which come first in each tail cell, so the marker
    /// is the ordinal. `None` when a step is not a portal.
    #[must_use]
    pub fn for_path(root: u64, chain: &[u32]) -> Option<SurfaceFrame> {
        use crate::r#gen::{Form, MarkerKind};
        use crate::nest::{generate_cell, path_seed};
        if chain.len() < 9 || chain.len() > 12 {
            return None;
        }
        let planet_chain = chain.get(..9)?;
        let mut frame = SurfaceFrame::planet(path_seed(root, planet_chain));
        if chain.len() == 9 {
            return Some(frame);
        }
        let marker = usize::try_from(*chain.get(9)?).ok()?;
        let planet = generate_cell(root, planet_chain);
        let point = planet.points.get(marker)?;
        if point.kind != MarkerKind::Portal {
            return None;
        }
        let ordinal = planet
            .points
            .iter()
            .take(marker)
            .filter(|p| p.kind == MarkerKind::Portal)
            .count();
        let up = match point.form {
            Form::Patch { normal } => normal,
            _ => IDENTITY_UP,
        };
        let seed = path_seed(root, chain.get(..10)?);
        frame = frame.child(ordinal, point.portal_position(), up, seed)?;
        for depth in 10..chain.len() {
            let ordinal = usize::try_from(*chain.get(depth)?).ok()?;
            let position = frame.portal_positions().get(ordinal).copied()?;
            let seed = path_seed(root, chain.get(..=depth)?);
            frame = frame.child(ordinal, position, IDENTITY_UP, seed)?;
        }
        Some(frame)
    }

    /// Steps into the `ordinal`-th enterable cluster, opened at `position`.
    fn child(
        &self,
        ordinal: usize,
        position: [f64; 3],
        up: [f64; 3],
        seed: u64,
    ) -> Option<SurfaceFrame> {
        let level = self.level.deeper()?;
        if level.get() > 13 {
            return None;
        }
        let cluster = self
            .clusters()
            .into_iter()
            .filter(|cluster| cluster.enterable)
            .nth(ordinal)?;
        let ratio = child_ratio(self.level)?;
        let mut steps = self.steps.clone();
        steps.push(Opened {
            marker: u32::try_from(ordinal).ok()?,
            position,
            ratio,
            anonymous: false,
            up,
        });
        Some(SurfaceFrame {
            level,
            planet_seed: self.planet_seed,
            cell_seed: seed,
            steps,
            parent_tiles: cluster.tiles,
        })
    }

    /// Open level of this frame.
    #[must_use]
    pub fn level(&self) -> Level {
        self.level
    }

    /// Seed of the L10 planet cell.
    #[must_use]
    pub fn planet_seed(&self) -> u64 {
        self.planet_seed
    }

    /// Opened steps from L10 down to this cell.
    #[must_use]
    pub fn steps(&self) -> &[Opened] {
        &self.steps
    }

    /// Icosphere depth of this cell's tiles.
    #[must_use]
    pub fn depth(&self) -> u8 {
        surface_depth(self.level).unwrap_or(1)
    }

    /// Planet point (L10 units) in this cell's units.
    #[must_use]
    pub fn to_cell(&self, planet: [f64; 3]) -> [f64; 3] {
        self.steps
            .iter()
            .fold(planet, |point, step| step.to_child_point(point))
    }

    /// Cell point in planet (L10) units.
    #[must_use]
    pub fn to_planet(&self, cell: [f64; 3]) -> [f64; 3] {
        self.steps
            .iter()
            .rev()
            .fold(cell, |point, step| step.to_parent_point(point))
    }

    /// Cell direction turned into planet axes (no origin, no scale).
    #[must_use]
    pub fn to_planet_direction(&self, cell: [f64; 3]) -> [f64; 3] {
        self.steps
            .iter()
            .rev()
            .fold(cell, |dir, step| step.to_parent_direction(dir))
    }

    /// The relief surface under `direction`, in planet units.
    #[must_use]
    pub fn surface_point(&self, direction: [f64; 3]) -> [f64; 3] {
        let unit = normalize(direction);
        let radius = surface_radius(self.planet_seed, unit);
        [unit[0] * radius, unit[1] * radius, unit[2] * radius]
    }

    /// The relief surface under `direction`, in cell units.
    #[must_use]
    pub fn surface_cell(&self, direction: [f64; 3]) -> [f64; 3] {
        self.to_cell(self.surface_point(direction))
    }

    /// Height of the relief surface under cell `(x, z)` (C2/C7).
    ///
    /// Solves for the surface point whose cell `x` and `z` are the inputs
    /// with a few fixed-point steps (inside a tail cell the planet's radial
    /// line is only slightly tilted from the cell's up). At L10 this is the
    /// upper surface over the `(x, z)` column. Non-finite input, or a
    /// column that misses the planet at L10, returns `0.0`.
    #[must_use]
    pub fn ground_height(&self, x: f64, z: f64) -> f64 {
        if !x.is_finite() || !z.is_finite() {
            return 0.0;
        }
        let start_y = if self.level.get() == 10 {
            let reach = PLANET_RADIUS_CELL * PLANET_RADIUS_CELL - (x * x + z * z);
            if reach <= 0.0 {
                return 0.0;
            }
            reach.sqrt()
        } else {
            0.0
        };
        let mut probe = [x, start_y, z];
        let mut height = start_y;
        for _ in 0..GROUND_STEPS {
            let ground = self.surface_cell(self.to_planet(probe));
            height = ground[1];
            probe = [
                probe[0] + (x - ground[0]),
                height,
                probe[2] + (z - ground[2]),
            ];
        }
        if height.is_finite() { height } else { 0.0 }
    }

    /// This cell's tiles: in-window descendants of the entered cluster.
    ///
    /// L10 keeps the whole sphere at its depth (80 triangles). Deeper cells
    /// keep the tiles whose surface centre lies within [`WINDOW_RADIUS`] of
    /// the cell origin.
    #[must_use]
    pub fn tiles(&self) -> Vec<TriAddr> {
        let depth = self.depth();
        if self.level.get() == 10 {
            return crate::icosphere::all_at_depth(depth);
        }
        let span = cell_span_l10(self.level).unwrap_or(1.0);
        let scale = (PLANET_RADIUS_CELL + RELIEF_RANGE_CELL) / span;
        crate::icosphere::window_tiles(
            &self.parent_tiles,
            depth,
            |centre, reach| {
                let cell = self.surface_cell(centre);
                let horizontal = (cell[0] * cell[0] + cell[2] * cell[2]).sqrt();
                horizontal <= (WINDOW_RADIUS + reach * scale) * 1.25
            },
            |centre| {
                let cell = self.surface_cell(centre);
                (cell[0] * cell[0] + cell[1] * cell[1] + cell[2] * cell[2]).sqrt() <= WINDOW_RADIUS
            },
        )
    }

    /// Clusters of this cell with enterable flags and anchors.
    ///
    /// L10 grows [`REGION_PORTALS`] region clusters over the whole sphere,
    /// all open; L11-L13 grow [`TAIL_CLUSTERS`] over the window and open
    /// half (today's counts, Q9). Order is the grow order.
    #[must_use]
    pub fn clusters(&self) -> Vec<SurfaceCluster> {
        let tiles = self.tiles();
        let (count, open) = if self.level.get() == 10 {
            (REGION_PORTALS, REGION_PORTALS as usize)
        } else {
            (TAIL_CLUSTERS, TAIL_CLUSTERS as usize / 2)
        };
        let grown = grow_clusters(self.cell_seed, &tiles, count);
        let flags = pick_enterable(self.cell_seed, &grown, open);
        grown
            .into_iter()
            .zip(flags)
            .filter_map(|(tiles, enterable)| {
                let anchor = anchor_tile(&tiles)?;
                Some(SurfaceCluster {
                    tiles,
                    enterable,
                    anchor,
                })
            })
            .collect()
    }

    /// Cell positions of the enterable clusters' anchors, in grow order.
    #[must_use]
    pub fn portal_positions(&self) -> Vec<[f64; 3]> {
        self.clusters()
            .iter()
            .filter(|cluster| cluster.enterable)
            .filter_map(|cluster| tri_centre(cluster.anchor))
            .map(|centre| self.surface_cell(centre))
            .collect()
    }
}

/// The tile of `tiles` whose centre is nearest their mean direction.
///
/// Ties keep the first by address order, so the anchor is deterministic
/// (C8) and always a member of the cluster (C4). `None` when empty.
#[must_use]
pub fn anchor_tile(tiles: &[TriAddr]) -> Option<TriAddr> {
    let centres: Vec<(TriAddr, [f64; 3])> = tiles
        .iter()
        .filter_map(|tile| tri_centre(*tile).map(|centre| (*tile, centre)))
        .collect();
    let mut sum = [0.0, 0.0, 0.0];
    for (_, centre) in &centres {
        sum = [sum[0] + centre[0], sum[1] + centre[1], sum[2] + centre[2]];
    }
    let mean = normalize(sum);
    let mut best: Option<(f64, TriAddr)> = None;
    for (tile, centre) in centres {
        let gap = (centre[0] - mean[0]).powi(2)
            + (centre[1] - mean[1]).powi(2)
            + (centre[2] - mean[2]).powi(2);
        if best.is_none_or(|(best_gap, _)| gap.total_cmp(&best_gap).is_lt()) {
            best = Some((gap, tile));
        }
    }
    best.map(|(_, tile)| tile)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(n: u8) -> Level {
        Level::new(n).expect("valid level")
    }

    #[test]
    fn planet_defaults_to_one_split() {
        assert_eq!(surface_depth(level(10)), Some(1), "Q2: 80 triangles");
        assert!(surface_depth(level(9)).is_none());
        assert!(surface_depth(level(14)).is_none());
    }

    #[test]
    fn tail_depths_follow_the_ladder_sizes() {
        for n in 11..=13u8 {
            let depth = surface_depth(level(n)).expect("tail depth");
            let span = cell_span_l10(level(n)).expect("tail span");
            let ratio = edge_l10(depth) / span;
            assert!(
                (0.125..=0.5).contains(&ratio),
                "L{n}: edge/cell {ratio} outside [1/8, 1/2]"
            );
        }
        let depths: Vec<u8> = (10..=13u8)
            .filter_map(|n| surface_depth(level(n)))
            .collect();
        assert!(
            depths.windows(2).all(|w| w[0] < w[1]),
            "depth grows per level"
        );
        assert!(
            depths.iter().all(|d| *d <= crate::icosphere::MAX_DEPTH),
            "addressable"
        );
    }

    fn journey(depth: usize) -> SurfaceFrame {
        let chain = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
        let named = chain.get(..depth).expect("journey reaches the surface");
        SurfaceFrame::for_path(crate::nav::DEMO_SEED, named).expect("surface frame")
    }

    #[test]
    fn frame_maps_round_trip_down_to_plots() {
        for depth in 9..=12usize {
            let frame = journey(depth);
            assert_eq!(usize::from(frame.level().get()), depth + 1);
            for cell in [[0.0, 0.0, 0.0], [0.3, -0.1, 0.2], [-0.45, 0.05, 0.1]] {
                let back = frame.to_cell(frame.to_planet(cell));
                for axis in 0..3 {
                    assert!(
                        (back[axis] - cell[axis]).abs() < 1e-9,
                        "L{} round trip {cell:?} -> {back:?}",
                        frame.level().get()
                    );
                }
            }
        }
    }

    #[test]
    fn ground_height_sits_on_the_relief_surface() {
        for depth in 9..=12usize {
            let frame = journey(depth);
            // L10 columns stay well inside the disc; near the limb the
            // column grazes the relief and the clamp is radial there anyway.
            let columns: &[(f64, f64)] = if depth == 9 {
                &[(0.0, 0.0), (0.1, -0.05), (-0.12, 0.08)]
            } else {
                &[(0.0, 0.0), (0.2, -0.1), (-0.3, 0.25)]
            };
            for &(x, z) in columns {
                let y = frame.ground_height(x, z);
                let planet = frame.to_planet([x, y, z]);
                let radius =
                    (planet[0] * planet[0] + planet[1] * planet[1] + planet[2] * planet[2]).sqrt();
                let want = surface_radius(frame.planet_seed(), planet);
                let span = cell_span_l10(frame.level()).expect("span");
                assert!(
                    ((radius - want) / span).abs() < 1e-6,
                    "L{} ({x}, {z}): off surface by {} cells",
                    frame.level().get(),
                    (radius - want) / span
                );
            }
        }
    }

    #[test]
    fn tail_cells_hold_windowed_nested_clusters() {
        for depth in 10..=12usize {
            let frame = journey(depth);
            let tiles = frame.tiles();
            assert!(
                tiles.len() >= TAIL_CLUSTERS as usize,
                "L{} window holds tiles",
                frame.level().get()
            );
            assert!(tiles.iter().all(|t| t.depth() == frame.depth()));
            assert!(
                tiles
                    .iter()
                    .all(|t| frame.parent_tiles.iter().any(|p| t.is_within(*p))),
                "every tile lies inside the entered cluster (C4)"
            );
            let clusters = frame.clusters();
            let covered: usize = clusters.iter().map(|c| c.tiles.len()).sum();
            assert_eq!(covered, tiles.len(), "clusters cover the window (C5)");
            assert_eq!(
                clusters.iter().filter(|c| c.enterable).count(),
                TAIL_CLUSTERS as usize / 2
            );
            assert!(
                clusters.iter().all(|c| c.tiles.contains(&c.anchor)),
                "anchors are members"
            );
        }
    }

    #[test]
    fn frames_are_deterministic() {
        assert_eq!(journey(12), journey(12), "C8");
        assert_eq!(journey(12).clusters(), journey(12).clusters());
    }

    #[test]
    fn frame_steps_match_the_navigation_chain() {
        use crate::nav::{DEMO_SEED, MarkerIndex, Universe};
        let chain = crate::nest::autopilot_path(DEMO_SEED);
        let mut universe = Universe::new(DEMO_SEED);
        for &marker in chain.iter().take(12) {
            assert!(universe.open(MarkerIndex(marker)), "journey opens");
            let n = universe.level().get();
            if !(11..=13).contains(&n) {
                continue;
            }
            let frame =
                SurfaceFrame::for_path(DEMO_SEED, &universe.path.indices()).expect("surface frame");
            let named: Vec<Opened> = universe
                .path
                .chain()
                .iter()
                .filter(|opened| !opened.anonymous)
                .skip(9)
                .copied()
                .map(|opened| Opened {
                    marker: 0,
                    ..opened
                })
                .collect();
            let steps: Vec<Opened> = frame
                .steps()
                .iter()
                .map(|step| Opened { marker: 0, ..*step })
                .collect();
            assert_eq!(steps, named, "L{n} frame retraces the opened chain exactly");
        }
    }

    #[test]
    fn regions_and_cities_nest_down_the_journey() {
        for depth in 10..=12usize {
            let parent = journey(depth - 1);
            let child = journey(depth);
            let open: Vec<SurfaceCluster> = parent
                .clusters()
                .into_iter()
                .filter(|c| c.enterable)
                .collect();
            assert!(
                open.iter()
                    .any(|cluster| cluster.tiles == child.parent_tiles),
                "L{} entered cluster is one of its parent's open clusters",
                child.level().get()
            );
            for tile in child.tiles() {
                let ancestor = tile.ancestor_at(parent.depth()).expect("ancestor");
                assert!(child.parent_tiles.contains(&ancestor), "C4 nesting");
            }
        }
    }
}
