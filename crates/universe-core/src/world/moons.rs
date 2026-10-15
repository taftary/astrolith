//! Moon systems: origins, limits, orbits, budgets, shapes and types
//! (#685, T10).
//!
//! Guides: `guides/universe/L10/generation/documents/moons.md` (Hill and
//! Roche limits, counts by planet, the `10^-4` disk budget, the potato
//! radius, the type table) and `moon-dynamics.md` (impact/disk/capture/
//! breakup origins, the Io-normalised heating score, 2:1 and Laplace
//! resonances with forced eccentricity, binaries by centre of mass,
//! Trojan and horseshoe co-orbitals). Locking, drift and decay at the age
//! belong to T11; this module places the orbits and types that lifetime
//! then ages. Everything is `f64` and deterministic (`E-DET-TIERS`): the
//! same planet seed always gives the same moons.

use super::air::{EARTH_MASS_KG, GRAVITY_G};
use super::classify::WorldKind;
use crate::seed::{Rng, hash_triple};

/// Stream tag of the moon-system draws.
const MOON_TAG: u64 = 0xA00A_0685_0000_0000;

/// Astronomical unit in km.
const AU_KM: f64 = 149_600_000.0;

/// Seconds per day.
const DAY_S: f64 = 86_400.0;

/// Rock density in kg/m^3 (inner moons, impact moons).
const ROCK_DENSITY: f64 = 3_000.0;

/// Ice density in kg/m^3 (outer moons).
const ICE_DENSITY: f64 = 1_500.0;

/// Icy below this density for the potato radius (kg/m^3).
pub(crate) const ICY_DENSITY: f64 = 2_000.0;

/// Potato radius of rocky moons in km (round above).
const POTATO_ROCK_KM: f64 = 300.0;

/// Potato radius of icy moons in km.
const POTATO_ICE_KM: f64 = 200.0;

/// Disk budget: regular moons total this share of a giant's mass.
const DISK_BUDGET_SHARE: f64 = 1e-4;

/// Budget scatter in dex either way.
const DISK_BUDGET_DEX: f64 = 0.3;

/// Neighbouring regular orbits differ by at least this factor.
const MIN_SPACING: f64 = 1.3;

/// Regular moons stay inside this share of the Hill radius.
const REGULAR_HILL_SHARE: f64 = 0.05;

/// Irregular moons stay inside this share of the Hill radius.
const IRREGULAR_HILL_SHARE: f64 = 0.45;

/// Fallback outer cap in km when the Hill sphere is unbounded (rogues).
const HILL_CAP_KM: f64 = 3_000_000.0;

/// Heating score for a volcanic or ocean surface (Io = 1).
const HOT_SCORE: f64 = 0.1;

/// Heating score for a cracked icy crust.
const CRACKED_SCORE: f64 = 0.01;

/// Hazy moons are larger than this radius in km (Titan-sized).
const HAZY_RADIUS_KM: f64 = 2_000.0;

/// Small regulars below this radius in km are shepherds.
const SHEPHERD_RADIUS_KM: f64 = 120.0;

/// Ringed moons are smaller than this radius in km.
const RINGED_RADIUS_KM: f64 = 250.0;

/// Ringed moons orbit past this Hill share.
const RINGED_HILL_SHARE: f64 = 0.3;

/// Ringed draw rate among qualifying far-out small moons.
const RINGED_RATE: f64 = 0.25;

/// A moon's origin (moon-dynamics guide).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoonOrigin {
    /// Debris of a giant impact (big rocky moons).
    Impact,
    /// Grown in the planet's disk (regulars).
    Disk,
    /// Caught passing body (irregulars).
    Capture,
    /// Debris of a breakup (inner smalls, co-orbitals).
    Breakup,
}

/// A moon's type (moons guide, type table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoonKind {
    /// Grey, cratered, geologically dead.
    Dead,
    /// Tidally heated rock: calderas, lava, sulfur plains.
    Volcanic,
    /// Tidally heated ice: ridges, cracks, maybe plumes.
    Ocean,
    /// Big cold moon under opaque orange haze.
    Hazy,
    /// Dark cratered captured irregular.
    Captured,
    /// Small regular near the rings, often ridged.
    Shepherd,
    /// Small far-out moon with its own thin ring.
    Ringed,
}

impl MoonKind {
    /// Every kind, for the coverage test.
    pub const ALL: [MoonKind; 7] = [
        MoonKind::Dead,
        MoonKind::Volcanic,
        MoonKind::Ocean,
        MoonKind::Hazy,
        MoonKind::Captured,
        MoonKind::Shepherd,
        MoonKind::Ringed,
    ];

    /// Its name in readouts.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            MoonKind::Dead => "Dead moon",
            MoonKind::Volcanic => "Volcanic moon",
            MoonKind::Ocean => "Ocean moon",
            MoonKind::Hazy => "Hazy moon",
            MoonKind::Captured => "Captured moon",
            MoonKind::Shepherd => "Shepherd moon",
            MoonKind::Ringed => "Ringed moon",
        }
    }

    /// Its file-safe slug for the type gallery.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            MoonKind::Dead => "dead-moon",
            MoonKind::Volcanic => "volcanic-moon",
            MoonKind::Ocean => "ocean-moon",
            MoonKind::Hazy => "hazy-moon",
            MoonKind::Captured => "captured-moon",
            MoonKind::Shepherd => "shepherd-moon",
            MoonKind::Ringed => "ringed-moon",
        }
    }
}

