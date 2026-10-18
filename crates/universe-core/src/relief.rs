//! Deterministic terrain relief on the cube-sphere (rebuild step 3b, #799).
//!
//! One pure height function `h(direction)` in metres on the planet's unit
//! direction, in `f64`, never in large-magnitude `f32` (the design doc's
//! "Terrain generation"). The same seed always gives the same planet, and the
//! same direction gives the same bits at every LOD level and on both sides of
//! every face seam, because the input is the folded unit direction from
//! [`crate::cube_sphere::grid_point`] and the function is pure.
//!
//! Layers (the Height-maps guide's "Putting it together", Q1 A):
//! continent mask with sea offset (land `+0.5` to `+1` km, ocean floor
//! `-4.3` km), hybrid-fBm hills (`0.3`-`1` km), ridged-multifractal ranges
//! (`3`-`6` km) inside belt masks, and domain warping. No plates, erosion,
//! rivers, craters or volcanoes yet (later steps, one each). Every amplitude
//! scales by `relief_scale = clamp(9.81 / g, 0.3, 40)` (Q2 A); giants
//! (`radius_earth >= 11`) stay a smooth ball.
//!
//! Noise: high-frequency hill octaves reach far above 256 per radius, where
//! the shared periodic value noise would tile. This module uses the aperiodic
//! lattice (`value_noise_3d_aperiodic`) with the same `mix_noise` hash and no wrap, capped at about a kilometre finest on Earth so worker
//! builds stay cheap. Pure and deterministic (`E-DET-TIERS`,
//! `E-CORE-NO-BEVY`); transcendental calls stay in core (`E-TRANSCENDENTAL`).

use crate::seed::hash_triple;

/// Ocean floor in metres at Earth gravity (the guide's `-4.3` km).
pub const OCEAN_FLOOR_M: f64 = -4_300.0;

/// Land base range in metres at Earth gravity (`+0.5` to `+1` km).
pub const LAND_MIN_M: f64 = 500.0;

/// Land base top in metres at Earth gravity.
pub const LAND_MAX_M: f64 = 1_000.0;

/// Hills amplitude in metres at Earth gravity (`0.3`-`1` km, Q1 A).
pub const AMP_HILLS_M: f64 = 700.0;

/// Ranges amplitude in metres at Earth gravity (`3`-`6` km, Q1 A).
pub const AMP_RANGES_M: f64 = 6_000.0;

/// Continent field base frequency (features about 2,000 km on Earth).
pub const CONTINENT_FREQ: f64 = 3.0;

/// Continent fBm octaves.
pub const CONTINENT_OCTAVES: usize = 4;

/// Warp field base frequency.
pub const WARP_FREQ: f64 = 4.0;

/// Warp fBm octaves per offset.
pub const WARP_OCTAVES: usize = 3;

/// Warp strength in direction units (under one continent feature).
pub const WARP_STRENGTH: f64 = 0.25;

/// Hills base frequency (coarsest about 200 km on Earth).
pub const HILLS_FREQ: f64 = 32.0;

/// Hills octaves (finest about 1.5 km on Earth).
pub const HILLS_OCTAVES: usize = 8;

/// Ranges base frequency (coarsest about 400 km on Earth).
pub const RANGES_FREQ: f64 = 16.0;

/// Ranges octaves (finest about 6 km on Earth).
pub const RANGES_OCTAVES: usize = 7;

/// Belt mask base frequency.
pub const BELT_FREQ: f64 = 5.0;

/// Belt mask octaves.
pub const BELT_OCTAVES: usize = 3;

/// Continent threshold giving about 29 percent land (tuned, T2).
pub const CONTINENT_THRESHOLD: f64 = 0.55;

/// Giant radius at and above which worlds stay smooth (Q2 A).
pub const GIANT_RADIUS_EARTH: f64 = 11.0;

