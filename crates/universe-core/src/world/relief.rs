//! Base relief: continents, hills, belts and detail (#685, T3).
//!
//! Guides: `guides/universe/L10/generation/documents/height-maps.md`
//! (the `h(dir)` recipe and the tuned starting values). This is the base
//! every later task extends: T4 plates add boundary relief, T5 craters by
//! surface age, T6 volcanoes, rivers and lakes, T7 colours, T8 retunes
//! the amplitudes per type below. Kind-agnostic callers read the
//! Earth-like row; the seam passes each world's own kind.
//!
//! Everything is `f64` and deterministic (`E-DET-TIERS`): the same seed
//! and direction always give the same metres. The octave schedule is
//! fixed, so a height never depends on the split depth; the cost is
//! bounded by [`NOISE_EVALS_PER_SAMPLE`] noise evaluations per sample
//! (`E-TEST-HERMETIC`, counted not timed).

use super::classify::{WorldKind, draw_body};
use super::noise3::{fbm_3d, ridged_3d, smoothstep};
use crate::icosphere::normalize;

/// Land height at Earth gravity, in metres (height-maps guide, Earth row).
pub const LAND_M: f64 = 750.0;

/// Ocean floor depth at Earth gravity, in metres (the −4.3 km mode).
pub const OCEAN_FLOOR_M: f64 = -4_300.0;

/// Hybrid hill amplitude at Earth gravity, in metres (0.3-1 km row).
pub const HILLS_M: f64 = 650.0;

/// Mountain-range amplitude at Earth gravity, in metres (3-6 km row).
pub const RANGES_M: f64 = 4_500.0;

/// Fine-detail amplitude at Earth gravity, in metres.
pub const DETAIL_M: f64 = 80.0;

/// Highest peak at 1 g, in metres (the 9 km cap, scaled by gravity).
pub const PEAK_CAP_M: f64 = 9_000.0;

/// Deepest trench at 1 g, in metres (the −11 km floor, scaled by gravity).
pub const TRENCH_CAP_M: f64 = -11_000.0;

/// Continent field: base frequency and octaves.
pub const CONTINENT_FREQ: f64 = 2.0;
/// Continent field octaves.
pub const CONTINENT_OCTAVES: u32 = 4;

/// Continent mask edges on the continent field (land share about 29%).
pub const CONTINENT_LO: f64 = 0.0;
/// Continent mask upper edge.
pub const CONTINENT_HI: f64 = 0.4;

/// Domain-warp field: base frequency and octaves per axis.
pub const WARP_FREQ: f64 = 3.0;
/// Warp octaves per axis.
pub const WARP_OCTAVES: u32 = 3;

/// Warp strength in direction units (under one low-frequency feature).
pub const WARP_AMP: f64 = 0.12;

/// Hill field: base frequency and octaves.
pub const HILLS_FREQ: f64 = 6.0;
/// Hill octaves.
pub const HILLS_OCTAVES: u32 = 5;

/// Belt field (ridged): base frequency and octaves.
pub const BELT_FREQ: f64 = 3.0;
/// Belt octaves.
pub const BELT_OCTAVES: u32 = 4;

/// Belt mask edges on the ridged field (sparse mountain belts, so the
/// land mode stays sharp).
pub const BELT_LO: f64 = 0.60;
/// Belt mask upper edge.
pub const BELT_HI: f64 = 0.80;

/// Detail field: base frequency and octaves.
pub const DETAIL_FREQ: f64 = 96.0;
/// Detail octaves.
pub const DETAIL_OCTAVES: u32 = 5;

/// Slope damping of the detail: `1 / (1 + DETAIL_DAMP * slope)`.
pub const DETAIL_DAMP: f64 = 0.05;

/// Stream tag of the continent draw.
const CONTINENT_TAG: u64 = 0xC071_0685_C071_0685;
/// Stream tag of the warp draw (one lane per axis).
const WARP_TAG: u64 = 0x9A29_0685_0000_0000;
/// Stream tag of the hill draw.
const HILLS_TAG: u64 = 0x4115_0685_0000_0000;
/// Stream tag of the belt draw.
const BELT_TAG: u64 = 0xBE17_0685_0000_0000;
/// Stream tag of the detail draw.
const DETAIL_TAG: u64 = 0xDE7A_0685_0000_0000;

/// Stream tag of the dichotomy axis draw.
const DICHOTOMY_TAG: u64 = 0xD1C4_0685_0000_0000;

