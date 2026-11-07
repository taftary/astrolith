//! Pure recipe/generation checks; not proof of real moon flight or contact.

use super::*;
use crate::moon_catalog::{MoonIdentity, MoonOrigin, PlanetIdentity};
use crate::nav::{MarkerIndex, Seed};
use crate::world_type::MoonPaint;

fn host(chain: &[u32]) -> PlanetIdentity {
    PlanetIdentity::new(Seed(42), chain).unwrap()
}

fn moon(chain: &[u32]) -> SurfaceIdentity {
    SurfaceIdentity::Moon(MoonIdentity::new(host(chain), MarkerIndex(1)).unwrap())
}

#[test]
fn owned_recipes_preserve_actual_generated_types_physics_and_provenance() {
    let fixtures = [
        (
            [7, 2, 7, 1, 2, 0, 0, 0, 5],
            WorldType::Barren,
            MoonOrigin::CatalogBased,
        ),
        (
            [7, 2, 7, 3, 2894, 0, 0, 0, 1],
            WorldType::Ice,
            MoonOrigin::Procedural,
        ),
        (
            [7, 2, 7, 5, 4149, 1, 0, 0, 0],
            WorldType::Lava,
            MoonOrigin::Procedural,
        ),
        (
            [7, 2, 7, 1, 2, 0, 0, 0, 1],
            WorldType::Barren,
            MoonOrigin::Procedural,
        ),
    ];
    let mut keys = Vec::new();
    for (chain, kind, origin) in fixtures {
        let before = crate::nest::generate_cell(42, &chain);
        let identity = moon(&chain);
        let recipe = SurfaceRecipe::for_owner(identity).unwrap();
        let record = *recipe.moon_record().unwrap();
        assert_eq!(record.kind, kind);
        assert_eq!(record.origin, origin);
        assert_eq!(recipe.kind(), kind);
        assert_eq!(recipe.key().owner(), identity);
        assert_eq!(recipe, SurfaceRecipe::for_owner(identity).unwrap());
        assert!(!keys.contains(&recipe.key()));
        keys.push(recipe.key());
        let relief = recipe.relief();
        assert_eq!(relief.radius_m(), record.info.radius_km * 1_000.0);
        assert_eq!(relief.seed(), record.paint_seed);
        assert!(relief.is_cratered());
        assert!(relief.water().is_none());
        let shape = relief.moon_surface().unwrap();
        let mass = crate::gravity::moon_mass_kg(relief.radius_m());
        assert_eq!(
            shape.gravity_m_s2(),
            crate::gravity::G * mass / relief.radius_m().powi(2)
        );
        let paint = MoonPaint::new(kind, record.paint_seed);
        let (lo, hi) = relief.height_bounds_m();
        let mut sampler = crate::relief::HeightSampler::new(relief);
        for dir in [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
        ] {
            assert_eq!(shape.paint().tone(dir), paint.tone(dir));
            let height = relief.height_m(dir);
            assert!((lo..=hi).contains(&height));
            assert_eq!(height, sampler.height_m(dir));
            assert_eq!(height, relief.terrain_m(dir));
            assert_eq!(height, relief.ground(dir).surface_m);
        }
        let after = crate::nest::generate_cell(42, &chain);
        assert_eq!(
            before, after,
            "full generated data, not only selected properties"
        );
        if origin == MoonOrigin::CatalogBased {
            assert_eq!(record.info.radius_km, 1_738.0);
            assert_eq!(record.terrain_origin(), "synthetic");
            assert_eq!(record.property_origin(), "supplied-reference-not-measured");
        } else {
            assert_eq!(record.terrain_origin(), "procedural");
            assert!((200.0..1_700.0).contains(&record.info.radius_km));
        }
    }
}

#[test]
fn planet_recipe_keeps_existing_relief_bit_for_bit() {
    for chain in [[7, 2, 7, 1, 2, 0, 0, 0, 5], [7, 2, 7, 1, 2, 0, 0, 0, 1]] {
        let recipe = SurfaceRecipe::for_owner(SurfaceIdentity::Planet(host(&chain))).unwrap();
        assert_eq!(recipe.relief(), &Relief::for_planet(42, &chain));
        assert!(recipe.moon_record().is_none());
        assert!(recipe.relief().moon_surface().is_none());
        assert_ne!(
            recipe.key(),
            SurfaceRecipe::for_owner(moon(&chain)).unwrap().key()
        );
    }
}

#[test]
fn exact_domain_boundaries_use_the_same_recipe_without_fabricated_identities() {
    assert_eq!(crate::moon_catalog::SAMPLED_RADIUS_MIN_KM, 200.0);
    assert_eq!(crate::moon_catalog::SAMPLED_RADIUS_END_KM, 1_700.0);
    // Boundary-only shapes deliberately have no SurfaceIdentity/ListedMoon.
    for radius in [
        200_000.0,
        f64::from_bits(1_700_000.0f64.to_bits() - 1),
        1_738_000.0,
    ] {
        for kind in [
            WorldType::Barren,
            WorldType::Ice,
            WorldType::Lava,
            WorldType::Iron,
        ] {
            let r = SurfaceRadiusMetres::new(radius).unwrap();
            let relief = Relief::for_moon(73, kind, r).unwrap();
            let shape = relief.moon_surface().unwrap();
            assert_eq!(relief.radius_m(), radius);
            assert!(shape.gravity_m_s2() > 0.0 && shape.gravity_m_s2() < 2.0);
            assert_ne!(shape.gravity_m_s2(), 9.81);
            assert!(relief.is_cratered());
            // Even an explicit water request cannot attach a planet sea to ice.
            assert!(
                relief
                    .clone()
                    .with_water(crate::water::Water::for_type(WorldType::Ice))
                    .water()
                    .is_none()
            );
            for i in 0..128u32 {
                let z = 2.0 * f64::from(i) / 127.0 - 1.0;
                let angle = f64::from(i) * 2.399_963;
                let radial = (1.0 - z * z).max(0.0).sqrt();
                let dir = [radial * angle.cos(), z, radial * angle.sin()];
                let (lo, hi) = relief.height_bounds_m();
                assert!((lo..=hi).contains(&relief.height_m(dir)));
                assert!(radius + relief.height_m(dir) > 0.0);
            }
            for dir in [[0.0; 3], [f64::NAN; 3], [f64::INFINITY; 3]] {
                assert_eq!(relief.height_m(dir), 0.0);
            }
        }
    }
}

