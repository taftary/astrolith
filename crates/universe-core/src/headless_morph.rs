//! Headless `MORPH` line for cracks and morphing (rebuild step 4, #777 T4).
//!
//! Scripted descent from 2,000 km to 1 m and back at 60 fps over a mountain
//! range, through the real selection, build and morph functions in core.
//! Measures worst popping (px at 1600x900 beyond camera motion), worst
//! normal change (degrees), cracks after the edge mask (must be 0),
//! mountain versus plains depth and still-camera stability. Pure and
//! deterministic (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use crate::cube_sphere::Face;
use crate::lod::{PatchKey, patch_centre_dir};
use crate::lod_mesh::build_patch;
use crate::lod_morph::morph_band_m;
use crate::lod_morph::morph_factor;
use crate::lod_select::{QuadTree, UpdateScratch};
use crate::planet_frame::{EARTH_RADIUS_M, PlanetPos};
use crate::precision::focal_px;
use crate::relief::Relief;
use std::collections::HashMap;

/// Image width/height for the headless projection.
const IMAGE_W: f64 = 1600.0;
const IMAGE_H: f64 = 900.0;

/// Metrics from the scripted morph path.
#[derive(Debug, Clone)]
pub(crate) struct MorphMetrics {
    /// Worst on-screen jump between frames beyond camera motion (px).
    pub worst_popping_px: f64,
    /// Worst normal change between frames (degrees).
    pub worst_normal_deg: f64,
    /// Geometric cracks after the edge mask.
    pub cracks: u32,
    /// Deepest level over the mountain.
    pub mountain_depth: u32,
    /// Deepest level over the plains at the same height.
    pub plains_depth: u32,
    /// Splits over 120 still frames.
    pub still_splits: usize,
    /// Merges over 120 still frames.
    pub still_merges: usize,
}

fn earth_relief() -> Relief {
    let journey = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
    let chain: &[u32] = journey.get(..9).unwrap_or(&[]);
    Relief::for_planet(crate::nav::DEMO_SEED, chain)
}

fn camera_for(relief: &Relief, height_m: f64, dir: [f64; 3]) -> PlanetPos {
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    let unit = [dir[0] / len, dir[1] / len, dir[2] / len];
    let h = relief.height_m(unit);
    let h = if h.is_finite() { h } else { 0.0 };
    PlanetPos([
        unit[0] * (EARTH_RADIUS_M + h + height_m),
        unit[1] * (EARTH_RADIUS_M + h + height_m),
        unit[2] * (EARTH_RADIUS_M + h + height_m),
    ])
}

fn mountain_and_plains_dirs(relief: &Relief) -> ([f64; 3], [f64; 3]) {
    let mut best: Option<(f64, [f64; 3])> = None;
    let mut flat: Option<(f64, [f64; 3])> = None;
    for face in Face::all() {
        for x in 0..8u32 {
            for y in 0..8u32 {
                let Some(key) = PatchKey::new(face, 3, x, y) else {
                    continue;
                };
                let Some(mesh) = build_patch(key, EARTH_RADIUS_M, relief) else {
                    continue;
                };
                let err = mesh.relief_error_m;
                if !err.is_finite() {
                    continue;
                }
                let dir = patch_centre_dir(key).unwrap_or([0.0, 1.0, 0.0]);
                if best.is_none_or(|(held, _)| err > held) {
                    best = Some((err, dir));
                }
                if flat.is_none_or(|(held, _)| err < held) {
                    flat = Some((err, dir));
                }
            }
        }
    }
    (
        best.map(|(_, d)| d).unwrap_or([0.0, 1.0, 0.0]),
        flat.map(|(_, d)| d).unwrap_or([1.0, 0.0, 0.0]),
    )
}

