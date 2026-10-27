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
#[cfg(test)]
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

/// One octave's last lattice cell and its eight corner values (#994 T2).
///
/// Neighbouring points of a patch mostly fall in the same cell of every
/// coarse octave, so a build keeps one memo per octave and hashes the
/// corners again only when the cell changes. The corners are the same bits
/// either way, so the field is unchanged. A memo belongs to one octave
/// seed: reusing it under another seed would read wrong corners.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct CellMemo {
    /// The cell the corners belong to, `None` before the first point.
    cell: Option<[i64; 3]>,
    /// The corners in `c000, c100, c010, c110, c001, c101, c011, c111` order.
    corners: [f64; 8],
}

/// The eight corner values of the cell at `(xi, yi, zi)` (#994 T2).
///
/// The same values as eight [`lattice_value_aperiodic`] calls: the hash
/// folds in `x`, then `y`, then `z`, so corners sharing a prefix share its
/// mixing (2 + 4 + 8 + 8 mixes instead of 32).
fn corners_aperiodic(seed: u64, xi: i64, yi: i64, zi: i64) -> [f64; 8] {
    #[expect(
        clippy::cast_sign_loss,
        reason = "E-CAST: lattice indices reinterpreted as bit patterns in the hash mix"
    )]
    let bits = |i: i64| i as u64;
    let start = seed.wrapping_add(NOISE_GAMMA);
    let hx = [
        mix_noise(start ^ bits(xi).wrapping_mul(NOISE_MULT_A)),
        mix_noise(start ^ bits(xi + 1).wrapping_mul(NOISE_MULT_A)),
    ];
    let y_terms = [
        bits(yi).wrapping_mul(NOISE_MULT_B),
        bits(yi + 1).wrapping_mul(NOISE_MULT_B),
    ];
    let z_terms = [
        bits(zi).wrapping_mul(NOISE_MULT_A),
        bits(zi + 1).wrapping_mul(NOISE_MULT_A),
    ];
    let mut corners = [0.0f64; 8];
    for (dy, y_term) in y_terms.iter().enumerate() {
        for (dx, hx) in hx.iter().enumerate() {
            let hxy = mix_noise(hx ^ y_term);
            for (dz, z_term) in z_terms.iter().enumerate() {
                let hash = mix_noise(mix_noise(hxy ^ z_term) ^ NOISE_MULT_B);
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "E-CAST: top 53 bits are exactly representable in f64"
                )]
                let unit = ((hash >> 11) as f64) * INV_2_POW_53;
                if let Some(slot) = corners.get_mut(dz * 4 + dy * 2 + dx) {
                    *slot = unit;
                }
            }
        }
    }
    corners
}

/// `floor(x)` as an integer for `x` within `±(2^53 - 1)` (#994 T2).
///
/// Without SSE4.1 (no CPU-specific flags, `E-NO-TARGET-CPU`) `f64::floor`
/// is a library call; truncating and stepping down below zero gives the
/// same integer in this range, where every float with a fraction is far
/// inside `i64`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "E-CAST: |x| below 2^53, the integer and its float are exact"
)]
fn floor_i64(x: f64) -> i64 {
    let t = x as i64;
    if (t as f64) > x { t - 1 } else { t }
}

/// The lattice cell of `(x, y, z)` and the faded fractions inside it, or
/// `None` for a non-finite or out-of-range point (the neutral `0.5`).
///
/// For a finite `x`, `floor(x)` lies within `±(2^53 - 1)` exactly when `x`
/// does (beyond it every float is a whole number), so the range check on
/// the point is the one on its floor.
fn aperiodic_cell(x: f64, y: f64, z: f64) -> Option<([i64; 3], [f64; 3])> {
    const SAFE: f64 = 9_007_199_254_740_991.0;
    let in_range = |f: f64| f.is_finite() && (-SAFE..=SAFE).contains(&f);
    if !in_range(x) || !in_range(y) || !in_range(z) {
        return None;
    }
    let cell = [floor_i64(x), floor_i64(y), floor_i64(z)];
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: lattice float differences below 2^53, exact enough for fade"
    )]
    let fades = [
        fade(x - cell[0] as f64),
        fade(y - cell[1] as f64),
        fade(z - cell[2] as f64),
    ];
    Some((cell, fades))
}

/// Trilinear blend of the cell `corners` at the faded fractions `f`.
fn blend(c: &[f64; 8], f: [f64; 3]) -> f64 {
    lerp(
        lerp(lerp(c[0], c[1], f[0]), lerp(c[2], c[3], f[0]), f[1]),
        lerp(lerp(c[4], c[5], f[0]), lerp(c[6], c[7], f[0]), f[1]),
        f[2],
    )
}

