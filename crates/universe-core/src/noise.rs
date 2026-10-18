//! Shared deterministic value-noise primitive (crate-internal).
//!
//! Levels L1-L3 ([`crate::density`]) and L10 ([`crate::terrain`]) both need
//! continuous hash-based fields with no lookup tables; this single
//! implementation keeps the two levels from drifting apart. Wrapping `u64`
//! arithmetic plus float interpolation only, so results are identical on
//! every platform. The field is periodic (see [`NOISE_PERIOD`]), hence
//! seamless everywhere including far from the origin.

/// Splitmix64 golden-ratio increment.
const NOISE_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// First avalanche multiplier.
const NOISE_MULT_A: u64 = 0xBF58_476D_1CE4_E5B9;

/// Second avalanche multiplier.
const NOISE_MULT_B: u64 = 0x94D0_49BB_1331_11EB;

/// Exact scale mapping the top 53 hash bits onto `[0.0, 1.0)`.
const INV_2_POW_53: f64 = 1.0 / 9_007_199_254_740_992.0;

/// Lattice period in field units: the field repeats every 256 units per axis.
///
/// Wrapping both the float coordinates and the integer lattice indices keeps
/// every lattice access in a tiny range for every finite input, so the
/// `[0.0, 1.0]` output guarantee below holds without clamping, and the wrap
/// seam vanishes because opposite period edges hash identically.
const NOISE_PERIOD: f64 = 256.0;

/// Lattice sub-period matching [`NOISE_PERIOD`] in integer steps.
const NOISE_LATTICE_PERIOD: i64 = 256;

/// Splitmix64-style avalanche finalizer (mirroring the one in `seed`).
const fn mix_noise(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(NOISE_MULT_A);
    z = (z ^ (z >> 27)).wrapping_mul(NOISE_MULT_B);
    z ^ (z >> 31)
}

/// Wraps a coordinate into `[0.0, 256.0)`; deterministic for finite inputs.
fn wrap_period(value: f64) -> f64 {
    value.rem_euclid(NOISE_PERIOD)
}

/// Hashes the integer lattice point `(ix, iy, iz)` under `seed`.
///
/// The output covers `[0.0, 1.0)` uniformly: nearby lattice points scatter
/// to unrelated values, which is what makes the interpolated field look
/// organic. Lattice indices wrap modulo 256, so the field has no seam.
pub(crate) fn lattice_value(seed: u64, ix: i64, iy: i64, iz: i64) -> f64 {
    let wrapped = [
        ix.rem_euclid(NOISE_LATTICE_PERIOD) as u64,
        iy.rem_euclid(NOISE_LATTICE_PERIOD) as u64,
        iz.rem_euclid(NOISE_LATTICE_PERIOD) as u64,
    ];
    let mut hash = seed.wrapping_add(NOISE_GAMMA);
    hash = mix_noise(hash ^ wrapped[0].wrapping_mul(NOISE_MULT_A));
    hash = mix_noise(hash ^ wrapped[1].wrapping_mul(NOISE_MULT_B));
    hash = mix_noise(hash ^ wrapped[2].wrapping_mul(NOISE_MULT_A));
    hash = mix_noise(hash ^ NOISE_MULT_B);
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: top 53 bits are exactly representable in f64"
    )]
    let unit = ((hash >> 11) as f64) * INV_2_POW_53;
    unit
}

/// Hashes the integer lattice point `(ix, iy, iz)` under `seed` without
/// wrapping, for high-frequency relief octaves (#799).
///
/// Same `mix_noise` hash as [`lattice_value`], but on the raw `i64` indices
/// reinterpreted as `u64` bit patterns, so the field never repeats and shows
/// no tiling at frequencies far above 256 per radius. Deterministic on every
/// platform (`E-DET-TIERS`).
pub(crate) fn lattice_value_aperiodic(seed: u64, ix: i64, iy: i64, iz: i64) -> f64 {
    #[expect(
        clippy::cast_sign_loss,
        reason = "E-CAST: lattice indices reinterpreted as bit patterns in the hash mix"
    )]
    let raw = [(ix as u64), (iy as u64), (iz as u64)];
    let mut hash = seed.wrapping_add(NOISE_GAMMA);
    hash = mix_noise(hash ^ raw[0].wrapping_mul(NOISE_MULT_A));
    hash = mix_noise(hash ^ raw[1].wrapping_mul(NOISE_MULT_B));
    hash = mix_noise(hash ^ raw[2].wrapping_mul(NOISE_MULT_A));
    hash = mix_noise(hash ^ NOISE_MULT_B);
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: top 53 bits are exactly representable in f64"
    )]
    let unit = ((hash >> 11) as f64) * INV_2_POW_53;
    unit
}

