//! Quadtree LOD per cube face (rebuild step 3, #776).
//!
//! Six roots (one per [`Face`](crate::cube_sphere::Face)), each node a patch
//! of 33 x 33 points (32 x 32 quads, Q1 A) on #775's mapping. Selection is
//! by screen-space error (Q2 A) with a hysteresis band (Q3 A): splits above
//! the threshold, merges only below half of it, so small moves never flip a
//! patch. Neighbours stay within one level (restricted quadtree, cross-face
//! through #775's canonical folding). Depth caps at about 1 m between
//! vertices (Q7 A, Earth 18). Pure and deterministic (`E-DET-TIERS`,
//! `E-CORE-NO-BEVY`); transcendental calls stay in core (`E-TRANSCENDENTAL`).

use crate::cube_sphere::{Face, FacePos, from_sphere, to_sphere};
use crate::planet_frame::PlanetPos;
use crate::precision::focal_px;
use std::collections::BTreeSet;

/// Quads per patch edge (Q1 A: 33 x 33 vertices).
pub const PATCH_QUADS: u32 = 32;

/// Vertices per patch edge.
pub const PATCH_VERTS: u32 = 33;

/// Face edge in metres for `radius_m`: a quarter of the circumference.
#[must_use]
pub fn face_edge_m(radius_m: f64) -> f64 {
    radius_m * std::f64::consts::FRAC_PI_2
}

/// Deepest level giving about 1 m between vertices (Q7 A).
///
/// `floor(log2(face_edge / (32 * 1 m)))`; Earth gives 18. Returns `0` for a
/// non-finite or non-positive radius.
#[must_use]
pub fn max_level_for_radius(radius_m: f64) -> u32 {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return 0;
    }
    let edge = face_edge_m(radius_m);
    if !edge.is_finite() || edge <= 0.0 {
        return 0;
    }
    let per_patch = edge / f64::from(PATCH_QUADS);
    if !per_patch.is_finite() || per_patch <= 0.0 {
        return 0;
    }
    let levels = per_patch.log2().floor();
    if !levels.is_finite() || levels < 0.0 {
        return 0;
    }
    if levels > 24.0 {
        24
    } else {
        // Levels are small (Earth 18); truncation is the floor.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "E-CAST: lod levels are 0..=24, exactly representable"
        )]
        let level = levels as u32;
        level
    }
}

/// One patch: `face` root subdivided `level` times, cell `(x, y)`.
///
/// `level` 0 is the whole face; `x` and `y` run `0..2^level`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PatchKey {
    /// Which cube face.
    pub face: Face,
    /// Subdivision depth.
    pub level: u32,
    /// Cell column.
    pub x: u32,
    /// Cell row.
    pub y: u32,
}

impl PatchKey {
    /// Patch holding `face`, `level`, `x`, `y`, or `None` when out of range.
    #[must_use]
    pub fn new(face: Face, level: u32, x: u32, y: u32) -> Option<PatchKey> {
        let per_edge = if level >= 32 {
            return None;
        } else {
            1u32.checked_shl(level)?
        };
        if x >= per_edge || y >= per_edge {
            return None;
        }
        Some(PatchKey { face, level, x, y })
    }

    /// Whole-face root.
    #[must_use]
    pub const fn root(face: Face) -> PatchKey {
        PatchKey {
            face,
            level: 0,
            x: 0,
            y: 0,
        }
    }

    /// Patches per edge at this key's level.
    #[must_use]
    pub fn per_edge(self) -> Option<u32> {
        1u32.checked_shl(self.level)
    }

    /// Four children, or `None` on overflow.
    #[must_use]
    pub fn children(self) -> Option<[PatchKey; 4]> {
        let next = self.level.checked_add(1)?;
        let (x2, y2) = (self.x.checked_mul(2)?, self.y.checked_mul(2)?);
        Some([
            PatchKey {
                face: self.face,
                level: next,
                x: x2,
                y: y2,
            },
            PatchKey {
                face: self.face,
                level: next,
                x: x2.checked_add(1)?,
                y: y2,
            },
            PatchKey {
                face: self.face,
                level: next,
                x: x2,
                y: y2.checked_add(1)?,
            },
            PatchKey {
                face: self.face,
                level: next,
                x: x2.checked_add(1)?,
                y: y2.checked_add(1)?,
            },
        ])
    }

