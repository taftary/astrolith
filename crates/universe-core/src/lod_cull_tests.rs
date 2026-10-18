//! Visibility tests (#778 T1): frustum, horizon, bounds.

use crate::cube_sphere::Face;
use crate::lod::{PatchKey, patch_centre_dir};
use crate::lod_cull::{Bounds, View, above_horizon, patch_bounds, patch_visible};
use crate::lod_mesh::build_patch;
use crate::planet_frame::{EARTH_RADIUS_M, PlanetPos};
use crate::relief::Relief;

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

fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// `dir` turned by `angle` away from itself, about an axis perpendicular to it.
fn turned(dir: [f64; 3], angle: f64) -> [f64; 3] {
    let helper = if dir[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let k = unit(cross(dir, helper));
    let kt = cross(k, dir);
    unit([
        dir[0] * angle.cos() + kt[0] * angle.sin(),
        dir[1] * angle.cos() + kt[1] * angle.sin(),
        dir[2] * angle.cos() + kt[2] * angle.sin(),
    ])
}

/// Deterministic points on the unit sphere (Fibonacci spiral).
fn sphere_points(n: u32) -> Vec<[f64; 3]> {
    let golden = std::f64::consts::PI * (3.0 - 5.0f64.sqrt());
    (0..n)
        .map(|i| {
            let y = 1.0 - 2.0 * (f64::from(i) + 0.5) / f64::from(n);
            let r = (1.0 - y * y).sqrt();
            let a = golden * f64::from(i);
            [r * a.cos(), y, r * a.sin()]
        })
        .collect()
}

/// Whether the segment from `camera` to `point` passes through the sphere
/// of `radius` at the origin (exact occlusion).
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
    if disc <= 0.0 {
        return false;
    }
    let t = (-b - disc.sqrt()) / (2.0 * a);
    t > 0.0 && t < 1.0
}

#[test]
fn relief_bounds_hold() {
    let relief = earth_relief();
    let (low, high) = relief.height_bounds_m();
    for dir in sphere_points(20_000) {
        let h = relief.height_m(dir);
        assert!(h >= low && h <= high, "height {h} outside [{low}, {high}]");
    }
}

#[test]
fn frustum_test_matches_projection() {
    // A camera 10 km up, looking at the horizon, 70° vertical, 16:9.
    let camera = PlanetPos([0.0, 0.0, EARTH_RADIUS_M + 10_000.0]);
    let view = View::new(
        camera,
        [1.0, 0.0, -0.1],
        [0.0, 0.0, 1.0],
        70f64.to_radians(),
        16.0 / 9.0,
    )
    .expect("view");
    // A zero-radius sphere is seen exactly when its point is seen.
    for dir in sphere_points(4_000) {
        let point = [
            camera.0[0] + dir[0] * 5_000.0,
            camera.0[1] + dir[1] * 5_000.0,
            camera.0[2] + dir[2] * 5_000.0,
        ];
        let bounds = Bounds {
            centre: point,
            radius: 0.0,
        };
        assert_eq!(
            view.sees_sphere(bounds),
            view.sees_point(point),
            "point {point:?}"
        );
    }
    // A sphere with any seen point inside it is seen (conservative).
    for (i, dir) in sphere_points(2_000).into_iter().enumerate() {
        let centre = [
            camera.0[0] + dir[0] * 8_000.0,
            camera.0[1] + dir[1] * 8_000.0,
            camera.0[2] + dir[2] * 8_000.0,
        ];
        let radius = 500.0 + 100.0 * f64::from(u32::try_from(i % 20).unwrap_or(0));
        let bounds = Bounds { centre, radius };
        let any_seen = sphere_points(200).into_iter().any(|o| {
            view.sees_point([
                centre[0] + o[0] * radius * 0.999,
                centre[1] + o[1] * radius * 0.999,
                centre[2] + o[2] * radius * 0.999,
            ])
        });
        if any_seen {
            assert!(view.sees_sphere(bounds), "sphere at {centre:?} r {radius}");
        }
    }
}

