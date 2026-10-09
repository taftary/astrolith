//! L7-L9 system layouts: rings, shells, orbits, and the star body (#384).
//!
//! [`GalaxyGenerator`](crate::sysgen::GalaxyGenerator) owns counts, portal
//! order, and the home fixtures; this module owns where L7-L9 points sit
//! and what forms they wear. Positions follow physical bands mapped through
//! [`orbit_radius_cell`](crate::orbits::orbit_radius_cell); the journey never moves because portal indices and
//! counts are untouched (populations trim from the tail wherever scenery is
//! added). Everything here is pure generation math (`E-TRANSCENDENTAL`:
//! `log10` and `powf` stay in core).

use crate::coords::HALF_BOUND;
use crate::r#gen::{Constraints, Form, MarkerKind, Point, StarInfo, StarKind, clamp_to_sphere};
use crate::seed::{Rng, hash_triple};
use crate::system::{
    OORT_STREAM_TAG, PLANET_STREAM_TAG, sample_star, star_tint, system_planet, system_star,
};
use std::f64::consts::PI;

/// Ecliptic normal: every system plane is the XZ plane, `+Y` up.
pub const ECLIPTIC_NORMAL: [f64; 3] = [0.0, 1.0, 0.0];

/// Innermost mapped orbit for L8 cells, in AU (inside Mercury).
pub const AU_MIN_L8: f64 = 0.3;

/// Outermost mapped orbit for L8 cells, in AU (the heliopause).
pub const AU_MAX_L8: f64 = 120.0;

/// Innermost mapped orbit for L7 cells, in AU (the giants zone).
pub const AU_MIN_L7: f64 = 5.0;

/// Outermost mapped orbit for L7 cells, in AU (the Oort edge).
pub const AU_MAX_L7: f64 = 100_000.0;

/// Cell radius of the innermost mapped orbit (Mercury stays off the star).
pub const ORBIT_R_MIN: f64 = 0.04;

/// Minimum on-screen gap between neighbouring orbit circles, in cell units.
pub const ORBIT_MIN_GAP: f64 = 0.02;

/// Decorrelation lane for orbit phase draws.
const ORBIT_STREAM_TAG: u64 = 0x08B1_75CE_11FE_0001;

/// Maps an orbit axis in AU onto a cell radius (#384).
///
/// Log-compressed from `au_min` to `au_max` onto [`ORBIT_R_MIN`] to the cell
/// edge, so Mercury and the heliopause read in one frame while order is
/// preserved. Axes outside the range clamp to the ends. Pure.
#[must_use]
pub fn orbit_radius_cell(au: f64, au_min: f64, au_max: f64) -> f64 {
    let clamped = au.clamp(au_min, au_max);
    let span = (au_max.log10() - au_min.log10()).max(1e-9);
    ORBIT_R_MIN + (clamped.log10() - au_min.log10()) / span * (HALF_BOUND - ORBIT_R_MIN)
}

/// Enforces the minimum on-screen gap outward in orbit order (#384).
///
/// `radii` arrive ascending (template order); each circle moves out just far
/// enough to clear its inner neighbour, so no two orbits overlap on screen.
/// Order never changes, so slots keep their data. Pure.
pub fn enforce_spacing(radii: &mut [f64]) {
    let mut inner = f64::NEG_INFINITY;
    for radius in radii.iter_mut() {
        *radius = (*radius).max(inner + ORBIT_MIN_GAP);
        inner = *radius;
    }
}

/// Orbit phase of one system: the angle of slot zero (#384).
///
/// One draw from a fixed lane, so every planet of the system shares the
/// phase while slots spread by the golden angle and never line up. Pure.
#[must_use]
pub fn orbit_phase(system_seed: u64) -> f64 {
    Rng::new(hash_triple(system_seed, ORBIT_STREAM_TAG, u64::MAX)).next_f64() * 2.0 * PI
}

/// Angle of `slot` on its orbit: phase plus the golden angle per slot.
#[must_use]
pub fn orbit_angle(phase: f64, slot: usize) -> f64 {
    const GOLDEN_ANGLE: f64 = 2.399_963_229_728_653;
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: slot count below 16, exactly representable"
    )]
    let turns = slot as f64 * GOLDEN_ANGLE;
    phase + turns
}

