//! The planet classifier (#685): numbers first, then the type.
//!
//! Guides: `guides/universe/L10/generation/documents/planet-types.md`.
//! Mass draws the radius (Chen and Kipping 2017, Table 1, read in full:
//! constant 1.008 Earth radii, slopes 0.279, 0.589 and −0.044, breaks at
//! 2.04 Earth masses and 0.414 Jupiter masses, spreads 4, 15 and 7 %); the
//! radius, the equilibrium temperature, the ice line and a few composition
//! draws then pick one [`WorldKind`] in the guides' order (first match
//! wins). Pure and deterministic.

use crate::seed::{Rng, hash_triple};

/// Terran-to-Neptunian break, Earth masses.
pub const TERRAN_MAX_EARTH: f64 = 2.04;

/// Neptunian-to-Jovian break, Earth masses (0.414 Jupiter masses).
pub const NEPTUNIAN_MAX_EARTH: f64 = 131.6;

/// Stream tag of a body's composition and mass draws.
const BODY_STREAM_TAG: u64 = 0xB0D7_0685_B0D7_0685;

/// A world's type (planet-types guide).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldKind {
    /// Mostly metal, cratered, airless.
    Iron,
    /// Water, air and continents.
    EarthLike,
    /// Dry, dusty, thin air.
    Desert,
    /// Hot enough to melt its surface.
    Lava,
    /// Frozen shell.
    Ice,
    /// Global ocean.
    Ocean,
    /// Carbon-rich (theoretical).
    Carbon,
    /// Rocky, heavier than Earth.
    SuperEarth,
    /// Small and round, airless.
    Dwarf,
    /// Hazy gas envelope, no visible ground.
    SubNeptune,
    /// Blue-green, faintly banded giant.
    IceGiant,
    /// Banded gas giant.
    GasGiant,
    /// Giant close to its star.
    HotJupiter,
    /// No star.
    Rogue,
}

impl WorldKind {
    /// Every kind, in classifier-table order.
    pub const ALL: [WorldKind; 14] = [
        WorldKind::Iron,
        WorldKind::EarthLike,
        WorldKind::Desert,
        WorldKind::Lava,
        WorldKind::Ice,
        WorldKind::Ocean,
        WorldKind::Carbon,
        WorldKind::SuperEarth,
        WorldKind::Dwarf,
        WorldKind::SubNeptune,
        WorldKind::IceGiant,
        WorldKind::GasGiant,
        WorldKind::HotJupiter,
        WorldKind::Rogue,
    ];

    /// Its name in readouts.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            WorldKind::Iron => "Iron world",
            WorldKind::EarthLike => "Earth-like",
            WorldKind::Desert => "Desert world",
            WorldKind::Lava => "Lava world",
            WorldKind::Ice => "Ice world",
            WorldKind::Ocean => "Ocean world",
            WorldKind::Carbon => "Carbon world",
            WorldKind::SuperEarth => "Super-Earth",
            WorldKind::Dwarf => "Dwarf planet",
            WorldKind::SubNeptune => "Sub-Neptune",
            WorldKind::IceGiant => "Ice giant",
            WorldKind::GasGiant => "Gas giant",
            WorldKind::HotJupiter => "Hot Jupiter",
            WorldKind::Rogue => "Rogue planet",
        }
    }

    /// Whether the world has a solid ground to draw relief on.
    #[must_use]
    pub const fn has_ground(self) -> bool {
        !matches!(
            self,
            WorldKind::SubNeptune
                | WorldKind::IceGiant
                | WorldKind::GasGiant
                | WorldKind::HotJupiter
        )
    }
}

/// Mean radius in Earth radii for `mass_earth` (Chen and Kipping, joined
/// into one continuous curve).
#[must_use]
pub fn mean_radius(mass_earth: f64) -> f64 {
    let m = mass_earth.max(1e-6);
    if m < TERRAN_MAX_EARTH {
        1.008 * m.powf(0.279)
    } else if m < NEPTUNIAN_MAX_EARTH {
        1.008 * TERRAN_MAX_EARTH.powf(0.279) * (m / TERRAN_MAX_EARTH).powf(0.589)
    } else {
        let at_break = 1.008
            * TERRAN_MAX_EARTH.powf(0.279)
            * (NEPTUNIAN_MAX_EARTH / TERRAN_MAX_EARTH).powf(0.589);
        at_break * (m / NEPTUNIAN_MAX_EARTH).powf(-0.044)
    }
}

/// The radius spread of the regime of `mass_earth` (4, 15 or 7 %).
#[must_use]
pub fn radius_spread(mass_earth: f64) -> f64 {
    if mass_earth < TERRAN_MAX_EARTH {
        0.0403
    } else if mass_earth < NEPTUNIAN_MAX_EARTH {
        0.146
    } else {
        0.0737
    }
}

/// The physical draw of one body before its type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyDraw {
    /// Mass in Earth masses.
    pub mass_earth: f64,
    /// Radius in Earth radii.
    pub radius_earth: f64,
    /// Iron share of a rocky body, 0..1.
    pub iron: f64,
    /// Water share, 0..1.
    pub water: f64,
    /// Carbon-rich star (rare).
    pub carbon: bool,
    /// Whether a Neptunian envelope was stripped (hot orbits).
    pub stripped: bool,
    /// Whether the body migrated inward after forming (wet worlds close in).
    pub migrated: bool,
}

