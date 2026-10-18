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

/// Scratch buffers for [`QuadTree::update_reused`], reused across frames
/// (`E-HOT-NOALLOC`): cleared and refilled in place, so steady-state frames
/// allocate nothing.
#[derive(Debug, Default)]
pub struct UpdateScratch {
    /// Leaves to split this pass.
    pub split: Vec<PatchKey>,
    /// Candidate parents to merge.
    pub parents: Vec<PatchKey>,
    /// Coarse leaves split by the neighbour rule.
    pub coarse: Vec<PatchKey>,
}

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
/// This is the whole-patch sag (kept for reference); selection uses the
/// drawn quad's sag below.
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

/// Geometric error of one drawn quad: the patch is drawn as 32 x 32 quads
/// that follow the sphere, so the drawn error is the sag of one quad,
/// about 1000 times smaller than the whole-patch sag above.
#[must_use]
pub fn patch_quad_sag_m(key: PatchKey, radius_m: f64) -> Option<f64> {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return None;
    }
    let theta = patch_edge_angle(key)?;
    let quad = theta / f64::from(PATCH_QUADS);
    if !quad.is_finite() || quad <= 0.0 {
        return None;
    }
    let half = quad / 2.0;
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

/// Bounding radius of `key` in metres: the farthest corner from the centre.
///
/// The patch's four corners come from the same canonical grid as the build,
/// so the radius is exact for the sphere. Used for the nearest-point distance
/// below.
#[must_use]
pub fn patch_bounding_radius_m(key: PatchKey, radius_m: f64) -> Option<f64> {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return None;
    }
    let (u0, u1, v0, v1) = patch_uv_range(key)?;
    let centre_dir = patch_centre_dir(key)?;
    let mut worst = 0.0f64;
    for (u, v) in [(u0, v0), (u1, v0), (u0, v1), (u1, v1)] {
        let pos = FacePos::new(key.face, u, v)?;
        let dir = to_sphere(pos)?;
        let chord = length([
            dir[0] - centre_dir[0],
            dir[1] - centre_dir[1],
            dir[2] - centre_dir[2],
        ]) * radius_m;
        if !chord.is_finite() {
            return None;
        }
        if chord > worst {
            worst = chord;
        }
    }
    Some(worst)
}

/// Distance in metres from `camera` to the nearest point of `key`.
///
/// Centre distance minus the bounding radius (the bounding sphere), clamped to
/// a hair above zero; inside the sphere reads as a large error (split). This
/// is the plan's "nearest point" rule: a close camera sees a large error even
/// when the centre is a patch-width away.
#[must_use]
pub fn patch_nearest_distance_m(key: PatchKey, camera: PlanetPos, radius_m: f64) -> Option<f64> {
    let centre = patch_distance_m(key, camera, radius_m)?;
    let bound = patch_bounding_radius_m(key, radius_m)?;
    if !centre.is_finite() || !bound.is_finite() {
        return None;
    }
    let nearest = centre - bound;
    if !nearest.is_finite() {
        return None;
    }
    if nearest < 1e-6 {
        return Some(1e-6);
    }
    Some(nearest)
}

