//! Landforms per region: volcanoes, rivers assembly, dunes, glacial
//! texture, coastal warp and karst (#685, T6).
//!
//! Guides: `guides/universe/L10/generation/documents/landforms.md`
//! (volcano profiles, dunes, glacial ground, karst) and the region cache
//! the plan files under landforms. Explicit landforms (volcanoes, rivers,
//! rift lakes) build once per land region and are cached; scattered
//! cinder cones and texture terms sample per query. Caldera water,
//! lava aprons and rift reaches fill through the flat-surface rule the
//! lakes share; T7 colours fresh, salty, dry and lava ground.
//!
//! Everything is `f64` and deterministic (`E-DET-TIERS`): the cache only
//! memoizes, never changes values (a flush test pins that). Sampling
//! stays allocation-free; construction allocates once per region
//! (`E-HOT-NOALLOC`).

use super::lakes::{RiftLake, find_rift_lakes, region_sample_dirs};
use super::plates::{BoundaryKind, Plates, boundary_at, plate_index_at, unit01_share};
use super::relief::base_unclamped_m;
use super::rivers::{River, build_rivers, channel_for_rivers, fan_for_rivers};
use crate::icosphere::normalize;
use std::cell::RefCell;
use std::collections::HashMap;

/// Cinder-cone class radius in metres (vents under 1 km, GEN-S056).
pub const CINDER_R_M: f64 = 150.0;

/// Cinder density near volcanic boundaries.
pub const CINDER_DENSITY_BOUNDARY: f64 = 0.4;

/// Cinder density on mobile ground.
pub const CINDER_DENSITY_MOBILE: f64 = 0.04;

/// Cinder density elsewhere.
pub const CINDER_DENSITY_QUIET: f64 = 0.005;

/// Cinder cone height in metres (rarely over 300 m, GEN-S056).
pub const CINDER_H_M: f64 = 100.0;
/// Cinder height span in metres.
pub const CINDER_H_SPAN_M: f64 = 200.0;

/// Dune crest height in metres.
pub const DUNE_M: f64 = 60.0;

/// Glacial carve depth in metres.
pub const GLACIAL_CARVE_M: f64 = 150.0;

/// Moraine ridge height in metres.
pub const MORAINE_M: f64 = 80.0;

/// Coastal warp in metres (rough natural coasts).
pub const COAST_WARP_M: f64 = 120.0;

/// Coastal band in metres of base relief.
pub const COAST_BAND_M: f64 = 250.0;

/// Sinkhole pit depth in metres.
pub const SINK_M: f64 = 40.0;

/// Karst tower height in metres.
pub const TOWER_M: f64 = 60.0;

/// Texture field base frequency and octaves (shared by every texture).
pub const TEXTURE_FREQ: f64 = 12.0;
/// Texture field octaves.
pub const TEXTURE_OCTAVES: u32 = 3;

/// Texture ridge field base frequency and octaves.
pub const TEXTURE_RIDGE_FREQ: f64 = 9.0;
/// Texture ridge octaves.
pub const TEXTURE_RIDGE_OCTAVES: u32 = 4;

/// Noise evaluations per landform sample (one fBm plus one ridged).
pub const LANDFORM_NOISE_EVALS_PER_SAMPLE: u32 = TEXTURE_OCTAVES + TEXTURE_RIDGE_OCTAVES;
/// Most cached regions (cleared past this; entries are per planet).
pub const REGION_CACHE_CAP: usize = 256;

/// Stream tag of the volcano draws.
const VOLCANO_TAG: u64 = 0xB01C_9685_0000_0000;
/// Stream tag of the carbonate mask.
const KARST_TAG: u64 = 0x4A97_9685_0000_0000;

/// Volcano kind (landforms guide, USGS groups).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum VolcKind {
    /// Wide gentle cone with a summit caldera (hot spots, rifts).
    Shield,
    /// Steep concave cone with a summit crater (volcanic arcs).
    Strato,
    /// Broad low shield with a huge caldera (rare, continental).
    Super,
}

