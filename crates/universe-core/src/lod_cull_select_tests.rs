//! Culled selection tests (#778 T2): hidden patches are not split, a fast
//! turn leaves no hole.

use crate::lod::PatchKey;
use crate::lod_cull::{Cull, View, patch_visible};
use crate::lod_mesh::{PatchMesh, build_patch};
use crate::lod_select::{QuadTree, UpdateScratch};
use crate::planet_frame::{EARTH_RADIUS_M, PlanetPos};
use crate::relief::Relief;
use std::collections::HashMap;

fn earth_relief() -> Relief {
    let journey = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
    let chain: &[u32] = journey.get(..9).unwrap_or(&[]);
    Relief::for_planet(crate::nav::DEMO_SEED, chain)
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let len = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    [a[0] / len, a[1] / len, a[2] / len]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Built patches (memoized) for the relief and height lookups.
struct Built {
    relief: Relief,
    meshes: HashMap<PatchKey, PatchMesh>,
}

impl Built {
    fn new() -> Built {
        Built {
            relief: earth_relief(),
            meshes: HashMap::new(),
        }
    }

    fn build(&mut self, keys: &[PatchKey]) {
        for key in keys {
            if !self.meshes.contains_key(key)
                && let Some(mesh) = build_patch(*key, EARTH_RADIUS_M, &self.relief)
            {
                self.meshes.insert(*key, mesh);
            }
        }
    }

    fn occluder(&self) -> f64 {
        EARTH_RADIUS_M + self.relief.height_bounds_m().0
    }

    /// One culled update, then build what it now sees.
    fn step(
        &mut self,
        tree: &mut QuadTree,
        view: View,
        scratch: &mut UpdateScratch,
    ) -> (usize, usize) {
        let meshes = &self.meshes;
        let lookup = |key: PatchKey| meshes.get(&key).map(|m| m.relief_info());
        let heights = |key: PatchKey| meshes.get(&key).map(|m| (m.min_h_m, m.max_h_m));
        let cull = Cull {
            view,
            heights: &heights,
            fallback: self.relief.height_bounds_m(),
            occluder_m: self.occluder(),
        };
        let counts = tree.update_culled_reused(
            view.camera(),
            crate::settings::PlanetDetail::Normal.quad_px(),
            &lookup,
            Some(&cull),
            scratch,
        );
        let visible = tree.visible_leaves();
        self.build(&visible);
        counts
    }
}

/// Camera 1 km above the ground at `dir`, looking along the ground at `yaw`
/// (0 east, pi west). Returns the view and its forward and up.
fn low_view(relief: &Relief, dir: [f64; 3], yaw: f64) -> (View, [f64; 3], [f64; 3]) {
    let up = unit(dir);
    let h = relief.height_m(up);
    let r = EARTH_RADIUS_M + h + 1_000.0;
    let camera = PlanetPos([up[0] * r, up[1] * r, up[2] * r]);
    let east = unit(cross([0.0, 0.0, 1.0], up));
    let north = cross(up, east);
    let forward = [
        east[0] * yaw.cos() + north[0] * yaw.sin(),
        east[1] * yaw.cos() + north[1] * yaw.sin(),
        east[2] * yaw.cos() + north[2] * yaw.sin(),
    ];
    let view = View::new(camera, forward, up, 70f64.to_radians(), 16.0 / 9.0).expect("view");
    (view, forward, up)
}

/// Key at `level` holding unit direction `dir`.
fn key_at(dir: [f64; 3], level: u32) -> Option<PatchKey> {
    let pos = crate::cube_sphere::from_sphere(dir)?;
    let per = f64::from(1u32 << level);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "E-CAST: clamped grid index in 0..per, exact"
    )]
    let cell = |t: f64| ((t + 1.0) / 2.0 * per).floor().clamp(0.0, per - 1.0) as u32;
    PatchKey::new(pos.face, level, cell(pos.u), cell(pos.v))
}