/// Smootherstep fade mapping [0.0, 1.0] onto itself with zero end slopes.
///
/// Zero end slopes make neighbouring trilinear patches join with matching
/// derivatives, which is what keeps the field `C1`-continuous everywhere.
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Linear interpolation between `a` and `b` at `t` in `[0.0, 1.0]`.
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Samples single-octave value noise in `[0.0, 1.0]`.
///
/// Trilinear interpolation of the eight surrounding lattice corners with
/// smootherstep blending. Non-finite coordinates yield the neutral `0.5`
/// instead of propagating `NaN`.
pub(crate) fn value_noise_3d(seed: u64, x: f64, y: f64, z: f64) -> f64 {
    if !x.is_finite() || !y.is_finite() || !z.is_finite() {
        return 0.5;
    }
    let wrapped_x = wrap_period(x);
    let wrapped_y = wrap_period(y);
    let wrapped_z = wrap_period(z);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: wrapped into [0, 256), always fits i64"
    )]
    let xi = wrapped_x.floor() as i64;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: wrapped into [0, 256), always fits i64"
    )]
    let yi = wrapped_y.floor() as i64;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: wrapped into [0, 256), always fits i64"
    )]
    let zi = wrapped_z.floor() as i64;
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: lattice coordinates below 256, exactly representable"
    )]
    let xf = fade(wrapped_x - xi as f64);
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: lattice coordinates below 256, exactly representable"
    )]
    let yf = fade(wrapped_y - yi as f64);
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: lattice coordinates below 256, exactly representable"
    )]
    let zf = fade(wrapped_z - zi as f64);
    let c000 = lattice_value(seed, xi, yi, zi);
    let c100 = lattice_value(seed, xi + 1, yi, zi);
    let c010 = lattice_value(seed, xi, yi + 1, zi);
    let c110 = lattice_value(seed, xi + 1, yi + 1, zi);
    let c001 = lattice_value(seed, xi, yi, zi + 1);
    let c101 = lattice_value(seed, xi + 1, yi, zi + 1);
    let c011 = lattice_value(seed, xi, yi + 1, zi + 1);
    let c111 = lattice_value(seed, xi + 1, yi + 1, zi + 1);
    lerp(
        lerp(lerp(c000, c100, xf), lerp(c010, c110, xf), yf),
        lerp(lerp(c001, c101, xf), lerp(c011, c111, xf), yf),
        zf,
    )
}

/// Samples single-octave aperiodic value noise in [0.0, 1.0] (#799).
///
/// Same trilinear smootherstep field as value_noise_3d, but on the unwrapped
/// lattice (lattice_value_aperiodic), so high-frequency hill octaves far above
/// 256 per radius show no tiling. Non-finite coordinates yield the neutral 0.5.
/// Deterministic on every platform.
pub(crate) fn value_noise_3d_aperiodic(seed: u64, x: f64, y: f64, z: f64) -> f64 {
    if !x.is_finite() || !y.is_finite() || !z.is_finite() {
        return 0.5;
    }
    let xif = x.floor();
    let yif = y.floor();
    let zif = z.floor();
    if !xif.is_finite() || !yif.is_finite() || !zif.is_finite() {
        return 0.5;
    }
    if !(-9007199254740991.0..=9007199254740991.0).contains(&xif) {
        return 0.5;
    }
    if !(-9007199254740991.0..=9007199254740991.0).contains(&yif) {
        return 0.5;
    }
    if !(-9007199254740991.0..=9007199254740991.0).contains(&zif) {
        return 0.5;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: floor clamped to 2^53, exactly representable"
    )]
    let xi = xif as i64;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: floor clamped to 2^53, exactly representable"
    )]
    let yi = yif as i64;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: floor clamped to 2^53, exactly representable"
    )]
    let zi = zif as i64;
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: lattice float differences below 2^53, exact enough for fade"
    )]
    let xf = fade(x - xi as f64);
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: lattice float differences below 2^53, exact enough for fade"
    )]
    let yf = fade(y - yi as f64);
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: lattice float differences below 2^53, exact enough for fade"
    )]
    let zf = fade(z - zi as f64);
    let c000 = lattice_value_aperiodic(seed, xi, yi, zi);
    let c100 = lattice_value_aperiodic(seed, xi + 1, yi, zi);
    let c010 = lattice_value_aperiodic(seed, xi, yi + 1, zi);
    let c110 = lattice_value_aperiodic(seed, xi + 1, yi + 1, zi);
    let c001 = lattice_value_aperiodic(seed, xi, yi, zi + 1);
    let c101 = lattice_value_aperiodic(seed, xi + 1, yi, zi + 1);
    let c011 = lattice_value_aperiodic(seed, xi, yi + 1, zi + 1);
    let c111 = lattice_value_aperiodic(seed, xi + 1, yi + 1, zi + 1);
    lerp(
        lerp(lerp(c000, c100, xf), lerp(c010, c110, xf), yf),
        lerp(lerp(c001, c101, xf), lerp(c011, c111, xf), yf),
        zf,
    )
}
