//! What a solid world's surface is made of (#1161, Spec v2 Q1 A, Q2 A).
//!
//! The type comes from the world's physics, in the order of the
//! `planet-types.md` classifier (Step 4, first match wins): the equilibrium
//! temperature from the host star's brightness and the orbit, the ice line,
//! the size, whether it keeps air, and seeded iron and water fractions from
//! the planet's path seed. The same inputs always give the same type
//! (`E-DET-TIERS`). Ocean worlds are drawn as their nearest solid type until
//! #1162 brings water, and carbon worlds are left out (Q1 A). Dwarf-sized
//! worlds and moons use the guide's dwarf row, [`crate::world_type::WorldType::Barren`].
//!
//! Palettes are sRGB tones from the rocks catalog's families (basalt, iron
//! oxide, sand, water ice, granite), as `planet-types.md` "Colour from
//! composition" asks. Pure and Bevy-free (`E-CORE-NO-BEVY`).

use crate::seed::hash_triple;

/// Radius in Earth radii from which a world is a smooth giant (as
/// [`crate::relief`]).
pub const GIANT_RADIUS_EARTH: f64 = 11.0;

/// Radius in Earth radii under which a world is dwarf-sized (the guide's
/// "about 0.3 Earth radii").
pub const DWARF_RADIUS_EARTH: f64 = 0.3;

/// Bond albedo used for the classifier's equilibrium temperature.
pub const CLASSIFY_ALBEDO: f64 = 0.3;

/// Equilibrium temperature in kelvin above which a rocky world is lava.
pub const LAVA_FROM_K: f64 = 1_200.0;

/// Upper equilibrium temperature in kelvin of the Earth-like band.
pub const TEMPERATE_TOP_K: f64 = 320.0;

/// Lower equilibrium temperature in kelvin of the Earth-like band.
pub const TEMPERATE_BOTTOM_K: f64 = 200.0;

/// Iron fraction above which a rocky world is an iron world.
pub const IRON_FROM: f64 = 0.6;

/// Water fraction above which a world formed beyond the ice line is icy.
pub const ICY_WATER_FROM: f64 = 0.5;

/// Water fraction under which a temperate world is a desert.
pub const DRY_WATER_UNDER: f64 = 0.2;

/// Radius in Earth radii from which a world is in the Neptunian regime
/// (the generator leaves the 1.5-2 valley empty): never an iron world, and
/// an ice giant when formed beyond the ice line (drawn as ice, since the
/// game gives it ground).
pub const NEPTUNIAN_RADIUS_EARTH: f64 = 2.0;

/// Ice line around a Sun-like star in AU (zero albedo, ice at 170 K).
pub const ICE_LINE_SUN_AU: f64 = 2.7;

/// Seed tag of the iron-fraction draw.
const TAG_IRON: u64 = 0x1161_0001;

/// Seed tag of the water-fraction draw.
const TAG_WATER: u64 = 0x1161_0002;

/// One solid world's surface type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldType {
    /// Green-brown and grey rock, snow up high, white caps.
    EarthLike,
    /// Ochre to red sand and rock, small polar caps.
    Desert,
    /// Dark grey metal tones, many craters.
    Iron,
    /// White to blue-grey ice, cracks and ridges.
    Ice,
    /// Black crust with glowing cracks and low lava fields.
    Lava,
    /// Grey to brownish cratered rock: dwarf-sized worlds and moons.
    Barren,
}

impl WorldType {
    /// Every type, in a fixed order (fixtures and histograms).
    pub const ALL: [WorldType; 6] = [
        WorldType::EarthLike,
        WorldType::Desert,
        WorldType::Iron,
        WorldType::Ice,
        WorldType::Lava,
        WorldType::Barren,
    ];

    /// Short lowercase name (shot keys, listings).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            WorldType::EarthLike => "earth-like",
            WorldType::Desert => "desert",
            WorldType::Iron => "iron",
            WorldType::Ice => "ice",
            WorldType::Lava => "lava",
            WorldType::Barren => "barren",
        }
    }

    /// The type named `name`, if any.
    #[must_use]
    pub fn from_name(name: &str) -> Option<WorldType> {
        WorldType::ALL.into_iter().find(|t| t.name() == name)
    }

    /// This type's palette.
    #[must_use]
    pub fn palette(self) -> Palette {
        palette_of(self)
    }
}