/// Per-type relief tuning at Earth gravity (height-maps guide, tuned
/// starting values). Giants keep the Earth-like row until T9 paints
/// their band fields; super-Earths share it (gravity flattens them).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeTuning {
    /// Land height in metres.
    pub land_m: f64,
    /// Ocean floor depth in metres.
    pub ocean_m: f64,
    /// Hybrid hill amplitude in metres.
    pub hills_m: f64,
    /// Belt-range amplitude in metres (scarps, pressure ridges).
    pub ranges_m: f64,
    /// Fine-detail amplitude in metres.
    pub detail_m: f64,
    /// Continent mask lower edge on the continent field.
    pub mask_lo: f64,
    /// Continent mask upper edge.
    pub mask_hi: f64,
    /// Hemispheric step in metres, 0 without a dichotomy (Mars row).
    pub dichotomy_m: f64,
    /// Highest peak at 1 g, in metres.
    pub peak_cap_m: f64,
    /// Deepest trench at 1 g, in metres.
    pub trench_cap_m: f64,
}

/// Relief tuning for one world type (T8).
#[must_use]
pub fn tuning_for(kind: WorldKind) -> TypeTuning {
    match kind {
        WorldKind::EarthLike | WorldKind::SuperEarth => TypeTuning {
            land_m: LAND_M,
            ocean_m: OCEAN_FLOOR_M,
            hills_m: HILLS_M,
            ranges_m: RANGES_M,
            detail_m: DETAIL_M,
            mask_lo: CONTINENT_LO,
            mask_hi: CONTINENT_HI,
            dichotomy_m: 0.0,
            peak_cap_m: PEAK_CAP_M,
            trench_cap_m: TRENCH_CAP_M,
        },
        WorldKind::Desert => TypeTuning {
            land_m: 1_500.0,
            ocean_m: -1_500.0,
            hills_m: 1_000.0,
            ranges_m: 3_000.0,
            detail_m: DETAIL_M,
            mask_lo: CONTINENT_LO,
            mask_hi: CONTINENT_HI,
            dichotomy_m: -3_000.0,
            peak_cap_m: PEAK_CAP_M,
            trench_cap_m: TRENCH_CAP_M,
        },
        WorldKind::Lava => TypeTuning {
            land_m: 50.0,
            ocean_m: -200.0,
            hills_m: 200.0,
            ranges_m: 0.0,
            detail_m: 40.0,
            mask_lo: -1.0,
            mask_hi: -0.4,
            dichotomy_m: 0.0,
            peak_cap_m: 3_000.0,
            trench_cap_m: -3_000.0,
        },
        WorldKind::Iron | WorldKind::Carbon => TypeTuning {
            land_m: 100.0,
            ocean_m: -500.0,
            hills_m: 750.0,
            ranges_m: 2_000.0,
            detail_m: 60.0,
            mask_lo: -0.8,
            mask_hi: -0.2,
            dichotomy_m: 0.0,
            peak_cap_m: 5_000.0,
            trench_cap_m: -5_000.0,
        },
        WorldKind::Ice => TypeTuning {
            land_m: 300.0,
            ocean_m: -800.0,
            hills_m: 300.0,
            ranges_m: 600.0,
            detail_m: 50.0,
            mask_lo: -0.8,
            mask_hi: -0.2,
            dichotomy_m: 0.0,
            peak_cap_m: 3_000.0,
            trench_cap_m: -3_000.0,
        },
        WorldKind::Ocean => TypeTuning {
            land_m: 400.0,
            ocean_m: -4_000.0,
            hills_m: 2_000.0,
            ranges_m: 0.0,
            detail_m: 100.0,
            mask_lo: 0.45,
            mask_hi: 0.75,
            dichotomy_m: 0.0,
            peak_cap_m: 5_000.0,
            trench_cap_m: TRENCH_CAP_M,
        },
        WorldKind::Dwarf => TypeTuning {
            land_m: 0.0,
            ocean_m: -300.0,
            hills_m: 200.0,
            ranges_m: 0.0,
            detail_m: 50.0,
            mask_lo: -0.8,
            mask_hi: -0.2,
            dichotomy_m: 0.0,
            peak_cap_m: 12_000.0,
            trench_cap_m: -6_000.0,
        },
        WorldKind::SubNeptune
        | WorldKind::IceGiant
        | WorldKind::GasGiant
        | WorldKind::HotJupiter
        | WorldKind::Rogue => TypeTuning {
            land_m: LAND_M,
            ocean_m: OCEAN_FLOOR_M,
            hills_m: HILLS_M,
            ranges_m: RANGES_M,
            detail_m: DETAIL_M,
            mask_lo: CONTINENT_LO,
            mask_hi: CONTINENT_HI,
            dichotomy_m: 0.0,
            peak_cap_m: PEAK_CAP_M,
            trench_cap_m: TRENCH_CAP_M,
        },
    }
}

