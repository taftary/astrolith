//! Gravity from every massive body around the player (#559).
//!
//! Real physics (owner answer Q1 A): every body the universe gives a mass
//! to pulls by `G M / d^2` with its real mass and its real distance (owner
//! answer Q3 A): stars and gas clouds from their generated solar masses,
//! planets and moons from their sizes at typical densities, galaxies and
//! galaxy clusters from typical masses. At L10 the planet pulls from its
//! centre, so its surface gravity is real (Earth
//! about 9.81 m/s2). [`pull`](crate::gravity::pull) sums every pull and names the strongest.
//! Pure and deterministic (`E-CORE-NO-BEVY`, `E-DET-TIERS`).

use crate::coords::Level;
use crate::r#gen::{Environment, GalaxyKind, Point};

/// Newton's gravitational constant, m3 kg-1 s-2 (CODATA 2018).
pub const G: f64 = 6.674_30e-11;

/// The Sun's mass, kg (IAU nominal).
pub const SOLAR_MASS_KG: f64 = 1.988_47e30;

/// The Earth's mass, kg.
pub const EARTH_MASS_KG: f64 = 5.972_2e24;

/// The Earth's equatorial radius, m.
pub const EARTH_RADIUS_M: f64 = 6.378_137e6;

/// The Moon's mass, kg, and mean radius, m (moons share its density).
pub const MOON_MASS_KG: f64 = 7.342e22;
/// The Moon's mean radius, m.
pub const MOON_RADIUS_M: f64 = 1.737_4e6;

/// Planets at or above this radius (Earth radii) are giants at Jupiter's
/// density relative to Earth's (318 Earth masses at 11.2 Earth radii).
pub const GIANT_RADIUS_EARTH: f64 = 2.0;
/// Giant density relative to Earth's (Jupiter).
pub const GIANT_DENSITY: f64 = 0.226;

/// Typical masses in solar masses for bodies without a generated mass:
/// spiral and elliptical galaxies (Milky Way scale), irregular galaxies,
/// galaxy groups (wall nodes), clusters (filaments), and rich clusters
/// (web nodes).
pub const GALAXY_MASS_SOLAR: f64 = 1.0e12;
/// Irregular galaxy mass, solar masses.
pub const IRREGULAR_MASS_SOLAR: f64 = 1.0e10;
/// Galaxy group mass, solar masses.
pub const GROUP_MASS_SOLAR: f64 = 1.0e13;
/// Galaxy cluster mass, solar masses.
pub const CLUSTER_MASS_SOLAR: f64 = 1.0e14;
/// Rich cluster mass, solar masses.
pub const RICH_CLUSTER_MASS_SOLAR: f64 = 1.0e15;

/// Pulls weaker than this (m/s2) count as almost zero for display.
pub const FAINT_PULL: f64 = 1.0e-6;

/// Real mass of a generated body in kg, or `None` when it has none.
#[must_use]
pub fn mass_kg(point: &Point) -> Option<f64> {
    if let Some(star) = point.star {
        return Some(star.mass_solar * SOLAR_MASS_KG);
    }
    if let Some(cloud) = point.cloud {
        return Some(cloud.mass_solar * SOLAR_MASS_KG);
    }
    if let Some(planet) = point.planet {
        return Some(planet_mass_kg(planet.radius_earth));
    }
    if let Some(moon) = point.moon {
        let r = moon.radius_km * 1.0e3 / MOON_RADIUS_M;
        return Some(MOON_MASS_KG * r * r * r);
    }
    if let Some(galaxy) = point.galaxy {
        return Some(
            match galaxy.kind {
                GalaxyKind::Spiral | GalaxyKind::Elliptical => GALAXY_MASS_SOLAR,
                GalaxyKind::Irregular => IRREGULAR_MASS_SOLAR,
            } * SOLAR_MASS_KG,
        );
    }
    match point.environment {
        Some(Environment::Wall) => Some(GROUP_MASS_SOLAR * SOLAR_MASS_KG),
        Some(Environment::Filament) => Some(CLUSTER_MASS_SOLAR * SOLAR_MASS_KG),
        Some(Environment::Node) => Some(RICH_CLUSTER_MASS_SOLAR * SOLAR_MASS_KG),
        Some(Environment::Void) | None => None,
    }
}

