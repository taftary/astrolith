//! Headless `LOD` line for the quadtree LOD (#776; moved out of
//! `headless.rs` under `E-FILE-SIZE`, #777 review item 8).
//!
//! Pure and deterministic (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use crate::headless::flag;

/// Checks the quadtree LOD selection and patch builds (#776).
///
/// Flies a scripted camera from 2,000 km down to 1 m over the face centre:
/// patch counts per height, worst shake within 1 km on the drawn patches,
/// the one-level neighbour rule, same-level edge mismatches (must be zero)
/// and cracks between drawn levels after the morph (must be zero, #777
/// review item 1, counted by `headless_morph_measure::crack_stats`). Prints one
/// `LOD` line.
/// Byte-identical on every run (`E-DET-TIERS`).
pub(crate) fn verify_lod(out: &mut String) -> bool {
    use crate::cube_sphere::{grid_point, to_planet};
    use crate::lod::{patch_centre_dir, patch_edge_angle};
    use crate::lod_mesh::build_patch;
    use crate::lod_select::{NeighbourDir, QuadTree, is_ancestor, neighbour_at_same_level};
    use crate::nav::DEMO_SEED;
    use crate::nest::autopilot_path;
    use crate::planet_frame::{EARTH_RADIUS_M, PlanetPos};
    use crate::relief::Relief;
    use std::fmt::Write as _;
    let detail_px = 1.5;
    // Earth relief for the journey home cell (#799): builds and true
    // positions both ride the relief surface, so shake still measures the
    // patch-origin path, not the relief itself.
    let earth_relief = {
        let journey = autopilot_path(DEMO_SEED);
        let home_chain: &[u32] = journey.get(..9).unwrap_or(&[]);
        Relief::for_planet(DEMO_SEED, home_chain)
    };
    let heights = [2_000_000.0, 1_000_000.0, 10_000.0, 100.0, 1.0];
    // Heights above the relief surface, not the sphere (#799 review fix):
    // the pole sits about 6 km deep, so sphere-relative cameras at 1 m would
    // float kilometres above the terrain and the shake check below would
    // measure nothing.
    let pole_h = earth_relief.height_m([0.0, 1.0, 0.0]);
    let pole_h = if pole_h.is_finite() { pole_h } else { 0.0 };
    let camera_at = |height: f64| {
        let r = EARTH_RADIUS_M + pole_h + height;
        PlanetPos([0.0, r, 0.0])
    };
    let mut counts: Vec<usize> = Vec::new();
    let mut neighbour_ok = true;
    let mut same_mismatch = 0u32;
    let mut cross_cracks = 0u32;
    let mut worst_mm = 0.0f64;
    // Incremental descent over the face centre (splits and merges like the
    // window). Selection is relief-aware (#777 Q1 A, the single error
    // function for splits and merges), matching the window's
    // `update_with_relief_reused`; unbuilt patches read as zero so detail
    // cascades over frames as children get built.
    use std::collections::HashMap;
    let mut memo: HashMap<crate::lod::PatchKey, (f64, f64, f64)> = HashMap::new();
    let mut built: HashMap<crate::lod::PatchKey, crate::lod_mesh::PatchMesh> = HashMap::new();
    let mut edge_bits = crate::headless_morph_measure::EdgeBits::default();
    let mut tree = match QuadTree::new(EARTH_RADIUS_M) {
        Some(tree) => tree,
        None => return false,
    };
    let mut scratch = crate::lod_select::UpdateScratch::default();
    for height in heights {
        let camera = camera_at(height);
        // A few passes per height so relief detail builds up (Q1 A).
        for _ in 0..3 {
            {
                let memo_ref = &memo;
                tree.update_with_relief_reused(
                    camera,
                    detail_px,
                    &|key| memo_ref.get(&key).copied(),
                    &mut scratch,
                );
            }
            for leaf in tree.leaves() {
                if let std::collections::hash_map::Entry::Vacant(e) = memo.entry(leaf)
                    && let Some(mesh) = build_patch(leaf, EARTH_RADIUS_M, &earth_relief)
                {
                    e.insert(mesh.relief_info());
                    built.insert(leaf, mesh);
                }
            }
        }
        {
            let memo_ref = &memo;
            tree.update_with_relief_reused(
                camera,
                detail_px,
                &|key| memo_ref.get(&key).copied(),
                &mut scratch,
            );
        }
        counts.push(tree.len());
        // Neighbour rule (restricted quadtree, still topological).
        for leaf in tree.leaves() {
            for edge in [
                NeighbourDir::East,
                NeighbourDir::West,
                NeighbourDir::North,
                NeighbourDir::South,
            ] {
                let Some(nkey) = neighbour_at_same_level(leaf, edge) else {
                    continue;
                };
                // Covering level via public API only.
                let mut cov: Option<u32> = None;
                for cand in tree.leaves() {
                    if cand == nkey {
                        cov = Some(cand.level);
                        break;
                    }
                }
                if cov.is_none() {
                    let mut cursor = nkey;
                    while let Some(parent) = cursor.parent() {
                        if tree.leaves().contains(&parent) {
                            cov = Some(parent.level);
                            break;
                        }
                        cursor = parent;
                    }
                }
                if cov.is_none() {
                    let mut deepest: Option<u32> = None;
                    for cand in tree.leaves() {
                        if is_ancestor(nkey, cand) {
                            let better = deepest.is_none_or(|held| cand.level > held);
                            if better {
                                deepest = Some(cand.level);
                            }
                        }
                    }
                    cov = deepest;
                }
                if let Some(level) = cov
                    && leaf.level.abs_diff(level) > 1
                {
                    neighbour_ok = false;
                }
            }
        }
        // Cracks between drawn patches after the morph (#777 review item
        // 1): every leaf drawn and blended as the shader does, shared edge
        // points compared on both sides, T-junctions against the coarse
        // edge. Must be 0.
        for leaf in tree.leaves() {
            if !built.contains_key(&leaf)
                && let Some(mesh) = build_patch(leaf, EARTH_RADIUS_M, &earth_relief)
            {
                memo.insert(leaf, mesh.relief_info());
                built.insert(leaf, mesh);
            }
        }
        let frame =
            crate::headless_morph_measure::blend_frame(&tree.leaves(), &built, camera, detail_px);
        cross_cracks +=
            crate::headless_morph_measure::crack_stats(&frame, camera, &mut edge_bits).0;
    }
    // Shake on the drawn patches at 1 m, 100 m, 10 km and 1,000 km.
    for height in [1.0, 100.0, 10_000.0, 1_000_000.0] {
        let camera = camera_at(height);
        let mut probe = match QuadTree::new(EARTH_RADIUS_M) {
            Some(tree) => tree,
            None => return false,
        };
        probe.update(camera, detail_px);
        for leaf in probe.leaves() {
            let Some(centre_dir) = patch_centre_dir(leaf) else {
                continue;
            };
            let centre_h = earth_relief.height_m(centre_dir);
            let centre_r = EARTH_RADIUS_M + centre_h;
            let centre = [
                centre_dir[0] * centre_r,
                centre_dir[1] * centre_r,
                centre_dir[2] * centre_r,
            ];
            let dist = ((centre[0] - camera.0[0]).powi(2)
                + (centre[1] - camera.0[1]).powi(2)
                + (centre[2] - camera.0[2]).powi(2))
            .sqrt();
            let edge = patch_edge_angle(leaf).unwrap_or(0.0) * EARTH_RADIUS_M;
            if dist - edge > 2_000.0 {
                continue;
            }
            let Some(mesh) = build_patch(leaf, EARTH_RADIUS_M, &earth_relief) else {
                continue;
            };
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: headless shake narrowed like the GPU, intended"
            )]
            let dc = [
                (mesh.centre.0[0] - camera.0[0]) as f32,
                (mesh.centre.0[1] - camera.0[1]) as f32,
                (mesh.centre.0[2] - camera.0[2]) as f32,
            ];
            for (vi, off) in mesh.offsets.iter().enumerate() {
                let side = 33usize;
                let di = vi % side;
                let dj = vi / side;
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "E-CAST: headless grid indices below 2^24, exact"
                )]
                let (di, dj) = (di as u32, dj as u32);
                let Some(per_edge) = leaf.per_edge() else {
                    continue;
                };
                let Some(n_global) = per_edge.checked_mul(32) else {
                    continue;
                };
                let Some(i0) = leaf.x.checked_mul(32) else {
                    continue;
                };
                let Some(j0) = leaf.y.checked_mul(32) else {
                    continue;
                };
                let Some(gi) = i0.checked_add(di) else {
                    continue;
                };
                let Some(gj) = j0.checked_add(dj) else {
                    continue;
                };
                if gi > n_global || gj > n_global {
                    continue;
                }
                let Some(d) = grid_point(leaf.face, gi, gj, n_global) else {
                    continue;
                };
                let h = earth_relief.height_m(d);
                let r = EARTH_RADIUS_M + h;
                if !r.is_finite() || r <= 0.0 {
                    continue;
                }
                let true_p = PlanetPos([d[0] * r, d[1] * r, d[2] * r]);
                let true_rel = [
                    true_p.0[0] - camera.0[0],
                    true_p.0[1] - camera.0[1],
                    true_p.0[2] - camera.0[2],
                ];
                let dist_m = (true_rel[0] * true_rel[0]
                    + true_rel[1] * true_rel[1]
                    + true_rel[2] * true_rel[2])
                    .sqrt();
                if dist_m > 1_000.0 || !dist_m.is_finite() {
                    continue;
                }
                let drawn = [dc[0] + off[0], dc[1] + off[1], dc[2] + off[2]];
                let err = ((f64::from(drawn[0]) - true_rel[0]).powi(2)
                    + (f64::from(drawn[1]) - true_rel[1]).powi(2)
                    + (f64::from(drawn[2]) - true_rel[2]).powi(2))
                .sqrt();
                let err_mm = err * 1_000.0;
                if err_mm.is_finite() && err_mm > worst_mm {
                    worst_mm = err_mm;
                }
                if !err.is_finite() || err >= 1e-3 {
                    neighbour_ok = false;
                }
            }
        }
    }
    // Same-level mismatches at 100 m (must be zero); cross-level counted.
    {
        let camera = camera_at(100.0);
        let mut tree = match QuadTree::new(EARTH_RADIUS_M) {
            Some(tree) => tree,
            None => return false,
        };
        tree.update(camera, detail_px);
        for leaf in tree.leaves() {
            for edge in [
                NeighbourDir::East,
                NeighbourDir::West,
                NeighbourDir::North,
                NeighbourDir::South,
            ] {
                let Some(nkey) = neighbour_at_same_level(leaf, edge) else {
                    continue;
                };
                let mut cov: Option<u32> = None;
                let mut cover_key = None;
                for cand in tree.leaves() {
                    if cand == nkey {
                        cov = Some(cand.level);
                        cover_key = Some(cand);
                        break;
                    }
                }
                if cov.is_none() {
                    let mut cursor = nkey;
                    while let Some(parent) = cursor.parent() {
                        if tree.leaves().contains(&parent) {
                            cov = Some(parent.level);
                            break;
                        }
                        cursor = parent;
                    }
                }
                if cov.is_none() {
                    let mut deepest: Option<u32> = None;
                    for cand in tree.leaves() {
                        if is_ancestor(nkey, cand) {
                            let better = deepest.is_none_or(|held| cand.level > held);
                            if better {
                                deepest = Some(cand.level);
                            }
                        }
                    }
                    cov = deepest;
                }
                if let Some(level) = cov {
                    if level != leaf.level {
                        // Cross-level edges are pinned by the edge mask
                        // (#777 Q3 A, proven in `edges_pin_to_coarser_neighbours`
                        // and `no_cracks_on_the_scripted_path`); they are not
                        // geometric cracks, so they no longer count here.
                        continue;
                    }
                    let Some(ckey) = cover_key else {
                        continue;
                    };
                    // Compare sorted planet-bit sets (any edge order matches).
                    let Some(per) = leaf.per_edge() else {
                        continue;
                    };
                    let Some(n_global) = per.checked_mul(32) else {
                        continue;
                    };
                    let Some(i0) = leaf.x.checked_mul(32) else {
                        continue;
                    };
                    let Some(j0) = leaf.y.checked_mul(32) else {
                        continue;
                    };
                    let Some(ci0) = ckey.x.checked_mul(32) else {
                        continue;
                    };
                    let Some(cj0) = ckey.y.checked_mul(32) else {
                        continue;
                    };
                    let leaf_bits = |dir: NeighbourDir| {
                        let mut v: Vec<[u64; 3]> = Vec::new();
                        for k in 0..=32u32 {
                            let (gi, gj) = match dir {
                                NeighbourDir::East => {
                                    let gi = i0.checked_add(32)?;
                                    let gj = j0.checked_add(k)?;
                                    (gi, gj)
                                }
                                NeighbourDir::West => {
                                    let gj = j0.checked_add(k)?;
                                    (i0, gj)
                                }
                                NeighbourDir::North => {
                                    let gi = i0.checked_add(k)?;
                                    let gj = j0.checked_add(32)?;
                                    (gi, gj)
                                }
                                NeighbourDir::South => {
                                    let gi = i0.checked_add(k)?;
                                    (gi, j0)
                                }
                            };
                            if gi > n_global || gj > n_global {
                                return None;
                            }
                            let d = grid_point(leaf.face, gi, gj, n_global)?;
                            let p = to_planet(d, EARTH_RADIUS_M)?;
                            v.push([p.0[0].to_bits(), p.0[1].to_bits(), p.0[2].to_bits()]);
                        }
                        Some(v)
                    };
                    let neigh_bits = |dir: NeighbourDir| {
                        let mut v: Vec<[u64; 3]> = Vec::new();
                        for k in 0..=32u32 {
                            let (gi, gj) = match dir {
                                NeighbourDir::East => {
                                    let gi = ci0.checked_add(32)?;
                                    let gj = cj0.checked_add(k)?;
                                    (gi, gj)
                                }
                                NeighbourDir::West => {
                                    let gj = cj0.checked_add(k)?;
                                    (ci0, gj)
                                }
                                NeighbourDir::North => {
                                    let gi = ci0.checked_add(k)?;
                                    let gj = cj0.checked_add(32)?;
                                    (gi, gj)
                                }
                                NeighbourDir::South => {
                                    let gi = ci0.checked_add(k)?;
                                    (gi, cj0)
                                }
                            };
                            if gi > n_global || gj > n_global {
                                return None;
                            }
                            let d = grid_point(ckey.face, gi, gj, n_global)?;
                            let p = to_planet(d, EARTH_RADIUS_M)?;
                            v.push([p.0[0].to_bits(), p.0[1].to_bits(), p.0[2].to_bits()]);
                        }
                        Some(v)
                    };
                    let Some(mut a) = leaf_bits(edge) else {
                        continue;
                    };
                    let mut matched = false;
                    for nedge in [
                        NeighbourDir::East,
                        NeighbourDir::West,
                        NeighbourDir::North,
                        NeighbourDir::South,
                    ] {
                        if let Some(mut b) = neigh_bits(nedge) {
                            a.sort();
                            b.sort();
                            if a == b {
                                matched = true;
                                break;
                            }
                        }
                    }
                    if !matched {
                        same_mismatch += 1;
                    }
                }
            }
        }
    }
    let passed = neighbour_ok && same_mismatch == 0 && cross_cracks == 0 && worst_mm < 1.0;
    let patches_str = counts
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let _ = writeln!(
        out,
        "LOD patches={patches_str} shake_mm={worst_mm:.3} neighbours={} same_level={same_mismatch} cross_level={cross_cracks} {}",
        flag(neighbour_ok),
        flag(passed),
    );
    passed
}
