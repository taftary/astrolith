//! Geomorph blending between LOD levels (rebuild step 4, #777).
//!
//! CDLOD-style per-vertex morph (Q2 A, Q6 A, Q7 A, #777 review items 1, 3
//! and 6): every vertex stores its parent-level position and normal plus a
//! morph error in metres, and blends by **its own** distance to the camera.
//! The band of a vertex ends where its parent splits
//! (`error x focal / detail`, the split rule of `lod_select`), and the blend
//! runs over the last 30% of that band. A patch that has just split
//! therefore starts in its parent's shape, and a patch about to merge is
//! already fully blended to it.
//!
//! Edges follow the morph mask (Q3 A): on an edge touching a coarser
//! neighbour the in-between points are pinned to the coarse edge (factor 1);
//! on an edge touching a finer neighbour they hold their own place
//! (factor 0), so both sides always draw the same line. Shared edge points
//! carry an error computed from the edge alone, so the two same-level sides
//! of an edge blend them by the same amount.
//!
//! This module is the single formula: `patch_morph.wgsl` mirrors
//! [`vertex_factor`](crate::lod_morph::vertex_factor) line for line, and the headless `MORPH` line measures
//! with it. Pure and deterministic (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use crate::lod::PatchKey;
use crate::lod_select::{NeighbourDir, neighbour_at_same_level};
use crate::precision::focal_px;
use crate::relief::Relief;

/// Share of the distance band used for blending (CDLOD default, Q7 A).
pub const MORPH_ZONE_SHARE: f64 = 0.3;

/// East edge bit (bit 0 of an edge set).
pub const EDGE_EAST: u8 = 1;
/// West edge bit (bit 1).
pub const EDGE_WEST: u8 = 2;
/// North edge bit (bit 2).
pub const EDGE_NORTH: u8 = 4;
/// South edge bit (bit 3).
pub const EDGE_SOUTH: u8 = 8;

/// Edges of a morph mask that touch a coarser neighbour (low nibble).
#[must_use]
pub const fn mask_coarser(mask: u8) -> u8 {
    mask & 0x0F
}

/// Edges of a morph mask that touch a finer neighbour (high nibble).
#[must_use]
pub const fn mask_finer(mask: u8) -> u8 {
    mask >> 4
}

/// Morph mask from its two edge sets (coarser low, finer high).
#[must_use]
pub const fn morph_mask(coarser: u8, finer: u8) -> u8 {
    (coarser & 0x0F) | ((finer & 0x0F) << 4)
}

/// Distance in metres where a vertex with `err_m` is fully blended.
///
/// `err_m x focal / detail`: the same rule that splits the parent
/// (`lod_select`), so the band ends where the parent splits (review
/// item 6). Returns 0 for a bad error or threshold (no blending).
#[must_use]
pub fn morph_end_m(err_m: f64, detail_px: f64) -> f64 {
    if !err_m.is_finite() || err_m <= 0.0 || !detail_px.is_finite() || detail_px <= 0.0 {
        return 0.0;
    }
    let end = err_m * focal_px() / detail_px;
    if end.is_finite() && end > 0.0 {
        end
    } else {
        0.0
    }
}

/// Blending factor at `distance_m` for a band ending at `end_m`.
///
/// 0 up to `0.7 x end` (own shape), 1 from `end` on (parent shape), linear
/// over the last 30% of the band in between. Returns 0 for a bad band.
#[must_use]
pub fn morph_factor(distance_m: f64, end_m: f64) -> f64 {
    if !distance_m.is_finite() || !end_m.is_finite() || end_m <= 0.0 {
        return 0.0;
    }
    let zone = end_m * MORPH_ZONE_SHARE;
    let start = end_m - zone;
    if distance_m <= start {
        return 0.0;
    }
    if distance_m >= end_m {
        return 1.0;
    }
    ((distance_m - start) / zone).clamp(0.0, 1.0)
}

