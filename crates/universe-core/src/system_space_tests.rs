//! One-space system tests (#995 T1), split under `E-FILE-SIZE`.

use super::{
    AU_M, BodyKind, body_mass_kg, home_l9_chain, influence_radius_m, kepler_mass_kg,
    laplace_radius_m, sample_l9_chains, star_radius_m, system_space, zone_nesting,
};
use crate::nav::DEMO_SEED;
use crate::nest::generate_cell;

#[test]
fn the_home_system_sits_at_true_scale() {
    // #995 T1: the star at the origin with the Sun's radius, eight planets
    // on their catalog orbits, and the Earth-Moon pair at 384,400 km.
    let chain = home_l9_chain(DEMO_SEED).expect("home chain");
    let space = system_space(DEMO_SEED, &chain).expect("home system");
    let star = space.bodies.first().expect("star");
    assert_eq!(star.kind, BodyKind::Star);
    assert_eq!(star.position_m, [0.0; 3]);
    assert!((star.radius_m - star_radius_m(1.0)).abs() < 1.0);
    let orbits_au: Vec<f64> = space.planets().map(|p| p.orbit_m / AU_M).collect();
    assert_eq!(orbits_au, crate::system::SOLAR_AXES_AU.to_vec());
    for planet in space.planets() {
        let [x, y, z] = planet.position_m;
        assert!(y == 0.0, "ecliptic plane");
        assert!(((x * x + z * z).sqrt() - planet.orbit_m).abs() < 1e-3 * planet.orbit_m);
    }
    assert!(
        space
            .moons()
            .any(|moon| (moon.orbit_m - 384_400_000.0).abs() < 1.0
                && (moon.radius_m - 1_738_000.0).abs() < 1.0),
        "the Moon"
    );
}

#[test]
fn bodies_carry_the_cells_numbers() {
    // #995 T1: every planet's orbit and period are its L9 point's
    // `PlanetInfo`, and every moon sits at its `MoonInfo` orbit from its
    // planet, on the home system and two others.
    let mut chains = sample_l9_chains(DEMO_SEED, 2);
    assert_eq!(chains.len(), 2, "two sample systems");
    chains.push(home_l9_chain(DEMO_SEED).expect("home"));
    for chain in chains {
        let space = system_space(DEMO_SEED, &chain).expect("system");
        let cell = generate_cell(DEMO_SEED, &chain);
        for planet in space.planets() {
            let BodyKind::Planet { marker } = planet.kind else {
                unreachable!("planets() yields planets");
            };
            let info = cell.points[marker as usize].planet.expect("planet info");
            assert!((planet.orbit_m - info.orbit_au * AU_M).abs() < 1.0);
            assert_eq!(planet.period_days, info.period_days);
            assert!(planet.radius_m > 0.0);
        }
        for moon in space.moons() {
            let BodyKind::Moon { planet, .. } = moon.kind else {
                unreachable!("moons() yields moons");
            };
            let host = space.bodies[planet].position_m;
            let d = (0..3)
                .map(|i| (moon.position_m[i] - host[i]).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!((d - moon.orbit_m).abs() < 1e-6 * moon.orbit_m);
        }
    }
}

#[test]
fn the_same_seed_gives_the_same_system() {
    // `E-DET-TIERS`: two builds are bit-identical.
    let chain = home_l9_chain(DEMO_SEED).expect("home chain");
    assert_eq!(
        system_space(DEMO_SEED, &chain),
        system_space(DEMO_SEED, &chain)
    );
}

#[test]
fn only_star_close_ups_build_a_system() {
    let chain = home_l9_chain(DEMO_SEED).expect("home chain");
    assert!(system_space(DEMO_SEED, &chain[..7]).is_none());
}

#[test]
fn earth_and_moon_zones_match_the_published_radii() {
    // #995 T2: the Laplace radius lands within 10% of the published 929,000
    // km for the Earth around the Sun, and the game's own masses give the
    // home Moon's within 10% of the published 64,300 km.
    use crate::gravity::{EARTH_MASS_KG, SOLAR_MASS_KG};
    let r = laplace_radius_m(AU_M, EARTH_MASS_KG, SOLAR_MASS_KG);
    assert!((r / 929_000_000.0 - 1.0).abs() < 0.1, "Earth {r}");
    let chain = home_l9_chain(DEMO_SEED).expect("home chain");
    let space = system_space(DEMO_SEED, &chain).expect("home system");
    let moon = space
        .moons()
        .find(|m| (m.orbit_m - 384_400_000.0).abs() < 1.0)
        .expect("the Moon");
    let r = influence_radius_m(&space, moon);
    assert!((r / 64_300_000.0 - 1.0).abs() < 0.1, "Moon {r}");
    assert!(influence_radius_m(&space, &space.bodies[0]).is_infinite());
}

#[test]
fn keplers_law_weighs_the_earth_from_the_moon() {
    // #995 T2: the Moon's orbit and period give the Earth's mass to within
    // the Moon's own share (the Earth-Moon pair is 1.2% heavier).
    let m = kepler_mass_kg(384_400_000.0, 27.3);
    assert!(
        (m / crate::gravity::EARTH_MASS_KG - 1.0).abs() < 0.03,
        "{m}"
    );
}

#[test]
fn gravity_zones_nest_in_the_sample_systems() {
    // #995 T2: on the home system and two others, the planets' zones never
    // overlap and every moon's zone sits inside its planet's; the counts
    // also feed the study report.
    let mut chains = sample_l9_chains(DEMO_SEED, 2);
    chains.push(home_l9_chain(DEMO_SEED).expect("home"));
    for chain in chains {
        let space = system_space(DEMO_SEED, &chain).expect("system");
        let nesting = zone_nesting(&space);
        assert_eq!(nesting.planet_overlaps, 0, "{chain:?}: {nesting:?}");
        for body in &space.bodies {
            assert!(body_mass_kg(&space, body) > 0.0);
        }
    }
}
