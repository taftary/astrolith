//! One star system in one space at true scale (#995 T1, study).
//!
//! Today the system is three cells: L8 maps true orbits onto a cell on a
//! log scale ([`crate::orbits::orbit_radius_cell`]), L9 repeats the planets
//! around the star close-up, and each L10 planet is its own cell. This
//! module rebuilds the same seeded system in one star-centred `f64` frame in
//! metres: the star, every planet on its true orbit at the angle the L9
//! cell gives it (no time yet, #981), and every moon at its true orbit
//! around its planet. Planet radii are the ones the ground uses
//! ([`crate::planet_frame::PlanetFrame::for_planet`]); the L8/L9 radius
//! rides along so the study can count where the two disagree.
//!
//! Pure and deterministic (`E-CORE-NO-BEVY`, `E-DET-TIERS`); the
//! trigonometry stays in core (`E-TRANSCENDENTAL`).

use crate::r#gen::MarkerKind;
use crate::nest::{generate_cell, path_level};
use crate::planet_frame::{EARTH_RADIUS_M, PlanetFrame};

/// One astronomical unit in metres (IAU 2012).
pub const AU_M: f64 = 149_597_870_700.0;

/// The Sun's nominal radius in metres (IAU 2015 B3).
pub const SUN_RADIUS_M: f64 = 695_700_000.0;

/// What a body is, and where it hangs in the system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyKind {
    /// The host star at the origin.
    Star,
    /// A planet: its portal marker in the L9 cell.
    Planet {
        /// Marker index in the L9 cell (the L10 chain's last entry).
        marker: u32,
    },
    /// A moon of the planet at `planet` in [`SystemSpace::bodies`].
    Moon {
        /// Index of the host planet in [`SystemSpace::bodies`].
        planet: usize,
        /// Point index in the planet's L10 cell.
        point: u32,
    },
}

/// One body of the system, in star-centred metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SystemBody {
    /// What it is.
    pub kind: BodyKind,
    /// Centre, star-centred, in metres (ecliptic in the XZ plane, `+Y` up).
    pub position_m: [f64; 3],
    /// Radius in metres.
    pub radius_m: f64,
    /// Orbit radius around its host in metres (0 for the star).
    pub orbit_m: f64,
    /// Orbital period in days (0 for the star).
    pub period_days: f64,
    /// For a planet, its radius in metres as the L8/L9 cells give it.
    pub listed_radius_m: Option<f64>,
}

/// A whole star system in one frame.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemSpace {
    /// Marker chain of the L9 cell (the star close-up) the system comes from.
    pub l9_chain: Vec<u32>,
    /// Host star mass in solar masses.
    pub star_mass_solar: f64,
    /// The star first, then planets in marker order, then moons.
    pub bodies: Vec<SystemBody>,
}

impl SystemSpace {
    /// The planets, in marker order.
    pub fn planets(&self) -> impl Iterator<Item = &SystemBody> {
        self.bodies
            .iter()
            .filter(|body| matches!(body.kind, BodyKind::Planet { .. }))
    }

    /// The moons of every planet.
    pub fn moons(&self) -> impl Iterator<Item = &SystemBody> {
        self.bodies
            .iter()
            .filter(|body| matches!(body.kind, BodyKind::Moon { .. }))
    }
}

/// A star's radius from its mass: the main-sequence rule
/// [`crate::orbits::star_body_radius`] uses, `R ~ M^0.8`, in metres.
#[must_use]
pub fn star_radius_m(mass_solar: f64) -> f64 {
    SUN_RADIUS_M * mass_solar.max(0.0).powf(0.8)
}