/// A type's ground tones in sRGB `0..1`, and its own light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// Low, flat ground.
    pub low: [f64; 3],
    /// High ground.
    pub high: [f64; 3],
    /// Steep slopes (bare rock or metal).
    pub steep: [f64; 3],
    /// Caps: snow, frost or pale ground near the poles and up high.
    pub cap: [f64; 3],
    /// Own light of cracks and low lava fields (black when none).
    pub glow: [f64; 3],
    /// Share of the polar region covered by caps (`0` none, `1` all).
    pub cap_reach: f64,
}

/// The fixed palette of `kind`.
fn palette_of(kind: WorldType) -> Palette {
    const NONE: [f64; 3] = [0.0, 0.0, 0.0];
    match kind {
        WorldType::EarthLike => Palette {
            low: [0.33, 0.37, 0.22],
            high: [0.45, 0.39, 0.30],
            steep: [0.42, 0.41, 0.40],
            cap: [0.92, 0.93, 0.95],
            glow: NONE,
            cap_reach: 0.25,
        },
        WorldType::Desert => Palette {
            low: [0.76, 0.56, 0.36],
            high: [0.63, 0.38, 0.24],
            steep: [0.52, 0.33, 0.24],
            cap: [0.90, 0.88, 0.85],
            glow: NONE,
            cap_reach: 0.08,
        },
        WorldType::Iron => Palette {
            low: [0.30, 0.30, 0.31],
            high: [0.36, 0.35, 0.34],
            steep: [0.22, 0.22, 0.23],
            cap: [0.40, 0.40, 0.41],
            glow: NONE,
            cap_reach: 0.0,
        },
        WorldType::Ice => Palette {
            low: [0.86, 0.90, 0.95],
            high: [0.74, 0.80, 0.88],
            steep: [0.55, 0.62, 0.72],
            cap: [0.95, 0.97, 1.0],
            glow: NONE,
            cap_reach: 1.0,
        },
        WorldType::Lava => Palette {
            // Fresh basalt: albedo about 0.05 to 0.1 (rocks catalog), dark
            // but not black in daylight.
            low: [0.22, 0.20, 0.19],
            high: [0.28, 0.26, 0.24],
            steep: [0.18, 0.17, 0.16],
            cap: [0.30, 0.28, 0.26],
            glow: [1.0, 0.36, 0.06],
            cap_reach: 0.0,
        },
        WorldType::Barren => Palette {
            low: [0.42, 0.41, 0.40],
            high: [0.55, 0.54, 0.52],
            steep: [0.35, 0.34, 0.33],
            cap: [0.62, 0.61, 0.60],
            glow: NONE,
            cap_reach: 0.0,
        },
    }
}

/// The physics the classifier reads for one world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldInputs {
    /// Path seed of the world's cell (for the iron and water draws).
    pub seed: u64,
    /// Radius in Earth radii.
    pub radius_earth: f64,
    /// Orbit around the star in AU (for a moon, its planet's orbit).
    pub orbit_au: f64,
    /// Host star mass in solar masses.
    pub star_mass_solar: f64,
    /// Whether the world keeps an atmosphere.
    pub has_air: bool,
    /// The home planet (always Earth-like, Q3 A).
    pub home: bool,
}

/// Main-sequence brightness in solar units for `mass_solar` (piecewise
/// mass-luminosity relation, `L ~ M^4` near the Sun).
#[must_use]
pub fn star_luminosity_solar(mass_solar: f64) -> f64 {
    if !mass_solar.is_finite() || mass_solar <= 0.0 {
        return 1.0;
    }
    if mass_solar < 0.43 {
        0.23 * mass_solar.powf(2.3)
    } else if mass_solar < 2.0 {
        mass_solar.powi(4)
    } else {
        1.4 * mass_solar.powf(3.5)
    }
}