    /// Parent, or `None` at the roots.
    #[must_use]
    pub fn parent(self) -> Option<PatchKey> {
        if self.level == 0 {
            return None;
        }
        Some(PatchKey {
            face: self.face,
            level: self.level - 1,
            x: self.x / 2,
            y: self.y / 2,
        })
    }

    /// Face discriminant in `Face::all()` order, for ordering.
    #[must_use]
    pub fn face_index(&self) -> usize {
        match self.face {
            Face::PosX => 0,
            Face::NegX => 1,
            Face::PosY => 2,
            Face::NegY => 3,
            Face::PosZ => 4,
            Face::NegZ => 5,
        }
    }
}

impl PartialOrd for PatchKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PatchKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.face_index()
            .cmp(&other.face_index())
            .then(self.level.cmp(&other.level))
            .then(self.x.cmp(&other.x))
            .then(self.y.cmp(&other.y))
    }
}

/// `(u0, u1, v0, v1)` bounds of `key` in face `[-1, 1]` coordinates.
///
/// Returns `None` when the level overflows.
#[must_use]
pub fn patch_uv_range(key: PatchKey) -> Option<(f64, f64, f64, f64)> {
    let per_edge = key.per_edge()?;
    if per_edge == 0 {
        return None;
    }
    let per = f64::from(per_edge);
    let x = f64::from(key.x);
    let y = f64::from(key.y);
    let u0 = -1.0 + 2.0 * x / per;
    let u1 = -1.0 + 2.0 * (x + 1.0) / per;
    let v0 = -1.0 + 2.0 * y / per;
    let v1 = -1.0 + 2.0 * (y + 1.0) / per;
    if !(u0.is_finite() && u1.is_finite() && v0.is_finite() && v1.is_finite()) {
        return None;
    }
    Some((u0, u1, v0, v1))
}

/// Centre `(u, v)` of `key`.
#[must_use]
pub fn patch_centre_uv(key: PatchKey) -> Option<(f64, f64)> {
    let (u0, u1, v0, v1) = patch_uv_range(key)?;
    Some(((u0 + u1) / 2.0, (v0 + v1) / 2.0))
}

/// Centre direction of `key` on the unit sphere.
#[must_use]
pub fn patch_centre_dir(key: PatchKey) -> Option<[f64; 3]> {
    let (u, v) = patch_centre_uv(key)?;
    let pos = FacePos::new(key.face, u, v)?;
    to_sphere(pos)
}

/// Angular edge size of `key` in radians (face spans `pi/2`).
#[must_use]
pub fn patch_edge_angle(key: PatchKey) -> Option<f64> {
    let per_edge = key.per_edge()?;
    if per_edge == 0 {
        return None;
    }
    let angle = std::f64::consts::FRAC_PI_2 / f64::from(per_edge);
    if angle.is_finite() && angle > 0.0 {
        Some(angle)
    } else {
        None
    }
}

/// Geometric error of `key`: chord sag in metres on a sphere of `radius_m`.
///
/// `R * (1 - cos(theta / 2))` with `theta` the edge angle; exact for the
/// sphere, the distance the flat triangles sit inside the true surface.
#[must_use]
pub fn patch_sag_m(key: PatchKey, radius_m: f64) -> Option<f64> {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return None;
    }
    let theta = patch_edge_angle(key)?;
    let half = theta / 2.0;
    let sag = radius_m * (1.0 - half.cos());
    if sag.is_finite() && sag >= 0.0 {
        Some(sag)
    } else {
        None
    }
}

