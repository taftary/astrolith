//! Patch meshes on the quadtree (rebuild step 3, #776, T2; relief #799).
//!
//! `PatchMesh` is 33 x 33 points on #775's mapping with an `f64` centre and
//! `f32` metre offsets (Q5 A). Heights come from the one relief function
//! ([`crate::relief::Relief`], #799): positions are `(radius + h) x dir`,
//! still relative to the patch centre. Each build also records
//! `relief_error_m` (largest gap to double-resolution heights) and the height
//! range, feeding the relief-aware screen error so mountains split earlier
//! than plains. Pure and deterministic (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use crate::cube_sphere::grid_point;
use crate::lod::{PATCH_QUADS, PATCH_VERTS, PatchKey, patch_centre_dir, patch_quad_sag_m};
use crate::lod_patch_morph::morph_shape;
use crate::planet_frame::PlanetPos;
use crate::relief::Relief;

/// Skirt vertices per edge (33, corners duplicated across edges).
pub const SKIRT_PER_EDGE: usize = 33;

/// Skirt vertices per patch (4 edges).
pub const SKIRT_VERTS: usize = SKIRT_PER_EDGE * 4;

/// Main surface vertices (33 x 33).
pub const MAIN_VERTS: usize = 1089;

/// One built patch: centre plus `f32` metre offsets (Q5 A, #799).
///
/// Vertices are 33 x 33 points on #775's mapping; edges use the canonical
/// grid so same-level neighbours match bit for bit. Heights come from the
/// one relief function, so every level and both sides of a seam agree.
/// `parent_offsets`/`parent_normals`/`morph_err_m` hold the parent-level
/// shape for CDLOD geomorphing and `skirt_*` the skirts below every edge
/// (#777, built in `lod_patch_morph`). Pure, so it runs on any thread.
#[derive(Debug, Clone, PartialEq)]
pub struct PatchMesh {
    /// Which patch.
    pub key: PatchKey,
    /// Patch centre on the relief surface (metres, `f64`).
    pub centre: PlanetPos,
    /// Vertex offsets from the centre in metres (`f32`, row-major, 1089).
    pub offsets: Vec<[f32; 3]>,
    /// Terrain normals from the relief at this level's step (`f32`, 1089).
    pub normals: Vec<[f32; 3]>,
    /// Parent-level offsets from the centre (`f32`, 1089, #777).
    pub parent_offsets: Vec<[f32; 3]>,
    /// Parent-level terrain normals (`f32`, 1089, #777).
    pub parent_normals: Vec<[f32; 3]>,
    /// Morph error per vertex in metres (`f32`, 1089): the band ends at
    /// `err x focal / detail` (`lod_morph`).
    pub morph_err_m: Vec<f32>,
    /// Triangle indices for the main surface.
    pub indices: Vec<u32>,
    /// Skirt offsets below the edges (`f32`, 132, #777 Q4 A).
    pub skirt_offsets: Vec<[f32; 3]>,
    /// Skirt parent offsets (`f32`, 132, morph with their edge point).
    pub skirt_parent_offsets: Vec<[f32; 3]>,
    /// Skirt normals (`f32`, 132).
    pub skirt_normals: Vec<[f32; 3]>,
    /// Skirt parent normals (`f32`, 132).
    pub skirt_parent_normals: Vec<[f32; 3]>,
    /// Skirt morph errors (`f32`, 132, copied from their edge point).
    pub skirt_morph_err_m: Vec<f32>,
    /// Triangle indices for the skirts (into the combined main+skirt space).
    pub skirt_indices: Vec<u32>,
    /// Largest gap between this patch and double-resolution heights (metres).
    pub relief_error_m: f64,
    /// Lowest vertex height in metres above the mean radius.
    pub min_h_m: f64,
    /// Highest vertex height in metres above the mean radius.
    pub max_h_m: f64,
}

impl PatchMesh {
    /// Main-surface vertex count (always 1089).
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.offsets.len()
    }

    /// Skirt vertex count (always 132).
    #[must_use]
    pub fn skirt_vertex_count(&self) -> usize {
        self.skirt_offsets.len()
    }

    /// Combined vertex count (main plus skirts).
    #[must_use]
    pub fn total_vertex_count(&self) -> usize {
        self.offsets.len() + self.skirt_offsets.len()
    }

    /// Height range in metres (`max - min`).
    #[must_use]
    pub fn relief_info(&self) -> (f64, f64, f64) {
        (self.relief_error_m, self.min_h_m, self.max_h_m)
    }

    /// Height range in metres (`max_h_m - min_h_m`).
    #[must_use]
    pub fn height_range_m(&self) -> f64 {
        (self.max_h_m - self.min_h_m).max(0.0)
    }
}

