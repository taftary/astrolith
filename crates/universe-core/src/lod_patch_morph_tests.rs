//! Morph shape tests (#777 T2, review item 7).
//!
//! Pure and hermetic (`E-TEST-HERMETIC`).

use crate::cube_sphere::Face;
use crate::lod::{PatchKey, patch_quad_sag_m};
use crate::lod_mesh::{PatchMesh, build_patch};
use crate::lod_morph::{level_step_rad, morph_end_m, morph_mask};
use crate::lod_patch_morph::blended_vertex;
use crate::lod_select::{NeighbourDir, neighbour_at_same_level};
use crate::planet_frame::{EARTH_RADIUS_M, PlanetPos};
use crate::relief::Relief;

const SIDE: usize = 33;

fn earth_relief() -> Relief {
    let journey = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
    let chain: &[u32] = journey.get(..9).unwrap_or(&[]);
    Relief::for_planet(crate::nav::DEMO_SEED, chain)
}

/// A patch at `level` over the highest sampled peak (rugged ground, so
/// the parent shape, the normals and the morph all differ from flat), with
/// an odd column so its east edge is its parent's boundary.
fn mountain_key(relief: &Relief, level: u32) -> PatchKey {
    use crate::cube_sphere::{FacePos, from_sphere, to_sphere};
    let mut best = (f64::NEG_INFINITY, Face::PosY, 0.0, 0.0);
    for face in Face::all() {
        for i in 0..48 {
            for j in 0..48 {
                let u = -1.0 + (f64::from(i) + 0.5) / 24.0;
                let v = -1.0 + (f64::from(j) + 0.5) / 24.0;
                let Some(dir) = FacePos::new(face, u, v).and_then(to_sphere) else {
                    continue;
                };
                let h = relief.height_m(dir);
                if h > best.0 {
                    best = (h, face, u, v);
                }
            }
        }
    }
    let (_, face, u, v) = best;
    let dir = FacePos::new(face, u, v).and_then(to_sphere).expect("peak");
    let pos = from_sphere(dir).expect("face pos");
    let per = f64::from(1u32 << level);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "E-CAST: test cell index inside the face, exact"
    )]
    let (x, y) = (
        (((pos.u + 1.0) / 2.0 * per).floor() as u32).min((1 << level) - 1),
        (((pos.v + 1.0) / 2.0 * per).floor() as u32).min((1 << level) - 1),
    );
    PatchKey::new(pos.face, level, (x | 1).min((1 << level) - 3), y).expect("key")
}

/// A camera `share` of the way to the band end straight above `point`.
fn camera_over(point: [f64; 3], end_m: f64, share: f64) -> PlanetPos {
    let len = dist(point, [0.0; 3]);
    let up = end_m * share;
    PlanetPos([
        point[0] / len * (len + up),
        point[1] / len * (len + up),
        point[2] / len * (len + up),
    ])
}