/// Draws a body's mass and composition from `seed` (log-uniform mass from
/// 0.01 to 3,000 Earth masses).
#[must_use]
pub fn draw_body(seed: u64) -> BodyDraw {
    let mut rng = Rng::new(hash_triple(seed, BODY_STREAM_TAG, 0));
    let mass_earth = 10f64.powf(-2.0 + 5.5 * rng.next_f64());
    let spread = radius_spread(mass_earth);
    let scatter = 1.0 + spread * (2.0 * rng.next_f64() - 1.0);
    let radius_earth = (mean_radius(mass_earth) * scatter).max(0.01);
    BodyDraw {
        mass_earth,
        radius_earth,
        iron: rng.next_f64(),
        water: rng.next_f64(),
        carbon: rng.next_f64() < 0.03,
        stripped: rng.next_f64() < 0.5,
        migrated: rng.next_f64() < 0.15,
    }
}

/// Where and around what a body orbits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Setting {
    /// Equilibrium temperature in kelvin.
    pub t_eq_k: f64,
    /// Orbit in AU (0 for a rogue).
    pub orbit_au: f64,
    /// The star's ice line in AU (0 for a rogue).
    pub ice_line_au: f64,
    /// No star.
    pub rogue: bool,
}

/// The type of a body (first match wins, planet-types guide, step 4).
#[must_use]
pub fn classify(body: &BodyDraw, at: &Setting) -> WorldKind {
    let m = body.mass_earth;
    let jovian = m >= NEPTUNIAN_MAX_EARTH;
    let neptunian = (TERRAN_MAX_EARTH..NEPTUNIAN_MAX_EARTH).contains(&m);
    let beyond_ice = at.orbit_au > at.ice_line_au;
    if at.rogue {
        return WorldKind::Rogue;
    }
    if jovian {
        return if at.t_eq_k > 1_000.0 {
            WorldKind::HotJupiter
        } else {
            WorldKind::GasGiant
        };
    }
    if neptunian {
        if beyond_ice {
            return WorldKind::IceGiant;
        }
        if at.t_eq_k > 600.0 && body.stripped {
            return WorldKind::SuperEarth;
        }
        return WorldKind::SubNeptune;
    }
    let wet = body.water > 0.6 && (beyond_ice || body.migrated);
    if body.radius_earth < 0.3 {
        WorldKind::Dwarf
    } else if at.t_eq_k > 1_200.0 {
        WorldKind::Lava
    } else if body.iron > 0.85 {
        WorldKind::Iron
    } else if wet && at.t_eq_k >= 230.0 {
        WorldKind::Ocean
    } else if wet {
        WorldKind::Ice
    } else if body.carbon {
        WorldKind::Carbon
    } else if (200.0..=320.0).contains(&at.t_eq_k) && body.water > 0.25 {
        WorldKind::EarthLike
    } else if at.t_eq_k > 200.0 {
        WorldKind::Desert
    } else {
        WorldKind::Ice
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_radius_follows_chen_and_kipping() {
        assert!((mean_radius(1.0) - 1.008).abs() < 1e-12);
        let at_break = mean_radius(TERRAN_MAX_EARTH);
        assert!((at_break - 1.230).abs() < 0.002, "{at_break}");
        let giant_break = mean_radius(NEPTUNIAN_MAX_EARTH);
        assert!((giant_break - 14.3).abs() < 0.1, "{giant_break}");
        // Continuous at both breaks, flat for giants.
        assert!((mean_radius(TERRAN_MAX_EARTH - 1e-9) - at_break).abs() < 1e-6);
        assert!((mean_radius(317.8) - 13.8).abs() < 0.1);
        assert!(radius_spread(1.0) < radius_spread(10.0));
    }

    #[test]
    fn the_classifier_follows_the_table() {
        let rocky = BodyDraw {
            mass_earth: 1.0,
            radius_earth: 1.0,
            iron: 0.3,
            water: 0.4,
            carbon: false,
            stripped: false,
            migrated: false,
        };
        let warm = Setting {
            t_eq_k: 255.0,
            orbit_au: 1.0,
            ice_line_au: 2.7,
            rogue: false,
        };
        assert_eq!(classify(&rocky, &warm), WorldKind::EarthLike);
        let hot = Setting {
            t_eq_k: 1_500.0,
            orbit_au: 0.03,
            ..warm
        };
        assert_eq!(classify(&rocky, &hot), WorldKind::Lava);
        let giant = BodyDraw {
            mass_earth: 318.0,
            radius_earth: 13.8,
            ..rocky
        };
        assert_eq!(classify(&giant, &warm), WorldKind::GasGiant);
        assert_eq!(classify(&giant, &hot), WorldKind::HotJupiter);
        let rogue = Setting {
            rogue: true,
            ..warm
        };
        assert_eq!(classify(&rocky, &rogue), WorldKind::Rogue);
        assert!(!WorldKind::GasGiant.has_ground() && WorldKind::Desert.has_ground());
    }
}
