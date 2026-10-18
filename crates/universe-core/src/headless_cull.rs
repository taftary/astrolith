//! Headless `CULL` and `DEPTH` lines for depth and culling (rebuild step 5,
//! #778 Q7 A, Q8 A).
//!
//! `CULL` flies the #777 descent and climb (2,000 km to 1 m and back over the
//! mountain direction at the default free-flight speed) with a view tilted
//! 20° below the horizon, through the culled selection. Builds land at most
//! [`BUILD_BUDGET`](crate::headless_cull::BUILD_BUDGET) per frame, as in the window. It prints the worst visible
//! patches, terrain triangles and builds in one frame, the culled leaves,
//! and two safety counts that must be 0: drawn patches the view cannot see,
//! and hidden patches with a grid point in view above the horizon.
//!
//! `DEPTH` checks the `f32` reverse-Z depth `near / distance` (Bevy's
//! infinite projection) with the real near-plane rule at 1 cm, 1 m and
//! 1,000 km: two surfaces 1 mm apart near the ground, and one pixel apart on
//! the ground at 1,000 km, must get different depths.
//!
//! Deterministic (`E-DET-TIERS`), std only (`E-CORE-NO-BEVY`).

use crate::cube_sphere::grid_point;
use crate::lod::{PATCH_QUADS, PatchKey};
use crate::lod_cull::{Cull, View, patch_visible};
use crate::lod_mesh::{PatchMesh, build_patch};
use crate::lod_select::{QuadTree, UpdateScratch};
use crate::planet_frame::{EARTH_RADIUS_M, PlanetPos, near_plane_m};
use crate::precision::{FOV_DEG, IMAGE_H_PX, focal_px};
use crate::relief::Relief;
use std::collections::HashMap;

/// Builds applied per frame, as `PATCH_APPLY_BUDGET` in the window.
pub const BUILD_BUDGET: usize = 100;

/// Most visible patches in one frame (Q7 A).
pub const MAX_PATCHES: usize = 2_000;

/// Most terrain triangles in one frame, skirts included (Q7 A).
pub const MAX_TRIANGLES: usize = 3_000_000;

/// Triangles per patch: 32 x 32 quads plus four 32-quad skirts, two each.
pub const TRIANGLES_PER_PATCH: usize = (32 * 32 + 4 * 32) * 2;

/// Image width for the view's aspect (1600 x 900).
const IMAGE_W_PX: f64 = 1_600.0;