/// Builds `key` on the relief surface of `radius_m` with `relief` (Q5 A, #799).
///
/// Returns `None` for a bad radius or overflowing indices. Deterministic:
/// the same key, radius and relief seed always give byte-identical floats.
#[must_use]
pub fn build_patch(key: PatchKey, radius_m: f64, relief: &Relief) -> Option<PatchMesh> {
    build_patch_with_heights(key, radius_m, relief).map(|(mesh, _)| mesh)
}

/// [`build_patch`], plus the relief heights (metres) it sampled on the 37 x
/// 37 grid of the patch and its ring (`None` off the face), so the
/// shared-grid tile reuses them (#920 T5) instead of sampling again.
pub(crate) fn build_patch_with_heights(
    key: PatchKey,
    radius_m: f64,
    relief: &Relief,
) -> Option<(PatchMesh, Vec<Option<f64>>)> {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return None;
    }
    let per_edge = key.per_edge()?;
    let n_global = per_edge.checked_mul(PATCH_QUADS)?;
    if n_global == 0 {
        return None;
    }
    let i0 = key.x.checked_mul(PATCH_QUADS)?;
    let j0 = key.y.checked_mul(PATCH_QUADS)?;
    if i0.checked_add(PATCH_QUADS)? > n_global || j0.checked_add(PATCH_QUADS)? > n_global {
        return None;
    }
    let centre_dir = patch_centre_dir(key)?;
    let centre_h = relief.height_m(centre_dir);
    if !centre_h.is_finite() {
        return None;
    }
    let centre_r = radius_m + centre_h;
    if !centre_r.is_finite() || centre_r <= 0.0 {
        return None;
    }
    let centre = PlanetPos([
        centre_dir[0] * centre_r,
        centre_dir[1] * centre_r,
        centre_dir[2] * centre_r,
    ]);
    let side = usize::try_from(PATCH_VERTS).ok()?;
    let mut offsets: Vec<[f32; 3]> = Vec::with_capacity(side * side);
    // Planet-frame positions and directions for the morph shape (`f64`).
    let mut abs: Vec<[f64; 3]> = Vec::with_capacity(side * side);
    let mut dirs: Vec<[f64; 3]> = Vec::with_capacity(side * side);
    // Coarse heights for the relief error below (row-major).
    let mut coarse_h: Vec<f64> = Vec::with_capacity(side * side);
    let mut min_h = f64::INFINITY;
    let mut max_h = f64::NEG_INFINITY;
    for dj in 0..=PATCH_QUADS {
        for di in 0..=PATCH_QUADS {
            let gi = i0.checked_add(di)?;
            let gj = j0.checked_add(dj)?;
            if gi > n_global || gj > n_global {
                return None;
            }
            let dir = grid_point(key.face, gi, gj, n_global)?;
            let h = relief.height_m(dir);
            if !h.is_finite() {
                return None;
            }
            if h < min_h {
                min_h = h;
            }
            if h > max_h {
                max_h = h;
            }
            coarse_h.push(h);
            let r = radius_m + h;
            if !r.is_finite() || r <= 0.0 {
                return None;
            }
            let px = dir[0] * r;
            let py = dir[1] * r;
            let pz = dir[2] * r;
            let dx = px - centre.0[0];
            let dy = py - centre.0[1];
            let dz = pz - centre.0[2];
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: patch offsets narrowed for the GPU, intended"
            )]
            let off = [dx as f32, dy as f32, dz as f32];
            offsets.push(off);
            abs.push([px, py, pz]);
            dirs.push(dir);
        }
    }
    if !min_h.is_finite() || !max_h.is_finite() {
        return None;
    }
    // Relief error: largest gap between the coarse heights and one extra
    // sampling at double resolution (65 x 65), bilinearly interpolated.
    let relief_error_m = double_res_error(key, n_global, i0, j0, relief, &coarse_h)?;
    let mut indices: Vec<u32> =
        Vec::with_capacity(usize::try_from(PATCH_QUADS * PATCH_QUADS * 6).ok()?);
    let right_handed = crate::lod::face_grid_right_handed(key.face);
    for jj in 0..PATCH_QUADS {
        for ii in 0..PATCH_QUADS {
            let stride = PATCH_QUADS + 1;
            let a = jj * stride + ii;
            let b = a + 1;
            let c = a + stride;
            let d = c + 1;
            // Same triangles (and diagonal) on every face, wound to face
            // out (#778 Q5 A): the grid order reverses on right-handed faces.
            let tris = if right_handed {
                [a, b, c, b, d, c]
            } else {
                [a, c, b, b, c, d]
            };
            for v in tris {
                indices.push(v);
            }
        }
    }
    // Morph shape, normals and skirts (#777, `lod_patch_morph`).
    let quad_sag = patch_quad_sag_m(key, radius_m).unwrap_or(0.0);
    let patch_err = quad_sag.max(if relief_error_m.is_finite() && relief_error_m > 0.0 {
        relief_error_m
    } else {
        0.0
    });
    let shape = morph_shape(key, radius_m, relief, centre, &abs, &dirs, patch_err)?;
    let mut heights = shape.ring_heights;
    let ring_side = side + 4;
    for (k, h) in coarse_h.iter().copied().enumerate() {
        *heights.get_mut((k / side + 2) * ring_side + k % side + 2)? = Some(h);
    }
    let mesh = PatchMesh {
        key,
        centre,
        offsets,
        normals: shape.normals,
        parent_offsets: shape.parent_offsets,
        parent_normals: shape.parent_normals,
        morph_err_m: shape.morph_err_m,
        indices,
        skirt_offsets: shape.skirt_offsets,
        skirt_parent_offsets: shape.skirt_parent_offsets,
        skirt_normals: shape.skirt_normals,
        skirt_parent_normals: shape.skirt_parent_normals,
        skirt_morph_err_m: shape.skirt_morph_err_m,
        skirt_indices: shape.skirt_indices,
        relief_error_m,
        min_h_m: min_h,
        max_h_m: max_h,
    };
    Some((mesh, heights))
}

