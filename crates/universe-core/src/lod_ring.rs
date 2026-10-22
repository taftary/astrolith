//! The ground ring and the build order (#918 T2, Q2 A, Q3 A, Q5 A, P1 B).
//!
//! Near the player the tree keeps every leaf within
//! [`RING_M`](crate::lod_ring::RING_M) of the
//! camera split and built in every direction, also behind the camera, so
//! turning around shows ready ground at once. A leaf counts as in the ring
//! when its relief box (the CDLOD box distance, #817) is closer than the
//! ring radius. That does not depend on where the camera looks, so the
//! ring's leaves never change while the camera only turns.
//!
//! Builds go nearest first: `wanted_nearest_into` orders the leaves to
//! build (the visible ones plus the ring) by that same box distance.
//! [`next_ring_m`](crate::lod_ring::next_ring_m) shrinks the ring while the ground is over its memory cap
//! and grows it back when there is room (the cap wins).
//!
//! Pure and deterministic (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use crate::lod::{PatchKey, patch_nearest_distance_with_relief_m};
use crate::planet_frame::PlanetPos;
use std::collections::BTreeSet;

/// Ring radius in metres (#918 P1 B).
pub const RING_M: f64 = 500.0;

/// Below this radius the ring is off rather than a few patches wide.
pub const RING_MIN_M: f64 = 25.0;

/// Share of the cap under which a shrunk ring may grow back.
const GROW_BELOW: f64 = 0.9;

/// Ring change per step when shrinking or growing.
const RING_STEP: f64 = 1.25;

/// Box distance from `camera` to `key` at its (or its nearest built
/// ancestor's) height range, in metres; `None` for a bad key.
#[must_use]
pub fn box_distance_m(
    key: PatchKey,
    camera: PlanetPos,
    radius_m: f64,
    relief_lookup: &dyn Fn(PatchKey) -> Option<(f64, f64, f64)>,
) -> Option<f64> {
    let (_, min_h, max_h) = crate::lod_select::lookup_or_ancestor(relief_lookup, key);
    patch_nearest_distance_with_relief_m(key, camera, radius_m, min_h, max_h)
}

/// Whether `key` lies in the ring of `ring_m` around `camera`.
pub(crate) fn in_ring(
    key: PatchKey,
    camera: PlanetPos,
    radius_m: f64,
    ring_m: f64,
    relief_lookup: &dyn Fn(PatchKey) -> Option<(f64, f64, f64)>,
) -> bool {
    ring_m > 0.0 && box_distance_m(key, camera, radius_m, relief_lookup).is_some_and(|d| d < ring_m)
}

/// The leaves to build, nearest first (#918 Q3 A): every leaf when `culled`
/// is false, else the `visible` ones (sorted) plus those in the ring.
/// Returns how many lead the list inside the ring: every ring leaf is
/// wanted, so they are exactly its first ones.
///
/// `scratch` and `out` keep their capacity (`E-HOT-NOALLOC`). Ties break
/// on the key, so the order is deterministic.
#[expect(
    clippy::too_many_arguments,
    reason = "a selection pass reads the tree, the view and the relief"
)]
pub(crate) fn wanted_nearest_into(
    leaves: &BTreeSet<PatchKey>,
    visible: &[PatchKey],
    culled: bool,
    camera: PlanetPos,
    radius_m: f64,
    ring_m: f64,
    relief_lookup: &dyn Fn(PatchKey) -> Option<(f64, f64, f64)>,
    scratch: &mut Vec<(f64, PatchKey)>,
    out: &mut Vec<PatchKey>,
) -> usize {
    scratch.clear();
    for leaf in leaves.iter().copied() {
        let distance =
            box_distance_m(leaf, camera, radius_m, relief_lookup).unwrap_or(f64::INFINITY);
        let wanted =
            !culled || visible.binary_search(&leaf).is_ok() || (ring_m > 0.0 && distance < ring_m);
        if wanted {
            scratch.push((distance, leaf));
        }
    }
    scratch.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    out.clear();
    out.extend(scratch.iter().map(|(_, key)| *key));
    scratch.iter().take_while(|(d, _)| *d < ring_m).count()
}

/// Next ring radius (#918 P1 B, P2 A): shrinks by a step while
/// `used_bytes` is over `cap_bytes` (turning off below [`RING_MIN_M`]),
/// grows back toward `target_m` while under `GROW_BELOW` of the cap, and
/// otherwise stays.
#[must_use]
pub fn next_ring_m(current_m: f64, target_m: f64, used_bytes: usize, cap_bytes: usize) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: byte counts compared as a share, intended"
    )]
    let (used, cap) = (used_bytes as f64, cap_bytes as f64);
    let current = current_m.clamp(0.0, target_m.max(0.0));
    if used > cap {
        let smaller = current / RING_STEP;
        return if smaller < RING_MIN_M { 0.0 } else { smaller };
    }
    if used < cap * GROW_BELOW && current < target_m {
        return (current.max(RING_MIN_M) * RING_STEP).min(target_m);
    }
    current
}
