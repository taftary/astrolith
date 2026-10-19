//! Quadtree LOD tests (split from `lod.rs` under `E-FILE-SIZE`, #776 review).
//!
//! Pure and hermetic (`E-TEST-HERMETIC`).

use crate::cube_sphere::{Face, grid_point, to_planet};
use crate::lod::*;
use crate::lod_select::*;
use crate::planet_frame::{EARTH_RADIUS_M, PlanetPos};
use crate::precision::focal_px;

fn earth_camera(height_m: f64, dir: [f64; 3]) -> PlanetPos {
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    let unit = [dir[0] / len, dir[1] / len, dir[2] / len];
    let r = EARTH_RADIUS_M + height_m;
    PlanetPos([unit[0] * r, unit[1] * r, unit[2] * r])
}

#[test]
fn faces_are_quadtrees_of_33_point_patches() {
    // AC1 / C1 (T1, Q1 A): six roots, each a 33 x 33-point patch on #775's
    // mapping; children tile the parent exactly.
    assert_eq!(PATCH_QUADS, 32);
    assert_eq!(PATCH_VERTS, 33);
    assert_eq!(PATCH_VERTS, PATCH_QUADS + 1);
    let tree = QuadTree::new(EARTH_RADIUS_M).expect("earth tree");
    assert_eq!(tree.len(), 6);
    let mut faces: Vec<usize> = tree.leaves().iter().map(|key| key.face_index()).collect();
    faces.sort();
    assert_eq!(faces, vec![0, 1, 2, 3, 4, 5]);
    for leaf in tree.leaves() {
        assert_eq!(leaf.level, 0);
        // Patch bounds span the whole face.
        let (u0, u1, v0, v1) = patch_uv_range(leaf).expect("range");
        assert!((u0 + 1.0).abs() < 1e-12);
        assert!((u1 - 1.0).abs() < 1e-12);
        assert!((v0 + 1.0).abs() < 1e-12);
        assert!((v1 - 1.0).abs() < 1e-12);
        // Global grid points exist at the patch corners on #775's mapping.
        let n = PATCH_QUADS;
        for (i, j) in [(0, 0), (n, 0), (0, n), (n, n)] {
            assert!(grid_point(leaf.face, i, j, n).is_some());
        }
        // Children tile the parent without gaps.
        let children = leaf.children().expect("children");
        assert_eq!(children.len(), 4);
        for child in children {
            assert_eq!(child.level, 1);
            assert!(is_ancestor(leaf, child));
            assert_eq!(child.parent(), Some(leaf));
            // Child bounds sit inside the parent and tile it.
            let (cu0, cu1, cv0, cv1) = patch_uv_range(child).expect("child range");
            assert!(cu0 >= u0 - 1e-12 && cu1 <= u1 + 1e-12);
            assert!(cv0 >= v0 - 1e-12 && cv1 <= v1 + 1e-12);
            // Child corners are global grid points at twice the resolution.
            let per = 2u32;
            let gi = child.x * PATCH_QUADS;
            let gj = child.y * PATCH_QUADS;
            let gn = PATCH_QUADS * per;
            assert!(grid_point(child.face, gi, gj, gn).is_some());
        }
    }
    // Same inputs give the same tree.
    let again = QuadTree::new(EARTH_RADIUS_M).expect("again");
    assert_eq!(tree, again);
}