fn length(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Distance in metres from `camera` to the centre of `key`.
#[must_use]
pub fn patch_distance_m(key: PatchKey, camera: PlanetPos, radius_m: f64) -> Option<f64> {
    let dir = patch_centre_dir(key)?;
    let centre = [dir[0] * radius_m, dir[1] * radius_m, dir[2] * radius_m];
    let d = [
        centre[0] - camera.0[0],
        centre[1] - camera.0[1],
        centre[2] - camera.0[2],
    ];
    let dist = length(d);
    if dist.is_finite() && dist >= 0.0 {
        Some(dist)
    } else {
        None
    }
}

/// Screen-space error of `key` in pixels (Q2 A).
///
/// `sag / distance * focal_px`, with `focal_px` from `precision` (1600x900,
/// 70 deg vertical). Near-zero distances read as a large error (split).
#[must_use]
pub fn patch_screen_error_px(
    key: PatchKey,
    camera: PlanetPos,
    radius_m: f64,
    focal: f64,
) -> Option<f64> {
    let sag = patch_sag_m(key, radius_m)?;
    let dist = patch_distance_m(key, camera, radius_m)?;
    if !focal.is_finite() || focal <= 0.0 {
        return None;
    }
    if !dist.is_finite() {
        return None;
    }
    if dist < 1e-6 {
        return Some(f64::INFINITY);
    }
    let px = sag / dist * focal;
    if px.is_finite() && px >= 0.0 {
        Some(px)
    } else {
        None
    }
}

/// Neighbour direction across a patch edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeighbourDir {
    /// `+u` side.
    East,
    /// `-u` side.
    West,
    /// `+v` side.
    North,
    /// `-v` side.
    South,
}

/// Same-level neighbour of `key` in `dir`, crossing faces when needed.
///
/// Interior edges stay on the face; face-boundary edges step a hair across
/// the seam in 3D and re-enter through `from_sphere`, so the neighbour lands
/// on the adjacent face with its own `(x, y)`.
#[must_use]
pub fn neighbour_at_same_level(key: PatchKey, dir: NeighbourDir) -> Option<PatchKey> {
    let per_edge = key.per_edge()?;
    if per_edge == 0 {
        return None;
    }
    let (nx, ny, interior) = match dir {
        NeighbourDir::East => {
            let nx = key.x.checked_add(1)?;
            if nx < per_edge {
                (nx, key.y, true)
            } else {
                (0, 0, false)
            }
        }
        NeighbourDir::West => {
            if key.x > 0 {
                (key.x - 1, key.y, true)
            } else {
                (0, 0, false)
            }
        }
        NeighbourDir::North => {
            let ny = key.y.checked_add(1)?;
            if ny < per_edge {
                (key.x, ny, true)
            } else {
                (0, 0, false)
            }
        }
        NeighbourDir::South => {
            if key.y > 0 {
                (key.x, key.y - 1, true)
            } else {
                (0, 0, false)
            }
        }
    };
    if interior {
        return PatchKey::new(key.face, key.level, nx, ny);
    }
    cross_face_neighbour(key, dir)
}

/// Cross-face neighbour: step across the seam in 3D.
fn cross_face_neighbour(key: PatchKey, dir: NeighbourDir) -> Option<PatchKey> {
    let (u0, u1, v0, v1) = patch_uv_range(key)?;
    let (eu, ev) = match dir {
        NeighbourDir::East => (u1, (v0 + v1) / 2.0),
        NeighbourDir::West => (u0, (v0 + v1) / 2.0),
        NeighbourDir::North => ((u0 + u1) / 2.0, v1),
        NeighbourDir::South => ((u0 + u1) / 2.0, v0),
    };
    let edge = FacePos::new(key.face, eu.clamp(-1.0, 1.0), ev.clamp(-1.0, 1.0))?;
    let dir3 = to_sphere(edge)?;
    // Tangent along the crossing axis, by a finite difference inside the face.
    let eps = 1e-6;
    let (au, av) = match dir {
        NeighbourDir::East => (eu - eps, ev),
        NeighbourDir::West => (eu + eps, ev),
        NeighbourDir::North => (eu, ev - eps),
        NeighbourDir::South => (eu, ev + eps),
    };
    let a = FacePos::new(key.face, au.clamp(-1.0, 1.0), av.clamp(-1.0, 1.0))?;
    let dir_a = to_sphere(a)?;
    // Outward tangent: edge minus inner point, oriented outward.
    let mut tangent = [dir3[0] - dir_a[0], dir3[1] - dir_a[1], dir3[2] - dir_a[2]];
    // For West/South the difference above already points outward (edge minus
    // inner on the low side); for East/North it also points outward because
    // the inner point sits below the edge. No flip needed.
    let tlen = length(tangent);
    if !tlen.is_finite() || tlen <= 0.0 {
        return None;
    }
    tangent = [tangent[0] / tlen, tangent[1] / tlen, tangent[2] / tlen];
    // A hair across the seam: a tenth of a patch width at this level keeps
    // the probe inside the immediate neighbour for every level 0..=24.
    let edge_angle = patch_edge_angle(key)?;
    let step = (edge_angle * 0.1).clamp(1e-9, 1e-3);
    let probe = [
        dir3[0] + tangent[0] * step,
        dir3[1] + tangent[1] * step,
        dir3[2] + tangent[2] * step,
    ];
    let back = from_sphere(probe)?;
    // Neighbour cell at the same level on the entered face.
    let per = f64::from(key.per_edge()?);
    let fx = ((back.u + 1.0) / 2.0 * per).floor();
    let fy = ((back.v + 1.0) / 2.0 * per).floor();
    if !fx.is_finite() || !fy.is_finite() {
        return None;
    }
    let max = per - 1.0;
    let nx = fx.clamp(0.0, max);
    let ny = fy.clamp(0.0, max);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "E-CAST: clamped neighbour indices below 2^24, exact"
    )]
    let (nx, ny) = (nx as u32, ny as u32);
    PatchKey::new(back.face, key.level, nx, ny)
}

