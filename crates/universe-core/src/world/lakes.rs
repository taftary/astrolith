//! Pure lakes: basins the generator already knows, plus rift segments
//! (#685, T6).
//!
//! Guides: `guides/universe/L10/generation/documents/lakes.md` (pure
//! lakes: crater bowls, calderas, rift troughs, kettles and sinkholes,
//! each with `level = floor + fill × (rim - floor)`; baked grid lakes are
//! out as non-pure). Kettle and sinkhole pits fill inside the landform
//! texture terms; crater bowls fill inside the crater pass (both know
//! their own floor and rim); calderas fill at their volcanoes; rift
//! troughs need segments, built here per land region and cached with it.
//! T7 colours fresh, salty, dry and lava water.
//!
//! Everything is `f64` and deterministic (`E-DET-TIERS`); sampling stays
//! allocation-free, construction allocates once per region and is cached
//! (`E-HOT-NOALLOC`).

use super::plates::{BoundaryKind, Plates, boundary_at, plate_index_at};
use super::relief::base_unclamped_m;
use crate::icosphere::normalize;

/// Smallest crater radius holding still water, in metres (big bowls read
/// from orbit; small pits stay dry).
pub const BOWL_LAKE_MIN_R_M: f64 = 8_000.0;

/// Most rift lakes per land region.
pub const MAX_RIFT_LAKES: usize = 2;

/// Rift-lake radius in radians (about 64 km at Earth size).
pub const RIFT_LAKE_RADIUS_RAD: f64 = 0.010;

/// Lake water: fresh, salty closed, dry playa, or lava.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LakeKind {
    /// Open or fresh closed water.
    Fresh,
    /// Closed water below its rim (salt ring in T7).
    Salty,
    /// Dry bed: flat bright floor, no water.
    Dry,
    /// Molten rock on heat-pipe worlds (glowing palette in T7).
    Lava,
}

/// Liquid standing on one world (planet-scale water budget; per-basin
/// kinds refine it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LakeLiquid {
    /// Water lakes and playas.
    Water,
    /// Lava lakes on heat-pipe worlds.
    Lava,
}

/// Which liquid pools on the world seeded `seed` (lava on heat-pipe
/// worlds, water everywhere else with air).
#[must_use]
pub fn surface_liquid(seed: u64) -> LakeLiquid {
    if super::plates::tectonic_input_for(seed).heat_01 > 0.85 {
        LakeLiquid::Lava
    } else {
        LakeLiquid::Water
    }
}

/// Whether a world of `radius_m` metres holds air (provisional
/// size-based proxy until T7 wires the shoreline; airless bodies hold no
/// lakes per the guides).
#[must_use]
pub fn has_air(radius_m: f64) -> bool {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return false;
    }
    crate::terrain::air_for_radius_earth(radius_m / super::plates::EARTH_RADIUS_M).is_some()
}

/// Fill fraction 0..1 from the liquid budget (lakes guide): the water
/// share times a basin factor (big catchments fill fuller).
#[must_use]
pub fn lake_fill(water_01: f64, basin_factor: f64) -> f64 {
    if !water_01.is_finite() || !basin_factor.is_finite() {
        return 0.0;
    }
    (water_01 * basin_factor).clamp(0.0, 1.0)
}

/// Kind from the fill: full reads fresh, partial salty, empty dry.
#[must_use]
pub fn lake_kind(fill: f64) -> LakeKind {
    if fill >= 1.0 {
        LakeKind::Fresh
    } else if fill > 0.0 {
        LakeKind::Salty
    } else {
        LakeKind::Dry
    }
}

/// Hashed unit directions in `region` (`count` tries): the shared sampler
/// for region construction (lakes, volcanoes, rivers). Stagnant worlds
/// read one region over the whole planet.
#[must_use]
pub(crate) fn region_sample_dirs(
    field: Option<&Plates>,
    seed: u64,
    region: usize,
    count: u32,
    tag: u64,
) -> Vec<[f64; 3]> {
    let mut out = Vec::new();
    for i in 0..count {
        let lane = u64::from(i);
        let rise = super::plates::unit01_share(seed, tag, 2 * lane) * 2.0 - 1.0;
        let around = super::plates::unit01_share(seed, tag, 2 * lane + 1) * std::f64::consts::TAU;
        let ring = (1.0 - rise * rise).max(0.0).sqrt();
        let dir = [ring * around.cos(), rise, ring * around.sin()];
        let owned = field.map_or(region == 0, |plates| {
            plate_index_at(plates, dir).is_some_and(|index| index == region)
        });
        if owned {
            out.push(dir);
        }
    }
    out
}

/// One rift-lake segment: still water in a trough reach.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RiftLake {
    /// Centre (unit direction on the boundary).
    pub(crate) centre: [f64; 3],
    /// Across-trough unit direction (trough walls contain the water;
    /// along-trough sills run sub-resolution).
    pub(crate) across: [f64; 3],
    /// Radius in radians.
    pub(crate) radius_ang: f64,
    /// Water level in unscaled base metres (flat surface).
    pub(crate) level_m: f64,
    /// Fresh, salty or dry bed.
    pub(crate) kind: LakeKind,
}