/// A resonance with the inner neighbour (moon-dynamics guide).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resonance {
    /// Period ratio 2:1 (outer snaps to `2^(2/3)` the inner orbit).
    TwoOne,
    /// Period ratio 3:2.
    ThreeTwo,
}

/// One moon of a system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Moon {
    /// How it formed.
    pub origin: MoonOrigin,
    /// What it looks like.
    pub kind: MoonKind,
    /// Mean radius in km.
    pub radius_km: f64,
    /// Semi-major axis in km.
    pub orbit_km: f64,
    /// Orbital period in days.
    pub period_days: f64,
    /// Eccentricity (forced by resonance, primordial for captures).
    pub eccentricity: f64,
    /// Inclination to the planet equator in degrees.
    pub inclination_deg: f64,
    /// Whether the orbit runs retrograde.
    pub retrograde: bool,
    /// Circular prograde equatorial (resonances and heating read it).
    pub regular: bool,
    /// Resonance with the inner neighbour, if snapped.
    pub resonance: Option<Resonance>,
    /// Tidal heating score (Io = 1).
    pub heating: f64,
    /// Orbital phase in degrees (Trojans sit ±60 from their host).
    pub phase_deg: f64,
    /// Host index for Trojan and horseshoe companions (`None` otherwise).
    pub trojan_host: Option<usize>,
    /// Centre of mass outside the planet (double planet).
    pub binary: bool,
    /// Spin locked to the orbit (`t_lock` under the system age, T11).
    pub locked: bool,
    /// Irregular shape in 0..1 (potato below its radius, sphere above).
    pub irregularity_01: f64,
    /// Bulk density in kg/m^3.
    pub density_kg_m3: f64,
}

/// One planet's moons.
#[derive(Debug, Clone, PartialEq)]
pub struct MoonSystem {
    /// Every moon: impact first, then regulars inner to outer, then
    /// irregulars, then co-orbital companions (decayed moons excluded).
    pub moons: Vec<Moon>,
    /// A binary pair orbits a point in space (centre of mass outside).
    pub binary: bool,
    /// Moons decayed past breakup by the system age (rings, T11).
    pub lost_to_rings: u32,
}

impl MoonSystem {
    /// No moons.
    #[must_use]
    pub fn empty() -> MoonSystem {
        MoonSystem {
            moons: Vec::new(),
            binary: false,
            lost_to_rings: 0,
        }
    }
}

/// What a moon system is built from (the planet's recipe values).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanetSpec {
    /// Mass in Earth masses.
    pub mass_earth: f64,
    /// Radius in Earth radii.
    pub radius_earth: f64,
    /// The planet's type.
    pub kind: WorldKind,
    /// Orbit in AU (0 for a rogue).
    pub orbit_au: f64,
    /// Star mass in Sun masses (0 for a rogue).
    pub star_mass_solar: f64,
    /// System age in billions of years.
    pub age_gyr: f64,
    /// Star luminosity at that age, in Sun units.
    pub luminosity: f64,
    /// Day length in hours (sets the synchronous radius for decay).
    pub spin_hours: f64,
}

impl PlanetSpec {
    /// The spec of a generated recipe spinning once per `spin_hours`.
    #[must_use]
    pub fn from_recipe(recipe: &super::WorldRecipe, spin_hours: f64) -> PlanetSpec {
        PlanetSpec {
            mass_earth: recipe.mass_earth,
            radius_earth: recipe.radius_earth,
            kind: recipe.kind,
            orbit_au: recipe.orbit_au,
            star_mass_solar: recipe.star_mass,
            age_gyr: recipe.age_gyr,
            luminosity: recipe.luminosity,
            spin_hours,
        }
    }
}

/// Hill radius in km: `a (m / 3M)^(1/3)` (moons guide). Unbounded
/// (`INFINITY`) without a star to compete with (rogues).
#[must_use]
pub fn hill_radius_km(orbit_au: f64, planet_mass_kg: f64, star_mass_kg: f64) -> f64 {
    if !orbit_au.is_finite()
        || !planet_mass_kg.is_finite()
        || !star_mass_kg.is_finite()
        || orbit_au <= 0.0
        || planet_mass_kg <= 0.0
        || star_mass_kg <= 0.0
    {
        if planet_mass_kg.is_finite() && planet_mass_kg > 0.0 {
            return f64::INFINITY;
        }
        return 0.0;
    }
    orbit_au * AU_KM * (planet_mass_kg / (3.0 * star_mass_kg)).powf(1.0 / 3.0)
}

/// Roche limit in km: `2.44 R (ρ_planet / ρ_moon)^(1/3)` (moons guide).
/// Inside it a moon breaks into rings. Degenerate input reads 0.
#[must_use]
pub fn roche_limit_km(planet_radius_km: f64, planet_density: f64, moon_density: f64) -> f64 {
    if !planet_radius_km.is_finite()
        || !planet_density.is_finite()
        || !moon_density.is_finite()
        || planet_radius_km <= 0.0
        || planet_density <= 0.0
        || moon_density <= 0.0
    {
        return 0.0;
    }
    2.44 * planet_radius_km * (planet_density / moon_density).powf(1.0 / 3.0)
}

/// Bulk density in kg/m^3 of a sphere of `mass_kg` and `radius_km`.
/// Degenerate input reads rock.
#[must_use]
pub fn density_kg_m3(mass_kg: f64, radius_km: f64) -> f64 {
    if !mass_kg.is_finite() || !radius_km.is_finite() || mass_kg <= 0.0 || radius_km <= 0.0 {
        return ROCK_DENSITY;
    }
    mass_kg / (4.0 / 3.0 * std::f64::consts::PI * (radius_km * 1_000.0).powi(3))
}