/// Edges on which grid point `(di, dj)` (0..=32) is an in-between point.
///
/// Only odd points on a patch edge move when that edge is pinned or held;
/// corners and even edge points are on the parent grid and never move.
#[must_use]
pub const fn odd_edge_bits(di: u32, dj: u32) -> u8 {
    let mut bits = 0u8;
    if di == 32 && dj % 2 == 1 {
        bits |= EDGE_EAST;
    }
    if di == 0 && dj % 2 == 1 {
        bits |= EDGE_WEST;
    }
    if dj == 32 && di % 2 == 1 {
        bits |= EDGE_NORTH;
    }
    if dj == 0 && di % 2 == 1 {
        bits |= EDGE_SOUTH;
    }
    bits
}

/// Blend factor of one vertex (the formula `patch_morph.wgsl` mirrors).
///
/// `distance_m` is the vertex's own distance to the camera, `err_m` its
/// morph error, `edge_bits` from [`odd_edge_bits`] and `mask` the patch's
/// morph mask: pinned to 1 on a coarser edge, held at 0 on a finer edge,
/// otherwise [`morph_factor`] over the vertex's band.
#[must_use]
pub fn vertex_factor(distance_m: f64, err_m: f64, detail_px: f64, edge_bits: u8, mask: u8) -> f64 {
    if edge_bits & mask_coarser(mask) != 0 {
        return 1.0;
    }
    if edge_bits & mask_finer(mask) != 0 {
        return 0.0;
    }
    morph_factor(distance_m, morph_end_m(err_m, detail_px))
}

/// Whether grid index `i` (0..=32) sits on the parent grid (even).
#[must_use]
pub const fn is_parent_grid(i: u32) -> bool {
    i.is_multiple_of(2)
}

/// Angular grid spacing at `level` in radians (a quarter turn per face).
///
/// The same for every patch of a level, so both sides of a seam use the
/// same normal stencil.
#[must_use]
pub fn level_step_rad(level: u32) -> f64 {
    let per_edge = 2f64.powi(i32::try_from(level).unwrap_or(i32::MAX));
    std::f64::consts::FRAC_PI_2 / (32.0 * per_edge)
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if !len.is_finite() || len <= 0.0 {
        return None;
    }
    Some([v[0] / len, v[1] / len, v[2] / len])
}