/// Rift-lake segments of one land region: still reaches along its
/// divergent continental boundaries (lakes guide: long, narrow, deep
/// rift-valley lakes in a chain). Levels sit below the local base inside
/// the trough the plates carve; ring samples hold them below the rim.
#[must_use]
pub(crate) fn find_rift_lakes(
    seed: u64,
    region: usize,
    radius_m: f64,
    field: Option<&Plates>,
    water_01: f64,
    kind: super::classify::WorldKind,
) -> Vec<RiftLake> {
    let Some(plates) = field else {
        return Vec::new();
    };
    if !radius_m.is_finite() || radius_m <= 0.0 || !has_air(radius_m) {
        return Vec::new();
    }
    // Edge points on divergent continental boundaries in this region.
    let mut edges = Vec::new();
    for dir in region_sample_dirs(field, seed, region, 96, 0x91F7_E685_0000_0000) {
        let Some(edge) = boundary_at(plates, dir) else {
            continue;
        };
        if edge.kind == BoundaryKind::Divergent
            && edge.side == super::plates::PlateKind::Continental
            && edge.edge_dist_rad < 0.02
        {
            edges.push((dir, edge.normal));
        }
    }
    // Up to two reaches far apart, levels below the trough shoulders
    // (across-trough walls hold the water; sills close the reach ends
    // below the mesh scale).
    let mut lakes = Vec::new();
    for (dir, across) in edges {
        if lakes.len() >= MAX_RIFT_LAKES {
            break;
        }
        let centre = normalize(dir);
        if lakes.iter().any(|lake: &RiftLake| {
            (lake.centre[0] * centre[0] + lake.centre[1] * centre[1] + lake.centre[2] * centre[2])
                .clamp(-1.0, 1.0)
                .acos()
                < 0.12
        }) {
            continue;
        }
        let base = base_unclamped_m(seed, centre, kind);
        let mut shoulder = f64::INFINITY;
        for side in [-1.0, 1.0] {
            let wall = normalize([
                centre[0] + across[0] * side * RIFT_LAKE_RADIUS_RAD * 1.5,
                centre[1] + across[1] * side * RIFT_LAKE_RADIUS_RAD * 1.5,
                centre[2] + across[2] * side * RIFT_LAKE_RADIUS_RAD * 1.5,
            ]);
            shoulder = shoulder.min(base_unclamped_m(seed, wall, kind));
        }
        let fill = lake_fill(water_01, 1.5);
        let level = (base - 200.0).min(shoulder - 20.0);
        lakes.push(RiftLake {
            centre,
            across,
            radius_ang: RIFT_LAKE_RADIUS_RAD,
            level_m: level,
            kind: if fill <= 0.0 && water_01 < 0.15 {
                LakeKind::Dry
            } else {
                lake_kind(fill)
            },
        });
    }
    lakes
}

/// Fill rise in metres for still water at `level_m` over ground at
/// `surface_m` (both unscaled): the flat-surface rule every basin shares.
#[must_use]
pub(crate) fn fill_rise(level_m: f64, surface_m: f64) -> f64 {
    (level_m - surface_m).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liquid_budget_fills_and_kinds() {
        assert_eq!(lake_fill(0.8, 1.2), 0.96);
        assert_eq!(lake_fill(0.0, 1.5), 0.0);
        assert_eq!(lake_fill(1.0, 2.0), 1.0);
        assert_eq!(lake_fill(f64::NAN, 1.0), 0.0);
        assert_eq!(lake_kind(1.0), LakeKind::Fresh);
        assert_eq!(lake_kind(0.4), LakeKind::Salty);
        assert_eq!(lake_kind(0.0), LakeKind::Dry);
        assert_eq!(fill_rise(100.0, 60.0), 40.0);
        assert_eq!(fill_rise(100.0, 140.0), 0.0);
        assert!(has_air(super::super::plates::EARTH_RADIUS_M));
        assert!(!has_air(0.3 * super::super::plates::EARTH_RADIUS_M));
        assert!(!has_air(f64::NAN));
    }

    #[test]
    fn rift_lakes_sit_below_their_rims() {
        // Level below the ring rim on a divergent continental boundary.
        let mut found = 0u32;
        for seed in 0..60u64 {
            let input = super::super::plates::tectonic_input_for(seed);
            if !matches!(
                super::super::plates::regime_for(seed, &input),
                super::super::plates::Regime::Mobile
            ) {
                continue;
            }
            let Some(field) = super::super::plates::plate_field_for(
                seed,
                super::super::plates::EARTH_RADIUS_M,
                &input,
            ) else {
                continue;
            };
            for region in 0..field.count {
                let lakes = find_rift_lakes(
                    seed,
                    region,
                    super::super::plates::EARTH_RADIUS_M,
                    Some(&field),
                    0.6,
                    super::super::classify::WorldKind::EarthLike,
                );
                for lake in &lakes {
                    assert!((0.004..=0.05).contains(&lake.radius_ang));
                    for side in [-1.0, 1.0] {
                        let wall = normalize([
                            lake.centre[0] + lake.across[0] * side * lake.radius_ang * 1.5,
                            lake.centre[1] + lake.across[1] * side * lake.radius_ang * 1.5,
                            lake.centre[2] + lake.across[2] * side * lake.radius_ang * 1.5,
                        ]);
                        let shoulder = base_unclamped_m(
                            seed,
                            wall,
                            super::super::classify::WorldKind::EarthLike,
                        );
                        assert!(
                            lake.level_m <= shoulder + 1.0,
                            "lake above its trough wall: {} vs {shoulder}",
                            lake.level_m
                        );
                    }
                    assert_eq!(lake.kind, LakeKind::Salty);
                    found += 1;
                }
            }
            if found >= 4 {
                break;
            }
        }
        assert!(found >= 4, "no rift lakes found to check");
    }
}