/// A planet's mass in kg from its radius in Earth radii.
#[must_use]
pub fn planet_mass_kg(radius_earth: f64) -> f64 {
    let density = if radius_earth >= GIANT_RADIUS_EARTH {
        GIANT_DENSITY
    } else {
        1.0
    };
    EARTH_MASS_KG * density * radius_earth * radius_earth * radius_earth
}

/// The summed pull at a point and the body pulling hardest.
#[derive(Debug, Clone, PartialEq)]
pub struct Pull {
    /// Summed acceleration in open-cell units per second squared.
    pub accel: [f64; 3],
    /// Strength of the strongest single pull, m/s2.
    pub strongest: f64,
    /// Name of the body pulling hardest (empty when nothing pulls).
    pub name: String,
    /// Unit direction toward the strongest body, open-cell units.
    pub toward: [f64; 3],
}

/// One metre in `level` units (a cell spans `10^e` metres).
fn cells_per_metre(level: Level) -> f64 {
    10f64.powf(-crate::nest::scale_exponent(level))
}

/// Adds one body's pull at `at` to `out`: `mass` in kg at `centre` (cell
/// units), `metres_per_cell` turning cell distances into metres (and the
/// acceleration back into cells, so each body's geometry stays
/// consistent and orbits close), distances held at least `floor` cells (no
/// pull blows up inside a body).
#[allow(
    clippy::too_many_arguments,
    reason = "one body's physical inputs; a struct would only rename them"
)]
fn add_pull(
    out: &mut Pull,
    at: [f64; 3],
    centre: [f64; 3],
    mass: f64,
    metres_per_cell: f64,
    floor: f64,
    name: impl FnOnce() -> String,
) {
    let to = [centre[0] - at[0], centre[1] - at[1], centre[2] - at[2]];
    let cells = (to[0] * to[0] + to[1] * to[1] + to[2] * to[2]).sqrt();
    if !cells.is_finite() || cells <= 0.0 || !mass.is_finite() || mass <= 0.0 {
        return;
    }
    let metres = cells.max(floor) * metres_per_cell;
    let strength = G * mass / (metres * metres);
    if !strength.is_finite() {
        return;
    }
    let direction = [to[0] / cells, to[1] / cells, to[2] / cells];
    let accel = strength / metres_per_cell;
    out.accel = [
        out.accel[0] + direction[0] * accel,
        out.accel[1] + direction[1] * accel,
        out.accel[2] + direction[2] * accel,
    ];
    if strength > out.strongest {
        out.strongest = strength;
        out.name = name();
        out.toward = direction;
    }
}

