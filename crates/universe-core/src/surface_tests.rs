//! Surface classifier tests (#1161 T2, Spec v2 criterion 2).

use super::*;

/// A world of `radius_earth` at `orbit_au` around a star of `star`, with air
/// when `air`, under `seed`.
fn world(seed: u64, radius_earth: f64, orbit_au: f64, star: f64, air: bool) -> WorldInputs {
    WorldInputs {
        seed,
        radius_earth,
        orbit_au,
        star_mass_solar: star,
        has_air: air,
        home: false,
    }
}

/// A seed whose iron and water draws satisfy `pick`.
fn seed_where(pick: impl Fn(f64, f64) -> bool) -> u64 {
    (0u64..100_000)
        .find(|&s| pick(iron_fraction(s), water_fraction(s)))
        .expect("a seed with the wanted draws")
}

#[test]
fn the_temperature_follows_the_guide_formula() {
    // Earth: L = 1, a = 1 AU, A = 0.3 gives about 255 K (planet-types.md).
    let earth = equilibrium_k(1.0, 1.0, 0.3);
    assert!((earth - 254.6).abs() < 1.0, "{earth}");
    // Four times closer is twice as warm.
    let close = equilibrium_k(1.0, 0.25, 0.3);
    assert!((close / earth - 2.0).abs() < 1e-9);
    assert!((ice_line_au(1.0) - 2.7).abs() < 1e-12);
    assert!((ice_line_au(4.0) - 5.4).abs() < 1e-12);
    assert!((star_luminosity_solar(1.0) - 1.0).abs() < 1e-12);
    assert!(star_luminosity_solar(0.3) < 0.05, "red dwarfs are dim");
    assert!(star_luminosity_solar(3.0) > 50.0, "B stars are bright");
}

#[test]
fn each_guide_row_gives_its_type() {
    // Criterion 2: the guide's Step 4 table, first match wins.
    let plain = seed_where(|iron, water| iron < IRON_FROM && water > 0.3 && water < ICY_WATER_FROM);
    let iron = seed_where(|iron, _| iron > IRON_FROM);
    let dry = seed_where(|iron, water| iron < IRON_FROM && water < DRY_WATER_UNDER);
    let icy = seed_where(|iron, water| iron < IRON_FROM && water > ICY_WATER_FROM);
    // Hot: a bright star at the innermost orbit is over 1,200 K.
    assert_eq!(
        classify(&world(plain, 1.0, 0.39, 4.0, true)),
        Some(WorldType::Lava)
    );
    // Iron draw over 0.6.
    assert_eq!(
        classify(&world(iron, 1.0, 1.0, 1.0, true)),
        Some(WorldType::Iron)
    );
    // Temperate with water and air.
    assert_eq!(
        classify(&world(plain, 1.0, 1.0, 1.0, true)),
        Some(WorldType::EarthLike)
    );
    // Temperate but dry, or without air, or hot: desert.
    assert_eq!(
        classify(&world(dry, 1.0, 1.0, 1.0, true)),
        Some(WorldType::Desert)
    );
    assert_eq!(
        classify(&world(plain, 1.0, 1.0, 1.0, false)),
        Some(WorldType::Desert)
    );
    assert_eq!(
        classify(&world(plain, 1.0, 0.39, 1.0, true)),
        Some(WorldType::Desert)
    );
    // Cold, or formed icy beyond the ice line.
    assert_eq!(
        classify(&world(plain, 1.0, 9.0, 1.0, true)),
        Some(WorldType::Ice)
    );
    assert_eq!(
        classify(&world(icy, 1.0, 3.0, 1.0, true)),
        Some(WorldType::Ice)
    );
    // Dwarf-sized: barren rock, ice when cold.
    assert_eq!(
        classify(&world(plain, 0.2, 1.0, 1.0, false)),
        Some(WorldType::Barren)
    );
    assert_eq!(
        classify(&world(plain, 0.2, 9.0, 1.0, false)),
        Some(WorldType::Ice)
    );
    // Neptunian worlds are never iron; beyond the ice line they are ice.
    assert_eq!(
        classify(&world(iron, 4.0, 19.2, 1.0, true)),
        Some(WorldType::Ice)
    );
    assert_ne!(
        classify(&world(iron, 4.0, 1.0, 1.0, true)),
        Some(WorldType::Iron)
    );
    // Super-Earths use the type they match; giants stay a smooth ball.
    assert_eq!(
        classify(&world(plain, 1.4, 1.0, 1.0, true)),
        Some(WorldType::EarthLike)
    );
    assert_eq!(classify(&world(plain, 11.0, 1.0, 1.0, true)), None);
}