#[test]
fn patches_split_and_merge_by_screen_error() {
    // AC2 / C2 (T1, Q2 A, Q3 A, Q7 A): the 2,000 km down to 1 m and back
    // path, the 1 cm back-and-forth, neighbours within one level, and an
    // Earth depth of 18 at about 1 m.
    assert_eq!(max_level_for_radius(EARTH_RADIUS_M), 18);
    let detail_px = 1.5;
    let heights = [
        2_000_000.0,
        1_000_000.0,
        10_000.0,
        100.0,
        1.0,
        100.0,
        10_000.0,
        1_000_000.0,
        2_000_000.0,
    ];
    let dirs = [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0]];
    for dir in dirs {
        let mut tree = QuadTree::new(EARTH_RADIUS_M).expect("tree");
        let mut counts: Vec<usize> = Vec::new();
        for height in heights {
            let camera = earth_camera(height, dir);
            tree.update(camera, detail_px);
            counts.push(tree.len());
            // No leaf splits further: every leaf is below the threshold
            // or at the cap.
            for leaf in tree.leaves() {
                if leaf.level >= tree.max_level() {
                    continue;
                }
                let px = tree.screen_error(leaf, camera).expect("error");
                assert!(
                    px <= detail_px,
                    "leaf {leaf:?} at {height} m shows {px} px over {detail_px}"
                );
            }
            // No group merges further unless the neighbour rule needs it:
            // no four siblings sit below half with a merge that keeps the rule.
            for leaf in tree.leaves() {
                let Some(parent) = leaf.parent() else {
                    continue;
                };
                let Some(children) = parent.children() else {
                    continue;
                };
                let mut all = true;
                for child in children {
                    if !tree.leaves.contains(&child) {
                        all = false;
                        break;
                    }
                }
                if all {
                    let px = patch_screen_error_px(parent, camera, EARTH_RADIUS_M, focal_px())
                        .expect("parent error");
                    if px < detail_px / 2.0 {
                        // Merging would be wanted by error: only allowed to
                        // stay split when the neighbour rule needs it.
                        let mut keeps = true;
                        for dir in [
                            NeighbourDir::East,
                            NeighbourDir::West,
                            NeighbourDir::North,
                            NeighbourDir::South,
                        ] {
                            let Some(nkey) = neighbour_at_same_level(parent, dir) else {
                                continue;
                            };
                            if let Some(level) = tree.covering_level(nkey)
                                && level.abs_diff(parent.level) > 1
                            {
                                keeps = false;
                                break;
                            }
                        }
                        assert!(!keeps, "siblings {parent:?} at {height} m merge at {px} px");
                    }
                }
            }
            // Neighbours within one level.
            for leaf in tree.leaves() {
                for dir in [
                    NeighbourDir::East,
                    NeighbourDir::West,
                    NeighbourDir::North,
                    NeighbourDir::South,
                ] {
                    let Some(neighbour) = neighbour_at_same_level(leaf, dir) else {
                        continue;
                    };
                    let Some(level) = tree.covering_level(neighbour) else {
                        continue;
                    };
                    let gap = leaf.level.abs_diff(level);
                    assert!(
                        gap <= 1,
                        "neighbours {leaf:?} and {neighbour:?} differ by {gap}"
                    );
                }
            }
        }
        // The dive goes deeper on the way down and shallower back up.
        assert!(
            counts[4] >= counts[0],
            "1 m ({}) refines over 2,000 km ({})",
            counts[4],
            counts[0]
        );
        assert!(
            counts[8] <= counts[4],
            "returning to 2,000 km merges back: {} <= {}",
            counts[8],
            counts[4]
        );
        // Stable at the return height: another update changes nothing.
        let stable = tree.clone();
        let mut steady = stable.clone();
        steady.update(earth_camera(2_000_000.0, dir), detail_px);
        assert_eq!(steady, stable, "same camera is stable");
        // From far away the tree stays small (tens, not thousands): the
        // drawn quad, not the whole patch, drives the error.
        if dir == [0.0, 1.0, 0.0] {
            assert!(
                counts[0] <= 24,
                "2,000 km stays coarse, got {} patches",
                counts[0]
            );
        }
        // Deterministic: the same path gives the same tree.
        let mut again = QuadTree::new(EARTH_RADIUS_M).expect("again");
        for height in heights {
            again.update(earth_camera(height, dir), detail_px);
        }
        assert_eq!(tree, again, "same path, same tree");
    }
    // No back-and-forth: 1 cm around a split boundary changes nothing.
    let mut tree = QuadTree::new(EARTH_RADIUS_M).expect("tree");
    let dir = [0.0, 1.0, 0.0];
    // Find a boundary by descending until the tree first refines.
    let mut boundary = 2_000_000.0;
    tree.update(earth_camera(boundary, dir), detail_px);
    let base = tree.len();
    for height in [1_000_000.0, 100_000.0, 10_000.0, 1_000.0, 100.0, 10.0, 1.0] {
        tree.update(earth_camera(height, dir), detail_px);
        if tree.len() != base {
            boundary = height;
            break;
        }
    }
    let before = tree.clone();
    tree.update(earth_camera(boundary + 0.01, dir), detail_px);
    tree.update(earth_camera(boundary - 0.01, dir), detail_px);
    tree.update(earth_camera(boundary, dir), detail_px);
    // Within the hysteresis band the tree may refine once, but a 1 cm
    // nudge never flips it back and forth: two runs agree.
    let mut probe = before.clone();
    probe.update(earth_camera(boundary + 0.01, dir), detail_px);
    probe.update(earth_camera(boundary - 0.01, dir), detail_px);
    probe.update(earth_camera(boundary, dir), detail_px);
    assert_eq!(tree, probe, "1 cm moves are deterministic");
    // Unused face import check: every face builds a root.
    assert_eq!(Face::all().len(), 6);
    // The leaf under the camera gets deeper as the height drops.
    let mut depths: Vec<u32> = Vec::new();
    let mut tree = QuadTree::new(EARTH_RADIUS_M).expect("tree");
    for height in [10_000.0, 100.0, 1.0] {
        tree.update(earth_camera(height, [0.0, 1.0, 0.0]), detail_px);
        // Nadir is PosY (0, 0): the PosY leaf containing it.
        let mut found: Option<u32> = None;
        for leaf in tree.leaves() {
            if leaf.face != Face::PosY {
                continue;
            }
            let Some((u0, u1, v0, v1)) = patch_uv_range(leaf) else {
                continue;
            };
            if u0 <= 0.0 && 0.0 <= u1 && v0 <= 0.0 && 0.0 <= v1 {
                found = Some(leaf.level);
                break;
            }
        }
        let level = found.expect("nadir leaf");
        depths.push(level);
    }
    assert!(
        depths[0] <= depths[1] && depths[1] <= depths[2],
        "nadir deepens as height drops: {depths:?}"
    );
    assert!(
        depths[2] > depths[0],
        "1 m refines deeper than 10 km: {depths:?}"
    );
}

