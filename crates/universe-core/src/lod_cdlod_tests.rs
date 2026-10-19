//! CDLOD selection tests (#817 T3): children appear fully blended to their
//! parent's shape, and a still camera holds the tree.

use crate::lod::PatchKey;
use crate::lod_mesh::PatchMesh;
use crate::lod_morph::{morph_mask_in, odd_edge_bits, vertex_factor};
use crate::lod_range::LevelRanges;
use crate::lod_select::{QuadTree, UpdateScratch};
use crate::planet_frame::{EARTH_RADIUS_M, PlanetPos};
use crate::relief::Relief;
use crate::settings::PlanetDetail;
use std::collections::{BTreeSet, HashMap};

fn unit(a: [f64; 3]) -> [f64; 3] {
    let len = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    [a[0] / len, a[1] / len, a[2] / len]
}

fn camera_at(relief: &Relief, dir: [f64; 3], height_m: f64) -> PlanetPos {
    let up = unit(dir);
    let r = EARTH_RADIUS_M + relief.height_m(up) + height_m;
    PlanetPos([up[0] * r, up[1] * r, up[2] * r])
}

struct Run {
    relief: Relief,
    tree: QuadTree,
    scratch: UpdateScratch,
    meshes: HashMap<PatchKey, PatchMesh>,
    memo: HashMap<PatchKey, (f64, f64, f64)>,
    quad_px: f64,
}

impl Run {
    fn new() -> Run {
        let relief = crate::headless_morph::earth_relief();
        let quad_px = PlanetDetail::Normal.quad_px();
        let ranges = LevelRanges::new(&relief, EARTH_RADIUS_M, quad_px);
        let tree = QuadTree::new(EARTH_RADIUS_M)
            .expect("tree")
            .with_ranges(ranges);
        Run {
            relief,
            tree,
            scratch: UpdateScratch::default(),
            meshes: HashMap::new(),
            memo: HashMap::new(),
            quad_px,
        }
    }

    /// One update, then build every leaf (as the window settles).
    fn step(&mut self, camera: PlanetPos) -> (usize, usize) {
        let counts = {
            let meshes = &self.meshes;
            let lookup = |key: PatchKey| meshes.get(&key).map(PatchMesh::relief_info);
            self.tree
                .update_with_relief_reused(camera, self.quad_px, &lookup, &mut self.scratch)
        };
        let leaves = self.tree.leaves();
        crate::headless_morph::build_keys(&leaves, &self.relief, &mut self.memo, &mut self.meshes);
        counts
    }
}

#[test]
fn children_appear_fully_blended() {
    // Descend over the mountain at the default flight speed (height shrinks
    // by e^(-1/60) per frame). Every patch that appears in a frame must be
    // its parent's shape there (factor 1 at every vertex, within one frame
    // of travel), so a switch shows no jump; the morph then moves it.
    let mut run = Run::new();
    let (mountain, _) = crate::headless_morph::earth_mountain_and_plains_dirs();
    let ratio = (-1.0f64 / 60.0).exp();
    let mut height = 200_000.0f64;
    let mut prev: BTreeSet<PatchKey> = BTreeSet::new();
    let mut appeared = 0usize;
    let mut worst = 1.0f64;
    while height > 1.0 {
        let camera = camera_at(&run.relief, mountain, height);
        run.step(camera);
        let leaves = run.tree.leaves();
        let is_leaf = |key: PatchKey| leaves.binary_search(&key).is_ok();
        if !prev.is_empty() {
            for key in leaves.iter().filter(|k| !prev.contains(k)) {
                // Only children of a split (merges are covered by the
                // symmetric rule and the MORPH line).
                if key.parent().is_none_or(|p| !prev.contains(&p)) {
                    continue;
                }
                let Some(mesh) = run.meshes.get(key) else {
                    continue;
                };
                appeared += 1;
                let mask = morph_mask_in(*key, &is_leaf);
                for idx in 0..33 * 33 {
                    let off = mesh.offsets.get(idx).expect("offset");
                    let own = [
                        mesh.centre.0[0] + f64::from(off[0]),
                        mesh.centre.0[1] + f64::from(off[1]),
                        mesh.centre.0[2] + f64::from(off[2]),
                    ];
                    let d = [
                        own[0] - camera.0[0],
                        own[1] - camera.0[1],
                        own[2] - camera.0[2],
                    ];
                    let distance = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                    let err = f64::from(*mesh.morph_err_m.get(idx).expect("err"));
                    let bits = odd_edge_bits(
                        u32::try_from(idx % 33).unwrap_or(0),
                        u32::try_from(idx / 33).unwrap_or(0),
                    );
                    let f = vertex_factor(distance, err, run.quad_px, bits, mask);
                    worst = worst.min(f);
                }
            }
        }
        prev = leaves.into_iter().collect();
        height *= ratio;
    }
    assert!(appeared > 100, "only {appeared} patches appeared");
    // A split triggers on the first frame the box is inside the range, so a
    // child can appear up to one frame of travel past it: the camera closes
    // `1 - e^(-1/60)` of its distance per frame, a factor at most that much
    // over the 30% zone below 1 (the MORPH line measures the pixels).
    let one_frame = (1.0 - (-1.0f64 / 60.0).exp()) / crate::lod_morph::MORPH_ZONE_SHARE;
    assert!(
        worst >= 1.0 - one_frame,
        "a split child appeared at factor {worst} (one frame allows {})",
        1.0 - one_frame
    );
}

#[test]
fn still_camera_holds() {
    let mut run = Run::new();
    let (mountain, _) = crate::headless_morph::earth_mountain_and_plains_dirs();
    let camera = camera_at(&run.relief, mountain, 1.7);
    for _ in 0..40 {
        run.step(camera);
    }
    for frame in 0..60 {
        let counts = run.step(camera);
        assert_eq!(
            counts,
            (0, 0),
            "frame {frame}: a still camera changed the tree"
        );
    }
}