/// Screen-space error of `key` in pixels (Q2 A).
///
/// The drawn quad's sag divided by the distance to the patch's nearest point
/// (its bounding sphere), times `focal_px` from `precision` (1600x900, 70 deg
/// vertical). Near-zero distances read as a large error (split).
#[must_use]
pub fn patch_screen_error_px(
    key: PatchKey,
    camera: PlanetPos,
    radius_m: f64,
    focal: f64,
) -> Option<f64> {
    let sag = patch_quad_sag_m(key, radius_m)?;
    let dist = patch_nearest_distance_m(key, camera, radius_m)?;
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
    pub(crate) leaves: BTreeSet<PatchKey>,
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

    /// Appends leaves to `out`, reusing its capacity (`E-HOT-NOALLOC`).
    pub fn append_leaves_to(&self, out: &mut Vec<PatchKey>) {
        out.clear();
        out.extend(self.leaves.iter().copied());
    }

    /// Screen error of `key` from `camera` at the default focal length.
    #[must_use]
    pub fn screen_error(&self, key: PatchKey, camera: PlanetPos) -> Option<f64> {
        patch_screen_error_px(key, camera, self.radius_m, focal_px())
    }

    /// One update reusing `scratch` (no steady-state allocation).
    ///
    /// Same rule as [`QuadTree::update`], but `to_split`, `parents` and
    /// `coarse` reuse the caller-provided buffers.
    pub fn update_reused(
        &mut self,
        camera: PlanetPos,
        detail_px: f64,
        scratch: &mut UpdateScratch,
    ) -> (usize, usize) {
        if !detail_px.is_finite() || detail_px <= 0.0 {
            return (0, 0);
        }
        let focal = focal_px();
        if !focal.is_finite() || focal <= 0.0 {
            return (0, 0);
        }
        let mut splits = 0usize;
        let mut merges = 0usize;
        // Splits.
        for _ in 0..32 {
            scratch.split.clear();
            for leaf in self.leaves.iter().copied() {
                if leaf.level >= self.max_level {
                    continue;
                }
                let Some(px) = patch_screen_error_px(leaf, camera, self.radius_m, focal) else {
                    continue;
                };
                if px > detail_px {
                    scratch.split.push(leaf);
                }
            }
            if scratch.split.is_empty() {
                break;
            }
            let mut split_any = false;
            for leaf in scratch.split.iter().copied() {
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
        // Neighbour rule.
        for _ in 0..32 {
            scratch.coarse.clear();
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
                        if leaf.level < self.max_level && !scratch.coarse.contains(&leaf) {
                            scratch.coarse.push(leaf);
                        }
                    } else if let Some(coarse) = self.covering_leaf(neighbour)
                        && coarse.level < self.max_level
                        && !scratch.coarse.contains(&coarse)
                    {
                        scratch.coarse.push(coarse);
                    }
                }
            }
            if scratch.coarse.is_empty() {
                break;
            }
            scratch.coarse.sort();
            let mut split_any = false;
            for leaf in scratch.coarse.iter().copied() {
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
        // Merges after the neighbour rule.
        for _ in 0..32 {
            scratch.parents.clear();
            for leaf in self.leaves.iter().copied() {
                if let Some(parent) = leaf.parent()
                    && !scratch.parents.contains(&parent)
                {
                    scratch.parents.push(parent);
                }
            }
            if scratch.parents.is_empty() {
                break;
            }
            scratch.parents.sort();
            let mut merged_any = false;
            for parent in scratch.parents.iter().copied() {
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
                if px >= detail_px / 2.0 {
                    continue;
                }
                let mut keeps_rule = true;
                for dir in [
                    NeighbourDir::East,
                    NeighbourDir::West,
                    NeighbourDir::North,
                    NeighbourDir::South,
                ] {
                    let Some(nkey) = neighbour_at_same_level(parent, dir) else {
                        continue;
                    };
                    if let Some(level) = self.covering_level(nkey)
                        && level.abs_diff(parent.level) > 1
                    {
                        keeps_rule = false;
                        break;
                    }
                }
                if !keeps_rule {
                    continue;
                }
                for child in children {
                    self.leaves.remove(&child);
                }
                self.leaves.insert(parent);
                merges += 1;
                merged_any = true;
            }
            if !merged_any {
                break;
            }
        }
        (splits, merges)
    }

    /// One update: split above `detail_px`, enforce the neighbour rule, then
    /// merge below half when the neighbour rule allows it.
    ///
    /// Order matters (review item 5): merges run after the neighbour rule and
    /// never merge a group the neighbour rule needs, so the split and merge
    /// queues settle instead of flipping every frame.
    ///
    /// Returns `(split_count, merge_count)`: parents split and groups merged.
    pub fn update(&mut self, camera: PlanetPos, detail_px: f64) -> (usize, usize) {
        let mut scratch = UpdateScratch::default();
        self.update_reused(camera, detail_px, &mut scratch)
    }

    /// Level covering `key`'s area: the leaf itself, its coarsest covering
    /// ancestor, or the finest descendant depth overlapping it.
    pub(crate) fn covering_level(&self, key: PatchKey) -> Option<u32> {
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
    pub(crate) fn covering_leaf(&self, key: PatchKey) -> Option<PatchKey> {
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