#[test]
fn home_is_earth_like_and_its_moon_is_barren() {
    // Criterion 3 (Q3 A): the home planet stays Earth-like.
    let root = crate::nav::DEMO_SEED;
    let journey = crate::nest::autopilot_path(root);
    let home = journey.get(..9).expect("home chain");
    let inputs = planet_inputs(root, home).expect("home inputs");
    assert!(inputs.home);
    assert_eq!(planet_type(root, home), Some(WorldType::EarthLike));
    assert_eq!(classify_moon(&inputs, 1, 0.27), WorldType::Barren);
}

#[test]
fn the_same_seed_gives_the_same_type() {
    // Criterion 2: deterministic, and every type has a palette and a name.
    let root = crate::nav::DEMO_SEED;
    let journey = crate::nest::autopilot_path(root);
    let start = journey.get(..4).expect("start chain");
    let first = list_worlds(root, start, 60);
    let again = list_worlds(root, start, 60);
    assert_eq!(first, again);
    assert_eq!(first.len(), 60);
    for kind in WorldType::ALL {
        assert_eq!(WorldType::from_name(kind.name()), Some(kind));
        let p = kind.palette();
        for c in [p.low, p.high, p.steep, p.cap, p.glow].iter().flatten() {
            assert!((0.0..=1.0).contains(c));
        }
    }
    let glowing: Vec<_> = WorldType::ALL
        .into_iter()
        .filter(|k| k.palette().glow.iter().any(|&c| c > 0.0))
        .collect();
    assert_eq!(glowing, vec![WorldType::Lava], "only lava has own light");
}

#[test]
fn the_home_system_holds_several_types() {
    // T1 fixtures: the home system alone gives three solid types and a giant.
    let root = crate::nav::DEMO_SEED;
    let journey = crate::nest::autopilot_path(root);
    let system = journey.get(..8).expect("home system");
    let kinds: Vec<_> = list_worlds(root, system, 32)
        .iter()
        .map(|w| w.kind)
        .collect();
    for want in [
        Some(WorldType::EarthLike),
        Some(WorldType::Desert),
        Some(WorldType::Ice),
    ] {
        assert!(kinds.contains(&want), "{want:?} in {kinds:?}");
    }
}

#[test]
#[ignore = "slow (~7 s on the dev PC): runs with --include-ignored (CI, validator), #847"]
fn every_type_appears_near_home() {
    // T2 histogram: 4,000 worlds under the home L3 cell, depth first. Lava
    // is rare (bright stars only), but present; most worlds are icy because
    // most stars are dim red dwarfs.
    let root = crate::nav::DEMO_SEED;
    let journey = crate::nest::autopilot_path(root);
    let start = journey.get(..3).expect("start chain");
    let worlds = list_worlds(root, start, 4_000);
    let count = |k: Option<WorldType>| worlds.iter().filter(|w| w.kind == k).count();
    for kind in [
        WorldType::EarthLike,
        WorldType::Desert,
        WorldType::Iron,
        WorldType::Ice,
        WorldType::Lava,
    ] {
        assert!(count(Some(kind)) > 0, "{kind:?} missing");
    }
    eprintln!(
        "histogram over {}: earth-like {}, desert {}, iron {}, ice {}, lava {}, barren {}, giant {}",
        worlds.len(),
        count(Some(WorldType::EarthLike)),
        count(Some(WorldType::Desert)),
        count(Some(WorldType::Iron)),
        count(Some(WorldType::Ice)),
        count(Some(WorldType::Lava)),
        count(Some(WorldType::Barren)),
        count(None),
    );
}
