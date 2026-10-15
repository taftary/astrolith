//! Deterministic 3D gradient noise with analytic derivatives (#685, T3).
//!
//! Guides: `guides/universe/L10/generation/documents/height-maps.md`
//! (building blocks 1-4: gradient noise, fractal sums, multifractals,
//! domain warping). Gradient (Perlin) noise is smoother and less grid-like
//! than the value noise in `crate::noise`, so it carries the relief;
//! value noise stays where a coarse field is enough. Like `crate::noise`
//! everything here is integer hashing plus plain float arithmetic, so the
//! same seed gives the same field on every platform (`E-DET-TIERS`), and
//! the field wraps every 256 units per axis, so it has no seam.
//!
//! The derivatives are analytic (the interpolation polynomial
//! differentiated exactly), not finite differences: the relief damps its
//! fine detail by the coarse slope, which needs the slope at the same
//! point the height is sampled.

/// Splitmix64-style avalanche finalizer with its own gamma, so this
/// stream never follows the value noise in [`crate::noise`].
const fn mix_noise(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Lattice period in field units: coordinates wrap every 256 units, and
/// lattice indices wrap modulo 256, so opposite period edges hash
/// identically and the wrap seam vanishes (as in [`crate::noise`]).
const NOISE_PERIOD: f64 = 256.0;

/// Lattice sub-period matching [`NOISE_PERIOD`] in integer steps.
const NOISE_LATTICE_PERIOD: i64 = 256;

/// Perlin's twelve edge gradients (unnormalized, each of length `sqrt(2)`).
const GRADIENTS: [[f64; 3]; 12] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
];

/// Wraps a coordinate into `[0.0, 256.0)`; deterministic for finite inputs.
fn wrap_period(value: f64) -> f64 {
    value.rem_euclid(NOISE_PERIOD)
}

/// Gradient at the integer lattice point `(ix, iy, iz)` under `seed`.
///
/// Lattice indices wrap modulo 256, so the field has no seam; nearby
/// lattice points scatter to unrelated gradients.
fn lattice_gradient(seed: u64, ix: i64, iy: i64, iz: i64) -> [f64; 3] {
    let x = ix.rem_euclid(NOISE_LATTICE_PERIOD) as u64;
    let y = iy.rem_euclid(NOISE_LATTICE_PERIOD) as u64;
    let z = iz.rem_euclid(NOISE_LATTICE_PERIOD) as u64;
    let mut hash = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    hash = mix_noise(hash ^ x.wrapping_mul(0xBF58_476D_1CE4_E5B9));
    hash = mix_noise(hash ^ y.wrapping_mul(0x94D0_49BB_1331_11EB));
    hash = mix_noise(hash ^ z.wrapping_mul(0xBF58_476D_1CE4_E5B9));
    hash = mix_noise(hash ^ 0x94D0_49BB_1331_11EB);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: hash modulo 12 always fits usize"
    )]
    let slot = (hash % GRADIENTS.len() as u64) as usize;
    GRADIENTS.get(slot).copied().unwrap_or(GRADIENTS[0])
}

/// Plane of one lattice corner at the fractional offsets: the gradient
/// dot the offset, plus the gradient itself as the corner's slope.
fn corner_plane(
    seed: u64,
    ix: i64,
    iy: i64,
    iz: i64,
    ox: f64,
    oy: f64,
    oz: f64,
) -> (f64, [f64; 3]) {
    let g = lattice_gradient(seed, ix, iy, iz);
    (g[0] * ox + g[1] * oy + g[2] * oz, g)
}

/// Smootherstep fade and its derivative at `t` in `[0.0, 1.0]`.
fn fade_pair(t: f64) -> (f64, f64) {
    let fade = t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let slope = 30.0 * t * t * (t * (t - 2.0) + 1.0);
    (fade, slope)
}

