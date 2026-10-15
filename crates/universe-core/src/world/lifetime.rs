//! Lifetime: activity fade, erosion, locking, decay and dry-out at the
//! system's age (#685, T11).
//!
//! Guides: `guides/universe/L10/generation/documents/lifetime.md` (the
//! interior cools, erosion grows with time since uplift, spins lock and
//! moons decay, young red dwarfs dry out close-in wet worlds) and
//! `moon-dynamics.md` (the lock time). The star's brightening ramp and
//! the shoreline test over age already live in
//! [`super::star::luminosity`] and [`super::air::keeps_air`]; this module
//! adds what those leave out. Everything is `f64` and deterministic
//! (`E-DET-TIERS`): the same seed at another age gives the same world at
//! another time of its life.
//!
//! Blast radius is deliberate: at the mature reference age
//! ([`MATURE_AGE_GYR`]) on an Earth-size world every factor reads near its
//! unwired value, so the tuned gallery (T8) keeps passing while young and
//! old worlds diverge.

use crate::seed::hash_triple;

/// Mature reference age in Gyr: factors read near-unwired here.
pub const MATURE_AGE_GYR: f64 = 4.5;

/// Activity time constant in Gyr per Mars radius (lifetime guide: about
/// 1 Gyr per Mars radius scales too fast for Earth, which still moves at
/// 4.5 Gyr; 2 Gyr keeps Earth mobile, Mars stagnant, young worlds hot).
pub const ACTIVITY_TAU_MARS_GYR: f64 = 2.0;

/// Mars radius in Earth radii (the time-constant yardstick).
pub const MARS_RADIUS_EARTH: f64 = 0.53;

/// Erosion reaches this share of the plate relief at most.
pub const EROSION_MAX: f64 = 0.5;

/// Lock-time rigidity in N/m^2 for rock (moon-dynamics guide).
pub const RIGIDITY_ROCK_PA: f64 = 3e10;

/// Lock-time rigidity in N/m^2 for ice.
pub const RIGIDITY_ICE_PA: f64 = 4e9;

/// Decay calibration in Gyr at one Roche radius (Phobos-like: tens of
/// millions of years just outside the limit, documented heuristic).
pub const DECAY_TAU_GYR: f64 = 0.03;

/// Star mass below which dwarfs dry out close-in wet worlds (Sun masses).
pub const DWARF_MASS_SOLAR: f64 = 0.5;

/// Orbit inside which a dwarf's early phase dries worlds (AU).
pub const DRY_ORBIT_AU: f64 = 0.25;

/// The early bright phase is over by this age in Gyr.
pub const DRY_PHASE_GYR: f64 = 0.2;

/// Stream tag of the dry-out draw.
const DRY_TAG: u64 = 0xD271_0685_0000_0000;

/// Interior activity in 0..1 at `age_gyr` on a world of `radius_earth`
/// Earth radii (lifetime guide: `exp(-t/τ)`, sooner dead on small
/// worlds). Degenerate input reads still (0.0).
#[must_use]
pub fn activity_for(radius_earth: f64, age_gyr: f64) -> f64 {
    if !radius_earth.is_finite() || !age_gyr.is_finite() || radius_earth <= 0.0 || age_gyr < 0.0 {
        return 0.0;
    }
    let tau = ACTIVITY_TAU_MARS_GYR * radius_earth / MARS_RADIUS_EARTH;
    (-age_gyr / tau).exp().clamp(0.0, 1.0)
}

/// Erosion share in 0..1 of `uplift_age_gyr` since the ground rose
/// (lifetime guide: old ranges low and rounded, young ones high and
/// sharp). Fresh crust reads 0; degenerate input reads 0.
#[must_use]
pub fn erosion_for(uplift_age_gyr: f64) -> f64 {
    if !uplift_age_gyr.is_finite() || uplift_age_gyr <= 0.0 {
        return 0.0;
    }
    EROSION_MAX * super::noise3::smoothstep(1.0, 9.0, uplift_age_gyr)
}

/// Time in Gyr for the moon to lock to its planet (moon-dynamics guide,
/// Gladman simplified: `6 a^6 R μ / (m_s m_p^2) × 10^10 yr`, rock or ice
/// rigidity by `moon_density`). `INFINITY` for degenerate input (never
/// locks); 0.0 for a moon on top of its planet.
#[must_use]
pub fn lock_time_gyr(
    orbit_km: f64,
    moon_radius_km: f64,
    moon_density: f64,
    moon_mass_kg: f64,
    planet_mass_kg: f64,
) -> f64 {
    if !orbit_km.is_finite()
        || !moon_radius_km.is_finite()
        || !moon_density.is_finite()
        || !moon_mass_kg.is_finite()
        || !planet_mass_kg.is_finite()
        || orbit_km <= 0.0
        || moon_radius_km <= 0.0
        || moon_mass_kg <= 0.0
        || planet_mass_kg <= 0.0
    {
        return f64::INFINITY;
    }
    let rigidity = if moon_density < super::moons::ICY_DENSITY {
        RIGIDITY_ICE_PA
    } else {
        RIGIDITY_ROCK_PA
    };
    let orbit_m = orbit_km * 1_000.0;
    let radius_m = moon_radius_km * 1_000.0;
    6.0 * orbit_m.powi(6) * radius_m * rigidity / (moon_mass_kg * planet_mass_kg.powi(2)) * 10.0
}