/// One explicit volcano: stamped profile plus caldera water and apron.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Volcano {
    /// Centre (unit direction).
    pub(crate) centre: [f64; 3],
    /// Radius in radians.
    pub(crate) radius_ang: f64,
    /// Height in unscaled base metres (gravity-scaled at query).
    pub(crate) height_m: f64,
    /// Kind (profile shape).
    pub(crate) kind: VolcKind,
    /// Summit caldera as (radius fraction, depth metres).
    pub(crate) caldera: Option<(f64, f64)>,
    /// Base relief at the centre (unscaled metres, for water levels).
    pub(crate) centre_base_m: f64,
    /// Reach cosine for the quick reject (three radii).
    pub(crate) reach_cos: f64,
}

/// One land region's built landforms (cached by seed, region, radius).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RegionData {
    /// Water share (drives lakes, dunes, karst).
    pub(crate) water_01: f64,
    /// Rift-lake segments.
    pub(crate) lakes: Vec<RiftLake>,
    /// Explicit volcanoes.
    pub(crate) volcanoes: Vec<Volcano>,
    /// Rivers with deltas.
    pub(crate) rivers: Vec<River>,
}

/// Cache key: one planet's one region of one type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RegionKey {
    /// World seed.
    seed: u64,
    /// Plate index (0 on a lid).
    region: usize,
    /// Planet radius bits.
    radius_bits: u64,
    /// World kind discriminant.
    kind: u8,
}

thread_local! {
    /// Built regions by key (memoization only; values never depend on it).
    static REGIONS: RefCell<HashMap<RegionKey, RegionData>> =
        RefCell::new(HashMap::new());
}

/// Drop every cached region (tests pin cache independence with it).
#[cfg(test)]
pub(crate) fn flush_region_cache() {
    REGIONS.with(|cache| cache.borrow_mut().clear());
}

/// Cached region data, built on first touch.
fn region_cached<R>(
    seed: u64,
    region: usize,
    radius_m: f64,
    field: Option<&Plates>,
    kind: super::classify::WorldKind,
    use_fn: impl FnOnce(&RegionData) -> R,
) -> R {
    let key = RegionKey {
        seed,
        region,
        radius_bits: radius_m.to_bits(),
        kind: kind as u8,
    };
    REGIONS.with(|cache| {
        if let Some(data) = cache.borrow().get(&key) {
            let out = use_fn(data);
            return out;
        }
        let data = build_region(seed, region, radius_m, field, kind);
        let out = use_fn(&data);
        let mut cache = cache.borrow_mut();
        if cache.len() > REGION_CACHE_CAP {
            cache.clear();
        }
        cache.insert(key, data);
        out
    })
}

/// Rift lakes of the sample's region, from the cache (the colour pass
/// paints still water by kind).
#[must_use]
pub(crate) fn region_lakes(
    seed: u64,
    direction: [f64; 3],
    radius_m: f64,
    field: Option<&Plates>,
    kind: super::classify::WorldKind,
) -> Vec<RiftLake> {
    if !direction.iter().all(|c| c.is_finite()) || !radius_m.is_finite() || radius_m <= 0.0 {
        return Vec::new();
    }
    let region = field
        .and_then(|plates| plate_index_at(plates, direction))
        .unwrap_or(0);
    region_cached(seed, region, radius_m, field, kind, |data| {
        data.lakes.clone()
    })
}

/// Build one region: rift lakes, then volcanoes, then rivers.
fn build_region(
    seed: u64,
    region: usize,
    radius_m: f64,
    field: Option<&Plates>,
    kind: super::classify::WorldKind,
) -> RegionData {
    let water_01 = super::classify::draw_body(seed).water;
    let lakes = find_rift_lakes(seed, region, radius_m, field, water_01, kind);
    let volcanoes = build_volcanoes(seed, region, radius_m, field, kind);
    let rivers = build_rivers(seed, region, radius_m, field, &lakes, kind);
    RegionData {
        water_01,
        lakes,
        volcanoes,
        rivers,
    }
}