/// Ground points on screen: a 17 x 9 grid of rays hitting the sphere of
/// `ground_m` (the ground radius under the camera).
fn ground_points(view: &View, forward: [f64; 3], up: [f64; 3], ground_m: f64) -> Vec<[f64; 3]> {
    let right = unit(cross(forward, up));
    let up = cross(right, forward);
    let camera = view.camera().0;
    let ty = 35f64.to_radians().tan();
    let tx = ty * 16.0 / 9.0;
    let mut out = Vec::new();
    for i in 0..=16u32 {
        for j in 0..=8u32 {
            let sx = (f64::from(i) / 16.0 * 2.0 - 1.0) * tx * 0.99;
            let sy = (f64::from(j) / 8.0 * 2.0 - 1.0) * ty * 0.99;
            let ray = unit([
                forward[0] + right[0] * sx + up[0] * sy,
                forward[1] + right[1] * sx + up[1] * sy,
                forward[2] + right[2] * sx + up[2] * sy,
            ]);
            let b = 2.0 * (camera[0] * ray[0] + camera[1] * ray[1] + camera[2] * ray[2]);
            let c = camera[0] * camera[0] + camera[1] * camera[1] + camera[2] * camera[2]
                - ground_m * ground_m;
            let disc = b * b - 4.0 * c;
            if disc <= 0.0 {
                continue;
            }
            let t = (-b - disc.sqrt()) / 2.0;
            if t > 0.0 {
                out.push([
                    camera[0] + ray[0] * t,
                    camera[1] + ray[1] * t,
                    camera[2] + ray[2] * t,
                ]);
            }
        }
    }
    out
}

/// Sum of covering levels over the ground points; panics on a hole.
fn check_covered(tree: &QuadTree, points: &[[f64; 3]]) -> u32 {
    let visible = tree.visible_leaves();
    let mut total = 0;
    for point in points {
        let key = key_at(unit(*point), tree.max_level()).expect("key");
        let leaf = tree.covering_leaf(key).expect("covered");
        assert!(
            visible.binary_search(&leaf).is_ok(),
            "ground point {point:?} in hidden leaf {leaf:?}"
        );
        total += leaf.level;
    }
    total
}

#[test]
#[ignore = "slow (~32 s on the dev PC): runs with --include-ignored (CI, validator), #847"]
fn frustum_culled_patches_are_not_split() {
    let mut built = Built::new();
    let (view, _, _) = low_view(&built.relief, [0.6, -0.2, 0.77], 0.0);
    let mut tree = QuadTree::new(EARTH_RADIUS_M).expect("tree");
    let mut scratch = UpdateScratch::default();
    for _ in 0..40 {
        built.step(&mut tree, view, &mut scratch);
    }
    // Still: the culled tree holds.
    let still = built.step(&mut tree, view, &mut scratch);
    assert_eq!(still, (0, 0), "still camera changed the tree");
    // The same camera without culling refines all around.
    let mut full = QuadTree::new(EARTH_RADIUS_M).expect("tree");
    let mut full_scratch = UpdateScratch::default();
    for _ in 0..40 {
        let meshes = &built.meshes;
        let lookup = |key: PatchKey| meshes.get(&key).map(|m| m.relief_info());
        full.update_with_relief_reused(
            view.camera(),
            crate::settings::PlanetDetail::Normal.quad_px(),
            &lookup,
            &mut full_scratch,
        );
        let leaves = full.leaves();
        built.build(&leaves);
    }
    let visible = tree.visible_leaves();
    assert!(
        visible.len() * 2 < full.len(),
        "culled draws {} of {} leaves",
        visible.len(),
        full.len()
    );
    // Every drawn leaf passes the visibility test.
    for leaf in &visible {
        let mesh = built.meshes.get(leaf).expect("visible leaf built");
        assert!(patch_visible(
            &view,
            *leaf,
            EARTH_RADIUS_M,
            (mesh.min_h_m, mesh.max_h_m),
            built.occluder()
        ));
    }
}

#[test]
#[ignore = "slow (~36 s on the dev PC): runs with --include-ignored (CI, validator), #847"]
fn fast_turn_has_no_holes() {
    let mut built = Built::new();
    let dir = [0.6, -0.2, 0.77];
    let (ahead, _, _) = low_view(&built.relief, dir, 0.0);
    let mut tree = QuadTree::new(EARTH_RADIUS_M).expect("tree");
    let mut scratch = UpdateScratch::default();
    for _ in 0..40 {
        built.step(&mut tree, ahead, &mut scratch);
    }
    // Turn around in one frame: covered at once.
    let (behind, forward, up) = low_view(&built.relief, dir, std::f64::consts::PI);
    built.step(&mut tree, behind, &mut scratch);
    let ground_m = EARTH_RADIUS_M + built.relief.height_m(unit(dir));
    let points = ground_points(&behind, forward, up, ground_m);
    assert!(points.len() > 20, "too few ground points");
    let first = check_covered(&tree, &points);
    // And over the next frames, still with no hole.
    for _ in 0..20 {
        built.step(&mut tree, behind, &mut scratch);
        check_covered(&tree, &points);
    }
    // The selection splits the new view in the turn's own frame (splits
    // cascade); the window draws cached parents until the builds land.
    let last = check_covered(&tree, &points);
    assert!(
        last >= first,
        "detail dropped after the turn: {first} -> {last}"
    );
}
