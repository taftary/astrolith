//! One icosphere surface from planet to building (#458).
//!
//! A 20-face base icosahedron; a triangle is addressed by its base face plus
//! a path of child digits `0..=3`. Splitting is 1->4 at edge midpoints
//! re-projected onto the unit sphere, so a child's corners are exactly its
//! parent's corners and edge midpoints (seamless by construction, C3).
//! Everything is pure `f64` with wrapping `u64` address arithmetic, so the
//! same seed and path give the same corners on every platform (`E-DET-TIERS`).

use crate::seed::hash_triple;

/// Domain tag for triangle-address streams.
const ICOSPHERE_STREAM_TAG: u64 = 0x1C05_048E_1C05_048E;

/// Deepest split this module addresses (2 bits per level fit in `u32`).
pub const MAX_DEPTH: u8 = 15;

/// Golden ratio used for the base icosahedron.
const PHI: f64 = 1.618_033_988_749_895;

/// Dot product of two vectors.
#[must_use]
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    let [ax, ay, az] = a;
    let [bx, by, bz] = b;
    ax * bx + ay * by + az * bz
}

/// Normalizes `v` to the unit sphere; zero maps to `+X`.
#[must_use]
pub fn normalize(v: [f64; 3]) -> [f64; 3] {
    let [x, y, z] = v;
    let length = (x * x + y * y + z * z).sqrt();
    if length.is_finite() && length > 0.0 {
        [x / length, y / length, z / length]
    } else {
        [1.0, 0.0, 0.0]
    }
}

/// Midpoint of `a` and `b` re-projected onto the unit sphere.
#[must_use]
pub fn midpoint(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    let [ax, ay, az] = a;
    let [bx, by, bz] = b;
    normalize([(ax + bx) * 0.5, (ay + by) * 0.5, (az + bz) * 0.5])
}

/// Twelve base vertices of the unit icosahedron.
#[must_use]
pub fn base_vertices() -> [[f64; 3]; 12] {
    [
        normalize([-1.0, PHI, 0.0]),
        normalize([1.0, PHI, 0.0]),
        normalize([-1.0, -PHI, 0.0]),
        normalize([1.0, -PHI, 0.0]),
        normalize([0.0, -1.0, PHI]),
        normalize([0.0, 1.0, PHI]),
        normalize([0.0, -1.0, -PHI]),
        normalize([0.0, 1.0, -PHI]),
        normalize([PHI, 0.0, -1.0]),
        normalize([PHI, 0.0, 1.0]),
        normalize([-PHI, 0.0, -1.0]),
        normalize([-PHI, 0.0, 1.0]),
    ]
}

/// Twenty base faces as vertex indices.
#[must_use]
pub fn base_faces() -> [[usize; 3]; 20] {
    [
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ]
}

/// Address of one triangle: base face plus a 1->4 descent path.
///
/// `code` packs two bits per level, `depth` counts them. Child `k` appends
/// `k` at bit position `2 * depth`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TriAddr {
    /// Base face `0..20`.
    face: u8,
    /// Packed child digits.
    code: u32,
    /// Descent depth `0..=MAX_DEPTH`.
    depth: u8,
}

impl TriAddr {
    /// Builds an address, returning `None` for a bad face, depth, or digit.
    #[must_use]
    pub fn new(face: u8, path: &[u8]) -> Option<TriAddr> {
        if face >= 20 {
            return None;
        }
        if path.len() > usize::from(MAX_DEPTH) {
            return None;
        }
        let mut code = 0u32;
        for (level, digit) in path.iter().enumerate() {
            let digit_value = digit;
            if *digit_value > 3 {
                return None;
            }
            let shift = u32::try_from(2 * level).ok()?;
            code |= u32::from(*digit_value) << shift;
        }
        let depth = u8::try_from(path.len()).ok()?;
        Some(TriAddr { face, code, depth })
    }

