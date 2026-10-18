//! Morph shape and skirts of one patch (rebuild step 4, #777).
//!
//! Split from `lod_mesh` (`E-FILE-SIZE`, #777 review item 8). For every
//! vertex of a 33 x 33 patch this builds:
//! - the terrain normal from the relief at the level's grid step
//!   (review item 2), and the parent normal at the parent's step;
//! - the parent-level position: even points keep their place, odd edge
//!   points take the midpoint along the edge, and odd-odd points take the
//!   midpoint of the diagonal the parent draws (`a, c, b / b, c, d` splits a
//!   quad along `b-c`, review item 5), so a fully blended patch is exactly
//!   its parent's triangles;
//! - the morph error (largest move to the parent shape, metres): each edge
//!   from its own points only, so both sides of a shared edge get the same
//!   number, the rest from the whole patch;
//! - the skirts below every edge (Q4 A), which move with their edge point.
//!
//! Pure and deterministic (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use crate::lod::{PATCH_QUADS, PatchKey};
use crate::lod_mesh::{PatchMesh, SKIRT_PER_EDGE, SKIRT_VERTS};
use crate::lod_morph::{
    EDGE_EAST, EDGE_NORTH, EDGE_SOUTH, EDGE_WEST, level_step_rad, odd_edge_bits, relief_normal,
    vertex_factor,
};
use crate::planet_frame::PlanetPos;
use crate::relief::Relief;

const SIDE: usize = 33;

/// Morph attributes and skirts for one patch.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MorphShape {
    pub normals: Vec<[f32; 3]>,
    pub parent_offsets: Vec<[f32; 3]>,
    pub parent_normals: Vec<[f32; 3]>,
    pub morph_err_m: Vec<f32>,
    pub skirt_offsets: Vec<[f32; 3]>,
    pub skirt_parent_offsets: Vec<[f32; 3]>,
    pub skirt_normals: Vec<[f32; 3]>,
    pub skirt_parent_normals: Vec<[f32; 3]>,
    pub skirt_morph_err_m: Vec<f32>,
    pub skirt_indices: Vec<u32>,
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "E-CAST: patch vectors narrowed for the GPU, intended"
)]
fn narrow(v: [f64; 3]) -> [f32; 3] {
    [v[0] as f32, v[1] as f32, v[2] as f32]
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

fn mid(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        (a[0] + b[0]) * 0.5,
        (a[1] + b[1]) * 0.5,
        (a[2] + b[2]) * 0.5,
    ]
}

fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if !len.is_finite() || len <= 0.0 {
        return None;
    }
    Some([v[0] / len, v[1] / len, v[2] / len])
}

/// The two parent-grid points an odd point `(di, dj)` sits between.
///
/// `None` for an even-even point (it is on the parent grid itself).
fn parent_pair(di: usize, dj: usize) -> Option<(usize, usize)> {
    let at = |i: usize, j: usize| j * SIDE + i;
    match (di % 2 == 1, dj % 2 == 1) {
        (false, false) => None,
        (false, true) => Some((at(di, dj - 1), at(di, dj + 1))),
        (true, false) => Some((at(di - 1, dj), at(di + 1, dj))),
        // The parent's diagonal b-c: (i+1, j) to (i, j+1).
        (true, true) => Some((at(di + 1, dj - 1), at(di - 1, dj + 1))),
    }
}

/// Edge index (East, West, North, South) for one odd edge bit.
fn edge_slot(bits: u8) -> Option<usize> {
    match bits {
        EDGE_EAST => Some(0),
        EDGE_WEST => Some(1),
        EDGE_NORTH => Some(2),
        EDGE_SOUTH => Some(3),
        _ => None,
    }
}