/// Largest `|fine - bilinear(coarse)|` over the double-resolution grid.
fn double_res_error(
    key: PatchKey,
    n_global: u32,
    i0: u32,
    j0: u32,
    relief: &Relief,
    coarse_h: &[f64],
) -> Option<f64> {
    let n2 = n_global.checked_mul(2)?;
    let i02 = i0.checked_mul(2)?;
    let j02 = j0.checked_mul(2)?;
    let stride = usize::try_from(PATCH_QUADS + 1).ok()?;
    let mut worst = 0.0f64;
    for dj2 in 0..=PATCH_QUADS * 2 {
        for di2 in 0..=PATCH_QUADS * 2 {
            let gi2 = i02.checked_add(di2)?;
            let gj2 = j02.checked_add(dj2)?;
            if gi2 > n2 || gj2 > n2 {
                return None;
            }
            // Even points coincide with coarse vertices (error zero); still
            // evaluate to keep the sampling uniform? Skip them: they add no
            // information and halve the cost.
            if di2 % 2 == 0 && dj2 % 2 == 0 {
                continue;
            }
            let dir2 = grid_point(key.face, gi2, gj2, n2)?;
            let fine = relief.height_m(dir2);
            if !fine.is_finite() {
                return None;
            }
            let di = (di2 / 2) as usize;
            let dj = (dj2 / 2) as usize;
            let fx = f64::from(di2 % 2) * 0.5;
            let fy = f64::from(dj2 % 2) * 0.5;
            let at = |di: usize, dj: usize| -> Option<f64> {
                if di > PATCH_QUADS as usize || dj > PATCH_QUADS as usize {
                    return None;
                }
                coarse_h.get(dj * stride + di).copied()
            };
            let h00 = at(di, dj)?;
            // Edge-aware bilinear: far-edge points (di == 32 or dj == 32)
            // have fx == 0 or fy == 0, so only the in-range corners matter.
            let pred = if fx == 0.0 && fy == 0.0 {
                h00
            } else if fx == 0.0 {
                let h01 = at(di, dj + 1)?;
                h00 * (1.0 - fy) + h01 * fy
            } else if fy == 0.0 {
                let h10 = at(di + 1, dj)?;
                h00 * (1.0 - fx) + h10 * fx
            } else {
                let h10 = at(di + 1, dj)?;
                let h01 = at(di, dj + 1)?;
                let h11 = at(di + 1, dj + 1)?;
                h00 * (1.0 - fx) * (1.0 - fy)
                    + h10 * fx * (1.0 - fy)
                    + h01 * (1.0 - fx) * fy
                    + h11 * fx * fy
            };
            let gap = (fine - pred).abs();
            if gap.is_finite() && gap > worst {
                worst = gap;
            }
        }
    }
    Some(worst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cube_sphere::Face;
    use crate::lod::patch_edge_angle;
    use crate::lod_select::QuadTree;
    use crate::planet_frame::EARTH_RADIUS_M;
    use crate::relief::{Relief, ReliefTint, tint_for_height};

    fn earth_relief() -> Relief {
        let journey = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
        let chain: &[u32] = journey.get(..9).unwrap_or(&[]);
        Relief::for_planet(crate::nav::DEMO_SEED, chain)
    }

    fn earth_camera(relief: &Relief, height_m: f64, dir: [f64; 3]) -> PlanetPos {
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        let unit = [dir[0] / len, dir[1] / len, dir[2] / len];
        // Height above the relief surface (#799 review fix), so the camera
        // never floats kilometres above the terrain it measures.
        let h = relief.height_m(unit);
        let h = if h.is_finite() { h } else { 0.0 };
        let r = EARTH_RADIUS_M + h + height_m;
        PlanetPos([unit[0] * r, unit[1] * r, unit[2] * r])
    }

    #[test]
    fn drawn_patches_shake_under_a_millimetre() {
        // AC4 / C4 (T2, Q5 A; still holds with relief, #799 AC9): under 1 mm
        // within 1 km on the drawn patches at 1 m, 100 m, 10 km and 1,000 km,
        // through the patch-origin path.
        let relief = earth_relief();
        let detail_px = 1.5;
        let heights = [1.0, 100.0, 10_000.0, 1_000_000.0];
        let dirs = [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0]];
        let mut worst_mm = 0.0f64;
        for dir in dirs {
            for height in heights {
                let camera = earth_camera(&relief, height, dir);
                let mut tree = QuadTree::new(EARTH_RADIUS_M).expect("tree");
                tree.update(camera, detail_px);
                for leaf in tree.leaves() {
                    // Skip patches with no vertex within 2 km (bounding check).
                    let Some(centre_dir) = patch_centre_dir(leaf) else {
                        continue;
                    };
                    let centre_h = relief.height_m(centre_dir);
                    let centre_r = EARTH_RADIUS_M + centre_h;
                    let centre = [
                        centre_dir[0] * centre_r,
                        centre_dir[1] * centre_r,
                        centre_dir[2] * centre_r,
                    ];
                    let dist = ((centre[0] - camera.0[0]).powi(2)
                        + (centre[1] - camera.0[1]).powi(2)
                        + (centre[2] - camera.0[2]).powi(2))
                    .sqrt();
                    let edge = patch_edge_angle(leaf).unwrap_or(0.0) * EARTH_RADIUS_M;
                    if dist - edge > 2_000.0 {
                        continue;
                    }
                    let Some(mesh) = build_patch(leaf, EARTH_RADIUS_M, &relief) else {
                        continue;
                    };
                    assert_eq!(mesh.vertex_count(), 1089);
                    assert_eq!(mesh.indices.len(), 32 * 32 * 6);
                    // Camera-relative centre narrowed to f32 (the Transform).
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "E-CAST: camera-relative centre narrowed for the GPU, intended"
                    )]
                    let dc = [
                        (mesh.centre.0[0] - camera.0[0]) as f32,
                        (mesh.centre.0[1] - camera.0[1]) as f32,
                        (mesh.centre.0[2] - camera.0[2]) as f32,
                    ];
                    for (vi, off) in mesh.offsets.iter().enumerate() {
                        // True vertex: relief surface at the grid point.
                        let side = usize::try_from(PATCH_VERTS).unwrap_or(33);
                        #[expect(
                            clippy::cast_possible_truncation,
                            reason = "E-CAST: test vertex indices below 33, exact"
                        )]
                        let di = (vi % side) as u32;
                        #[expect(
                            clippy::cast_possible_truncation,
                            reason = "E-CAST: test vertex indices below 33, exact"
                        )]
                        let dj = (vi / side) as u32;
                        let per_edge = leaf.per_edge().expect("per edge");
                        let n_global = per_edge.checked_mul(PATCH_QUADS).expect("n");
                        let i0 = leaf.x.checked_mul(PATCH_QUADS).expect("i0");
                        let j0 = leaf.y.checked_mul(PATCH_QUADS).expect("j0");
                        let gi = i0.checked_add(di).expect("gi");
                        let gj = j0.checked_add(dj).expect("gj");
                        let d = grid_point(leaf.face, gi, gj, n_global).expect("grid");
                        let h = relief.height_m(d);
                        let r = EARTH_RADIUS_M + h;
                        let true_p = [d[0] * r, d[1] * r, d[2] * r];
                        let true_rel = [
                            true_p[0] - camera.0[0],
                            true_p[1] - camera.0[1],
                            true_p[2] - camera.0[2],
                        ];
                        let dist_m = (true_rel[0] * true_rel[0]
                            + true_rel[1] * true_rel[1]
                            + true_rel[2] * true_rel[2])
                            .sqrt();
                        if dist_m > 1_000.0 || !dist_m.is_finite() {
                            continue;
                        }
                        // Drawn vertex: f32 centre plus f32 offset (GPU adds).
                        let drawn = [dc[0] + off[0], dc[1] + off[1], dc[2] + off[2]];
                        let err = ((f64::from(drawn[0]) - true_rel[0]).powi(2)
                            + (f64::from(drawn[1]) - true_rel[1]).powi(2)
                            + (f64::from(drawn[2]) - true_rel[2]).powi(2))
                        .sqrt();
                        let err_mm = err * 1_000.0;
                        if err_mm > worst_mm {
                            worst_mm = err_mm;
                        }
                        assert!(
                            err < 1e-3,
                            "patch {leaf:?} vertex {vi} at {height} m shakes {err} m"
                        );
                    }
                }
            }
        }
        assert!(worst_mm < 1.0, "worst {worst_mm} mm");
        // Deterministic: same patch builds byte-identical floats.
        let key = PatchKey::new(Face::PosY, 2, 1, 1).expect("key");
        let a = build_patch(key, EARTH_RADIUS_M, &relief).expect("patch");
        let b = build_patch(key, EARTH_RADIUS_M, &relief).expect("patch");
        assert_eq!(a.offsets, b.offsets);
        assert_eq!(a.normals, b.normals);
        assert_eq!(a.indices, b.indices);
        assert_eq!(
            a.relief_error_m.to_bits(),
            b.relief_error_m.to_bits(),
            "same patch, same relief error"
        );
    }

    #[test]
    #[ignore = "slow (~14 s on the dev PC): runs with --include-ignored (CI, validator), #847"]
    fn mountains_split_before_plains() {
        // AC5 / C5 (T3): the patch error includes relief, so a mountain patch
        // splits before a flat one at the same distance.
        use crate::lod::{patch_nearest_distance_with_relief_m, patch_quad_sag_m};
        use crate::precision::focal_px;
        let relief = earth_relief();
        // Find the most and least mountainous level-6 patches by relief error.
        let mut best: Option<(PatchKey, f64)> = None;
        let mut flat: Option<(PatchKey, f64)> = None;
        for face in Face::all() {
            for x in 0..8u32 {
                for y in 0..8u32 {
                    let Some(key) = PatchKey::new(face, 3, x, y) else {
                        continue;
                    };
                    let Some(mesh) = build_patch(key, EARTH_RADIUS_M, &relief) else {
                        continue;
                    };
                    let err = mesh.relief_error_m;
                    if !err.is_finite() {
                        continue;
                    }
                    if best.is_none_or(|(_, held)| err > held) {
                        best = Some((key, err));
                    }
                    if flat.is_none_or(|(_, held)| err < held) {
                        flat = Some((key, err));
                    }
                }
            }
        }
        let (mountain_key, mountain_err) = best.expect("mountain patch");
        let (plains_key, plains_err) = flat.expect("plains patch");
        assert!(
            mountain_err > plains_err * 3.0,
            "mountain error {mountain_err} not well above plains {plains_err}"
        );
        assert!(
            mountain_err > 10.0,
            "mountain patch should carry real relief error, got {mountain_err}"
        );
        // At the same camera distance the mountain screen error is larger,
        // and there is a height where mountains split while plains do not.
        let focal = focal_px();
        let mut found = false;
        for height in [
            2_000_000.0,
            1_000_000.0,
            200_000.0,
            100_000.0,
            50_000.0,
            20_000.0,
            10_000.0,
            5_000.0,
        ] {
            let mountain_cam = earth_camera(
                &relief,
                height,
                patch_centre_dir(mountain_key).expect("dir"),
            );
            let plains_cam =
                earth_camera(&relief, height, patch_centre_dir(plains_key).expect("dir"));
            let sag_m = patch_quad_sag_m(mountain_key, EARTH_RADIUS_M).expect("sag");
            let sag_p = patch_quad_sag_m(plains_key, EARTH_RADIUS_M).expect("sag");
            // Height ranges grow the bounding radius (mountains nearer).
            let mountain_mesh = build_patch(mountain_key, EARTH_RADIUS_M, &relief).expect("mesh");
            let plains_mesh = build_patch(plains_key, EARTH_RADIUS_M, &relief).expect("mesh");
            let dist_m = patch_nearest_distance_with_relief_m(
                mountain_key,
                mountain_cam,
                EARTH_RADIUS_M,
                mountain_mesh.min_h_m,
                mountain_mesh.max_h_m,
            )
            .expect("dist");
            let dist_p = patch_nearest_distance_with_relief_m(
                plains_key,
                plains_cam,
                EARTH_RADIUS_M,
                plains_mesh.min_h_m,
                plains_mesh.max_h_m,
            )
            .expect("dist");
            let err_m = sag_m.max(mountain_err) / dist_m * focal;
            let err_p = sag_p.max(plains_err) / dist_p * focal;
            assert!(
                err_m > err_p,
                "mountain {err_m} px not above plains {err_p} px at {height} m"
            );
            if err_m > 1.5 && err_p <= 1.5 {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "no height splits mountains ({mountain_key:?} err {mountain_err}) before plains ({plains_key:?} err {plains_err})"
        );
    }

    #[test]
    fn patches_are_tinted_by_height() {
        // AC6 / C6 (T3, Q3 A, Q4 A): vertices tint by height (deep below sea
        // level, low 0-1 km, high 1-4 km, snow above 4 km), unlit.
        let relief = earth_relief();
        let scale = relief.relief_scale();
        // Tint boundaries follow the spec, scaled (Earth scale is ~1).
        let low_top = 1_000.0 * scale;
        let high_top = 4_000.0 * scale;
        assert_eq!(tint_for_height(-4_300.0, scale), ReliefTint::Deep);
        assert_eq!(tint_for_height(-0.01, scale), ReliefTint::Deep);
        assert_eq!(tint_for_height(0.0, scale), ReliefTint::Low);
        assert_eq!(tint_for_height(low_top * 0.5, scale), ReliefTint::Low);
        assert_eq!(tint_for_height(low_top, scale), ReliefTint::High);
        assert_eq!(tint_for_height(high_top * 0.999, scale), ReliefTint::High);
        assert_eq!(tint_for_height(high_top, scale), ReliefTint::Snow);
        assert_eq!(tint_for_height(high_top * 1.75, scale), ReliefTint::Snow);
        // Earth carries all four bands across its patches.
        let mut seen = [false; 4];
        for face in Face::all() {
            for (x, y) in [(0u32, 0u32), (1, 0), (0, 1), (1, 1)] {
                let Some(key) = PatchKey::new(face, 1, x, y) else {
                    continue;
                };
                let Some(mesh) = build_patch(key, EARTH_RADIUS_M, &relief) else {
                    continue;
                };
                // Vertex heights from the mesh centre and offsets.
                for off in &mesh.offsets {
                    let px = mesh.centre.0[0] + f64::from(off[0]);
                    let py = mesh.centre.0[1] + f64::from(off[1]);
                    let pz = mesh.centre.0[2] + f64::from(off[2]);
                    let r = (px * px + py * py + pz * pz).sqrt();
                    let h = r - EARTH_RADIUS_M;
                    match tint_for_height(h, scale) {
                        ReliefTint::Deep => seen[0] = true,
                        ReliefTint::Low => seen[1] = true,
                        ReliefTint::High => seen[2] = true,
                        ReliefTint::Snow => seen[3] = true,
                    }
                }
            }
        }
        assert_eq!(
            seen,
            [true, true, true, true],
            "Earth shows {seen:?}, want all four tints"
        );
        // Giants stay smooth: every height is zero, every tint is Low.
        let giant = Relief::for_recipe(42, 11.0);
        let gkey = PatchKey::new(Face::PosY, 1, 0, 0).expect("giant patch");
        let gmesh =
            build_patch(gkey, crate::planet_frame::radius_m_for(11.0), &giant).expect("giant mesh");
        assert_eq!(gmesh.relief_error_m, 0.0);
        assert_eq!(gmesh.min_h_m, 0.0);
        assert_eq!(gmesh.max_h_m, 0.0);
    }
}
