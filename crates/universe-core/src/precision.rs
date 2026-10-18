//! Precision measurement for the planet frame (#774, Q5 A, T3).
//!
//! `shake` places test points around a camera, runs them through the full
//! chain (`PlanetPos` to `LocalPos` to `CameraRel` to `to_gpu`), rebuilds
//! them in `f64`, and reports the worst error: in mm for points within 1 km,
//! in pixels at 1600×900 (70° vertical field of view) farther out.
//!
//! Measured at 1 cm, 1 m, 100 m, 10 km, 1,000 km and 2,000 km above the true
//! radius, over the cube-face centre `(0,1,0)`, an edge `norm(1,1,0)` and a
//! corner `norm(1,1,1)` (where step 2 puts them), on Earth and two other
//! seeded planets. Pure and deterministic (`E-DET-TIERS`).

use crate::planet_frame::{CameraRel, LocalFrame, PlanetFrame, PlanetPos};

/// Camera heights above the true surface in metres (Q5 A).
pub const HEIGHTS_M: [f64; 6] = [0.01, 1.0, 100.0, 10_000.0, 1_000_000.0, 2_000_000.0];

/// Face positions: centre, edge, corner (step 2 cube faces).
pub const DIRS: [[f64; 3]; 3] = [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0]];

/// Near-camera bound in metres: points within this shake under 1 mm.
pub const NEAR_M: f64 = 1_000.0;

/// Shake limit near the camera in metres (1 mm).
pub const NEAR_LIMIT_M: f64 = 1e-3;

/// Shake limit farther out in pixels at 1600×900.
pub const FAR_LIMIT_PX: f64 = 1.0;

/// Vertical field of view for the pixel rule, degrees.
pub const FOV_DEG: f64 = 70.0;

/// Image height for the pixel rule.
pub const IMAGE_H_PX: f64 = 900.0;

/// Focal length in pixels: `(H/2) / tan(FOV/2)`.
#[must_use]
pub fn focal_px() -> f64 {
    let half = (FOV_DEG * std::f64::consts::PI / 180.0) / 2.0;
    (IMAGE_H_PX / 2.0) / half.tan()
}

fn length(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn normalize(v: [f64; 3]) -> [f64; 3] {
    let len = length(v);
    if len.is_finite() && len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// Error of one point through the full chain in metres.
///
/// `camera` and `point` are planet positions; the chain runs planet →
/// local (frame at the camera) → camera-relative → `f32` → back to `f64`,
/// and the error is the distance between the rebuilt and the true
/// camera-relative vector.
#[must_use]
pub fn point_error_m(camera: PlanetPos, point: PlanetPos) -> f64 {
    let frame = LocalFrame::at(camera);
    let local_point = frame.to_local(point);
    let local_camera = frame.to_local(camera);
    let rel = CameraRel::from_local(local_point, local_camera);
    let gpu = rel.to_gpu();
    let rebuilt = [f64::from(gpu[0]), f64::from(gpu[1]), f64::from(gpu[2])];
    length([
        rebuilt[0] - rel.0[0],
        rebuilt[1] - rel.0[1],
        rebuilt[2] - rel.0[2],
    ])
}

/// Pixel error of one point: `error_m / distance * focal`.
#[must_use]
pub fn point_error_px(camera: PlanetPos, point: PlanetPos) -> f64 {
    let d = [
        point.0[0] - camera.0[0],
        point.0[1] - camera.0[1],
        point.0[2] - camera.0[2],
    ];
    let distance = length(d);
    if !(distance.is_finite() && distance > 0.0) {
        return 0.0;
    }
    point_error_m(camera, point) / distance * focal_px()
}

/// Test points around `camera`: near offsets plus the surface below.
///
/// Near offsets span 1 mm to 1 km along the local axes and diagonals; the
/// surface point sits `height` below (the far case for high cameras).
fn points_around(camera: PlanetPos, frame: &PlanetFrame) -> Vec<PlanetPos> {
    let local = LocalFrame::at(camera);
    let mut points = Vec::new();
    // Near offsets in metres (local axes + diagonals).
    let offsets = [0.001, 0.01, 0.1, 1.0, 10.0, 100.0, 1_000.0];
    let dirs_local = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [-1.0, 0.5, 0.25],
        [0.3, -0.7, 0.6],
    ];
    for &scale in &offsets {
        for dir in dirs_local {
            let len = length(dir);
            let unit = [dir[0] / len, dir[1] / len, dir[2] / len];
            let local_pt =
                crate::planet_frame::LocalPos([unit[0] * scale, unit[1] * scale, unit[2] * scale]);
            // Local offset from the camera: add to the camera's local pos.
            let cam_local = local.to_local(camera);
            let shifted = crate::planet_frame::LocalPos([
                cam_local.0[0] + local_pt.0[0],
                cam_local.0[1] + local_pt.0[1],
                cam_local.0[2] + local_pt.0[2],
            ]);
            points.push(local.to_planet(shifted));
        }
    }
    // The surface point directly below the camera (radial).
    let r = length(camera.0);
    if r.is_finite() && r > 0.0 {
        let up = [camera.0[0] / r, camera.0[1] / r, camera.0[2] / r];
        let surf = frame.radius_m();
        points.push(PlanetPos([up[0] * surf, up[1] * surf, up[2] * surf]));
    }
    points
}

/// Worst shake for one planet frame: per-height worst in mm plus overall.
///
/// Returns `(per_height_mm, worst_mm, worst_px_far, pass)`: `per_height_mm`
/// holds the worst mm within 1 km at each height in [`HEIGHTS_M`];
/// `worst_px_far` is the worst pixel error beyond 1 km. Passes when every
/// near point shakes under 1 mm and every far point stays under one pixel.
#[must_use]
pub fn shake(frame: &PlanetFrame) -> ([f64; 6], f64, f64, bool) {
    let mut per_height = [0.0f64; 6];
    let mut worst_px = 0.0f64;
    let mut pass = true;
    for (hi, &h) in HEIGHTS_M.iter().enumerate() {
        let mut worst_mm = 0.0f64;
        for dir in DIRS {
            let unit = normalize(dir);
            let r = frame.radius_m() + h;
            let camera = PlanetPos([unit[0] * r, unit[1] * r, unit[2] * r]);
            for point in points_around(camera, frame) {
                let d = length([
                    point.0[0] - camera.0[0],
                    point.0[1] - camera.0[1],
                    point.0[2] - camera.0[2],
                ]);
                if d <= NEAR_M {
                    let err_mm = point_error_m(camera, point) * 1_000.0;
                    if err_mm > worst_mm {
                        worst_mm = err_mm;
                    }
                    if !err_mm.is_finite() || err_mm >= 1.0 {
                        pass = false;
                    }
                } else {
                    let err_px = point_error_px(camera, point);
                    if err_px > worst_px {
                        worst_px = err_px;
                    }
                    if !err_px.is_finite() || err_px >= FAR_LIMIT_PX {
                        pass = false;
                    }
                }
            }
        }
        #[expect(
            clippy::indexing_slicing,
            reason = "E-NO-PANIC: hi enumerates HEIGHTS_M (len 6) into per_height (len 6)"
        )]
        {
            per_height[hi] = worst_mm;
        }
    }
    let worst_mm = per_height.iter().cloned().fold(0.0f64, f64::max);
    (per_height, worst_mm, worst_px, pass)
}