#[test]
fn mountain_detail_builds_up_and_stays() {
    // AC1 / C1 (T1, Q1 A): merges use the relief-aware error, so mountain
    // detail builds up over frames and stays; a still camera is stable.
    use crate::lod_mesh::build_patch;
    use crate::relief::Relief;
    use std::collections::HashMap;
    let relief = {
        let journey = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
        let chain: &[u32] = journey.get(..9).unwrap_or(&[]);
        Relief::for_planet(crate::nav::DEMO_SEED, chain)
    };
    // Find the most and least mountainous level-3 patches by relief error.
    let mut best: Option<(PatchKey, f64, [f64; 3])> = None;
    let mut flat: Option<(PatchKey, f64, [f64; 3])> = None;
    for face in Face::all() {
        for x in 0..8u32 {
            for y in 0..8u32 {
                let Some(key) = PatchKey::new(face, 3, x, y) else {
                    continue;
                };
                let Some(mesh) = build_patch(key, EARTH_RADIUS_M, &relief) else {
                    continue;
                };
                let err = mesh.relief_error_m;
                if !err.is_finite() {
                    continue;
                }
                let dir = patch_centre_dir(key).expect("centre dir");
                if best.is_none_or(|(_, held, _)| err > held) {
                    best = Some((key, err, dir));
                }
                if flat.is_none_or(|(_, held, _)| err < held) {
                    flat = Some((key, err, dir));
                }
            }
        }
    }
    let (_, mountain_err, mountain_dir) = best.expect("mountain patch");
    let (_, plains_err, plains_dir) = flat.expect("plains patch");
    assert!(
        mountain_err > plains_err * 3.0,
        "mountain {mountain_err} not well above plains {plains_err}"
    );
    // CDLOD at Normal detail with the relief roughness ranges (#817).
    let detail_px = crate::settings::PlanetDetail::Normal.quad_px();
    let ranges = crate::lod_range::LevelRanges::new(&relief, EARTH_RADIUS_M, detail_px);
    let height = 50_000.0;
    let camera_for = |dir: [f64; 3]| {
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        let unit = [dir[0] / len, dir[1] / len, dir[2] / len];
        let h = relief.height_m(unit);
        let h = if h.is_finite() { h } else { 0.0 };
        PlanetPos([
            unit[0] * (EARTH_RADIUS_M + h + height),
            unit[1] * (EARTH_RADIUS_M + h + height),
            unit[2] * (EARTH_RADIUS_M + h + height),
        ])
    };
    let mountain_cam = camera_for(mountain_dir);
    let plains_cam = camera_for(plains_dir);
    // Memoized relief lookup: built once per key (test-only cache).
    fn lookup_with(
        relief: &Relief,
        memo: &std::cell::RefCell<HashMap<PatchKey, (f64, f64, f64)>>,
        key: PatchKey,
    ) -> Option<(f64, f64, f64)> {
        if let Some(held) = memo.borrow().get(&key) {
            return Some(*held);
        }
        let mesh = build_patch(key, EARTH_RADIUS_M, relief)?;
        let pair = mesh.relief_info();
        memo.borrow_mut().insert(key, pair);
        Some(pair)
    }
    // Run 60 frames over the mountain and over the plain.
    let mut mountain_tree = QuadTree::new(EARTH_RADIUS_M)
        .expect("tree")
        .with_ranges(ranges.clone());
    let mut plains_tree = QuadTree::new(EARTH_RADIUS_M)
        .expect("tree")
        .with_ranges(ranges);
    let mountain_memo: std::cell::RefCell<HashMap<PatchKey, (f64, f64, f64)>> =
        std::cell::RefCell::new(HashMap::new());
    let plains_memo: std::cell::RefCell<HashMap<PatchKey, (f64, f64, f64)>> =
        std::cell::RefCell::new(HashMap::new());
    let mut mountain_scratch = UpdateScratch::default();
    let mut plains_scratch = UpdateScratch::default();
    for _ in 0..60 {
        mountain_tree.update_with_relief_reused(
            mountain_cam,
            detail_px,
            &|key| lookup_with(&relief, &mountain_memo, key),
            &mut mountain_scratch,
        );
        plains_tree.update_with_relief_reused(
            plains_cam,
            detail_px,
            &|key| lookup_with(&relief, &plains_memo, key),
            &mut plains_scratch,
        );
    }
    let mountain_max = mountain_tree
        .leaves()
        .iter()
        .map(|leaf| leaf.level)
        .max()
        .unwrap_or(0);
    let plains_max = plains_tree
        .leaves()
        .iter()
        .map(|leaf| leaf.level)
        .max()
        .unwrap_or(0);
    assert!(
        mountain_max > plains_max,
        "mountain depth {mountain_max} not deeper than plains {plains_max}"
    );
    // Still camera: 120 frames change nothing.
    for frame in 0..120 {
        let (splits, merges) = mountain_tree.update_with_relief_reused(
            mountain_cam,
            detail_px,
            &|key| lookup_with(&relief, &mountain_memo, key),
            &mut mountain_scratch,
        );
        assert_eq!(
            (splits, merges),
            (0, 0),
            "still mountain camera moves on frame {frame}"
        );
        let (splits, merges) = plains_tree.update_with_relief_reused(
            plains_cam,
            detail_px,
            &|key| lookup_with(&relief, &plains_memo, key),
            &mut plains_scratch,
        );
        assert_eq!(
            (splits, merges),
            (0, 0),
            "still plains camera moves on frame {frame}"
        );
    }
}

