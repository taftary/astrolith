//! The fixed patch set the build speed is measured on (#994 T1).
//!
//! Sixty patches: two on every cube face at each of five depths, from a
//! continent-sized patch down to a few hundred metres, spread over land,
//! sea and mountains by a fixed scatter. The app's `--patch-bench` mode
//! times their builds, and the same-ground check samples every direction
//! they read. Pure and deterministic (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use crate::cube_sphere::Face;
use crate::lod::PatchKey;
use crate::relief::Relief;

/// Depths of the fixed set, coarse to fine.
pub const BENCH_LEVELS: [u32; 5] = [2, 6, 10, 14, 18];

/// Patches per face and depth.
pub const BENCH_PER_CELL: u32 = 2;

/// Fixed scatter seed for the patch positions.
const SCATTER_SEED: u64 = 0x0994_0994_5EED_0994;

/// The fixed patch set: [`BENCH_LEVELS`] on every face, [`BENCH_PER_CELL`]
/// each, at scattered positions.
#[must_use]
pub fn bench_keys() -> Vec<PatchKey> {
    let mut keys = Vec::new();
    let mut state = SCATTER_SEED;
    for face in Face::all() {
        for level in BENCH_LEVELS {
            for k in 0..BENCH_PER_CELL {
                let per_edge = 1u64 << level;
                state = crate::seed::hash_triple(state, u64::from(level), per_edge);
                let x = u32::try_from((state >> 8) % per_edge).unwrap_or(0);
                // Each pick in its own share of the rows: never the same patch.
                let share = per_edge / u64::from(BENCH_PER_CELL);
                let y = u32::try_from(((state >> 36) % share) + u64::from(k) * share).unwrap_or(0);
                if let Some(key) = PatchKey::new(face, level, x, y) {
                    keys.push(key);
                }
            }
        }
    }
    keys
}

/// The home planet's relief and drawn radius, as the game builds them.
#[must_use]
pub fn home_relief() -> (Relief, f64) {
    let seed = crate::nav::DEMO_SEED;
    let journey = crate::nest::autopilot_path(seed);
    let l10 = journey.get(..9).unwrap_or(&[]);
    let frame = crate::planet_frame::PlanetFrame::for_planet(seed, l10);
    (Relief::for_planet(seed, l10), frame.radius_m())
}

#[cfg(test)]
#[path = "patch_bench_tests.rs"]
mod tests;
