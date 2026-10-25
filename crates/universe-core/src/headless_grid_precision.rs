//! Shared-grid precision probe (#920 T1).
//!
//! Settles the 500 m ring selection 2 m over the plains and over the
//! mountain of the demo home planet, then rebuilds every leaf's offsets,
//! morph-parent offsets and normals from heights only with
//! [`crate::grid_rebuild`] (`f32`, as a shader would) and measures the gap
//! to today's patches ([`crate::lod_mesh::build_patch`]).
//!
//! Heights are stored relative to the patch centre (`h - h_c`), on a
//! 37 x 37 grid: the patch plus a ring of two points, which the normals'
//! stencils need. Stencils that leave the cube face fall back to the relief
//! normal today; those vertices are counted, not rebuilt.
//!
//! Deterministic (`E-DET-TIERS`), std only (`E-CORE-NO-BEVY`).

use crate::cube_sphere::grid_point;
use crate::grid_rebuild::{GridFrame, exact_offset, naive_offset, steps_of};
use crate::lod::{PATCH_QUADS, PatchKey};
use crate::lod_mesh::{PatchMesh, build_patch};
use crate::lod_ring::RING_M;
use crate::lod_select::{QuadTree, UpdateScratch};
use crate::planet_frame::{EARTH_RADIUS_M, PlanetPos};
use crate::relief::Relief;
use std::collections::HashMap;

/// Camera height over the ground in metres.
const HEIGHT_M: f64 = 2.0;

/// Leaves within this box distance of the camera count as near.
const NEAR_M: f64 = 500.0;

/// Most update rounds to settle the selection.
const SETTLE_ROUNDS: usize = 40;

/// Points of the height grid per side: the patch plus a ring of two.
const RING_SIDE: i32 = 37;

/// Ring width around the patch.
const RING: i32 = 2;

/// One site's numbers (all maxima over the settled leaves).
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct PrecisionMetrics {
    /// The selection settled with every wanted leaf built.
    pub settled: bool,
    /// Leaves within [`NEAR_M`] of the camera.
    pub near_leaves: usize,
    /// Leaves beyond it.
    pub far_leaves: usize,
    /// Deepest level among the leaves.
    pub deepest_level: u32,
    /// Exact form: largest offset gap near the camera, millimetres.
    pub exact_near_mm: f64,
    /// The shader's path (a packed tile with `f64` tangent steps): largest
    /// offset gap near the camera, millimetres.
    pub tile_near_mm: f64,
    /// Naive form: largest offset gap near the camera, millimetres.
    pub naive_near_mm: f64,
    /// Exact form: largest morph-parent gap near the camera, millimetres.
    pub parent_near_mm: f64,
    /// Exact form: largest gap anywhere, as a share of that patch's quad.
    pub exact_far_share: f64,
    /// Naive form: the same share.
    pub naive_far_share: f64,
    /// Normals rebuilt from the `f32` positions: largest angle to today's,
    /// degrees, near the camera.
    pub normal_near_deg: f64,
    /// Parent normals: the same, degrees.
    pub parent_normal_near_deg: f64,
    /// Vertices whose normal stencil leaves the cube face (not rebuilt).
    pub fallback_normals: usize,
}

/// Built meshes by key.
type Built = HashMap<PatchKey, PatchMesh>;

/// Settles the selection at `camera`: every wanted leaf built.
fn settle(tree: &mut QuadTree, camera: PlanetPos, relief: &Relief, built: &mut Built) -> bool {
    let quad_px = crate::settings::PlanetDetail::Normal.quad_px();
    let mut scratch = UpdateScratch::default();
    let mut wanted = Vec::new();
    for _ in 0..SETTLE_ROUNDS {
        {
            let lookup = |key: PatchKey| built.get(&key).map(PatchMesh::relief_info);
            tree.update_culled_reused(camera, quad_px, &lookup, None, &mut scratch);
        }
        tree.append_wanted_to(&mut wanted);
        let missing: Vec<PatchKey> = wanted
            .iter()
            .copied()
            .filter(|key| !built.contains_key(key))
            .collect();
        if missing.is_empty() {
            return true;
        }
        let meshes = crate::headless_morph_measure::par_chunks(&missing, |chunk| {
            chunk
                .iter()
                .filter_map(|key| build_patch(*key, EARTH_RADIUS_M, relief))
                .collect::<Vec<_>>()
        });
        for mesh in meshes.into_iter().flatten() {
            built.insert(mesh.key, mesh);
        }
    }
    false
}

