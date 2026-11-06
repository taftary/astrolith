//! Lake tests (#1162 T3, Spec v2 criteria 6 and 7).

use super::*;
use crate::relief::Relief;
use crate::water::WaterCode;
use crate::water_places::fibonacci_dir;
use std::collections::BTreeMap;

const HOME: [u32; 9] = [7, 2, 7, 1, 2, 0, 0, 0, 5];
const ICE: [u32; 9] = [7, 2, 7, 1, 303, 1, 0, 0, 1];
const DRY: [[u32; 9]; 4] = [
    [7, 2, 7, 1, 2, 0, 0, 0, 0],
    [7, 2, 7, 2, 3781, 1, 0, 0, 0],
    [7, 2, 7, 9, 1294, 1, 0, 0, 1],
    [7, 2, 7, 5, 4149, 1, 0, 0, 0],
];

fn relief(chain: &[u32; 9]) -> Relief {
    Relief::for_planet(crate::nav::DEMO_SEED, chain)
}

/// Every lake of `r`, by key (`_n` kept for the call sites' reading).
fn lakes_of(r: &Relief, _n: usize) -> BTreeMap<u64, Lake> {
    r.lakes().into_iter().map(|l| (l.seed.key, l)).collect()
}

/// Points across a lake at radii up to `max_r`, `k` per ring.
fn across(lake: &Lake, max_r: f64, k: usize) -> Vec<([f64; 3], f64)> {
    let s = lake.seed;
    let mut out = vec![(s.centre, 0.0)];
    for ring in 1..=6 {
        let r = max_r * f64::from(ring) / 6.0;
        for j in 0..k {
            #[expect(clippy::cast_precision_loss, reason = "E-CAST: small counts")]
            let a = std::f64::consts::TAU * j as f64 / k as f64;
            let (sa, ca) = a.sin_cos();
            let p = [0, 1, 2].map(|i| {
                s.centre[i] + r * (ca * s.semi[0] * s.along[i] + sa * s.semi[1] * s.across[i])
            });
            let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            out.push((p.map(|v| v / len), r));
        }
    }
    out
}

#[test]
fn earth_like_land_holds_a_modest_number_of_lakes_of_each_kind() {
    let home = relief(&HOME);
    let lakes = lakes_of(&home, 120_000);
    assert!(
        (60..=1_500).contains(&lakes.len()),
        "{} lakes found",
        lakes.len()
    );
    for kind in [LakeKind::Crater, LakeKind::Caldera, LakeKind::Valley] {
        assert!(
            lakes.values().any(|l| l.seed.kind == kind),
            "no {kind:?} lake"
        );
    }
    for lake in lakes.values() {
        assert!(
            lake.full_m >= lake.level_m && lake.level_m > 0.0,
            "{lake:?}"
        );
    }
}

#[test]
fn lake_water_is_flat_and_under_its_rim() {
    // Criterion 6: flat water at or below the basin's rim.
    for chain in [HOME, ICE] {
        let r = relief(&chain);
        let lakes = lakes_of(&r, 60_000);
        assert!(!lakes.is_empty(), "{chain:?}");
        let mut wet = 0;
        for lake in lakes.values() {
            for (dir, _) in across(lake, 0.9, 16) {
                let g = r.ground(dir);
                if g.code.is_water() {
                    wet += 1;
                    assert_eq!(g.surface_m, lake.level_m.max(0.0), "{chain:?} {lake:?}");
                }
            }
            // On the rim no water stands above the terrain, and nearly all
            // of the rim stands above the water (the level is read from
            // 24 rim points; a notch between them only lets the water
            // fall away, never stand as a wall).
            let mut above = 0;
            for (dir, _) in across(lake, 1.0, 48).into_iter().skip(1 + 5 * 48) {
                let g = r.ground(dir);
                assert!(
                    !g.code.is_water() || g.surface_m <= 0.0,
                    "{chain:?} water at the rim of {lake:?}"
                );
                above += usize::from(g.terrain_m >= lake.level_m);
            }
            assert!(above >= 44, "{chain:?} rim {above}/48 above {lake:?}");
        }
        assert!(wet > 0, "{chain:?}");
    }
}

#[test]
fn partly_filled_lakes_show_a_dry_ring() {
    let home = relief(&HOME);
    let lakes = lakes_of(&home, 60_000);
    let partly: Vec<&Lake> = lakes.values().filter(|l| l.seed.drop > 0.0).collect();
    assert!(!partly.is_empty());
    let rings = partly
        .iter()
        .flat_map(|lake| across(lake, 0.95, 32))
        .filter(|(dir, _)| home.ground(*dir).code == WaterCode::RING)
        .count();
    assert!(
        rings > 0,
        "no ring point in {} partly filled lakes",
        partly.len()
    );
}

#[test]
fn dry_worlds_have_no_lakes() {
    for chain in DRY {
        let r = relief(&chain);
        for i in 0..5_000 {
            assert!(r.lake(fibonacci_dir(i, 5_000)).is_none(), "{chain:?}");
        }
    }
}

#[test]
fn the_basin_meets_the_terrain_at_its_rim() {
    // No step: the carved terrain tends to the terrain at the rim.
    let home = relief(&HOME);
    let lake = *lakes_of(&home, 60_000).values().next().expect("a lake");
    for base in [-50.0, 0.0, 300.0, 1_200.0] {
        let near = lake.carve_m(0.999_999, base);
        assert!(
            (near - base).abs() < 1e-6 * (1.0 + base.abs()),
            "{near} {base}"
        );
    }
    assert!(lake.seed.bowl(0.999_999) < 1e-5 && lake.seed.bowl(1.0) == 0.0);
}