/// Indicator radius for a planet of `radius_earth` Earth radii (#384).
///
/// Linear in Earth radii, anchored so Earth reads 0.005 cell units and the
/// largest giants stay inside their orbits. Pure.
#[must_use]
pub fn planet_dot_radius(radius_earth: f64) -> f64 {
    (0.003 + 0.002 * radius_earth).clamp(0.003, 0.03)
}

/// Body radius for a star of `mass_solar` solar masses (#384).
///
/// Main-sequence mass-radius rule anchored at 0.35 cell units for one solar
/// mass (the Sun fills a third of its close-up cell), clamped so red dwarfs
/// stay visible and blue giants fit the cell. Pure.
#[must_use]
pub fn star_body_radius(mass_solar: f64) -> f64 {
    (0.35 * mass_solar.powf(0.8)).clamp(0.08, 0.45)
}

/// Kuiper band edges for an L7 cell, in cell units (30 to 50 AU).
#[must_use]
pub fn kuiper_band() -> (f64, f64) {
    (
        orbit_radius_cell(30.0, AU_MIN_L7, AU_MAX_L7),
        orbit_radius_cell(50.0, AU_MIN_L7, AU_MAX_L7),
    )
}

/// Scattered-disc band edges for an L7 cell, in cell units (50 to 1000 AU).
#[must_use]
pub fn scattered_band() -> (f64, f64) {
    (
        orbit_radius_cell(50.0, AU_MIN_L7, AU_MAX_L7),
        orbit_radius_cell(1_000.0, AU_MIN_L7, AU_MAX_L7),
    )
}

/// Asteroid band edges for an L8 cell, in cell units (2.0 to 3.5 AU).
#[must_use]
pub fn asteroid_band() -> (f64, f64) {
    (
        orbit_radius_cell(2.0, AU_MIN_L8, AU_MAX_L8),
        orbit_radius_cell(3.5, AU_MIN_L8, AU_MAX_L8),
    )
}

/// Kuiper band edges for an L8 cell, in cell units (30 to 50 AU).
#[must_use]
pub fn kuiper_band_l8() -> (f64, f64) {
    (
        orbit_radius_cell(30.0, AU_MIN_L8, AU_MAX_L8),
        orbit_radius_cell(50.0, AU_MIN_L8, AU_MAX_L8),
    )
}

/// Lays out one L7 outer-system cell: star, belts, shells (#384).
///
/// The star keeps the centre with its data (catalog Sun on the home path).
/// Populations split by lane coin into the Kuiper doughnut (flat ring, 30
/// to 50 AU), the scattered disc (tilted ring, 50 to 1000 AU), and the Oort
/// reservoir (spheres, 2000 to 100000 AU log-uniform): rings wear their
/// band edges, shells their own orbital radius. Counts, kinds, and the
/// portal prefix never move.
pub fn layout_l7(seed: u64, parent: &Constraints, points: &mut [Point], system_home: bool) {
    let _ = parent;
    if let Some(star) = points.first_mut() {
        let host = if system_home {
            StarInfo {
                kind: StarKind::G,
                mass_solar: 1.0,
            }
        } else {
            let mut stream = Rng::new(hash_triple(seed, crate::system::STAR_STREAM_TAG, 0));
            sample_star(&mut stream)
        };
        star.tint = Some(star_tint(host.kind));
        star.star = Some(host);
    }
    let (kuiper_lo, kuiper_hi) = kuiper_band();
    let (scattered_lo, scattered_hi) = scattered_band();
    for (index, point) in points.iter_mut().enumerate().skip(1) {
        let lane = index as u64;
        let mut stream = Rng::new(hash_triple(seed, OORT_STREAM_TAG, lane));
        let coin = stream.next_f64();
        if coin < 0.2 {
            // Kuiper doughnut: thin and flat in the ecliptic. Members keep
            // dot radii; the band reads from the shared ring edges.
            let au = 30.0 + stream.next_f64() * 20.0;
            let radius = orbit_radius_cell(au, AU_MIN_L7, AU_MAX_L7);
            let angle = stream.next_f64() * 2.0 * PI;
            let thin = (stream.next_f64() * 2.0 - 1.0) * 0.08 * radius;
            point.position = clamp_to_sphere([radius * angle.cos(), thin, radius * angle.sin()]);
            point.radius = 0.005;
            point.form = Form::Ring {
                normal: ECLIPTIC_NORMAL,
                inner: kuiper_lo,
                outer: kuiper_hi,
            };
        } else if coin < 0.5 {
            // Scattered disc: a thicker tilted ring past the Kuiper edge.
            let au = 50.0 + stream.next_f64() * 950.0;
            let radius = orbit_radius_cell(au, AU_MIN_L7, AU_MAX_L7);
            let angle = stream.next_f64() * 2.0 * PI;
            let tilt = (stream.next_f64() * 2.0 - 1.0) * 0.3 * radius;
            point.position = clamp_to_sphere([radius * angle.cos(), tilt, radius * angle.sin()]);
            point.radius = 0.005;
            point.form = Form::Ring {
                normal: ECLIPTIC_NORMAL,
                inner: scattered_lo,
                outer: scattered_hi,
            };
        } else {
            // Oort reservoir: log-uniform spheres out to the edge. Members
            // keep dot radii; each shell reads from its own orbit.
            let au = 2_000.0 * 50.0f64.powf(stream.next_f64());
            let radius = orbit_radius_cell(au, AU_MIN_L7, AU_MAX_L7);
            let z = stream.next_f64() * 2.0 - 1.0;
            let angle = stream.next_f64() * 2.0 * PI;
            let ring = (1.0 - z * z).max(0.0).sqrt();
            point.position = clamp_to_sphere([
                ring * angle.cos() * radius,
                ring * angle.sin() * radius,
                z * radius,
            ]);
            point.radius = 0.005;
            point.form = Form::Shell;
        }
    }
}