/// `h - h_c` on the 37 x 37 grid around `key` (`None` off the cube face).
fn ring_heights(key: PatchKey, relief: &Relief, centre_h: f64) -> Option<Vec<Option<f32>>> {
    let n = key.per_edge()?.checked_mul(PATCH_QUADS)?;
    let (i0, j0) = (
        i64::from(key.x.checked_mul(PATCH_QUADS)?),
        i64::from(key.y.checked_mul(PATCH_QUADS)?),
    );
    let mut out = Vec::new();
    for rj in 0..RING_SIDE {
        for ri in 0..RING_SIDE {
            let gi = u32::try_from(i0 + i64::from(ri - RING)).ok();
            let gj = u32::try_from(j0 + i64::from(rj - RING)).ok();
            let dh = match (gi, gj) {
                (Some(gi), Some(gj)) if gi <= n && gj <= n => {
                    let dir = grid_point(key.face, gi, gj, n)?;
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "E-CAST: stored heights are f32, as on the GPU"
                    )]
                    Some((relief.height_m(dir) - centre_h) as f32)
                }
                _ => None,
            };
            out.push(dh);
        }
    }
    Some(out)
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn gap_m(a: [f32; 3], b: [f32; 3]) -> f64 {
    let d = sub(a, b);
    let (x, y, z) = (f64::from(d[0]), f64::from(d[1]), f64::from(d[2]));
    (x * x + y * y + z * z).sqrt()
}

fn unit(v: [f32; 3]) -> Option<[f32; 3]> {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    (len.is_finite() && len > 0.0).then(|| [v[0] / len, v[1] / len, v[2] / len])
}

/// Angle in degrees between two unit vectors, sign-free (orientation is
/// fixed outward by the caller in the shader, so only the line matters).
fn angle_deg(a: [f32; 3], b: [f32; 3]) -> f64 {
    let dot = f64::from(a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).abs();
    dot.min(1.0).acos().to_degrees()
}

/// The two vertices an odd vertex's parent position sits between
/// (`lod_patch_morph::parent_pair`, kept equal by the tests below).
fn parent_pair(di: i32, dj: i32) -> Option<((i32, i32), (i32, i32))> {
    match (di % 2 == 1, dj % 2 == 1) {
        (false, false) => None,
        (false, true) => Some(((di, dj - 1), (di, dj + 1))),
        (true, false) => Some(((di - 1, dj), (di + 1, dj))),
        (true, true) => Some(((di + 1, dj - 1), (di - 1, dj + 1))),
    }
}