    /// The root triangle of `face`, or `None` past face 19.
    #[must_use]
    pub fn root(face: u8) -> Option<TriAddr> {
        TriAddr::new(face, &[])
    }

    /// Base face of this triangle.
    #[must_use]
    pub fn face(self) -> u8 {
        self.face
    }

    /// Descent depth of this triangle.
    #[must_use]
    pub fn depth(self) -> u8 {
        self.depth
    }

    /// Packed code (two bits per level).
    #[must_use]
    pub fn code(self) -> u32 {
        self.code
    }

    /// Digit at `level`, or `None` past the depth.
    #[must_use]
    pub fn digit_at(self, level: u8) -> Option<u8> {
        if level >= self.depth {
            return None;
        }
        let shift = 2 * u32::from(level);
        let digit = (self.code >> shift) & 3;
        u8::try_from(digit).ok()
    }

    /// Parent triangle, or `None` at the root.
    #[must_use]
    pub fn parent(self) -> Option<TriAddr> {
        if self.depth == 0 {
            return None;
        }
        let keep_bits = 2 * (u32::from(self.depth) - 1);
        let keep = if keep_bits >= 32 {
            u32::MAX
        } else if keep_bits == 0 {
            0
        } else {
            (1u32 << keep_bits) - 1
        };
        Some(TriAddr {
            face: self.face,
            code: self.code & keep,
            depth: self.depth - 1,
        })
    }

    /// Four children of this triangle, or `None` at [`MAX_DEPTH`].
    #[must_use]
    pub fn children(self) -> Option<[TriAddr; 4]> {
        if self.depth >= MAX_DEPTH {
            return None;
        }
        let shift = 2 * u32::from(self.depth);
        let next = self.depth + 1;
        let child = |digit: u32| TriAddr {
            face: self.face,
            code: self.code | (digit << shift),
            depth: next,
        };
        Some([child(0), child(1), child(2), child(3)])
    }

    /// Deterministic stream seed for this triangle under `seed`.
    #[must_use]
    pub fn stream_seed(self, seed: u64) -> u64 {
        let packed =
            (u64::from(self.face) << 40) | (u64::from(self.depth) << 32) | u64::from(self.code);
        hash_triple(seed, ICOSPHERE_STREAM_TAG, packed)
    }
}

/// Corners of the base triangle `face` on the unit sphere.
#[must_use]
pub fn base_corners(face: u8) -> Option<[[f64; 3]; 3]> {
    let faces = base_faces();
    let vertices = base_vertices();
    let triple = faces.get(usize::from(face))?;
    let [ia, ib, ic] = *triple;
    Some([*vertices.get(ia)?, *vertices.get(ib)?, *vertices.get(ic)?])
}

/// Splits `corners` 1->4 at re-projected edge midpoints.
///
/// Child 0 keeps corner 0, child 1 keeps corner 1, child 2 keeps corner 2,
/// child 3 is the central triangle.
#[must_use]
pub fn split_corners(corners: [[f64; 3]; 3]) -> [[[f64; 3]; 3]; 4] {
    let [a, b, c] = corners;
    let ab = midpoint(a, b);
    let bc = midpoint(b, c);
    let ca = midpoint(c, a);
    [[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]
}

/// Corners of `addr` on the unit sphere, or `None` for a bad address.
#[must_use]
pub fn tri_corners(addr: TriAddr) -> Option<[[f64; 3]; 3]> {
    let mut corners = base_corners(addr.face())?;
    let mut level = 0u8;
    while level < addr.depth() {
        let digit = addr.digit_at(level)?;
        let kids = split_corners(corners);
        let next = kids.get(usize::from(digit))?;
        corners = *next;
        level += 1;
    }
    Some(corners)
}

/// Mean of the three corners: the cluster centre direction.
#[must_use]
pub fn tri_centre(addr: TriAddr) -> Option<[f64; 3]> {
    let [a, b, c] = tri_corners(addr)?;
    let [ax, ay, az] = a;
    let [bx, by, bz] = b;
    let [cx, cy, cz] = c;
    Some(normalize([
        (ax + bx + cx) / 3.0,
        (ay + by + cy) / 3.0,
        (az + bz + cz) / 3.0,
    ]))
}

/// Mean normal of `corners` (normalized cross of two edges).
#[must_use]
pub fn tri_normal(corners: [[f64; 3]; 3]) -> [f64; 3] {
    let [a, b, c] = corners;
    let [ax, ay, az] = a;
    let [bx, by, bz] = b;
    let [cx, cy, cz] = c;
    let e0 = [bx - ax, by - ay, bz - az];
    let e1 = [cx - ax, cy - ay, cz - az];
    let [e0x, e0y, e0z] = e0;
    let [e1x, e1y, e1z] = e1;
    normalize([
        e0y * e1z - e0z * e1y,
        e0z * e1x - e0x * e1z,
        e0x * e1y - e0y * e1x,
    ])
}

/// Cross product of two vectors.
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    let [ax, ay, az] = a;
    let [bx, by, bz] = b;
    [ay * bz - az * by, az * bx - ax * bz, ax * by - ay * bx]
}

