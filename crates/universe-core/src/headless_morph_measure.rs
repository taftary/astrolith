//! Crack, popping and normal measures on blended frames (#777 review
//! items 1 and 2).
//!
//! A frame is every drawn patch blended exactly as the vertex shader does
//! ([`crate::lod_patch_morph::blended_vertex`], masks from
//! [`crate::lod_morph::morph_mask_in`] against the drawn set). Points are
//! matched by their grid position, never by their blended position:
//! - **cracks:** every shared edge point is compared across the patches that
//!   draw it, and every in-between point of a finer edge against the coarse
//!   edge it must lie on (the midpoint of the coarse points around it);
//! - **popping and normals:** every point of one frame is compared with the
//!   other frame's surface at the same grid position, read from the patch
//!   that covers it there (the same patch, a coarser one through its
//!   triangles, or a finer one through its vertex), both ways, so points
//!   that appear on a split or vanish on a merge are compared too.
//!
//! Pure and deterministic (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use crate::cube_sphere::grid_point;
use crate::lod::{PATCH_QUADS, PatchKey};
use crate::lod_mesh::PatchMesh;
use crate::lod_morph::morph_mask_in;
use crate::lod_patch_morph::blended_vertex;
use crate::planet_frame::PlanetPos;
use crate::precision::focal_px;
use std::collections::{BTreeMap, HashMap};

/// Image size for the headless projection (1600 x 900).
const IMAGE_W: f64 = 1600.0;
const IMAGE_H: f64 = 900.0;

/// A gap this wide on screen counts as a crack (pixels): half a pixel, the
/// review's threshold. The only gaps left are `f32` rounding of patch
/// offsets, about 0.13 px (0.2 mm) one metre from a 5 km patch; the line
/// prints the worst gap so it stays visible.
pub(crate) const CRACK_PX: f64 = 0.5;

const SIDE: usize = 33;

/// One blended patch: 1089 positions and unit normals (planet frame).
#[derive(Debug, Clone)]
pub(crate) struct Blended {
    pos: Vec<[f64; 3]>,
    nrm: Vec<[f64; 3]>,
}

/// Every drawn patch of one frame, blended.
#[derive(Debug, Clone, Default)]
pub(crate) struct Frame {
    patches: BTreeMap<PatchKey, Blended>,
}

