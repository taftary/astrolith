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

/// A planet's orbit context: where it circles and which system ages it
/// (#685, T7).
///
/// The ladder already knows every orbit (L8/L9 planet points carry their
/// [`crate::gen::PlanetInfo`]); the frame re-reads its own instead of
/// re-drawing one, so a planet's climate matches its distance from its
/// star. Climates, air and colours read this; relief never needs it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitContext {
    /// Semi-major axis in AU.
    pub orbit_au: f64,
    /// Orbital period in days.
    pub period_days: f64,
    /// Seed aging the whole system.
    pub system_seed: u64,
    /// Whether this is the home system (the Sun exactly).
    pub home: bool,
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
    /// Orbit context re-read from the ladder, if the planet was reached
    /// through its system (`None` for tool-built frames).
    orbit: Option<OrbitContext>,
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
            orbit: None,
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
            orbit: None,
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
            orbit: orbit_context_for_path(root, chain),
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

    /// The planet's radius in Earth radii: the body's record
    /// ([`crate::terrain::body_recipe`]), except on the home planet in a
    /// Generated frame with an orbit context, which runs the generator
    /// (T12, Q4): the recipe draws its own mass and radius from the seed
    /// inside the Solar orbits, so the home planet may no longer be
    /// Earth-like. Other Generated planets keep their record sizes until
    /// the sampling rewire unifies them; tool-built frames without an
    /// orbit keep the record too.
    #[must_use]
    pub fn radius_earth(&self) -> f64 {
        if let Some(bits) = self.radius_bits {
            return f64::from_bits(bits);
        }
        if self.world.mode == WorldMode::Generated
            && self.home
            && let Some(orbit) = self.orbit
        {
            let info = crate::r#gen::PlanetInfo {
                radius_earth: crate::terrain::body_recipe(self.planet_seed, self.home).radius_earth,
                orbit_au: orbit.orbit_au,
                period_days: orbit.period_days,
            };
            let recipe =
                crate::world::planet_recipe(self.planet_seed, &info, orbit.system_seed, orbit.home);
            if recipe.radius_earth.is_finite() && recipe.radius_earth > 0.0 {
                return recipe.radius_earth;
            }
        }
        crate::terrain::body_recipe(self.planet_seed, self.home).radius_earth
    }

    /// Whether this is the home planet (its Generated size comes from the
    /// recipe; the render draws the generator's moons for it).
    #[must_use]
    pub const fn home(&self) -> bool {
        self.home
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

    /// The orbit context re-read from the ladder, if any.
    #[must_use]
    pub fn orbit(&self) -> Option<OrbitContext> {
        self.orbit
    }

    /// The same frame with an explicit orbit context (tests and tools
    /// that build frames without a path).
    #[must_use]
    pub fn with_orbit(self, orbit: OrbitContext) -> SurfaceFrame {
        SurfaceFrame {
            orbit: Some(orbit),
            ..self
        }
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
    /// the height never depends on the split depth. Since T8 the relief
    /// reads the planet's own type from its orbit context (Earth-like
    /// without one); since T11 its system age too (mature without one).
    ///
    /// Spec v3: this reads full micro detail (walk/collision). Drawn
    /// ground reads [`SurfaceFrame::ground_radius_lod`] at the view LOD.
    #[must_use]
    pub fn ground_radius(&self, direction: [f64; 3]) -> f64 {
        self.ground_radius_lod(direction, crate::world::relief::MAX_EXTRA_DETAIL)
    }

    /// [`SurfaceFrame::ground_radius`] at `lod` extra micro octaves
    /// (Spec v3 dynamic height): `0` is the Spec v2 base exactly, full is
    /// [`SurfaceFrame::ground_radius`]. Classic ignores `lod` and reads
    /// the flat faces bit-for-bit. Pure in `(seed, direction, world, lod)`.
    #[must_use]
    pub fn ground_radius_lod(&self, direction: [f64; 3], lod: f64) -> f64 {
        let flat = surface_radius(direction);
        if self.world.mode == WorldMode::Classic {
            return flat;
        }
        let radius_m = self.radius_earth() * crate::terrain::EARTH_RADIUS_KM * 1000.0;
        let (kind, age) = self.orbit.map_or(
            (
                crate::world::WorldKind::EarthLike,
                crate::world::lifetime::MATURE_AGE_GYR,
            ),
            |orbit| {
                let info = crate::r#gen::PlanetInfo {
                    radius_earth: self.radius_earth(),
                    orbit_au: orbit.orbit_au,
                    period_days: orbit.period_days,
                };
                let recipe = crate::world::planet_recipe(
                    self.planet_seed,
                    &info,
                    orbit.system_seed,
                    orbit.home,
                );
                (recipe.kind, recipe.age_gyr)
            },
        );
        let relief_m = crate::world::relief::total_relief_m_lod(
            self.planet_seed,
            direction,
            radius_m,
            Some(kind),
            Some(age),
            lod,
        );
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

/// Orbit context of the L10 chain `chain` under `root`: the planet data
/// attached to the ladder point the journey opened, plus the seed aging
/// its system.
///
/// Through the star (marker 0 in L8) the L9 close-up carries the planet
/// points with their data, aged by the L9 cell seed; through a planet
/// marker the L8 system view carries them, aged by its shared system
/// seed. Anything else (out of range, a population, a regenerated
/// mismatch) reads `None`, and colours fall back to grey.
fn orbit_context_for_path(root: u64, chain: &[u32]) -> Option<OrbitContext> {
    let home = crate::home::is_home_planet_cell(root, chain);
    let star_marker = chain.get(7).copied()?;
    if star_marker == 0 {
        let closeup = crate::nest::generate_cell(root, chain.get(..8)?);
        let marker = chain.get(8).copied()?;
        let planet = closeup.points.get(marker as usize)?.planet?;
        return Some(OrbitContext {
            orbit_au: planet.orbit_au,
            period_days: planet.period_days,
            system_seed: crate::nest::path_seed(root, chain.get(..8)?),
            home,
        });
    }
    let system = crate::nest::generate_cell(root, chain.get(..7)?);
    let planet = system.points.get(star_marker as usize)?.planet?;
    Some(OrbitContext {
        orbit_au: planet.orbit_au,
        period_days: planet.period_days,
        system_seed: crate::seed::hash_cell(
            crate::nest::path_seed(root, chain.get(..7)?),
            9,
            0,
            0,
            0,
        ),
        home,
    })
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

    #[test]
    fn orbit_context_resolves_on_the_journey() {
        // T7: the frame re-reads its own orbit from the ladder, so a
        // planet's climate matches its distance from its star.
        let frame = planet();
        let ctx = frame.orbit().expect("journey planet has an orbit");
        assert!(ctx.orbit_au > 0.0 && ctx.period_days > 0.0, "{ctx:?}");
        assert_eq!(frame.orbit(), frame.orbit(), "replay differs");
        assert!(frame.home, "the demo journey ends home");
        // Tool-built frames carry no orbit; colours fall back to grey.
        assert_eq!(SurfaceFrame::planet(7).orbit(), None);
        assert_eq!(SurfaceFrame::body(7, 1.0).orbit(), None);
        // End to end: the journey frame colours deterministically.
        let coloured = frame.with_world(WorldOptions {
            mode: WorldMode::Generated,
            relief: crate::look::ReliefFactor::default(),
        });
        let dir = [0.3, 0.8, 0.5];
        let first = crate::world::biome::ground_colour(&coloured, dir, 100.0);
        assert!(first.iter().all(|c| c.is_finite()), "{first:?}");
        assert_eq!(
            first,
            crate::world::biome::ground_colour(&coloured, dir, 100.0)
        );
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
    fn the_home_planet_is_generated_and_reached() {
        // C14 / AC14 (Generated part, T12, Q4): the autopilot journey opens
        // to L10 on the home planet in both worlds and the Solar orbits do
        // not move; in Classic the home frame wears the Earth fixture, in
        // Generated it runs the generator (recipe radius, kind relief and
        // world moons).
        let chain = crate::nest::autopilot_path(DEMO_SEED);
        let generated_world = WorldOptions {
            mode: WorldMode::Generated,
            relief: crate::look::ReliefFactor::default(),
        };
        // Classic: Earth fixture, flat ground.
        let mut classic = crate::nav::Universe::new(DEMO_SEED);
        for &marker in chain.get(..9).expect("journey to L10") {
            assert!(
                classic.open(crate::nav::MarkerIndex(marker)),
                "classic stalls"
            );
        }
        assert_eq!(classic.level().get(), 10);
        let classic_frame = classic.frame.as_ref().expect("planet frame");
        assert!(classic_frame.home, "the journey ends elsewhere");
        assert_eq!(classic_frame.radius_earth(), 1.0, "fixture moved");
        // Generated: the same markers arrive at the same home planet.
        let mut generated = crate::nav::Universe::new(DEMO_SEED);
        generated.set_world(generated_world);
        for &marker in chain.get(..9).expect("journey to L10") {
            assert!(
                generated.open(crate::nav::MarkerIndex(marker)),
                "generated stalls"
            );
        }
        assert_eq!(generated.level().get(), 10);
        let frame = generated.frame.as_ref().expect("planet frame");
        assert!(frame.home, "the journey ends elsewhere");
        assert_eq!(frame.world().mode, WorldMode::Generated);
        let classic_orbit = classic_frame.orbit().expect("classic orbit");
        let orbit = frame.orbit().expect("generated orbit");
        assert_eq!(
            (orbit.orbit_au, orbit.period_days),
            (classic_orbit.orbit_au, classic_orbit.period_days),
            "the Solar orbits moved"
        );
        // ...but the planet runs the generator, not the fixture.
        let info = crate::r#gen::PlanetInfo {
            radius_earth: 1.0,
            orbit_au: orbit.orbit_au,
            period_days: orbit.period_days,
        };
        let recipe =
            crate::world::planet_recipe(frame.planet_seed(), &info, orbit.system_seed, orbit.home);
        assert_eq!(
            frame.radius_earth(),
            recipe.radius_earth,
            "size not generated"
        );
        assert!(
            frame.radius_earth() != 1.0,
            "home still Earth-sized: {}",
            recipe.kind.label()
        );
        let mut relief_seen = 0.0f64;
        for dir in [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.3, -0.5, 0.8],
            [-0.6, 0.2, -0.4],
        ] {
            relief_seen = relief_seen.max((frame.ground_radius(dir) - surface_radius(dir)).abs());
        }
        assert!(
            relief_seen > 100.0 / frame.metres_per_unit(),
            "home has no generated relief"
        );
        // Moons from the generator, not the one-Moon fixture.
        let body = crate::terrain::body_recipe(frame.planet_seed(), true);
        let system = crate::world::moons::system_for(
            frame.planet_seed(),
            &crate::world::moons::PlanetSpec {
                mass_earth: recipe.mass_earth,
                radius_earth: recipe.radius_earth,
                kind: recipe.kind,
                orbit_au: recipe.orbit_au,
                star_mass_solar: recipe.star_mass,
                age_gyr: recipe.age_gyr,
                luminosity: recipe.luminosity,
                spin_hours: body.spin_hours,
            },
        );
        assert!(system.moons.len() > 1, "home keeps the one-Moon fixture");
        assert_eq!(
            system,
            crate::world::moons::system_for(
                frame.planet_seed(),
                &crate::world::moons::PlanetSpec {
                    mass_earth: recipe.mass_earth,
                    radius_earth: recipe.radius_earth,
                    kind: recipe.kind,
                    orbit_au: recipe.orbit_au,
                    star_mass_solar: recipe.star_mass,
                    age_gyr: recipe.age_gyr,
                    luminosity: recipe.luminosity,
                    spin_hours: body.spin_hours,
                },
            ),
            "moon replay differs"
        );
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

    #[test]
    fn walked_ground_equals_full_detail_drawn() {
        // Spec v3 C3: walk/collision reads full detail; drawn at full LOD
        // matches. Classic ignores LOD bit-for-bit.
        let frame = SurfaceFrame::planet(7).with_world(WorldOptions {
            mode: WorldMode::Generated,
            relief: crate::look::ReliefFactor::default(),
        });
        let dir = [0.3, -0.5, 0.8];
        let full = frame.ground_radius(dir);
        let lod = frame.ground_radius_lod(dir, crate::world::relief::MAX_EXTRA_DETAIL);
        assert!((full - lod).abs() < 1e-12, "full must equal lod max");
        assert_eq!(
            lod,
            frame.ground_radius_lod(dir, crate::world::relief::MAX_EXTRA_DETAIL)
        );
        // Walked ground (body level_ground) sits on the drawn point.
        let chain = crate::nest::autopilot_path(DEMO_SEED);
        let mut universe = crate::nav::Universe::new(DEMO_SEED);
        for &marker in chain.get(..9).expect("journey to L10") {
            assert!(universe.open(crate::nav::MarkerIndex(marker)));
        }
        universe.set_world(WorldOptions {
            mode: WorldMode::Generated,
            relief: crate::look::ReliefFactor::default(),
        });
        let walked = crate::body::level_ground(&universe, dir).expect("planet ground");
        let drawn = universe.frame.as_ref().expect("frame").surface_point(dir);
        for (got, want) in walked.point.iter().zip(drawn) {
            assert!((got - want).abs() < 1e-12, "walked left drawn");
        }
        // Classic: LOD ignored exactly.
        let classic = SurfaceFrame::planet(7);
        for dir in [[1.0, 0.0, 0.0], [0.3, -0.5, 0.8]] {
            assert_eq!(
                classic.ground_radius(dir),
                classic.ground_radius_lod(dir, 0.0),
                "classic must ignore lod"
            );
            assert_eq!(
                classic.ground_radius(dir),
                classic.ground_radius_lod(dir, crate::world::relief::MAX_EXTRA_DETAIL),
                "classic must ignore lod"
            );
        }
    }

    #[test]
    fn shells_read_relief_ground_with_order() {
        // Spec v3 C4: shells from the relief ground; L14 below 10 m, order
        // kept. Tool frame (home=false) keeps record radius on both sides,
        // avoiding the home recipe/record size split staged for the
        // sampler rewire (T12 follow-up, out of scope here).
        let frame = SurfaceFrame::planet(7).with_world(WorldOptions {
            mode: WorldMode::Generated,
            relief: crate::look::ReliefFactor::default(),
        });
        let radius = frame.radius_earth();
        let mpu = frame.metres_per_unit();
        let mut found = false;
        for i in 0..500u32 {
            let step = f64::from(i);
            let y = 1.0 - 2.0 * (step + 0.5) / 500.0;
            let r = (1.0 - y * y).max(0.0).sqrt();
            let angle = step * 2.399_963_229_728_653;
            let dir = [r * angle.cos(), y, r * angle.sin()];
            let unit = normalize(dir);
            let ground = frame.ground_radius(unit);
            let relief_m = (ground - surface_radius(unit)) * mpu;
            if relief_m > 50.0 {
                // 5 m above the relief ground reads L14 from the
                // relief-aware height; the flat datum at the same point
                // reads relief+5 m (L13+), proving shells must use relief.
                let rest = [
                    unit[0] * (ground + 5.0 / mpu),
                    unit[1] * (ground + 5.0 / mpu),
                    unit[2] * (ground + 5.0 / mpu),
                ];
                let height = frame.height_above_ground_m(rest);
                assert!((height - 5.0).abs() < 1e-6, "rest height {height} m");
                let shell = crate::shell::ShellLevel::from_height(height, radius);
                assert_eq!(shell.level().get(), 14, "relief rest reads L14");
                let flat = crate::shell::height_above_ground_m(rest, radius);
                assert!(
                    (flat - (relief_m + 5.0)).abs() < 1e-6,
                    "flat height {flat} m"
                );
                assert_ne!(
                    crate::shell::ShellLevel::from_height(flat, radius)
                        .level()
                        .get(),
                    14,
                    "flat datum would leave L14"
                );
                found = true;
                break;
            }
        }
        assert!(found, "no high-relief direction found for shell check");
        // L14 always 10 m; order kept on tiny planets.
        for radius in [0.1, 0.5, 1.0, 3.0, 11.0] {
            assert_eq!(
                crate::shell::enter_height_m(Level::new(14).expect("L14"), radius),
                Some(crate::shell::ROOM_SHELL_M),
                "L14 always 10 m"
            );
        }
        let tiny = 1.0e-4;
        let mut previous = f64::INFINITY;
        for n in 11..=14u8 {
            let enter =
                crate::shell::enter_height_m(Level::new(n).expect("shell"), tiny).expect("shell");
            assert!(enter < previous, "L{n} below L{}", n - 1);
            previous = enter;
        }
    }
}
