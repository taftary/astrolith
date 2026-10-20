//! Drawn ground tests (#779 T1): the ground is the drawn surface.

use crate::drawn_surface::{DrawnPatch, DrawnSurface};
use crate::lod::PatchKey;
use crate::lod_mesh::{PatchMesh, build_patch};
use crate::lod_morph::morph_mask_in;
use crate::lod_patch_morph::blended_vertex;
use crate::lod_select::{QuadTree, UpdateScratch};
use crate::planet_frame::{EARTH_RADIUS_M, PlanetFrame, PlanetPos};
use crate::relief::Relief;
use std::collections::HashMap;
use std::sync::Arc;

fn unit(a: [f64; 3]) -> [f64; 3] {
    let len = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    [a[0] / len, a[1] / len, a[2] / len]
}

fn length(a: [f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

/// The surface the window would draw with the camera `height_m` above the
/// relief at `dir` (unculled, settled over 40 frames).
pub(crate) fn surface_at(
    relief: &Relief,
    frame: PlanetFrame,
    dir: [f64; 3],
    height_m: f64,
) -> (DrawnSurface, PlanetPos) {
    let up = unit(dir);
    let r = EARTH_RADIUS_M + relief.height_m(up) + height_m;
    let camera = PlanetPos([up[0] * r, up[1] * r, up[2] * r]);
    let quad_px = crate::settings::PlanetDetail::Normal.quad_px();
    let mut tree = QuadTree::new(EARTH_RADIUS_M).expect("tree").with_ranges(
        crate::lod_range::LevelRanges::new(relief, EARTH_RADIUS_M, quad_px),
    );
    let mut scratch = UpdateScratch::default();
    let mut meshes: HashMap<PatchKey, Arc<PatchMesh>> = HashMap::new();
    for _ in 0..40 {
        {
            let lookup = |key: PatchKey| meshes.get(&key).map(|m| m.relief_info());
            tree.update_with_relief_reused(camera, quad_px, &lookup, &mut scratch);
        }
        for leaf in tree.leaves() {
            if !meshes.contains_key(&leaf)
                && let Some(mesh) = build_patch(leaf, EARTH_RADIUS_M, relief)
            {
                meshes.insert(leaf, Arc::new(mesh));
            }
        }
    }
    let drawn = tree.leaves();
    let is_drawn = |key: PatchKey| drawn.binary_search(&key).is_ok();
    let patches: Vec<DrawnPatch> = drawn
        .iter()
        .filter_map(|key| {
            let mesh = meshes.get(key)?;
            Some((*key, Arc::clone(mesh), morph_mask_in(*key, &is_drawn)))
        })
        .collect();
    (DrawnSurface::new(frame, camera, quad_px, patches), camera)
}

pub(crate) fn earth() -> (Relief, PlanetFrame) {
    let journey = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
    let chain: &[u32] = journey.get(..9).unwrap_or(&[]);
    (
        Relief::for_planet(crate::nav::DEMO_SEED, chain),
        PlanetFrame::for_planet(crate::nav::DEMO_SEED, chain),
    )
}

#[test]
#[ignore = "slow (~114 s on the dev PC): runs with --include-ignored (CI, validator), #847"]
fn ground_is_the_drawn_surface() {
    let (relief, frame) = earth();
    let (mountain, _) = crate::headless_morph::earth_mountain_and_plains_dirs();
    let (surface, camera) = surface_at(&relief, frame, mountain, 1.7);
    assert!(!surface.is_empty());
    // 1. Exact at the blended vertices of the patch under the camera: the
    //    radial line through a drawn vertex meets the ground at it.
    let key = surface.covering(camera).expect("covered");
    let (_, mesh, mask) = surface.patch(key).expect("drawn");
    let mut checked = 0usize;
    for idx in (0..33 * 33).step_by(7) {
        let Some((p, _)) = blended_vertex(
            mesh,
            idx,
            camera,
            crate::settings::PlanetDetail::Normal.quad_px(),
            *mask,
        ) else {
            continue;
        };
        // Probe 3 m above it along the radial line.
        let radial = unit(p);
        let r = length(p) + 3.0;
        let (g, up) = surface
            .ground_at_planet(PlanetPos([radial[0] * r, radial[1] * r, radial[2] * r]))
            .expect("ground");
        let gap = length([g.0[0] - p[0], g.0[1] - p[1], g.0[2] - p[2]]);
        assert!(
            gap < 1e-3,
            "vertex {idx}: ground {gap} m off the drawn vertex"
        );
        let along = up[0] * radial[0] + up[1] * radial[1] + up[2] * radial[2];
        assert!((along - 1.0).abs() < 1e-12, "up not radial");
        checked += 1;
    }
    assert!(checked > 100);
    // 2. Continuous across patch edges: walk 12 km over the ground and, at
    //    every change of covering patch, compare both sides 1 µm apart.
    let up = unit(camera.0);
    let east = unit([-up[1], up[0], 0.0]);
    let mut crossings = 0usize;
    let step_m = 4.0;
    let mut last: Option<(PatchKey, [f64; 3])> = None;
    for k in 0..3_000u32 {
        let s = f64::from(k) * step_m - 6_000.0;
        let d = unit([
            up[0] + east[0] * s / EARTH_RADIUS_M,
            up[1] + east[1] * s / EARTH_RADIUS_M,
            up[2] + east[2] * s / EARTH_RADIUS_M,
        ]);
        let probe = PlanetPos([
            d[0] * (EARTH_RADIUS_M + 9_000.0),
            d[1] * (EARTH_RADIUS_M + 9_000.0),
            d[2] * (EARTH_RADIUS_M + 9_000.0),
        ]);
        let Some(here) = surface.covering(probe) else {
            continue;
        };
        if let Some((before, before_dir)) = last
            && before != here
        {
            // Bisect to the edge, then compare the two sides.
            let (mut lo, mut hi) = (before_dir, d);
            for _ in 0..40 {
                let mid = unit([
                    (lo[0] + hi[0]) / 2.0,
                    (lo[1] + hi[1]) / 2.0,
                    (lo[2] + hi[2]) / 2.0,
                ]);
                let at = PlanetPos([
                    mid[0] * EARTH_RADIUS_M,
                    mid[1] * EARTH_RADIUS_M,
                    mid[2] * EARTH_RADIUS_M,
                ]);
                if surface.covering(at) == Some(before) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            let side = |dir: [f64; 3]| {
                surface
                    .ground_at_planet(PlanetPos([
                        dir[0] * EARTH_RADIUS_M,
                        dir[1] * EARTH_RADIUS_M,
                        dir[2] * EARTH_RADIUS_M,
                    ]))
                    .map(|(g, _)| length(g.0))
                    .expect("ground on both sides")
            };
            let jump = (side(lo) - side(hi)).abs();
            assert!(jump < 1e-3, "edge {before:?} -> {here:?}: {jump} m step");
            crossings += 1;
        }
        last = Some((here, d));
    }
    assert!(crossings > 3, "only {crossings} patch edges crossed");
}
