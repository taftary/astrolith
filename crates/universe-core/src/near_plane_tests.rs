//! Near-plane tests (#778 T4, Q1 A): the L10 near plane follows the height
//! above the drawn terrain and never reaches the ground.

use crate::planet_frame::{EARTH_RADIUS_M, NEAR_FLOOR_M, PlanetPos, near_plane_m};
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

#[test]
fn near_plane_follows_height_above_terrain() {
    // The rule: 5% of the height, never under 1 mm, either side of the ground.
    assert!((near_plane_m(0.01) - NEAR_FLOOR_M).abs() < 1e-15);
    assert!((near_plane_m(1.0) - 0.05).abs() < 1e-12);
    assert!((near_plane_m(1_000_000.0) - 50_000.0).abs() < 1e-6);
    assert!((near_plane_m(-2.0) - 0.1).abs() < 1e-12);
    assert!((near_plane_m(f64::NAN) - NEAR_FLOOR_M).abs() < 1e-15);
    // On a descent over relief, at every height the near plane stays closer
    // than any ground point sampled around the camera (cliffs included).
    let relief = earth_relief();
    for dir in [[0.6, -0.2, 0.77], [0.3, 0.5, 0.8], [-0.7, 0.1, 0.2]] {
        let up = unit(dir);
        let east = unit([-up[1], up[0], 0.0]);
        let north = [
            up[1] * east[2] - up[2] * east[1],
            up[2] * east[0] - up[0] * east[2],
            up[0] * east[1] - up[1] * east[0],
        ];
        let ground_r = EARTH_RADIUS_M + relief.height_m(up);
        let mut height = 2_000_000.0f64;
        while height > 0.01 {
            let r = ground_r + height;
            let camera = PlanetPos([up[0] * r, up[1] * r, up[2] * r]);
            let above = relief.height_above_m(camera);
            assert!(
                (above - height).abs() < 1e-6 * height.max(1.0),
                "height {height}: above {above}"
            );
            let near = near_plane_m(above);
            // Ground points on rings out to 20 heights away.
            let mut nearest = f64::INFINITY;
            for ring in 1..=20u32 {
                let reach = height * f64::from(ring) / EARTH_RADIUS_M;
                for step in 0..16u32 {
                    let a = f64::from(step) * std::f64::consts::TAU / 16.0;
                    let d = unit([
                        up[0] + (east[0] * a.cos() + north[0] * a.sin()) * reach,
                        up[1] + (east[1] * a.cos() + north[1] * a.sin()) * reach,
                        up[2] + (east[2] * a.cos() + north[2] * a.sin()) * reach,
                    ]);
                    let g = EARTH_RADIUS_M + relief.height_m(d);
                    let p = [d[0] * g, d[1] * g, d[2] * g];
                    let off = [p[0] - camera.0[0], p[1] - camera.0[1], p[2] - camera.0[2]];
                    nearest =
                        nearest.min((off[0] * off[0] + off[1] * off[1] + off[2] * off[2]).sqrt());
                }
            }
            assert!(
                near < nearest,
                "height {height}: near {near} m reaches ground {nearest} m"
            );
            height /= 3.0;
        }
    }
}