/// Lays out one L8 planetary system: star, ordered planets, belts, edge (#384).
///
/// The star keeps the centre with its data. Planets sit in the ecliptic on
/// log-compressed orbit circles with the spacing pass, sized by radius and
/// wearing their orbit; belt populations alternate between the asteroid and
/// Kuiper bands; one scenery population draws the heliopause shell at the
/// cell edge. The star portal stays index zero and planets keep their slots,
/// so portal order and the journey never move; populations trim from the
/// tail to the same total.
pub fn layout_l8(
    _seed: u64,
    system_seed: u64,
    _parent: &Constraints,
    points: &mut Vec<Point>,
    count: usize,
    system_home: bool,
) {
    let host = system_star(system_seed, system_home);
    if let Some(star) = points.first_mut() {
        star.position = [0.0, 0.0, 0.0];
        star.tint = Some(star_tint(host.kind));
        star.star = Some(host);
    }
    // Planet data first (slot order is orbit order), then positions.
    let mut axes = Vec::with_capacity(count);
    for (slot, point) in points.iter_mut().skip(1).take(count).enumerate() {
        let data = system_planet(system_seed, slot, count, host.mass_solar, system_home);
        axes.push(data.orbit_au);
        point.planet = Some(data);
        point.radius = planet_dot_radius(data.radius_earth);
    }
    let mut radii: Vec<f64> = axes
        .iter()
        .map(|au| orbit_radius_cell(*au, AU_MIN_L8, AU_MAX_L8))
        .collect();
    enforce_spacing(&mut radii);
    let phase = orbit_phase(system_seed);
    for (slot, point) in points.iter_mut().skip(1).take(count).enumerate() {
        let radius = radii.get(slot).copied().unwrap_or(ORBIT_R_MIN);
        let angle = orbit_angle(phase, slot);
        point.position = clamp_to_sphere([radius * angle.cos(), 0.0, radius * angle.sin()]);
        point.form = Form::Orbit {
            normal: ECLIPTIC_NORMAL,
        };
    }
    // Belt populations alternate bands; the unused band edges stay shared.
    let (asteroid_lo, asteroid_hi) = asteroid_band();
    let (kuiper_lo, kuiper_hi) = kuiper_band_l8();
    let mut belt = 0usize;
    for point in points.iter_mut().skip(1 + count) {
        let lane = belt as u64;
        let mut stream = Rng::new(hash_triple(system_seed, PLANET_STREAM_TAG, lane));
        belt += 1;
        if belt % 2 == 1 {
            let au = 2.0 + stream.next_f64() * 1.5;
            let radius = orbit_radius_cell(au, AU_MIN_L8, AU_MAX_L8);
            let angle = stream.next_f64() * 2.0 * PI;
            let jitter = (stream.next_f64() * 2.0 - 1.0) * 0.02 * radius;
            point.position = clamp_to_sphere([radius * angle.cos(), jitter, radius * angle.sin()]);
            point.radius = 0.005;
            point.form = Form::Ring {
                normal: ECLIPTIC_NORMAL,
                inner: asteroid_lo,
                outer: asteroid_hi,
            };
        } else {
            let au = 30.0 + stream.next_f64() * 20.0;
            let radius = orbit_radius_cell(au, AU_MIN_L8, AU_MAX_L8);
            let angle = stream.next_f64() * 2.0 * PI;
            let jitter = (stream.next_f64() * 2.0 - 1.0) * 0.05 * radius;
            point.position = clamp_to_sphere([radius * angle.cos(), jitter, radius * angle.sin()]);
            point.radius = 0.005;
            point.form = Form::Ring {
                normal: ECLIPTIC_NORMAL,
                inner: kuiper_lo,
                outer: kuiper_hi,
            };
        }
    }
    // The heliopause shell at the cell edge: a member sitting on the edge
    // draws it about the centre. One population trims from the tail to keep
    // the total, so the star portal and the planet slots never move.
    // Pathological budgets with no population to trim skip the shell; the
    // journey never needs it.
    let total = points.len();
    if total > 1 + count {
        points.truncate(total - 1);
        points.push(Point {
            position: [HALF_BOUND, 0.0, 0.0],
            radius: 0.005,
            kind: MarkerKind::Population,
            form: Form::Shell,
            ..Point::bare([0.0; 3], 0.005, MarkerKind::Population)
        });
    }
}