/// One `CULL` run's numbers.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CullMetrics {
    /// Most visible patches in one frame.
    pub max_patches: usize,
    /// Most terrain triangles in one frame.
    pub max_triangles: usize,
    /// Most builds applied in one frame.
    pub max_builds: usize,
    /// Most leaves culled in one frame.
    pub max_culled: usize,
    /// Drawn patches the view cannot see (must be 0).
    pub drawn_hidden: usize,
    /// Hidden patches with a grid point in view above the horizon (must be 0).
    pub visible_dropped: usize,
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let len = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if len > 0.0 && len.is_finite() {
        [a[0] / len, a[1] / len, a[2] / len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// A view at `camera` looking 20° below the local horizon, toward east.
fn tilted_view(camera: PlanetPos) -> Option<View> {
    let up = unit(camera.0);
    let helper = if up[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let east = unit(cross(helper, up));
    let tilt = 20f64.to_radians();
    let forward = [
        east[0] * tilt.cos() - up[0] * tilt.sin(),
        east[1] * tilt.cos() - up[1] * tilt.sin(),
        east[2] * tilt.cos() - up[2] * tilt.sin(),
    ];
    View::new(
        camera,
        forward,
        up,
        FOV_DEG.to_radians(),
        IMAGE_W_PX / IMAGE_H_PX,
    )
}

/// Whether segment `camera -> point` passes through the occluder sphere.
fn occluded(camera: [f64; 3], point: [f64; 3], radius: f64) -> bool {
    let d = [
        point[0] - camera[0],
        point[1] - camera[1],
        point[2] - camera[2],
    ];
    let a = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
    let b = 2.0 * (camera[0] * d[0] + camera[1] * d[1] + camera[2] * d[2]);
    let c = camera[0] * camera[0] + camera[1] * camera[1] + camera[2] * camera[2] - radius * radius;
    let disc = b * b - 4.0 * a * c;
    if disc <= 0.0 || a <= 0.0 {
        return false;
    }
    let t = (-b - disc.sqrt()) / (2.0 * a);
    t > 0.0 && t < 1.0
}

/// Whether any of `key`'s 5 x 5 sampled grid points (on the relief) is in
/// view and above the occluder: a hidden patch with one is wrongly dropped.
fn shows_a_point(key: PatchKey, view: &View, relief: &Relief, occluder_m: f64) -> bool {
    let Some(per_edge) = key.per_edge() else {
        return false;
    };
    let Some(n) = per_edge.checked_mul(PATCH_QUADS) else {
        return false;
    };
    for sj in 0..=4u32 {
        for si in 0..=4u32 {
            let (Some(gi), Some(gj)) = (
                key.x
                    .checked_mul(PATCH_QUADS)
                    .and_then(|v| v.checked_add(si * 8)),
                key.y
                    .checked_mul(PATCH_QUADS)
                    .and_then(|v| v.checked_add(sj * 8)),
            ) else {
                continue;
            };
            let Some(dir) = grid_point(key.face, gi, gj, n) else {
                continue;
            };
            let r = EARTH_RADIUS_M + relief.height_m(dir);
            let p = [dir[0] * r, dir[1] * r, dir[2] * r];
            if view.sees_point(p) && !occluded(view.camera().0, p, occluder_m) {
                return true;
            }
        }
    }
    false
}

/// Runs the culled descent and climb.
#[must_use]
pub fn cull_metrics() -> CullMetrics {
    let relief = crate::headless_morph::earth_relief();
    let (mountain_dir, _) = crate::headless_morph::earth_mountain_and_plains_dirs();
    let bounds = relief.height_bounds_m();
    let occluder_m = EARTH_RADIUS_M + bounds.0;
    let mut heights: Vec<f64> = Vec::new();
    let ratio = (-1.0f64 / 60.0).exp();
    let mut height = 2_000_000.0f64;
    while height > 1.0 {
        heights.push(height);
        height *= ratio;
    }
    heights.push(1.0);
    let climb: Vec<f64> = heights.iter().rev().skip(1).copied().collect();
    heights.extend(climb);
    let Some(mut tree) = QuadTree::new(EARTH_RADIUS_M) else {
        return CullMetrics {
            drawn_hidden: usize::MAX,
            ..CullMetrics::default()
        };
    };
    let mut scratch = UpdateScratch::default();
    let mut meshes: HashMap<PatchKey, PatchMesh> = HashMap::new();
    let mut queue: Vec<PatchKey> = Vec::new();
    let mut visible: Vec<PatchKey> = Vec::new();
    let mut metrics = CullMetrics::default();
    for (frame, height) in heights.iter().copied().enumerate() {
        let camera = crate::headless_morph::camera_for(&relief, height, mountain_dir);
        let Some(view) = tilted_view(camera) else {
            metrics.drawn_hidden = usize::MAX;
            break;
        };
        {
            let lookup = |key: PatchKey| {
                meshes
                    .get(&key)
                    .map(|m| (m.relief_error_m, m.height_range_m()))
            };
            let heights_of = |key: PatchKey| meshes.get(&key).map(|m| (m.min_h_m, m.max_h_m));
            let cull = Cull {
                view,
                heights: &heights_of,
                fallback: bounds,
                occluder_m,
            };
            tree.update_culled_reused(camera, 1.5, &lookup, Some(&cull), &mut scratch);
        }
        tree.append_visible_leaves_to(&mut visible);
        // Safety counts on every 10th frame, with the heights the selection
        // used (before this frame's builds): drawn means seen, hidden means
        // no sampled ground point shows.
        if frame % 10 == 0 {
            for key in &visible {
                let h = meshes.get(key).map_or(bounds, |m| (m.min_h_m, m.max_h_m));
                if !patch_visible(&view, *key, EARTH_RADIUS_M, h, occluder_m) {
                    metrics.drawn_hidden += 1;
                }
            }
            let hidden: Vec<PatchKey> = tree
                .leaves()
                .into_iter()
                .filter(|key| visible.binary_search(key).is_err())
                .collect();
            let dropped = crate::headless_morph_measure::par_chunks(&hidden, |chunk| {
                chunk
                    .iter()
                    .filter(|key| shows_a_point(**key, &view, &relief, occluder_m))
                    .count()
            });
            metrics.visible_dropped += dropped.into_iter().sum::<usize>();
        }
        // Builds: the wanted unbuilt leaves, at most the budget per frame.
        queue.retain(|key| visible.binary_search(key).is_ok() && !meshes.contains_key(key));
        for key in &visible {
            if !meshes.contains_key(key) && !queue.contains(key) {
                queue.push(*key);
            }
        }
        let take = queue.len().min(BUILD_BUDGET);
        let batch: Vec<PatchKey> = queue.drain(..take).collect();
        let built = crate::headless_morph_measure::par_chunks(&batch, |chunk| {
            chunk
                .iter()
                .map(|key| (*key, build_patch(*key, EARTH_RADIUS_M, &relief)))
                .collect::<Vec<_>>()
        });
        for (key, mesh) in built.into_iter().flatten() {
            if let Some(mesh) = mesh {
                meshes.insert(key, mesh);
            }
        }
        metrics.max_builds = metrics.max_builds.max(take);
        metrics.max_patches = metrics.max_patches.max(visible.len());
        metrics.max_triangles = metrics
            .max_triangles
            .max(visible.len().saturating_mul(TRIANGLES_PER_PATCH));
        metrics.max_culled = metrics
            .max_culled
            .max(tree.len().saturating_sub(visible.len()));
    }
    metrics
}

/// Checks the culled descent and prints one `CULL` line.
pub fn verify_cull(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let m = cull_metrics();
    let passed = m.max_patches <= MAX_PATCHES
        && m.max_triangles <= MAX_TRIANGLES
        && m.max_builds <= BUILD_BUDGET
        && m.max_culled > 0
        && m.drawn_hidden == 0
        && m.visible_dropped == 0;
    let _ = writeln!(
        out,
        "CULL max_patches={} max_triangles={} max_builds={} max_culled={} drawn_hidden={} visible_dropped={} {}",
        m.max_patches,
        m.max_triangles,
        m.max_builds,
        m.max_culled,
        m.drawn_hidden,
        m.visible_dropped,
        if passed { "PASS" } else { "FAIL" },
    );
    passed
}

/// One `DEPTH` probe: whether surfaces at `distance_m` and
/// `distance_m + gap_m` get different `f32` reverse-Z depths with the near
/// plane for a camera `height_m` above the ground, in L10 cell units.
#[must_use]
pub fn depth_separates(height_m: f64, distance_m: f64, gap_m: f64) -> bool {
    let mpc = crate::planet_frame::metres_per_cell_l10();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: the GPU depth path is f32, intended"
    )]
    let (near, a, b) = (
        (near_plane_m(height_m) / mpc) as f32,
        (distance_m / mpc) as f32,
        ((distance_m + gap_m) / mpc) as f32,
    );
    let (da, db) = (near / a, near / b);
    da.is_finite() && db.is_finite() && da > db && da <= 1.0
}

/// Checks depth separation at 1 cm, 1 m and 1,000 km and prints one `DEPTH`
/// line: the gap tested at each height, and the near plane at 1,000 km.
pub fn verify_depth(out: &mut String) -> bool {
    use std::fmt::Write as _;
    // One pixel on the ground at 1,000 km (focal length of the pixel rule).
    let pixel_m = 1_000_000.0 / focal_px();
    let probes = [(0.01, 0.001), (1.0, 0.001), (1_000_000.0, pixel_m)];
    let mut ok = true;
    let mut parts = Vec::new();
    for (height, gap) in probes {
        let separated = depth_separates(height, height, gap);
        ok &= separated;
        parts.push(format!(
            "{}={}",
            match height {
                h if h < 0.1 => "h_1cm",
                h if h < 10.0 => "h_1m",
                _ => "h_1000km",
            },
            if separated { "ok" } else { "fight" }
        ));
    }
    let _ = writeln!(
        out,
        "DEPTH {} gap_1000km_m={pixel_m:.0} near_1000km_m={:.0} {}",
        parts.join(" "),
        near_plane_m(1_000_000.0),
        if ok { "PASS" } else { "FAIL" },
    );
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_fighting_is_absent_at_every_height() {
        let mut out = String::new();
        assert!(verify_depth(&mut out), "{out}");
        assert!(out.contains("h_1cm=ok h_1m=ok h_1000km=ok"), "{out}");
        // A gap far under a pixel at 1,000 km does fight (the probe is real).
        assert!(!depth_separates(1_000_000.0, 1_000_000.0, 0.001));
    }
}
