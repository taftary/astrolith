//! Frozen copy of the relief height maths as it stood before the #994 speed
//! work (test-only). Every faster path is checked against it: the ground
//! must look exactly the same (#994 Q2 A, AC1). Never edit these numbers to
//! follow a change in `relief.rs` or `noise.rs`: a difference is the point.

use super::{
    AMP_HILLS_M, AMP_RANGES_M, BELT_FREQ, CONTINENT_FREQ, CONTINENT_THRESHOLD, HILLS_FREQ,
    LAND_MAX_M, LAND_MIN_M, OCEAN_FLOOR_M, RANGES_FREQ, Relief, WARP_FREQ, WARP_STRENGTH,
};

const NOISE_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;
const NOISE_MULT_A: u64 = 0xBF58_476D_1CE4_E5B9;
const NOISE_MULT_B: u64 = 0x94D0_49BB_1331_11EB;
const INV_2_POW_53: f64 = 1.0 / 9_007_199_254_740_992.0;
const SAFE: f64 = 9_007_199_254_740_991.0;

const fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(NOISE_MULT_A);
    z = (z ^ (z >> 27)).wrapping_mul(NOISE_MULT_B);
    z ^ (z >> 31)
}

#[expect(
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "E-CAST: frozen copy of noise::lattice_value_aperiodic"
)]
fn lattice(seed: u64, ix: i64, iy: i64, iz: i64) -> f64 {
    let mut hash = seed.wrapping_add(NOISE_GAMMA);
    hash = mix(hash ^ (ix as u64).wrapping_mul(NOISE_MULT_A));
    hash = mix(hash ^ (iy as u64).wrapping_mul(NOISE_MULT_B));
    hash = mix(hash ^ (iz as u64).wrapping_mul(NOISE_MULT_A));
    hash = mix(hash ^ NOISE_MULT_B);
    ((hash >> 11) as f64) * INV_2_POW_53
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "E-CAST: frozen copy of noise::value_noise_3d_aperiodic"
)]
fn noise(seed: u64, x: f64, y: f64, z: f64) -> f64 {
    if !x.is_finite() || !y.is_finite() || !z.is_finite() {
        return 0.5;
    }
    let (xif, yif, zif) = (x.floor(), y.floor(), z.floor());
    if !(-SAFE..=SAFE).contains(&xif) || !(-SAFE..=SAFE).contains(&yif) {
        return 0.5;
    }
    if !(-SAFE..=SAFE).contains(&zif) {
        return 0.5;
    }
    let (xi, yi, zi) = (xif as i64, yif as i64, zif as i64);
    let xf = fade(x - xi as f64);
    let yf = fade(y - yi as f64);
    let zf = fade(z - zi as f64);
    let c000 = lattice(seed, xi, yi, zi);
    let c100 = lattice(seed, xi + 1, yi, zi);
    let c010 = lattice(seed, xi, yi + 1, zi);
    let c110 = lattice(seed, xi + 1, yi + 1, zi);
    let c001 = lattice(seed, xi, yi, zi + 1);
    let c101 = lattice(seed, xi + 1, yi, zi + 1);
    let c011 = lattice(seed, xi, yi + 1, zi + 1);
    let c111 = lattice(seed, xi + 1, yi + 1, zi + 1);
    lerp(
        lerp(lerp(c000, c100, xf), lerp(c010, c110, xf), yf),
        lerp(lerp(c001, c101, xf), lerp(c011, c111, xf), yf),
        zf,
    )
}

fn fbm(p: [f64; 3], base_freq: f64, seeds: &[u64]) -> f64 {
    let (mut sum, mut norm, mut freq, mut amp) = (0.0f64, 0.0f64, base_freq, 1.0f64);
    for seed in seeds {
        sum += amp * noise(*seed, p[0] * freq, p[1] * freq, p[2] * freq);
        norm += amp;
        freq *= 2.0;
        amp *= 0.5;
    }
    if norm > 0.0 && sum.is_finite() {
        sum / norm
    } else {
        0.5
    }
}

fn ridged(p: [f64; 3], base_freq: f64, seeds: &[u64]) -> f64 {
    let (mut sum, mut norm, mut freq, mut amp, mut weight) =
        (0.0f64, 0.0f64, base_freq, 1.0f64, 1.0f64);
    for seed in seeds {
        let n = noise(*seed, p[0] * freq, p[1] * freq, p[2] * freq);
        let signal = 1.0 - (2.0 * n - 1.0).abs();
        sum += amp * (signal * signal * weight);
        norm += amp;
        weight = signal.clamp(0.0, 1.0);
        freq *= 2.0;
        amp *= 0.5;
    }
    if norm > 0.0 && sum.is_finite() {
        (sum / norm).clamp(0.0, 1.0)
    } else {
        0.5
    }
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    if !x.is_finite() {
        return 0.0;
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn length(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Height in metres at `dir`, exactly as `Relief::height_m` computed it
/// before #994.
pub(crate) fn height_m(relief: &Relief, dir: [f64; 3]) -> f64 {
    if relief.is_giant || dir.iter().any(|c| !c.is_finite()) {
        return 0.0;
    }
    let len = length(dir);
    if !len.is_finite() || len <= 0.0 {
        return 0.0;
    }
    let unit = [dir[0] / len, dir[1] / len, dir[2] / len];
    let w = [
        unit[0] * WARP_FREQ,
        unit[1] * WARP_FREQ,
        unit[2] * WARP_FREQ,
    ];
    let wa = fbm(w, 1.0, &relief.warp_a_oct);
    let wb = fbm(w, 1.0, &relief.warp_b_oct);
    let warped_in = [
        unit[0] + (wa - 0.5) * WARP_STRENGTH * 2.0,
        unit[1] + (wb - 0.5) * WARP_STRENGTH * 2.0,
        unit[2] + ((wa + wb) / 2.0 - 0.5) * WARP_STRENGTH * 2.0,
    ];
    let wlen = length(warped_in);
    let warped = if wlen.is_finite() && wlen > 0.0 {
        [
            warped_in[0] / wlen,
            warped_in[1] / wlen,
            warped_in[2] / wlen,
        ]
    } else {
        unit
    };
    let continent = fbm(warped, CONTINENT_FREQ, &relief.continent_oct);
    let land_t = smoothstep(
        CONTINENT_THRESHOLD - 0.02,
        CONTINENT_THRESHOLD + 0.02,
        continent,
    );
    let land_h = if continent > CONTINENT_THRESHOLD {
        let t = ((continent - CONTINENT_THRESHOLD) / (1.0 - CONTINENT_THRESHOLD)).clamp(0.0, 1.0);
        LAND_MIN_M + (LAND_MAX_M - LAND_MIN_M) * t
    } else {
        LAND_MIN_M
    };
    let sea_offset = (1.0 - land_t) * OCEAN_FLOOR_M + land_t * land_h;
    let hills_f = fbm(warped, HILLS_FREQ, &relief.hills_oct);
    let hills = (hills_f - 0.5) * 2.0 * AMP_HILLS_M * (0.3 + 0.7 * land_t);
    let belt = smoothstep(0.55, 0.75, fbm(unit, BELT_FREQ, &relief.belt_oct));
    let signed = belt * (2.0 * land_t - 1.0);
    let ranges = signed * ridged(warped, RANGES_FREQ, &relief.ranges_oct) * AMP_RANGES_M;
    let total = sea_offset + hills + ranges;
    if !total.is_finite() {
        return 0.0;
    }
    total * relief.relief_scale
}