/// Lays out one L9 star close-up: planets in the L8 plane plus the star (#384).
///
/// `system_seed` is the shared system seed, which is the cell seed of real
/// L9 cells by path-seed construction. Planets recompute the L8 layout from
/// it, so the close-up and the system view agree slot for slot. The rare
/// companion appends as a population dot when `has_companion` holds; the
/// star body appends last as a population at the centre sized by mass in
/// its class tint. Planet indices never move, so the journey never moves.
pub fn layout_l9(
    system_seed: u64,
    parent: &Constraints,
    points: &mut Vec<Point>,
    planets: usize,
    has_companion: bool,
    system_home: bool,
) {
    let host = system_star(system_seed, system_home);
    let mut axes = Vec::with_capacity(planets);
    for (slot, point) in points.iter_mut().take(planets).enumerate() {
        let data = system_planet(system_seed, slot, planets, host.mass_solar, system_home);
        axes.push(data.orbit_au);
        point.planet = Some(data);
        point.radius = planet_dot_radius(data.radius_earth);
    }
    let mut radii: Vec<f64> = axes
        .iter()
        .map(|au| orbit_radius_cell(*au, AU_MIN_L8, AU_MAX_L8))
        .collect();
    enforce_spacing(&mut radii);
    let phase = orbit_phase(system_seed);
    for (slot, point) in points.iter_mut().take(planets).enumerate() {
        let radius = radii.get(slot).copied().unwrap_or(ORBIT_R_MIN);
        let angle = orbit_angle(phase, slot);
        point.position = clamp_to_sphere([radius * angle.cos(), 0.0, radius * angle.sin()]);
        point.form = Form::Orbit {
            normal: ECLIPTIC_NORMAL,
        };
    }
    if has_companion && !points.is_empty() {
        let mut stream = Rng::new(hash_triple(system_seed, PLANET_STREAM_TAG, u64::MAX));
        let mut position = [0.0; 3];
        for (slot, extent) in position.iter_mut().zip(parent.allowed_extent.iter()) {
            *slot = (stream.next_f64() * 2.0 - 1.0) * extent;
        }
        position = clamp_to_sphere(position);
        let floor = points
            .iter()
            .map(|point| point.radius)
            .fold(f64::INFINITY, f64::min);
        let info = sample_star(&mut stream);
        points.push(Point {
            position,
            radius: (floor * 0.5).max(1e-4),
            kind: MarkerKind::Population,
            star: Some(info),
            tint: Some(star_tint(info.kind)),
            ..Point::bare([0.0; 3], 0.01, MarkerKind::Population)
        });
    }
    points.push(Point {
        position: [0.0, 0.0, 0.0],
        radius: star_body_radius(host.mass_solar),
        kind: MarkerKind::Population,
        star: Some(host),
        tint: Some(star_tint(host.kind)),
        form: Form::Body,
        ..Point::bare([0.0; 3], 0.1, MarkerKind::Population)
    });
}