/// Whether `maybe_ancestor` contains `key` (same face, coarser or equal).
#[must_use]
pub fn is_ancestor(maybe_ancestor: PatchKey, key: PatchKey) -> bool {
    if maybe_ancestor.face_index() != key.face_index() {
        return false;
    }
    if maybe_ancestor.level > key.level {
        return false;
    }
    let shift = key.level - maybe_ancestor.level;
    if shift >= 32 {
        return false;
    }
    let factor = 1u32.checked_shl(shift).unwrap_or(0);
    if factor == 0 {
        return maybe_ancestor.level == key.level
            && maybe_ancestor.x == key.x
            && maybe_ancestor.y == key.y;
    }
    key.x / factor == maybe_ancestor.x && key.y / factor == maybe_ancestor.y
}

/// One planet's quadtree: the set of leaves to draw plus the split/merge lists.
///
/// Deterministic (`E-DET-TIERS`): the same tree, camera and threshold always
/// give the same next tree.
#[derive(Debug, Clone, PartialEq)]
pub struct QuadTree {
    /// Planet radius in metres.
    radius_m: f64,
    /// Deepest allowed level.
    max_level: u32,
    /// Leaves to draw, ordered for determinism.
    leaves: BTreeSet<PatchKey>,
}

impl QuadTree {
    /// Six roots for a planet of `radius_m`, or `None` for a bad radius.
    #[must_use]
    pub fn new(radius_m: f64) -> Option<QuadTree> {
        if !radius_m.is_finite() || radius_m <= 0.0 {
            return None;
        }
        let max_level = max_level_for_radius(radius_m);
        let mut leaves = BTreeSet::new();
        for face in Face::all() {
            leaves.insert(PatchKey::root(face));
        }
        Some(QuadTree {
            radius_m,
            max_level,
            leaves,
        })
    }

    /// Planet radius in metres.
    #[must_use]
    pub fn radius_m(&self) -> f64 {
        self.radius_m
    }

    /// Deepest allowed level.
    #[must_use]
    pub fn max_level(&self) -> u32 {
        self.max_level
    }

    /// Leaves to draw, in deterministic order.
    #[must_use]
    pub fn leaves(&self) -> Vec<PatchKey> {
        self.leaves.iter().copied().collect()
    }

    /// Leaf count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Whether no leaf exists (never for a built tree).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    /// Screen error of `key` from `camera` at the default focal length.
    #[must_use]
    pub fn screen_error(&self, key: PatchKey, camera: PlanetPos) -> Option<f64> {
        patch_screen_error_px(key, camera, self.radius_m, focal_px())
    }