/// Blends every drawn patch with its mask against the drawn set.
///
/// `drawn` must be sorted (as `QuadTree::leaves` returns it); patches
/// without a mesh are skipped.
pub(crate) fn blend_frame(
    drawn: &[PatchKey],
    meshes: &HashMap<PatchKey, PatchMesh>,
    camera: PlanetPos,
    detail_px: f64,
) -> Frame {
    let is_drawn = |key: PatchKey| drawn.binary_search(&key).is_ok();
    let mut patches = BTreeMap::new();
    for key in drawn.iter().copied() {
        let Some(mesh) = meshes.get(&key) else {
            continue;
        };
        let mask = morph_mask_in(key, &is_drawn);
        let mut pos = Vec::with_capacity(SIDE * SIDE);
        let mut nrm = Vec::with_capacity(SIDE * SIDE);
        for idx in 0..SIDE * SIDE {
            let Some((p, n)) = blended_vertex(mesh, idx, camera, detail_px, mask) else {
                break;
            };
            pos.push(p);
            nrm.push(n);
        }
        if pos.len() == SIDE * SIDE {
            patches.insert(key, Blended { pos, nrm });
        }
    }
    Frame { patches }
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// Gap between two drawn copies of a point, in pixels at `camera`.
fn gap_px(a: [f64; 3], b: [f64; 3], camera: PlanetPos) -> f64 {
    let range = dist(a, camera.0).min(dist(b, camera.0));
    if !range.is_finite() || range <= 0.0 {
        return f64::INFINITY;
    }
    dist(a, b) / range * focal_px()
}

/// Canonical bits of grid point `(di, dj)` of `key` (same point, same bits,
/// on every level and both faces of a seam).
fn point_bits(key: PatchKey, di: u32, dj: u32) -> Option<[u64; 3]> {
    let per_edge = key.per_edge()?;
    let n = per_edge.checked_mul(PATCH_QUADS)?;
    let gi = key.x.checked_mul(PATCH_QUADS)?.checked_add(di)?;
    let gj = key.y.checked_mul(PATCH_QUADS)?.checked_add(dj)?;
    let dir = grid_point(key.face, gi, gj, n)?;
    Some([dir[0].to_bits(), dir[1].to_bits(), dir[2].to_bits()])
}

/// Grid indices of one edge in order (South, North, West, East).
fn edge_points(edge: usize) -> [(u32, u32); SIDE] {
    let mut out = [(0u32, 0u32); SIDE];
    for (k, slot) in out.iter_mut().enumerate() {
        let k = u32::try_from(k).unwrap_or(0);
        *slot = match edge {
            0 => (k, 0),
            1 => (k, PATCH_QUADS),
            2 => (0, k),
            _ => (PATCH_QUADS, k),
        };
    }
    out
}

/// Cracks over one frame: `(count over CRACK_PX, worst gap in px)`.
pub(crate) fn crack_stats(frame: &Frame, camera: PlanetPos) -> (u32, f64) {
    // Every drawn copy of every edge point, by canonical bits.
    let mut copies: HashMap<[u64; 3], Vec<(PatchKey, [f64; 3])>> = HashMap::new();
    for (key, blended) in &frame.patches {
        for edge in 0..4 {
            for (di, dj) in edge_points(edge) {
                let Some(bits) = point_bits(*key, di, dj) else {
                    continue;
                };
                let idx = dj as usize * SIDE + di as usize;
                if let Some(p) = blended.pos.get(idx) {
                    copies.entry(bits).or_default().push((*key, *p));
                }
            }
        }
    }
    let mut cracks = 0u32;
    let mut worst = 0.0f64;
    let mut record = |gap: f64| {
        if gap > CRACK_PX || !gap.is_finite() {
            cracks += 1;
        }
        if gap.is_finite() && gap > worst {
            worst = gap;
        }
    };
    // Shared points: all copies must coincide.
    let mut keys: Vec<&[u64; 3]> = copies.keys().collect();
    keys.sort();
    for bits in keys {
        let Some(list) = copies.get(bits) else {
            continue;
        };
        if list.len() < 2 {
            continue;
        }
        let mut gap = 0.0f64;
        for (i, (_, a)) in list.iter().enumerate() {
            for (_, b) in list.iter().skip(i + 1) {
                gap = gap.max(gap_px(*a, *b, camera));
            }
        }
        record(gap);
    }
    // T-junctions: an edge point no other patch draws must lie on the
    // coarse edge, halfway between the coarse points either side of it.
    for (key, blended) in &frame.patches {
        for edge in 0..4 {
            let points = edge_points(edge);
            for k in (1..SIDE - 1).step_by(2) {
                let (Some(&(di, dj)), Some(&(ai, aj)), Some(&(bi, bj))) =
                    (points.get(k), points.get(k - 1), points.get(k + 1))
                else {
                    continue;
                };
                let Some(bits) = point_bits(*key, di, dj) else {
                    continue;
                };
                if copies.get(&bits).is_some_and(|list| list.len() > 1) {
                    continue;
                }
                let Some(fine) = blended.pos.get(dj as usize * SIDE + di as usize) else {
                    continue;
                };
                // The coarse patch drawing both points either side (corners
                // are shared by several patches, so match the patch).
                let coarse_copies = |i: u32, j: u32| -> &[(PatchKey, [f64; 3])] {
                    point_bits(*key, i, j)
                        .and_then(|bits| copies.get(&bits))
                        .map_or(&[], Vec::as_slice)
                };
                let (list_a, list_b) = (coarse_copies(ai, aj), coarse_copies(bi, bj));
                let pair = list_a.iter().find_map(|(ka, a)| {
                    if ka.level >= key.level {
                        return None;
                    }
                    list_b
                        .iter()
                        .find(|(kb, _)| kb == ka)
                        .map(|(_, b)| (*a, *b))
                });
                match pair {
                    Some((a, b)) => {
                        let want = [
                            (a[0] + b[0]) * 0.5,
                            (a[1] + b[1]) * 0.5,
                            (a[2] + b[2]) * 0.5,
                        ];
                        record(gap_px(*fine, want, camera));
                    }
                    // No coarse edge under an unshared point: a hole.
                    None => record(f64::INFINITY),
                }
            }
        }
    }
    (cracks, worst)
}

/// Projects `point` with a camera at `camera` looking at the planet centre.
pub(crate) fn project_to_screen(point: [f64; 3], camera: PlanetPos) -> Option<[f64; 2]> {
    let [cx, cy, cz] = camera.0;
    let cam_len = (cx * cx + cy * cy + cz * cz).sqrt();
    if !cam_len.is_finite() || cam_len <= 0.0 {
        return None;
    }
    let fwd = [-cx / cam_len, -cy / cam_len, -cz / cam_len];
    let up0 = if fwd[1].abs() > 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let mut right = [
        fwd[1] * up0[2] - fwd[2] * up0[1],
        fwd[2] * up0[0] - fwd[0] * up0[2],
        fwd[0] * up0[1] - fwd[1] * up0[0],
    ];
    let rlen = (right[0] * right[0] + right[1] * right[1] + right[2] * right[2]).sqrt();
    if !rlen.is_finite() || rlen <= 0.0 {
        return None;
    }
    right = [right[0] / rlen, right[1] / rlen, right[2] / rlen];
    let up = [
        right[1] * fwd[2] - right[2] * fwd[1],
        right[2] * fwd[0] - right[0] * fwd[2],
        right[0] * fwd[1] - right[1] * fwd[0],
    ];
    let rel = [point[0] - cx, point[1] - cy, point[2] - cz];
    let x = rel[0] * right[0] + rel[1] * right[1] + rel[2] * right[2];
    let y = rel[0] * up[0] + rel[1] * up[1] + rel[2] * up[2];
    let z = rel[0] * fwd[0] + rel[1] * fwd[1] + rel[2] * fwd[2];
    if !z.is_finite() || z <= 0.0 {
        return None;
    }
    let focal = focal_px();
    Some([IMAGE_W / 2.0 + x / z * focal, IMAGE_H / 2.0 - y / z * focal])
}

fn on_screen(p: [f64; 2]) -> bool {
    (0.0..=IMAGE_W).contains(&p[0]) && (0.0..=IMAGE_H).contains(&p[1])
}

/// Position and normal of `frame`'s surface at grid point `(di, dj)` of
/// `key`, from whichever drawn patch covers it there.
fn surface_at(frame: &Frame, key: PatchKey, di: u32, dj: u32) -> Option<([f64; 3], [f64; 3])> {
    // Same patch.
    if let Some(b) = frame.patches.get(&key) {
        let idx = dj as usize * SIDE + di as usize;
        return Some((*b.pos.get(idx)?, *b.nrm.get(idx)?));
    }
    let q = f64::from(PATCH_QUADS);
    let gi = f64::from(key.x) * q + f64::from(di);
    let gj = f64::from(key.y) * q + f64::from(dj);
    // Coarser: the ancestor's triangles at the fractional grid position.
    let mut cursor = key;
    let mut scale = 1.0f64;
    while let Some(parent) = cursor.parent() {
        scale *= 2.0;
        cursor = parent;
        if let Some(b) = frame.patches.get(&parent) {
            let li = gi / scale - f64::from(parent.x) * q;
            let lj = gj / scale - f64::from(parent.y) * q;
            return triangle_at(b, li, lj);
        }
    }
    // Finer: the point is a vertex of whichever descendant covers it.
    let (mut gi_u, mut gj_u) = (
        key.x.checked_mul(PATCH_QUADS)?.checked_add(di)?,
        key.y.checked_mul(PATCH_QUADS)?.checked_add(dj)?,
    );
    let mut level = key.level;
    for t in 1..=24u32 {
        level = level.checked_add(1)?;
        gi_u = gi_u.checked_mul(2)?;
        gj_u = gj_u.checked_mul(2)?;
        // Stay inside `key`'s subtree (a point on its far edge belongs to
        // its last child, not to the neighbour's first).
        let x_max = key.x.checked_add(1)?.checked_shl(t)?.checked_sub(1)?;
        let y_max = key.y.checked_add(1)?.checked_shl(t)?.checked_sub(1)?;
        let x = (gi_u / PATCH_QUADS).min(x_max);
        let y = (gj_u / PATCH_QUADS).min(y_max);
        let child = PatchKey::new(key.face, level, x, y)?;
        if let Some(b) = frame.patches.get(&child) {
            let li = gi_u - x * PATCH_QUADS;
            let lj = gj_u - y * PATCH_QUADS;
            let idx = lj as usize * SIDE + li as usize;
            return Some((*b.pos.get(idx)?, *b.nrm.get(idx)?));
        }
    }
    None
}

/// The patch surface at local grid position `(li, lj)` (0..=32), through
/// the triangles the patch draws (`a, c, b / b, c, d`).
fn triangle_at(b: &Blended, li: f64, lj: f64) -> Option<([f64; 3], [f64; 3])> {
    if !(0.0..=32.0).contains(&li) || !(0.0..=32.0).contains(&lj) {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "E-CAST: clamped cell index 0..=31, exact"
    )]
    let (ci, cj) = (li.floor().min(31.0) as usize, lj.floor().min(31.0) as usize);
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: cell index under 32, exact"
    )]
    let (fx, fy) = (li - ci as f64, lj - cj as f64);
    let at = |i: usize, j: usize| j * SIDE + i;
    let (a, bb, c, d) = (
        at(ci, cj),
        at(ci + 1, cj),
        at(ci, cj + 1),
        at(ci + 1, cj + 1),
    );
    // Barycentric weights over (a, b, c) or (b, c, d).
    let weights: [(usize, f64); 3] = if fx + fy <= 1.0 {
        [(a, 1.0 - fx - fy), (bb, fx), (c, fy)]
    } else {
        [(d, fx + fy - 1.0), (c, 1.0 - fx), (bb, 1.0 - fy)]
    };
    let mut pos = [0.0f64; 3];
    let mut nrm = [0.0f64; 3];
    for (idx, w) in weights {
        let p = b.pos.get(idx)?;
        let n = b.nrm.get(idx)?;
        for (acc, v) in pos.iter_mut().zip(p) {
            *acc += v * w;
        }
        for (acc, v) in nrm.iter_mut().zip(n) {
            *acc += v * w;
        }
    }
    let len = (nrm[0] * nrm[0] + nrm[1] * nrm[1] + nrm[2] * nrm[2]).sqrt();
    if !len.is_finite() || len <= 0.0 {
        return None;
    }
    Some((pos, [nrm[0] / len, nrm[1] / len, nrm[2] / len]))
}