/// Time in Gyr for a moon at `orbit_km` to decay past `roche_km`
/// (moon-dynamics guide: Phobos-like, tens of millions of years near the
/// limit, steeply longer farther out). Only moons inside the synchronous
/// radius decay inward (the caller gates that); the rest recede.
/// Degenerate input reads `INFINITY`.
#[must_use]
pub fn decay_time_gyr(orbit_km: f64, roche_km: f64) -> f64 {
    if !orbit_km.is_finite() || !roche_km.is_finite() || orbit_km <= 0.0 || roche_km <= 0.0 {
        return f64::INFINITY;
    }
    DECAY_TAU_GYR * (orbit_km / roche_km).powi(3)
}

/// Synchronous orbit radius in km for `planet_mass_kg` spinning once per
/// `spin_hours`: inside it moons decay inward, outside it they recede
/// (moon-dynamics guide). Degenerate spin reads 0 (nothing inside).
#[must_use]
pub fn sync_radius_km(planet_mass_kg: f64, spin_hours: f64) -> f64 {
    if !planet_mass_kg.is_finite()
        || !spin_hours.is_finite()
        || planet_mass_kg <= 0.0
        || spin_hours <= 0.0
    {
        return 0.0;
    }
    let period_s = spin_hours * 3_600.0;
    (super::air::GRAVITY_G * planet_mass_kg * period_s * period_s
        / (4.0 * std::f64::consts::PI * std::f64::consts::PI))
        .powf(1.0 / 3.0)
        / 1_000.0
}

/// Whether a wet world dried out: an M dwarf's bright early phase bakes
/// close-in worlds that kept their air, with a seeded chance scaled by
/// gravity (lifetime guide: loss scales with surface gravity, low worlds
/// lose more). Needs the phase over (`age_gyr` past [`DRY_PHASE_GYR`]).
#[must_use]
pub fn dried_out(
    seed: u64,
    star_mass_solar: f64,
    orbit_au: f64,
    age_gyr: f64,
    gravity: f64,
) -> bool {
    if !star_mass_solar.is_finite()
        || !orbit_au.is_finite()
        || !age_gyr.is_finite()
        || !gravity.is_finite()
        || gravity <= 0.0
    {
        return false;
    }
    if star_mass_solar >= DWARF_MASS_SOLAR || orbit_au > DRY_ORBIT_AU || age_gyr <= DRY_PHASE_GYR {
        return false;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: 53-bit hash share as a unit float, intended"
    )]
    let share = (hash_triple(seed, DRY_TAG, 0) >> 11) as f64 / 9_007_199_254_740_992.0;
    share < (0.8 * 9.81 / gravity).clamp(0.0, 0.95)
}