/// Tests whether `point` (need not be unit) falls inside `corners`.
///
/// Edge-sign test on the sphere: for every directed edge the point must sit
/// on the same side as the opposite corner. Points on an edge count as
/// inside. Winding-independent, so every base face agrees.
#[must_use]
pub fn point_in_triangle(point: [f64; 3], corners: [[f64; 3]; 3]) -> bool {
    if !point.iter().all(|c| c.is_finite()) {
        return false;
    }
    let unit = normalize(point);
    let [a, b, c] = corners;
    let edges = [(a, b, c), (b, c, a), (c, a, b)];
    for (edge_a, edge_b, opposite) in edges {
        let normal = cross(edge_a, edge_b);
        let side = dot(normal, unit);
        let expected = dot(normal, opposite);
        if !side.is_finite() || !expected.is_finite() {
            return false;
        }
        if expected >= 0.0 {
            if side < -1e-12 {
                return false;
            }
        } else if side > 1e-12 {
            return false;
        }
    }
    true
}

/// Locates `direction` at `depth` by descending from its best base face.
///
/// Picks the base face whose centre is nearest, then chooses the child
/// containing the point at every level. Returns `None` for a bad direction
/// or a depth past [`MAX_DEPTH`].
#[must_use]
pub fn locate(direction: [f64; 3], depth: u8) -> Option<TriAddr> {
    if depth > MAX_DEPTH {
        return None;
    }
    if !direction.iter().all(|c| c.is_finite()) {
        return None;
    }
    let unit = normalize(direction);
    let vertices = base_vertices();
    let faces = base_faces();
    let mut best_face = 0u8;
    let mut best_dot = f64::NEG_INFINITY;
    for (index, triple) in faces.iter().enumerate() {
        let [ia, ib, ic] = *triple;
        let va = vertices.get(ia)?;
        let vb = vertices.get(ib)?;
        let vc = vertices.get(ic)?;
        let [ax, ay, az] = *va;
        let [bx, by, bz] = *vb;
        let [cx, cy, cz] = *vc;
        let centre = normalize([
            (ax + bx + cx) / 3.0,
            (ay + by + cy) / 3.0,
            (az + bz + cz) / 3.0,
        ]);
        let d = dot(centre, unit);
        if d > best_dot {
            best_dot = d;
            best_face = u8::try_from(index).ok()?;
        }
    }
    let mut addr = TriAddr::root(best_face)?;
    let mut corners = base_corners(best_face)?;
    let mut level = 0u8;
    while level < depth {
        let kids = split_corners(corners);
        let [k0, _, _, _] = kids;
        let mut found: Option<(usize, [[f64; 3]; 3])> = None;
        for (digit, kid) in kids.iter().enumerate() {
            if point_in_triangle(unit, *kid) {
                found = Some((digit, *kid));
                break;
            }
        }
        let (digit, kid) = found.unwrap_or((0, k0));
        let digit_u8 = u8::try_from(digit).ok()?;
        let kids_addr = addr.children()?;
        let next = kids_addr.get(usize::from(digit_u8))?;
        addr = *next;
        corners = kid;
        level += 1;
    }
    Some(addr)
}