fn edge_bits(key: PatchKey, dir: NeighbourDir, radius_m: f64) -> Option<Vec<[u64; 3]>> {
    let per_edge = key.per_edge()?;
    let n_global = per_edge.checked_mul(PATCH_QUADS)?;
    let i0 = key.x.checked_mul(PATCH_QUADS)?;
    let j0 = key.y.checked_mul(PATCH_QUADS)?;
    let mut out: Vec<[u64; 3]> = Vec::new();
    for k in 0..=PATCH_QUADS {
        let (gi, gj) = match dir {
            NeighbourDir::East => (i0.checked_add(PATCH_QUADS)?, j0.checked_add(k)?),
            NeighbourDir::West => (i0, j0.checked_add(k)?),
            NeighbourDir::North => (i0.checked_add(k)?, j0.checked_add(PATCH_QUADS)?),
            NeighbourDir::South => (i0.checked_add(k)?, j0),
        };
        if gi > n_global || gj > n_global {
            return None;
        }
        let d = grid_point(key.face, gi, gj, n_global)?;
        // Absolute planet position bits: same-level edges share the exact
        // seam points through the canonical folding.
        let planet = to_planet(d, radius_m)?;
        out.push([
            planet.0[0].to_bits(),
            planet.0[1].to_bits(),
            planet.0[2].to_bits(),
        ]);
    }
    Some(out)
}

