//! Stars over time: luminosity by mass and age, and system ages (#685).
//!
//! Guides: `guides/universe/L10/generation/documents/lifetime.md`
//! (brightening by star class, young red dwarfs). The ramps here are the
//! guides' recommendations, tuned so the Sun's row matches the facts: about
//! 0.7 of today's output at birth, about 1.0 at mid-life (the Sun now).

use crate::seed::{Rng, hash_triple};

/// The Sun's age in billions of years (the home system keeps the Sun).
pub const SUN_AGE_GYR: f64 = 4.57;

/// The universe's age in billions of years: no system is older.
pub const UNIVERSE_AGE_GYR: f64 = 13.0;

/// The youngest system drawn, in billions of years.
pub const MIN_SYSTEM_AGE_GYR: f64 = 0.1;

/// Stream tag of the system-age draw.
const AGE_STREAM_TAG: u64 = 0xA6E0_0685_A6E0_0685;

/// Main-sequence lifetime in billions of years: `10 × M^-2.5`, capped at
/// the universe's age for drawing (red dwarfs live far longer).
#[must_use]
pub fn main_sequence_gyr(mass_solar: f64) -> f64 {
    let mass = mass_solar.max(crate::system::MIN_STAR_MASS);
    10.0 * mass.powf(-2.5)
}

/// Luminosity at mid main-sequence life, in Sun units: about `M^3.5`.
#[must_use]
pub fn mid_luminosity(mass_solar: f64) -> f64 {
    mass_solar.max(crate::system::MIN_STAR_MASS).powf(3.5)
}

/// Luminosity at `age_gyr`, in Sun units: `L_mid × (0.7 + 0.6 t / t_MS)`
/// on the main sequence, plus a bright early phase for red dwarfs (under
/// 0.5 Sun masses), fading over a few hundred million years.
#[must_use]
pub fn luminosity(mass_solar: f64, age_gyr: f64) -> f64 {
    let life = main_sequence_gyr(mass_solar);
    let share = (age_gyr.max(0.0) / life).min(1.0);
    let mut lum = mid_luminosity(mass_solar) * (0.7 + 0.6 * share);
    if mass_solar < 0.5 {
        lum *= 1.0 + 4.0 * (-age_gyr.max(0.0) / 0.3).exp();
    }
    lum
}

/// A system's age in billions of years: the Sun's age at home, otherwise
/// a seeded draw from [`MIN_SYSTEM_AGE_GYR`] to the star's main-sequence
/// life (capped at the universe's age).
#[must_use]
pub fn system_age_gyr(system_seed: u64, mass_solar: f64, home: bool) -> f64 {
    if home {
        return SUN_AGE_GYR;
    }
    let top = main_sequence_gyr(mass_solar).clamp(MIN_SYSTEM_AGE_GYR, UNIVERSE_AGE_GYR);
    let mut rng = Rng::new(hash_triple(system_seed, AGE_STREAM_TAG, 0));
    MIN_SYSTEM_AGE_GYR + rng.next_f64() * (top - MIN_SYSTEM_AGE_GYR)
}

/// The water-ice line in AU for a star of luminosity `lum` (Sun units):
/// about 2.7 AU for the Sun, scaled by `sqrt(L)`.
#[must_use]
pub fn ice_line_au(lum: f64) -> f64 {
    2.7 * lum.max(0.0).sqrt()
}

/// Equilibrium temperature in kelvin at `orbit_au` from a star of
/// luminosity `lum`, for Bond albedo `albedo`:
/// `278 K × L^(1/4) × (1 − A)^(1/4) / sqrt(a)`.
#[must_use]
pub fn equilibrium_k(lum: f64, orbit_au: f64, albedo: f64) -> f64 {
    let a = orbit_au.max(1e-6);
    278.0 * lum.max(0.0).powf(0.25) * (1.0 - albedo.clamp(0.0, 1.0)).powf(0.25) / a.sqrt()
}

/// Star mass in Sun masses from an orbit: Kepler's law
/// `M = a³ / P²` with `a` in AU and `P` in years.
#[must_use]
pub fn kepler_mass(orbit_au: f64, period_days: f64) -> f64 {
    let years = period_days / crate::system::DAYS_PER_YEAR;
    if years <= 0.0 || !years.is_finite() {
        return 1.0;
    }
    (orbit_au.powi(3) / (years * years)).max(crate::system::MIN_STAR_MASS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sun_brightens_as_the_guides_say() {
        // 0.7 at birth, about 1.0 at 4.57 Gyr, about +1 % per 100 Myr.
        assert!((luminosity(1.0, 0.0) - 0.7).abs() < 1e-12);
        let now = luminosity(1.0, SUN_AGE_GYR);
        assert!((now - 0.974).abs() < 0.01, "{now}");
        let later = luminosity(1.0, SUN_AGE_GYR + 0.1);
        assert!(((later / now) - 1.0 - 0.006).abs() < 0.003);
        assert!((main_sequence_gyr(1.0) - 10.0).abs() < 1e-9);
        assert!((ice_line_au(1.0) - 2.7).abs() < 1e-12);
        let earth = equilibrium_k(1.0, 1.0, 0.3);
        assert!((earth - 254.0).abs() < 2.0, "{earth}");
    }

    #[test]
    fn star_classes_age_differently() {
        // F stars live short, red dwarfs start bright and settle.
        assert!(main_sequence_gyr(1.4) < 5.0);
        assert!(luminosity(0.2, 0.0) > 3.0 * luminosity(0.2, 2.0));
        let age = system_age_gyr(7, 1.5, false);
        assert!((MIN_SYSTEM_AGE_GYR..=main_sequence_gyr(1.5)).contains(&age));
        assert_eq!(system_age_gyr(7, 1.5, false), age, "seeded");
        assert_eq!(system_age_gyr(7, 1.0, true), SUN_AGE_GYR);
        assert!((kepler_mass(1.0, 365.25) - 1.0).abs() < 1e-12);
    }
}