/// Measures one leaf into `m`.
fn measure_leaf(
    mesh: &PatchMesh,
    relief: &Relief,
    near: bool,
    m: &mut PrecisionMetrics,
) -> Option<()> {
    let key = mesh.key;
    let c = mesh.centre.0;
    let centre_r = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
    let centre_h = centre_r - EARTH_RADIUS_M;
    let frame = GridFrame::new(key, centre_r)?;
    let heights = ring_heights(key, relief, centre_h)?;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: the naive form subtracts an f32 centre, on purpose"
    )]
    let centre32 = [c[0] as f32, c[1] as f32, c[2] as f32];
    let quad_m = crate::lod_range::edge_m(key.level, EARTH_RADIUS_M) / f64::from(PATCH_QUADS);
    let dh_at = |di: i32, dj: i32| -> Option<f32> {
        let idx = usize::try_from((dj + RING) * RING_SIDE + di + RING).ok()?;
        heights.get(idx).copied().flatten()
    };
    let rebuilt = |di: i32, dj: i32| -> Option<[f32; 3]> {
        let dh = dh_at(di, dj)?;
        Some(exact_offset(
            &frame,
            di - crate::grid_rebuild::CENTRE_INDEX,
            dj - crate::grid_rebuild::CENTRE_INDEX,
            dh,
        ))
    };
    let normal_at = |di: i32, dj: i32, step: i32| -> Option<[f32; 3]> {
        let e = rebuilt(di + step, dj)?;
        let w = rebuilt(di - step, dj)?;
        let n = rebuilt(di, dj + step)?;
        let s = rebuilt(di, dj - step)?;
        let (a, b) = (sub(e, w), sub(n, s));
        unit([
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ])
    };
    let side = i32::try_from(PATCH_QUADS + 1).ok()?;
    let tile = if near {
        crate::grid_rebuild::grid_tile(mesh, EARTH_RADIUS_M, relief)
    } else {
        None
    };
    let mut parent_normals: Vec<Option<[f32; 3]>> = Vec::with_capacity(mesh.offsets.len());
    for (index, today) in mesh.offsets.iter().copied().enumerate() {
        let (k, l) = steps_of(index)?;
        let (di, dj) = (
            k + crate::grid_rebuild::CENTRE_INDEX,
            l + crate::grid_rebuild::CENTRE_INDEX,
        );
        let dh = dh_at(di, dj)?;
        let exact = exact_offset(&frame, k, l, dh);
        let naive = naive_offset(&frame, k, l, dh, centre32);
        let (e, n) = (gap_m(exact, today), gap_m(naive, today));
        m.exact_far_share = m.exact_far_share.max(e / quad_m);
        m.naive_far_share = m.naive_far_share.max(n / quad_m);
        // Even points keep the k = 2 parent stencil; odd ones are filled below.
        parent_normals.push(if key.level > 0 && di % 2 == 0 && dj % 2 == 0 {
            normal_at(di, dj, 2)
        } else {
            None
        });
        if !near {
            continue;
        }
        m.exact_near_mm = m.exact_near_mm.max(e * 1_000.0);
        if let Some(tile) = tile.as_ref() {
            let shader = crate::grid_rebuild::tile_offset(tile, di, dj)?;
            m.tile_near_mm = m.tile_near_mm.max(gap_m(shader, today) * 1_000.0);
        }
        m.naive_near_mm = m.naive_near_mm.max(n * 1_000.0);
        match normal_at(di, dj, 1) {
            Some(rebuilt_n) => {
                let today_n = *mesh.normals.get(index)?;
                m.normal_near_deg = m.normal_near_deg.max(angle_deg(rebuilt_n, today_n));
            }
            None => m.fallback_normals += 1,
        }
    }
    if !near || key.level == 0 {
        return Some(());
    }
    for dj in 0..side {
        for di in 0..side {
            let index = usize::try_from(dj * side + di).ok()?;
            let today_p = *mesh.parent_offsets.get(index)?;
            let today_pn = *mesh.parent_normals.get(index)?;
            let (rebuilt_p, rebuilt_pn) = match parent_pair(di, dj) {
                None => (rebuilt(di, dj)?, *parent_normals.get(index)?),
                Some(((ai, aj), (bi, bj))) => {
                    let (a, b) = (rebuilt(ai, aj)?, rebuilt(bi, bj)?);
                    let mid = [
                        (a[0] + b[0]) * 0.5,
                        (a[1] + b[1]) * 0.5,
                        (a[2] + b[2]) * 0.5,
                    ];
                    let at = |i: i32, j: i32| -> Option<Option<[f32; 3]>> {
                        parent_normals
                            .get(usize::try_from(j * side + i).ok()?)
                            .copied()
                    };
                    let pn = match (at(ai, aj)?, at(bi, bj)?) {
                        (Some(x), Some(y)) => unit([
                            (x[0] + y[0]) * 0.5,
                            (x[1] + y[1]) * 0.5,
                            (x[2] + y[2]) * 0.5,
                        ]),
                        _ => None,
                    };
                    (mid, pn)
                }
            };
            m.parent_near_mm = m.parent_near_mm.max(gap_m(rebuilt_p, today_p) * 1_000.0);
            if let Some(pn) = rebuilt_pn {
                m.parent_normal_near_deg = m.parent_normal_near_deg.max(angle_deg(pn, today_pn));
            }
        }
    }
    Some(())
}