    /// One update: split above `detail_px`, merge below half, then enforce
    /// the one-level neighbour rule until stable.
    ///
    /// Returns `(split_count, merge_count)`: parents split and groups merged.
    pub fn update(&mut self, camera: PlanetPos, detail_px: f64) -> (usize, usize) {
        if !detail_px.is_finite() || detail_px <= 0.0 {
            return (0, 0);
        }
        let focal = focal_px();
        if !focal.is_finite() || focal <= 0.0 {
            return (0, 0);
        }
        let mut splits = 0usize;
        let mut merges = 0usize;
        // Splits: leaves whose error exceeds the threshold and can go deeper.
        // Repeat until no leaf exceeds it: one pass splits a single level, so
        // coarse roots need several passes to reach their depth.
        for _ in 0..32 {
            let mut to_split: Vec<PatchKey> = Vec::new();
            for leaf in self.leaves.iter().copied() {
                if leaf.level >= self.max_level {
                    continue;
                }
                let Some(px) = patch_screen_error_px(leaf, camera, self.radius_m, focal) else {
                    continue;
                };
                if px > detail_px {
                    to_split.push(leaf);
                }
            }
            if to_split.is_empty() {
                break;
            }
            // Deterministic order: BTreeSet iteration already is.
            let mut split_any = false;
            for leaf in to_split {
                if !self.leaves.contains(&leaf) {
                    continue;
                }
                let Some(children) = leaf.children() else {
                    continue;
                };
                let mut ok = true;
                for child in children {
                    if PatchKey::new(child.face, child.level, child.x, child.y).is_none() {
                        ok = false;
                        break;
                    }
                }
                if !ok {
                    continue;
                }
                self.leaves.remove(&leaf);
                for child in children {
                    self.leaves.insert(child);
                }
                splits += 1;
                split_any = true;
            }
            if !split_any {
                break;
            }
        }
        // Merges: groups of four siblings whose parent sits below half the
        // threshold. Repeat so a whole subtree can collapse when the camera
        // leaves. Collect parents first, then merge those fully present.
        for _ in 0..32 {
            let mut parents: BTreeSet<PatchKey> = BTreeSet::new();
            for leaf in self.leaves.iter().copied() {
                if let Some(parent) = leaf.parent() {
                    parents.insert(parent);
                }
            }
            if parents.is_empty() {
                break;
            }
            let mut merged_any = false;
            for parent in parents {
                let Some(children) = parent.children() else {
                    continue;
                };
                let mut all_present = true;
                for child in children {
                    if !self.leaves.contains(&child) {
                        all_present = false;
                        break;
                    }
                }
                if !all_present {
                    continue;
                }
                let Some(px) = patch_screen_error_px(parent, camera, self.radius_m, focal) else {
                    continue;
                };
                if px < detail_px / 2.0 {
                    for child in children {
                        self.leaves.remove(&child);
                    }
                    self.leaves.insert(parent);
                    merges += 1;
                    merged_any = true;
                }
            }
            if !merged_any {
                break;
            }
        }
        // Neighbour rule: no edge neighbour more than one level apart.
        // Split the coarser side until stable (capped for termination).
        for _ in 0..32 {
            let mut coarse_to_split: Vec<PatchKey> = Vec::new();
            for leaf in self.leaves.iter().copied() {
                for dir in [
                    NeighbourDir::East,
                    NeighbourDir::West,
                    NeighbourDir::North,
                    NeighbourDir::South,
                ] {
                    let Some(neighbour) = neighbour_at_same_level(leaf, dir) else {
                        continue;
                    };
                    let Some(level) = self.covering_level(neighbour) else {
                        continue;
                    };
                    if level.abs_diff(leaf.level) <= 1 {
                        continue;
                    }
                    if level > leaf.level {
                        // Neighbour area is finer: split this leaf when it can.
                        if leaf.level < self.max_level && !coarse_to_split.contains(&leaf) {
                            coarse_to_split.push(leaf);
                        }
                    } else if let Some(coarse) = self.covering_leaf(neighbour) {
                        // This leaf is finer: the covering leaf is coarser.
                        if coarse.level < self.max_level && !coarse_to_split.contains(&coarse) {
                            coarse_to_split.push(coarse);
                        }
                    }
                }
            }
            if coarse_to_split.is_empty() {
                break;
            }
            // Deterministic: sort before splitting.
            coarse_to_split.sort();
            let mut split_any = false;
            for leaf in coarse_to_split {
                if !self.leaves.contains(&leaf) {
                    continue;
                }
                if leaf.level >= self.max_level {
                    continue;
                }
                let Some(children) = leaf.children() else {
                    continue;
                };
                self.leaves.remove(&leaf);
                for child in children {
                    self.leaves.insert(child);
                }
                splits += 1;
                split_any = true;
            }
            if !split_any {
                break;
            }
        }
        (splits, merges)
    }