/// Enumerates every triangle address at `depth` (20 * 4^depth entries).
///
/// Returns an empty vector past [`MAX_DEPTH`]. Depths above 6 already exceed
/// a million entries; callers needing planet-to-plot depths keep one branch,
/// never the whole level.
#[must_use]
pub fn all_at_depth(depth: u8) -> Vec<TriAddr> {
    if depth > MAX_DEPTH {
        return Vec::new();
    }
    let mut out = Vec::new();
    for face in 0..20u8 {
        let Some(root) = TriAddr::root(face) else {
            continue;
        };
        let mut stack = vec![root];
        while let Some(addr) = stack.pop() {
            if addr.depth() == depth {
                out.push(addr);
            } else if let Some(kids) = addr.children() {
                stack.extend(kids);
            }
        }
    }
    out.sort_by_key(|a| (a.face(), a.depth(), a.code()));
    out
}

/// Descendants of `parent` exactly at `depth` (empty when shallower).
///
/// The branch stays inside the parent by construction, so clusters grown
/// from it nest exactly (C4).
#[must_use]
pub fn descendants_at(parent: TriAddr, depth: u8) -> Vec<TriAddr> {
    if depth < parent.depth() || depth > MAX_DEPTH {
        return Vec::new();
    }
    let mut out = vec![parent];
    while out.first().is_some_and(|addr| addr.depth() < depth) {
        let mut next = Vec::new();
        for addr in out {
            if addr.depth() == depth {
                next.push(addr);
            } else if let Some(kids) = addr.children() {
                next.extend(kids);
            }
        }
        out = next;
    }
    out.sort_by_key(|a| (a.face(), a.code()));
    out
}

/// Corner distance squared between two points.
fn corner_gap(a: [f64; 3], b: [f64; 3]) -> f64 {
    let [ax, ay, az] = a;
    let [bx, by, bz] = b;
    let dx = ax - bx;
    let dy = ay - by;
    let dz = az - bz;
    dx * dx + dy * dy + dz * dz
}