/// Push one volcano unless its radius is unusable on this planet.
fn push_volcano(
    out: &mut Vec<Volcano>,
    seed: u64,
    radius_m: f64,
    volcano: VolcanoSeed,
    kind: super::classify::WorldKind,
) {
    let radius_ang = volcano.radius_m_v / radius_m;
    if radius_ang > 0.3 || !radius_ang.is_finite() {
        return;
    }
    out.push(Volcano {
        centre: volcano.centre,
        radius_ang,
        height_m: volcano.height_m,
        kind: volcano.kind,
        caldera: volcano.caldera,
        centre_base_m: base_unclamped_m(seed, volcano.centre, kind),
        reach_cos: (3.0 * radius_ang).cos(),
    });
}

/// One volcano seed before its planet-relative fields are filled.
struct VolcanoSeed {
    /// Centre (unit direction).
    centre: [f64; 3],
    /// Radius in metres.
    radius_m_v: f64,
    /// Height in unscaled base metres.
    height_m: f64,
    /// Kind (profile shape).
    kind: VolcKind,
    /// Summit caldera as (radius fraction, depth metres).
    caldera: Option<(f64, f64)>,
}

/// Explicit volcanoes of one region: hotspot chains along the plate
/// motion, rift shields on divergent reaches, stratovolcano arcs above
/// subduction zones, and rare continental supervolcanoes (landforms
/// guide, volcano table).
fn build_volcanoes(
    seed: u64,
    region: usize,
    radius_m: f64,
    field: Option<&Plates>,
    kind: super::classify::WorldKind,
) -> Vec<Volcano> {
    let mut out = Vec::new();
    // Hotspot chains ride the plate motion (Hawaii rule: the chain
    // records the direction, older away from the active end).
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: chain counts below 4 always fit"
    )]
    #[expect(
        clippy::cast_sign_loss,
        reason = "E-CAST: chain counts non-negative, saturates like `as`"
    )]
    let chains = 1 + (unit01_share(seed, VOLCANO_TAG, 0) * 2.0) as usize % 2;
    for c in 0..chains.min(2) {
        let start = region_sample_dirs(field, seed, region, 12, VOLCANO_TAG + 1 + c as u64)
            .into_iter()
            .find(|dir| base_unclamped_m(seed, *dir, kind) > 200.0);
        let Some(start) = start else {
            continue;
        };
        let motion = field
            .and_then(|plates| {
                let axis = plates.axes.get(region).copied()?;
                let rate = plates.rates.get(region).copied()?;
                Some(normalize([
                    rate * (axis[1] * start[2] - axis[2] * start[1]),
                    rate * (axis[2] * start[0] - axis[0] * start[2]),
                    rate * (axis[0] * start[1] - axis[1] * start[0]),
                ]))
            })
            .unwrap_or([1.0, 0.0, 0.0]);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: chain counts below 8 always fit"
        )]
        #[expect(
            clippy::cast_sign_loss,
            reason = "E-CAST: chain counts non-negative, saturates like `as`"
        )]
        let count = 3 + (unit01_share(seed, VOLCANO_TAG, 4 + c as u64) * 3.0) as usize;
        for i in 0..count.min(5) {
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: chain steps below 8, exactly representable"
            )]
            let step = i as f64;
            let back = 0.025 * (1.0 + unit01_share(seed, VOLCANO_TAG, 8 + c as u64));
            let pos = normalize([
                start[0] - motion[0] * back * step,
                start[1] - motion[1] * back * step,
                start[2] - motion[2] * back * step,
            ]);
            let lane = 20 + c as u64 * 8 + i as u64;
            let radius = (8_000.0 + 12_000.0 * unit01_share(seed, VOLCANO_TAG, lane)
                - 2_000.0 * step)
                .max(6_000.0);
            let height = (3_000.0 + 5_000.0 * unit01_share(seed, VOLCANO_TAG, lane + 4)
                - 400.0 * step)
                .max(1_500.0);
            push_volcano(
                &mut out,
                seed,
                radius_m,
                VolcanoSeed {
                    centre: pos,
                    radius_m_v: radius,
                    height_m: height,
                    kind: VolcKind::Shield,
                    caldera: Some((0.12, 0.15 * height)),
                },
                kind,
            );
        }
    }
    // Rift shields and arc stratovolcanoes sit on their boundaries.
    let mut rifts = 0u32;
    let mut arcs = 0u32;
    for dir in region_sample_dirs(field, seed, region, 64, VOLCANO_TAG + 40) {
        let Some(plates) = field else {
            break;
        };
        let Some(edge) = boundary_at(plates, dir) else {
            continue;
        };
        let near: bool = out.iter().any(|v: &Volcano| {
            (v.centre[0] * dir[0] + v.centre[1] * dir[1] + v.centre[2] * dir[2])
                .clamp(-1.0, 1.0)
                .acos()
                < 0.06
        });
        if near || edge.edge_dist_rad > 0.02 {
            continue;
        }
        if edge.kind == BoundaryKind::Divergent && rifts < 2 {
            push_volcano(
                &mut out,
                seed,
                radius_m,
                VolcanoSeed {
                    centre: dir,
                    radius_m_v: 10_000.0,
                    height_m: 4_000.0,
                    kind: VolcKind::Shield,
                    caldera: Some((0.12, 600.0)),
                },
                kind,
            );
            rifts += 1;
        }
        if edge.kind == BoundaryKind::ConvergentOceanContinent
            && edge.side == super::plates::PlateKind::Continental
            && arcs < 6
        {
            let lane = 60 + u64::from(arcs);
            let height = 2_000.0 + 2_000.0 * unit01_share(seed, VOLCANO_TAG, lane + 8);
            push_volcano(
                &mut out,
                seed,
                radius_m,
                VolcanoSeed {
                    centre: dir,
                    radius_m_v: 6_000.0 + 4_000.0 * unit01_share(seed, VOLCANO_TAG, lane),
                    height_m: height,
                    kind: VolcKind::Strato,
                    caldera: Some((0.08, 0.1 * height)),
                },
                kind,
            );
            arcs += 1;
        }
    }
    // Rare continental supervolcanoes (Toba rule).
    if unit01_share(seed, VOLCANO_TAG, 90) < 0.12 {
        let inland = region_sample_dirs(field, seed, region, 16, VOLCANO_TAG + 91)
            .into_iter()
            .find(|dir| base_unclamped_m(seed, *dir, kind) > 500.0);
        if let Some(centre) = inland {
            push_volcano(
                &mut out,
                seed,
                radius_m,
                VolcanoSeed {
                    centre,
                    radius_m_v: 35_000.0,
                    height_m: 900.0,
                    kind: VolcKind::Super,
                    caldera: Some((0.45, 500.0)),
                },
                kind,
            );
        }
    }
    out
}