/// Samples single-octave aperiodic value noise in [0.0, 1.0] (#799).
///
/// Same trilinear smootherstep field as value_noise_3d, but on the unwrapped
/// lattice (lattice_value_aperiodic), so high-frequency hill octaves far above
/// 256 per radius show no tiling. Non-finite coordinates yield the neutral 0.5.
/// Deterministic on every platform.
#[cfg(test)]
pub(crate) fn value_noise_3d_aperiodic(seed: u64, x: f64, y: f64, z: f64) -> f64 {
    aperiodic_cell(x, y, z).map_or(0.5, |(cell, fades)| {
        blend(&corners_aperiodic(seed, cell[0], cell[1], cell[2]), fades)
    })
}

/// [`value_noise_3d_aperiodic`] with the octave's `memo` (#994 T2): the
/// corners are hashed again only when the point leaves the memo's cell.
/// The same bits as without the memo.
pub(crate) fn value_noise_3d_aperiodic_memo(
    seed: u64,
    x: f64,
    y: f64,
    z: f64,
    memo: &mut CellMemo,
) -> f64 {
    let Some((cell, fades)) = aperiodic_cell(x, y, z) else {
        return 0.5;
    };
    if memo.cell != Some(cell) {
        memo.corners = corners_aperiodic(seed, cell[0], cell[1], cell[2]);
        memo.cell = Some(cell);
    }
    blend(&memo.corners, fades)
}

/// Two fields at one point (#994 T2): [`value_noise_3d_aperiodic_memo`]
/// under `seeds.0` and `seeds.1`, finding the cell and its fractions once.
pub(crate) fn value_noise_3d_aperiodic_memo_pair(
    seeds: (u64, u64),
    x: f64,
    y: f64,
    z: f64,
    memos: (&mut CellMemo, &mut CellMemo),
) -> (f64, f64) {
    let Some((cell, fades)) = aperiodic_cell(x, y, z) else {
        return (0.5, 0.5);
    };
    let one = |seed: u64, memo: &mut CellMemo| {
        if memo.cell != Some(cell) {
            memo.corners = corners_aperiodic(seed, cell[0], cell[1], cell[2]);
            memo.cell = Some(cell);
        }
        blend(&memo.corners, fades)
    };
    (one(seeds.0, memos.0), one(seeds.1, memos.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_prefix_corners_are_the_eight_lattice_values() {
        // #994 T2: the same bits as hashing every corner on its own.
        for (seed, xi, yi, zi) in [
            (7u64, 0i64, 0i64, 0i64),
            (0x1234_5678, -3, 17, -250_000),
            (u64::MAX, i64::from(i32::MAX), -1, 42),
        ] {
            let corners = corners_aperiodic(seed, xi, yi, zi);
            let mut k = 0;
            for dz in 0..2 {
                for dy in 0..2 {
                    for dx in 0..2 {
                        let want = lattice_value_aperiodic(seed, xi + dx, yi + dy, zi + dz);
                        assert_eq!(corners[k].to_bits(), want.to_bits(), "corner {k}");
                        k += 1;
                    }
                }
            }
        }
    }

    #[test]
    fn the_integer_floor_matches_the_float_floor() {
        for x in [
            0.0,
            -0.0,
            0.5,
            -0.5,
            -1.0,
            1.0 - f64::EPSILON,
            -1.0e-300,
            123_456.789,
            -123_456.789,
            4_503_599_627_370_495.5,
            -4_503_599_627_370_495.5,
            9_007_199_254_740_991.0,
            -9_007_199_254_740_991.0,
        ] {
            #[expect(clippy::cast_possible_truncation, reason = "test values in range")]
            let want = x.floor() as i64;
            assert_eq!(floor_i64(x), want, "{x}");
        }
    }

    #[test]
    fn the_memo_gives_the_same_bits() {
        let mut memo = CellMemo::default();
        let seed = 0x0994_u64;
        for step in 0..2_000u32 {
            let t = f64::from(step) * 0.013;
            let (x, y, z) = (t * 3.1 - 9.0, (t * 0.7).sin() * 4.0, 1.0e6 + t);
            let plain = value_noise_3d_aperiodic(seed, x, y, z);
            let memoised = value_noise_3d_aperiodic_memo(seed, x, y, z, &mut memo);
            assert_eq!(plain.to_bits(), memoised.to_bits(), "step {step}");
        }
        let (mut pa, mut pb) = (CellMemo::default(), CellMemo::default());
        for step in 0..500u32 {
            let x = f64::from(step) * 0.031 - 3.0;
            let (a, b) =
                value_noise_3d_aperiodic_memo_pair((1, 2), x, -x, 0.25, (&mut pa, &mut pb));
            assert_eq!(
                a.to_bits(),
                value_noise_3d_aperiodic(1, x, -x, 0.25).to_bits()
            );
            assert_eq!(
                b.to_bits(),
                value_noise_3d_aperiodic(2, x, -x, 0.25).to_bits()
            );
        }
        let bad = value_noise_3d_aperiodic_memo(seed, f64::NAN, 0.0, 0.0, &mut memo);
        assert_eq!(bad.to_bits(), 0.5f64.to_bits());
    }
}
