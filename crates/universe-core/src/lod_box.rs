//! A patch's 3D box at its real height range (#817 Q1 A, Q5 A).
//!
//! The box is aligned with the patch: its axes are the radial up at the
//! patch centre and two tangents, so the height range stretches it along
//! up only (a planet-axis box of a tilted patch spread every height range
//! sideways, and the camera sat inside the boxes of every patch within
//! kilometres). It covers the patch's 3 × 3
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

/// A box aligned with its patch (metres): extents along three orthonormal
/// axes (two tangents and the up at the patch centre).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatchBox {
    /// The box axes (unit, orthonormal).
    pub axes: [[f64; 3]; 3],
    /// Lowest coordinate along each axis.
    pub lo: [f64; 3],
    /// Highest coordinate along each axis.
    pub hi: [f64; 3],
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let len = dot(a, a).sqrt();
    if !len.is_finite() || len <= 0.0 {
        return None;
    }
    Some([a[0] / len, a[1] / len, a[2] / len])
}

impl PatchBox {
    /// Distance from `point` to the box (0 inside).
    #[must_use]
    pub fn distance(&self, point: PlanetPos) -> f64 {
        let mut sum = 0.0f64;
        for ((axis, lo), hi) in self.axes.iter().zip(&self.lo).zip(&self.hi) {
            let p = &dot(point.0, *axis);
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
    // Axes: the up at the patch centre and two tangents.
    let up = to_sphere(FacePos::new(key.face, (u0 + u1) / 2.0, (v0 + v1) / 2.0)?)?;
    let helper = if up[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let east = unit(cross(helper, up))?;
    let north = cross(up, east);
    let axes = [east, north, up];
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for u in [u0, (u0 + u1) / 2.0, u1] {
        for v in [v0, (v0 + v1) / 2.0, v1] {
            let dir = to_sphere(FacePos::new(key.face, u, v)?)?;
            for r in [inner, outer] {
                let p = [dir[0] * r, dir[1] * r, dir[2] * r];
                for ((l, h), axis) in lo.iter_mut().zip(hi.iter_mut()).zip(axes) {
                    let c = dot(p, axis);
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
        Some(PatchBox { axes, lo, hi })
    } else {
        None
    }
}