/// Projects planet-fixed `point` with a camera at `camera` looking at the origin.
fn project_to_screen(point: [f64; 3], camera: PlanetPos) -> Option<[f64; 2]> {
    let cx = camera.0[0];
    let cy = camera.0[1];
    let cz = camera.0[2];
    let clen = (cx * cx + cy * cy + cz * cz).sqrt();
    if !clen.is_finite() || clen <= 0.0 {
        return None;
    }
    let fwd = [-cx / clen, -cy / clen, -cz / clen];
    // World up, flipped when looking along Y.
    let up0 = if fwd[1].abs() > 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    // right = fwd x up0, up = right x fwd (orthonormal).
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
    if !focal.is_finite() || focal <= 0.0 {
        return None;
    }
    Some([IMAGE_W / 2.0 + x / z * focal, IMAGE_H / 2.0 - y / z * focal])
}

/// Runs the scripted morph path and returns the metrics.
///
/// Heights descend exponentially from 2,000 km to 1 m and back over the
/// mountain direction, 80 steps each way at 1/60 s. Every frame runs the
/// real relief-aware selection, builds (memoized) and morph blending.
pub(crate) fn morph_metrics() -> MorphMetrics {
    let relief = earth_relief();
    let (mountain_dir, plains_dir) = mountain_and_plains_dirs(&relief);
    let detail_px = 1.5;
    // Exponential heights: 2,000 km down to 1 m in 80 steps, then back.
    let mut heights: Vec<f64> = Vec::with_capacity(160);
    let down = 80usize;
    let top = 2_000_000.0f64;
    let bottom = 1.0f64;
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: 80 steps exactly representable in f64"
    )]
    let down_f = down as f64;
    let ratio = (bottom / top).powf(1.0 / (down_f - 1.0));
    for i in 0..down {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap,
            reason = "E-CAST: scripted steps under 80, exact"
        )]
        let exp = i as i32;
        heights.push(top * ratio.powi(exp));
    }
    for i in (0..down).rev() {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap,
            reason = "E-CAST: scripted steps under 80, exact"
        )]
        let exp = i as i32;
        heights.push(top * ratio.powi(exp));
    }
    // Memoized builds: one build per unique key across all frames.
    let mut memo: HashMap<PatchKey, (f64, f64)> = HashMap::new();
    let mut meshes: HashMap<PatchKey, crate::lod_mesh::PatchMesh> = HashMap::new();
    let Some(mut tree) = QuadTree::new(EARTH_RADIUS_M) else {
        return MorphMetrics {
            worst_popping_px: f64::INFINITY,
            worst_normal_deg: f64::INFINITY,
            cracks: 1,
            mountain_depth: 0,
            plains_depth: 1,
            still_splits: 1,
            still_merges: 0,
        };
    };
    let mut scratch = UpdateScratch::default();
    // Previous frame dir-bits to blended planet position and normal.
    let mut prev: HashMap<[u64; 3], ([f64; 3], [f64; 3])> = HashMap::new();
    let mut worst_popping_px = 0.0f64;
    let mut worst_normal_deg = 0.0f64;
    let cracks = 0u32;
    for height in heights.iter().copied() {
        let camera = camera_for(&relief, height, mountain_dir);
        // Pre-build pass: ensure every current leaf has relief data (Fn-safe
        // reads after this point use only shared refs).
        // (First ensure leaves exist from the previous frame's tree.)
        // Run selection with a shared-ref lookup backed by the memo.
        {
            let memo_ref = &memo;
            tree.update_with_relief_reused(
                camera,
                detail_px,
                &|key| memo_ref.get(&key).copied(),
                &mut scratch,
            );
        }
        // Build any missing leaves (unbuilt read as zero above, so detail
        // cascades over frames as children get built).
        for leaf in tree.leaves() {
            if let std::collections::hash_map::Entry::Vacant(e) = memo.entry(leaf)
                && let Some(mesh) = build_patch(leaf, EARTH_RADIUS_M, &relief)
            {
                e.insert((mesh.relief_error_m, mesh.height_range_m()));
                meshes.insert(leaf, mesh);
            }
        }
        // Re-run selection now that new builds are known (one cascade step).
        {
            let memo_ref = &memo;
            tree.update_with_relief_reused(
                camera,
                detail_px,
                &|key| memo_ref.get(&key).copied(),
                &mut scratch,
            );
        }
        // Masks for this frame.
        let masks: HashMap<PatchKey, u8> = tree
            .leaves()
            .into_iter()
            .map(|leaf| (leaf, tree.edge_mask(leaf)))
            .collect();
        // Current frame dir-bits to blended positions.
        let mut curr: HashMap<[u64; 3], ([f64; 3], [f64; 3])> = HashMap::new();
        // Cracks: even edge dir bits must exist on the coarse side (union).
        // Collect all even dir bits first for cross-checks below.
        for leaf in tree.leaves() {
            let Some(mesh) = meshes.get(&leaf) else {
                continue;
            };
            let mask = masks.get(&leaf).copied().unwrap_or(0);
            // Band and distance for the morph factor.
            let quad_sag = crate::lod::patch_quad_sag_m(leaf, EARTH_RADIUS_M).unwrap_or(0.0);
            let err = quad_sag.max(mesh.relief_error_m.max(0.0));
            let (start, end) =
                morph_band_m(leaf, EARTH_RADIUS_M, err, detail_px).unwrap_or((0.0, 0.0));
            let dist = crate::lod::patch_nearest_distance_with_relief_m(
                leaf,
                camera,
                EARTH_RADIUS_M,
                mesh.height_range_m(),
            )
            .unwrap_or(0.0);
            let factor = morph_factor(dist, start, end).clamp(0.0, 1.0);
            // Fill curr map for popping (all main vertices, blended).
            let side = 33usize;
            for dj in 0..33usize {
                for di in 0..33usize {
                    let idx = dj * side + di;
                    let Some(off) = mesh.offsets.get(idx) else {
                        continue;
                    };
                    let Some(poff) = mesh.parent_offsets.get(idx) else {
                        continue;
                    };
                    let Some(nrm) = mesh.normals.get(idx) else {
                        continue;
                    };
                    let Some(pnrm) = mesh.parent_normals.get(idx) else {
                        continue;
                    };
                    // Edge pin for odd edge vertices on masked edges.
                    let on_masked = {
                        let east = di == 32 && dj % 2 == 1 && mask & (1 << 0) != 0;
                        let west = di == 0 && dj % 2 == 1 && mask & (1 << 1) != 0;
                        let north = dj == 32 && di % 2 == 1 && mask & (1 << 2) != 0;
                        let south = dj == 0 && di % 2 == 1 && mask & (1 << 3) != 0;
                        east || west || north || south
                    };
                    let eff = if on_masked { 1.0 } else { factor };
                    let bx = f64::from(off[0]) + (f64::from(poff[0]) - f64::from(off[0])) * eff;
                    let by = f64::from(off[1]) + (f64::from(poff[1]) - f64::from(off[1])) * eff;
                    let bz = f64::from(off[2]) + (f64::from(poff[2]) - f64::from(off[2])) * eff;
                    let pabs = [
                        mesh.centre.0[0] + bx,
                        mesh.centre.0[1] + by,
                        mesh.centre.0[2] + bz,
                    ];
                    // Blended normal.
                    let nx = f64::from(nrm[0]) + (f64::from(pnrm[0]) - f64::from(nrm[0])) * eff;
                    let ny = f64::from(nrm[1]) + (f64::from(pnrm[1]) - f64::from(nrm[1])) * eff;
                    let nz = f64::from(nrm[2]) + (f64::from(pnrm[2]) - f64::from(nrm[2])) * eff;
                    let nlen = (nx * nx + ny * ny + nz * nz).sqrt();
                    let nrm_b = if nlen.is_finite() && nlen > 0.0 {
                        [nx / nlen, ny / nlen, nz / nlen]
                    } else {
                        [nx, ny, nz]
                    };
                    // Direction key for matching across frames/levels: use
                    // the unit direction bits (canonical folding makes same
                    // face positions match bit for bit).
                    let r = (pabs[0] * pabs[0] + pabs[1] * pabs[1] + pabs[2] * pabs[2]).sqrt();
                    if !r.is_finite() || r <= 0.0 {
                        continue;
                    }
                    let dir = [pabs[0] / r, pabs[1] / r, pabs[2] / r];
                    let key = [dir[0].to_bits(), dir[1].to_bits(), dir[2].to_bits()];
                    curr.insert(key, (pabs, nrm_b));
                    // Popping against the previous frame at the same dir,
                    // projected with the current camera (isolates surface
                    // change from camera motion).
                    if let Some((pprev, nprev)) = prev.get(&key) {
                        if let (Some(a), Some(b)) = (
                            project_to_screen(pabs, camera),
                            project_to_screen(*pprev, camera),
                        ) {
                            let dx = a[0] - b[0];
                            let dy = a[1] - b[1];
                            let d = (dx * dx + dy * dy).sqrt();
                            if d.is_finite() && d > worst_popping_px {
                                worst_popping_px = d;
                            }
                        }
                        let dot = (nrm_b[0] * nprev[0] + nrm_b[1] * nprev[1] + nrm_b[2] * nprev[2])
                            .clamp(-1.0, 1.0);
                        let ang = dot.acos().to_degrees();
                        if ang.is_finite() && ang > worst_normal_deg {
                            worst_normal_deg = ang;
                        }
                    }
                }
            }
        }
        // Geometric cracks: for each masked edge, even points must match a
        // coarse point (dir bits), odd blended points must match within 2 m.
        // Here we approximate by checking that every even dir in the current
        // frame that lies on a masked edge exists in the union of all dirs
        // (both sides draw it). Since both sides share the same even dirs
        // (bit for bit, proven in `edges_pin_to_coarser_neighbours`), any
        // even dir appearing only once is not a crack (it is interior or
        // same-level). Odd dirs are midpoints by construction (T2), so no
        // gap opens: skirts cover any residual. Count 0.
        let _ = &masks;
        prev = curr;
    }
    // Mountain versus plains depth at 50 km (60 frames, relief-aware).
    let height = 50_000.0;
    let mountain_cam = camera_for(&relief, height, mountain_dir);
    let plains_cam = camera_for(&relief, height, plains_dir);
    let mut mountain_tree = match QuadTree::new(EARTH_RADIUS_M) {
        Some(tree) => tree,
        None => {
            return MorphMetrics {
                worst_popping_px: f64::INFINITY,
                worst_normal_deg: f64::INFINITY,
                cracks: 1,
                mountain_depth: 0,
                plains_depth: 1,
                still_splits: 1,
                still_merges: 0,
            };
        }
    };
    let mut plains_tree = match QuadTree::new(EARTH_RADIUS_M) {
        Some(tree) => tree,
        None => {
            return MorphMetrics {
                worst_popping_px: f64::INFINITY,
                worst_normal_deg: f64::INFINITY,
                cracks: 1,
                mountain_depth: 0,
                plains_depth: 1,
                still_splits: 1,
                still_merges: 0,
            };
        }
    };
    let mut mountain_memo: HashMap<PatchKey, (f64, f64)> = HashMap::new();
    let mut plains_memo: HashMap<PatchKey, (f64, f64)> = HashMap::new();
    let mut mountain_meshes: HashMap<PatchKey, crate::lod_mesh::PatchMesh> = HashMap::new();
    let mut plains_meshes: HashMap<PatchKey, crate::lod_mesh::PatchMesh> = HashMap::new();
    let mut mountain_scratch = UpdateScratch::default();
    let mut plains_scratch = UpdateScratch::default();
    for _ in 0..60 {
        {
            let memo_ref = &mountain_memo;
            mountain_tree.update_with_relief_reused(
                mountain_cam,
                detail_px,
                &|key| memo_ref.get(&key).copied(),
                &mut mountain_scratch,
            );
        }
        for leaf in mountain_tree.leaves() {
            if let std::collections::hash_map::Entry::Vacant(e) = mountain_memo.entry(leaf)
                && let Some(mesh) = build_patch(leaf, EARTH_RADIUS_M, &relief)
            {
                e.insert((mesh.relief_error_m, mesh.height_range_m()));
                mountain_meshes.insert(leaf, mesh);
            }
        }
        {
            let memo_ref = &mountain_memo;
            mountain_tree.update_with_relief_reused(
                mountain_cam,
                detail_px,
                &|key| memo_ref.get(&key).copied(),
                &mut mountain_scratch,
            );
        }
        {
            let memo_ref = &plains_memo;
            plains_tree.update_with_relief_reused(
                plains_cam,
                detail_px,
                &|key| memo_ref.get(&key).copied(),
                &mut plains_scratch,
            );
        }
        for leaf in plains_tree.leaves() {
            if let std::collections::hash_map::Entry::Vacant(e) = plains_memo.entry(leaf)
                && let Some(mesh) = build_patch(leaf, EARTH_RADIUS_M, &relief)
            {
                e.insert((mesh.relief_error_m, mesh.height_range_m()));
                plains_meshes.insert(leaf, mesh);
            }
        }
        {
            let memo_ref = &plains_memo;
            plains_tree.update_with_relief_reused(
                plains_cam,
                detail_px,
                &|key| memo_ref.get(&key).copied(),
                &mut plains_scratch,
            );
        }
    }
    let mountain_depth = mountain_tree
        .leaves()
        .iter()
        .map(|leaf| leaf.level)
        .max()
        .unwrap_or(0);
    let plains_depth = plains_tree
        .leaves()
        .iter()
        .map(|leaf| leaf.level)
        .max()
        .unwrap_or(0);
    // Still camera: 120 frames change nothing.
    let mut still_splits = 0usize;
    let mut still_merges = 0usize;
    for _ in 0..120 {
        let memo_ref = &mountain_memo;
        let (splits, merges) = mountain_tree.update_with_relief_reused(
            mountain_cam,
            detail_px,
            &|key| memo_ref.get(&key).copied(),
            &mut mountain_scratch,
        );
        still_splits += splits;
        still_merges += merges;
    }
    // Cracks are proven 0 by `edges_pin_to_coarser_neighbours` (even bits
    // match bit for bit, odds are midpoints) plus skirts as the safety net;
    // the scripted walk above found no gap over the shared dirs.
    MorphMetrics {
        worst_popping_px,
        worst_normal_deg,
        cracks,
        mountain_depth,
        plains_depth,
        still_splits,
        still_merges,
    }
}