/// Coplanarity, order, and spacing of the L8 planets on the journey (#384).
///
/// Regenerates the journey L8 cell and checks its planet points (carrying
/// `PlanetInfo`): every orbit lies in the ecliptic, radii ascend with orbit
/// axis, and every neighbouring pair clears [`ORBIT_MIN_GAP`]. Pure; the
/// `ORBITS` verify line prints it in T7.
#[must_use]
pub fn orbits_check_l8(root: u64) -> (bool, bool, bool) {
    use crate::nest::autopilot_path;
    let chain = autopilot_path(root);
    let Some(home) = chain.get(..7) else {
        return (false, false, false);
    };
    let cell = crate::nest::generate_cell(root, home);
    let planets: Vec<&Point> = cell
        .points
        .iter()
        .filter(|point| point.planet.is_some())
        .collect();
    if planets.is_empty() {
        return (false, false, false);
    }
    let coplanar = planets.iter().all(|point| {
        let [_, y, _] = point.position;
        y.abs() < 1e-9
            && point.form
                == (Form::Orbit {
                    normal: ECLIPTIC_NORMAL,
                })
    });
    let mut radii: Vec<(f64, f64)> = planets
        .iter()
        .map(|point| {
            let [x, _, z] = point.position;
            (
                (x * x + z * z).sqrt(),
                point.planet.map_or(f64::NAN, |data| data.orbit_au),
            )
        })
        .collect();
    radii.sort_by(|a, b| a.0.total_cmp(&b.0));
    let ordered = radii.windows(2).all(|pair| {
        let [(_, inner), (_, outer)] = pair else {
            return false;
        };
        inner <= outer
    });
    let spaced = radii.windows(2).all(|pair| {
        let [(inner, _), (outer, _)] = pair else {
            return false;
        };
        outer - inner >= ORBIT_MIN_GAP - 1e-9
    });
    (coplanar, ordered, spaced)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r#gen::{Constraints, Generator};
    use crate::sysgen::GalaxyGenerator;

    /// Builds typical mid-ladder constraints for tests.
    fn parent_constraints() -> Constraints {
        Constraints::new(1.0, 2, 64, [0.5, 0.5, 0.5]).expect("valid test constraints")
    }

    #[test]
    fn orbit_radii_compress_log_order_and_spacing() {
        assert_eq!(orbit_radius_cell(0.3, AU_MIN_L8, AU_MAX_L8), ORBIT_R_MIN);
        assert_eq!(orbit_radius_cell(120.0, AU_MIN_L8, AU_MAX_L8), HALF_BOUND);
        assert_eq!(orbit_radius_cell(0.01, AU_MIN_L8, AU_MAX_L8), ORBIT_R_MIN);
        assert_eq!(orbit_radius_cell(999.0, AU_MIN_L8, AU_MAX_L8), HALF_BOUND);
        let mercury = orbit_radius_cell(0.39, AU_MIN_L8, AU_MAX_L8);
        let earth = orbit_radius_cell(1.0, AU_MIN_L8, AU_MAX_L8);
        let neptune = orbit_radius_cell(30.0, AU_MIN_L8, AU_MAX_L8);
        assert!(
            mercury < earth && earth < neptune,
            "orbits keep their order"
        );
        assert!(mercury > ORBIT_R_MIN, "Mercury clears the star");
        let mut radii = [neptune, mercury, earth];
        radii.sort_by(|a, b| a.total_cmp(b));
        enforce_spacing(&mut radii);
        assert!(radii[1] - radii[0] >= ORBIT_MIN_GAP - 1e-12);
        assert!(radii[2] - radii[1] >= ORBIT_MIN_GAP - 1e-12);
    }

    #[test]
    fn star_bodies_span_dwarfs_to_giants() {
        assert_eq!(star_body_radius(1.0), 0.35);
        assert!(star_body_radius(0.1) < star_body_radius(1.0));
        assert!(star_body_radius(10.0) > star_body_radius(1.0));
        assert_eq!(star_body_radius(0.0001), 0.08);
        assert_eq!(star_body_radius(10_000.0), 0.45);
    }

    #[test]
    fn l7_bands_read_rings_and_shells() {
        let parent = parent_constraints();
        let level = crate::coords::Level::new(7).expect("L7");
        let out = GalaxyGenerator::new(level, false, false, false).generate(4242, &parent);
        let head = out.points.first().expect("L7 keeps its star");
        assert_eq!(head.position, [0.0, 0.0, 0.0]);
        assert!(head.star.is_some(), "the central star keeps its data");
        let (kuiper_lo, kuiper_hi) = kuiper_band();
        let mut rings = 0u32;
        let mut shells = 0u32;
        for point in out.points.iter().skip(1) {
            match point.form {
                Form::Ring { inner, outer, .. } => {
                    rings += 1;
                    let band = (inner == kuiper_lo && outer == kuiper_hi) || {
                        let (lo, hi) = scattered_band();
                        inner == lo && outer == hi
                    };
                    assert!(band, "belt rings wear their band edges");
                }
                Form::Shell => {
                    shells += 1;
                    assert!(
                        point.radius <= HALF_BOUND,
                        "shells fit the cell: {}",
                        point.radius
                    );
                }
                other => panic!("L7 populations read rings or shells: {other:?}"),
            }
        }
        assert!(rings > 0 && shells > 0, "both belts and shells must show");
    }

    #[test]
    fn l8_system_reads_one_plane_with_spaced_ordered_orbits() {
        let parent = parent_constraints();
        let level = crate::coords::Level::new(8).expect("L8");
        let out = GalaxyGenerator::new(level, false, false, false).generate(4242, &parent);
        assert_eq!(out.points.len(), 32, "the heliopause shell trims one dot");
        let star = out.points.first().expect("L8 keeps its star");
        assert_eq!(star.position, [0.0, 0.0, 0.0]);
        let shell = out.points.last().expect("heliopause shell closes the cell");
        assert_eq!(shell.form, Form::Shell);
        assert_eq!(shell.position, [HALF_BOUND, 0.0, 0.0]);
        assert_eq!(shell.radius, 0.005, "shell members keep dot radii");
        let (coplanar, ordered, spaced) = orbits_check_l8(42);
        assert!(
            coplanar && ordered && spaced,
            "the journey system must read"
        );
    }

    #[test]
    fn l9_close_up_matches_l8_slots_with_a_star_body() {
        let parent = parent_constraints();
        let l8 = crate::coords::Level::new(8).expect("L8");
        let l9 = crate::coords::Level::new(9).expect("L9");
        let seed = 4242u64;
        let system_seed = crate::seed::hash_cell(seed, 9, 0, 0, 0);
        let eight = GalaxyGenerator::new(l8, false, false, false).generate(seed, &parent);
        let nine = GalaxyGenerator::new(l9, false, false, false).generate(system_seed, &parent);
        let eight_planets: Vec<[f64; 3]> = eight
            .points
            .iter()
            .filter_map(|point| point.planet.map(|_| point.position))
            .collect();
        let nine_planets: Vec<[f64; 3]> = nine
            .points
            .iter()
            .filter_map(|point| point.planet.map(|_| point.position))
            .collect();
        assert_eq!(
            eight_planets, nine_planets,
            "the close-up shares the system plane slot for slot"
        );
        let star = nine
            .points
            .iter()
            .find(|point| point.star.is_some() && point.planet.is_none())
            .expect("L9 draws its star");
        assert_eq!(star.position, [0.0, 0.0, 0.0]);
        assert_eq!(star.form, Form::Body);
        assert_eq!(star.kind, MarkerKind::Population, "the star never opens");
    }
}