/// Noise evaluations per relief sample: every field's octaves, the warp
/// sampled once per axis. The mesh cache and the frame budget read this,
/// never wall time.
pub const NOISE_EVALS_PER_SAMPLE: u32 =
    CONTINENT_OCTAVES + 3 * WARP_OCTAVES + HILLS_OCTAVES + BELT_OCTAVES + DETAIL_OCTAVES;

/// Height modes over pooled samples (the gallery hypsometry check):
/// kernel density at 24 points, local maxima above 8% of the tallest.
/// Edge bins never count (distributions peak interior in practice).
#[must_use]
pub fn hypsometry_modes(heights: &[f64]) -> usize {
    let mut evals = [0.0f64; 24];
    let mut count = 0u32;
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for sample in heights {
        let height = *sample;
        if !height.is_finite() {
            continue;
        }
        lo = lo.min(height);
        hi = hi.max(height);
        count += 1;
    }
    if count == 0 || hi.total_cmp(&lo) != std::cmp::Ordering::Greater {
        return usize::from(count > 0);
    }
    let width = (hi - lo) / 16.0;
    for (j, slot) in evals.iter_mut().enumerate() {
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: density index below 24, exactly representable"
        )]
        let at = lo + (hi - lo) * (j as f64 + 0.5) / 24.0;
        let mut density = 0.0;
        for sample in heights {
            let height = *sample;
            if !height.is_finite() {
                continue;
            }
            let t = (at - height) / width;
            density += (-t * t).exp();
        }
        *slot = density;
    }
    let mut tallest = 0.0f64;
    for density in evals.iter() {
        tallest = tallest.max(*density);
    }
    let mut modes = 0usize;
    let mut before = f64::INFINITY;
    let mut current = f64::INFINITY;
    let mut started = false;
    for density in evals.iter() {
        let here = *density;
        if started && current > before && current > here && current > 0.08 * tallest {
            modes += 1;
        }
        before = current;
        current = here;
        started = true;
    }
    modes
}

/// Relief scale for surface gravity `g` (height-maps guide): rock strength
/// caps relief as `1/g`, Earth setting the base. Unknown or degenerate
/// gravity reads Earth-like (`1.0`).
#[must_use]
pub fn relief_scale(gravity: f64) -> f64 {
    if !gravity.is_finite() || gravity <= 0.0 {
        return 1.0;
    }
    (9.81 / gravity).clamp(0.3, 40.0)
}

/// Surface gravity in m/s² for the body drawn from `seed`.
#[must_use]
pub fn gravity_for(seed: u64) -> f64 {
    let body = draw_body(seed);
    9.81 * body.mass_earth / (body.radius_earth * body.radius_earth)
}

/// Base relief in metres above (or below) the mean radius, before the cap.
///
/// One pure function of the seed, the unit direction and the type
/// tuning: continent mask times the sea offset, hybrid hills rougher on
/// high ground, ridged belt ranges, slope-damped fine detail and the
/// hemispheric dichotomy step, all scaled by gravity. A non-finite
/// direction reads `0.0`.
#[must_use]
pub(crate) fn base_unclamped_m(seed: u64, direction: [f64; 3], kind: WorldKind) -> f64 {
    base_unclamped_tuned(seed, direction, &tuning_for(kind))
}

