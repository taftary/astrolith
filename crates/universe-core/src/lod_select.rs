//! Quadtree LOD selection per cube face (rebuild step 4, #777, T1).
//!
//! Selection and update moved here from `lod.rs` under `E-FILE-SIZE`.
//! `lod.rs` keeps the keys, geometry and error functions; this module owns
//! neighbour walking, the restricted-quadtree rule and the `QuadTree`
//! updates, including the relief-aware merge (Q1 A). Pure and deterministic
//! (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use crate::cube_sphere::{Face, FacePos, from_sphere, to_sphere};
use crate::lod::{
    PatchKey, max_level_for_radius, patch_edge_angle, patch_screen_error_px,
    patch_screen_error_with_relief_px, patch_uv_range,
};
use crate::lod_cull::Cull;
use crate::planet_frame::PlanetPos;
use crate::precision::focal_px;
use std::collections::BTreeSet;

fn length(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

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

/// The relief of `key` from the lookup, or for an unbuilt patch its
/// nearest built ancestor's height range with no relief error (#817 T1):
/// the ancestor's range contains the patch's, so its box is never too far
/// and splits happen no later than the real heights would ask.
fn lookup_or_ancestor(
    relief_lookup: &dyn Fn(PatchKey) -> Option<(f64, f64, f64)>,
    key: PatchKey,
) -> (f64, f64, f64) {
    if let Some(known) = relief_lookup(key) {
        return known;
    }
    let mut cursor = key;
    while let Some(parent) = cursor.parent() {
        if let Some((_, min_h, max_h)) = relief_lookup(parent) {
            return (0.0, min_h, max_h);
        }
        cursor = parent;
    }
    (0.0, 0.0, 0.0)
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
    /// Leaves seen by the last culled update, sorted (#778 Q3 A, Q4 A).
    visible: Vec<PatchKey>,
    /// Whether the last update culled (else every leaf is visible).
    culled: bool,
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
            visible: Vec::new(),
            culled: false,
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

    /// Appends the leaves the last update saw (every leaf when it did not
    /// cull) to `out`, reusing its capacity (#778, `E-HOT-NOALLOC`).
    pub fn append_visible_leaves_to(&self, out: &mut Vec<PatchKey>) {
        out.clear();
        if self.culled {
            out.extend(self.visible.iter().copied());
        } else {
            out.extend(self.leaves.iter().copied());
        }
    }

    /// Leaves the last update saw, in deterministic order.
    #[must_use]
    pub fn visible_leaves(&self) -> Vec<PatchKey> {
        let mut out = Vec::new();
        self.append_visible_leaves_to(&mut out);
        out
    }

    /// Screen error of `key` from `camera` at the default focal length.
    #[must_use]
    pub fn screen_error(&self, key: PatchKey, camera: PlanetPos) -> Option<f64> {
        patch_screen_error_px(key, camera, self.radius_m, focal_px())
    }

    /// Screen error of `key` with relief (#799, AC5).
    ///
    /// `relief_error_m` and the height range `min_h_m..max_h_m` come from
    /// the built patch cache; missing entries read as `(0, 0, 0)`, which
    /// equals [`QuadTree::screen_error`].
    #[must_use]
    pub fn screen_error_with_relief(
        &self,
        key: PatchKey,
        camera: PlanetPos,
        relief_error_m: f64,
        min_h_m: f64,
        max_h_m: f64,
    ) -> Option<f64> {
        patch_screen_error_with_relief_px(
            key,
            camera,
            self.radius_m,
            focal_px(),
            relief_error_m,
            min_h_m,
            max_h_m,
        )
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

    /// One relief-aware update reusing `scratch` (#799 AC5, #777 Q1 A).
    ///
    /// One error function for splits and merges:
    /// `max(quad_sag, relief_error)` over the relief-grown nearest distance
    /// ([`patch_screen_error_with_relief_px`]), with `relief_lookup`
    /// (`(relief_error_m, min_h_m, max_h_m)` per patch, `None` when unbuilt).
    /// Unbuilt leaves read as `(0, 0)` (sag only), so detail cascades over
    /// frames as children get built. Merges use the same relief-aware error
    /// and run only when the parent's relief data is known (`Some`), so
    /// mountain detail builds up and stays instead of collapsing every frame
    /// (the #799 review item 1). Neighbour rule re-enforced after splits.
    pub fn update_with_relief_reused(
        &mut self,
        camera: PlanetPos,
        detail_px: f64,
        relief_lookup: &dyn Fn(PatchKey) -> Option<(f64, f64, f64)>,
        scratch: &mut UpdateScratch,
    ) -> (usize, usize) {
        self.update_culled_reused(camera, detail_px, relief_lookup, None, scratch)
    }

    /// [`QuadTree::update_with_relief_reused`] with view culling (#778 Q3 A,
    /// Q4 A): a leaf the view cannot see is never split, a hidden parent
    /// merges whatever its error (off-view detail collapses over frames),
    /// and the visible leaves are kept for
    /// [`QuadTree::append_visible_leaves_to`]. The tree stays a full cover,
    /// so the neighbour rule is unchanged.
    pub fn update_culled_reused(
        &mut self,
        camera: PlanetPos,
        detail_px: f64,
        relief_lookup: &dyn Fn(PatchKey) -> Option<(f64, f64, f64)>,
        cull: Option<&Cull<'_>>,
        scratch: &mut UpdateScratch,
    ) -> (usize, usize) {
        let counts = self.update_inner(camera, detail_px, relief_lookup, cull, scratch);
        self.visible.clear();
        self.culled = cull.is_some();
        if let Some(cull) = cull {
            let radius_m = self.radius_m;
            self.visible.extend(
                self.leaves
                    .iter()
                    .copied()
                    .filter(|leaf| cull.sees(*leaf, radius_m)),
            );
        }
        counts
    }

    fn update_inner(
        &mut self,
        camera: PlanetPos,
        detail_px: f64,
        relief_lookup: &dyn Fn(PatchKey) -> Option<(f64, f64, f64)>,
        cull: Option<&Cull<'_>>,
        scratch: &mut UpdateScratch,
    ) -> (usize, usize) {
        let hidden = |key: PatchKey, radius_m: f64| cull.is_some_and(|c| !c.sees(key, radius_m));
        if !detail_px.is_finite() || detail_px <= 0.0 {
            return (0, 0);
        }
        let focal = focal_px();
        if !focal.is_finite() || focal <= 0.0 {
            return (0, 0);
        }
        let mut splits = 0usize;
        let mut merges = 0usize;
        // Splits: relief-aware, cascading (up to 32 passes).
        for _ in 0..32 {
            scratch.split.clear();
            for leaf in self.leaves.iter().copied() {
                if leaf.level >= self.max_level || hidden(leaf, self.radius_m) {
                    continue;
                }
                let (relief_error, min_h, max_h) = lookup_or_ancestor(relief_lookup, leaf);
                let Some(px) = patch_screen_error_with_relief_px(
                    leaf,
                    camera,
                    self.radius_m,
                    focal,
                    relief_error,
                    min_h,
                    max_h,
                ) else {
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
                if leaf.level >= self.max_level {
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
        // Neighbour rule (same as sag-only).
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
        // Merges: relief-aware, only when the parent's relief data is known.
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
                // A hidden parent merges at once (nothing of it is drawn).
                // Otherwise parents-stay: merge only when the parent's relief
                // error is known (cached in the window, computed in headless).
                if !hidden(parent, self.radius_m) {
                    let Some((relief_error, min_h, max_h)) = relief_lookup(parent) else {
                        continue;
                    };
                    let Some(px) = patch_screen_error_with_relief_px(
                        parent,
                        camera,
                        self.radius_m,
                        focal,
                        relief_error,
                        min_h,
                        max_h,
                    ) else {
                        continue;
                    };
                    if px >= detail_px / 2.0 {
                        continue;
                    }
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

    /// One relief-aware update (#799, AC5).
    ///
    /// Returns `(split_count, merge_count)`.
    pub fn update_with_relief(
        &mut self,
        camera: PlanetPos,
        detail_px: f64,
        relief_lookup: &dyn Fn(PatchKey) -> Option<(f64, f64, f64)>,
    ) -> (usize, usize) {
        let mut scratch = UpdateScratch::default();
        self.update_with_relief_reused(camera, detail_px, relief_lookup, &mut scratch)
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
        // Finer: deepest leaf below `key`, walking only its subtree
        // (review item 4: no scan over every leaf).
        self.deepest_below(key)
    }

    /// Deepest leaf level inside `key`'s subtree, `None` when empty.
    fn deepest_below(&self, key: PatchKey) -> Option<u32> {
        if key.level >= self.max_level {
            return None;
        }
        let children = key.children()?;
        let mut deepest: Option<u32> = None;
        for child in children {
            let level = if self.leaves.contains(&child) {
                Some(child.level)
            } else {
                self.deepest_below(child)
            };
            if let Some(level) = level
                && deepest.is_none_or(|held| level > held)
            {
                deepest = Some(level);
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

    /// Morph mask for `leaf` against the tree's leaves (#777 Q3 A).
    ///
    /// See [`crate::lod_morph::morph_mask_in`]: low nibble coarser edges, high nibble finer
    /// edges, East, West, North, South. Pure; `O(depth)` per edge.
    #[must_use]
    pub fn edge_mask(&self, leaf: PatchKey) -> u8 {
        crate::lod_morph::morph_mask_in(leaf, &|key| self.leaves.contains(&key))
    }

    /// Edge masks for every leaf, in deterministic order.
    #[must_use]
    pub fn edge_masks(&self) -> Vec<(PatchKey, u8)> {
        let mut out: Vec<(PatchKey, u8)> = Vec::with_capacity(self.leaves.len());
        for leaf in self.leaves.iter().copied() {
            out.push((leaf, self.edge_mask(leaf)));
        }
        out
    }
}