/// Radius in km of `mass_kg` at `density_kg_m3`.
#[must_use]
pub fn radius_for_mass_km(mass_kg: f64, density_kg_m3: f64) -> f64 {
    if !mass_kg.is_finite() || !density_kg_m3.is_finite() || mass_kg <= 0.0 || density_kg_m3 <= 0.0
    {
        return 0.0;
    }
    (3.0 * mass_kg / (4.0 * std::f64::consts::PI * density_kg_m3)).powf(1.0 / 3.0) / 1_000.0
}

/// Period in days of `orbit_km` around `planet_mass_kg` (Kepler).
/// Degenerate input reads 0.
#[must_use]
pub fn period_days(orbit_km: f64, planet_mass_kg: f64) -> f64 {
    if !orbit_km.is_finite()
        || !planet_mass_kg.is_finite()
        || orbit_km <= 0.0
        || planet_mass_kg <= 0.0
    {
        return 0.0;
    }
    let orbit_m = orbit_km * 1_000.0;
    2.0 * std::f64::consts::PI * (orbit_m * orbit_m * orbit_m / (GRAVITY_G * planet_mass_kg)).sqrt()
        / DAY_S
}

/// Potato radius in km: round above (moons guide: 200 icy, 300 rocky).
#[must_use]
pub fn potato_radius_km(density_kg_m3: f64) -> f64 {
    if density_kg_m3 < ICY_DENSITY {
        POTATO_ICE_KM
    } else {
        POTATO_ROCK_KM
    }
}

/// Irregular shape in 0..1: 0 at and above the potato radius, ramping to
/// 1 deeper below (the render displaces the sphere by it).
#[must_use]
pub fn irregularity_01(radius_km: f64, density_kg_m3: f64) -> f64 {
    if !radius_km.is_finite() {
        return 0.0;
    }
    let potato = potato_radius_km(density_kg_m3);
    ((potato - radius_km) / (0.6 * potato)).clamp(0.0, 1.0)
}

/// Tidal heating score, normalised so Io scores 1 (moon-dynamics guide:
/// `ρ² n^5 R^7 e²`, with `n²a³ = GM` giving `M^2.5 a^-7.5` at fixed
/// eccentricity). Non-zero only with a forced eccentricity, which only
/// resonances provide: circular moons score exactly 0.
#[must_use]
pub fn heating_score(
    density_kg_m3: f64,
    planet_mass_kg: f64,
    radius_km: f64,
    eccentricity: f64,
    orbit_km: f64,
) -> f64 {
    if !density_kg_m3.is_finite()
        || !planet_mass_kg.is_finite()
        || !radius_km.is_finite()
        || !eccentricity.is_finite()
        || !orbit_km.is_finite()
        || density_kg_m3 <= 0.0
        || planet_mass_kg <= 0.0
        || radius_km <= 0.0
        || eccentricity <= 0.0
        || orbit_km <= 0.0
    {
        return 0.0;
    }
    const IO_DENSITY: f64 = 3_500.0;
    const IO_PLANET_KG: f64 = 1.898e27;
    const IO_RADIUS_KM: f64 = 1_800.0;
    const IO_ECC: f64 = 0.0043;
    const IO_ORBIT_KM: f64 = 421_700.0;
    (density_kg_m3 / IO_DENSITY).powi(2)
        * (planet_mass_kg / IO_PLANET_KG).powf(2.5)
        * (radius_km / IO_RADIUS_KM).powi(7)
        * (eccentricity / IO_ECC).powi(2)
        * (IO_ORBIT_KM / orbit_km).powf(7.5)
}

/// Whether the pair is a double planet: the centre of mass
/// `a × m_moon / (M + m_moon)` lies outside the planet (moons guide).
#[must_use]
pub fn is_binary(
    orbit_km: f64,
    planet_mass_kg: f64,
    moon_mass_kg: f64,
    planet_radius_km: f64,
) -> bool {
    if !orbit_km.is_finite()
        || !planet_mass_kg.is_finite()
        || !moon_mass_kg.is_finite()
        || !planet_radius_km.is_finite()
        || orbit_km <= 0.0
        || planet_mass_kg <= 0.0
        || moon_mass_kg <= 0.0
        || planet_radius_km <= 0.0
    {
        return false;
    }
    orbit_km * moon_mass_kg / (planet_mass_kg + moon_mass_kg) > planet_radius_km
}

/// Count in `base..base+span` from a unit `draw` (table counts).
fn count(base: f64, span: f64, draw: f64) -> usize {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: small moon counts saturate like `as`"
    )]
    #[expect(
        clippy::cast_sign_loss,
        reason = "E-CAST: count draws non-negative, saturates like `as`"
    )]
    return (base + draw * span) as usize;
}

