//! Base relief: continents, hills, belts and detail (#685, T3).
//!
//! Guides: `guides/universe/L10/generation/documents/height-maps.md`
//! (the `h(dir)` recipe and the tuned starting values). This is the base
//! every later task extends: T4 replaces the provisional belt mask with
//! plates, T5 adds craters by surface age, T6 volcanoes, rivers and lakes,
//! T7 colours, T8 retunes the amplitudes per type. The staging is kind
//! agnostic on purpose: the frame does not know the orbit yet, so the base
//! uses the Earth-like row (about +0.75 km land, about −4.3 km ocean
//! floor, 9 km peak cap at 1 g) scaled by gravity for every world.
//!
//! Everything is `f64` and deterministic (`E-DET-TIERS`): the same seed
//! and direction always give the same metres. The octave schedule is
//! fixed, so a height never depends on the split depth; the cost is
//! bounded by [`NOISE_EVALS_PER_SAMPLE`] noise evaluations per sample
//! (`E-TEST-HERMETIC`, counted not timed).

use super::classify::draw_body;
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

/// Belt mask edges on the ridged field (provisional until T4 plates).
pub const BELT_LO: f64 = 0.45;
/// Belt mask upper edge.
pub const BELT_HI: f64 = 0.65;

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

/// Noise evaluations per relief sample: every field's octaves, the warp
/// sampled once per axis. The mesh cache and the frame budget read this,
/// never wall time.
pub const NOISE_EVALS_PER_SAMPLE: u32 =
    CONTINENT_OCTAVES + 3 * WARP_OCTAVES + HILLS_OCTAVES + BELT_OCTAVES + DETAIL_OCTAVES;

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
/// One pure function of the seed and the unit direction: continent mask
/// times the sea offset, hybrid hills rougher on high ground, a ridged
/// belt range (T4 plates add boundary relief on top), and slope-damped
/// fine detail, all scaled by gravity. A non-finite direction reads `0.0`.
#[must_use]
fn base_unclamped_m(seed: u64, direction: [f64; 3]) -> f64 {
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
    let mask = smoothstep(CONTINENT_LO, CONTINENT_HI, continent);
    let sea = OCEAN_FLOOR_M + (LAND_M - OCEAN_FLOOR_M) * mask;
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
    let hills = HILLS_M * hills_raw * rough;
    // Ridged ranges where the belt field runs high (T4 plates replace the
    // mask with boundary distances; the shape stays).
    let lane = crate::seed::hash_triple(seed, BELT_TAG, 0);
    let belt_field = ridged_3d(lane, scaled(bent, BELT_FREQ), BELT_OCTAVES);
    let belt01 = (belt_field / 1.2).clamp(0.0, 1.0);
    let ranges = RANGES_M * smoothstep(BELT_LO, BELT_HI, belt01) * belt01;
    // Fine detail, damped where the hills run steep.
    let lane = crate::seed::hash_triple(seed, DETAIL_TAG, 0);
    let (detail_raw, _) = fbm_3d(lane, scaled(bent, DETAIL_FREQ), DETAIL_OCTAVES, 2.0, 0.5);
    let slope = (hills_slope[0] * hills_slope[0]
        + hills_slope[1] * hills_slope[1]
        + hills_slope[2] * hills_slope[2])
        .sqrt();
    let detail = DETAIL_M * detail_raw / (1.0 + DETAIL_DAMP * slope);
    (sea + hills + ranges + detail) * scale
}

/// Base relief in metres above (or below) the mean radius: the uncapped
/// base within the peak and trench caps.
#[must_use]
pub fn base_relief_m(seed: u64, direction: [f64; 3]) -> f64 {
    let scale = relief_scale(gravity_for(seed));
    base_unclamped_m(seed, direction).clamp(TRENCH_CAP_M * scale, PEAK_CAP_M * scale)
}

/// Base plus plates plus craters in metres, within the caps.
///
/// T4 extends the height function with boundaries, T5 with craters: the
/// base plus [`crate::world::plates`] deltas plus
/// [`crate::world::craters`] deltas, capped together so the highest peak
/// still stays under its gravity cap. `radius_m` is the planet's mean
/// radius in metres (spreading ages and crater sizes read it). The plate
/// field builds once per sample and serves every pass.
#[must_use]
pub fn total_relief_m(seed: u64, direction: [f64; 3], radius_m: f64) -> f64 {
    let scale = relief_scale(gravity_for(seed));
    let field = super::plates::field_for_relief(seed, radius_m);
    let plate = field.as_ref().map_or(0.0, |plates| {
        super::plates::delta_for_field(plates, seed, direction)
    });
    let crater = super::craters::relief_for_field(field.as_ref(), seed, direction, radius_m);
    let total = base_unclamped_m(seed, direction) + plate + crater;
    total.clamp(TRENCH_CAP_M * scale, PEAK_CAP_M * scale)
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
                let total = total_relief_m(seed, dir, radius_m);
                assert!(
                    total >= TRENCH_CAP_M * scale - 1e-9 && total <= PEAK_CAP_M * scale + 1e-9,
                    "seed {seed}: {total} m outside its caps"
                );
                assert_eq!(total, total_relief_m(seed, dir, radius_m), "replay differs");
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