/// Builds the morph shape and skirts for `key` (see the module docs).
///
/// `abs` and `dirs` are the 1089 vertex positions (planet frame, `f64`) and
/// unit directions in row-major order; `centre` is the patch origin and
/// `patch_err_m` sets the skirt depth (`max(4 x err, 1 m)`).
pub(crate) fn morph_shape(
    key: PatchKey,
    radius_m: f64,
    relief: &Relief,
    centre: PlanetPos,
    abs: &[[f64; 3]],
    dirs: &[[f64; 3]],
    patch_err_m: f64,
) -> Option<MorphShape> {
    if abs.len() != SIDE * SIDE || dirs.len() != SIDE * SIDE {
        return None;
    }
    let step = level_step_rad(key.level);
    let mut own_n: Vec<[f64; 3]> = Vec::with_capacity(SIDE * SIDE);
    for dir in dirs.iter().copied() {
        own_n.push(relief_normal(relief, radius_m, dir, step)?);
    }
    // Parent shape (roots have none: they never merge).
    let mut parent_abs: Vec<[f64; 3]> = abs.to_vec();
    let mut parent_n: Vec<[f64; 3]> = own_n.clone();
    if key.level > 0 {
        let parent_step = step * 2.0;
        for dj in (0..SIDE).step_by(2) {
            for di in (0..SIDE).step_by(2) {
                let idx = dj * SIDE + di;
                let dir = *dirs.get(idx)?;
                *parent_n.get_mut(idx)? = relief_normal(relief, radius_m, dir, parent_step)?;
            }
        }
        for dj in 0..SIDE {
            for di in 0..SIDE {
                let Some((a, b)) = parent_pair(di, dj) else {
                    continue;
                };
                let idx = dj * SIDE + di;
                *parent_abs.get_mut(idx)? = mid(*abs.get(a)?, *abs.get(b)?);
                *parent_n.get_mut(idx)? = unit(mid(*parent_n.get(a)?, *parent_n.get(b)?))?;
            }
        }
    }
    // Morph errors: each edge from its own odd points (f64 from the shared
    // grid, so both sides agree bit for bit), the rest from the patch.
    let mut edge_err = [0.0f64; 4];
    let mut all_err = 0.0f64;
    for dj in 0..SIDE {
        for di in 0..SIDE {
            let idx = dj * SIDE + di;
            let moved = dist(*abs.get(idx)?, *parent_abs.get(idx)?);
            if !moved.is_finite() {
                return None;
            }
            all_err = all_err.max(moved);
            let bits = odd_edge_bits(u32::try_from(di).ok()?, u32::try_from(dj).ok()?);
            if let Some(slot) = edge_slot(bits) {
                let held = edge_err.get_mut(slot)?;
                *held = held.max(moved);
            }
        }
    }
    let mut normals = Vec::with_capacity(SIDE * SIDE);
    let mut parent_offsets = Vec::with_capacity(SIDE * SIDE);
    let mut parent_normals = Vec::with_capacity(SIDE * SIDE);
    let mut morph_err_m = Vec::with_capacity(SIDE * SIDE);
    for dj in 0..SIDE {
        for di in 0..SIDE {
            let idx = dj * SIDE + di;
            let p = *parent_abs.get(idx)?;
            normals.push(narrow(*own_n.get(idx)?));
            parent_offsets.push(narrow([
                p[0] - centre.0[0],
                p[1] - centre.0[1],
                p[2] - centre.0[2],
            ]));
            parent_normals.push(narrow(*parent_n.get(idx)?));
            let bits = odd_edge_bits(u32::try_from(di).ok()?, u32::try_from(dj).ok()?);
            let err = match edge_slot(bits) {
                Some(slot) => *edge_err.get(slot)?,
                None => all_err,
            };
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: morph error narrowed for the GPU, intended"
            )]
            morph_err_m.push(err as f32);
        }
    }
    // Skirts (Q4 A): one ring below every edge, down along the radius by
    // `max(4 x patch error, 1 m)`, in South, North, West, East order.
    let skirt_depth = (4.0 * patch_err_m.max(0.0)).max(1.0);
    if !skirt_depth.is_finite() {
        return None;
    }
    let last = SIDE - 1;
    let edges: [Vec<usize>; 4] = [
        (0..SIDE).collect(),
        (0..SIDE).map(|k| last * SIDE + k).collect(),
        (0..SIDE).map(|k| k * SIDE).collect(),
        (0..SIDE).map(|k| k * SIDE + last).collect(),
    ];
    let mut skirt_offsets = Vec::with_capacity(SKIRT_VERTS);
    let mut skirt_parent_offsets = Vec::with_capacity(SKIRT_VERTS);
    let mut skirt_normals = Vec::with_capacity(SKIRT_VERTS);
    let mut skirt_parent_normals = Vec::with_capacity(SKIRT_VERTS);
    let mut skirt_morph_err_m = Vec::with_capacity(SKIRT_VERTS);
    for edge in &edges {
        for idx in edge.iter().copied() {
            let down = *dirs.get(idx)?;
            let drop = |p: [f64; 3]| {
                narrow([
                    p[0] - down[0] * skirt_depth - centre.0[0],
                    p[1] - down[1] * skirt_depth - centre.0[1],
                    p[2] - down[2] * skirt_depth - centre.0[2],
                ])
            };
            skirt_offsets.push(drop(*abs.get(idx)?));
            skirt_parent_offsets.push(drop(*parent_abs.get(idx)?));
            skirt_normals.push(*normals.get(idx)?);
            skirt_parent_normals.push(*parent_normals.get(idx)?);
            skirt_morph_err_m.push(*morph_err_m.get(idx)?);
        }
    }
    let main_len = SIDE * SIDE;
    let mut skirt_indices: Vec<u32> = Vec::with_capacity(4 * 32 * 6);
    for (edge_no, edge) in edges.iter().enumerate() {
        let base = main_len + edge_no * SKIRT_PER_EDGE;
        for k in 0..PATCH_QUADS as usize {
            let e0 = u32::try_from(*edge.get(k)?).ok()?;
            let e1 = u32::try_from(*edge.get(k + 1)?).ok()?;
            let s0 = u32::try_from(base + k).ok()?;
            let s1 = u32::try_from(base + k + 1).ok()?;
            skirt_indices.extend_from_slice(&[e0, s0, e1, e1, s0, s1]);
        }
    }
    Some(MorphShape {
        normals,
        parent_offsets,
        parent_normals,
        morph_err_m,
        skirt_offsets,
        skirt_parent_offsets,
        skirt_normals,
        skirt_parent_normals,
        skirt_morph_err_m,
        skirt_indices,
    })
}