/// Linear interpolation between `a` and `b` at `t`.
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Samples single-octave gradient noise and its analytic gradient.
///
/// Returns `(value, derivative)`: the signed field (about `[-1.0, 1.0]`)
/// and its slope per unit of `p`. Trilinear interpolation of the eight
/// surrounding lattice corners with smootherstep blending, differentiated
/// exactly. A non-finite coordinate yields neutral `(0.0, zero)` instead
/// of propagating `NaN`.
#[must_use]
pub fn gradient_3d(seed: u64, p: [f64; 3]) -> (f64, [f64; 3]) {
    let [x, y, z] = p;
    if !x.is_finite() || !y.is_finite() || !z.is_finite() {
        return (0.0, [0.0; 3]);
    }
    let (wx, wy, wz) = (wrap_period(x), wrap_period(y), wrap_period(z));
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: wrapped into [0, 256), always fits i64"
    )]
    let xi = wx.floor() as i64;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: wrapped into [0, 256), always fits i64"
    )]
    let yi = wy.floor() as i64;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: wrapped into [0, 256), always fits i64"
    )]
    let zi = wz.floor() as i64;
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: lattice coordinates below 256, exactly representable"
    )]
    let (u, du) = fade_pair(wx - xi as f64);
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: lattice coordinates below 256, exactly representable"
    )]
    let (v, dv) = fade_pair(wy - yi as f64);
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: lattice coordinates below 256, exactly representable"
    )]
    let (w, dw) = fade_pair(wz - zi as f64);
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: fractions below 256, exactly representable"
    )]
    let fractions = [wx - xi as f64, wy - yi as f64, wz - zi as f64];
    let [fx, fy, fz] = fractions;
    // Eight corners: plane value plus gradient, offsets from the fractions.
    let (c000, g000) = corner_plane(seed, xi, yi, zi, fx, fy, fz);
    let (c100, g100) = corner_plane(seed, xi + 1, yi, zi, fx - 1.0, fy, fz);
    let (c010, g010) = corner_plane(seed, xi, yi + 1, zi, fx, fy - 1.0, fz);
    let (c110, g110) = corner_plane(seed, xi + 1, yi + 1, zi, fx - 1.0, fy - 1.0, fz);
    let (c001, g001) = corner_plane(seed, xi, yi, zi + 1, fx, fy, fz - 1.0);
    let (c101, g101) = corner_plane(seed, xi + 1, yi, zi + 1, fx - 1.0, fy, fz - 1.0);
    let (c011, g011) = corner_plane(seed, xi, yi + 1, zi + 1, fx, fy - 1.0, fz - 1.0);
    let (c111, g111) = corner_plane(seed, xi + 1, yi + 1, zi + 1, fx - 1.0, fy - 1.0, fz - 1.0);
    // Trilinear blends along x, then y, then z; each level's fade slope
    // lands on its own axis only.
    let x00 = lerp(c000, c100, u);
    let x00x = lerp(g000[0], g100[0], u) + du * (c100 - c000);
    let x00y = lerp(g000[1], g100[1], u);
    let x00z = lerp(g000[2], g100[2], u);
    let x10 = lerp(c010, c110, u);
    let x10x = lerp(g010[0], g110[0], u) + du * (c110 - c010);
    let x10y = lerp(g010[1], g110[1], u);
    let x10z = lerp(g010[2], g110[2], u);
    let x01 = lerp(c001, c101, u);
    let x01x = lerp(g001[0], g101[0], u) + du * (c101 - c001);
    let x01y = lerp(g001[1], g101[1], u);
    let x01z = lerp(g001[2], g101[2], u);
    let x11 = lerp(c011, c111, u);
    let x11x = lerp(g011[0], g111[0], u) + du * (c111 - c011);
    let x11y = lerp(g011[1], g111[1], u);
    let x11z = lerp(g011[2], g111[2], u);
    let y0 = lerp(x00, x10, v);
    let y0x = lerp(x00x, x10x, v);
    let y0y = lerp(x00y, x10y, v) + dv * (x10 - x00);
    let y0z = lerp(x00z, x10z, v);
    let y1 = lerp(x01, x11, v);
    let y1x = lerp(x01x, x11x, v);
    let y1y = lerp(x01y, x11y, v) + dv * (x11 - x01);
    let y1z = lerp(x01z, x11z, v);
    let value = lerp(y0, y1, w);
    let dx = lerp(y0x, y1x, w);
    let dy = lerp(y0y, y1y, w);
    let dz = lerp(y0z, y1z, w) + dw * (y1 - y0);
    (value, [dx, dy, dz])
}

