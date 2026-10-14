//! The planet's one icosphere surface and its ground (#458, ADR 0024, #632).
//!
//! The planet is the core icosphere split once (80 triangles). Since #632
//! nothing opens below the planet: the levels L11-L14 are shells picked by
//! height ([`crate::shell`]), so the only surface cell is the open planet
//! itself (L10). Its clusters still place the region portals of L9 planet
//! interiors. Everything here is pure `f64` and deterministic
//! (`E-DET-TIERS`).

use crate::coords::Level;
use crate::icosphere::{TriAddr, grow_clusters, normalize, pick_enterable, tri_centre};
use crate::look::{WorldMode, WorldOptions};
use crate::terrain::{PLANET_RADIUS_CELL, REGION_PORTALS, surface_radius};

/// Fixed-point steps of [`SurfaceFrame::ground_height`] (converges to
/// about 1e-9 cell units on the steepest L10 columns the tests probe).
const GROUND_STEPS: usize = 24;

/// Icosphere depth of the planet surface (Q2: one split, 80 triangles).
pub const PLANET_DEPTH: u8 = 1;

/// Mesh split depth on Earth: the finest edge reads about 31-40 cm.
/// Big planets add one level per doubling above this (see
/// [`max_depth_for_radius`]); the addressable limit
/// ([`crate::icosphere::MAX_DEPTH`]) caps the growth.
const BASE_MESH_DEPTH: u8 = 24;

/// Extra mesh splits above [`BASE_MESH_DEPTH`]: one per doubling of
/// radius keeps the finest edge at about 31-40 cm, capped by the
/// addressable headroom.
const MAX_EXTRA_DEPTH: u8 = 2;

/// Edge of a base icosahedron face over its circumradius.
const BASE_EDGE: f64 = 1.051_462_224_238_267_3;

/// Nominal triangle edge at `depth`, in L10 units.
#[must_use]
pub fn edge_l10(depth: u8) -> f64 {
    BASE_EDGE * PLANET_RADIUS_CELL / 2f64.powi(i32::from(depth))
}

/// One cluster of the planet surface: its tiles, whether it opens, its anchor.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceCluster {
    /// Tiles at the surface depth, sorted by address.
    pub tiles: Vec<TriAddr>,
    /// Whether the cluster is enterable (a portal) or closed (a population).
    pub enterable: bool,
    /// The tile whose centre is nearest the cluster mean (C4 anchor).
    pub anchor: TriAddr,
}

/// Deepest mesh split on a planet of `radius_earth` Earth radii (#685,
/// the Q1 caveat): `BASE_MESH_DEPTH` on Earth and below, plus one level
/// per doubling above Earth's, so the finest edge stays at about 31-40 cm
/// on big rocky worlds. A non-finite or non-positive radius reads the
/// base depth.
#[must_use]
pub fn max_depth_for_radius(radius_earth: f64) -> u8 {
    let mut extra = 0u8;
    let mut span = 1.0;
    while extra < MAX_EXTRA_DEPTH && span * 2.0 <= radius_earth {
        span *= 2.0;
        extra += 1;
    }
    BASE_MESH_DEPTH.saturating_add(extra)
}

/// The open planet's surface (L10, #458, #632).
///
/// Holds the planet seed and whether it is the home planet (its size,
/// #648). Answers the flat-faced surface under a
/// direction, the ground height over a column, and grows the planet's
/// clusters. Pure and deterministic.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceFrame {
    /// Seed of the L10 planet cell (its clusters read it).
    planet_seed: u64,
    /// Whether this is the home planet (Earth's record, #648).
    home: bool,
    /// A given radius in Earth radii (as `f64` bits), for a body that is
    /// not a planet record, such as a moon shown on its own by a
    /// development tool ([`SurfaceFrame::body`], #654); `None` reads the
    /// planet record.
    radius_bits: Option<u64>,
    /// Classic or generated ground (#685): Classic reads the flat faces
    /// exactly as before; Generated adds the pure relief.
    world: WorldOptions,
}

impl SurfaceFrame {
    /// The open L10 planet cell for `planet_seed` (not the home planet).
    #[must_use]
    pub const fn planet(planet_seed: u64) -> SurfaceFrame {
        SurfaceFrame {
            planet_seed,
            home: false,
            radius_bits: None,
            world: WorldOptions::classic(),
        }
    }

    /// A body of `radius_earth` Earth radii drawn as a planet surface, for
    /// a development tool showing a moon on its own (#654). The game never
    /// builds one. A non-finite or non-positive radius reads the record of
    /// `seed`, as [`SurfaceFrame::planet`].
    #[must_use]
    pub fn body(seed: u64, radius_earth: f64) -> SurfaceFrame {
        SurfaceFrame {
            planet_seed: seed,
            home: false,
            radius_bits: (radius_earth.is_finite() && radius_earth > 0.0)
                .then(|| radius_earth.to_bits()),
            world: WorldOptions::classic(),
        }
    }