/// One blended main vertex, as the vertex shader draws it.
///
/// Mirrors `patch_morph.wgsl`: the factor comes from the vertex's own
/// distance to `camera` (its unblended place), its morph error and the
/// patch's morph `mask` ([`vertex_factor`]); position and normal blend
/// together. Returns the planet-frame position and the unit normal, or
/// `None` for an out-of-range index.
#[must_use]
pub fn blended_vertex(
    mesh: &PatchMesh,
    idx: usize,
    camera: PlanetPos,
    detail_px: f64,
    mask: u8,
) -> Option<([f64; 3], [f64; 3])> {
    let off = mesh.offsets.get(idx)?;
    let poff = mesh.parent_offsets.get(idx)?;
    let nrm = mesh.normals.get(idx)?;
    let pnrm = mesh.parent_normals.get(idx)?;
    let err = f64::from(*mesh.morph_err_m.get(idx)?);
    let di = u32::try_from(idx % SIDE).ok()?;
    let dj = u32::try_from(idx / SIDE).ok()?;
    let own = [
        mesh.centre.0[0] + f64::from(off[0]),
        mesh.centre.0[1] + f64::from(off[1]),
        mesh.centre.0[2] + f64::from(off[2]),
    ];
    let distance = dist(own, camera.0);
    let f = vertex_factor(distance, err, detail_px, odd_edge_bits(di, dj), mask);
    let lerp = |a: f32, b: f32| f64::from(a) + (f64::from(b) - f64::from(a)) * f;
    let pos = [
        mesh.centre.0[0] + lerp(off[0], poff[0]),
        mesh.centre.0[1] + lerp(off[1], poff[1]),
        mesh.centre.0[2] + lerp(off[2], poff[2]),
    ];
    let normal = unit([
        lerp(nrm[0], pnrm[0]),
        lerp(nrm[1], pnrm[1]),
        lerp(nrm[2], pnrm[2]),
    ])?;
    Some((pos, normal))
}