/// Equilibrium temperature in kelvin (`planet-types.md` Step 2):
/// `278 K * L^(1/4) * (1 - A)^(1/4) / sqrt(a)`.
#[must_use]
pub fn equilibrium_k(luminosity_solar: f64, orbit_au: f64, albedo: f64) -> f64 {
    if !orbit_au.is_finite() || orbit_au <= 0.0 || !luminosity_solar.is_finite() {
        return f64::NAN;
    }
    let absorbed = (1.0 - albedo.clamp(0.0, 1.0)).max(0.0);
    278.0 * luminosity_solar.max(0.0).powf(0.25) * absorbed.powf(0.25) / orbit_au.sqrt()
}

/// Ice line in AU for a star of `luminosity_solar` (`2.7 AU * sqrt(L)`).
#[must_use]
pub fn ice_line_au(luminosity_solar: f64) -> f64 {
    ICE_LINE_SUN_AU * luminosity_solar.max(0.0).sqrt()
}

/// A seeded fraction in `[0, 1)` for `tag` under `seed`.
fn draw(seed: u64, tag: u64) -> f64 {
    let bits = hash_triple(seed, tag, 0) >> 11;
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: top 53 bits are exactly representable in f64"
    )]
    let unit = bits as f64 / (1u64 << 53) as f64;
    unit
}

/// Iron mass fraction: Earth-like `0.2`-`0.35` mostly, rarely over
/// [`IRON_FROM`] (about 7 % of worlds).
#[must_use]
pub fn iron_fraction(seed: u64) -> f64 {
    0.2 + 0.5 * draw(seed, TAG_IRON).powi(3)
}

/// Water fraction draw in `[0, 1)`.
#[must_use]
pub fn water_fraction(seed: u64) -> f64 {
    draw(seed, TAG_WATER)
}

/// The surface type of a world, or `None` for a smooth giant.
///
/// First match wins, as the guide's table: home, giant, lava, dwarf, ice
/// giant (Neptunian beyond the ice line), iron (rocky only), icy beyond the
/// ice line, cold, hot or dry, temperate. A world without
/// air is never Earth-like (it becomes a desert, or ice when cold).
#[must_use]
pub fn classify(inputs: &WorldInputs) -> Option<WorldType> {
    let radius = inputs.radius_earth;
    if inputs.home {
        return Some(WorldType::EarthLike);
    }
    if !radius.is_finite() || radius >= GIANT_RADIUS_EARTH {
        return None;
    }
    let luminosity = star_luminosity_solar(inputs.star_mass_solar);
    let t_eq = equilibrium_k(luminosity, inputs.orbit_au, CLASSIFY_ALBEDO);
    let t = if t_eq.is_finite() { t_eq } else { 255.0 };
    if t > LAVA_FROM_K {
        return Some(WorldType::Lava);
    }
    if radius < DWARF_RADIUS_EARTH {
        return Some(if t < TEMPERATE_BOTTOM_K {
            WorldType::Ice
        } else {
            WorldType::Barren
        });
    }
    let beyond_ice = inputs.orbit_au > ice_line_au(luminosity);
    if radius >= NEPTUNIAN_RADIUS_EARTH && beyond_ice {
        return Some(WorldType::Ice);
    }
    if radius < NEPTUNIAN_RADIUS_EARTH && iron_fraction(inputs.seed) > IRON_FROM {
        return Some(WorldType::Iron);
    }
    let water = water_fraction(inputs.seed);
    if beyond_ice && water > ICY_WATER_FROM {
        // Warm ocean worlds are drawn as their nearest solid type (#1162).
        return Some(if t >= TEMPERATE_BOTTOM_K && inputs.has_air {
            WorldType::EarthLike
        } else {
            WorldType::Ice
        });
    }
    if t < TEMPERATE_BOTTOM_K {
        return Some(WorldType::Ice);
    }
    if t > TEMPERATE_TOP_K || water < DRY_WATER_UNDER || !inputs.has_air {
        return Some(WorldType::Desert);
    }
    Some(WorldType::EarthLike)
}

