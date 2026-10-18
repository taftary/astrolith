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
use std::hash::{BuildHasherDefault, Hasher};

/// Image size for the headless projection (1600 x 900).
const IMAGE_W: f64 = 1600.0;
const IMAGE_H: f64 = 900.0;

/// A gap this wide on screen counts as a crack (pixels): half a pixel, the
/// review's threshold. The only gaps left are `f32` rounding of patch
/// offsets, about 0.13 px (0.2 mm) one metre from a 5 km patch; the line
/// prints the worst gap so it stays visible.
pub(crate) const CRACK_PX: f64 = 0.5;

const SIDE: usize = 33;

/// Runs `work` over `items` split into one chunk per core and returns the
/// chunk results in item order. Only per-patch results, maxima and counts
/// are combined from them, so the output does not depend on the thread
/// count (`E-DET-TIERS`); on one core the path took about 90 s (#777).
pub(crate) fn par_chunks<T: Sync, R: Send>(items: &[T], work: impl Fn(&[T]) -> R + Sync) -> Vec<R> {
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    let size = items.len().div_ceil(threads).max(1);
    if threads == 1 || items.len() <= size {
        return vec![work(items)];
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = items
            .chunks(size)
            .map(|chunk| {
                let work = &work;
                scope.spawn(move || work(chunk))
            })
            .collect();
        handles
            .into_iter()
            .filter_map(|handle| handle.join().ok())
            .collect()
    })
}

/// Canonical bits of every edge point of a patch, per edge (South, North,
/// West, East) in [`edge_points`] order.
type PatchEdgeBits = [[Option<[u64; 3]>; SIDE]; 4];

/// Edge bits per patch, kept across frames: a grid point's bits never
/// change, and recomputing them every frame was a third of the path's time.
#[derive(Debug, Default)]
pub(crate) struct EdgeBits {
    map: HashMap<PatchKey, Box<PatchEdgeBits>>,
}

impl EdgeBits {
    fn fill(&mut self, key: PatchKey) {
        self.map.entry(key).or_insert_with(|| {
            let mut bits = Box::new([[None; SIDE]; 4]);
            for (edge, row) in bits.iter_mut().enumerate() {
                for (slot, (di, dj)) in row.iter_mut().zip(edge_points(edge)) {
                    *slot = point_bits(key, di, dj);
                }
            }
            bits
        });
    }

    fn row(&self, key: PatchKey) -> Option<&PatchEdgeBits> {
        self.map.get(&key).map(|bits| &**bits)
    }
}

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
    let blend_one = |key: PatchKey| -> Option<(PatchKey, Blended)> {
        let mesh = meshes.get(&key)?;
        let mask = morph_mask_in(key, &is_drawn);
        let mut pos = Vec::with_capacity(SIDE * SIDE);
        let mut nrm = Vec::with_capacity(SIDE * SIDE);
        for idx in 0..SIDE * SIDE {
            let (p, n) = blended_vertex(mesh, idx, camera, detail_px, mask)?;
            pos.push(p);
            nrm.push(n);
        }
        Some((key, Blended { pos, nrm }))
    };
    let patches = par_chunks(drawn, |chunk| {
        chunk
            .iter()
            .copied()
            .filter_map(blend_one)
            .collect::<Vec<_>>()
    })
    .into_iter()
    .flatten()
    .collect();
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

/// One drawn copy of an edge point: canonical bits, patch, blended position.
type Copy = ([u64; 3], PatchKey, [f64; 3]);

/// Multiply-xor hasher for point bits (already well mixed `f64` bits). The
/// default SipHash made the per-frame point grouping the slowest step.
#[derive(Debug, Default)]
struct BitsHasher(u64);

impl Hasher for BitsHasher {
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.write_u64(u64::from(*byte));
        }
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = (self.0.rotate_left(5) ^ value).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// Copies of one frame grouped by point, each group in patch order.
struct Groups {
    copies: Vec<Copy>,
    ids: HashMap<[u64; 3], usize, BuildHasherDefault<BitsHasher>>,
    /// Group `g` is `order[start[g]..start[g + 1]]` (indices into `copies`).
    start: Vec<usize>,
    order: Vec<usize>,
}

impl Groups {
    /// Groups `copies` by bits in linear time (a counting sort by group id,
    /// ids in first-seen order), so the result never depends on hashing.
    fn new(copies: Vec<Copy>) -> Groups {
        let mut ids: HashMap<[u64; 3], usize, BuildHasherDefault<BitsHasher>> =
            HashMap::with_capacity_and_hasher(copies.len(), BuildHasherDefault::default());
        let mut group_of = Vec::with_capacity(copies.len());
        for copy in &copies {
            let next = ids.len();
            group_of.push(*ids.entry(copy.0).or_insert(next));
        }
        let mut start = vec![0usize; ids.len() + 1];
        for g in &group_of {
            if let Some(count) = start.get_mut(g + 1) {
                *count += 1;
            }
        }
        for g in 1..start.len() {
            let before = start.get(g - 1).copied().unwrap_or(0);
            if let Some(at) = start.get_mut(g) {
                *at += before;
            }
        }
        let mut fill = start.clone();
        let mut order = vec![0usize; copies.len()];
        for (index, g) in group_of.iter().enumerate() {
            if let Some(at) = fill.get_mut(*g) {
                if let Some(slot) = order.get_mut(*at) {
                    *slot = index;
                }
                *at += 1;
            }
        }
        Groups {
            copies,
            ids,
            start,
            order,
        }
    }