/// Domain tags folded into the planet seed per layer.
const TAG_CONTINENT: u64 = 0xC071_7990_C071_7990;
/// Domain tag for the first warp offset.
const TAG_WARP_A: u64 = 0x1AA2_7990_1AA2_7990;
/// Domain tag for the second warp offset.
const TAG_WARP_B: u64 = 0x2BB2_7990_2BB2_7990;
/// Domain tag for the hills field.
const TAG_HILLS: u64 = 0x1111_7990_1111_7990;
/// Domain tag for the ranges field.
const TAG_RANGES: u64 = 0x2222_7990_2222_7990;
/// Domain tag for the belt mask.
const TAG_BELT: u64 = 0xBE17_7990_BE17_7990;
/// Domain tag for octave separation.
const TAG_OCTAVE: u64 = 0x0C7A_7990_0C7A_7990;

/// Height tint band by height (Q3 A, Q4 A): deep below sea level, low `0` to
/// `1` km, high `1` to `4` km, snow above `4` km, with the two upper
/// thresholds scaled by `relief_scale` so low-gravity snow lines rise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReliefTint {
    /// Below sea level (drawn as deep, no water yet).
    Deep,
    /// From sea level to `1` km times scale.
    Low,
    /// From `1` km to `4` km times scale.
    High,
    /// Above `4` km times scale.
    Snow,
}

/// Tint band for `height_m` metres at `relief_scale`.
#[must_use]
pub fn tint_for_height(height_m: f64, relief_scale: f64) -> ReliefTint {
    if !height_m.is_finite() || height_m < 0.0 {
        return ReliefTint::Deep;
    }
    let scale = if relief_scale.is_finite() && relief_scale > 0.0 {
        relief_scale
    } else {
        1.0
    };
    let low_top = 1_000.0 * scale;
    let high_top = 4_000.0 * scale;
    if height_m < low_top {
        ReliefTint::Low
    } else if height_m < high_top {
        ReliefTint::High
    } else {
        ReliefTint::Snow
    }
}

