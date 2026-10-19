//! A patch's 3D box at its real height range (#817 Q1 A, Q5 A).
//!
//! The box is axis-aligned in the planet frame. It covers the patch's 3 × 3
//! sampled directions (corners, edge middles, centre) at `radius + min_h`
//! and `radius + max_h`, grown on every side by how far the curved cap
//! bulges between samples plus the chord sag of the parent's quads. Every
//! point the patch draws, its own or its parent's (the #777 morph), lies
//! inside, so the box distance is a lower bound of every vertex's distance
//! to the camera. That bound is what keeps level switches pop-free: a patch
//! whose box is beyond a range has every vertex beyond it.
//!
//! Pure and deterministic (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use crate::cube_sphere::{FacePos, to_sphere};
use crate::lod::{PATCH_QUADS, PatchKey, patch_edge_angle, patch_uv_range};
use crate::planet_frame::PlanetPos;

/// An axis-aligned box in the planet frame (metres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatchBox {
    /// Lowest corner.
    pub lo: [f64; 3],
    /// Highest corner.
    pub hi: [f64; 3],
}

impl PatchBox {
    /// Distance from `point` to the box (0 inside).
    #[must_use]
    pub fn distance(&self, point: PlanetPos) -> f64 {
        let mut sum = 0.0f64;
        for ((p, lo), hi) in point.0.iter().zip(&self.lo).zip(&self.hi) {
            let d = if p < lo {
                lo - p
            } else if p > hi {
                p - hi
            } else {
                0.0
            };
            sum += d * d;
        }
        sum.sqrt()
    }
}

/// The box of `key` on a planet of `radius_m` with relief in
/// `[min_h_m, max_h_m]`; `None` for bad inputs.
#[must_use]
pub fn patch_box(key: PatchKey, radius_m: f64, min_h_m: f64, max_h_m: f64) -> Option<PatchBox> {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return None;
    }
    if !min_h_m.is_finite() || !max_h_m.is_finite() || min_h_m > max_h_m {
        return None;
    }
    let (u0, u1, v0, v1) = patch_uv_range(key)?;
    let inner = radius_m + min_h_m;
    let outer = radius_m + max_h_m;
    if inner <= 0.0 {
        return None;
    }
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for u in [u0, (u0 + u1) / 2.0, u1] {
        for v in [v0, (v0 + v1) / 2.0, v1] {
            let dir = to_sphere(FacePos::new(key.face, u, v)?)?;
            for r in [inner, outer] {
                for ((l, h), d) in lo.iter_mut().zip(hi.iter_mut()).zip(dir) {
                    let c = d * r;
                    *l = l.min(c);
                    *h = h.max(c);
                }
            }
        }
    }
    // Between samples half a patch apart the cap bulges by at most
    // `outer (1 - cos(theta / 2))` (theta the patch edge angle); the flat
    // quads and the parent's quads sit at most their chord sag inside it.
    let theta = patch_edge_angle(key)?;
    let bulge = outer * (1.0 - (theta / 2.0).cos());
    let parent_quad = theta * 2.0 / f64::from(PATCH_QUADS);
    let sag = outer * (1.0 - (parent_quad / 2.0).cos());
    let grow = bulge + sag;
    if !grow.is_finite() {
        return None;
    }
    for (l, h) in lo.iter_mut().zip(hi.iter_mut()) {
        *l -= grow;
        *h += grow;
    }
    if lo.iter().chain(hi.iter()).all(|c| c.is_finite()) {
        Some(PatchBox { lo, hi })
    } else {
        None
    }
}
