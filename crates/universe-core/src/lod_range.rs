//! CDLOD distance ranges per level (#817 Q2 A, F1 A, Q3 A).
//!
//! Each level owns a distance range proportional to its patch size, so
//! ranges double from one level to the next coarser one. At the end of a
//! range one patch quad (1/32 of a patch) covers `quad_px` pixels at
//! 1600×900; Planet detail sets `quad_px` (High 2, Normal 4, Low 8).
//!
//! Ranges are scaled between 1× and 4× by a smooth relief roughness
//! ([`crate::relief::Relief::roughness`]). It is sampled on a fixed lattice
//! per cube face (`LATTICE_N` cells per edge, canonical folding so seams
//! share their nodes) and interpolated bilinearly, so the scale is a
//! continuous function of position: both sides of any patch edge read the
//! same value. A patch's largest scale over its area
//! ([`LevelRanges::scale_max`](crate::lod_range::LevelRanges::scale_max)) is exact for that interpolation, because a
//! bilinear patch takes its extremes at lattice nodes.
//!
//! Pure and deterministic (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use crate::cube_sphere::{Face, from_sphere, grid_point};
use crate::lod::{PATCH_QUADS, PatchKey, patch_uv_range};
use crate::precision::focal_px;
use crate::relief::Relief;

/// Lattice cells per cube-face edge (level 6 of the grid, about 150 km on
/// Earth: the roughness changes over hundreds of kilometres).
pub const LATTICE_N: u32 = 64;

/// Largest range scale (Q3 A: 1× to 4×).
pub const MAX_SCALE: f64 = 4.0;

/// First level whose ranges may be scaled: below it patches are larger
/// than a lattice cell, so one rough corner would scale a whole huge patch
/// and its plain neighbours would be forced to split two levels at once
/// (pops measured at 250-2,000 km).
pub const ROUGH_FROM_LEVEL: u32 = 6;

/// Largest range scale at `level`: 1 up to [`ROUGH_FROM_LEVEL`], then
/// growing by `sqrt(2)` per level up to [`MAX_SCALE`] (2 at level 8, 4
/// from level 10). Growing slower than the ranges shrink keeps every finer
/// range inside its coarser one, and neighbours rarely more than one level
/// apart (a doubling ramp still forced normal pops near 370 km).
#[must_use]
pub fn scale_cap(level: u32) -> f64 {
    if level <= ROUGH_FROM_LEVEL {
        return 1.0;
    }
    let steps = f64::from(level - ROUGH_FROM_LEVEL);
    std::f64::consts::SQRT_2.powf(steps).min(MAX_SCALE)
}

/// Range scale for a roughness in `[0, 1]`: `1 + 3 × roughness`.
#[must_use]
pub fn scale_for(roughness: f64) -> f64 {
    let r = if roughness.is_finite() {
        roughness.clamp(0.0, 1.0)
    } else {
        0.0
    };
    1.0 + (MAX_SCALE - 1.0) * r
}

/// Patch edge length at `level` on a planet of `radius_m` (metres, along
/// the sphere: a quarter turn per face, halved per level).
#[must_use]
pub fn edge_m(level: u32, radius_m: f64) -> f64 {
    let per = 2f64.powi(i32::try_from(level).unwrap_or(i32::MAX));
    radius_m * std::f64::consts::FRAC_PI_2 / per
}

/// Range scale at unit `dir` straight from the relief: the bilinear blend
/// of the four lattice nodes around it. [`LevelRanges::scale_at`] reads the
/// same values from its stored lattice; the patch build uses this one.
#[must_use]
pub fn scale_at_dir(relief: &Relief, dir: [f64; 3]) -> f64 {
    lattice_blend(dir, |face, i, j| node_scale(relief, face, i, j))
}

/// [`scale_at_dir`] with the lattice nodes it reads kept in `cache`
/// (`(face, i, j, scale)`): one patch's vertices share a handful of nodes,
/// so each is evaluated once per build instead of once per vertex.
#[must_use]
pub fn scale_at_dir_cached(
    relief: &Relief,
    dir: [f64; 3],
    cache: &mut Vec<(Face, u32, u32, f64)>,
) -> f64 {
    let cell = std::cell::RefCell::new(cache);
    lattice_blend(dir, |face, i, j| {
        let mut held = cell.borrow_mut();
        if let Some(&(_, _, _, value)) = held.iter().find(|n| n.0 == face && n.1 == i && n.2 == j) {
            return value;
        }
        let value = node_scale(relief, face, i, j);
        held.push((face, i, j, value));
        value
    })
}