/// Surface water share after dry-out: 0 on dried worlds, the drawn share
/// otherwise. The climate and biome passes read this, never the raw draw.
#[must_use]
pub fn surface_water_01(
    seed: u64,
    drawn_01: f64,
    star_mass_solar: f64,
    orbit_au: f64,
    age_gyr: f64,
    gravity: f64,
) -> f64 {
    if dried_out(seed, star_mass_solar, orbit_au, age_gyr, gravity) {
        0.0
    } else if drawn_01.is_finite() {
        drawn_01.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::moons::PlanetSpec;

    #[test]
    fn activity_fades_sooner_on_small_worlds() {
        // Young worlds hot, Earth mobile at 4.5 Gyr, Mars stagnant.
        assert!(
            (activity_for(1.0, 0.5) - 0.88).abs() < 0.05,
            "{}",
            activity_for(1.0, 0.5)
        );
        assert!(activity_for(1.0, MATURE_AGE_GYR) > 0.15, "earth dies");
        assert!(activity_for(0.53, MATURE_AGE_GYR) < 0.15, "mars lives");
        assert!(activity_for(1.0, 10.0) < activity_for(1.0, MATURE_AGE_GYR));
        assert_eq!(activity_for(f64::NAN, 4.5), 0.0);
        assert_eq!(activity_for(1.0, -1.0), 0.0);
        assert_eq!(activity_for(1.0, 4.5), activity_for(1.0, 4.5));
    }

    #[test]
    fn erosion_grows_with_uplift_age() {
        assert_eq!(erosion_for(0.3), 0.0, "fresh crust worn");
        assert!(erosion_for(8.0) > erosion_for(4.5));
        assert!(erosion_for(4.5) > erosion_for(1.5));
        assert!(erosion_for(4.5) < 0.25, "reference wear too deep");
        assert!(erosion_for(20.0) <= EROSION_MAX);
        assert_eq!(erosion_for(f64::NAN), 0.0);
    }

    #[test]
    fn lock_times_sort_close_before_far() {
        // The Moon locks early; a far speck never does.
        let moon = lock_time_gyr(384_400.0, 1_738.0, 3_344.0, 7.3e22, 5.972e24);
        assert!(moon < 4.5, "the Moon never locks: {moon}");
        let far = lock_time_gyr(10_000_000.0, 100.0, 1_500.0, 6e15, 5.972e24);
        assert!(far > 13.8, "a far speck locks: {far}");
        assert!(lock_time_gyr(400_000.0, 1_700.0, 3_300.0, 7e22, 6e24) < far);
        assert_eq!(
            lock_time_gyr(f64::NAN, 1_738.0, 3_344.0, 7.3e22, 5.972e24),
            f64::INFINITY
        );
    }

    #[test]
    fn decay_runs_phobos_fast_and_moons_slow() {
        // Near the limit gone in tens of Myr; the Moon outlives the stars.
        assert!(decay_time_gyr(9_000.0, 6_500.0) < 0.1, "phobos lingers");
        assert!(
            decay_time_gyr(384_400.0, 18_000.0) > 100.0,
            "the Moon decays"
        );
        assert!(decay_time_gyr(9_000.0, 6_500.0) < decay_time_gyr(20_000.0, 6_500.0));
        assert_eq!(decay_time_gyr(f64::NAN, 6_500.0), f64::INFINITY);
    }

    #[test]
    fn dry_out_hits_close_dwarf_worlds_once_old() {
        // Same world young (wet) and old (dry); Sun-like stars never dry.
        assert!(!dried_out(7, 0.2, 0.1, 0.1, 9.81), "dries before the phase");
        assert!(!dried_out(7, 1.0, 0.1, 5.0, 9.81), "sun dries");
        assert!(!dried_out(7, 0.2, 1.0, 5.0, 9.81), "far orbit dries");
        assert!(!dried_out(7, 0.2, 0.1, 5.0, f64::NAN), "degenerate dries");
        assert_eq!(
            dried_out(7, 0.2, 0.1, 5.0, 9.81),
            dried_out(7, 0.2, 0.1, 5.0, 9.81)
        );
        assert_eq!(
            surface_water_01(7, 0.5, 0.2, 0.1, 0.1, 9.81),
            0.5,
            "young world dried"
        );
        // Some seed in range dries once old (seeded chance, not every seed).
        let dries = (0..200u64).any(|seed| dried_out(seed, 0.2, 0.1, 5.0, 3.7));
        assert!(dries, "no dwarf world ever dries");
        let dried = (0..200u64).find(|seed| dried_out(*seed, 0.2, 0.1, 5.0, 3.7));
        if let Some(seed) = dried {
            assert_eq!(surface_water_01(seed, 0.5, 0.2, 0.1, 5.0, 3.7), 0.0);
            assert!(
                !dried_out(seed, 0.2, 0.1, 0.1, 3.7),
                "dries before the phase"
            );
        }
    }

    #[test]
    fn lifetime_changes_worlds_with_age() {
        // C12 / AC12: the same seed older reads more cratered, more
        // eroded and less active, with more locked and decayed moons.
        let seed = 11u64;
        let radius_m = super::super::plates::EARTH_RADIUS_M;
        let young = 0.5;
        let old = 8.0;
        // Craters pile up (age-explicit relief, same directions).
        let cratered = |age: f64| {
            let mut sum = 0.0;
            let mut count = 0u32;
            for dir in super::super::relief::sweep_directions(64) {
                let relief = super::super::craters::crater_relief_for_age(
                    seed,
                    dir,
                    radius_m,
                    age,
                    super::super::classify::WorldKind::EarthLike,
                );
                sum += relief.abs();
                count += u32::from(relief.abs() > 50.0);
            }
            (sum, count)
        };
        let (young_sum, young_count) = cratered(young);
        let (old_sum, old_count) = cratered(old);
        assert!(old_sum > young_sum * 1.5, "craters do not pile up");
        assert!(old_count > young_count, "cratered ground does not spread");
        // Erosion deepens, activity fades.
        assert!(erosion_for(old) > erosion_for(young));
        assert!(activity_for(1.0, old) < activity_for(1.0, young));
        // Moons lock and decay with age.
        let giant = PlanetSpec {
            mass_earth: 318.0,
            radius_earth: 11.2,
            kind: super::super::classify::WorldKind::GasGiant,
            orbit_au: 5.2,
            star_mass_solar: 1.0,
            age_gyr: young,
            luminosity: 1.0,
            spin_hours: 60.0,
        };
        let young_system = super::super::moons::system_for(seed, &giant);
        let old_system = super::super::moons::system_for(
            seed,
            &PlanetSpec {
                age_gyr: old,
                ..giant
            },
        );
        let locked = |system: &super::super::moons::MoonSystem| {
            system.moons.iter().filter(|moon| moon.locked).count()
        };
        assert!(locked(&old_system) >= locked(&young_system));
        assert!(locked(&old_system) > 0, "nothing ever locks");
        assert!(
            old_system.moons.len() <= young_system.moons.len(),
            "age adds moons"
        );
        assert!(
            old_system.lost_to_rings >= young_system.lost_to_rings,
            "decay does not grow"
        );
    }
}