/// Base relief with explicit tuning (the gallery and the seam share it).
fn base_unclamped_tuned(seed: u64, direction: [f64; 3], tuning: &TypeTuning) -> f64 {
    if !direction.iter().all(|c| c.is_finite()) {
        return 0.0;
    }
    let scale = relief_scale(gravity_for(seed));
    let unit = normalize(direction);
    let field = crate::seed::hash_triple(seed, CONTINENT_TAG, 0);
    let (continent, _) = fbm_3d(
        field,
        scaled(unit, CONTINENT_FREQ),
        CONTINENT_OCTAVES,
        2.0,
        0.5,
    );
    let mask = smoothstep(tuning.mask_lo, tuning.mask_hi, continent);
    let sea = tuning.ocean_m + (tuning.land_m - tuning.ocean_m) * mask;
    // Domain warp, one low-frequency field per axis.
    let lane_x = crate::seed::hash_triple(seed, WARP_TAG, 0);
    let lane_y = crate::seed::hash_triple(seed, WARP_TAG, 1);
    let lane_z = crate::seed::hash_triple(seed, WARP_TAG, 2);
    let (warp_x, _) = fbm_3d(lane_x, scaled(unit, WARP_FREQ), WARP_OCTAVES, 2.0, 0.5);
    let (warp_y, _) = fbm_3d(lane_y, scaled(unit, WARP_FREQ), WARP_OCTAVES, 2.0, 0.5);
    let (warp_z, _) = fbm_3d(lane_z, scaled(unit, WARP_FREQ), WARP_OCTAVES, 2.0, 0.5);
    let bent = normalize([
        unit[0] + WARP_AMP * warp_x,
        unit[1] + WARP_AMP * warp_y,
        unit[2] + WARP_AMP * warp_z,
    ]);
    // Hybrid hills: high ground rough, low ground smooth.
    let lane = crate::seed::hash_triple(seed, HILLS_TAG, 0);
    let (hills_raw, hills_slope) = fbm_3d(lane, scaled(bent, HILLS_FREQ), HILLS_OCTAVES, 2.0, 0.5);
    let rough = 0.35 + 0.65 * smoothstep(-2_500.0, 1_500.0, sea);
    let hills = tuning.hills_m * hills_raw * rough;
    // Ridged ranges where the belt field runs high (plates add boundary
    // relief on top; the shape stays). Full on land, low on the sea
    // floor, so both hypsometry modes stay sharp.
    let lane = crate::seed::hash_triple(seed, BELT_TAG, 0);
    let belt_field = ridged_3d(lane, scaled(bent, BELT_FREQ), BELT_OCTAVES);
    let belt01 = (belt_field / 1.2).clamp(0.0, 1.0);
    let ranges =
        tuning.ranges_m * smoothstep(BELT_LO, BELT_HI, belt01) * belt01 * (0.25 + 0.75 * mask);
    // Fine detail, damped where the hills run steep.
    let lane = crate::seed::hash_triple(seed, DETAIL_TAG, 0);
    let (detail_raw, _) = fbm_3d(lane, scaled(bent, DETAIL_FREQ), DETAIL_OCTAVES, 2.0, 0.5);
    let slope = (hills_slope[0] * hills_slope[0]
        + hills_slope[1] * hills_slope[1]
        + hills_slope[2] * hills_slope[2])
        .sqrt();
    let detail = tuning.detail_m * detail_raw / (1.0 + DETAIL_DAMP * slope);
    // Hemispheric dichotomy step (Mars row): one side drops.
    let dichotomy = if tuning.dichotomy_m == 0.0 {
        0.0
    } else {
        let axis = super::plates::sphere_dir(seed, DICHOTOMY_TAG, 0, 1);
        let side = unit[0] * axis[0] + unit[1] * axis[1] + unit[2] * axis[2];
        tuning.dichotomy_m * smoothstep(-0.3, 0.3, side)
    };
    (sea + hills + ranges + detail + dichotomy) * scale
}

/// Base relief in metres above (or below) the mean radius: the uncapped
/// base within the peak and trench caps.
#[must_use]
pub fn base_relief_m(seed: u64, direction: [f64; 3]) -> f64 {
    let scale = relief_scale(gravity_for(seed));
    base_unclamped_m(seed, direction, WorldKind::EarthLike)
        .clamp(TRENCH_CAP_M * scale, PEAK_CAP_M * scale)
}