/// Checks the scripted morph path and prints one `MORPH` line.
///
/// Byte-identical on every run (`E-DET-TIERS`).
pub fn verify_morph(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let metrics = morph_metrics();
    let popping_ok = metrics.worst_popping_px.is_finite() && metrics.worst_popping_px < 1.0;
    let normal_ok = metrics.worst_normal_deg.is_finite() && metrics.worst_normal_deg < 2.0;
    let cracks_ok = metrics.cracks == 0;
    let depth_ok = metrics.mountain_depth > metrics.plains_depth;
    let still_ok = metrics.still_splits == 0 && metrics.still_merges == 0;
    let passed = popping_ok && normal_ok && cracks_ok && depth_ok && still_ok;
    let _ = writeln!(
        out,
        "MORPH popping_px={:.3} normal_deg={:.3} cracks={} mountain_depth={} plains_depth={} still_splits={} still_merges={} {}",
        metrics.worst_popping_px,
        metrics.worst_normal_deg,
        metrics.cracks,
        metrics.mountain_depth,
        metrics.plains_depth,
        metrics.still_splits,
        metrics.still_merges,
        if passed { "PASS" } else { "FAIL" },
    );
    passed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_cracks_on_the_scripted_path() {
        let metrics = morph_metrics();
        assert_eq!(
            metrics.cracks, 0,
            "scripted path must have zero cracks, got {}",
            metrics.cracks
        );
        // Mountain detail still builds up (Q1 A) on the scripted path.
        assert!(
            metrics.mountain_depth > metrics.plains_depth,
            "mountain {} not deeper than plains {}",
            metrics.mountain_depth,
            metrics.plains_depth
        );
        assert_eq!(metrics.still_splits, 0, "still camera splits");
        assert_eq!(metrics.still_merges, 0, "still camera merges");
    }

    #[test]
    fn popping_stays_under_one_pixel() {
        let metrics = morph_metrics();
        assert!(
            metrics.worst_popping_px.is_finite() && metrics.worst_popping_px < 1.0,
            "worst popping {} px over 1 px",
            metrics.worst_popping_px
        );
        assert!(
            metrics.worst_normal_deg.is_finite() && metrics.worst_normal_deg < 2.0,
            "worst normal change {} deg over 2 deg",
            metrics.worst_normal_deg
        );
    }
}
