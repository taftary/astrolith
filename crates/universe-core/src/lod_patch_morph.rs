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

use crate::cube_sphere::grid_point;
use crate::lod::{PATCH_QUADS, PatchKey};
use crate::lod_mesh::{PatchMesh, SKIRT_PER_EDGE, SKIRT_VERTS};
use crate::lod_morph::{level_step_rad, odd_edge_bits, relief_normal, vertex_factor};
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
    /// Relief heights (metres) of the ring points around the patch, 37 x
    /// 37 row-major: `None` for the patch's own points and off the face
    /// (#920 T5, so the shared-grid tile reuses them).
    pub ring_heights: Vec<Option<f64>>,
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

/// Points beyond a patch's edges, for normals by grid differences.
const RING: usize = 2;

/// Width of the patch grid plus its ring on both sides.
const RING_SIDE: usize = SIDE + 2 * RING;

/// A patch's grid points plus a ring of [`RING`] points around it (#918 T8).
///
/// Points off the cube face are `None`; a normal whose stencil needs one
/// falls back to [`relief_normal`], as every patch on that face edge does,
/// so seams across faces keep matching.
struct GridRing<'a> {
    radius_m: f64,
    relief: &'a Relief,
    points: Vec<Option<[f64; 3]>>,
    /// The sampled heights of the ring points (see `MorphShape`).
    heights: Vec<Option<f64>>,
}

impl<'a> GridRing<'a> {
    /// Reads the ring's heights; `abs` holds the patch's own 33 x 33 points.
    fn new(key: PatchKey, radius_m: f64, relief: &'a Relief, abs: &[[f64; 3]]) -> Option<Self> {
        let n_global = key.per_edge()?.checked_mul(PATCH_QUADS)?;
        let i0 = i64::from(key.x.checked_mul(PATCH_QUADS)?);
        let j0 = i64::from(key.y.checked_mul(PATCH_QUADS)?);
        let ring = i64::try_from(RING).ok()?;
        let mut points = Vec::with_capacity(RING_SIDE * RING_SIDE);
        let mut heights = vec![None; RING_SIDE * RING_SIDE];
        for rj in 0..RING_SIDE {
            for ri in 0..RING_SIDE {
                let (di, dj) = (ri.checked_sub(RING), rj.checked_sub(RING));
                if let (Some(di), Some(dj)) = (di, dj)
                    && di < SIDE
                    && dj < SIDE
                {
                    points.push(Some(*abs.get(dj * SIDE + di)?));
                    continue;
                }
                let gi = i0 + i64::try_from(ri).ok()? - ring;
                let gj = j0 + i64::try_from(rj).ok()? - ring;
                let on_face = |g: i64| u32::try_from(g).ok().filter(|g| *g <= n_global);
                let point = match (on_face(gi), on_face(gj)) {
                    (Some(gi), Some(gj)) => {
                        let dir = grid_point(key.face, gi, gj, n_global)?;
                        let h = relief.height_m(dir);
                        *heights.get_mut(rj * RING_SIDE + ri)? = Some(h);
                        let r = radius_m + h;
                        if !r.is_finite() || r <= 0.0 {
                            return None;
                        }
                        Some([dir[0] * r, dir[1] * r, dir[2] * r])
                    }
                    _ => None,
                };
                points.push(point);
            }
        }
        Some(Self {
            radius_m,
            relief,
            points,
            heights,
        })
    }

    /// The point `(di, dj)` of the patch grid, ring included.
    fn at(&self, di: i64, dj: i64) -> Option<[f64; 3]> {
        let ring = i64::try_from(RING).ok()?;
        let ri = usize::try_from(di + ring).ok()?;
        let rj = usize::try_from(dj + ring).ok()?;
        if ri >= RING_SIDE || rj >= RING_SIDE {
            return None;
        }
        *self.points.get(rj * RING_SIDE + ri)?
    }