/// Base plus plates plus craters plus landforms in metres, within the
/// per-type caps.
///
/// T4 extends the height function with boundaries, T5 with craters, T6
/// with volcanoes, rivers, dunes, glacial ground and karst, T8 retunes
/// every amplitude per type: every pass adds its deltas, river valleys
/// carve toward their channels, and still water fills to its level.
/// Capped together at the type's peak and trench caps times the gravity
/// scale, so the highest peak stays under its limit. `radius_m` is the
/// planet's mean radius in metres (spreading ages and landform sizes
/// read it). The plate field builds once per sample and serves every
/// pass. `None` reads the Earth-like row (kind-agnostic callers).
#[must_use]
pub fn total_relief_m(
    seed: u64,
    direction: [f64; 3],
    radius_m: f64,
    kind: Option<WorldKind>,
) -> f64 {
    let tuned_kind = kind.unwrap_or(WorldKind::EarthLike);
    let tuned = tuning_for(tuned_kind);
    let scale = relief_scale(gravity_for(seed));
    let base = base_unclamped_tuned(seed, direction, &tuned);
    let field = super::plates::field_for_relief(seed, radius_m);
    let plate = field.as_ref().map_or(0.0, |plates| {
        super::plates::delta_for_field(plates, seed, direction)
    });
    let crater = super::craters::relief_for_field(
        field.as_ref(),
        seed,
        direction,
        radius_m,
        base,
        tuned_kind,
    );
    let land = super::landforms::relief_for_field(
        field.as_ref(),
        seed,
        direction,
        radius_m,
        base,
        tuned_kind,
    );
    let mut total = base + plate + crater + land;
    if let Some(channel) =
        super::landforms::channel_for_field(field.as_ref(), seed, direction, radius_m, tuned_kind)
    {
        total = total.min(channel);
    }
    total += super::landforms::fill_for_field(
        field.as_ref(),
        seed,
        direction,
        radius_m,
        total,
        tuned_kind,
    );
    total.clamp(tuned.trench_cap_m * scale, tuned.peak_cap_m * scale)
}

