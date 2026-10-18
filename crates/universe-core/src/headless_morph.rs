//! Headless `MORPH` line for cracks and morphing (rebuild step 4, #777 T4).
//!
//! Scripted descent from 2,000 km to 1 m and back at 60 fps over a mountain
//! range, through the real selection, build and per-vertex morph functions
//! in core (the formula the shader mirrors). Every frame is blended and
//! measured by grid position (`headless_morph_measure`): worst popping (px at
//! 1600x900 beyond camera motion), worst normal change (degrees), cracks
//! and the worst edge gap (must be 0 and under 0.5 px), then mountain versus
//! plains depth and still-camera stability. Pure and deterministic
//! (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use crate::cube_sphere::Face;
use crate::headless_morph_measure::{EdgeBits, Frame, blend_frame, crack_stats, surface_change};
use crate::lod::{PatchKey, patch_centre_dir};
use crate::lod_mesh::{PatchMesh, build_patch};
use crate::lod_select::{QuadTree, UpdateScratch};
use crate::planet_frame::{EARTH_RADIUS_M, PlanetPos};
use crate::relief::Relief;
use std::collections::HashMap;

/// Metrics from the scripted morph path.
#[derive(Debug, Clone)]
pub(crate) struct MorphMetrics {
    /// Worst on-screen jump between frames beyond camera motion, below
    /// 3 km (px).
    pub worst_popping_px: f64,
    /// Worst normal change between frames (degrees), below 3 km.
    pub worst_normal_deg: f64,
    /// Worst on-screen jump at or above 3 km (px).
    pub high_popping_px: f64,
    /// Worst normal change at or above 3 km (degrees, reported only).
    pub high_normal_deg: f64,
    /// Edge points whose drawn copies are 0.5 px or more apart.
    pub cracks: u32,
    /// Worst gap between drawn copies of an edge point (px).
    pub worst_gap_px: f64,
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

/// Camera height from which popping may reach [`HIGH_POPPING_PX`] and
/// normals are reported only (owner decision on #777, option 2: up high
/// several level bands overlap, and the switch pops go with the selection
/// rewrite to true CDLOD ranges, a later Issue).
pub const HIGH_ALTITUDE_M: f64 = 3_000.0;

/// Popping limit at or above [`HIGH_ALTITUDE_M`] (px).
pub const HIGH_POPPING_PX: f64 = 2.5;

/// Metrics that fail every check (bad radius).
fn failed() -> MorphMetrics {
    MorphMetrics {
        worst_popping_px: f64::INFINITY,
        worst_normal_deg: f64::INFINITY,
        high_popping_px: f64::INFINITY,
        high_normal_deg: f64::INFINITY,
        cracks: 1,
        worst_gap_px: f64::INFINITY,
        mountain_depth: 0,
        plains_depth: 1,
        still_splits: 1,
        still_merges: 0,
    }
}

/// Builds every leaf of `tree` not built yet (memoized across frames).
fn build_missing(
    tree: &QuadTree,
    relief: &Relief,
    memo: &mut HashMap<PatchKey, (f64, f64)>,
    meshes: &mut HashMap<PatchKey, PatchMesh>,
) {
    // Builds are independent: run them on every core, insert in leaf order.
    let missing: Vec<PatchKey> = tree
        .leaves()
        .into_iter()
        .filter(|leaf| !memo.contains_key(leaf))
        .collect();
    let built = crate::headless_morph_measure::par_chunks(&missing, |chunk| {
        chunk
            .iter()
            .map(|leaf| (*leaf, build_patch(*leaf, EARTH_RADIUS_M, relief)))
            .collect::<Vec<_>>()
    });
    for (leaf, mesh) in built.into_iter().flatten() {
        if let Some(mesh) = mesh {
            memo.insert(leaf, (mesh.relief_error_m, mesh.height_range_m()));
            meshes.insert(leaf, mesh);
        }
    }
}

/// Runs the scripted morph path and returns the metrics.
///
/// Heights descend from 2,000 km to 1 m and back over the mountain
/// direction at the default free-flight speed (step 5), one frame per
/// 1/60 s. Popping and normals are kept apart below and above
/// [`HIGH_ALTITUDE_M`] (owner decision on #777, option 2). Every frame runs the
/// real relief-aware selection, builds (memoized) and morph blending.
pub(crate) fn morph_metrics() -> MorphMetrics {
    let relief = earth_relief();
    let (mountain_dir, plains_dir) = mountain_and_plains_dirs(&relief);
    let detail_px = 1.5;
    // Free flight at the default speed step (5): speed equals the height
    // above the ground, so the height shrinks by `e^(-1/60)` per frame at
    // 60 fps, 2,000 km to 1 m in about 14.5 s (871 frames), then back up.
    let top = 2_000_000.0f64;
    let bottom = 1.0f64;
    let ratio = (-1.0f64 / 60.0).exp();
    let mut heights: Vec<f64> = Vec::new();
    let mut height = top;
    while height > bottom {
        heights.push(height);
        height *= ratio;
    }
    heights.push(bottom);
    let climb: Vec<f64> = heights.iter().rev().skip(1).copied().collect();
    heights.extend(climb);
    // Memoized builds: one build per unique key across all frames.
    let mut memo: HashMap<PatchKey, (f64, f64)> = HashMap::new();
    let mut meshes: HashMap<PatchKey, PatchMesh> = HashMap::new();
    let Some(mut tree) = QuadTree::new(EARTH_RADIUS_M) else {
        return failed();
    };
    let mut scratch = UpdateScratch::default();
    let mut prev: Option<Frame> = None;
    let mut edge_bits = EdgeBits::default();
    let mut worst_popping_px = 0.0f64;
    let mut worst_normal_deg = 0.0f64;
    let mut high_popping_px = 0.0f64;
    let mut high_normal_deg = 0.0f64;
    let mut cracks = 0u32;
    let mut worst_gap_px = 0.0f64;
    for height in heights.iter().copied() {
        let camera = camera_for(&relief, height, mountain_dir);
        // Selection, builds for new leaves (unbuilt read as zero, so detail
        // cascades over frames), one more cascade step, then every leaf
        // built so the whole tree is drawn.
        for _ in 0..2 {
            {
                let memo_ref = &memo;
                tree.update_with_relief_reused(
                    camera,
                    detail_px,
                    &|key| memo_ref.get(&key).copied(),
                    &mut scratch,
                );
            }
            build_missing(&tree, &relief, &mut memo, &mut meshes);
        }
        let frame = blend_frame(&tree.leaves(), &meshes, camera, detail_px);
        let (frame_cracks, gap) = crack_stats(&frame, camera, &mut edge_bits);
        cracks += frame_cracks;
        worst_gap_px = worst_gap_px.max(gap);
        if let Some(prev) = prev.as_ref() {
            let (px, deg) = surface_change(prev, &frame, camera);
            if height < HIGH_ALTITUDE_M {
                worst_popping_px = worst_popping_px.max(px);
                worst_normal_deg = worst_normal_deg.max(deg);
            } else {
                high_popping_px = high_popping_px.max(px);
                high_normal_deg = high_normal_deg.max(deg);
            }
        }
        prev = Some(frame);
    }
    // Mountain versus plains depth at 50 km (60 frames, relief-aware).
    let height = 50_000.0;
    let mountain_cam = camera_for(&relief, height, mountain_dir);
    let plains_cam = camera_for(&relief, height, plains_dir);
    let mut mountain_tree = match QuadTree::new(EARTH_RADIUS_M) {
        Some(tree) => tree,
        None => return failed(),
    };
    let mut plains_tree = match QuadTree::new(EARTH_RADIUS_M) {
        Some(tree) => tree,
        None => return failed(),
    };
    let mut mountain_memo: HashMap<PatchKey, (f64, f64)> = HashMap::new();
    let mut plains_memo: HashMap<PatchKey, (f64, f64)> = HashMap::new();
    let mut mountain_meshes: HashMap<PatchKey, PatchMesh> = HashMap::new();
    let mut plains_meshes: HashMap<PatchKey, PatchMesh> = HashMap::new();
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
        build_missing(
            &mountain_tree,
            &relief,
            &mut mountain_memo,
            &mut mountain_meshes,
        );
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
        build_missing(&plains_tree, &relief, &mut plains_memo, &mut plains_meshes);
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
    MorphMetrics {
        worst_popping_px,
        worst_normal_deg,
        high_popping_px,
        high_normal_deg,
        cracks,
        worst_gap_px,
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
    let popping_ok = metrics.worst_popping_px.is_finite()
        && metrics.worst_popping_px < 1.0
        && metrics.high_popping_px.is_finite()
        && metrics.high_popping_px < HIGH_POPPING_PX;
    let normal_ok = metrics.worst_normal_deg.is_finite() && metrics.worst_normal_deg < 2.0;
    let cracks_ok = metrics.cracks == 0 && metrics.worst_gap_px < 0.5;
    let depth_ok = metrics.mountain_depth > metrics.plains_depth;
    let still_ok = metrics.still_splits == 0 && metrics.still_merges == 0;
    let passed = popping_ok && normal_ok && cracks_ok && depth_ok && still_ok;
    let _ = writeln!(
        out,
        "MORPH popping_px={:.3} normal_deg={:.3} high_popping_px={:.3} high_normal_deg={:.3} cracks={} gap_px={:.3} mountain_depth={} plains_depth={} still_splits={} still_merges={} {}",
        metrics.worst_popping_px,
        metrics.worst_normal_deg,
        metrics.high_popping_px,
        metrics.high_normal_deg,
        metrics.cracks,
        metrics.worst_gap_px,
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
    fn scripted_path_has_no_cracks_and_smooth_morphs() {
        // AC5, AC6 (below 3 km), AC1 (#777 review items 1 and 2): measured,
        // not constant.
        // One run (the path is slow) checks every number.
        let metrics = morph_metrics();
        assert_eq!(metrics.cracks, 0, "cracks on the scripted path");
        assert!(
            metrics.worst_gap_px < 0.5,
            "worst edge gap {} px",
            metrics.worst_gap_px
        );
        // Popping and normals are real measures: something moves on a
        // descent through twenty levels, but never past the targets.
        assert!(
            metrics.worst_popping_px > 0.0 && metrics.worst_popping_px < 1.0,
            "worst popping {} px",
            metrics.worst_popping_px
        );
        assert!(
            metrics.worst_normal_deg > 0.0 && metrics.worst_normal_deg < 2.0,
            "worst normal change {} deg",
            metrics.worst_normal_deg
        );
        // Up high: popping under the agreed 2.5 px (#777 option 2).
        assert!(
            metrics.high_popping_px < HIGH_POPPING_PX,
            "worst popping above 3 km {} px",
            metrics.high_popping_px
        );
        assert!(
            metrics.mountain_depth > metrics.plains_depth,
            "mountain {} not deeper than plains {}",
            metrics.mountain_depth,
            metrics.plains_depth
        );
        assert_eq!(metrics.still_splits, 0, "still camera splits");
        assert_eq!(metrics.still_merges, 0, "still camera merges");
    }
}