/// Stamped volcano relief in unscaled base metres (table profiles,
/// gravity-scaled at query like every relief amplitude).
fn volcano_relief(volcanoes: &[Volcano], direction: [f64; 3], gravity: f64) -> f64 {
    let scale = super::relief::relief_scale(gravity);
    let mut sum = 0.0;
    for volcano in volcanoes {
        let dot = direction[0] * volcano.centre[0]
            + direction[1] * volcano.centre[1]
            + direction[2] * volcano.centre[2];
        if dot < volcano.reach_cos {
            continue;
        }
        let rho = dot.clamp(-1.0, 1.0).acos() / volcano.radius_ang;
        if rho > 3.0 {
            continue;
        }
        let stamp = match volcano.kind {
            VolcKind::Shield | VolcKind::Super => volcano.height_m * (1.0 - rho).max(0.0).powf(1.5),
            VolcKind::Strato => volcano.height_m * (1.0 - rho).max(0.0).powf(0.6),
        };
        let caldera = volcano.caldera.map_or(0.0, |(frac, depth)| {
            -depth * (1.0 - super::noise3::smoothstep(0.0, frac, rho))
        });
        sum += (stamp + caldera) * scale;
    }
    sum
}

/// One cinder-lattice coordinate mapped into direction units.
#[expect(
    clippy::cast_precision_loss,
    reason = "E-CAST: lattice coords near the origin, exactly representable"
)]
fn cinder_axis(lattice: i64, cell: f64, jitter: f64) -> f64 {
    (lattice as f64 + 0.5 + (jitter - 0.5) * 0.5) * cell
}

