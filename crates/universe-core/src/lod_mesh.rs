//! Patch meshes on the quadtree (rebuild step 3, #776, T2).
//!
//! `PatchMesh` is 33 x 33 points on #775's mapping with an `f64` centre and
//! `f32` metre offsets (Q5 A). Pure and deterministic (`E-DET-TIERS`,
//! `E-CORE-NO-BEVY`).

use crate::cube_sphere::{grid_point, to_planet};
use crate::lod::{PATCH_QUADS, PATCH_VERTS, PatchKey, patch_centre_dir};
use crate::planet_frame::PlanetPos;

/// One built patch: centre plus `f32` metre offsets (Q5 A).
///
/// Vertices are 33 x 33 points on #775's mapping; edges use the canonical
/// grid so same-level neighbours match bit for bit. Normals are the sphere
/// directions. Pure, so it runs on any thread.
#[derive(Debug, Clone, PartialEq)]
pub struct PatchMesh {
    /// Which patch.
    pub key: PatchKey,
    /// Patch centre on the true sphere (metres, `f64`).
    pub centre: PlanetPos,
    /// Vertex offsets from the centre in metres (`f32`, row-major).
    pub offsets: Vec<[f32; 3]>,
    /// Vertex normals (unit sphere directions, `f32`).
    pub normals: Vec<[f32; 3]>,
    /// Triangle indices.
    pub indices: Vec<u32>,
}

impl PatchMesh {
    /// Vertex count (always 1089).
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.offsets.len()
    }
}

/// Builds `key` on the sphere of `radius_m` (Q5 A).
///
/// Returns `None` for a bad radius or overflowing indices. Deterministic:
/// the same key and radius always give byte-identical floats.
#[must_use]
pub fn build_patch(key: PatchKey, radius_m: f64) -> Option<PatchMesh> {
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
    let centre = PlanetPos([
        centre_dir[0] * radius_m,
        centre_dir[1] * radius_m,
        centre_dir[2] * radius_m,
    ]);
    let side = usize::try_from(PATCH_VERTS).ok()?;
    let mut offsets: Vec<[f32; 3]> = Vec::with_capacity(side * side);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(side * side);
    for dj in 0..=PATCH_QUADS {
        for di in 0..=PATCH_QUADS {
            let gi = i0.checked_add(di)?;
            let gj = j0.checked_add(dj)?;
            if gi > n_global || gj > n_global {
                return None;
            }
            let dir = grid_point(key.face, gi, gj, n_global)?;
            let planet = to_planet(dir, radius_m)?;
            let dx = planet.0[0] - centre.0[0];
            let dy = planet.0[1] - centre.0[1];
            let dz = planet.0[2] - centre.0[2];
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: patch offsets narrowed for the GPU, intended"
            )]
            let off = [dx as f32, dy as f32, dz as f32];
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: sphere normals narrowed for the GPU, intended"
            )]
            let nrm = [dir[0] as f32, dir[1] as f32, dir[2] as f32];
            offsets.push(off);
            normals.push(nrm);
        }
    }
    let mut indices: Vec<u32> =
        Vec::with_capacity(usize::try_from(PATCH_QUADS * PATCH_QUADS * 6).ok()?);
    for jj in 0..PATCH_QUADS {
        for ii in 0..PATCH_QUADS {
            let stride = PATCH_QUADS + 1;
            let a = jj * stride + ii;
            let b = a + 1;
            let c = a + stride;
            let d = c + 1;
            for v in [a, c, b, b, c, d] {
                indices.push(v);
            }
        }
    }
    Some(PatchMesh {
        key,
        centre,
        offsets,
        normals,
        indices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cube_sphere::Face;
    use crate::lod::{QuadTree, patch_edge_angle};
    use crate::planet_frame::EARTH_RADIUS_M;

    fn earth_camera(height_m: f64, dir: [f64; 3]) -> PlanetPos {
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        let unit = [dir[0] / len, dir[1] / len, dir[2] / len];
        let r = EARTH_RADIUS_M + height_m;
        PlanetPos([unit[0] * r, unit[1] * r, unit[2] * r])
    }

    #[test]
    fn drawn_patches_shake_under_a_millimetre() {
        // AC4 / C4 (T2, Q5 A): under 1 mm within 1 km on the drawn patches at
        // 1 m, 100 m, 10 km and 1,000 km, through the patch-origin path.
        let detail_px = 1.5;
        let heights = [1.0, 100.0, 10_000.0, 1_000_000.0];
        let dirs = [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0]];
        let mut worst_mm = 0.0f64;
        for dir in dirs {
            for height in heights {
                let camera = earth_camera(height, dir);
                let mut tree = QuadTree::new(EARTH_RADIUS_M).expect("tree");
                tree.update(camera, detail_px);
                for leaf in tree.leaves() {
                    // Skip patches with no vertex within 2 km (bounding check).
                    let Some(centre_dir) = patch_centre_dir(leaf) else {
                        continue;
                    };
                    let centre = [
                        centre_dir[0] * EARTH_RADIUS_M,
                        centre_dir[1] * EARTH_RADIUS_M,
                        centre_dir[2] * EARTH_RADIUS_M,
                    ];
                    let dist = ((centre[0] - camera.0[0]).powi(2)
                        + (centre[1] - camera.0[1]).powi(2)
                        + (centre[2] - camera.0[2]).powi(2))
                    .sqrt();
                    let edge = patch_edge_angle(leaf).unwrap_or(0.0) * EARTH_RADIUS_M;
                    if dist - edge > 2_000.0 {
                        continue;
                    }
                    let Some(mesh) = build_patch(leaf, EARTH_RADIUS_M) else {
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
                        // True vertex: centre plus the exact f64 offset.
                        // Rebuild the f64 offset from the mesh key to avoid
                        // storing it: the true position is the grid point.
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
                        let true_p = [
                            d[0] * EARTH_RADIUS_M,
                            d[1] * EARTH_RADIUS_M,
                            d[2] * EARTH_RADIUS_M,
                        ];
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
        let a = build_patch(key, EARTH_RADIUS_M).expect("patch");
        let b = build_patch(key, EARTH_RADIUS_M).expect("patch");
        assert_eq!(a.offsets, b.offsets);
        assert_eq!(a.normals, b.normals);
        assert_eq!(a.indices, b.indices);
    }
}
