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
use crate::terrain::{PLANET_RADIUS_CELL, REGION_PORTALS, surface_radius};

/// Fixed-point steps of [`SurfaceFrame::ground_height`] (converges to
/// about 1e-9 cell units on the steepest L10 columns the tests probe).
const GROUND_STEPS: usize = 24;

/// Icosphere depth of the planet surface (Q2: one split, 80 triangles).
pub const PLANET_DEPTH: u8 = 1;

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

/// The open planet's surface (L10, #458, #632).
///
/// Holds the planet seed and whether it is the home planet (its size,
/// #648). Answers the flat-faced surface under a
/// direction, the ground height over a column, and grows the planet's
/// clusters. Pure and deterministic.
#[derive(Clone, Debug, PartialEq, Eq)]
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
}

impl SurfaceFrame {
    /// The open L10 planet cell for `planet_seed` (not the home planet).
    #[must_use]
    pub const fn planet(planet_seed: u64) -> SurfaceFrame {
        SurfaceFrame {
            planet_seed,
            home: false,
            radius_bits: None,
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
    #[must_use]
    pub fn surface_point(&self, direction: [f64; 3]) -> [f64; 3] {
        let unit = normalize(direction);
        let radius = surface_radius(unit);
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
}