/// The system around the L9 cell at `l9_chain` under `root`, in one frame.
///
/// `None` when the chain is not an L9 cell or the cell has no star.
#[must_use]
pub fn system_space(root: u64, l9_chain: &[u32]) -> Option<SystemSpace> {
    if path_level(l9_chain).get() != 9 {
        return None;
    }
    let cell = generate_cell(root, l9_chain);
    let star = cell.points.iter().find_map(|point| point.star)?;
    let mut bodies = vec![SystemBody {
        kind: BodyKind::Star,
        position_m: [0.0; 3],
        radius_m: star_radius_m(star.mass_solar),
        orbit_m: 0.0,
        period_days: 0.0,
        listed_radius_m: None,
    }];
    let mut chain = l9_chain.to_vec();
    for (marker, point) in (0u32..).zip(&cell.points) {
        let Some(planet) = point.planet else {
            continue;
        };
        if point.kind != MarkerKind::Portal {
            continue;
        }
        let orbit_m = planet.orbit_au * AU_M;
        let angle = point.position[2].atan2(point.position[0]);
        chain.push(marker);
        let frame = PlanetFrame::for_planet(root, &chain);
        let host = bodies.len();
        let centre = [orbit_m * angle.cos(), 0.0, orbit_m * angle.sin()];
        bodies.push(SystemBody {
            kind: BodyKind::Planet { marker },
            position_m: centre,
            radius_m: frame.radius_m(),
            orbit_m,
            period_days: planet.period_days,
            listed_radius_m: Some(planet.radius_earth * EARTH_RADIUS_M),
        });
        let planet_cell = generate_cell(root, &chain);
        for (index, moon_point) in (0u32..).zip(&planet_cell.points) {
            let Some(moon) = moon_point.moon else {
                continue;
            };
            let [x, y, z] = moon_point.position;
            let length = (x * x + y * y + z * z).sqrt();
            if length <= 0.0 {
                continue;
            }
            let orbit = moon.orbit_km * 1_000.0;
            bodies.push(SystemBody {
                kind: BodyKind::Moon {
                    planet: host,
                    point: index,
                },
                position_m: [
                    centre[0] + x / length * orbit,
                    centre[1] + y / length * orbit,
                    centre[2] + z / length * orbit,
                ],
                radius_m: moon.radius_km * 1_000.0,
                orbit_m: orbit,
                period_days: moon.period_days,
                listed_radius_m: None,
            });
        }
        chain.pop();
    }
    Some(SystemSpace {
        l9_chain: l9_chain.to_vec(),
        star_mass_solar: star.mass_solar,
        bodies,
    })
}

/// The home system's L9 chain (the journey's star close-up) under `root`.
#[must_use]
pub fn home_l9_chain(root: u64) -> Option<Vec<u32>> {
    crate::home::autopilot_path(root)
        .get(..8)
        .map(<[u32]>::to_vec)
}

/// Up to `count` other systems near the home one: the home galactic
/// structure's (L5) other neighbourhoods, each reached through the first
/// portal of every level down to its star close-up (L9), in marker order.
#[must_use]
pub fn sample_l9_chains(root: u64, count: usize) -> Vec<Vec<u32>> {
    let path = crate::home::autopilot_path(root);
    let (Some(l5), Some(home_l6)) = (path.get(..4), path.get(4)) else {
        return Vec::new();
    };
    let structure = generate_cell(root, l5);
    let mut out = Vec::new();
    for (marker, point) in (0u32..).zip(&structure.points) {
        if out.len() >= count {
            break;
        }
        if marker == *home_l6 || point.kind != MarkerKind::Portal {
            continue;
        }
        let mut chain = l5.to_vec();
        chain.push(marker);
        while path_level(&chain).get() < 9 {
            let cell = generate_cell(root, &chain);
            let Some(next) = (0u32..)
                .zip(&cell.points)
                .find_map(|(m, p)| (p.kind == MarkerKind::Portal).then_some(m))
            else {
                break;
            };
            chain.push(next);
        }
        if path_level(&chain).get() == 9 {
            out.push(chain);
        }
    }
    out
}

#[cfg(test)]
#[path = "system_space_tests.rs"]
mod tests;