fn node_scale(relief: &Relief, face: Face, i: u32, j: u32) -> f64 {
    grid_point(face, i, j, LATTICE_N).map_or(1.0, |dir| scale_for(relief.roughness(dir)))
}

/// Bilinear blend at `dir` of the lattice values given by `node`.
fn lattice_blend(dir: [f64; 3], node: impl Fn(Face, u32, u32) -> f64) -> f64 {
    let Some(pos) = from_sphere(dir) else {
        return 1.0;
    };
    let n = f64::from(LATTICE_N);
    let gu = ((pos.u + 1.0) / 2.0 * n).clamp(0.0, n);
    let gv = ((pos.v + 1.0) / 2.0 * n).clamp(0.0, n);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "E-CAST: clamped lattice index in 0..LATTICE_N, exact"
    )]
    let (i0, j0) = (
        gu.floor().min(n - 1.0) as u32,
        gv.floor().min(n - 1.0) as u32,
    );
    let (tu, tv) = (gu - f64::from(i0), gv - f64::from(j0));
    let a = node(pos.face, i0, j0);
    let b = node(pos.face, i0 + 1, j0);
    let c = node(pos.face, i0, j0 + 1);
    let d = node(pos.face, i0 + 1, j0 + 1);
    let low = a + (b - a) * tu;
    let high = c + (d - c) * tu;
    low + (high - low) * tv
}

/// The ranges of one planet at one Planet detail.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelRanges {
    radius_m: f64,
    quad_px: f64,
    /// Scales at the lattice nodes, face by face, row by row.
    lattice: Vec<f64>,
}

impl LevelRanges {
    /// Ranges for a planet of `radius_m` with `relief`, a quad covering
    /// `quad_px` pixels at the end of each range.
    #[must_use]
    pub fn new(relief: &Relief, radius_m: f64, quad_px: f64) -> LevelRanges {
        let side = LATTICE_N + 1;
        let mut lattice = Vec::with_capacity(6 * (side * side) as usize);
        for face in Face::all() {
            for j in 0..side {
                for i in 0..side {
                    lattice.push(node_scale(relief, face, i, j));
                }
            }
        }
        LevelRanges {
            radius_m,
            quad_px,
            lattice,
        }
    }

    /// The same ranges at another Planet detail (`quad_px`).
    #[must_use]
    pub fn with_quad_px(&self, quad_px: f64) -> LevelRanges {
        LevelRanges {
            quad_px,
            ..self.clone()
        }
    }

    /// Pixels per quad at the end of a range.
    #[must_use]
    pub fn quad_px(&self) -> f64 {
        self.quad_px
    }

    fn node(&self, face: Face, i: u32, j: u32) -> f64 {
        let side = (LATTICE_N + 1) as usize;
        let face_at = Face::all().iter().position(|f| *f == face).unwrap_or(0);
        let at = face_at * side * side + j as usize * side + i as usize;
        self.lattice.get(at).copied().unwrap_or(1.0)
    }

    /// Range scale at unit `dir` (1 to [`MAX_SCALE`]).
    #[must_use]
    pub fn scale_at(&self, dir: [f64; 3]) -> f64 {
        lattice_blend(dir, |face, i, j| self.node(face, i, j))
    }

    /// Largest range scale over `key`'s area: the largest lattice node of
    /// the cells it overlaps.
    #[must_use]
    pub fn scale_max(&self, key: PatchKey) -> f64 {
        let Some((u0, u1, v0, v1)) = patch_uv_range(key) else {
            return MAX_SCALE;
        };
        let n = f64::from(LATTICE_N);
        let cell = |t: f64| ((t + 1.0) / 2.0 * n).clamp(0.0, n);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "E-CAST: clamped lattice index in 0..=LATTICE_N, exact"
        )]
        let span = |a: f64, b: f64| (cell(a).floor() as u32, cell(b).ceil() as u32);
        let (i0, i1) = span(u0, u1);
        let (j0, j1) = span(v0, v1);
        let mut worst = 1.0f64;
        for j in j0..=j1.min(LATTICE_N) {
            for i in i0..=i1.min(LATTICE_N) {
                worst = worst.max(self.node(key.face, i, j));
            }
        }
        worst
    }

    /// Distance range of `level` at range scale `scale` (metres): a quad
    /// (1/32 of the patch edge) covers `quad_px` pixels at its end.
    #[must_use]
    pub fn range_m(&self, level: u32, scale: f64) -> f64 {
        let quad_m = edge_m(level, self.radius_m) / f64::from(PATCH_QUADS);
        if !self.quad_px.is_finite() || self.quad_px <= 0.0 {
            return 0.0;
        }
        quad_m * scale * focal_px() / self.quad_px
    }
}