/// Scattered cinder cones in unscaled base metres (one lattice class;
/// dense near volcanic boundaries, sparse elsewhere).
fn cinder_relief(
    seed: u64,
    direction: [f64; 3],
    radius_m: f64,
    field: Option<&Plates>,
    base_m: f64,
    gravity: f64,
) -> f64 {
    if base_m <= 0.0 {
        return 0.0;
    }
    let density = match field.and_then(|plates| boundary_at(plates, direction)) {
        Some(edge)
            if matches!(
                edge.kind,
                BoundaryKind::Divergent
                    | BoundaryKind::ConvergentOceanContinent
                    | BoundaryKind::ConvergentOceanOcean
            ) && edge.edge_dist_rad < 0.05 =>
        {
            CINDER_DENSITY_BOUNDARY
        }
        _ if field.is_some() => CINDER_DENSITY_MOBILE,
        _ => CINDER_DENSITY_QUIET,
    };
    let class_r = CINDER_R_M;
    let radius_ang = class_r / radius_m;
    if radius_ang > 0.5 {
        return 0.0;
    }
    let cell = 8.0 * radius_ang;
    let unit = normalize(direction);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: lattice coords near the origin always fit"
    )]
    let base = [
        (unit[0] / cell).floor() as i64,
        (unit[1] / cell).floor() as i64,
        (unit[2] / cell).floor() as i64,
    ];
    let cos_reach = (2.5 * radius_ang + cell * 0.9).cos();
    let mut sum = 0.0;
    for dx in -1i64..=1 {
        for dy in -1i64..=1 {
            for dz in -1i64..=1 {
                let lattice = [base[0] + dx, base[1] + dy, base[2] + dz];
                let [cx, cy, cz] = lattice;
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "E-CAST: lattice coords near the origin, exactly representable"
                )]
                let approx = normalize([
                    (cx as f64 + 0.5) * cell,
                    (cy as f64 + 0.5) * cell,
                    (cz as f64 + 0.5) * cell,
                ]);
                if unit[0] * approx[0] + unit[1] * approx[1] + unit[2] * approx[2] < cos_reach {
                    continue;
                }
                #[expect(
                    clippy::cast_sign_loss,
                    reason = "E-CAST: lattice coords reinterpreted as bits in the hash"
                )]
                let hash = crate::seed::hash_triple(
                    crate::seed::hash_triple(seed, VOLCANO_TAG, cx as u64),
                    cy as u64,
                    cz as u64,
                );
                if super::craters::share01(hash) >= density {
                    continue;
                }
                let centre = normalize([
                    cinder_axis(
                        cx,
                        cell,
                        super::craters::share01(crate::seed::hash_triple(hash, VOLCANO_TAG, 0)),
                    ),
                    cinder_axis(
                        cy,
                        cell,
                        super::craters::share01(crate::seed::hash_triple(hash, VOLCANO_TAG, 1)),
                    ),
                    cinder_axis(
                        cz,
                        cell,
                        super::craters::share01(crate::seed::hash_triple(hash, VOLCANO_TAG, 2)),
                    ),
                ]);
                let rho = (unit[0] * centre[0] + unit[1] * centre[1] + unit[2] * centre[2])
                    .clamp(-1.0, 1.0)
                    .acos()
                    / radius_ang;
                if rho > 2.5 {
                    continue;
                }
                let height = CINDER_H_M
                    + CINDER_H_SPAN_M
                        * super::craters::share01(crate::seed::hash_triple(hash, VOLCANO_TAG, 3));
                sum += height * (1.0 - rho).max(0.0);
            }
        }
    }
    sum * super::relief::relief_scale(gravity)
}