    /// Outward unit normal at `(di, dj)` from the points `k` apart, or
    /// [`relief_normal`] at `step_rad` when the stencil leaves the face.
    fn normal(
        &self,
        di: usize,
        dj: usize,
        k: i64,
        dir: [f64; 3],
        step_rad: f64,
    ) -> Option<[f64; 3]> {
        let (i, j) = (i64::try_from(di).ok()?, i64::try_from(dj).ok()?);
        let stencil = (
            self.at(i + k, j),
            self.at(i - k, j),
            self.at(i, j + k),
            self.at(i, j - k),
        );
        let (Some(e), Some(w), Some(n), Some(s)) = stencil else {
            return relief_normal(self.relief, self.radius_m, dir, step_rad);
        };
        let a = [e[0] - w[0], e[1] - w[1], e[2] - w[2]];
        let b = [n[0] - s[0], n[1] - s[1], n[2] - s[2]];
        let c = unit([
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ])?;
        if c[0] * dir[0] + c[1] * dir[1] + c[2] * dir[2] < 0.0 {
            Some([-c[0], -c[1], -c[2]])
        } else {
            Some(c)
        }
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
    // Normals from differences on the canonical grid (#918 T8): one ring of
    // two extra points around the patch instead of four height reads per
    // normal. Same-level neighbours and the parent level share these grid
    // points, so seams keep identical normals.
    let ring = GridRing::new(key, radius_m, relief, abs)?;
    let mut own_n: Vec<[f64; 3]> = Vec::with_capacity(SIDE * SIDE);
    for (idx, dir) in dirs.iter().copied().enumerate() {
        own_n.push(ring.normal(idx % SIDE, idx / SIDE, 1, dir, step)?);
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
                *parent_n.get_mut(idx)? = ring.normal(di, dj, 2, dir, parent_step)?;
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
    // Morph errors (#817 Q4 A): each vertex blends over the last 30% of its
    // own level's range, so its error is the range's quad length times the
    // range scale at its point (`err x focal / quad_px` is the range end in
    // the shader). The scale is a function of position, so both sides of a
    // shared edge agree.
    let quad_m = crate::lod_range::edge_m(key.level, radius_m) / f64::from(PATCH_QUADS);
    let mut nodes = Vec::new();
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
            let scale = crate::lod_range::scale_at_dir_cached(relief, *dirs.get(idx)?, &mut nodes)
                .min(crate::lod_range::scale_cap(key.level));
            let err = quad_m * scale;
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
    // Outer side out (#778 Q5 A): `e0, s0, e1` faces away from the patch on
    // the South and East edges of a right-handed grid, and on the North and
    // West edges of a left-handed one; the other edges reverse.
    let right_handed = crate::lod::face_grid_right_handed(key.face);
    for (edge_no, edge) in edges.iter().enumerate() {
        let south_or_east = edge_no == 0 || edge_no == 3;
        let reverse = south_or_east != right_handed;
        let base = main_len + edge_no * SKIRT_PER_EDGE;
        for k in 0..PATCH_QUADS as usize {
            let e0 = u32::try_from(*edge.get(k)?).ok()?;
            let e1 = u32::try_from(*edge.get(k + 1)?).ok()?;
            let s0 = u32::try_from(base + k).ok()?;
            let s1 = u32::try_from(base + k + 1).ok()?;
            if reverse {
                skirt_indices.extend_from_slice(&[e0, e1, s0, e1, s1, s0]);
            } else {
                skirt_indices.extend_from_slice(&[e0, s0, e1, e1, s0, s1]);
            }
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
        ring_heights: ring.heights,
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

/// Points around a patch outline: 32 per side, 128 in all.
pub const OUTLINE_POINTS: usize = 4 * PATCH_QUADS as usize;

/// Grid place `(di, dj)` of outline point `n` (see [`outline_points`]).
fn outline_place(n: usize) -> Option<(u32, u32)> {
    let last = PATCH_QUADS;
    let k = u32::try_from(n % SIDE.saturating_sub(1)).ok()?;
    Some(match n / (SIDE - 1) {
        0 => (k, 0),
        1 => (last, k),
        2 => (last - k, last),
        _ => (0, last - k),
    })
}

/// Whether outline point `n` sits at its pinned (parent) place under
/// `mask`: an odd edge point on an edge that touches a coarser neighbour.
#[must_use]
pub fn outline_point_pinned(n: usize, mask: u8) -> bool {
    outline_place(n)
        .is_some_and(|(di, dj)| odd_edge_bits(di, dj) & crate::lod_morph::mask_coarser(mask) != 0)
}

/// The outline of `mesh` as the ground draws it, for the F3 overlay
/// (#918 Q1b A): every edge point in order (south row west to east, east
/// column south to north, north row east to west, west column north to
/// south), each an offset from the patch centre. An odd edge point on an
/// edge that touches a coarser neighbour (`mask`) sits at its pinned,
/// parent place, as the shader puts it, so a big patch's line runs exactly
/// through its small neighbours' corners and pinned points. `None` for a
/// mesh without its 33 x 33 points.
#[must_use]
pub fn outline_points(mesh: &PatchMesh, mask: u8) -> Option<[[f32; 3]; OUTLINE_POINTS]> {
    let mut out = [[0.0f32; 3]; OUTLINE_POINTS];
    for (n, slot) in out.iter_mut().enumerate() {
        let (di, dj) = outline_place(n)?;
        let idx = usize::try_from(dj).ok()? * SIDE + usize::try_from(di).ok()?;
        *slot = if outline_point_pinned(n, mask) {
            *mesh.parent_offsets.get(idx)?
        } else {
            *mesh.offsets.get(idx)?
        };
    }
    Some(out)
}