#[test]
fn painted_crater_centres_are_actual_bowls_not_an_unrelated_fine_world() {
    let relief = Relief::for_moon(
        73,
        WorldType::Barren,
        SurfaceRadiusMetres::new(200_000.0).unwrap(),
    )
    .unwrap();
    let paint = relief.moon_surface().unwrap().paint();
    let mut bowls = 0;
    for centre in paint.shape_centres() {
        if relief.crater_height_m(centre).unwrap() < 0.0 {
            bowls += 1;
        }
        assert_eq!(
            paint.tone(centre),
            MoonPaint::new(WorldType::Barren, 73).tone(centre)
        );
    }
    assert!(bowls > 0);
}

#[test]
fn moon_heights_agree_across_cube_faces_and_levels_and_are_continuous() {
    use crate::cube_sphere::{Face, grid_point};
    let relief = Relief::for_moon(
        11,
        WorldType::Ice,
        SurfaceRadiusMetres::new(200_000.0).unwrap(),
    )
    .unwrap();
    for i in 0..=32 {
        let a = grid_point(Face::PosX, 32, i, 32).unwrap();
        let b = grid_point(Face::PosY, 32, i, 32).unwrap();
        assert_eq!(a, b);
        assert_eq!(relief.height_m(a), relief.height_m(b));
        let finer = grid_point(Face::PosX, 64, 2 * i, 64).unwrap();
        assert_eq!(a, finer);
        assert_eq!(relief.height_m(a), relief.height_m(finer));
    }
    for dir in [
        [1.0, 1.0, 0.0],
        [1.0, 1.0, 1.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
    ] {
        let mut near = dir;
        near[0] += 1e-10;
        assert!((relief.height_m(dir) - relief.height_m(near)).abs() < 0.01);
    }
}

#[test]
fn existing_patch_builder_and_tile_path_accept_the_exact_moon_recipe() {
    let relief = Relief::for_moon(
        11,
        WorldType::Ice,
        SurfaceRadiusMetres::new(200_000.0).unwrap(),
    )
    .unwrap();
    let key = crate::lod::PatchKey::new(crate::cube_sphere::Face::PosY, 0, 0, 0).unwrap();
    let patch = crate::lod_mesh::build_patch(key, relief.radius_m(), &relief).unwrap();
    assert!(patch.full);
    assert_eq!(patch.vertex_count(), 1_089);
    assert!(
        patch.water.is_empty(),
        "solid ice is not a planet water layer"
    );
    let (lo, hi) = relief.height_bounds_m();
    assert!(patch.min_h_m >= lo && patch.max_h_m <= hi);
    assert!(patch.offsets.iter().flatten().all(|v| v.is_finite()));
    let (tiled_patch, tile) =
        crate::grid_tile::build_patch_and_tile(key, relief.radius_m(), &relief, true).unwrap();
    assert_eq!(patch, tiled_patch);
    assert_eq!(
        tile,
        crate::grid_tile::grid_tile(&patch, relief.radius_m(), &relief)
    );
}

#[test]
fn every_catalogued_moon_of_six_systems_uses_one_owned_recipe_path() {
    let mut count = 0;
    for chain in [
        [7, 2, 7, 1, 2, 0, 0, 0, 2], // 15-moon host
        [0, 0, 0, 0, 61, 0, 0, 0, 2],
        [0, 0, 0, 0, 206, 0, 0, 0, 1],
        [0, 0, 0, 0, 413, 0, 0, 0, 2],
        [0, 0, 0, 0, 606, 0, 0, 0, 0],
        [0, 0, 0, 0, 1175, 0, 0, 0, 0],
    ] {
        let host = host(&chain);
        let before = crate::nest::generate_cell(42, &chain);
        let records = crate::moon_catalog::moons_for_planet(host).unwrap();
        if chain[0] == 7 {
            assert_eq!(records.len(), 15);
        }
        let mut keys = Vec::new();
        for record in records {
            let recipe = SurfaceRecipe::for_owner(SurfaceIdentity::Moon(record.identity)).unwrap();
            assert!(!keys.contains(&recipe.key()));
            keys.push(recipe.key());
            assert_eq!(recipe.moon_record(), Some(&record));
            assert_eq!(recipe.relief().radius_m(), record.info.radius_km * 1_000.0);
            assert!(recipe.relief().height_m([1.0, 0.0, 0.0]).is_finite());
            count += 1;
        }
        assert_eq!(before, crate::nest::generate_cell(42, &chain));
    }
    assert!(
        count >= 20,
        "finite coverage is not a universal-minimum claim"
    );
}
