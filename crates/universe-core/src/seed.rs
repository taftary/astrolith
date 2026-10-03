//! Deterministic seed derivation and pseudo-random numbers.
//!
//! The ladder rule `child_seed = H(parent_seed, level, x, y, z)`
//! (`docs/universe/ladder.md`) is implemented here with an inline
//! splitmix64-style mixer: wrapping `u64` arithmetic only, so results are
//! identical on every platform. [`Rng`](crate::seed::Rng) is the matching small generator used
//! by [`crate::gen`] to turn a seed into reproducible content.

/// Golden-ratio additive constant of the splitmix64 sequence.
const GOLDEN_GAMMA: u64 = 0x9E3779B97F4A7C15;

/// First splitmix64 finalizer multiplier.
const MIX_MULT_A: u64 = 0xBF58476D1CE4E5B9;

/// Second splitmix64 finalizer multiplier.
const MIX_MULT_B: u64 = 0x94D049BB133111EB;

/// Applies the splitmix64 avalanche finalizer to `z`.
///
/// Every input bit influences every output bit; this is what makes nearby
/// cells (whose inputs differ by one coordinate) scatter to unrelated seeds.
const fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(MIX_MULT_A);
    z = (z ^ (z >> 27)).wrapping_mul(MIX_MULT_B);
    z ^ (z >> 31)
}

/// Derives a child cell seed `H(parent, level, x, y, z)`.
///
/// The parent seed is folded together with the level and the `i64` cell
/// coordinates (reinterpreted as `u64` bit patterns, which is platform
/// independent), then each lane is avalanched through `mix64`. Calling
/// order does not matter: the result depends only on the argument values, so
/// deriving a set of cells in any order yields the same mapping.
#[must_use]
#[expect(
    clippy::cast_sign_loss,
    reason = "E-CAST: coordinates reinterpreted as bit patterns in the hash mix"
)]
pub fn hash_cell(parent: u64, level: u8, x: i64, y: i64, z: i64) -> u64 {
    let mut acc = parent.wrapping_add(GOLDEN_GAMMA);
    acc = mix64(acc ^ (u64::from(level).wrapping_mul(MIX_MULT_A)));
    acc = mix64(acc ^ ((x as u64).wrapping_mul(MIX_MULT_B)));
    acc = mix64(acc ^ ((y as u64).wrapping_mul(MIX_MULT_A)));
    acc = mix64(acc ^ ((z as u64).wrapping_mul(MIX_MULT_B)));
    acc
}

/// Small deterministic generator (splitmix64 sequence) seeded from a `u64`.
///
/// Created per cell from [`hash_cell`]; the same seed replays the same
/// stream, so generated content is reproducible without storing it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng {
    /// Current sequence state; advanced by [`Rng::next_u64`].
    state: u64,
}

impl Rng {
    /// Creates a generator replaying the stream for `seed`.
    #[must_use]
    pub const fn new(seed: u64) -> Rng {
        Rng { state: seed }
    }

    /// Returns the next `u64` of the stream.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GOLDEN_GAMMA);
        mix64(self.state)
    }

    /// Returns the next `u32` of the stream (upper half of [`Rng::next_u64`]).
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Returns the next float in `[0.0, 1.0)` (53 significant bits).
    pub fn next_f64(&mut self) -> f64 {
        // Keep the top 53 bits and scale by 2^-53; the product is exact.
        let bits = self.next_u64() >> 11;
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: top 53 bits are exactly representable in f64"
        )]
        let unit = (bits as f64) * (1.0 / 9_007_199_254_740_992.0);
        unit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn same_input_derives_same_seed() {
        assert_eq!(hash_cell(12345, 7, 3, -4, 5), hash_cell(12345, 7, 3, -4, 5));
        let mut first = Rng::new(999);
        let mut second = Rng::new(999);
        for _ in 0..16 {
            assert_eq!(first.next_u64(), second.next_u64());
        }
        let mut third = Rng::new(999);
        let mut fourth = Rng::new(999);
        for _ in 0..16 {
            assert_eq!(third.next_f64().to_bits(), fourth.next_f64().to_bits());
        }
    }

    #[test]
    fn nearby_cells_scatter_to_different_seeds() {
        let origin = hash_cell(0x1234_5678_9ABC_DEF0, 5, 0, 0, 0);
        let neighbours = [
            hash_cell(0x1234_5678_9ABC_DEF0, 5, 1, 0, 0),
            hash_cell(0x1234_5678_9ABC_DEF0, 5, 0, 1, 0),
            hash_cell(0x1234_5678_9ABC_DEF0, 5, 0, 0, 1),
            hash_cell(0x1234_5678_9ABC_DEF0, 5, -1, -1, -1),
            hash_cell(0x1234_5678_9ABC_DEF0, 6, 0, 0, 0),
            hash_cell(0xDEAD_BEEF_CAFE_F00D, 5, 0, 0, 0),
        ];
        for other in neighbours {
            assert_ne!(origin, other, "avalanche failure");
            // Avalanche: flipping one input coordinate flips ~half the bits.
            assert!(
                (origin ^ other).count_ones() > 8,
                "weak avalanche between {origin:#x} and {other:#x}"
            );
        }
    }

    #[test]
    fn derivation_order_does_not_matter() {
        let cells = [(0i64, 0i64, 0i64), (1, 0, 0), (0, 1, 0), (-3, 7, 2)];
        let forward: BTreeMap<_, _> = cells
            .iter()
            .map(|&(x, y, z)| ((x, y, z), hash_cell(42, 9, x, y, z)))
            .collect();
        let backward: BTreeMap<_, _> = cells
            .iter()
            .rev()
            .map(|&(x, y, z)| ((x, y, z), hash_cell(42, 9, x, y, z)))
            .collect();
        assert_eq!(forward, backward);
    }

    #[test]
    fn rng_output_stays_in_range() {
        let mut rng = Rng::new(u64::MAX);
        for _ in 0..64 {
            let sample = rng.next_f64();
            assert!((0.0..1.0).contains(&sample), "out of range: {sample}");
        }
    }

    #[test]
    fn rng_output_covers_the_whole_unit_interval() {
        // R6 regression: a 2^-54 scale once confined every sample to [0, 0.5),
        // which pushed all generated content into one octant of each cell.
        let mut rng = Rng::new(7);
        let samples: Vec<f64> = (0..256).map(|_| rng.next_f64()).collect();
        let above_half = samples.iter().filter(|s| **s >= 0.5).count();
        assert!(
            (64..=192).contains(&above_half),
            "samples not spread over [0, 1): {above_half}/256 above 0.5"
        );
        assert!(
            samples.iter().any(|s| *s > 0.9),
            "upper decile never reached"
        );
    }
}