/// The surface type of a moon of `moon_radius_earth` around a planet whose
/// orbit and star are in `planet` (Q7 A): moons keep no air, so they are
/// barren rock, ice when cold or formed icy, lava when very hot.
#[must_use]
pub fn classify_moon(planet: &WorldInputs, moon_seed: u64, moon_radius_earth: f64) -> WorldType {
    if planet.home {
        return WorldType::Barren;
    }
    let luminosity = star_luminosity_solar(planet.star_mass_solar);
    let t_eq = equilibrium_k(luminosity, planet.orbit_au, CLASSIFY_ALBEDO);
    let t = if t_eq.is_finite() { t_eq } else { 255.0 };
    if t > LAVA_FROM_K {
        return WorldType::Lava;
    }
    let beyond_ice = planet.orbit_au > ice_line_au(luminosity);
    if t < TEMPERATE_BOTTOM_K || (beyond_ice && water_fraction(moon_seed) > ICY_WATER_FROM) {
        return WorldType::Ice;
    }
    if moon_radius_earth >= DWARF_RADIUS_EARTH && iron_fraction(moon_seed) > IRON_FROM {
        return WorldType::Iron;
    }
    WorldType::Barren
}

/// The classifier inputs of the L10 planet at `chain` under `root`, or
/// `None` when the chain is not an L10 planet with a star above it.
#[must_use]
pub fn planet_inputs(root: u64, chain: &[u32]) -> Option<WorldInputs> {
    use crate::nest::{generate_cell, path_level};
    if path_level(chain).get() != 10 {
        return None;
    }
    let (marker, parent) = chain.split_last()?;
    let cell = generate_cell(root, parent);
    let planet = cell.points.get(*marker as usize)?.planet?;
    let star = cell.points.iter().find_map(|point| point.star)?;
    let recipe = crate::terrain::planet_recipe(root, chain);
    Some(WorldInputs {
        seed: crate::nest::path_seed(root, chain),
        radius_earth: recipe.radius_earth,
        orbit_au: planet.orbit_au,
        star_mass_solar: star.mass_solar,
        has_air: recipe.air.is_some(),
        home: crate::home::is_home_planet_cell(root, chain),
    })
}

/// The surface type of the L10 planet at `chain` under `root`.
#[must_use]
pub fn planet_type(root: u64, chain: &[u32]) -> Option<WorldType> {
    planet_inputs(root, chain).and_then(|inputs| classify(&inputs))
}

/// One solid or giant world found by [`list_worlds`].
#[derive(Debug, Clone, PartialEq)]
pub struct ListedWorld {
    /// Full marker chain of the L10 planet cell.
    pub chain: Vec<u32>,
    /// The classifier inputs.
    pub inputs: WorldInputs,
    /// The type, `None` for a smooth giant.
    pub kind: Option<WorldType>,
    /// Equilibrium temperature in kelvin.
    pub t_eq_k: f64,
}

/// Up to `limit` L10 planets under the cell at `start` (any level up to 9),
/// depth first through portal markers in marker order: deterministic, so a
/// listing names the same worlds on every run (#1161 T1 fixtures).
#[must_use]
pub fn list_worlds(root: u64, start: &[u32], limit: usize) -> Vec<ListedWorld> {
    let mut out = Vec::new();
    let mut chain = start.to_vec();
    walk(root, &mut chain, limit, &mut out);
    out
}

/// Depth-first step of [`list_worlds`].
fn walk(root: u64, chain: &mut Vec<u32>, limit: usize, out: &mut Vec<ListedWorld>) {
    use crate::r#gen::MarkerKind;
    use crate::nest::{generate_cell, path_level};
    let level = path_level(chain).get();
    if out.len() >= limit || level > 9 {
        return;
    }
    let cell = generate_cell(root, chain);
    for (marker, point) in (0u32..).zip(&cell.points) {
        if out.len() >= limit {
            return;
        }
        if point.kind != MarkerKind::Portal {
            continue;
        }
        chain.push(marker);
        if level == 9 {
            if let Some(inputs) = planet_inputs(root, chain) {
                let luminosity = star_luminosity_solar(inputs.star_mass_solar);
                out.push(ListedWorld {
                    chain: chain.clone(),
                    inputs,
                    kind: classify(&inputs),
                    t_eq_k: equilibrium_k(luminosity, inputs.orbit_au, CLASSIFY_ALBEDO),
                });
            }
        } else {
            walk(root, chain, limit, out);
        }
        chain.pop();
    }
}

#[cfg(test)]
#[path = "world_type_tests.rs"]
mod tests;