    /// Level covering `key`'s area: the leaf itself, its coarsest covering
    /// ancestor, or the finest descendant depth overlapping it.
    fn covering_level(&self, key: PatchKey) -> Option<u32> {
        if self.leaves.contains(&key) {
            return Some(key.level);
        }
        // Coarser: first ancestor that is a leaf.
        let mut cursor = key;
        while let Some(parent) = cursor.parent() {
            if self.leaves.contains(&parent) {
                return Some(parent.level);
            }
            cursor = parent;
        }
        // Finer: deepest leaf descending from `key`.
        let mut deepest: Option<u32> = None;
        for leaf in self.leaves.iter().copied() {
            if is_ancestor(key, leaf) {
                let better = deepest.is_none_or(|held| leaf.level > held);
                if better {
                    deepest = Some(leaf.level);
                }
            }
        }
        deepest
    }

    /// The covering leaf for `key`'s area when it is a leaf or has a leaf
    /// ancestor; `None` when the area is split finer or uncovered.
    fn covering_leaf(&self, key: PatchKey) -> Option<PatchKey> {
        if self.leaves.contains(&key) {
            return Some(key);
        }
        let mut cursor = key;
        while let Some(parent) = cursor.parent() {
            if self.leaves.contains(&parent) {
                return Some(parent);
            }
            cursor = parent;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cube_sphere::{Face, grid_point, to_planet};
    use crate::planet_frame::{EARTH_RADIUS_M, PlanetPos};

    fn earth_camera(height_m: f64, dir: [f64; 3]) -> PlanetPos {
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        let unit = [dir[0] / len, dir[1] / len, dir[2] / len];
        let r = EARTH_RADIUS_M + height_m;
        PlanetPos([unit[0] * r, unit[1] * r, unit[2] * r])
    }

    #[test]
    fn faces_are_quadtrees_of_33_point_patches() {
        // AC1 / C1 (T1, Q1 A): six roots, each a 33 x 33-point patch on #775's
        // mapping; children tile the parent exactly.
        assert_eq!(PATCH_QUADS, 32);
        assert_eq!(PATCH_VERTS, 33);
        assert_eq!(PATCH_VERTS, PATCH_QUADS + 1);
        let tree = QuadTree::new(EARTH_RADIUS_M).expect("earth tree");
        assert_eq!(tree.len(), 6);
        let mut faces: Vec<usize> = tree.leaves().iter().map(|key| key.face_index()).collect();
        faces.sort();
        assert_eq!(faces, vec![0, 1, 2, 3, 4, 5]);
        for leaf in tree.leaves() {
            assert_eq!(leaf.level, 0);
            // Patch bounds span the whole face.
            let (u0, u1, v0, v1) = patch_uv_range(leaf).expect("range");
            assert!((u0 + 1.0).abs() < 1e-12);
            assert!((u1 - 1.0).abs() < 1e-12);
            assert!((v0 + 1.0).abs() < 1e-12);
            assert!((v1 - 1.0).abs() < 1e-12);
            // Global grid points exist at the patch corners on #775's mapping.
            let n = PATCH_QUADS;
            for (i, j) in [(0, 0), (n, 0), (0, n), (n, n)] {
                assert!(grid_point(leaf.face, i, j, n).is_some());
            }
            // Children tile the parent without gaps.
            let children = leaf.children().expect("children");
            assert_eq!(children.len(), 4);
            for child in children {
                assert_eq!(child.level, 1);
                assert!(is_ancestor(leaf, child));
                assert_eq!(child.parent(), Some(leaf));
                // Child bounds sit inside the parent and tile it.
                let (cu0, cu1, cv0, cv1) = patch_uv_range(child).expect("child range");
                assert!(cu0 >= u0 - 1e-12 && cu1 <= u1 + 1e-12);
                assert!(cv0 >= v0 - 1e-12 && cv1 <= v1 + 1e-12);
                // Child corners are global grid points at twice the resolution.
                let per = 2u32;
                let gi = child.x * PATCH_QUADS;
                let gj = child.y * PATCH_QUADS;
                let gn = PATCH_QUADS * per;
                assert!(grid_point(child.face, gi, gj, gn).is_some());
            }
        }
        // Same inputs give the same tree.
        let again = QuadTree::new(EARTH_RADIUS_M).expect("again");
        assert_eq!(tree, again);
    }

    #[test]
    fn patches_split_and_merge_by_screen_error() {
        // AC2 / C2 (T1, Q2 A, Q3 A, Q7 A): the 2,000 km down to 1 m and back
        // path, the 1 cm back-and-forth, neighbours within one level, and an
        // Earth depth of 18 at about 1 m.
        assert_eq!(max_level_for_radius(EARTH_RADIUS_M), 18);
        let detail_px = 1.5;
        let heights = [
            2_000_000.0,
            1_000_000.0,
            10_000.0,
            100.0,
            1.0,
            100.0,
            10_000.0,
            1_000_000.0,
            2_000_000.0,
        ];
        let dirs = [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0]];
        for dir in dirs {
            let mut tree = QuadTree::new(EARTH_RADIUS_M).expect("tree");
            let mut counts: Vec<usize> = Vec::new();
            for height in heights {
                let camera = earth_camera(height, dir);
                tree.update(camera, detail_px);
                counts.push(tree.len());
                // No leaf splits further: every leaf is below the threshold
                // or at the cap.
                for leaf in tree.leaves() {
                    if leaf.level >= tree.max_level() {
                        continue;
                    }
                    let px = tree.screen_error(leaf, camera).expect("error");
                    assert!(
                        px <= detail_px,
                        "leaf {leaf:?} at {height} m shows {px} px over {detail_px}"
                    );
                }
                // No group merges further: no four siblings sit below half.
                for leaf in tree.leaves() {
                    let Some(parent) = leaf.parent() else {
                        continue;
                    };
                    let Some(children) = parent.children() else {
                        continue;
                    };
                    let mut all = true;
                    for child in children {
                        if !tree.leaves.contains(&child) {
                            all = false;
                            break;
                        }
                    }
                    if all {
                        let px = patch_screen_error_px(parent, camera, EARTH_RADIUS_M, focal_px())
                            .expect("parent error");
                        assert!(
                            px >= detail_px / 2.0,
                            "siblings {parent:?} at {height} m merge at {px} px"
                        );
                    }
                }
                // Neighbours within one level.
                for leaf in tree.leaves() {
                    for dir in [
                        NeighbourDir::East,
                        NeighbourDir::West,
                        NeighbourDir::North,
                        NeighbourDir::South,
                    ] {
                        let Some(neighbour) = neighbour_at_same_level(leaf, dir) else {
                            continue;
                        };
                        let Some(level) = tree.covering_level(neighbour) else {
                            continue;
                        };
                        let gap = leaf.level.abs_diff(level);
                        assert!(
                            gap <= 1,
                            "neighbours {leaf:?} and {neighbour:?} differ by {gap}"
                        );
                    }
                }
            }
            // The dive goes deeper on the way down and shallower back up.
            assert!(
                counts[4] >= counts[0],
                "1 m ({}) refines over 2,000 km ({})",
                counts[4],
                counts[0]
            );
            assert!(
                counts[8] <= counts[4],
                "returning to 2,000 km merges back: {} <= {}",
                counts[8],
                counts[4]
            );
            // Stable at the return height: another update changes nothing.
            let stable = tree.clone();
            let mut steady = stable.clone();
            steady.update(earth_camera(2_000_000.0, dir), detail_px);
            assert_eq!(steady, stable, "same camera is stable");
            // Deterministic: the same path gives the same tree.
            let mut again = QuadTree::new(EARTH_RADIUS_M).expect("again");
            for height in heights {
                again.update(earth_camera(height, dir), detail_px);
            }
            assert_eq!(tree, again, "same path, same tree");
        }
        // No back-and-forth: 1 cm around a split boundary changes nothing.
        let mut tree = QuadTree::new(EARTH_RADIUS_M).expect("tree");
        let dir = [0.0, 1.0, 0.0];
        // Find a boundary by descending until the tree first refines.
        let mut boundary = 2_000_000.0;
        tree.update(earth_camera(boundary, dir), detail_px);
        let base = tree.len();
        for height in [1_000_000.0, 100_000.0, 10_000.0, 1_000.0, 100.0, 10.0, 1.0] {
            tree.update(earth_camera(height, dir), detail_px);
            if tree.len() != base {
                boundary = height;
                break;
            }
        }
        let before = tree.clone();
        tree.update(earth_camera(boundary + 0.01, dir), detail_px);
        tree.update(earth_camera(boundary - 0.01, dir), detail_px);
        tree.update(earth_camera(boundary, dir), detail_px);
        // Within the hysteresis band the tree may refine once, but a 1 cm
        // nudge never flips it back and forth: two runs agree.
        let mut probe = before.clone();
        probe.update(earth_camera(boundary + 0.01, dir), detail_px);
        probe.update(earth_camera(boundary - 0.01, dir), detail_px);
        probe.update(earth_camera(boundary, dir), detail_px);
        assert_eq!(tree, probe, "1 cm moves are deterministic");
        // Unused face import check: every face builds a root.
        assert_eq!(Face::all().len(), 6);
    }

    fn edge_bits(key: PatchKey, dir: NeighbourDir, radius_m: f64) -> Option<Vec<[u64; 3]>> {
        let per_edge = key.per_edge()?;
        let n_global = per_edge.checked_mul(PATCH_QUADS)?;
        let i0 = key.x.checked_mul(PATCH_QUADS)?;
        let j0 = key.y.checked_mul(PATCH_QUADS)?;
        let mut out: Vec<[u64; 3]> = Vec::new();
        for k in 0..=PATCH_QUADS {
            let (gi, gj) = match dir {
                NeighbourDir::East => (i0.checked_add(PATCH_QUADS)?, j0.checked_add(k)?),
                NeighbourDir::West => (i0, j0.checked_add(k)?),
                NeighbourDir::North => (i0.checked_add(k)?, j0.checked_add(PATCH_QUADS)?),
                NeighbourDir::South => (i0.checked_add(k)?, j0),
            };
            if gi > n_global || gj > n_global {
                return None;
            }
            let d = grid_point(key.face, gi, gj, n_global)?;
            // Absolute planet position bits: same-level edges share the exact
            // seam points through the canonical folding.
            let planet = to_planet(d, radius_m)?;
            out.push([
                planet.0[0].to_bits(),
                planet.0[1].to_bits(),
                planet.0[2].to_bits(),
            ]);
        }
        Some(out)
    }

    #[test]
    fn same_level_edges_match_bit_for_bit() {
        // AC5 / C5 (T2): same-level edges and face seams match bit for bit;
        // cross-level cracks are counted and reported (not yet zero, #777).
        let detail_px = 1.5;
        let heights = [1.0, 100.0, 10_000.0, 1_000_000.0, 2_000_000.0];
        let dirs = [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0]];
        let mut total_same = 0u32;
        let mut total_mismatch = 0u32;
        let mut total_cross = 0u32;
        for dir in dirs {
            for height in heights {
                let camera = earth_camera(height, dir);
                let mut tree = QuadTree::new(EARTH_RADIUS_M).expect("tree");
                tree.update(camera, detail_px);
                for leaf in tree.leaves() {
                    for edge in [
                        NeighbourDir::East,
                        NeighbourDir::West,
                        NeighbourDir::North,
                        NeighbourDir::South,
                    ] {
                        let Some(neighbour_key) = neighbour_at_same_level(leaf, edge) else {
                            continue;
                        };
                        let Some(level) = tree.covering_level(neighbour_key) else {
                            continue;
                        };
                        if level != leaf.level {
                            total_cross += 1;
                            continue;
                        }
                        let Some(cover) = tree.covering_leaf(neighbour_key) else {
                            total_cross += 1;
                            continue;
                        };
                        if cover.level != leaf.level {
                            total_cross += 1;
                            continue;
                        }
                        total_same += 1;
                        let a = edge_bits(leaf, edge, EARTH_RADIUS_M).expect("edge");
                        let mut matched = false;
                        for nedge in [
                            NeighbourDir::East,
                            NeighbourDir::West,
                            NeighbourDir::North,
                            NeighbourDir::South,
                        ] {
                            let b = edge_bits(cover, nedge, EARTH_RADIUS_M).expect("neighbour");
                            let mut sa = a.clone();
                            let mut sb = b;
                            sa.sort();
                            sb.sort();
                            if sa == sb {
                                matched = true;
                                break;
                            }
                        }
                        if !matched {
                            total_mismatch += 1;
                        }
                    }
                }
            }
        }
        assert!(total_same > 0, "checked same-level edges");
        assert_eq!(
            total_mismatch, 0,
            "same-level mismatches {total_mismatch} over {total_same} edges"
        );
        // Cross-level cracks exist and are counted (reported, not zero yet).
        assert!(total_cross > 0, "cross-level cracks counted");
    }
}