/// One planet's moon system from `seed` and `spec` (moons guide, step by
/// step): impact moon, disk regulars with resonance snaps, captured
/// irregulars, heating types, co-orbitals, binaries. Empty for degenerate
/// planets. T11 ages the orbits; decayed moons becoming rings rides on
/// these same orbits.
#[must_use]
pub fn system_for(seed: u64, spec: &PlanetSpec) -> MoonSystem {
    if !spec.mass_earth.is_finite()
        || !spec.radius_earth.is_finite()
        || spec.mass_earth <= 0.0
        || spec.radius_earth <= 0.0
    {
        return MoonSystem::empty();
    }
    let planet_kg = spec.mass_earth * EARTH_MASS_KG;
    let planet_km = spec.radius_earth * super::air::EARTH_RADIUS_M / 1_000.0;
    let planet_density = density_kg_m3(planet_kg, planet_km);
    let star_kg = spec.star_mass_solar * 1.989e30;
    let hill_km = hill_radius_km(spec.orbit_au, planet_kg, star_kg);
    let ice_au = super::star::ice_line_au(spec.luminosity);
    let mut rng = Rng::new(hash_triple(seed, MOON_TAG, 0));
    let mut moons: Vec<Moon> = Vec::new();
    // Counts by planet (moons guide, count table).
    let draw_a = rng.next_f64();
    let draw_b = rng.next_f64();
    let (mut regular_want, irregular_want, impact_rate) = match spec.kind {
        WorldKind::GasGiant => (count(4.0, 27.0, draw_a), count(20.0, 231.0, draw_b), 0.0),
        WorldKind::IceGiant => (count(5.0, 16.0, draw_a), count(5.0, 11.0, draw_b), 0.0),
        WorldKind::HotJupiter => (0, 0, 0.0),
        WorldKind::SubNeptune => (count(0.0, 4.0, draw_a), count(0.0, 6.0, draw_b), 0.0),
        WorldKind::Dwarf => (0, count(0.0, 6.0, draw_a), 0.0),
        WorldKind::EarthLike | WorldKind::Desert | WorldKind::SuperEarth => {
            (0, count(0.0, 3.0, draw_a), 0.25)
        }
        _ => {
            let small = spec.radius_earth < 0.5;
            (0, count(0.0, 3.0, draw_a), if small { 0.05 } else { 0.0 })
        }
    };
    // A big retrograde capture clears the original regulars (Triton rule).
    let triton_like = irregular_want > 0 && rng.next_f64() < 0.10;
    if triton_like && regular_want >= 4 {
        regular_want /= 2;
    }
    let mut lost = 0u32;
    // Inside the synchronous radius moons decay inward (T11); outside
    // they recede. Decayed past breakup at the system age: gone into
    // rings.
    let sync_km = super::lifetime::sync_radius_km(planet_kg, spec.spin_hours);
    let decayed = |orbit_km: f64, roche_km: f64| {
        orbit_km < sync_km && super::lifetime::decay_time_gyr(orbit_km, roche_km) < spec.age_gyr
    };
    // Impact moon: one big rocky moon, close at first.
    if rng.next_f64() < impact_rate {
        let mass_kg = planet_kg * 10f64.powf(-1.1 - 0.9 * rng.next_f64());
        let radius_km = radius_for_mass_km(mass_kg, ROCK_DENSITY + 300.0);
        let mut orbit_km = planet_km * (10.0 + 30.0 * rng.next_f64());
        orbit_km = orbit_km.min(0.4 * hill_km).min(HILL_CAP_KM);
        let roche_km = roche_limit_km(planet_km, planet_density, ROCK_DENSITY + 300.0);
        if orbit_km > 1.2 * roche_km && radius_km > 0.0 {
            if decayed(orbit_km, roche_km) {
                lost += 1;
            } else {
                let binary = is_binary(orbit_km, planet_kg, mass_kg, planet_km);
                moons.push(make_moon(
                    MoonOrigin::Impact,
                    radius_km,
                    orbit_km,
                    planet_kg,
                    ROCK_DENSITY + 300.0,
                    0.0,
                    rng.next_f64() * 5.0,
                    false,
                    true,
                    None,
                    rng.next_f64() * 360.0,
                    None,
                    binary,
                    spec,
                    ice_au,
                ));
                if binary {
                    return finish(moons, true, lost);
                }
            }
        }
    }
    // Dwarf collisional moon (Pluto-Charon type): big, close, often binary.
    if spec.kind == WorldKind::Dwarf && rng.next_f64() < 0.5 {
        let mass_kg = planet_kg * (0.05 + 0.10 * rng.next_f64());
        let density = if spec.orbit_au > ice_au {
            ICE_DENSITY
        } else {
            ROCK_DENSITY
        };
        let radius_km = radius_for_mass_km(mass_kg, density);
        let mut orbit_km = planet_km * (5.0 + 10.0 * rng.next_f64());
        orbit_km = orbit_km.min(0.4 * hill_km).min(HILL_CAP_KM);
        let roche_km = roche_limit_km(planet_km, planet_density, density);
        if orbit_km > 1.2 * roche_km && radius_km > 0.0 {
            if decayed(orbit_km, roche_km) {
                lost += 1;
            } else {
                let binary = is_binary(orbit_km, planet_kg, mass_kg, planet_km);
                moons.push(make_moon(
                    MoonOrigin::Impact,
                    radius_km,
                    orbit_km,
                    planet_kg,
                    density,
                    0.0,
                    rng.next_f64() * 5.0,
                    false,
                    true,
                    None,
                    rng.next_f64() * 360.0,
                    None,
                    binary,
                    spec,
                    ice_au,
                ));
                if binary {
                    return finish(moons, true, lost);
                }
            }
        }
    }
    // Disk regulars in two passes: orbits first (geometric spacing with
    // resonance snaps on giants, rocky inside and icy outside like the
    // Galileans), then masses from the disk budget normalised over the
    // moons that fit, so the budget holds exactly.
    let regular_outer = (REGULAR_HILL_SHARE * hill_km).min(HILL_CAP_KM);
    let roche_rock = roche_limit_km(planet_km, planet_density, ROCK_DENSITY);
    let roche_ice = roche_limit_km(planet_km, planet_density, ICE_DENSITY);
    let roche_inner = roche_ice.max(roche_rock).max(planet_km);
    if regular_want > 0 && regular_outer > roche_inner * MIN_SPACING {
        let giant = matches!(spec.kind, WorldKind::GasGiant | WorldKind::IceGiant);
        let budget_kg = if giant {
            DISK_BUDGET_SHARE
                * planet_kg
                * 10f64.powf((rng.next_f64() * 2.0 - 1.0) * DISK_BUDGET_DEX)
        } else {
            0.0
        };
        // Pass one: orbits and resonances (rocky inside, icy outside).
        let rocky_n = (regular_want / 3).max(1);
        let mut orbits: Vec<(f64, Option<Resonance>, bool, f64)> = Vec::new();
        let mut orbit_km = roche_inner * (1.05 + 0.2 * rng.next_f64());
        while orbits.len() < regular_want {
            if orbit_km > regular_outer {
                break;
            }
            let mut resonance = None;
            if giant && !orbits.is_empty() {
                let roll = rng.next_f64();
                if roll < 0.30 {
                    let inner = orbits.last().map_or(orbit_km, |(orbit, _, _, _)| *orbit);
                    let snap = if roll < 0.22 {
                        resonance = Some(Resonance::TwoOne);
                        2f64.powf(2.0 / 3.0)
                    } else {
                        resonance = Some(Resonance::ThreeTwo);
                        1.5f64.powf(2.0 / 3.0)
                    };
                    let snapped = inner * snap;
                    if snapped > orbit_km * 0.9 && snapped <= regular_outer {
                        orbit_km = snapped;
                    } else {
                        resonance = None;
                    }
                }
            }
            // A snap forces the inner neighbour's eccentricity (the heat).
            if resonance.is_some()
                && let Some(inner) = orbits.last_mut()
            {
                inner.2 = true;
            }
            let density = if orbits.len() < rocky_n {
                ROCK_DENSITY
            } else {
                ICE_DENSITY
            };
            if decayed(orbit_km, roche_limit_km(planet_km, planet_density, density)) {
                lost += 1;
                orbit_km *= MIN_SPACING + rng.next_f64() * 1.2;
                continue;
            }
            orbits.push((orbit_km, resonance, false, density));
            orbit_km *= MIN_SPACING + rng.next_f64() * 1.2;
        }
        // Pass two: masses, shapes and types.
        // The budget normalises over the split slots (1..); slot 0 runs
        // tiny outside it, so no share strands on the fixed mass.
        let mut weight_sum = 0.0;
        for slot in 1..orbits.len() {
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: moon slot below 300, exactly representable"
            )]
            let rank = slot as f64;
            weight_sum += 1.0 / (rank + 1.0).powf(1.5);
        }
        for (slot, (orbit, resonance, forced, density)) in orbits.into_iter().enumerate() {
            // The innermost regular runs tiny (inner smalls like Metis);
            // the rest split the disk budget steepest-first.
            let (mass_kg, origin) = if slot == 0 {
                let radius_km = 15.0 + rng.next_f64() * 105.0;
                (
                    density * 4.0 / 3.0 * std::f64::consts::PI * (radius_km * 1_000.0).powi(3),
                    MoonOrigin::Disk,
                )
            } else if giant && budget_kg > 0.0 {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "E-CAST: moon slot below 300, exactly representable"
                )]
                let rank = slot as f64;
                let weight = 1.0 / (rank + 1.0).powf(1.5);
                (budget_kg * weight / weight_sum, MoonOrigin::Disk)
            } else {
                let radius_km = 50.0 + rng.next_f64() * 350.0;
                (
                    density * 4.0 / 3.0 * std::f64::consts::PI * (radius_km * 1_000.0).powi(3),
                    MoonOrigin::Disk,
                )
            };
            let mut radius_km = radius_for_mass_km(mass_kg, density);
            if radius_km <= 0.0 {
                radius_km = 50.0;
            }
            // Forced eccentricity from the resonance; circular otherwise,
            // so only resonant moons heat.
            let eccentricity = if resonance.is_some() || forced {
                0.004 + 0.004 * rng.next_f64()
            } else {
                0.0
            };
            let origin = if orbit < 1.5 * roche_limit_km(planet_km, planet_density, density) {
                MoonOrigin::Breakup
            } else {
                origin
            };
            moons.push(make_moon(
                origin,
                radius_km,
                orbit,
                planet_kg,
                density,
                eccentricity,
                rng.next_f64() * 5.0,
                false,
                true,
                resonance,
                rng.next_f64() * 360.0,
                None,
                false,
                spec,
                ice_au,
            ));
        }
    }
    // Captured irregulars: eccentric, inclined, mostly retrograde.
    let irregular_outer = (IRREGULAR_HILL_SHARE * hill_km).min(HILL_CAP_KM);
    let irregular_inner = (0.05 * hill_km).min(irregular_outer / 1.1);
    if irregular_want > 0 && irregular_outer > roche_inner && irregular_outer > irregular_inner {
        let lo = irregular_inner.max(roche_inner).ln();
        let hi = irregular_outer.ln();
        for i in 0..irregular_want {
            let orbit_km = (lo + rng.next_f64() * (hi - lo)).exp();
            let big = triton_like && i == 0;
            let radius_km = if big {
                200.0 + rng.next_f64() * 1_200.0
            } else {
                5.0 * 30f64.powf(rng.next_f64())
            };
            let rocky = if spec.orbit_au <= 0.0 {
                rng.next_f64() < 0.5
            } else if spec.orbit_au < ice_au {
                rng.next_f64() < 0.8
            } else {
                rng.next_f64() < 0.2
            };
            let density = if rocky { ROCK_DENSITY } else { ICE_DENSITY };
            if decayed(orbit_km, roche_limit_km(planet_km, planet_density, density)) {
                lost += 1;
                continue;
            }
            let (inclination_deg, retrograde) = if rng.next_f64() < 0.83 {
                (90.0 + rng.next_f64() * 80.0, true)
            } else {
                (rng.next_f64() * 90.0, false)
            };
            moons.push(make_moon(
                MoonOrigin::Capture,
                radius_km,
                orbit_km,
                planet_kg,
                density,
                0.2 + 0.4 * rng.next_f64(),
                inclination_deg,
                retrograde,
                false,
                None,
                rng.next_f64() * 360.0,
                None,
                false,
                spec,
                ice_au,
            ));
        }
    }
    // Co-orbitals on large regulars: Trojans at ±60°, rare horseshoe pairs.
    let hosts: Vec<usize> = moons
        .iter()
        .enumerate()
        .filter(|(_, moon)| moon.regular && moon.trojan_host.is_none() && moon.radius_km > 500.0)
        .map(|(index, _)| index)
        .collect();
    for host in hosts {
        let roll = rng.next_f64();
        let companions = if roll < 0.03 {
            2
        } else if roll < 0.15 {
            if rng.next_f64() < 0.5 { 1 } else { 2 }
        } else {
            0
        };
        let horseshoe = roll < 0.03;
        for c in 0..companions {
            let radius_km = 10.0 + rng.next_f64() * 50.0;
            let Some(host_moon) = moons.get(host) else {
                continue;
            };
            let density = host_moon.density_kg_m3;
            let host_orbit = host_moon.orbit_km;
            let host_phase = host_moon.phase_deg;
            let host_incl = host_moon.inclination_deg;
            let host_retro = host_moon.retrograde;
            let phase_deg = if horseshoe {
                host_phase + 120.0 * (f64::from(c) + 1.0)
            } else {
                host_phase + if c == 0 { 60.0 } else { -60.0 }
            };
            moons.push(make_moon(
                MoonOrigin::Breakup,
                radius_km,
                host_orbit,
                planet_kg,
                density,
                0.0,
                host_incl,
                host_retro,
                true,
                None,
                phase_deg,
                Some(host),
                false,
                spec,
                ice_au,
            ));
        }
    }
    finish(moons, false, lost)
}