#[test]
fn patch_bounds_hold_every_drawn_point() {
    let relief = earth_relief();
    let global = relief.height_bounds_m();
    for (level, x, y) in [
        (0u32, 0u32, 0u32),
        (3, 5, 2),
        (8, 100, 37),
        (14, 9000, 4100),
    ] {
        for face in Face::all() {
            let key = PatchKey::new(face, level, x, y).expect("key");
            let mesh = build_patch(key, EARTH_RADIUS_M, &relief).expect("mesh");
            for heights in [(mesh.min_h_m, mesh.max_h_m), global] {
                let b = patch_bounds(key, EARTH_RADIUS_M, heights.0, heights.1).expect("bounds");
                let points = mesh.offsets.iter().chain(mesh.parent_offsets.iter());
                for off in points {
                    let p = [
                        mesh.centre.0[0] + f64::from(off[0]),
                        mesh.centre.0[1] + f64::from(off[1]),
                        mesh.centre.0[2] + f64::from(off[2]),
                    ];
                    let d = [p[0] - b.centre[0], p[1] - b.centre[1], p[2] - b.centre[2]];
                    let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                    // f32 offsets: allow their rounding (well under 1 m).
                    assert!(
                        dist <= b.radius + 1.0,
                        "{key:?} point {dist} outside {}",
                        b.radius
                    );
                }
            }
        }
    }
}

#[test]
fn horizon_keeps_mountain_tops() {
    let key = PatchKey::new(Face::all()[0], 10, 500, 500).expect("key");
    let target = patch_centre_dir(key).expect("dir");
    let altitude = 1_000_000.0;
    let d = EARTH_RADIUS_M + altitude;
    let horizon = (EARTH_RADIUS_M / d).acos();
    // 1.5° beyond the horizon an 8 km peak still shows (it does up to
    // about 2.9°); 5° beyond it, flat ground is hidden.
    let camera = PlanetPos(scale(turned(target, horizon + 1.5f64.to_radians()), d));
    let beyond = PlanetPos(scale(turned(target, horizon + 5f64.to_radians()), d));
    let flat = patch_bounds(key, EARTH_RADIUS_M, 0.0, 0.0).expect("flat");
    let peak = patch_bounds(key, EARTH_RADIUS_M, 0.0, 8_000.0).expect("peak");
    assert!(
        !above_horizon(beyond, flat, EARTH_RADIUS_M),
        "flat patch shown"
    );
    assert!(above_horizon(camera, peak, EARTH_RADIUS_M), "peak dropped");
    // The far side is hidden even with the peak; the near side is shown.
    let far = PlanetPos(scale(turned(target, 3.0), d));
    assert!(!above_horizon(far, peak, EARTH_RADIUS_M), "far side shown");
    let over = PlanetPos(scale(target, d));
    assert!(
        above_horizon(over, flat, EARTH_RADIUS_M),
        "patch below dropped"
    );
}

#[test]
fn hidden_patches_have_no_visible_point() {
    // Every level-3 patch the cull drops, seen from orbit and from low
    // flight: none of its drawn points is in view and above the occluder.
    let relief = earth_relief();
    let (low, _) = relief.height_bounds_m();
    let occluder = EARTH_RADIUS_M + low;
    let views = [
        (2_000_000.0, [0.3, 0.5, 0.8], -1.0),
        (1_000.0, [0.6, -0.2, 0.77], 0.0),
    ];
    let mut dropped = 0usize;
    for (height, dir, tilt) in views {
        let up = unit(dir);
        let camera = PlanetPos(scale(up, EARTH_RADIUS_M + height));
        let side = unit(cross(up, [0.0, 0.0, 1.0]));
        let forward = unit([
            side[0] + up[0] * tilt,
            side[1] + up[1] * tilt,
            side[2] + up[2] * tilt,
        ]);
        let view = View::new(camera, forward, up, 70f64.to_radians(), 16.0 / 9.0).expect("view");
        for face in Face::all() {
            for x in 0..8u32 {
                for y in 0..8u32 {
                    let key = PatchKey::new(face, 3, x, y).expect("key");
                    let mesh = build_patch(key, EARTH_RADIUS_M, &relief).expect("mesh");
                    let heights = (mesh.min_h_m, mesh.max_h_m);
                    if patch_visible(&view, key, EARTH_RADIUS_M, heights, occluder) {
                        continue;
                    }
                    dropped += 1;
                    for off in &mesh.offsets {
                        let p = [
                            mesh.centre.0[0] + f64::from(off[0]),
                            mesh.centre.0[1] + f64::from(off[1]),
                            mesh.centre.0[2] + f64::from(off[2]),
                        ];
                        assert!(
                            !view.sees_point(p) || occluded(camera.0, p, occluder),
                            "{key:?} dropped but shows {p:?}"
                        );
                    }
                }
            }
        }
    }
    assert!(dropped > 100, "only {dropped} patches dropped");
}
