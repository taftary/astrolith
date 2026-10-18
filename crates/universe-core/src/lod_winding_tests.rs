//! Winding tests (#778 T3, Q5 A): every patch triangle faces out, so
//! back-face culling keeps the ground and drops only its far side.

use crate::cube_sphere::Face;
use crate::lod::PatchKey;
use crate::lod_mesh::{PatchMesh, build_patch};
use crate::planet_frame::EARTH_RADIUS_M;
use crate::relief::Relief;

fn earth_relief() -> Relief {
    let journey = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
    let chain: &[u32] = journey.get(..9).unwrap_or(&[]);
    Relief::for_planet(crate::nav::DEMO_SEED, chain)
}

/// Positions of the main and skirt vertices, relative to the patch centre
/// (the combined index space of `indices` and `skirt_indices`).
fn positions(mesh: &PatchMesh) -> Vec<[f64; 3]> {
    mesh.offsets
        .iter()
        .chain(mesh.skirt_offsets.iter())
        .map(|o| [f64::from(o[0]), f64::from(o[1]), f64::from(o[2])])
        .collect()
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Counter-clockwise normal of triangle `tri` (Bevy's front face).
fn normal(pos: &[[f64; 3]], tri: &[u32]) -> Option<([f64; 3], [f64; 3])> {
    let a = *pos.get(usize::try_from(*tri.first()?).ok()?)?;
    let b = *pos.get(usize::try_from(*tri.get(1)?).ok()?)?;
    let c = *pos.get(usize::try_from(*tri.get(2)?).ok()?)?;
    let n = cross(sub(b, a), sub(c, a));
    let centroid = [
        (a[0] + b[0] + c[0]) / 3.0,
        (a[1] + b[1] + c[1]) / 3.0,
        (a[2] + b[2] + c[2]) / 3.0,
    ];
    Some((n, centroid))
}

fn keys() -> Vec<PatchKey> {
    let mut out = Vec::new();
    for face in Face::all() {
        for (level, x, y) in [(0u32, 0u32, 0u32), (4, 3, 11), (12, 1000, 3000)] {
            out.push(PatchKey::new(face, level, x, y).expect("key"));
        }
    }
    out
}

#[test]
fn triangles_face_out() {
    let relief = earth_relief();
    let mut bad = Vec::new();
    for key in keys() {
        let mesh = build_patch(key, EARTH_RADIUS_M, &relief).expect("mesh");
        let pos = positions(&mesh);
        let up = mesh.centre.0;
        let mut inward = 0usize;
        for tri in mesh.indices.chunks(3) {
            let (n, _) = normal(&pos, tri).expect("triangle");
            if dot(n, up) <= 0.0 {
                inward += 1;
            }
        }
        if inward > 0 {
            bad.push(format!("{key:?}: {inward}"));
        }
    }
    assert!(bad.is_empty(), "triangles facing in: {bad:?}");
}

#[test]
fn skirts_face_out() {
    let relief = earth_relief();
    let mut bad = Vec::new();
    for key in keys() {
        let mesh = build_patch(key, EARTH_RADIUS_M, &relief).expect("mesh");
        let pos = positions(&mesh);
        let len = dot(mesh.centre.0, mesh.centre.0).sqrt();
        let up = [
            mesh.centre.0[0] / len,
            mesh.centre.0[1] / len,
            mesh.centre.0[2] / len,
        ];
        let mut inward = 0usize;
        for tri in mesh.skirt_indices.chunks(3) {
            let (n, centroid) = normal(&pos, tri).expect("triangle");
            // Away from the patch: the centroid's offset from the patch
            // centre, without its vertical part.
            let v = dot(centroid, up);
            let away = [
                centroid[0] - up[0] * v,
                centroid[1] - up[1] * v,
                centroid[2] - up[2] * v,
            ];
            if dot(n, away) <= 0.0 {
                inward += 1;
            }
        }
        if inward > 0 {
            bad.push(format!("{key:?}: {inward}"));
        }
    }
    assert!(bad.is_empty(), "skirt triangles facing in: {bad:?}");
}