/// Scales the unit direction by `frequency` for field sampling.
fn scaled(unit: [f64; 3], frequency: f64) -> [f64; 3] {
    [
        unit[0] * frequency,
        unit[1] * frequency,
        unit[2] * frequency,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spiral directions covering the sphere without a pole pinch.
    fn sweep_directions(count: u32) -> Vec<[f64; 3]> {
        let mut out = Vec::new();
        for i in 0..count {
            let step = f64::from(i);
            let total = f64::from(count);
            let y = 1.0 - 2.0 * (step + 0.5) / total;
            let r = (1.0 - y * y).max(0.0).sqrt();
            let angle = step * 2.399_963_229_728_653;
            out.push([r * angle.cos(), y, r * angle.sin()]);
        }
        out
    }

    #[test]
    fn relief_scale_follows_inverse_gravity() {
        assert_eq!(relief_scale(9.81), 1.0);
        assert!((relief_scale(9.81 / 2.6) - 2.6).abs() < 1e-12);
        assert_eq!(relief_scale(1.0e9), 0.3, "heavy worlds flatten");
        assert_eq!(relief_scale(1.0e-3), 40.0, "light worlds peak");
        assert_eq!(relief_scale(f64::NAN), 1.0);
        assert_eq!(relief_scale(0.0), 1.0);
        for seed in [1u64, 7, 42] {
            let g = gravity_for(seed);
            assert!(g.is_finite() && g > 0.0, "gravity {g}");
            let s = relief_scale(g);
            assert!((0.3..=40.0).contains(&s), "scale {s}");
        }
    }

    #[test]
    fn relief_respects_caps_and_hypsometry() {
        // C5 / AC5 (base part; T8 tunes per type): every sample stays under
        // its gravity cap, and pooled over seeds the worlds show land and
        // sea with both modes present.
        let dirs = sweep_directions(256);
        let mut land = 0u64;
        let mut total = 0u64;
        let mut highest = f64::NEG_INFINITY;
        let mut deepest = f64::INFINITY;
        for seed in 0..120u64 {
            let scale = relief_scale(gravity_for(seed));
            for dir in &dirs {
                let h = base_relief_m(seed, *dir);
                assert!(
                    h >= TRENCH_CAP_M * scale - 1e-9 && h <= PEAK_CAP_M * scale + 1e-9,
                    "seed {seed}: {h} m outside its caps"
                );
                if h > 0.0 {
                    land += 1;
                }
                total += 1;
                highest = highest.max(h);
                deepest = deepest.min(h);
            }
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: test share from small counts, approximate"
        )]
        let share = land as f64 / total as f64;
        assert!(
            (0.05..0.70).contains(&share),
            "land share {share} outside the broad band"
        );
        assert!(highest > 300.0, "no land mode: {highest} m");
        assert!(deepest < -2_000.0, "no sea mode: {deepest} m");
        assert_eq!(base_relief_m(7, [f64::NAN, 0.0, 0.0]), 0.0);
        // T8 gallery (height-maps guide, gallery check): per-type caps on
        // the capped total, land shares, and modes on the base (continents
        // against ocean floor; landforms ride on top). Earth-like worlds
        // show two modes, airless dwarfs one.
        let radius_m = crate::terrain::EARTH_RADIUS_KM * 1000.0;
        for (kind, share_lo, share_hi, min_peak) in [
            (WorldKind::EarthLike, 0.05, 0.70, 1_500.0),
            (WorldKind::Desert, 0.20, 1.00, 800.0),
            (WorldKind::Lava, 0.50, 1.00, 50.0),
            (WorldKind::Iron, 0.20, 1.00, 150.0),
            (WorldKind::Ice, 0.20, 1.00, 80.0),
            (WorldKind::Ocean, 0.00, 0.30, 300.0),
            (WorldKind::Dwarf, 0.20, 1.00, 80.0),
            (WorldKind::Carbon, 0.20, 1.00, 80.0),
            (WorldKind::SuperEarth, 0.05, 0.70, 800.0),
        ] {
            let tuning = tuning_for(kind);
            let mut pooled = Vec::new();
            let mut land = 0u64;
            let mut count = 0u64;
            let mut peak = 0.0f64;
            for seed in 0..40u64 {
                let scale = relief_scale(gravity_for(seed));
                for dir in sweep_directions(64) {
                    let height = total_relief_m(seed, dir, radius_m, Some(kind));
                    assert!(
                        height >= tuning.trench_cap_m * scale - 1.0
                            && height <= tuning.peak_cap_m * scale + 1.0,
                        "{kind:?} seed {seed}: {height} m outside its caps"
                    );
                    // Modes read shape, not size: normalize each seed by
                    // its own gravity scale first.
                    pooled.push(base_unclamped_tuned(seed, dir, &tuning) / scale);
                    if height > 0.0 {
                        land += 1;
                    }
                    count += 1;
                    peak = peak.max(height);
                }
            }
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: test share from small counts, approximate"
            )]
            let share = land as f64 / count as f64;
            assert!(
                (share_lo..=share_hi).contains(&share),
                "{kind:?} land share {share}"
            );
            assert!(peak > min_peak, "{kind:?} peak {peak} m too low");
            if kind == WorldKind::EarthLike {
                assert_eq!(hypsometry_modes(&pooled), 2, "earth should read two modes");
            }
            if kind == WorldKind::Dwarf {
                assert_eq!(hypsometry_modes(&pooled), 1, "dwarf should read one mode");
            }
        }
    }

    #[test]
    fn relief_is_deterministic_and_direction_pure() {
        for (seed, dir) in [(7u64, [0.3, -0.5, 0.8]), (42, [0.0, 1.0, 0.0])] {
            assert_eq!(base_relief_m(seed, dir), base_relief_m(seed, dir));
            assert_ne!(base_relief_m(seed, dir), base_relief_m(seed + 1, dir));
            // Scaling the direction changes nothing but float datum: the
            // normalize runs differ by 1 ulp and steep octaves amplify it,
            // far below any gameplay scale (as in terrain).
            let long = [dir[0] * 10.0, dir[1] * 10.0, dir[2] * 10.0];
            assert!(
                (base_relief_m(seed, dir) - base_relief_m(seed, long)).abs() < 1e-9,
                "orbit and ground disagree"
            );
        }
    }

    #[test]
    fn total_relief_stays_within_caps() {
        // The seam reads the total (base plus plates): capped together so
        // the highest peak stays under its gravity cap (C5 gallery check).
        let radius_m = crate::terrain::EARTH_RADIUS_KM * 1000.0;
        for seed in 0..60u64 {
            let scale = relief_scale(gravity_for(seed));
            for dir in sweep_directions(128) {
                let total = total_relief_m(seed, dir, radius_m, None);
                assert!(
                    total >= TRENCH_CAP_M * scale - 1e-9 && total <= PEAK_CAP_M * scale + 1e-9,
                    "seed {seed}: {total} m outside its caps"
                );
                assert_eq!(
                    total,
                    total_relief_m(seed, dir, radius_m, None),
                    "replay differs"
                );
            }
        }
    }

    #[test]
    fn relief_cost_stays_within_the_operation_budget() {
        // E-TEST-HERMETIC: an operation count, never wall time. The count
        // is computed from the same octave schedule the sampler runs.
        assert_eq!(NOISE_EVALS_PER_SAMPLE, 4 + 3 * 3 + 5 + 4 + 5);
        const _: () = assert!(
            NOISE_EVALS_PER_SAMPLE <= 40,
            "schedule grew past the budget"
        );
    }
}