/// The summed pull at `at` (open-cell units) in `universe` (#559).
///
/// Every massive body in the open cell pulls with its real mass at its real
/// distance; at L10 the planet itself pulls from the cell centre, scaled so
/// its former surface radius carries its real surface gravity. Pure.
#[must_use]
pub fn pull(universe: &crate::nav::Universe, at: [f64; 3]) -> Pull {
    let level = universe.level();
    let cells_per_m = cells_per_metre(level);
    let metres_per_cell = 1.0 / cells_per_m;
    let mut out = Pull {
        accel: [0.0; 3],
        strongest: 0.0,
        name: String::new(),
        toward: [0.0; 3],
    };
    // The planet under the body (L10).
    if let Some(radius_cells) = crate::frame::planet_radius_cells(level) {
        let chain = universe.path.indices();
        if let Some(l10) = chain.get(..9) {
            let seed = crate::nest::path_seed(universe.root.0, l10);
            let home = crate::home::is_home_planet_cell(universe.root.0, l10);
            let recipe = crate::terrain::body_recipe(seed, home);
            let radius_m = recipe.radius_earth * EARTH_RADIUS_M;
            let centre = [0.0; 3];
            // Distances on the planet scale so the drawn radius is its
            // real radius: real surface gravity at the drawn surface.
            let planet_metres_per_cell = radius_m / radius_cells;
            add_pull(
                &mut out,
                at,
                centre,
                planet_mass_kg(recipe.radius_earth),
                planet_metres_per_cell,
                radius_cells,
                || {
                    if home {
                        "Earth".to_string()
                    } else {
                        "planet".to_string()
                    }
                },
            );
        }
    }
    let chain = universe.path.indices();
    for (index, point) in universe.open.points.iter().enumerate() {
        let Some(mass) = mass_kg(point) else {
            continue;
        };
        let Ok(marker) = u32::try_from(index) else {
            continue;
        };
        add_pull(
            &mut out,
            at,
            point.position,
            mass,
            metres_per_cell,
            point.radius,
            || crate::names::marker_name(universe.root.0, &chain, level, marker, point),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opened(depth: usize) -> crate::nav::Universe {
        let chain = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
        let mut universe = crate::nav::Universe::new(crate::nav::DEMO_SEED);
        for &marker in chain.get(..depth).expect("journey depth") {
            assert!(universe.open(crate::nav::MarkerIndex(marker)));
        }
        universe
    }

    #[test]
    fn earth_pulls_nine_point_eight_at_its_surface() {
        // The demo journey's planet is Earth (the home planet).
        let universe = opened(9);
        assert_eq!(universe.level().get(), 10);
        let surface = [0.0, crate::frame::L10_PLANET_RADIUS, 0.0];
        let pull = pull(&universe, surface);
        assert_eq!(pull.name, "Earth");
        assert!((pull.strongest - 9.81).abs() < 0.05, "{}", pull.strongest);
        assert!(pull.toward[1] < -0.99, "the pull points to the centre");
        // Twice as far, a quarter of the pull.
        let far = pull_at(&universe, [0.0, 2.0 * crate::frame::L10_PLANET_RADIUS, 0.0]);
        assert!(
            (far * 4.0 - pull.strongest).abs() < 0.05,
            "inverse square: {far}"
        );
    }

    fn pull_at(universe: &crate::nav::Universe, at: [f64; 3]) -> f64 {
        pull(universe, at).strongest
    }

    #[test]
    fn the_ground_of_a_region_still_pulls_like_the_planet() {
        // #770: region portals removed; pull at the former surface height.
        let universe = opened(9);
        assert_eq!(universe.level().get(), 10);
        // #770: former surface height; pull stays 9.81 downward.
        let at = [0.0, crate::frame::L10_PLANET_RADIUS, 0.0];
        let pull = pull(&universe, at);
        assert_eq!(pull.name, "Earth");
        assert!((pull.strongest - 9.81).abs() < 0.05, "{}", pull.strongest);
        let length = (at[0] * at[0] + at[1] * at[1] + at[2] * at[2]).sqrt();
        let along =
            (pull.toward[0] * at[0] + pull.toward[1] * at[1] + pull.toward[2] * at[2]) / length;
        assert!(
            along < -0.99,
            "the sum pulls down toward the planet: {along}"
        );
    }

    #[test]
    fn between_the_stars_the_pull_is_tiny() {
        let universe = opened(5);
        let level = universe.level().get();
        let pull = pull(&universe, [0.31, -0.22, 0.17]);
        assert!(pull.strongest < FAINT_PULL, "L{level}: {}", pull.strongest);
    }

    #[test]
    fn masses_follow_their_bodies() {
        assert!((planet_mass_kg(1.0) - EARTH_MASS_KG).abs() < 1.0);
        let jupiter = planet_mass_kg(11.2) / EARTH_MASS_KG;
        assert!(
            (jupiter - 318.0).abs() < 10.0,
            "a giant at Jupiter's mass: {jupiter}"
        );
        let moon = Point {
            moon: Some(crate::r#gen::MoonInfo {
                radius_km: 1_737.4,
                orbit_km: 384_400.0,
                period_days: 27.3,
            }),
            ..Point::bare([0.0; 3], 0.005, crate::r#gen::MarkerKind::Population)
        };
        assert!((mass_kg(&moon).expect("a moon has mass") - MOON_MASS_KG).abs() < 1e18);
        let empty = Point::bare([0.0; 3], 0.01, crate::r#gen::MarkerKind::Portal);
        assert!(mass_kg(&empty).is_none(), "a bare point has no mass");
    }
}
