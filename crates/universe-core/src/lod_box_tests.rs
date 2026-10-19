//! Box distance tests (#817 T1): real height, lower bound, full depth.

use crate::cube_sphere::Face;
use crate::lod::PatchKey;
use crate::lod_box::patch_box;
use crate::lod_mesh::{PatchMesh, build_patch};
use crate::lod_select::{QuadTree, UpdateScratch};
use crate::planet_frame::{EARTH_RADIUS_M, PlanetPos};
use std::collections::HashMap;

fn unit(a: [f64; 3]) -> [f64; 3] {
    let len = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    [a[0] / len, a[1] / len, a[2] / len]
}

#[test]
fn box_holds_every_drawn_and_parent_point() {
    let relief = crate::headless_morph::earth_relief();
    for (level, x, y) in [
        (0u32, 0u32, 0u32),
        (3, 5, 2),
        (9, 300, 170),
        (16, 40_000, 21_000),
    ] {
        for face in Face::all() {
            let key = PatchKey::new(face, level, x, y).expect("key");
            let mesh = build_patch(key, EARTH_RADIUS_M, &relief).expect("mesh");
            let b = patch_box(key, EARTH_RADIUS_M, mesh.min_h_m, mesh.max_h_m).expect("box");
            for off in mesh.offsets.iter().chain(mesh.parent_offsets.iter()) {
                let p = PlanetPos([
                    mesh.centre.0[0] + f64::from(off[0]),
                    mesh.centre.0[1] + f64::from(off[1]),
                    mesh.centre.0[2] + f64::from(off[2]),
                ]);
                // f32 offsets: allow their rounding (well under 1 m).
                assert!(
                    b.distance(p) < 1.0,
                    "{key:?}: point {} m outside its box",
                    b.distance(p)
                );
            }
        }
    }
}

#[test]
fn tree_reaches_the_deepest_level_near_the_ground() {
    let relief = crate::headless_morph::earth_relief();
    let (mountain, _) = crate::headless_morph::earth_mountain_and_plains_dirs();
    let up = unit(mountain);
    let r = EARTH_RADIUS_M + relief.height_m(up) + 1.7;
    let camera = PlanetPos([up[0] * r, up[1] * r, up[2] * r]);
    let mut tree = QuadTree::new(EARTH_RADIUS_M).expect("tree");
    let mut scratch = UpdateScratch::default();
    let mut meshes: HashMap<PatchKey, PatchMesh> = HashMap::new();
    for _ in 0..60 {
        {
            let lookup = |key: PatchKey| meshes.get(&key).map(PatchMesh::relief_info);
            tree.update_with_relief_reused(
                camera,
                crate::settings::PlanetDetail::Normal.quad_px(),
                &lookup,
                &mut scratch,
            );
        }
        for leaf in tree.leaves() {
            if !meshes.contains_key(&leaf)
                && let Some(mesh) = build_patch(leaf, EARTH_RADIUS_M, &relief)
            {
                meshes.insert(leaf, mesh);
            }
        }
    }
    let deepest = tree
        .leaves()
        .iter()
        .map(|leaf| leaf.level)
        .max()
        .unwrap_or(0);
    assert_eq!(deepest, tree.max_level(), "tree stops at level {deepest}");
}