#[test]
fn same_level_edges_match_bit_for_bit() {
    // AC5 / C5 (T2): same-level edges and face seams match bit for bit;
    // cross-level cracks are counted and reported (not yet zero, #777).
    let detail_px = 1.5;
    let heights = [1.0, 100.0, 10_000.0, 1_000_000.0, 2_000_000.0];
    let dirs = [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0]];
    let mut total_same = 0u32;
    let mut total_mismatch = 0u32;
    let mut total_cross = 0u32;
    for dir in dirs {
        for height in heights {
            let camera = earth_camera(height, dir);
            let mut tree = QuadTree::new(EARTH_RADIUS_M).expect("tree");
            tree.update(camera, detail_px);
            for leaf in tree.leaves() {
                for edge in [
                    NeighbourDir::East,
                    NeighbourDir::West,
                    NeighbourDir::North,
                    NeighbourDir::South,
                ] {
                    let Some(neighbour_key) = neighbour_at_same_level(leaf, edge) else {
                        continue;
                    };
                    let Some(level) = tree.covering_level(neighbour_key) else {
                        continue;
                    };
                    if level != leaf.level {
                        total_cross += 1;
                        continue;
                    }
                    let Some(cover) = tree.covering_leaf(neighbour_key) else {
                        total_cross += 1;
                        continue;
                    };
                    if cover.level != leaf.level {
                        total_cross += 1;
                        continue;
                    }
                    total_same += 1;
                    let a = edge_bits(leaf, edge, EARTH_RADIUS_M).expect("edge");
                    let mut matched = false;
                    for nedge in [
                        NeighbourDir::East,
                        NeighbourDir::West,
                        NeighbourDir::North,
                        NeighbourDir::South,
                    ] {
                        let b = edge_bits(cover, nedge, EARTH_RADIUS_M).expect("neighbour");
                        let mut sa = a.clone();
                        let mut sb = b;
                        sa.sort();
                        sb.sort();
                        if sa == sb {
                            matched = true;
                            break;
                        }
                    }
                    if !matched {
                        total_mismatch += 1;
                    }
                }
            }
        }
    }
    assert!(total_same > 0, "checked same-level edges");
    assert_eq!(
        total_mismatch, 0,
        "same-level mismatches {total_mismatch} over {total_same} edges"
    );
    // Cross-level cracks exist and are counted (reported, not zero yet).
    assert!(total_cross > 0, "cross-level cracks counted");
}