/// Terrain normal at unit `dir` from the relief (Q6 A, review item 2).
///
/// Central differences of the relief surface `(radius + h) x dir` along two
/// tangents fixed by `dir` alone, `step_rad` apart. A pure function of
/// `(dir, step)`, so a shared point gets the same normal from every patch
/// and every face. Returns `None` for a bad input.
#[must_use]
pub fn relief_normal(
    relief: &Relief,
    radius_m: f64,
    dir: [f64; 3],
    step_rad: f64,
) -> Option<[f64; 3]> {
    if !radius_m.is_finite() || radius_m <= 0.0 || !step_rad.is_finite() || step_rad <= 0.0 {
        return None;
    }
    let dir = unit(dir)?;
    let up = if dir[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let t1 = unit(cross(up, dir))?;
    let t2 = cross(dir, t1);
    let surface = |offset: [f64; 3]| -> Option<[f64; 3]> {
        let d = unit([dir[0] + offset[0], dir[1] + offset[1], dir[2] + offset[2]])?;
        let h = relief.height_m(d);
        if !h.is_finite() {
            return None;
        }
        let r = radius_m + h;
        Some([d[0] * r, d[1] * r, d[2] * r])
    };
    let scaled = |t: [f64; 3], s: f64| [t[0] * s, t[1] * s, t[2] * s];
    let a = sub(
        surface(scaled(t1, step_rad))?,
        surface(scaled(t1, -step_rad))?,
    );
    let b = sub(
        surface(scaled(t2, step_rad))?,
        surface(scaled(t2, -step_rad))?,
    );
    let n = unit(cross(a, b))?;
    let outward = n[0] * dir[0] + n[1] * dir[1] + n[2] * dir[2];
    if outward < 0.0 {
        Some([-n[0], -n[1], -n[2]])
    } else {
        Some(n)
    }
}

/// Morph mask for `key` against any drawn set (#777 Q3 A, review item 4).
///
/// For each edge (bit 0 East, 1 West, 2 North, 3 South): the bit in the low
/// nibble is set when the area across the edge is drawn coarser (an
/// ancestor of the same-level neighbour is drawn), the bit in the high
/// nibble when it is drawn finer (neither the neighbour nor an ancestor is
/// drawn, so its children are). Same-level edges leave both clear. `drawn`
/// answers membership (a sorted slice or a set), so each edge costs
/// `O(depth)` lookups, not a scan over every patch.
#[must_use]
pub fn morph_mask_in(key: PatchKey, drawn: &dyn Fn(PatchKey) -> bool) -> u8 {
    let mut coarser = 0u8;
    let mut finer = 0u8;
    for (bit, dir) in [
        (1u8, NeighbourDir::East),
        (2u8, NeighbourDir::West),
        (4u8, NeighbourDir::North),
        (8u8, NeighbourDir::South),
    ] {
        let Some(nkey) = neighbour_at_same_level(key, dir) else {
            continue;
        };
        if drawn(nkey) {
            continue;
        }
        let mut cursor = nkey;
        let mut is_coarser = false;
        while let Some(parent) = cursor.parent() {
            if drawn(parent) {
                is_coarser = true;
                break;
            }
            cursor = parent;
        }
        if is_coarser {
            coarser |= bit;
        } else {
            finer |= bit;
        }
    }
    morph_mask(coarser, finer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factor_is_zero_outside_and_one_at_the_end() {
        let end = 200.0;
        assert_eq!(morph_factor(50.0, end), 0.0);
        // Zone is the last 30%: 140..200.
        assert_eq!(morph_factor(140.0, end), 0.0);
        let mid = morph_factor(170.0, end);
        assert!((mid - 0.5).abs() < 1e-12, "mid {mid}");
        assert_eq!(morph_factor(200.0, end), 1.0);
        assert_eq!(morph_factor(300.0, end), 1.0);
        assert_eq!(morph_factor(f64::NAN, end), 0.0);
        assert_eq!(morph_factor(170.0, 0.0), 0.0);
    }

    #[test]
    fn masks_pin_coarse_edges_and_hold_fine_edges() {
        // Q3 A: an in-between point on a coarser edge is pinned (1), on a
        // finer edge it holds (0), elsewhere it follows its band.
        let err = 2.0;
        let detail = 1.5;
        let far = morph_end_m(err, detail) * 0.85;
        let free = vertex_factor(far, err, detail, EDGE_EAST, 0);
        assert!(free > 0.0 && free < 1.0, "free {free}");
        let pinned = vertex_factor(far, err, detail, EDGE_EAST, morph_mask(EDGE_EAST, 0));
        assert_eq!(pinned, 1.0);
        let held = vertex_factor(far, err, detail, EDGE_EAST, morph_mask(0, EDGE_EAST));
        assert_eq!(held, 0.0);
        // Interior points ignore the mask.
        let inner = vertex_factor(far, err, detail, 0, morph_mask(0x0F, 0));
        assert_eq!(inner, free);
        assert_eq!(odd_edge_bits(32, 5), EDGE_EAST);
        assert_eq!(odd_edge_bits(0, 32), 0, "corners never move");
        assert_eq!(odd_edge_bits(7, 0), EDGE_SOUTH);
        assert_eq!(odd_edge_bits(16, 31), 0);
    }

    #[test]
    fn band_ends_where_the_parent_splits() {
        // Review item 6: the end is `err x focal / detail`, the distance at
        // which a patch with that error reaches the split threshold.
        let err = 3.0;
        let detail = 1.5;
        let end = morph_end_m(err, detail);
        let px_at_end = err / end * focal_px();
        assert!((px_at_end - detail).abs() < 1e-9, "px {px_at_end}");
        assert_eq!(morph_end_m(0.0, detail), 0.0);
        assert_eq!(morph_end_m(err, 0.0), 0.0);
    }
}