/// Shared tail: recompute nothing, just flag the binary and the decayed.
fn finish(moons: Vec<Moon>, binary: bool, lost_to_rings: u32) -> MoonSystem {
    MoonSystem {
        moons,
        binary,
        lost_to_rings,
    }
}

/// Build one moon and pick its type from the heating score and orbit.
#[allow(
    clippy::too_many_arguments,
    reason = "moon fields share one constructor; call sites pass them together"
)]
fn make_moon(
    origin: MoonOrigin,
    radius_km: f64,
    orbit_km: f64,
    planet_kg: f64,
    density_kg_m3: f64,
    eccentricity: f64,
    inclination_deg: f64,
    retrograde: bool,
    regular: bool,
    resonance: Option<Resonance>,
    phase_deg: f64,
    trojan_host: Option<usize>,
    binary: bool,
    spec: &PlanetSpec,
    ice_au: f64,
) -> Moon {
    let heating = heating_score(density_kg_m3, planet_kg, radius_km, eccentricity, orbit_km);
    let moon_kg = density_kg_m3 * 4.0 / 3.0 * std::f64::consts::PI * (radius_km * 1_000.0).powi(3);
    let locked =
        super::lifetime::lock_time_gyr(orbit_km, radius_km, density_kg_m3, moon_kg, planet_kg)
            <= spec.age_gyr;
    let seed_moon = radius_km.to_bits() ^ orbit_km.to_bits();
    let mut rng = Rng::new(hash_triple(seed_moon, MOON_TAG, 1));
    let rocky = density_kg_m3 >= ICY_DENSITY;
    let hill_km = hill_radius_km(spec.orbit_au, planet_kg, spec.star_mass_solar * 1.989e30);
    let far_out = orbit_km > RINGED_HILL_SHARE * hill_km.min(HILL_CAP_KM);
    let kind =
        if far_out && radius_km < RINGED_RADIUS_KM && !regular && rng.next_f64() < RINGED_RATE {
            MoonKind::Ringed
        } else if !regular {
            MoonKind::Captured
        } else if heating >= HOT_SCORE {
            if rocky {
                MoonKind::Volcanic
            } else {
                MoonKind::Ocean
            }
        } else if !rocky && heating >= CRACKED_SCORE {
            MoonKind::Ocean
        } else if radius_km > HAZY_RADIUS_KM
            && eccentricity < 0.05
            && spec.orbit_au > ice_au
            && ice_au.is_finite()
        {
            MoonKind::Hazy
        } else if regular && radius_km < SHEPHERD_RADIUS_KM {
            MoonKind::Shepherd
        } else {
            MoonKind::Dead
        };
    Moon {
        origin,
        kind,
        radius_km,
        orbit_km,
        period_days: period_days(orbit_km, planet_kg),
        eccentricity,
        inclination_deg,
        retrograde,
        regular,
        resonance,
        heating,
        phase_deg,
        trojan_host,
        binary,
        locked,
        irregularity_01: irregularity_01(radius_km, density_kg_m3),
        density_kg_m3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An Earth-like spec (1 AU around a Sun-like star, 4.5 Gyr).
    fn earth_spec() -> PlanetSpec {
        PlanetSpec {
            mass_earth: 1.0,
            radius_earth: 1.0,
            kind: WorldKind::EarthLike,
            orbit_au: 1.0,
            star_mass_solar: 1.0,
            age_gyr: 4.5,
            luminosity: 1.0,
            spin_hours: 24.0,
        }
    }

    /// A cold gas giant (Jupiter-like).
    fn giant_spec() -> PlanetSpec {
        PlanetSpec {
            mass_earth: 318.0,
            radius_earth: 11.2,
            kind: WorldKind::GasGiant,
            orbit_au: 5.2,
            star_mass_solar: 1.0,
            age_gyr: 4.5,
            luminosity: 1.0,
            spin_hours: 10.0,
        }
    }

    #[test]
    fn limits_match_their_textbook_values() {
        // Earth: Hill ~1.5M km, the Moon at 0.26 r_H; fluid Roche ~18,400 km.
        let earth_kg = EARTH_MASS_KG;
        let sun_kg = 1.989e30;
        let hill = hill_radius_km(1.0, earth_kg, sun_kg);
        assert!((hill - 1_500_000.0).abs() < 60_000.0, "hill {hill}");
        assert!((384_400.0 / hill - 0.26).abs() < 0.02);
        let roche = roche_limit_km(6_371.0, 5_515.0, 3_344.0);
        assert!((roche - 18_400.0).abs() < 800.0, "roche {roche}");
        assert_eq!(hill_radius_km(0.0, earth_kg, sun_kg), f64::INFINITY);
        assert_eq!(hill_radius_km(1.0, f64::NAN, sun_kg), 0.0);
        assert_eq!(roche_limit_km(f64::NAN, 5_515.0, 3_344.0), 0.0);
        // Io scores 1 by construction.
        let io = heating_score(3_500.0, 1.898e27, 1_800.0, 0.0043, 421_700.0);
        assert!((io - 1.0).abs() < 0.05, "io {io}");
        assert_eq!(
            heating_score(3_500.0, 1.898e27, 1_800.0, 0.0, 421_700.0),
            0.0
        );
        // Earth-Moon outside, Pluto-Charon inside-out (binary).
        assert!(!is_binary(384_400.0, 5.972e24, 7.3e22, 6_371.0));
        assert!(is_binary(19_600.0, 1.31e22, 1.59e21, 1_188.0));
        assert!(!is_binary(f64::NAN, 5.972e24, 7.3e22, 6_371.0));
        // Round above the potato radius, potato below.
        assert_eq!(irregularity_01(1_738.0, ROCK_DENSITY), 0.0);
        assert!(irregularity_01(10.0, ROCK_DENSITY) > 0.9);
        assert_eq!(potato_radius_km(1_500.0), POTATO_ICE_KM);
        assert_eq!(potato_radius_km(3_000.0), POTATO_ROCK_KM);
    }

    #[test]
    fn moon_systems_respect_limits() {
        // C13 / AC13 (orbits part; locking and decay ride with T11):
        // regulars between the Roche limit and 0.05 r_H with 1.3x spacing,
        // irregulars inside 0.45 r_H, giant budgets near 1e-4, potato shapes
        // below the potato radius, binaries flagged by centre of mass.
        let specs = [
            earth_spec(),
            giant_spec(),
            PlanetSpec {
                mass_earth: 17.0,
                radius_earth: 4.0,
                kind: WorldKind::IceGiant,
                orbit_au: 19.2,
                star_mass_solar: 1.0,
                age_gyr: 4.5,
                luminosity: 1.0,
                spin_hours: 16.0,
            },
            PlanetSpec {
                mass_earth: 0.002,
                radius_earth: 0.15,
                kind: WorldKind::Dwarf,
                orbit_au: 39.0,
                star_mass_solar: 1.0,
                age_gyr: 4.5,
                luminosity: 1.0,
                spin_hours: 24.0,
            },
            PlanetSpec {
                mass_earth: 300.0,
                radius_earth: 11.0,
                kind: WorldKind::HotJupiter,
                orbit_au: 0.05,
                star_mass_solar: 1.0,
                age_gyr: 4.5,
                luminosity: 1.0,
                spin_hours: 72.0,
            },
            PlanetSpec {
                mass_earth: 1.0,
                radius_earth: 1.0,
                kind: WorldKind::Rogue,
                orbit_au: 0.0,
                star_mass_solar: 0.0,
                age_gyr: 4.5,
                luminosity: 0.0,
                spin_hours: 24.0,
            },
        ];
        let mut binaries = 0u32;
        let mut trojans = 0u32;
        for spec in &specs {
            let planet_kg = spec.mass_earth * EARTH_MASS_KG;
            let planet_km = spec.radius_earth * super::super::air::EARTH_RADIUS_M / 1_000.0;
            let hill_km = hill_radius_km(spec.orbit_au, planet_kg, spec.star_mass_solar * 1.989e30);
            for seed in 0..200u64 {
                let system = system_for(seed, spec);
                assert_eq!(system, system_for(seed, spec), "replay differs");
                let mut last_regular = 0.0;
                let mut regular_mass = 0.0;
                for moon in system.moons.iter() {
                    assert!(moon.radius_km > 0.0 && moon.orbit_km > 0.0, "{moon:?}");
                    assert!(moon.period_days > 0.0, "{moon:?}");
                    assert!(
                        moon.irregularity_01 == irregularity_01(moon.radius_km, moon.density_kg_m3),
                        "shape disagrees"
                    );
                    let roche = roche_limit_km(
                        planet_km,
                        density_kg_m3(planet_kg, planet_km),
                        moon.density_kg_m3,
                    );
                    assert!(moon.orbit_km > roche, "{moon:?} inside its Roche limit");
                    if let Some(host_index) = moon.trojan_host {
                        let host = &system.moons[host_index];
                        assert!(
                            (moon.orbit_km - host.orbit_km).abs() < 1e-9,
                            "trojan off its orbit"
                        );
                        let offset = (moon.phase_deg - host.phase_deg).abs();
                        assert!(
                            (offset - 60.0).abs() < 1e-9
                                || (offset - 120.0).abs() < 1e-9
                                || (offset - 240.0).abs() < 1e-9,
                            "trojan off its station: {offset}"
                        );
                        trojans += 1;
                    }
                    if moon.binary {
                        binaries += 1;
                        let moon_kg = moon.density_kg_m3 * 4.0 / 3.0
                            * std::f64::consts::PI
                            * (moon.radius_km * 1_000.0).powi(3);
                        assert!(
                            is_binary(moon.orbit_km, planet_kg, moon_kg, planet_km),
                            "binary flag without an outside centre of mass"
                        );
                    }
                    if moon.regular && moon.trojan_host.is_none() {
                        if moon.origin == MoonOrigin::Impact {
                            // Impact moons are their own category (ours at
                            // 0.26 r_H): bound by the 0.4 placement line.
                            assert!(
                                moon.orbit_km <= (0.4 * hill_km).min(HILL_CAP_KM) + 1.0,
                                "{moon:?} past the impact line"
                            );
                        } else {
                            assert!(
                                moon.orbit_km
                                    <= (REGULAR_HILL_SHARE * hill_km).min(HILL_CAP_KM) + 1.0,
                                "{moon:?} past the regular line"
                            );
                            if last_regular > 0.0 {
                                assert!(
                                    moon.orbit_km / last_regular >= MIN_SPACING - 1e-9,
                                    "regulars overlap"
                                );
                            }
                            last_regular = moon.orbit_km;
                        }
                        if matches!(moon.origin, MoonOrigin::Disk | MoonOrigin::Breakup) {
                            regular_mass += moon.density_kg_m3 * 4.0 / 3.0
                                * std::f64::consts::PI
                                * (moon.radius_km * 1_000.0).powi(3);
                        }
                    }
                    if !moon.regular {
                        assert!(
                            moon.orbit_km
                                <= (IRREGULAR_HILL_SHARE * hill_km).min(HILL_CAP_KM) + 1.0,
                            "{moon:?} past the irregular line"
                        );
                    }
                }
                if matches!(spec.kind, WorldKind::GasGiant | WorldKind::IceGiant)
                    && regular_mass > 0.0
                {
                    let share = regular_mass / planet_kg;
                    assert!(
                        (0.4e-4..2.5e-4).contains(&share),
                        "disk budget {share} far from 1e-4 for {:?} seed {seed}: {} moons",
                        spec.kind,
                        system.moons.len(),
                    );
                }
            }
        }
        assert!(binaries > 0, "no binary pair in range");
        assert!(trojans > 0, "no trojan companion in range");
        // Hot Jupiters keep no moons; degenerate planets keep none.
        for seed in 0..50u64 {
            assert!(
                system_for(
                    seed,
                    &PlanetSpec {
                        kind: WorldKind::HotJupiter,
                        ..giant_spec()
                    }
                )
                .moons
                .is_empty()
            );
        }
        assert!(
            system_for(
                7,
                &PlanetSpec {
                    mass_earth: 0.0,
                    ..earth_spec()
                }
            )
            .moons
            .is_empty()
        );
        assert!(
            system_for(
                7,
                &PlanetSpec {
                    mass_earth: f64::NAN,
                    ..earth_spec()
                }
            )
            .moons
            .is_empty()
        );
    }

    #[test]
    fn heated_moons_are_close_in_and_resonant() {
        // Moon-dynamics checks: volcanic and ocean moons ride high heating
        // scores on forced eccentricities; resonant pairs snap to 2:1/3:2.
        let mut hot = 0u32;
        let mut resonant_pairs = 0u32;
        let mut laplace = 0u32;
        for seed in 0..400u64 {
            let system = system_for(seed, &giant_spec());
            for moon in &system.moons {
                if matches!(moon.kind, MoonKind::Volcanic | MoonKind::Ocean) {
                    assert!(moon.heating >= CRACKED_SCORE, "{moon:?}");
                    assert!(moon.eccentricity > 0.0, "heat without eccentricity");
                    hot += 1;
                }
                if moon.resonance.is_some() {
                    resonant_pairs += 1;
                }
            }
            let resonances: Vec<_> = system
                .moons
                .iter()
                .filter_map(|moon| moon.resonance)
                .collect();
            for pair in resonances.windows(2) {
                if pair == [Resonance::TwoOne, Resonance::TwoOne] {
                    laplace += 1;
                    break;
                }
            }
        }
        assert!(hot > 0, "no heated moon in range");
        assert!(resonant_pairs > 0, "no resonance in range");
        assert!(laplace > 0, "no Laplace chain in range");
    }
}
