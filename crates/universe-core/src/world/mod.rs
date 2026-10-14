//! Generated worlds (#685, Spec v2): every planet and moon from its seed
//! and its system's age.
//!
//! The research is in `guides/universe/L10/generation/`; each module names
//! its guide page. Everything here is pure `f64` and deterministic
//! (`E-DET-TIERS`): the same seed and age always give the same world. The
//! classic universe (Planet look Normal or Squares) never reads this
//! module; the generated one ([`crate::look::WorldMode::Generated`])
//! builds a [`WorldRecipe`](crate::world::WorldRecipe) per planet on top of
//! the classic cells.

pub mod classify;
pub mod noise3;
pub mod plates;
pub mod relief;
pub mod star;

pub use classify::WorldKind;

use crate::r#gen::PlanetInfo;

/// Everything the generator decides about one planet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldRecipe {
    /// The planet's type.
    pub kind: WorldKind,
    /// Mass in Earth masses.
    pub mass_earth: f64,
    /// Radius in Earth radii.
    pub radius_earth: f64,
    /// Surface gravity in m/s².
    pub gravity: f64,
    /// Equilibrium temperature in kelvin, at the system's age.
    pub t_eq_k: f64,
    /// The system's age in billions of years.
    pub age_gyr: f64,
    /// The star's mass in Sun masses (0 for a rogue).
    pub star_mass: f64,
    /// The star's luminosity at that age, in Sun units.
    pub luminosity: f64,
    /// Orbit in AU.
    pub orbit_au: f64,
    /// Iron share, water share (composition draws, 0..1).
    pub iron: f64,
    /// Water share, 0..1.
    pub water: f64,
}

/// Bond albedo assumed for the equilibrium temperature.
const ALBEDO: f64 = 0.3;

/// The recipe of the planet whose L9 data is `planet`, in the system
/// seeded `system_seed`, drawn from `planet_seed`. The star's mass comes
/// from the orbit by Kepler's law (the Sun exactly at home); the system's
/// age is seeded per system (the Sun's age at home).
#[must_use]
pub fn planet_recipe(
    planet_seed: u64,
    planet: &PlanetInfo,
    system_seed: u64,
    home: bool,
) -> WorldRecipe {
    let star_mass = if home {
        1.0
    } else {
        star::kepler_mass(planet.orbit_au, planet.period_days)
    };
    let age_gyr = star::system_age_gyr(system_seed, star_mass, home);
    let luminosity = star::luminosity(star_mass, age_gyr);
    let setting = classify::Setting {
        t_eq_k: star::equilibrium_k(luminosity, planet.orbit_au, ALBEDO),
        orbit_au: planet.orbit_au,
        ice_line_au: star::ice_line_au(luminosity),
        rogue: false,
    };
    build(planet_seed, &setting, age_gyr, star_mass, luminosity)
}

/// The recipe of a body drawn from `seed` in `setting`.
#[must_use]
pub fn build(
    seed: u64,
    setting: &classify::Setting,
    age_gyr: f64,
    star_mass: f64,
    luminosity: f64,
) -> WorldRecipe {
    let body = classify::draw_body(seed);
    let kind = classify::classify(&body, setting);
    WorldRecipe {
        kind,
        mass_earth: body.mass_earth,
        radius_earth: body.radius_earth,
        gravity: 9.81 * body.mass_earth / (body.radius_earth * body.radius_earth),
        t_eq_k: setting.t_eq_k,
        age_gyr,
        star_mass,
        luminosity,
        orbit_au: setting.orbit_au,
        iron: body.iron,
        water: body.water,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::{Rng, hash_triple};

    /// A seeded spread of settings: star masses from the stellar mix,
    /// template orbits, system ages, and a few rogues.
    fn sampled(seed: u64) -> WorldRecipe {
        let mut rng = Rng::new(hash_triple(seed, 0x7E57, 0));
        let star = crate::system::sample_star(&mut rng);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: test orbit slot 0-7 from a 0..1 draw, saturates like `as`"
        )]
        #[expect(
            clippy::cast_sign_loss,
            reason = "E-CAST: test draw is non-negative, saturates like `as`"
        )]
        let slot = (rng.next_f64() * 8.0) as usize;
        let orbit_au = crate::system::template_axis(slot, 8);
        let age = star::system_age_gyr(seed, star.mass_solar, false);
        let lum = star::luminosity(star.mass_solar, age);
        let rogue = rng.next_f64() < 0.03;
        let setting = classify::Setting {
            t_eq_k: if rogue {
                35.0
            } else {
                star::equilibrium_k(lum, orbit_au, ALBEDO)
            },
            orbit_au,
            ice_line_au: star::ice_line_au(lum),
            rogue,
        };
        build(seed, &setting, age, star.mass_solar, lum)
    }

    #[test]
    fn every_planet_and_moon_type_appears() {
        // C4 / AC4 (planet half; moons arrive with T10): over a fixed list
        // of seeds, every planet type appears at least once.
        let mut seen = std::collections::BTreeSet::new();
        for seed in 0..6_000u64 {
            seen.insert(WorldKind::ALL.iter().position(|k| *k == sampled(seed).kind));
        }
        let missing: Vec<&str> = WorldKind::ALL
            .iter()
            .enumerate()
            .filter(|(i, _)| !seen.contains(&Some(*i)))
            .map(|(_, k)| k.label())
            .collect();
        assert!(missing.is_empty(), "never generated: {missing:?}");
    }

    #[test]
    fn generation_is_deterministic_for_seed_and_age() {
        // C2 / AC2 (recipe part): same inputs, same recipe; a different
        // seed changes it.
        let earth_orbit = PlanetInfo {
            radius_earth: 1.0,
            orbit_au: 1.0,
            period_days: 365.25,
        };
        let a = planet_recipe(42, &earth_orbit, 9, false);
        assert_eq!(a, planet_recipe(42, &earth_orbit, 9, false));
        assert_ne!(a, planet_recipe(43, &earth_orbit, 9, false));
        let home = planet_recipe(42, &earth_orbit, 9, true);
        assert_eq!(home.star_mass, 1.0);
        assert_eq!(home.age_gyr, star::SUN_AGE_GYR);
        assert!(home.gravity.is_finite() && home.gravity > 0.0);
        for seed in 0..200 {
            let r = sampled(seed);
            assert_eq!(r, sampled(seed));
            assert!(r.radius_earth > 0.0 && r.t_eq_k.is_finite());
        }
    }
}