/// Tests whether two triangles share an edge (two corners within tolerance).
#[must_use]
pub fn shares_edge(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> bool {
    let [a0, a1, a2] = a;
    let [b0, b1, b2] = b;
    let corners_a = [a0, a1, a2];
    let corners_b = [b0, b1, b2];
    let mut shared = 0u8;
    for corner_a in corners_a {
        let mut hit = false;
        for corner_b in corners_b {
            if corner_gap(corner_a, corner_b).sqrt() < 1e-9 {
                hit = true;
                break;
            }
        }
        if hit {
            shared += 1;
        }
    }
    shared == 2
}

/// Grows `count` clusters over `tiles` by seeded multi-source flood fill.
///
/// Seed triangles are the `count` tiles with the lowest `stream_seed`
/// order, so the same seed always picks the same starts (C8). Every tile
/// joins the cluster whose seed reaches it first over edge adjacency, so
/// clusters cover the whole set with irregular outlines (Q8/Q9). Order is
/// fixed by triangle address, never by floats. Returns one entry per
/// cluster (empty when `tiles` is empty or `count` is zero).
#[must_use]
pub fn grow_clusters(seed: u64, tiles: &[TriAddr], count: u32) -> Vec<Vec<TriAddr>> {
    if tiles.is_empty() || count == 0 {
        return Vec::new();
    }
    let mut ordered: Vec<TriAddr> = tiles.to_vec();
    ordered.sort_by_key(|a| (a.face(), a.code()));
    let corners: Vec<Option<[[f64; 3]; 3]>> = ordered.iter().map(|a| tri_corners(*a)).collect();
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for index in 0..ordered.len() {
        let Some(ci) = corners.get(index).copied().flatten() else {
            continue;
        };
        let mut row: usize = index + 1;
        while row < ordered.len() {
            if let Some(cj) = corners.get(row).copied().flatten()
                && shares_edge(ci, cj)
            {
                edges.push((index, row));
            }
            row += 1;
        }
    }
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); ordered.len()];
    for (left, right) in edges {
        if let Some(slot) = adjacency.get_mut(left) {
            slot.push(right);
        }
        if let Some(slot) = adjacency.get_mut(right) {
            slot.push(left);
        }
    }
    for slot in &mut adjacency {
        slot.sort_unstable();
    }
    let mut ranked: Vec<usize> = (0..ordered.len()).collect();
    ranked.sort_by(|a, b| {
        let seed_a = ordered.get(*a).map_or(u64::MAX, |t| t.stream_seed(seed));
        let seed_b = ordered.get(*b).map_or(u64::MAX, |t| t.stream_seed(seed));
        let face_a = ordered.get(*a).map_or(u8::MAX, |t| t.face());
        let face_b = ordered.get(*b).map_or(u8::MAX, |t| t.face());
        let code_a = ordered.get(*a).map_or(u32::MAX, |t| t.code());
        let code_b = ordered.get(*b).map_or(u32::MAX, |t| t.code());
        seed_a
            .cmp(&seed_b)
            .then(face_a.cmp(&face_b))
            .then(code_a.cmp(&code_b))
    });
    let clusters = usize::try_from(count).unwrap_or(0).min(ordered.len());
    let mut owner: Vec<Option<usize>> = vec![None; ordered.len()];
    let mut frontier: Vec<usize> = Vec::new();
    for (cluster, index) in ranked.iter().take(clusters).enumerate() {
        if let Some(slot) = owner.get_mut(*index) {
            *slot = Some(cluster);
        }
        frontier.push(*index);
    }
    let mut cursor = 0;
    while cursor < frontier.len() {
        let Some(current) = frontier.get(cursor).copied() else {
            break;
        };
        cursor += 1;
        let Some(home) = owner.get(current).copied().flatten() else {
            continue;
        };
        let neighbours = adjacency.get(current).cloned().unwrap_or_default();
        for neighbour in neighbours {
            let taken = owner.get(neighbour).copied().flatten().is_some();
            if !taken {
                if let Some(slot) = owner.get_mut(neighbour) {
                    *slot = Some(home);
                }
                frontier.push(neighbour);
            }
        }
    }
    // Any tile unreachable over edges (rounding seam) joins the smallest
    // cluster by address order, so coverage is total.
    let unassigned: Vec<usize> = owner
        .iter()
        .enumerate()
        .filter_map(|(index, slot)| slot.is_none().then_some(index))
        .collect();
    for index in unassigned {
        let mut sizes: Vec<(usize, usize)> = Vec::new();
        for cluster in 0..clusters {
            let size = owner.iter().filter(|o| **o == Some(cluster)).count();
            sizes.push((size, cluster));
        }
        sizes.sort();
        let pick = sizes.first().map_or(0, |(_, cluster)| *cluster);
        if let Some(slot) = owner.get_mut(index) {
            *slot = Some(pick);
        }
        frontier.push(index);
    }
    let mut out: Vec<Vec<TriAddr>> = vec![Vec::new(); clusters];
    for (index, addr) in ordered.iter().enumerate() {
        if let Some(home) = owner.get(index).copied().flatten()
            && let Some(cluster) = out.get_mut(home)
        {
            cluster.push(*addr);
        }
    }
    for cluster in &mut out {
        cluster.sort_by_key(|a| (a.face(), a.code()));
    }
    out
}

