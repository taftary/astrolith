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
use crate::headless_morph_measure::{
    EdgeBits, Frame, blend_frame_in, crack_stats_in, surface_change,
};
use crate::lod::{PatchKey, patch_centre_dir};
use crate::lod_mesh::{PatchMesh, build_patch};
use crate::lod_select::{QuadTree, UpdateScratch};
use crate::planet_frame::{EARTH_RADIUS_M, PlanetPos};
use crate::relief::Relief;
use std::collections::HashMap;

/// Metrics from the scripted morph path.
#[derive(Debug, Clone)]
pub(crate) struct MorphMetrics {
    /// Worst on-screen jump between frames beyond camera motion over the
    /// whole path (px, #817 Q6 A: no 3 km split).
    pub worst_popping_px: f64,
    /// Worst normal change between frames over the whole path (degrees).
    pub worst_normal_deg: f64,
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

pub(crate) fn earth_relief() -> Relief {
    let journey = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
    let chain: &[u32] = journey.get(..9).unwrap_or(&[]);
    Relief::for_planet(crate::nav::DEMO_SEED, chain)
}

pub(crate) fn camera_for(relief: &Relief, height_m: f64, dir: [f64; 3]) -> PlanetPos {
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

/// [`mountain_and_plains_dirs`] for the demo Earth, computed once per
/// process: the `MORPH` and `CULL` lines share it (#778, validation time).
pub(crate) fn earth_mountain_and_plains_dirs() -> ([f64; 3], [f64; 3]) {
    static DIRS: std::sync::OnceLock<([f64; 3], [f64; 3])> = std::sync::OnceLock::new();
    *DIRS.get_or_init(|| mountain_and_plains_dirs(&earth_relief()))
}

pub(crate) fn mountain_and_plains_dirs(relief: &Relief) -> ([f64; 3], [f64; 3]) {
    // The 384 level-3 builds run on every core; the scan below keeps their
    // face, x, y order, so the pick is the same as one by one.
    let mut keys = Vec::with_capacity(6 * 8 * 8);
    for face in Face::all() {
        for x in 0..8u32 {
            for y in 0..8u32 {
                if let Some(key) = PatchKey::new(face, 3, x, y) {
                    keys.push(key);
                }
            }
        }
    }
    let errors = crate::headless_morph_measure::par_chunks(&keys, |chunk| {
        chunk
            .iter()
            .map(|key| {
                build_patch(*key, EARTH_RADIUS_M, relief).map(|mesh| (*key, mesh.relief_error_m))
            })
            .collect::<Vec<_>>()
    });
    let mut best: Option<(f64, [f64; 3])> = None;
    let mut flat: Option<(f64, [f64; 3])> = None;
    for (key, err) in errors.into_iter().flatten().flatten() {
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
    (
        best.map(|(_, d)| d).unwrap_or([0.0, 1.0, 0.0]),
        flat.map(|(_, d)| d).unwrap_or([1.0, 0.0, 0.0]),
    )
}

/// Metrics that fail every check (bad radius).
fn failed() -> MorphMetrics {
    MorphMetrics {
        worst_popping_px: f64::INFINITY,
        worst_normal_deg: f64::INFINITY,
        cracks: 1,
        worst_gap_px: f64::INFINITY,
        mountain_depth: 0,
        plains_depth: 1,
        still_splits: 1,
        still_merges: 0,
    }
}

/// Builds every key of `keys` not built yet (memoized across frames).
pub(crate) fn build_keys(
    keys: &[PatchKey],
    relief: &Relief,
    memo: &mut HashMap<PatchKey, (f64, f64, f64)>,
    meshes: &mut HashMap<PatchKey, PatchMesh>,
) {
    let missing: Vec<PatchKey> = keys
        .iter()
        .copied()
        .filter(|key| !memo.contains_key(key))
        .collect();
    let built = crate::headless_morph_measure::par_chunks(&missing, |chunk| {
        chunk
            .iter()
            .map(|key| (*key, build_patch(*key, EARTH_RADIUS_M, relief)))
            .collect::<Vec<_>>()
    });
    for (key, mesh) in built.into_iter().flatten() {
        if let Some(mesh) = mesh {
            memo.insert(key, mesh.relief_info());
            meshes.insert(key, mesh);
        }
    }
}

/// Builds every leaf of `tree` not built yet (memoized across frames).
fn build_missing(
    tree: &QuadTree,
    relief: &Relief,
    memo: &mut HashMap<PatchKey, (f64, f64, f64)>,
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
            memo.insert(leaf, mesh.relief_info());
            meshes.insert(leaf, mesh);
        }
    }
}

/// Runs the scripted morph path and returns the metrics.
///
/// Heights descend from 2,000 km to 1 m and back over the mountain
/// direction at the default free-flight speed (step 5), one frame per
/// 1/60 s. Popping and normals are measured over the whole path (#817
/// Q6 A). Every frame runs the real culled CDLOD selection, builds
/// (memoized) and morph blending, as the window draws it.
pub(crate) fn morph_metrics() -> MorphMetrics {
    let relief = earth_relief();
    let (mountain_dir, plains_dir) = earth_mountain_and_plains_dirs();
    // Pixels per quad at Normal detail (#817 F1 A) and the planet's ranges.
    let detail_px = crate::settings::PlanetDetail::Normal.quad_px();
    let ranges = crate::lod_range::LevelRanges::new(&relief, EARTH_RADIUS_M, detail_px);
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
    let mut memo: HashMap<PatchKey, (f64, f64, f64)> = HashMap::new();
    let mut meshes: HashMap<PatchKey, PatchMesh> = HashMap::new();
    let Some(mut tree) = QuadTree::new(EARTH_RADIUS_M).map(|t| t.with_ranges(ranges.clone()))
    else {
        return failed();
    };
    let mut scratch = UpdateScratch::default();
    let mut prev: Option<Frame> = None;
    let mut edge_bits = EdgeBits::default();
    let mut visible: Vec<PatchKey> = Vec::new();
    let bounds = relief.height_bounds_m();
    let mountain_dir_unit = {
        let d = mountain_dir;
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        [d[0] / len, d[1] / len, d[2] / len]
    };
    // Any direction across the line of sight as the view's up.
    let side = {
        let u = mountain_dir_unit;
        let helper = if u[2].abs() < 0.9 {
            [0.0, 0.0, 1.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        [
            u[1] * helper[2] - u[2] * helper[1],
            u[2] * helper[0] - u[0] * helper[2],
            u[0] * helper[1] - u[1] * helper[0],
        ]
    };
    let mut worst_popping_px = 0.0f64;
    let mut worst_normal_deg = 0.0f64;
    let mut cracks = 0u32;
    let mut worst_gap_px = 0.0f64;
    for height in heights.iter().copied() {
        let camera = camera_for(&relief, height, mountain_dir);
        // Culled selection as the window draws it (#778, #817: only drawn
        // patches are built and measured), looking down at the planet;
        // builds for new visible leaves, one more cascade step.
        let view = crate::lod_cull::View::new(
            camera,
            [
                -mountain_dir_unit[0],
                -mountain_dir_unit[1],
                -mountain_dir_unit[2],
            ],
            side,
            crate::precision::FOV_DEG.to_radians(),
            16.0 / 9.0,
        );
        for _ in 0..2 {
            {
                let memo_ref = &memo;
                let heights = |key: PatchKey| memo_ref.get(&key).map(|r| (r.1, r.2));
                let cull = view.map(|view| crate::lod_cull::Cull {
                    view,
                    heights: &heights,
                    fallback: bounds,
                    occluder_m: EARTH_RADIUS_M + bounds.0,
                });
                tree.update_culled_reused(
                    camera,
                    detail_px,
                    &|key| memo_ref.get(&key).copied(),
                    cull.as_ref(),
                    &mut scratch,
                );
            }
            tree.append_visible_leaves_to(&mut visible);
            build_keys(&visible, &relief, &mut memo, &mut meshes);
        }
        let cover = tree.leaves();
        let frame = blend_frame_in(&visible, &cover, &meshes, camera, detail_px);
        let neighbour_drawn = |key: PatchKey, edge: usize| {
            use crate::lod_select::{NeighbourDir, neighbour_at_same_level};
            let dir = match edge {
                0 => NeighbourDir::South,
                1 => NeighbourDir::North,
                2 => NeighbourDir::West,
                _ => NeighbourDir::East,
            };
            neighbour_at_same_level(key, dir)
                .and_then(|n| tree.covering_leaf(n))
                .is_some_and(|leaf| visible.binary_search(&leaf).is_ok())
        };
        let (frame_cracks, gap) = crack_stats_in(&frame, camera, &mut edge_bits, &neighbour_drawn);
        cracks += frame_cracks;
        worst_gap_px = worst_gap_px.max(gap);
        if let Some(prev) = prev.as_ref() {
            let (px, deg) = surface_change(prev, &frame, camera);
            worst_popping_px = worst_popping_px.max(px);
            worst_normal_deg = worst_normal_deg.max(deg);
        }
        prev = Some(frame);
    }
    // Mountain versus plains depth at 50 km (60 frames, relief-aware).
    let height = 50_000.0;
    let mountain_cam = camera_for(&relief, height, mountain_dir);
    let plains_cam = camera_for(&relief, height, plains_dir);
    let mut mountain_tree = match QuadTree::new(EARTH_RADIUS_M) {
        Some(tree) => tree.with_ranges(ranges.clone()),
        None => return failed(),
    };
    let mut plains_tree = match QuadTree::new(EARTH_RADIUS_M) {
        Some(tree) => tree.with_ranges(ranges.clone()),
        None => return failed(),
    };
    let mut mountain_memo: HashMap<PatchKey, (f64, f64, f64)> = HashMap::new();
    let mut plains_memo: HashMap<PatchKey, (f64, f64, f64)> = HashMap::new();
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
    let popping_ok = metrics.worst_popping_px.is_finite() && metrics.worst_popping_px < 1.0;
    let normal_ok = metrics.worst_normal_deg.is_finite() && metrics.worst_normal_deg < 2.0;
    let cracks_ok = metrics.cracks == 0 && metrics.worst_gap_px < 0.5;
    let depth_ok = metrics.mountain_depth > metrics.plains_depth;
    let still_ok = metrics.still_splits == 0 && metrics.still_merges == 0;
    let passed = popping_ok && normal_ok && cracks_ok && depth_ok && still_ok;
    let _ = writeln!(
        out,
        "MORPH popping_px={:.3} normal_deg={:.3} cracks={} gap_px={:.3} mountain_depth={} plains_depth={} still_splits={} still_merges={} {}",
        metrics.worst_popping_px,
        metrics.worst_normal_deg,
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