    /// The planet surface of the cell reached by `chain` under `root`, or
    /// `None` unless the chain ends at a planet (L10).
    #[must_use]
    pub fn for_path(root: u64, chain: &[u32]) -> Option<SurfaceFrame> {
        if chain.len() != 9 {
            return None;
        }
        Some(SurfaceFrame {
            planet_seed: crate::nest::path_seed(root, chain),
            home: crate::home::is_home_planet_cell(root, chain),
            radius_bits: None,
            world: WorldOptions::classic(),
        })
    }

    /// Open level of this frame: always the planet (L10).
    #[must_use]
    pub fn level(&self) -> Level {
        Level::new(10).unwrap_or(Level::MAX)
    }

    /// Seed of the L10 planet cell.
    #[must_use]
    pub const fn planet_seed(&self) -> u64 {
        self.planet_seed
    }

    /// The planet's radius in Earth radii, from its record
    /// ([`crate::terrain::body_recipe`]).
    #[must_use]
    pub fn radius_earth(&self) -> f64 {
        self.radius_bits.map_or_else(
            || crate::terrain::body_recipe(self.planet_seed, self.home).radius_earth,
            f64::from_bits,
        )
    }

    /// Metres per cell unit on this planet ([`crate::shell::metres_per_unit`],
    /// #648).
    #[must_use]
    pub fn metres_per_unit(&self) -> f64 {
        crate::shell::metres_per_unit(self.radius_earth())
    }

    /// The world this frame draws.
    #[must_use]
    pub fn world(&self) -> WorldOptions {
        self.world
    }

    /// The same frame reading `world` instead of Classic (#685).
    ///
    /// [`crate::nav::Universe::reload`] applies the universe's world, so
    /// the open planet follows Settings > Display > Scene; tests build
    /// Generated frames directly. Classic frames read bit-identical ground
    /// to before.
    #[must_use]
    pub fn with_world(self, world: WorldOptions) -> SurfaceFrame {
        SurfaceFrame { world, ..self }
    }

    /// Radius of the ground under `direction`, in planet units (#685, T3):
    /// the one seam every reader shares.
    ///
    /// Classic is exactly the flat faces ([`surface_radius`]), bit for
    /// bit. Generated adds the pure relief
    /// ([`crate::world::relief::total_relief_m`]: base plus plate
    /// boundaries) times the relief height, in this planet's own metre,
    /// so the mesh corners, the walked ground and the headless check all
    /// read the same numbers (AC3). Pure in `(seed, direction, world)`:
    /// the height never depends on the split depth.
    #[must_use]
    pub fn ground_radius(&self, direction: [f64; 3]) -> f64 {
        let flat = surface_radius(direction);
        if self.world.mode == WorldMode::Classic {
            return flat;
        }
        let radius_m = self.radius_earth() * crate::terrain::EARTH_RADIUS_KM * 1000.0;
        let relief_m = crate::world::relief::total_relief_m(self.planet_seed, direction, radius_m);
        flat + relief_m * self.world.relief.get() / self.metres_per_unit()
    }

    /// Deepest mesh split on this frame's planet.
    #[must_use]
    pub fn max_depth(&self) -> u8 {
        max_depth_for_radius(self.radius_earth())
    }

    /// Height above the ground in metres of `camera` in planet units: the
    /// reach past [`SurfaceFrame::ground_radius`] in this planet's own
    /// metre. Walkers and dives read this; the shell tracker still reads
    /// the flat datum until the Generated wiring lands (T12).
    #[must_use]
    pub fn height_above_ground_m(&self, camera: [f64; 3]) -> f64 {
        let [x, y, z] = camera;
        let reach = (x * x + y * y + z * z).sqrt();
        (reach - self.ground_radius(camera)) * self.metres_per_unit()
    }

    /// [`crate::ground::GROUND_STANDOFF_M`] in cell units on this planet.
    #[must_use]
    pub fn standoff(&self) -> f64 {
        crate::ground::GROUND_STANDOFF_M / self.metres_per_unit()
    }

    /// Icosphere depth of the surface tiles.
    #[must_use]
    pub const fn depth(&self) -> u8 {
        PLANET_DEPTH
    }

    /// The flat-faced surface under `direction`, in planet units (#593).
    ///
    /// Since T3 the corners rest on [`SurfaceFrame::ground_radius`], so a
    /// Generated mesh wears its relief; Classic corners sit on the flat
    /// faces exactly as before.
    #[must_use]
    pub fn surface_point(&self, direction: [f64; 3]) -> [f64; 3] {
        let unit = normalize(direction);
        let radius = self.ground_radius(unit);
        [unit[0] * radius, unit[1] * radius, unit[2] * radius]
    }