fn abs(mesh: &PatchMesh, off: [f32; 3]) -> [f64; 3] {
    [
        mesh.centre.0[0] + f64::from(off[0]),
        mesh.centre.0[1] + f64::from(off[1]),
        mesh.centre.0[2] + f64::from(off[2]),
    ]
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Narrowing tolerance for `f32` offsets of this mesh (a few ulps of the
/// largest offset, sub-millimetre for the deep patches used here).
fn f32_tol(mesh: &PatchMesh) -> f64 {
    let mut largest = 0.0f64;
    for off in &mesh.offsets {
        for c in off {
            largest = largest.max(f64::from(*c).abs());
        }
    }
    largest * f64::from(f32::EPSILON) * 4.0
}

/// The parent's drawn surface at the child's grid point `(di, dj)`, through
/// its triangles `a, c, b / b, c, d` (split along `b-c`).
fn parent_surface(child: PatchKey, parent: &PatchMesh, di: usize, dj: usize) -> [f64; 3] {
    let ox = (child.x % 2) as usize * 16;
    let oy = (child.y % 2) as usize * 16;
    let li = ox * 2 + di;
    let lj = oy * 2 + dj;
    let (ci, cj) = ((li / 2).min(31), (lj / 2).min(31));
    // Child steps inside the parent cell: 0, 1 or 2 (the far edge).
    let half = |steps: usize| match steps {
        0 => 0.0,
        1 => 0.5,
        _ => 1.0,
    };
    let (fx, fy) = (half(li - ci * 2), half(lj - cj * 2));
    let at = |i: usize, j: usize| abs(parent, parent.offsets[j * SIDE + i]);
    let (a, b, c, d) = (
        at(ci, cj),
        at(ci + 1, cj),
        at(ci, cj + 1),
        at(ci + 1, cj + 1),
    );
    let w: [([f64; 3], f64); 3] = if fx + fy <= 1.0 {
        [(a, 1.0 - fx - fy), (b, fx), (c, fy)]
    } else {
        [(d, fx + fy - 1.0), (c, 1.0 - fx), (b, 1.0 - fy)]
    };
    let mut out = [0.0f64; 3];
    for (p, wt) in w {
        for k in 0..3 {
            out[k] += p[k] * wt;
        }
    }
    out
}

#[test]
fn fully_blended_patch_is_its_parent() {
    // AC2 (Q2 A, Q6 A, review items 5 and 7): every point of a fully
    // blended child lies on the real parent patch's triangles, odd-odd
    // points included (the diagonal midpoint), and its normals are the
    // parent's own normals on the parent grid.
    let relief = earth_relief();
    for level in [6u32, 12] {
        let key = mountain_key(&relief, level);
        let child = build_patch(key, EARTH_RADIUS_M, &relief).expect("child");
        let parent_key = key.parent().expect("parent");
        let parent = build_patch(parent_key, EARTH_RADIUS_M, &relief).expect("parent");
        let tol = f32_tol(&child).max(f32_tol(&parent));
        let mut worst = 0.0f64;
        for dj in 0..SIDE {
            for di in 0..SIDE {
                let idx = dj * SIDE + di;
                let blended = abs(&child, child.parent_offsets[idx]);
                let want = parent_surface(key, &parent, di, dj);
                worst = worst.max(dist(blended, want));
            }
        }
        assert!(
            worst <= tol,
            "{key:?}: blended child {worst} m off the parent (tol {tol})"
        );
        // Normals on the parent grid are the parent's own (same stencil).
        let ox = (key.x % 2) as usize * 16;
        let oy = (key.y % 2) as usize * 16;
        for dj in (0..SIDE).step_by(2) {
            for di in (0..SIDE).step_by(2) {
                let pidx = (oy + dj / 2) * SIDE + ox + di / 2;
                assert_eq!(
                    child.parent_normals[dj * SIDE + di],
                    parent.normals[pidx],
                    "{key:?} parent normal at ({di},{dj})"
                );
            }
        }
        // And the odd-odd parent point really moved off the 4-corner mean
        // somewhere (the diagonal rule is not the old average).
        let mut differs = false;
        for dj in (1..SIDE).step_by(2) {
            for di in (1..SIDE).step_by(2) {
                let corner = |i: usize, j: usize| abs(&child, child.offsets[j * SIDE + i]);
                let (a, b, c, d) = (
                    corner(di - 1, dj - 1),
                    corner(di + 1, dj - 1),
                    corner(di - 1, dj + 1),
                    corner(di + 1, dj + 1),
                );
                let mean = [
                    (a[0] + b[0] + c[0] + d[0]) / 4.0,
                    (a[1] + b[1] + c[1] + d[1]) / 4.0,
                    (a[2] + b[2] + c[2] + d[2]) / 4.0,
                ];
                if dist(mean, abs(&child, child.parent_offsets[dj * SIDE + di])) > tol * 10.0 {
                    differs = true;
                }
            }
        }
        assert!(
            differs,
            "{key:?}: diagonal rule indistinguishable from the mean"
        );
    }
}

#[test]
fn edges_pin_exactly_to_the_coarse_edge() {
    // AC3 (Q3 A, review item 7): with the coarser bit set, every in-between
    // point of the fine edge lies on the coarse patch's edge, and with the
    // finer bit set the coarse edge holds its own place, whatever the
    // distance. Checked against the real coarse neighbour.
    let relief = earth_relief();
    let fine_key = mountain_key(&relief, 9);
    let fine = build_patch(fine_key, EARTH_RADIUS_M, &relief).expect("fine mesh");
    // East neighbour's parent is the coarse patch across the east edge
    // (fine x is odd, so the east edge is the parent's boundary).
    let across = neighbour_at_same_level(fine_key, NeighbourDir::East).expect("across");
    let coarse_key = across.parent().expect("coarse");
    let coarse = build_patch(coarse_key, EARTH_RADIUS_M, &relief).expect("coarse mesh");
    let tol = f32_tol(&fine).max(f32_tol(&coarse));
    let detail = 1.5;
    // A camera over an east-edge in-between point, inside its band, so a
    // free point there would move.
    let probe = 15 * SIDE + 32;
    let end = morph_end_m(f64::from(fine.morph_err_m[probe]), detail);
    let camera = camera_over(abs(&fine, fine.offsets[probe]), end, 0.85);
    let fine_mask = morph_mask(1, 0); // East coarser.
    let coarse_mask = morph_mask(0, 2); // West finer (the shared edge).
    // Coarse west edge points, by planet position (cross-check by nearest).
    let coarse_edge: Vec<[f64; 3]> = (0..SIDE)
        .map(|k| {
            blended_vertex(&coarse, k * SIDE, camera, detail, coarse_mask)
                .expect("coarse vertex")
                .0
        })
        .collect();
    let mut worst = 0.0f64;
    let mut moved_free = false;
    for k in 0..SIDE {
        let idx = k * SIDE + 32;
        let (pinned, _) = blended_vertex(&fine, idx, camera, detail, fine_mask).expect("fine");
        let (free, _) = blended_vertex(&fine, idx, camera, detail, 0).expect("fine free");
        if k % 2 == 1 && dist(pinned, free) > tol * 10.0 {
            moved_free = true;
        }
        // Distance to the coarse polyline (segments between coarse points).
        let mut best = f64::INFINITY;
        for seg in coarse_edge.windows(2) {
            let (a, b) = (seg[0], seg[1]);
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let ap = [pinned[0] - a[0], pinned[1] - a[1], pinned[2] - a[2]];
            let t = ((ap[0] * ab[0] + ap[1] * ab[1] + ap[2] * ab[2])
                / (ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2]))
                .clamp(0.0, 1.0);
            let q = [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t];
            best = best.min(dist(pinned, q));
        }
        worst = worst.max(best);
    }
    assert!(
        worst <= tol,
        "fine edge {worst} m off the coarse edge (tol {tol})"
    );
    assert!(
        moved_free,
        "the camera sits inside the band (free points move)"
    );
}

#[test]
fn shared_edges_blend_alike() {
    // AC5 (review items 1 and 3): two same-level patches with different
    // parents carry the same morph error on their shared edge, bit for
    // bit, so the per-vertex factor agrees and the edge stays closed at any
    // distance.
    let relief = earth_relief();
    let a_key = mountain_key(&relief, 9);
    let b_key = neighbour_at_same_level(a_key, NeighbourDir::East).expect("b");
    assert_ne!(a_key.parent(), b_key.parent(), "cousins, not siblings");
    let a = build_patch(a_key, EARTH_RADIUS_M, &relief).expect("a mesh");
    let b = build_patch(b_key, EARTH_RADIUS_M, &relief).expect("b mesh");
    let tol = f32_tol(&a).max(f32_tol(&b)) * 2.0;
    for k in (1..SIDE).step_by(2) {
        assert_eq!(
            a.morph_err_m[k * SIDE + 32],
            b.morph_err_m[k * SIDE],
            "shared edge error at k={k}"
        );
    }
    let probe = 15 * SIDE + 32;
    let end = morph_end_m(f64::from(a.morph_err_m[probe]), 1.5);
    let mut moved = false;
    for share in [0.5, 0.75, 0.85, 0.95, 1.2] {
        let camera = camera_over(abs(&a, a.offsets[probe]), end, share);
        let (free, _) = blended_vertex(&a, probe, camera, 1.5, 0).expect("probe");
        if dist(free, abs(&a, a.offsets[probe])) > tol * 10.0 {
            moved = true;
        }
        for k in 0..SIDE {
            let (pa, _) = blended_vertex(&a, k * SIDE + 32, camera, 1.5, 0).expect("a");
            let (pb, _) = blended_vertex(&b, k * SIDE, camera, 1.5, 0).expect("b");
            assert!(
                dist(pa, pb) <= tol,
                "edge k={k} at {share}: {} m apart",
                dist(pa, pb)
            );
        }
    }
    assert!(moved, "the probe point blends on this path");
}

#[test]
fn normals_follow_the_relief() {
    // Q6 A (review item 2): normals are terrain normals, not sphere
    // directions, unit length and outward; the parent normal on odd points
    // is the parent's interpolated normal.
    let relief = earth_relief();
    let key = mountain_key(&relief, 9);
    let mesh = build_patch(key, EARTH_RADIUS_M, &relief).expect("mesh");
    let mut tilted = 0.0f64;
    for (idx, n) in mesh.normals.iter().enumerate() {
        let n = [f64::from(n[0]), f64::from(n[1]), f64::from(n[2])];
        let len = dist(n, [0.0; 3]);
        assert!((len - 1.0).abs() < 1e-5, "unit normal {len}");
        let p = abs(&mesh, mesh.offsets[idx]);
        let r = dist(p, [0.0; 3]);
        let up = [p[0] / r, p[1] / r, p[2] / r];
        let dot = n[0] * up[0] + n[1] * up[1] + n[2] * up[2];
        assert!(dot > 0.0, "outward normal");
        tilted = tilted.max(dot.clamp(-1.0, 1.0).acos().to_degrees());
    }
    assert!(tilted > 1.0, "relief tilts some normal, worst {tilted} deg");
    assert!(
        level_step_rad(1) * 2.0 == level_step_rad(0),
        "parent step doubles"
    );
}

#[test]
fn grid_normals_match_across_seams_and_levels() {
    // #918 T8: normals come from differences on the canonical grid. Two
    // same-level neighbours give their shared edge bit-identical normals,
    // a child's parent normal on an even point is its parent's own normal
    // there, and the grid normal stays close to the relief normal.
    let relief = earth_relief();
    let key = mountain_key(&relief, 9);
    let mesh = build_patch(key, EARTH_RADIUS_M, &relief).expect("mesh");
    let east = neighbour_at_same_level(key, NeighbourDir::East).expect("east");
    let east_mesh = build_patch(east, EARTH_RADIUS_M, &relief).expect("east mesh");
    if east.face == key.face {
        for j in 0..SIDE {
            let ours = mesh.normals[j * SIDE + SIDE - 1];
            let theirs = east_mesh.normals[j * SIDE];
            assert_eq!(ours, theirs, "shared edge normal at row {j}");
        }
    }
    let parent = key.parent().expect("parent");
    let parent_mesh = build_patch(parent, EARTH_RADIUS_M, &relief).expect("parent mesh");
    let (px, py) = ((key.x % 2) as usize * 16, (key.y % 2) as usize * 16);
    for dj in (0..SIDE).step_by(2) {
        for di in (0..SIDE).step_by(2) {
            let child = mesh.parent_normals[dj * SIDE + di];
            let own = parent_mesh.normals[(py + dj / 2) * SIDE + px + di / 2];
            assert_eq!(child, own, "parent normal at {di},{dj}");
        }
    }
    let step = level_step_rad(key.level);
    let mut worst = 0.0f64;
    for (idx, n) in mesh.normals.iter().enumerate() {
        let p = abs(&mesh, mesh.offsets[idx]);
        let r = dist(p, [0.0; 3]);
        let dir = [p[0] / r, p[1] / r, p[2] / r];
        let want = crate::lod_morph::relief_normal(&relief, EARTH_RADIUS_M, dir, step).expect("n");
        let n = [f64::from(n[0]), f64::from(n[1]), f64::from(n[2])];
        let dot = (n[0] * want[0] + n[1] * want[1] + n[2] * want[2]).clamp(-1.0, 1.0);
        worst = worst.max(dot.acos().to_degrees());
    }
    assert!(
        worst < 5.0,
        "grid normal within 5 deg of the relief normal, worst {worst}"
    );
}

#[test]
fn every_edge_has_a_skirt() {
    // AC4 (Q4 A): every edge has a skirt hanging `max(4 x err, 1 m)` below
    // it along the radius, moving with its edge point.
    let relief = earth_relief();
    for (face, level, x, y) in [
        (Face::PosY, 0u32, 0u32, 0u32),
        (Face::PosY, 4, 5, 7),
        (Face::NegX, 12, 2000, 3001),
    ] {
        let key = PatchKey::new(face, level, x, y).expect("key");
        let mesh = build_patch(key, EARTH_RADIUS_M, &relief).expect("mesh");
        assert_eq!(mesh.skirt_offsets.len(), 132, "skirt verts {key:?}");
        assert_eq!(mesh.skirt_parent_offsets.len(), 132);
        assert_eq!(mesh.skirt_normals.len(), 132);
        assert_eq!(mesh.skirt_parent_normals.len(), 132);
        assert_eq!(mesh.skirt_morph_err_m.len(), 132);
        assert_eq!(mesh.skirt_indices.len(), 4 * 32 * 6);
        let quad_sag = patch_quad_sag_m(key, EARTH_RADIUS_M).unwrap_or(0.0);
        let depth = (4.0 * quad_sag.max(mesh.relief_error_m.max(0.0))).max(1.0);
        let tol = f32_tol(&mesh).max(1e-3) * 4.0;
        for (i, soff) in mesh.skirt_offsets.iter().enumerate() {
            let (edge_no, k) = (i / SIDE, i % SIDE);
            let eidx = match edge_no {
                0 => k,
                1 => 32 * SIDE + k,
                2 => k * SIDE,
                _ => k * SIDE + 32,
            };
            let e = abs(&mesh, mesh.offsets[eidx]);
            let s = abs(&mesh, *soff);
            let drop = dist(e, [0.0; 3]) - dist(s, [0.0; 3]);
            assert!(
                (drop - depth).abs() <= tol,
                "skirt {i} drops {drop}, want {depth}"
            );
            // The skirt moves with its edge point: same delta to the parent.
            let pe = abs(&mesh, mesh.parent_offsets[eidx]);
            let ps = abs(&mesh, mesh.skirt_parent_offsets[i]);
            for c in 0..3 {
                let want = e[c] - pe[c];
                let got = s[c] - ps[c];
                assert!((want - got).abs() <= tol, "skirt {i} follows its edge");
            }
            assert_eq!(mesh.skirt_morph_err_m[i], mesh.morph_err_m[eidx]);
        }
        let total = u32::try_from(mesh.total_vertex_count()).expect("fits");
        assert!(mesh.skirt_indices.iter().all(|idx| *idx < total));
    }
}