/// Texture relief in unscaled base metres: dunes on dry subtropical
/// land, glacial carve and moraines past 55 degrees, coastal warp at the
/// shoreline, karst pits (filled when wet) and towers in carbonate tropics.
fn texture_relief(
    seed: u64,
    region: usize,
    direction: [f64; 3],
    base_m: f64,
    water_01: f64,
    gravity: f64,
) -> f64 {
    let lat = direction[1].clamp(-1.0, 1.0).asin().abs();
    let (tex, _) = super::noise3::fbm_3d(
        crate::seed::hash_triple(seed, VOLCANO_TAG, 200),
        [
            direction[0] * TEXTURE_FREQ,
            direction[1] * TEXTURE_FREQ,
            direction[2] * TEXTURE_FREQ,
        ],
        TEXTURE_OCTAVES,
        2.0,
        0.5,
    );
    let ridge = super::noise3::ridged_3d(
        crate::seed::hash_triple(seed, VOLCANO_TAG, 201),
        [
            direction[0] * TEXTURE_RIDGE_FREQ,
            direction[1] * TEXTURE_RIDGE_FREQ,
            direction[2] * TEXTURE_RIDGE_FREQ,
        ],
        TEXTURE_RIDGE_OCTAVES,
    );
    let scale = super::relief::relief_scale(gravity);
    let mut sum = 0.0;
    if base_m > 0.0 {
        // Dunes: dry subtropical land wears crests, never pits.
        if (0.21..0.56).contains(&lat) && water_01 < 0.45 {
            sum += DUNE_M * super::noise3::smoothstep(0.3, 0.8, ridge);
        }
        // Glacial ground past 55 degrees: scoured hollows, moraine ridges.
        if lat > 0.96 {
            sum -= GLACIAL_CARVE_M * (1.0 - super::noise3::smoothstep(-0.9, -0.4, tex));
            sum += MORAINE_M
                * super::noise3::smoothstep(0.3, 0.6, tex)
                * (1.0 - super::noise3::smoothstep(0.6, 0.9, tex));
        }
        // Karst in carbonate regions: sink pits (lake-filled when wet),
        // towers in the wet tropics.
        if unit01_share(seed, KARST_TAG, region as u64) > 0.8 {
            let fill = super::lakes::lake_fill(water_01, 0.8);
            sum -= SINK_M * (1.0 - super::noise3::smoothstep(-1.1, -0.7, tex)) * (1.0 - fill);
            if lat < 0.35 && water_01 > 0.5 {
                sum += TOWER_M * super::noise3::smoothstep(0.7, 1.1, tex);
            }
        }
    }
    // Rough coasts: warp within a few hundred metres of sea level.
    let shore = (-(base_m / COAST_BAND_M) * (base_m / COAST_BAND_M)).exp();
    sum += COAST_WARP_M * shore * tex * 0.5;
    sum * scale
}