    /// Height of the flat-faced surface over the column `(x, z)` (C2/C7).
    ///
    /// The upper surface over the column, solved with a few fixed-point
    /// steps from the sphere through the face corners. Non-finite input, or
    /// a column that misses the planet, returns `0.0`.
    #[must_use]
    pub fn ground_height(&self, x: f64, z: f64) -> f64 {
        if !x.is_finite() || !z.is_finite() {
            return 0.0;
        }
        let reach = PLANET_RADIUS_CELL * PLANET_RADIUS_CELL - (x * x + z * z);
        if reach <= 0.0 {
            return 0.0;
        }
        let mut probe = [x, reach.sqrt(), z];
        let mut height = probe[1];
        for _ in 0..GROUND_STEPS {
            let ground = self.surface_point(probe);
            height = ground[1];
            probe = [
                probe[0] + (x - ground[0]),
                height,
                probe[2] + (z - ground[2]),
            ];
        }
        if height.is_finite() { height } else { 0.0 }
    }

    /// The planet's tiles: the whole sphere at its depth (80 triangles).
    #[must_use]
    pub fn tiles(&self) -> Vec<TriAddr> {
        crate::icosphere::all_at_depth(self.depth())
    }

    /// The planet's [`REGION_PORTALS`] clusters, all enterable, in grow
    /// order (the region portals of L9 planet interiors).
    #[must_use]
    pub fn clusters(&self) -> Vec<SurfaceCluster> {
        let tiles = self.tiles();
        let grown = grow_clusters(self.planet_seed, &tiles, REGION_PORTALS);
        let flags = pick_enterable(self.planet_seed, &grown, REGION_PORTALS as usize);
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
    use crate::nav::DEMO_SEED;

    fn planet() -> SurfaceFrame {
        let chain = crate::nest::autopilot_path(DEMO_SEED);
        SurfaceFrame::for_path(DEMO_SEED, chain.get(..9).expect("planet chain"))
            .expect("planet frame")
    }

    #[test]
    fn only_a_planet_chain_has_a_surface() {
        let chain = crate::nest::autopilot_path(DEMO_SEED);
        assert!(SurfaceFrame::for_path(DEMO_SEED, chain.get(..8).expect("L9")).is_none());
        assert_eq!(planet().level().get(), 10);
        assert_eq!(planet().tiles().len(), 80, "Q2: 80 triangles");
    }

    #[test]
    fn ground_height_sits_on_the_flat_surface() {
        let frame = planet();
        for (x, z) in [(0.0, 0.0), (0.1, -0.05), (-0.12, 0.08)] {
            let y = frame.ground_height(x, z);
            let radius = (x * x + y * y + z * z).sqrt();
            let want = surface_radius([x, y, z]);
            assert!(
                (radius - want).abs() < 1e-9,
                "({x}, {z}): off surface by {}",
                radius - want
            );
        }
        assert_eq!(frame.ground_height(1.0, 1.0), 0.0, "column misses");
        assert_eq!(frame.ground_height(f64::NAN, 0.0), 0.0);
    }

    #[test]
    fn frames_are_deterministic() {
        assert_eq!(planet(), planet());
        assert_eq!(planet().clusters(), planet().clusters());
        assert!(planet().clusters().iter().all(|cluster| cluster.enterable));
    }

    /// A Generated frame at default relief for `seed`.
    fn generated(seed: u64) -> SurfaceFrame {
        SurfaceFrame::planet(seed).with_world(WorldOptions {
            mode: WorldMode::Generated,
            relief: crate::look::ReliefFactor::default(),
        })
    }

    #[test]
    fn classic_ground_is_bit_identical_to_the_flat_faces() {
        // Classic frames (and moon bodies) read exactly the old numbers.
        for frame in [
            SurfaceFrame::planet(7),
            SurfaceFrame::planet(42),
            SurfaceFrame::body(7, 2.0),
            planet(),
        ] {
            assert_eq!(frame.world(), WorldOptions::classic());
            for direction in [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.3, -0.5, 0.8],
                [-0.6, 0.2, -0.4],
            ] {
                assert_eq!(
                    frame.ground_radius(direction),
                    surface_radius(direction),
                    "classic ground moved"
                );
            }
        }
    }