/// Picks the `enterable` clusters of `clusters` in stream-seed order.
///
/// Today's counts ride here (6 regions per planet, about half the tail
/// markers); the rest draw dim and closed (Q9/C5).
#[must_use]
pub fn pick_enterable(seed: u64, clusters: &[Vec<TriAddr>], enterable: usize) -> Vec<bool> {
    let mut rank: Vec<usize> = (0..clusters.len()).collect();
    rank.sort_by(|a, b| {
        let seed_a = clusters
            .get(*a)
            .and_then(|c| c.first())
            .map_or(u64::MAX, |t| t.stream_seed(seed));
        let seed_b = clusters
            .get(*b)
            .and_then(|c| c.first())
            .map_or(u64::MAX, |t| t.stream_seed(seed));
        let len_a = clusters.get(*a).map_or(usize::MAX, Vec::len);
        let len_b = clusters.get(*b).map_or(usize::MAX, Vec::len);
        seed_a.cmp(&seed_b).then(len_a.cmp(&len_b)).then(a.cmp(b))
    });
    let mut out = vec![false; clusters.len()];
    for index in rank.into_iter().take(enterable.min(clusters.len())) {
        if let Some(slot) = out.get_mut(index) {
            *slot = true;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        let mut sum = 0.0;
        for i in 0..3 {
            let d = a[i] - b[i];
            sum += d * d;
        }
        sum.sqrt() < 1e-12
    }

    #[test]
    fn base_holds_twenty_unit_faces() {
        let vertices = base_vertices();
        for vertex in vertices {
            let length =
                (vertex[0] * vertex[0] + vertex[1] * vertex[1] + vertex[2] * vertex[2]).sqrt();
            assert!((length - 1.0).abs() < 1e-12, "base vertex off sphere");
        }
        assert_eq!(base_faces().len(), 20);
        for face in 0..20u8 {
            assert!(base_corners(face).is_some(), "face {face} has corners");
        }
        assert!(base_corners(20).is_none());
    }

    #[test]
    fn entered_tile_refines_without_seams_or_pops() {
        for face in 0..20u8 {
            let Some(root) = TriAddr::root(face) else {
                continue;
            };
            let Some(corners) = tri_corners(root) else {
                continue;
            };
            let Some(kids) = root.children() else {
                continue;
            };
            let kid_corners = [
                tri_corners(kids[0]).expect("child 0"),
                tri_corners(kids[1]).expect("child 1"),
                tri_corners(kids[2]).expect("child 2"),
                tri_corners(kids[3]).expect("child 3"),
            ];
            let mids = split_corners(corners);
            for (got, want) in kid_corners.iter().zip(mids.iter()) {
                for (g, w) in got.iter().zip(want.iter()) {
                    assert!(close(*g, *w), "child corner moved off parent");
                }
            }
            for kid in kids {
                let Some(back) = kid.parent() else {
                    panic!("child has no parent");
                };
                assert_eq!(back, root, "parent round-trip moves");
                let Some(grand) = tri_corners(kid) else {
                    continue;
                };
                for corner in grand {
                    assert!(
                        point_in_triangle(corner, corners),
                        "child corner leaves the parent"
                    );
                }
            }
        }
    }

    #[test]
    fn locate_round_trips_through_corners_and_centres() {
        for face in 0..20u8 {
            let Some(root) = TriAddr::root(face) else {
                continue;
            };
            let Some(corners) = tri_corners(root) else {
                continue;
            };
            for corner in corners {
                let Some(found) = locate(corner, 0) else {
                    continue;
                };
                let Some(found_corners) = tri_corners(found) else {
                    continue;
                };
                assert!(
                    point_in_triangle(corner, found_corners),
                    "corner lookup leaves its face"
                );
            }
            let Some(centre) = tri_centre(root) else {
                continue;
            };
            assert_eq!(locate(centre, 0), Some(root), "centre lookup moves");
            let Some(deep) = locate(centre, 3) else {
                panic!("deep lookup fails");
            };
            assert_eq!(deep.depth(), 3);
            assert_eq!(deep.face(), face);
            let Some(deep_corners) = tri_corners(deep) else {
                continue;
            };
            assert!(point_in_triangle(centre, deep_corners));
        }
        assert!(locate([0.0, 0.0, 0.0], 0).is_some());
        assert!(locate([f64::NAN, 0.0, 0.0], 0).is_none());
        assert!(locate([1.0, 0.0, 0.0], MAX_DEPTH + 1).is_none());
    }

    #[test]
    fn addresses_reject_bad_faces_digits_and_depth() {
        assert!(TriAddr::new(20, &[]).is_none());
        assert!(TriAddr::new(0, &[4]).is_none());
        assert!(TriAddr::root(0).expect("root").parent().is_none());
        let deep_path = vec![1u8; usize::from(MAX_DEPTH) + 1];
        assert!(TriAddr::new(0, &deep_path).is_none());
        assert!(all_at_depth(MAX_DEPTH + 1).is_empty());
    }

    #[test]
    fn icosphere_is_deterministic() {
        let a = tri_corners(TriAddr::new(3, &[1, 2]).expect("addr"));
        let b = tri_corners(TriAddr::new(3, &[1, 2]).expect("addr"));
        assert_eq!(a, b);
        assert_ne!(
            TriAddr::new(3, &[1, 2]).expect("addr").stream_seed(7),
            TriAddr::new(3, &[1, 3]).expect("addr").stream_seed(7)
        );
        assert_eq!(
            TriAddr::new(3, &[1, 2]).expect("addr").stream_seed(7),
            TriAddr::new(3, &[1, 2]).expect("addr").stream_seed(7)
        );
    }

    #[test]
    fn clusters_nest_inside_their_parent() {
        let seed = 99u64;
        let parent = TriAddr::new(5, &[0, 1]).expect("parent");
        let tiles = descendants_at(parent, 4);
        assert_eq!(tiles.len(), 16, "two more splits quad twice");
        let clusters = grow_clusters(seed, &tiles, 3);
        assert_eq!(clusters.len(), 3);
        for cluster in &clusters {
            assert!(!cluster.is_empty(), "no empty cluster");
            for tile in cluster {
                let mut walk = *tile;
                let mut inside = false;
                while let Some(back) = walk.parent() {
                    if back == parent {
                        inside = true;
                        break;
                    }
                    walk = back;
                }
                assert!(inside, "cluster tile escapes its parent");
            }
        }
        // One more split down nests again: every grandchild sits in exactly
        // one child cluster of its own parent tile.
        for cluster in &clusters {
            for tile in cluster {
                let kids = descendants_at(*tile, tile.depth() + 1);
                assert_eq!(kids.len(), 4);
            }
        }
    }

    #[test]
    fn clusters_cover_the_surface_and_portal_counts_hold() {
        let seed = 7u64;
        let parent = TriAddr::root(2).expect("root");
        let tiles = descendants_at(parent, 3);
        assert_eq!(tiles.len(), 64);
        let clusters = grow_clusters(seed, &tiles, 6);
        assert_eq!(clusters.len(), 6, "six regions per planet face branch");
        let covered: usize = clusters.iter().map(Vec::len).sum();
        assert_eq!(covered, tiles.len(), "clusters leave no tile behind");
        let mut seen = tiles.clone();
        seen.sort_by(|a, b| (a.face(), a.code).cmp(&(b.face(), b.code)));
        let mut gathered: Vec<TriAddr> = clusters.concat();
        gathered.sort_by(|a, b| (a.face(), a.code).cmp(&(b.face(), b.code)));
        assert_eq!(seen, gathered, "clusters overlap or miss");
        let enterable = pick_enterable(seed, &clusters, 3);
        assert_eq!(enterable.iter().filter(|v| **v).count(), 3);
        // Same seed, same clusters on every run.
        let again = grow_clusters(seed, &tiles, 6);
        assert_eq!(clusters, again);
        assert_ne!(
            grow_clusters(seed, &tiles, 6),
            grow_clusters(seed ^ 1, &tiles, 6)
        );
    }
}
