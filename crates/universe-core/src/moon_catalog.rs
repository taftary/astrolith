//! Read-only identities and catalog records for existing generated moons (#1238).
//!
//! This module never changes generation, opens a new navigation level or supplies
//! collision ground. A catalog reference is not an observed terrain measurement.

use crate::nav::{MarkerIndex, Seed, Universe};
use crate::world_type::{WorldType, classify_moon, list_worlds, planet_inputs};

/// Largest bounded catalog search, in parent planets.
pub const CATALOG_PLANETS: usize = 4_000;
/// Maximum retained moon records in one catalog request.
pub const CATALOG_MOONS: usize = 64_000;
/// Inclusive lower bound of the current procedural radius draw, in kilometres.
pub const SAMPLED_RADIUS_MIN_KM: f64 = 200.0;
/// Exclusive upper bound of the current procedural radius draw, in kilometres.
pub const SAMPLED_RADIUS_END_KM: f64 = 1_700.0;

/// A malformed identity or an absent generated object, never a substituted moon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogError {
    /// A planet identity needs exactly nine marker indices.
    PlanetDepth,
    /// A prefix is too deep or contains a marker that cannot be opened.
    InvalidPath,
    /// The existing planet has no classification inputs.
    MissingPlanet,
    /// The marker does not identify a moon in that planet cell.
    MissingMoon,
    /// A requested catalog bound is zero or exceeds its declared ceiling.
    SearchLimit,
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::PlanetDepth => "planet identity needs nine marker indices",
            Self::InvalidPath => "not an existing portal prefix",
            Self::MissingPlanet => "planet classification inputs are absent",
            Self::MissingMoon => "marker does not identify an existing moon",
            Self::SearchLimit => "catalog search exceeds its nonzero bounds",
        })
    }
}

impl std::error::Error for CatalogError {}

fn validate_prefix(seed: Seed, chain: &[u32]) -> Result<(), CatalogError> {
    if chain.len() > 9 {
        return Err(CatalogError::InvalidPath);
    }
    let mut universe = Universe::new(seed.0);
    for &marker in chain {
        if !universe.open(MarkerIndex(marker)) {
            return Err(CatalogError::InvalidPath);
        }
    }
    Ok(())
}

/// A validated permanent generated L10 planet identity, not a camera position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlanetIdentity {
    seed: Seed,
    chain: [u32; 9],
}

impl PlanetIdentity {
    /// Labels an already-open production planet without regenerating any cell.
    ///
    /// # Errors
    /// Refuses a different level, missing surface record or malformed open chain.
    pub fn from_open_cell(universe: &Universe) -> Result<Self, CatalogError> {
        let chain: [u32; 9] = universe
            .path
            .indices()
            .try_into()
            .map_err(|_| CatalogError::PlanetDepth)?;
        if universe.level().get() != 10 || !universe.open.points.iter().any(|p| p.surface.is_some())
        {
            return Err(CatalogError::MissingPlanet);
        }
        Ok(Self {
            seed: universe.root,
            chain,
        })
    }

    /// Validates every portal and the existing planet's classification context.
    ///
    /// # Errors
    /// Refuses wrong depth, invalid portals and absent planet inputs.
    pub fn new(seed: Seed, chain: &[u32]) -> Result<Self, CatalogError> {
        let chain: [u32; 9] = chain.try_into().map_err(|_| CatalogError::PlanetDepth)?;
        validate_prefix(seed, &chain)?;
        planet_inputs(seed.0, &chain).ok_or(CatalogError::MissingPlanet)?;
        Ok(Self { seed, chain })
    }

    /// Universe seed that scopes the marker chain.
    #[must_use]
    pub const fn seed(self) -> Seed {
        self.seed
    }

    /// Full validated L10 chain.
    #[must_use]
    pub const fn chain(&self) -> &[u32; 9] {
        &self.chain
    }

    /// Containing L8 system chain, distinct from the host planet identity.
    #[must_use]
    pub fn system(&self) -> &[u32] {
        self.chain.get(..7).unwrap_or(&[])
    }
}

/// A permanent moon marker within a validated parent planet and universe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoonIdentity {
    planet: PlanetIdentity,
    marker: MarkerIndex,
}

impl MoonIdentity {
    /// Identifies an actual generated moon, refusing a different marker kind.
    ///
    /// # Errors
    /// Returns `MissingMoon` when the marker is absent or is not a moon.
    pub fn new(planet: PlanetIdentity, marker: MarkerIndex) -> Result<Self, CatalogError> {
        let cell = crate::nest::generate_cell(planet.seed.0, &planet.chain);
        let index = usize::try_from(marker.0).map_err(|_| CatalogError::MissingMoon)?;
        cell.points
            .get(index)
            .and_then(|point| point.moon)
            .ok_or(CatalogError::MissingMoon)?;
        Ok(Self { planet, marker })
    }