/// Worst shake over Earth and two other seeded planets (Q5 A).
///
/// Earth is the journey home cell under [`crate::nav::DEMO_SEED`]; the other
/// two are fixed procedural frames (radius 0.3 and 11.0 Earth radii from the
/// sampler's class draws). Returns `(per_height_mm, worst_mm, pass)`.
#[must_use]
pub fn shake_all() -> ([f64; 6], f64, bool) {
    let journey = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
    let home_chain: &[u32] = journey.get(..9).unwrap_or(&[]);
    let earth = PlanetFrame::for_planet(crate::nav::DEMO_SEED, home_chain);
    let small = PlanetFrame::for_radius_earth(0.3);
    let giant = PlanetFrame::for_radius_earth(11.0);
    let mut per_height = [0.0f64; 6];
    let mut worst = 0.0f64;
    let mut pass = true;
    for frame in [earth, small, giant] {
        let (per, _, _, ok) = shake(&frame);
        pass &= ok;
        for (slot, value) in per_height.iter_mut().zip(per) {
            if value > *slot {
                *slot = value;
            }
        }
        if per.iter().cloned().fold(0.0f64, f64::max) > worst {
            worst = per.iter().cloned().fold(0.0f64, f64::max);
        }
    }
    (per_height, worst, pass)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shake_under_a_millimetre_near_the_camera() {
        // AC4 / C4: under 1 mm within 1 km at every height x face position x
        // planet; under one pixel farther out.
        let journey = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
        let home_chain: &[u32] = journey.get(..9).unwrap_or(&[]);
        let earth = PlanetFrame::for_planet(crate::nav::DEMO_SEED, home_chain);
        for frame in [
            earth,
            PlanetFrame::for_radius_earth(0.3),
            PlanetFrame::for_radius_earth(11.0),
        ] {
            let (per_height, worst_mm, worst_px, pass) = shake(&frame);
            assert!(
                pass,
                "shake fails on radius {} m: per-height mm {per_height:?}, worst {worst_mm} mm, far {worst_px} px",
                frame.radius_m()
            );
            for (h, mm) in HEIGHTS_M.iter().zip(per_height) {
                assert!(
                    mm < 1.0,
                    "height {h} m shakes {mm} mm on radius {} m",
                    frame.radius_m()
                );
            }
            assert!(worst_px < 1.0, "far pixels {worst_px}");
        }
        // Deterministic: the same inputs give byte-identical numbers.
        let (a, _, _) = shake_all();
        let (b, _, _) = shake_all();
        assert_eq!(a, b, "same seed, same shake");
    }
}