/// Settles at `dir` and measures every leaf (see the module docs).
#[must_use]
pub(crate) fn precision_at(relief: &Relief, dir: [f64; 3]) -> PrecisionMetrics {
    let camera = crate::headless_morph::camera_for(relief, HEIGHT_M, dir);
    let quad_px = crate::settings::PlanetDetail::Normal.quad_px();
    let ranges = crate::lod_range::LevelRanges::new(relief, EARTH_RADIUS_M, quad_px);
    let mut metrics = PrecisionMetrics::default();
    let Some(mut tree) = QuadTree::new(EARTH_RADIUS_M).map(|t| t.with_ranges(ranges)) else {
        return metrics;
    };
    tree.set_ring_m(RING_M);
    let mut built = Built::new();
    metrics.settled = settle(&mut tree, camera, relief, &mut built);
    if !metrics.settled {
        return metrics;
    }
    let lookup = |key: PatchKey| built.get(&key).map(PatchMesh::relief_info);
    for key in tree.leaves() {
        let Some(mesh) = built.get(&key) else {
            continue;
        };
        let near = crate::lod_ring::box_distance_m(key, camera, EARTH_RADIUS_M, &lookup)
            .is_some_and(|d| d <= NEAR_M);
        if near {
            metrics.near_leaves += 1;
        } else {
            metrics.far_leaves += 1;
        }
        metrics.deepest_level = metrics.deepest_level.max(key.level);
        if measure_leaf(mesh, relief, near, &mut metrics).is_none() {
            metrics.settled = false;
        }
    }
    metrics
}

#[cfg(test)]
mod tests {
    use super::{PrecisionMetrics, precision_at};
    use std::sync::OnceLock;

    /// Plains and mountain, measured once for the tests below.
    fn sites() -> &'static [PrecisionMetrics; 2] {
        static SITES: OnceLock<[PrecisionMetrics; 2]> = OnceLock::new();
        SITES.get_or_init(|| {
            let relief = crate::headless_morph::earth_relief();
            let (mountain, plains) = crate::headless_morph::earth_mountain_and_plains_dirs();
            [
                precision_at(&relief, plains),
                precision_at(&relief, mountain),
            ]
        })
    }

    #[test]
    #[ignore = "slow (~15 s on the dev PC): runs with --include-ignored (CI, validator), #847"]
    fn the_exact_rebuild_stays_under_a_millimetre_near_the_player() {
        // #920 T1, Spec AC2: the shared-grid offsets and morph parents
        // rebuilt in f32 from heights land under 1 mm from today's patches
        // within 500 m of the player, on the plains and the mountain.
        for m in sites() {
            assert!(m.settled && m.near_leaves > 0, "{m:?}");
            assert!(m.exact_near_mm < 1.0, "{m:?}");
            assert!(m.tile_near_mm < 1.0, "{m:?}");
            assert!(m.parent_near_mm < 1.0, "{m:?}");
        }
    }

    #[test]
    #[ignore = "slow (~15 s on the dev PC): runs with --include-ignored (CI, validator), #847"]
    fn the_naive_rebuild_misses_by_far_more() {
        // The reason for the exact form: the straightforward f32 rebuild
        // subtracts planet-sized numbers and misses by centimetres or more.
        for m in sites() {
            assert!(m.naive_near_mm > 10.0 * m.exact_near_mm.max(0.01), "{m:?}");
        }
    }

    #[test]
    #[ignore = "slow (~15 s on the dev PC): runs with --include-ignored (CI, validator), #847"]
    fn rebuilt_normals_match_todays() {
        // Normals from the rebuilt f32 positions stay within a tenth of a
        // degree of today's, own and parent level.
        for m in sites() {
            assert!(m.normal_near_deg < 0.1, "{m:?}");
            assert!(m.parent_normal_near_deg < 0.1, "{m:?}");
        }
    }
}