/// Landform relief in unscaled base metres for the built `field`
/// (`None` reads one lid region): volcanoes, cinder cones, river fans,
/// dunes, glacial ground, coastal warp and karst. River valleys and lake
/// water apply through the channel and fill queries.
pub(crate) fn relief_for_field(
    field: Option<&Plates>,
    seed: u64,
    direction: [f64; 3],
    radius_m: f64,
    base_m: f64,
    kind: super::classify::WorldKind,
) -> f64 {
    if !direction.iter().all(|c| c.is_finite()) || !radius_m.is_finite() || radius_m <= 0.0 {
        return 0.0;
    }
    let gravity = super::relief::gravity_for(seed);
    let region = field
        .and_then(|plates| plate_index_at(plates, direction))
        .unwrap_or(0);
    region_cached(seed, region, radius_m, field, kind, |data| {
        let unit = normalize(direction);
        let mut sum = volcano_relief(&data.volcanoes, unit, gravity);
        sum += cinder_relief(seed, unit, radius_m, field, base_m, gravity);
        sum += fan_for_rivers(&data.rivers, unit, radius_m) * super::relief::relief_scale(gravity);
        sum += texture_relief(seed, region, unit, base_m, data.water_01, gravity);
        sum
    })
}

/// Lowest channel target in unscaled base metres (`None` past every
/// valley): rivers carve toward it through [`channel_for_rivers`].
pub(crate) fn channel_for_field(
    field: Option<&Plates>,
    seed: u64,
    direction: [f64; 3],
    radius_m: f64,
    kind: super::classify::WorldKind,
) -> Option<f64> {
    if !direction.iter().all(|c| c.is_finite()) || !radius_m.is_finite() || radius_m <= 0.0 {
        return None;
    }
    let region = field
        .and_then(|plates| plate_index_at(plates, direction))
        .unwrap_or(0);
    region_cached(seed, region, radius_m, field, kind, |data| {
        channel_for_rivers(&data.rivers, direction, radius_m)
    })
}