    /// Indices of group `g`'s copies.
    fn members(&self, g: usize) -> &[usize] {
        let (Some(&from), Some(&to)) = (self.start.get(g), self.start.get(g + 1)) else {
            return &[];
        };
        self.order.get(from..to).unwrap_or(&[])
    }

    /// Indices of every copy of the point with `bits`.
    fn of(&self, bits: [u64; 3]) -> &[usize] {
        self.ids.get(&bits).map_or(&[], |g| self.members(*g))
    }

    fn copy(&self, index: usize) -> Option<&Copy> {
        self.copies.get(index)
    }

    fn len(&self) -> usize {
        self.ids.len()
    }
}

/// Cracks over one frame: `(count over CRACK_PX, worst gap in px)`.
pub(crate) fn crack_stats(
    frame: &Frame,
    camera: PlanetPos,
    edge_bits: &mut EdgeBits,
) -> (u32, f64) {
    for key in frame.patches.keys() {
        edge_bits.fill(*key);
    }
    let edge_bits = &*edge_bits;
    let patches: Vec<(&PatchKey, &Blended, &PatchEdgeBits)> = frame
        .patches
        .iter()
        .filter_map(|(key, blended)| Some((key, blended, edge_bits.row(*key)?)))
        .collect();
    // Every drawn copy of every edge point, grouped by point; each group
    // keeps patch order (deterministic).
    let mut copies: Vec<Copy> = Vec::with_capacity(patches.len() * 4 * SIDE);
    for &(key, blended, bits_of) in &patches {
        for (edge, row) in bits_of.iter().enumerate() {
            for ((di, dj), bits) in edge_points(edge).into_iter().zip(row) {
                let Some(bits) = *bits else {
                    continue;
                };
                if let Some(p) = blended.pos.get(dj as usize * SIDE + di as usize) {
                    copies.push((bits, *key, *p));
                }
            }
        }
    }
    let groups = Groups::new(copies);
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
    for g in 0..groups.len() {
        let list = groups.members(g);
        if list.len() < 2 {
            continue;
        }
        let mut gap = 0.0f64;
        for (i, a) in list.iter().enumerate() {
            for b in list.iter().skip(i + 1) {
                if let (Some(a), Some(b)) = (groups.copy(*a), groups.copy(*b)) {
                    gap = gap.max(gap_px(a.2, b.2, camera));
                }
            }
        }
        record(gap);
    }
    // T-junctions: an edge point no other patch draws must lie on the
    // coarse edge, halfway between the coarse points either side of it.
    // Per patch on every core; counts add and maxima combine exactly.
    let t_junctions = par_chunks(&patches, |chunk| {
        let mut gaps: Vec<f64> = Vec::new();
        for &(key, blended, bits_of) in chunk {
            for (edge, row) in bits_of.iter().enumerate() {
                let points = edge_points(edge);
                for k in (1..SIDE - 1).step_by(2) {
                    let (Some(&(di, dj)), Some(Some(bits))) = (points.get(k), row.get(k)) else {
                        continue;
                    };
                    if groups.of(*bits).len() > 1 {
                        continue;
                    }
                    let Some(fine) = blended.pos.get(dj as usize * SIDE + di as usize) else {
                        continue;
                    };
                    // The coarse patch drawing both points either side
                    // (corners are shared by several patches, so match it).
                    let coarse_copies = |at: usize| -> &[usize] {
                        row.get(at)
                            .copied()
                            .flatten()
                            .map_or(&[], |bits| groups.of(bits))
                    };
                    let (list_a, list_b) = (coarse_copies(k - 1), coarse_copies(k + 1));
                    let pair =
                        list_a
                            .iter()
                            .filter_map(|i| groups.copy(*i))
                            .find_map(|(_, ka, a)| {
                                if ka.level >= key.level {
                                    return None;
                                }
                                list_b
                                    .iter()
                                    .filter_map(|i| groups.copy(*i))
                                    .find(|(_, kb, _)| kb == ka)
                                    .map(|(_, _, b)| (*a, *b))
                            });
                    gaps.push(match pair {
                        Some((a, b)) => {
                            let want = [
                                (a[0] + b[0]) * 0.5,
                                (a[1] + b[1]) * 0.5,
                                (a[2] + b[2]) * 0.5,
                            ];
                            gap_px(*fine, want, camera)
                        }
                        // No coarse edge under an unshared point: a hole.
                        None => f64::INFINITY,
                    });
                }
            }
        }
        gaps
    });
    for gap in t_junctions.into_iter().flatten() {
        record(gap);
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
    let patches: Vec<(&PatchKey, &Blended)> = from.patches.iter().collect();
    par_chunks(&patches, |chunk| one_way_chunk(chunk, to, camera))
        .into_iter()
        .fold((0.0f64, 0.0f64), |(px, deg), (p, d)| {
            (px.max(p), deg.max(d))
        })
}

/// [`one_way`] over one chunk of `from`'s patches.
fn one_way_chunk(patches: &[(&PatchKey, &Blended)], to: &Frame, camera: PlanetPos) -> (f64, f64) {
    let mut worst_px = 0.0f64;
    let mut worst_deg = 0.0f64;
    for &(key, b) in patches {
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
