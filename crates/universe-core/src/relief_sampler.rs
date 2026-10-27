//! [`HeightSampler`]: many height reads of one relief in a row (#994 T2).
//!
//! Split from `relief` (`E-FILE-SIZE`). The fBm and ridged sums read each
//! octave through a [`CellMemo`], so neighbouring directions hash a noise
//! cell once; every height keeps the bits of the maths before #994.

use super::{
    AMP_HILLS_M, AMP_RANGES_M, BELT_FREQ, BELT_OCTAVES, CONTINENT_FREQ, CONTINENT_OCTAVES,
    CONTINENT_THRESHOLD, HILLS_FREQ, HILLS_OCTAVES, LAND_MAX_M, LAND_MIN_M, OCEAN_FLOOR_M,
    RANGES_FREQ, RANGES_OCTAVES, Relief, WARP_FREQ, WARP_OCTAVES, WARP_STRENGTH, length,
    smoothstep,
};
use crate::noise::{CellMemo, value_noise_3d_aperiodic_memo, value_noise_3d_aperiodic_memo_pair};

/// fBm in `[0.0, 1.0]` over `octaves` of aperiodic value noise at `p`.
///
/// Each octave uses its own seed from `oct_seeds`; frequencies double from
/// `base_freq`, amplitudes halve, normalized by the amplitude sum.
/// `memos` holds one cell memo per octave (#994 T2).
pub(super) fn fbm_with_seeds(
    p: [f64; 3],
    base_freq: f64,
    oct_seeds: &[u64],
    memos: &mut [CellMemo],
) -> f64 {
    if oct_seeds.is_empty() {
        return 0.5;
    }
    let mut sum = 0.0f64;
    let mut norm = 0.0f64;
    let mut freq = base_freq;
    let mut amp = 1.0f64;
    for (seed, memo) in oct_seeds.iter().zip(memos.iter_mut()) {
        let n = value_noise_3d_aperiodic_memo(*seed, p[0] * freq, p[1] * freq, p[2] * freq, memo);
        sum += amp * n;
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

/// [`fbm_with_seeds`] of two fields at one point (the warp offsets, #994
/// T2): the same two values, each octave's cell found once for both.
fn fbm_pair(
    p: [f64; 3],
    base_freq: f64,
    seeds: (&[u64; WARP_OCTAVES], &[u64; WARP_OCTAVES]),
    memos: (&mut [CellMemo; WARP_OCTAVES], &mut [CellMemo; WARP_OCTAVES]),
) -> (f64, f64) {
    let (mut sum_a, mut sum_b) = (0.0f64, 0.0f64);
    let mut norm = 0.0f64;
    let mut freq = base_freq;
    let mut amp = 1.0f64;
    for k in 0..WARP_OCTAVES {
        let (Some(sa), Some(sb), Some(ma)) = (seeds.0.get(k), seeds.1.get(k), memos.0.get_mut(k))
        else {
            continue;
        };
        let Some(mb) = memos.1.get_mut(k) else {
            continue;
        };
        let (a, b) = value_noise_3d_aperiodic_memo_pair(
            (*sa, *sb),
            p[0] * freq,
            p[1] * freq,
            p[2] * freq,
            (ma, mb),
        );
        sum_a += amp * a;
        sum_b += amp * b;
        norm += amp;
        freq *= 2.0;
        amp *= 0.5;
    }
    let done = |sum: f64| {
        if norm > 0.0 && sum.is_finite() {
            sum / norm
        } else {
            0.5
        }
    };
    (done(sum_a), done(sum_b))
}

/// Ridged multifractal in `[0.0, 1.0]` (sharp crests for ranges, Q1 A).
///
/// Each octave takes `1 - |2n - 1|`, squares it, and feeds it into the next
/// octave's weight, so fine detail concentrates on the crests.
/// `memos` holds one cell memo per octave (#994 T2).
fn ridged_with_seeds(
    p: [f64; 3],
    base_freq: f64,
    oct_seeds: &[u64],
    memos: &mut [CellMemo],
) -> f64 {
    if oct_seeds.is_empty() {
        return 0.5;
    }
    let mut sum = 0.0f64;
    let mut norm = 0.0f64;
    let mut freq = base_freq;
    let mut amp = 1.0f64;
    let mut weight = 1.0f64;
    for (seed, memo) in oct_seeds.iter().zip(memos.iter_mut()) {
        let n = value_noise_3d_aperiodic_memo(*seed, p[0] * freq, p[1] * freq, p[2] * freq, memo);
        let centred = 2.0 * n - 1.0;
        let signal = 1.0 - centred.abs();
        let shaped = signal * signal * weight;
        sum += amp * shaped;
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

/// Reads many heights of one relief in a row (#994 T2).
///
/// Keeps each octave's last noise cell and its corners, so neighbouring
/// directions (a patch's grid points) hash a cell once instead of once per
/// point. Every height has the same bits as [`Relief::height_m`]. One
/// sampler per build: it holds no allocation (`E-HOT-NOALLOC`).
#[derive(Debug, Clone)]
pub struct HeightSampler<'a> {
    /// The relief read.
    relief: &'a Relief,
    /// Cell memos of the first warp offset's octaves.
    warp_a: [CellMemo; WARP_OCTAVES],
    /// Cell memos of the second warp offset's octaves.
    warp_b: [CellMemo; WARP_OCTAVES],
    /// Cell memos of the continent octaves.
    continent: [CellMemo; CONTINENT_OCTAVES],
    /// Cell memos of the hills octaves.
    hills: [CellMemo; HILLS_OCTAVES],
    /// Cell memos of the ranges octaves.
    ranges: [CellMemo; RANGES_OCTAVES],
    /// Cell memos of the belt mask octaves.
    belt: [CellMemo; BELT_OCTAVES],
}

impl<'a> HeightSampler<'a> {
    /// A sampler of `relief` with empty memos.
    #[must_use]
    pub fn new(relief: &'a Relief) -> Self {
        Self {
            relief,
            warp_a: [CellMemo::default(); WARP_OCTAVES],
            warp_b: [CellMemo::default(); WARP_OCTAVES],
            continent: [CellMemo::default(); CONTINENT_OCTAVES],
            hills: [CellMemo::default(); HILLS_OCTAVES],
            ranges: [CellMemo::default(); RANGES_OCTAVES],
            belt: [CellMemo::default(); BELT_OCTAVES],
        }
    }

    /// The relief this sampler reads.
    #[must_use]
    pub fn relief(&self) -> &'a Relief {
        self.relief
    }

    /// [`Relief::height_m`] at `dir`, through the memos.
    pub fn height_m(&mut self, dir: [f64; 3]) -> f64 {
        let relief = self.relief;
        if relief.is_giant {
            return 0.0;
        }
        if dir.iter().any(|c| !c.is_finite()) {
            return 0.0;
        }
        let len = length(dir);
        if !len.is_finite() || len <= 0.0 {
            return 0.0;
        }
        let unit = [dir[0] / len, dir[1] / len, dir[2] / len];
        // Domain warping by two low-frequency offsets (Q1 A).
        let (wa, wb) = fbm_pair(
            [
                unit[0] * WARP_FREQ,
                unit[1] * WARP_FREQ,
                unit[2] * WARP_FREQ,
            ],
            1.0,
            (&relief.warp_a_oct, &relief.warp_b_oct),
            (&mut self.warp_a, &mut self.warp_b),
        );
        let warp_vec = [
            (wa - 0.5) * WARP_STRENGTH * 2.0,
            (wb - 0.5) * WARP_STRENGTH * 2.0,
            ((wa + wb) / 2.0 - 0.5) * WARP_STRENGTH * 2.0,
        ];
        let warped_in = [
            unit[0] + warp_vec[0],
            unit[1] + warp_vec[1],
            unit[2] + warp_vec[2],
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
        // Continent mask with sea offset: land +0.5..+1 km, ocean -4.3 km.
        let continent = fbm_with_seeds(
            warped,
            CONTINENT_FREQ,
            &relief.continent_oct,
            &mut self.continent,
        );
        let land_t = smoothstep(
            CONTINENT_THRESHOLD - 0.02,
            CONTINENT_THRESHOLD + 0.02,
            continent,
        );
        let land_h = if continent > CONTINENT_THRESHOLD {
            let t =
                ((continent - CONTINENT_THRESHOLD) / (1.0 - CONTINENT_THRESHOLD)).clamp(0.0, 1.0);
            LAND_MIN_M + (LAND_MAX_M - LAND_MIN_M) * t
        } else {
            LAND_MIN_M
        };
        let sea_offset = (1.0 - land_t) * OCEAN_FLOOR_M + land_t * land_h;
        // Hills: hybrid-style fBm, calmer in the oceans so the deep mode stays.
        let hills_f = fbm_with_seeds(warped, HILLS_FREQ, &relief.hills_oct, &mut self.hills);
        let hills_bipolar = (hills_f - 0.5) * 2.0;
        let hills_scale = 0.3 + 0.7 * land_t;
        let hills = hills_bipolar * AMP_HILLS_M * hills_scale;
        // Belt mask (unwarped, Q1 A) times ridged ranges (warped): mountains
        // on land, trenches in the oceans, both as chains.
        let belt_f = fbm_with_seeds(unit, BELT_FREQ, &relief.belt_oct, &mut self.belt);
        let belt = smoothstep(0.55, 0.75, belt_f);
        let signed = belt * (2.0 * land_t - 1.0);
        let ridged = ridged_with_seeds(warped, RANGES_FREQ, &relief.ranges_oct, &mut self.ranges);
        let ranges = signed * ridged * AMP_RANGES_M;
        let total = sea_offset + hills + ranges;
        if !total.is_finite() {
            return 0.0;
        }
        total * relief.relief_scale
    }
}