    /// Permanent host-planet identity.
    #[must_use]
    pub const fn planet(self) -> PlanetIdentity {
        self.planet
    }

    /// Existing generated moon marker, never a new selection/travel target.
    #[must_use]
    pub const fn marker(self) -> MarkerIndex {
        self.marker
    }
}

/// Instance origin is separate from the evidence for this kind of object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoonOrigin {
    /// Home reference with supplied catalog-based radius/orbit/period inputs.
    CatalogBased,
    /// Deterministic procedural instance, not a claimed catalog observation.
    Procedural,
}

impl MoonOrigin {
    /// Stable developer-record value.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::CatalogBased => "catalog-based",
            Self::Procedural => "procedural",
        }
    }
}

/// Separate evidence status: moons are observed kinds, not measured instances here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoonEvidence {
    /// This kind of astronomical object is observed.
    ObservedKind,
}

/// One unmodified generated moon and the provenance of its supplied properties.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ListedMoon {
    /// Universe/host/moon identity, distinct from illustrative fixture placement.
    pub identity: MoonIdentity,
    /// Exactly the existing generator's kilometre/day record.
    pub info: crate::r#gen::MoonInfo,
    /// Existing production paint classifier's result.
    pub kind: WorldType,
    /// Same stable seed the production moon painter uses.
    pub paint_seed: u64,
    /// Evidence for the object kind, not a fabricated instance measurement.
    pub evidence: MoonEvidence,
    /// Catalog-based home inputs or a generated instance.
    pub origin: MoonOrigin,
}

impl ListedMoon {
    /// Home terrain is synthetic; other instance terrain is procedural.
    #[must_use]
    pub const fn terrain_origin(self) -> &'static str {
        match self.origin {
            MoonOrigin::CatalogBased => "synthetic",
            MoonOrigin::Procedural => "procedural",
        }
    }

    /// Supplied versus generated properties; neither claims a new measurement.
    #[must_use]
    pub const fn property_origin(self) -> &'static str {
        match self.origin {
            MoonOrigin::CatalogBased => "supplied-reference-not-measured",
            MoonOrigin::Procedural => "generated-not-measured",
        }
    }

    /// Repository source for the inputs actually used, not an invented citation.
    #[must_use]
    pub const fn property_source(self) -> &'static str {
        match self.origin {
            MoonOrigin::CatalogBased => "terrain.rs:MOON_RADIUS_KM,MOON_ORBIT_KM,MOON_PERIOD_DAYS",
            MoonOrigin::Procedural => "terrain.rs:TerrainSampler::generate/MOON_STREAM_TAG",
        }
    }
}

/// Reads all actual moons of one validated planet without changing its universe.
///
/// # Errors
/// Refuses absent planet inputs rather than using a default classification.
pub fn moons_for_planet(planet: PlanetIdentity) -> Result<Vec<ListedMoon>, CatalogError> {
    let inputs = planet_inputs(planet.seed.0, &planet.chain).ok_or(CatalogError::MissingPlanet)?;
    let cell = crate::nest::generate_cell(planet.seed.0, &planet.chain);
    let seed = crate::nest::path_seed(planet.seed.0, &planet.chain);
    let mut out = Vec::new();
    for (marker, point) in (0u32..).zip(&cell.points) {
        let Some(info) = point.moon else {
            continue;
        };
        let paint_seed = crate::seed::hash_triple(seed, u64::from(marker), 0x1161);
        out.push(ListedMoon {
            identity: MoonIdentity {
                planet,
                marker: MarkerIndex(marker),
            },
            info,
            kind: classify_moon(&inputs, paint_seed, info.radius_km / 6_371.0),
            paint_seed,
            evidence: MoonEvidence::ObservedKind,
            origin: if inputs.home {
                MoonOrigin::CatalogBased
            } else {
                MoonOrigin::Procedural
            },
        });
    }
    Ok(out)
}

/// Reads one validated identity using the current production generator/classifier.
///
/// # Errors
/// Refuses missing planet/moon records instead of substituting another body.
pub fn moon_record(identity: MoonIdentity) -> Result<ListedMoon, CatalogError> {
    moons_for_planet(identity.planet)?
        .into_iter()
        .find(|moon| moon.identity == identity)
        .ok_or(CatalogError::MissingMoon)
}