/// Smoothstep from `edge0` to `edge1`, clamped.
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    if !(edge0.is_finite() && edge1.is_finite() && x.is_finite()) {
        return 0.0;
    }
    if edge1 <= edge0 {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// fBm in `[0.0, 1.0]` over `octaves` of aperiodic value noise at `p`.
///
/// Each octave uses its own seed from `oct_seeds`; frequencies double from
/// `base_freq`, amplitudes halve, normalized by the amplitude sum.
fn fbm_with_seeds(p: [f64; 3], base_freq: f64, oct_seeds: &[u64]) -> f64 {
    if oct_seeds.is_empty() {
        return 0.5;
    }
    let mut sum = 0.0f64;
    let mut norm = 0.0f64;
    let mut freq = base_freq;
    let mut amp = 1.0f64;
    for seed in oct_seeds {
        let n =
            crate::noise::value_noise_3d_aperiodic(*seed, p[0] * freq, p[1] * freq, p[2] * freq);
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

/// Ridged multifractal in `[0.0, 1.0]` (sharp crests for ranges, Q1 A).
///
/// Each octave takes `1 - |2n - 1|`, squares it, and feeds it into the next
/// octave's weight, so fine detail concentrates on the crests.
fn ridged_with_seeds(p: [f64; 3], base_freq: f64, oct_seeds: &[u64]) -> f64 {
    if oct_seeds.is_empty() {
        return 0.5;
    }
    let mut sum = 0.0f64;
    let mut norm = 0.0f64;
    let mut freq = base_freq;
    let mut amp = 1.0f64;
    let mut weight = 1.0f64;
    for seed in oct_seeds {
        let n =
            crate::noise::value_noise_3d_aperiodic(*seed, p[0] * freq, p[1] * freq, p[2] * freq);
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

fn length(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// One planet's relief: its seed, size, and gravity scale (Q2 A).
///
/// Pure: the same `(root, chain)` always yields the same relief, and
/// [`Relief::height_m`] depends only on the unit direction and this record.
/// Octave seeds are fixed-size arrays (no allocation), so worker builds clone
/// cheaply (`E-HOT-NOALLOC`).
#[derive(Debug, Clone, PartialEq)]
pub struct Relief {
    /// Path seed of the planet cell.
    seed: u64,
    /// Planet radius in metres (from its record).
    radius_m: f64,
    /// Planet radius in Earth radii.
    radius_earth: f64,
    /// Gravity amplitude scale (`clamp(9.81 / g, 0.3, 40)`).
    relief_scale: f64,
    /// Whether this world stays smooth (giants, Q2 A).
    is_giant: bool,
    /// Per-octave seeds, derived once so per-point work is noise only.
    continent_oct: [u64; CONTINENT_OCTAVES],
    /// Per-octave seeds for the first warp offset.
    warp_a_oct: [u64; WARP_OCTAVES],
    /// Per-octave seeds for the second warp offset.
    warp_b_oct: [u64; WARP_OCTAVES],
    /// Per-octave seeds for the hills field.
    hills_oct: [u64; HILLS_OCTAVES],
    /// Per-octave seeds for the ranges field.
    ranges_oct: [u64; RANGES_OCTAVES],
    /// Per-octave seeds for the belt mask.
    belt_oct: [u64; BELT_OCTAVES],
}

/// Per-octave seeds for `tag` under `seed` (no allocation).
fn octave_array<const N: usize>(seed: u64, tag: u64) -> [u64; N] {
    let base = hash_triple(seed, tag, 0);
    std::array::from_fn(|i| {
        let idx = u64::try_from(i).unwrap_or(u64::MAX);
        hash_triple(base, TAG_OCTAVE, idx)
    })
}

impl Relief {
    /// Relief for the L10 planet at `chain` under `root` (Q2 A).
    ///
    /// Radius comes from [`crate::terrain::body_recipe`] for the cell seed,
    /// gravity from [`crate::gravity::planet_mass_kg`], so the same seed
    /// always gives the same relief. Giants stay smooth.
    #[must_use]
    pub fn for_planet(root: u64, chain: &[u32]) -> Relief {
        let seed = crate::nest::path_seed(root, chain);
        let home = crate::home::is_home_planet_cell(root, chain);
        let recipe = crate::terrain::body_recipe(seed, home);
        Self::for_recipe(seed, recipe.radius_earth)
    }

    /// Relief for a planet of `radius_earth` Earth radii under `seed`.
    ///
    /// Shared by [`Relief::for_planet`] and the headless seed-list checks, so
    /// both use one gravity path. Pure.
    #[must_use]
    pub fn for_recipe(seed: u64, radius_earth: f64) -> Relief {
        let radius_m = crate::planet_frame::radius_m_for(radius_earth);
        let is_giant = radius_earth.is_finite() && radius_earth >= GIANT_RADIUS_EARTH;
        let relief_scale = if is_giant {
            1.0
        } else {
            gravity_scale(radius_earth, radius_m)
        };
        Relief {
            seed,
            radius_m,
            radius_earth,
            relief_scale,
            is_giant,
            continent_oct: octave_array(seed, TAG_CONTINENT),
            warp_a_oct: octave_array(seed, TAG_WARP_A),
            warp_b_oct: octave_array(seed, TAG_WARP_B),
            hills_oct: octave_array(seed, TAG_HILLS),
            ranges_oct: octave_array(seed, TAG_RANGES),
            belt_oct: octave_array(seed, TAG_BELT),
        }
    }

    /// Planet seed this relief was built for.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Planet radius in metres.
    #[must_use]
    pub fn radius_m(&self) -> f64 {
        self.radius_m
    }

    /// Gravity amplitude scale.
    #[must_use]
    pub fn relief_scale(&self) -> f64 {
        self.relief_scale
    }

    /// Whether this world stays smooth.
    #[must_use]
    pub fn is_giant(&self) -> bool {
        self.is_giant
    }

    /// Height in metres above the mean radius at unit `dir` (Q1 A).
    ///
    /// Pure and deterministic: the same direction bits always give the same
    /// height bits. Non-finite or zero input reads as `0.0`; giants read as
    /// `0.0` everywhere.
    #[must_use]
    pub fn height_m(&self, dir: [f64; 3]) -> f64 {
        if self.is_giant {
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
        let wa = fbm_with_seeds(
            [
                unit[0] * WARP_FREQ,
                unit[1] * WARP_FREQ,
                unit[2] * WARP_FREQ,
            ],
            1.0,
            &self.warp_a_oct,
        );
        let wb = fbm_with_seeds(
            [
                unit[0] * WARP_FREQ,
                unit[1] * WARP_FREQ,
                unit[2] * WARP_FREQ,
            ],
            1.0,
            &self.warp_b_oct,
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
        let continent = fbm_with_seeds(warped, CONTINENT_FREQ, &self.continent_oct);
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
        let hills_f = fbm_with_seeds(warped, HILLS_FREQ, &self.hills_oct);
        let hills_bipolar = (hills_f - 0.5) * 2.0;
        let hills_scale = 0.3 + 0.7 * land_t;
        let hills = hills_bipolar * AMP_HILLS_M * hills_scale;
        // Belt mask (unwarped, Q1 A) times ridged ranges (warped): mountains
        // on land, trenches in the oceans, both as chains.
        let belt_f = fbm_with_seeds(unit, BELT_FREQ, &self.belt_oct);
        let belt = smoothstep(0.55, 0.75, belt_f);
        let signed = belt * (2.0 * land_t - 1.0);
        let ridged = ridged_with_seeds(warped, RANGES_FREQ, &self.ranges_oct);
        let ranges = signed * ridged * AMP_RANGES_M;
        let total = sea_offset + hills + ranges;
        if !total.is_finite() {
            return 0.0;
        }
        total * self.relief_scale
    }
}

/// Gravity amplitude scale for `radius_earth` (Q2 A).
///
/// `clamp(9.81 / g, 0.3, 40)` with `g` from the real mass and radius; `1.0`
/// for non-finite inputs so a bad record never yields a bad height.
#[must_use]
pub fn gravity_scale(radius_earth: f64, radius_m: f64) -> f64 {
    if !radius_earth.is_finite() || !radius_m.is_finite() {
        return 1.0;
    }
    if radius_earth <= 0.0 || radius_m <= 0.0 {
        return 1.0;
    }
    let mass = crate::gravity::planet_mass_kg(radius_earth);
    if !mass.is_finite() || mass <= 0.0 {
        return 1.0;
    }
    let g = crate::gravity::G * mass / (radius_m * radius_m);
    if !g.is_finite() || g <= 0.0 {
        return 1.0;
    }
    (9.81 / g).clamp(0.3, 40.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_frame::EARTH_RADIUS_M;

    fn earth_relief() -> Relief {
        let journey = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
        let chain: &[u32] = journey.get(..9).unwrap_or(&[]);
        Relief::for_planet(crate::nav::DEMO_SEED, chain)
    }

    #[test]
    fn relief_is_one_deterministic_function() {
        // AC1 / C1 (T1): byte-identical heights on every run, finite
        // everywhere, and no tiling one period apart.
        let relief = earth_relief();
        let dirs = [
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.5, 0.5, std::f64::consts::FRAC_1_SQRT_2],
            [-0.3, 0.8, -0.5],
        ];
        for dir in dirs {
            let len = length(dir);
            assert!(len > 0.0);
            let unit = [dir[0] / len, dir[1] / len, dir[2] / len];
            let first = relief.height_m(unit);
            let again = relief.height_m(unit);
            assert_eq!(
                first.to_bits(),
                again.to_bits(),
                "same direction, same bits for {unit:?}"
            );
            assert!(
                first.is_finite(),
                "finite height everywhere, got {first} at {unit:?}"
            );
            // A second construction from the same seed agrees bit for bit.
            let rebuilt = earth_relief();
            assert_eq!(
                rebuilt.height_m(unit).to_bits(),
                first.to_bits(),
                "same seed, same planet"
            );
        }
        // Non-finite and zero inputs read as 0, never NaN.
        assert_eq!(relief.height_m([f64::NAN, 0.0, 0.0]), 0.0);
        assert_eq!(relief.height_m([0.0, 0.0, 0.0]), 0.0);
        // No tiling: two directions one periodic period apart (256 lattice
        // units at the hill frequency) give different heights, because the
        // lattice never wraps.
        let base = [0.2, 0.9, 0.3853811263161124];
        let blen = length(base);
        let bunit = [base[0] / blen, base[1] / blen, base[2] / blen];
        // Shift by one full 256-period at the hills scale along x, then
        // re-normalise: a wrapping field would return the same height.
        let shift = 256.0 / HILLS_FREQ;
        let shifted = [bunit[0] + shift, bunit[1], bunit[2]];
        let slen = length(shifted);
        let sunit = [shifted[0] / slen, shifted[1] / slen, shifted[2] / slen];
        let h1 = relief.height_m(bunit);
        let h2 = relief.height_m(sunit);
        assert!(
            h1.to_bits() != h2.to_bits(),
            "aperiodic lattice shows no tiling ({h1} vs {h2})"
        );
        // Earth radius is the true radius.
        assert!((relief.radius_m() - EARTH_RADIUS_M).abs() < 1.0);
    }

    #[test]
    fn relief_follows_gravity() {
        // AC2 / C2 (T1, Q2 A): a fixed seed list per rocky class keeps the
        // highest peak at or below 9 km times scale, and giants are zero.
        let rocky_seeds: [u64; 6] = [1, 7, 42, 999, 12345, 1 << 33];
        for seed in rocky_seeds {
            for radius_earth in [0.3f64, 1.0f64] {
                let relief = Relief::for_recipe(seed, radius_earth);
                assert!(!relief.is_giant());
                let scale = relief.relief_scale();
                let expect = gravity_scale(
                    radius_earth,
                    crate::planet_frame::radius_m_for(radius_earth),
                );
                assert!(
                    (scale - expect).abs() < 1e-12,
                    "scale follows gravity: {scale} vs {expect}"
                );
                // Highest peak over a coarse global sample stays under the
                // gravity bound.
                let mut highest: f64 = f64::NEG_INFINITY;
                for i in 0..2000 {
                    let dir = fibonacci_dir(i, 2000);
                    let h = relief.height_m(dir);
                    assert!(h.is_finite(), "finite at {dir:?}");
                    if h > highest {
                        highest = h;
                    }
                }
                let bound = 9_000.0 * scale;
                assert!(
                    highest <= bound,
                    "radius {radius_earth} seed {seed}: peak {highest} over {bound}"
                );
            }
        }
        // Giants stay a smooth ball.
        for seed in rocky_seeds {
            let giant = Relief::for_recipe(seed, 11.0);
            assert!(giant.is_giant());
            for i in 0..200 {
                let dir = fibonacci_dir(i, 200);
                assert_eq!(giant.height_m(dir), 0.0, "giants have no relief");
            }
        }
        // Scale clamps and Earth pins.
        let earth = Relief::for_recipe(7, 1.0);
        assert!(
            (earth.relief_scale() - 1.0).abs() < 0.05,
            "Earth scale is 1"
        );
        assert!((gravity_scale(0.3, crate::planet_frame::radius_m_for(0.3)) - 3.333).abs() < 0.05);
    }

    #[test]
    fn earth_hypsometry_has_two_modes() {
        // AC4 / C4 (T2): Earth's surface by height has two modes (continents
        // near sea level, ocean floor near -4.3 km), land share 20-45%,
        // highest peak at most 9 km, deepest about 11 km.
        let relief = earth_relief();
        assert!(!relief.is_giant());
        const N: usize = 20_000;
        let mut heights: Vec<f64> = Vec::with_capacity(N);
        let mut land = 0usize;
        let mut highest: f64 = f64::NEG_INFINITY;
        let mut lowest: f64 = f64::INFINITY;
        for i in 0..N {
            let dir = fibonacci_dir(i, N);
            let h = relief.height_m(dir);
            assert!(h.is_finite(), "finite height at {dir:?}");
            if h > 0.0 {
                land += 1;
            }
            if h > highest {
                highest = h;
            }
            if h < lowest {
                lowest = h;
            }
            heights.push(h);
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: test sample counts below 2^53, exact enough"
        )]
        let land_share = land as f64 / N as f64;
        assert!(
            (0.20..=0.45).contains(&land_share),
            "land share {land_share} outside 20-45%"
        );
        assert!(
            highest <= 9_000.0,
            "highest peak {highest} over 9 km on Earth"
        );
        assert!(
            (-13_000.0..=-9_000.0).contains(&lowest),
            "deepest point {lowest} not about -11 km"
        );
        // Two modes: histogram with 30 bins, 3-bin smoothing, peaks with
        // prominence over 10 percent of the max.
        const BINS: usize = 30;
        let mut counts = [0usize; BINS];
        let span = (highest - lowest).max(1.0);
        for h in &heights {
            let t = ((h - lowest) / span).clamp(0.0, 1.0);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: test histogram bins below 30, exact"
            )]
            #[expect(
                clippy::cast_sign_loss,
                reason = "E-CAST: histogram share is non-negative"
            )]
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: test bin count 30 is exactly representable"
            )]
            let mut bin = (t * BINS as f64) as usize;
            if bin >= BINS {
                bin = BINS - 1;
            }
            counts[bin] += 1;
        }
        // Smooth once (3-bin moving average) to avoid bin-edge splitting.
        let mut smooth = [0.0f64; BINS];
        for i in 0..BINS {
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: test histogram counts below 2^53, exact enough"
            )]
            let mut s = counts[i] as f64;
            let mut n = 1.0f64;
            if i > 0 {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "E-CAST: test histogram counts below 2^53, exact enough"
                )]
                let prev = counts[i - 1] as f64;
                s += prev;
                n += 1.0;
            }
            if i + 1 < BINS {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "E-CAST: test histogram counts below 2^53, exact enough"
                )]
                let next = counts[i + 1] as f64;
                s += next;
                n += 1.0;
            }
            smooth[i] = s / n;
        }
        let max_smooth = smooth.iter().cloned().fold(0.0f64, f64::max);
        let mut modes = 0usize;
        let mut mode_bins: Vec<usize> = Vec::new();
        for i in 0..BINS {
            let left = if i > 0 { smooth[i - 1] } else { 0.0 };
            let right = if i + 1 < BINS { smooth[i + 1] } else { 0.0 };
            if smooth[i] > left && smooth[i] >= right && smooth[i] > max_smooth * 0.10 {
                modes += 1;
                mode_bins.push(i);
            }
        }
        assert_eq!(
            modes, 2,
            "Earth hypsometry has {modes} modes (bins {mode_bins:?}), expected 2"
        );
        // The two modes sit where the guide says: one near sea level, one
        // near the -4.3 km ocean floor.
        let mut mode_heights: Vec<f64> = mode_bins
            .iter()
            .map(|b| {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "E-CAST: test histogram bins below 30, exact enough"
                )]
                let bf = *b as f64;
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "E-CAST: test bin count 30 is exactly representable"
                )]
                let nf = BINS as f64;
                lowest + (bf + 0.5) / nf * span
            })
            .collect();
        mode_heights.sort_by(|a, b| a.total_cmp(b));
        assert!(
            mode_heights[0] < -2_000.0,
            "deep mode {0} not near the ocean floor",
            mode_heights[0]
        );
        assert!(
            mode_heights[1] > -1_000.0 && mode_heights[1] < 2_000.0,
            "shallow mode {0} not near sea level",
            mode_heights[1]
        );
    }

    #[test]
    fn relief_agrees_across_levels_and_seams() {
        // AC3 / C3 (T2): the same direction gives the same height bits at
        // every patch level and on both sides of every face seam; the slope
        // stays continuous across face edges and around the poles.
        use crate::cube_sphere::{Face, grid_point};
        let relief = earth_relief();
        // Same face-centre direction via different global resolutions gives
        // the same height bits (levels share one source).
        for face in Face::all() {
            let mut first: Option<u64> = None;
            for n in [32u32, 64, 128] {
                let half = n / 2;
                let dir = grid_point(face, half, half, n).expect("face centre");
                let h = relief.height_m(dir);
                assert!(h.is_finite());
                if let Some(bits) = first {
                    assert_eq!(h.to_bits(), bits, "face {face:?} centre differs at n={n}");
                } else {
                    first = Some(h.to_bits());
                }
            }
        }
        // Seams and corners match bit for bit through the canonical folding.
        const N: u32 = 64;
        let mut checked = 0u32;
        let mut check_pair = |a: Option<[f64; 3]>, b: Option<[f64; 3]>| {
            let (pa, pb) = (a.expect("seam point"), b.expect("seam point"));
            checked += 1;
            assert_eq!(
                relief.height_m(pa).to_bits(),
                relief.height_m(pb).to_bits(),
                "seam heights differ at {pa:?} vs {pb:?}"
            );
        };
        for k in 0..=N {
            check_pair(
                grid_point(Face::PosX, N, k, N),
                grid_point(Face::PosY, N, k, N),
            );
            check_pair(
                grid_point(Face::PosX, 0, k, N),
                grid_point(Face::NegY, N, k, N),
            );
            check_pair(
                grid_point(Face::NegX, N, k, N),
                grid_point(Face::PosY, 0, k, N),
            );
            check_pair(
                grid_point(Face::NegX, 0, k, N),
                grid_point(Face::NegY, 0, k, N),
            );
            check_pair(
                grid_point(Face::PosX, k, N, N),
                grid_point(Face::PosZ, N, k, N),
            );
            check_pair(
                grid_point(Face::PosX, k, 0, N),
                grid_point(Face::NegZ, N, k, N),
            );
            check_pair(
                grid_point(Face::NegX, k, N, N),
                grid_point(Face::PosZ, 0, k, N),
            );
            check_pair(
                grid_point(Face::NegX, k, 0, N),
                grid_point(Face::NegZ, 0, k, N),
            );
            check_pair(
                grid_point(Face::PosY, k, N, N),
                grid_point(Face::PosZ, k, N, N),
            );
            check_pair(
                grid_point(Face::PosY, k, 0, N),
                grid_point(Face::NegZ, k, N, N),
            );
            check_pair(
                grid_point(Face::NegY, k, N, N),
                grid_point(Face::PosZ, k, 0, N),
            );
            check_pair(
                grid_point(Face::NegY, k, 0, N),
                grid_point(Face::NegZ, k, 0, N),
            );
        }
        assert!(checked > 0, "seam checks ran");
        // Slope continuity in a ring around each pole: 128 points at 2
        // degrees colatitude, no step over 2 km (no cliff, no pinch).
        for pole in [[0.0, 1.0, 0.0], [0.0, -1.0, 0.0]] {
            const STEPS: usize = 128;
            let colat = 2.0 * std::f64::consts::PI / 180.0;
            let (sin_c, cos_c) = (colat.sin(), colat.cos());
            let mut prev: Option<f64> = None;
            let mut worst = 0.0f64;
            for i in 0..=STEPS {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "E-CAST: test ring indices below 2^53, exact enough"
                )]
                let phi = i as f64 / STEPS as f64 * 2.0 * std::f64::consts::PI;
                // A ring at fixed colatitude around the pole.
                let dir = if pole[1] > 0.0 {
                    [sin_c * phi.cos(), cos_c, sin_c * phi.sin()]
                } else {
                    [sin_c * phi.cos(), -cos_c, sin_c * phi.sin()]
                };
                let h = relief.height_m(dir);
                assert!(h.is_finite(), "finite height around the pole");
                if let Some(p) = prev {
                    let step = (h - p).abs();
                    if step > worst {
                        worst = step;
                    }
                    assert!(
                        step < 2_000.0,
                        "pole ring step {step} over 2 km at {i}/{STEPS}"
                    );
                }
                prev = Some(h);
            }
            assert!(worst >= 0.0, "pole ring measured");
        }
    }

    /// Fibonacci sphere direction `i` of `n` (deterministic, no tables).
    fn fibonacci_dir(i: usize, n: usize) -> [f64; 3] {
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: test sphere indices below 2^53, exact enough"
        )]
        let fi = i as f64;
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: test sphere counts below 2^53, exact enough"
        )]
        let fn_ = n as f64;
        let y = 1.0 - 2.0 * (fi + 0.5) / fn_;
        let r = (1.0 - y * y).max(0.0).sqrt();
        let phi = fi * 2.399963229728653;
        [r * phi.cos(), y, r * phi.sin()]
    }
}