/// Smoothstep from `edge0` to `edge1` at `x`: `0.0` below, `1.0` above, a
/// smooth S between. Coincident edges read `0.0` below and `1.0` at and
/// above, without ever dividing by zero.
#[must_use]
pub fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    if !edge0.is_finite() || !edge1.is_finite() || !x.is_finite() {
        return 0.0;
    }
    if edge1 <= edge0 {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Fractal sum of `octaves` gradient-noise layers at `p`.
///
/// Each layer doubles the frequency (`lacunarity`) and scales the
/// amplitude by `gain` (about 2 and 0.5 for relief). Returns `(value,
/// derivative)`: the summed field and its summed slope, the slope scaled
/// by each layer's frequency. A fixed schedule: the same inputs always
/// sum the same layers, so the height never depends on the split depth.
#[must_use]
pub fn fbm_3d(seed: u64, p: [f64; 3], octaves: u32, lacunarity: f64, gain: f64) -> (f64, [f64; 3]) {
    let mut value = 0.0;
    let mut derivative = [0.0; 3];
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    for _ in 0..octaves {
        let q = [p[0] * frequency, p[1] * frequency, p[2] * frequency];
        let (layer, slope) = gradient_3d(seed, q);
        let [sx, sy, sz] = slope;
        let [dx, dy, dz] = derivative;
        value += amplitude * layer;
        derivative = [
            dx + amplitude * frequency * sx,
            dy + amplitude * frequency * sy,
            dz + amplitude * frequency * sz,
        ];
        amplitude *= gain;
        frequency *= lacunarity;
    }
    (value, derivative)
}

/// Ridged multifractal at `p`: sharp crests for mountain belts.
///
/// Each layer takes `(1 - |noise|)` squared, so valleys sharpen into
/// ridges, and feeds its height into the next layer's weight, so ridges
/// grow where they already stand (Musgrave-style, offset 1, gain 0.5).
/// Returns a non-negative field of about `[0.0, 1.0]`.
#[must_use]
pub fn ridged_3d(seed: u64, p: [f64; 3], octaves: u32) -> f64 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    let mut weight = 1.0;
    for _ in 0..octaves {
        let q = [p[0] * frequency, p[1] * frequency, p[2] * frequency];
        let (layer, _) = gradient_3d(seed, q);
        let signal = 1.0 - layer.abs();
        let crest = signal * signal * weight;
        value += crest * amplitude;
        weight = (signal * 2.0).clamp(0.0, 1.0);
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_deterministic_and_seed_sensitive() {
        let (a, _) = gradient_3d(7, [0.3, -0.5, 0.8]);
        let (b, _) = gradient_3d(7, [0.3, -0.5, 0.8]);
        assert_eq!(a, b, "same seed and point, same value");
        let (c, _) = gradient_3d(8, [0.3, -0.5, 0.8]);
        assert_ne!(a, c, "another seed, another value");
        let (x, gx) = gradient_3d(7, [0.3, -0.5, 0.8]);
        let (y, gy) = gradient_3d(7, [0.3, -0.5, 0.8]);
        assert_eq!((x, gx), (y, gy), "derivatives replay too");
    }

    #[test]
    fn noise_stays_bounded_and_joins_without_seams() {
        let mut smallest = f64::INFINITY;
        let mut largest = f64::NEG_INFINITY;
        for i in 0..2_000 {
            let t = f64::from(i) * 0.618_033_988_749_895;
            let (v, _) = gradient_3d(11, [t, t * 1.7, -t * 0.3]);
            smallest = smallest.min(v);
            largest = largest.max(v);
        }
        assert!(
            smallest > -1.5 && largest < 1.5,
            "signed field: [{smallest}, {largest}]"
        );
        assert!(
            largest - smallest > 1.0,
            "the field varies: {largest} vs {smallest}"
        );
        // Across the wrap edge the field joins: sampling at 255.9 and at
        // 256.1 (which wraps to 0.1) lands on neighbouring lattice, so the
        // values stay close.
        let (before, _) = gradient_3d(11, [255.9, 3.0, 3.0]);
        let (after, _) = gradient_3d(11, [256.1, 3.0, 3.0]);
        assert!(
            (before - after).abs() < 0.3,
            "wrap seam jumps: {before} vs {after}"
        );
    }

    #[test]
    fn analytic_derivatives_match_finite_differences() {
        let step = 1e-5;
        for (seed, point) in [
            (7u64, [0.3, -0.5, 0.8]),
            (99, [3.1, 7.7, -2.2]),
            (1_000_000, [100.25, -40.5, 90.75]),
        ] {
            let (_, grad) = gradient_3d(seed, point);
            for axis in 0..3 {
                let mut plus = point;
                let mut minus = point;
                plus[axis] += step;
                minus[axis] -= step;
                let (hi, _) = gradient_3d(seed, plus);
                let (lo, _) = gradient_3d(seed, minus);
                let numeric = (hi - lo) / (2.0 * step);
                assert!(
                    (grad[axis] - numeric).abs() < 1e-6,
                    "axis {axis} at {point:?}: analytic {} vs numeric {numeric}",
                    grad[axis]
                );
            }
        }
        // fBm derivatives chain the same way.
        let (_, grad) = fbm_3d(7, [0.3, -0.5, 0.8], 4, 2.0, 0.5);
        for axis in 0..3 {
            let mut plus = [0.3, -0.5, 0.8];
            let mut minus = [0.3, -0.5, 0.8];
            plus[axis] += step;
            minus[axis] -= step;
            let (hi, _) = fbm_3d(7, plus, 4, 2.0, 0.5);
            let (lo, _) = fbm_3d(7, minus, 4, 2.0, 0.5);
            let numeric = (hi - lo) / (2.0 * step);
            assert!(
                (grad[axis] - numeric).abs() < 1e-4,
                "fbm axis {axis}: analytic {} vs numeric {numeric}",
                grad[axis]
            );
        }
    }

    #[test]
    fn fbm_and_ridged_stay_in_their_bands() {
        for seed in [1u64, 7, 42, 999] {
            let (v, g) = fbm_3d(seed, [0.3, 0.8, -0.2], 5, 2.0, 0.5);
            assert!(v.abs() < 2.0, "fbm value {v}");
            assert!(g.iter().all(|s| s.is_finite()), "fbm slopes stay finite");
            let r = ridged_3d(seed, [0.3, 0.8, -0.2], 4);
            assert!((0.0..=1.2).contains(&r), "ridged band: {r}");
        }
        assert_eq!(fbm_3d(7, [0.3, 0.8, -0.2], 0, 2.0, 0.5).0, 0.0);
        assert_eq!(ridged_3d(7, [0.3, 0.8, -0.2], 0), 0.0);
    }

    #[test]
    fn degenerate_input_stays_neutral() {
        assert_eq!(gradient_3d(7, [f64::NAN, 0.0, 0.0]), (0.0, [0.0; 3]));
        assert_eq!(gradient_3d(7, [f64::INFINITY, 0.0, 0.0]), (0.0, [0.0; 3]));
        assert_eq!(smoothstep(1.0, 1.0, 0.5), 0.0);
        assert_eq!(smoothstep(1.0, 1.0, 1.5), 1.0);
        assert_eq!(smoothstep(0.0, 1.0, f64::NAN), 0.0);
        assert!((smoothstep(0.0, 2.0, 1.0) - 0.5).abs() < 1e-12);
    }
}