    #[test]
    fn height_is_pure_across_detail_and_ground() {
        // C3 / AC3 (T3 seam): the height at a place never depends on the
        // split depth, and the walked ground equals the drawn ground.
        let frame = generated(7);
        let dirs = [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.3, -0.5, 0.8],
            [-0.6, 0.2, -0.4],
            [0.01, 0.99, 0.02],
        ];
        let mut relief_seen: f64 = 0.0;
        for dir in dirs {
            let ground = frame.ground_radius(dir);
            assert_eq!(ground, frame.ground_radius(dir), "replay differs");
            // Orbit and ground agree: scaling the direction changes nothing
            // (within the flat faces' own 1-ulp datum, as in terrain).
            let far = frame.ground_radius([dir[0] * 10.0, dir[1] * 10.0, dir[2] * 10.0]);
            assert!(
                (ground - far).abs() < 1e-12,
                "orbit and ground disagree: {ground} vs {far}"
            );
            // Drawn ground: the surface point sits on the radius (within
            // the unit-length datum).
            let point = frame.surface_point(dir);
            let reach = (point[0] * point[0] + point[1] * point[1] + point[2] * point[2]).sqrt();
            assert!(
                (reach - ground).abs() < 1e-12,
                "drawn ground left the radius: {reach} vs {ground}"
            );
            relief_seen = relief_seen.max((ground - surface_radius(dir)).abs());
        }
        assert!(
            relief_seen > 100.0 / frame.metres_per_unit(),
            "no relief: {relief_seen}"
        );
        // Relief height scales the relief, not the datum.
        let tall = SurfaceFrame::planet(7).with_world(WorldOptions {
            mode: WorldMode::Generated,
            relief: crate::look::ReliefFactor::of(
                crate::look::ReliefHeight::new(20).expect("in range"),
            ),
        });
        for dir in dirs {
            let base = frame.ground_radius(dir) - surface_radius(dir);
            let doubled = tall.ground_radius(dir) - surface_radius(dir);
            assert!(
                (doubled - 2.0 * base).abs() < 1e-12,
                "relief height does not scale: {base} vs {doubled}"
            );
        }
        // Walked ground: a Generated universe stands on the drawn ground.
        let chain = crate::nest::autopilot_path(DEMO_SEED);
        let mut universe = crate::nav::Universe::new(DEMO_SEED);
        for &marker in chain.get(..9).expect("journey to L10") {
            assert!(universe.open(crate::nav::MarkerIndex(marker)));
        }
        universe.set_world(WorldOptions {
            mode: WorldMode::Generated,
            relief: crate::look::ReliefFactor::default(),
        });
        let open = universe.frame.as_ref().expect("planet frame");
        assert_eq!(open.world().mode, WorldMode::Generated);
        for dir in dirs {
            let walked = crate::body::level_ground(&universe, dir).expect("planet ground");
            let drawn = open.surface_point(dir);
            for (got, want) in walked.point.iter().zip(drawn) {
                assert!((got - want).abs() < 1e-12, "walked left drawn");
            }
        }
    }

    #[test]
    fn big_planets_keep_metre_detail() {
        // C6 / AC6 (the Q1 caveat, T3 part): depth grows with radius so
        // the finest edge stays at about 31-40 cm, in exact `f64`.
        let earth = SurfaceFrame::body(7, 1.0);
        assert_eq!(earth.max_depth(), max_depth_for_radius(1.0));
        let edge_earth = edge_l10(earth.max_depth()) * earth.metres_per_unit();
        assert!(
            (0.2..0.6).contains(&edge_earth),
            "earth finest edge {edge_earth} m"
        );
        assert_eq!(max_depth_for_radius(0.3), earth.max_depth());
        assert_eq!(max_depth_for_radius(f64::NAN), earth.max_depth());
        assert_eq!(max_depth_for_radius(2.0), earth.max_depth() + 1);
        assert_eq!(max_depth_for_radius(4.0), earth.max_depth() + 2);
        assert_eq!(
            max_depth_for_radius(8.0),
            crate::icosphere::MAX_DEPTH,
            "growth stops at the addressable limit"
        );
        // A 2-Earth-radius world splits one level deeper, for the same
        // finest edge in metres.
        let big = SurfaceFrame::body(7, 2.0).with_world(WorldOptions {
            mode: WorldMode::Generated,
            relief: crate::look::ReliefFactor::default(),
        });
        assert_eq!(big.radius_earth(), 2.0);
        assert_eq!(big.max_depth(), earth.max_depth() + 1);
        let edge_big = edge_l10(big.max_depth()) * big.metres_per_unit();
        assert!(
            (edge_big - edge_earth).abs() < 0.05,
            "big finest edge {edge_big} m vs {edge_earth} m"
        );
        // Near the ground the heights stay exact: finite and replayable
        // across a dense sweep, including the poles.
        for i in 0..2_000 {
            let step = f64::from(i);
            let y = 1.0 - 2.0 * (step + 0.5) / 2_000.0;
            let r = (1.0 - y * y).max(0.0).sqrt();
            let angle = step * 2.399_963_229_728_653;
            let dir = [r * angle.cos(), y, r * angle.sin()];
            let h = big.ground_radius(dir);
            assert!(h.is_finite(), "non-finite ground at {dir:?}");
            assert_eq!(h, big.ground_radius(dir), "replay differs");
        }
    }
}