/// Bounded, depth-first catalog under an existing portal prefix.
///
/// # Errors
/// Refuses invalid prefixes/limits and an inconsistent generated planet record.
/// A valid search with no moons succeeds with an empty list, not an invented moon.
pub fn list_moons(
    seed: Seed,
    start: &[u32],
    planet_limit: usize,
    moon_limit: usize,
) -> Result<Vec<ListedMoon>, CatalogError> {
    if !(1..=CATALOG_PLANETS).contains(&planet_limit) || !(1..=CATALOG_MOONS).contains(&moon_limit)
    {
        return Err(CatalogError::SearchLimit);
    }
    validate_prefix(seed, start)?;
    let mut out = Vec::new();
    let planets = if start.len() == 9 {
        vec![PlanetIdentity::new(seed, start)?]
    } else {
        list_worlds(seed.0, start, planet_limit)
            .into_iter()
            .map(|world| PlanetIdentity::new(seed, &world.chain))
            .collect::<Result<Vec<_>, _>>()?
    };
    for planet in planets {
        for moon in moons_for_planet(planet)? {
            out.push(moon);
            if out.len() == moon_limit {
                return Ok(out);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> PlanetIdentity {
        PlanetIdentity::new(Seed(42), &crate::nest::autopilot_path(42)).unwrap()
    }

    #[test]
    fn home_catalog_preserves_supplied_inputs_and_separates_their_origin() {
        let planet = home();
        let before = crate::nest::generate_cell(42, planet.chain());
        let moons = moons_for_planet(planet).unwrap();
        assert_eq!(moons.len(), 1);
        let moon = moons[0];
        assert_eq!(moon.origin, MoonOrigin::CatalogBased);
        assert_eq!(moon.evidence, MoonEvidence::ObservedKind);
        assert_eq!(moon.kind, WorldType::Barren);
        assert_eq!(moon.info.radius_km, crate::terrain::MOON_RADIUS_KM);
        assert_eq!(moon.info.orbit_km, crate::terrain::MOON_ORBIT_KM);
        assert_eq!(moon.info.period_days, crate::terrain::MOON_PERIOD_DAYS);
        assert_eq!(moon.terrain_origin(), "synthetic");
        assert_eq!(moon.property_origin(), "supplied-reference-not-measured");
        assert_eq!(moon_record(moon.identity).unwrap(), moon);
        let after = crate::nest::generate_cell(42, planet.chain());
        assert_eq!(
            crate::snapshot::snapshot_generated(&before),
            crate::snapshot::snapshot_generated(&after)
        );
    }

    #[test]
    fn invalid_identity_never_substitutes_a_different_planet_or_moon() {
        assert_eq!(
            PlanetIdentity::new(Seed(42), &[7]),
            Err(CatalogError::PlanetDepth)
        );
        assert_eq!(
            PlanetIdentity::new(Seed(42), &[u32::MAX; 9]),
            Err(CatalogError::InvalidPath)
        );
        assert_eq!(
            MoonIdentity::new(home(), MarkerIndex(u32::MAX)),
            Err(CatalogError::MissingMoon)
        );
        assert_eq!(
            MoonIdentity::new(home(), MarkerIndex(0)),
            Err(CatalogError::MissingMoon)
        );
        assert_eq!(
            list_moons(Seed(42), &[u32::MAX], 1, 1),
            Err(CatalogError::InvalidPath)
        );
        assert_eq!(
            list_moons(Seed(42), &[], 0, 1),
            Err(CatalogError::SearchLimit)
        );
    }

    #[test]
    fn bounded_listing_is_repeatable_and_matches_generated_moon_records() {
        let planet = home();
        let first = list_moons(Seed(42), planet.system(), 16, 64).unwrap();
        assert!(!first.is_empty());
        assert_eq!(
            first,
            list_moons(Seed(42), planet.system(), 16, 64).unwrap()
        );
        for moon in &first {
            let host = moon.identity.planet();
            let cell = crate::nest::generate_cell(42, host.chain());
            let point = &cell.points[moon.identity.marker().0 as usize];
            assert_eq!(Some(moon.info), point.moon);
            if moon.origin == MoonOrigin::Procedural {
                assert!(
                    (SAMPLED_RADIUS_MIN_KM..SAMPLED_RADIUS_END_KM).contains(&moon.info.radius_km)
                );
            }
        }
        assert_eq!(
            list_moons(Seed(42), planet.system(), 16, 1).unwrap(),
            first[..1]
        );
    }

    #[test]
    fn exact_radius_boundary_is_data_not_a_claim_about_a_finite_search() {
        let boundary = crate::terrain::moon_info(SAMPLED_RADIUS_MIN_KM, 150_000.0, 1.0);
        assert_eq!(boundary.radius_km, 200.0);
        assert_eq!(SAMPLED_RADIUS_END_KM, 1_700.0);
        // Not a ListedMoon: boundary-only data has no fabricated generated identity.
        assert!(boundary.radius_km < crate::terrain::MOON_RADIUS_KM);
    }
}