/// Still-water rise in metres over ground at `pre_total` (the subtotal
/// before fills): rift reaches, caldera lakes and lava aprons share the
/// flat-surface rule (highest water wins).
pub(crate) fn fill_for_field(
    field: Option<&Plates>,
    seed: u64,
    direction: [f64; 3],
    radius_m: f64,
    pre_total: f64,
    kind: super::classify::WorldKind,
) -> f64 {
    if !direction.iter().all(|c| c.is_finite())
        || !radius_m.is_finite()
        || radius_m <= 0.0
        || !pre_total.is_finite()
    {
        return 0.0;
    }
    let region = field
        .and_then(|plates| plate_index_at(plates, direction))
        .unwrap_or(0);
    let unit = normalize(direction);
    region_cached(seed, region, radius_m, field, kind, |data| {
        let mut rise = 0.0f64;
        for lake in &data.lakes {
            let rho =
                (unit[0] * lake.centre[0] + unit[1] * lake.centre[1] + unit[2] * lake.centre[2])
                    .clamp(-1.0, 1.0)
                    .acos();
            if rho < lake.radius_ang {
                rise = rise.max(super::lakes::fill_rise(lake.level_m, pre_total));
            }
        }
        for volcano in &data.volcanoes {
            let dot = unit[0] * volcano.centre[0]
                + unit[1] * volcano.centre[1]
                + unit[2] * volcano.centre[2];
            if dot < volcano.reach_cos {
                continue;
            }
            let rho = dot.clamp(-1.0, 1.0).acos() / volcano.radius_ang;
            if let Some((frac, depth)) = volcano.caldera
                && rho < frac
            {
                let fill = super::lakes::lake_fill(data.water_01, 1.0);
                if fill > 0.0 {
                    let floor = volcano.centre_base_m + volcano.height_m - depth;
                    let level = floor + fill * depth;
                    rise = rise.max(super::lakes::fill_rise(level, pre_total));
                }
            }
            if rho < 2.0 && matches!(volcano.kind, VolcKind::Shield | VolcKind::Super) {
                let apron = volcano.centre_base_m + 0.03 * volcano.height_m;
                rise = rise.max(super::lakes::fill_rise(apron, pre_total));
            }
        }
        rise
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volcanoes_build_up_never_down() {
        // Stamps add relief; calderas dip but the edifice dominates.
        for seed in [7u64, 42, 999] {
            let input = super::super::plates::tectonic_input_for(seed);
            let field = super::super::plates::plate_field_for(
                seed,
                super::super::plates::EARTH_RADIUS_M,
                &input,
            );
            let volcanoes = build_volcanoes(
                seed,
                0,
                super::super::plates::EARTH_RADIUS_M,
                field.as_ref(),
                super::super::classify::WorldKind::EarthLike,
            );
            let gravity = super::super::relief::gravity_for(seed);
            let mut highest = 0.0f64;
            for dir in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.3, -0.5, 0.8]] {
                let relief = volcano_relief(&volcanoes, dir, gravity);
                assert!(relief >= -1.0, "volcano carves: {relief}");
                highest = highest.max(relief);
            }
            if field.is_some_and(|f| {
                f.regime == super::super::plates::Regime::Mobile
                    || f.regime == super::super::plates::Regime::Episodic
            }) {
                assert!(highest > 200.0, "no volcano on a live seed {seed}");
            }
        }
    }

    #[test]
    fn textures_stay_bounded_and_masked() {
        let gravity = 9.81;
        // Deep ocean wears nothing: land masks gate dunes, karst and ice,
        // and the coastal warp vanishes with depth.
        for dir in [normalize([0.0, 0.2, 0.9]), normalize([0.5, -0.3, 0.7])] {
            let deep = texture_relief(7, 0, dir, -3_000.0, 0.2, gravity);
            assert!(deep.abs() < 1e-9, "deep ocean textured: {deep}");
        }
        // Deserts crest, never pit; high latitudes carve and ridge.
        let mut dune_top = 0.0f64;
        let mut glacial_low = 0.0f64;
        for i in 0..64u32 {
            let lat = -1.2 + 2.4 * f64::from(i) / 63.0;
            let ring = (1.0 - lat.sin() * lat.sin()).max(0.0).sqrt();
            let dir = normalize([ring, lat.sin(), 0.0]);
            let relief = texture_relief(7, 0, dir, 800.0, 0.2, gravity);
            if lat.abs() < 0.56 {
                dune_top = dune_top.max(relief);
            }
            if lat.abs() > 0.96 {
                glacial_low = glacial_low.min(relief);
            }
        }
        assert!(dune_top > 5.0, "no dunes in the subtropics: {dune_top}");
        assert!(glacial_low < -20.0, "no glacial carve: {glacial_low}");
    }

    #[test]
    fn region_cache_replays_identically() {
        // The cache memoizes: flushing changes no value.
        let seed = 42u64;
        let radius_m = super::super::plates::EARTH_RADIUS_M;
        let input = super::super::plates::tectonic_input_for(seed);
        let field = super::super::plates::plate_field_for(seed, radius_m, &input);
        let dir = normalize([0.3, -0.5, 0.8]);
        let base = base_unclamped_m(seed, dir, super::super::classify::WorldKind::EarthLike);
        let before = (
            relief_for_field(
                field.as_ref(),
                seed,
                dir,
                radius_m,
                base,
                super::super::classify::WorldKind::EarthLike,
            ),
            channel_for_field(
                field.as_ref(),
                seed,
                dir,
                radius_m,
                super::super::classify::WorldKind::EarthLike,
            ),
            fill_for_field(
                field.as_ref(),
                seed,
                dir,
                radius_m,
                base,
                super::super::classify::WorldKind::EarthLike,
            ),
        );
        flush_region_cache();
        let after = (
            relief_for_field(
                field.as_ref(),
                seed,
                dir,
                radius_m,
                base,
                super::super::classify::WorldKind::EarthLike,
            ),
            channel_for_field(
                field.as_ref(),
                seed,
                dir,
                radius_m,
                super::super::classify::WorldKind::EarthLike,
            ),
            fill_for_field(
                field.as_ref(),
                seed,
                dir,
                radius_m,
                base,
                super::super::classify::WorldKind::EarthLike,
            ),
        );
        assert_eq!(before, after, "cache changes values");
    }
}