/// Worst on-screen move (px) and normal turn (degrees) of `from`'s points
/// against `to`'s surface at the same grid positions, seen from `camera`.
fn one_way(from: &Frame, to: &Frame, camera: PlanetPos) -> (f64, f64) {
    let mut worst_px = 0.0f64;
    let mut worst_deg = 0.0f64;
    for (key, b) in &from.patches {
        for dj in 0..=PATCH_QUADS {
            for di in 0..=PATCH_QUADS {
                let idx = dj as usize * SIDE + di as usize;
                let (Some(p), Some(n)) = (b.pos.get(idx), b.nrm.get(idx)) else {
                    continue;
                };
                let Some(a) = project_to_screen(*p, camera) else {
                    continue;
                };
                if !on_screen(a) {
                    continue;
                }
                let Some((q, m)) = surface_at(to, *key, di, dj) else {
                    continue;
                };
                if let Some(s) = project_to_screen(q, camera) {
                    let moved = ((a[0] - s[0]).powi(2) + (a[1] - s[1]).powi(2)).sqrt();
                    if moved.is_finite() {
                        worst_px = worst_px.max(moved);
                    }
                }
                let dot = (n[0] * m[0] + n[1] * m[1] + n[2] * m[2]).clamp(-1.0, 1.0);
                let deg = dot.acos().to_degrees();
                if deg.is_finite() {
                    worst_deg = worst_deg.max(deg);
                }
            }
        }
    }
    (worst_px, worst_deg)
}

/// Worst popping (px) and normal change (degrees) between two frames.
///
/// Both frames are projected with the current `camera`, so the camera's own
/// motion cancels out; only the surface change is left.
pub(crate) fn surface_change(prev: &Frame, curr: &Frame, camera: PlanetPos) -> (f64, f64) {
    let (px_a, deg_a) = one_way(curr, prev, camera);
    let (px_b, deg_b) = one_way(prev, curr, camera);
    (px_a.max(px_b), deg_a.max(deg_b))
}
