//! Precision measurement for the planet frame (#774, Q5 A, T3).
//!
//! `shake` places test vertices as stored L10 cell-unit positions around a
//! camera, runs them through the exact render path
//! (`l10_drawn_f32`: the same math `RenderOrigin::at` uses at L10, review
//! #774 item 1), rebuilds them in metres, and reports the worst error: in mm
//! for points within 1 km, in pixels at 1600×900 (70° vertical field of view)
//! farther out.
//!
//! Measured at 1 cm, 1 m, 100 m, 10 km, 1,000 km and 2,000 km above the true
//! radius, over the cube-face centre `(0,1,0)`, an edge `norm(1,1,0)` and a
//! corner `norm(1,1,1)` (where step 2 puts them), on Earth and two other
//! seeded planets. Pure and deterministic (`E-DET-TIERS`).
//!
//! The worst near value is the `f32` rounding of a ~1 km vector by
//! construction: it is constant across heights and planets while the
//! camera-relative pipeline holds, which is exactly the property step 1 must
//! prove (a height- or planet-dependent shake would mean absolute scale
//! leaking into the error). The gate is real all the same: it exercises the
//! render conversion itself, so breaking that conversion fails the test.

use crate::planet_frame::{PlanetFrame, PlanetPos, l10_drawn_f32};

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

/// Error of one stored cell-unit vertex through the exact render path.
///
/// Returns `(error_m, distance_m)`: the drawn `[f32; 3]` cells rebuilt to
/// metres against the true camera-relative vector, plus the true distance.
#[must_use]
pub fn drawn_error(
    point_cells: [f64; 3],
    camera_cells: [f64; 3],
    frame: &PlanetFrame,
) -> (f64, f64) {
    let drawn = l10_drawn_f32(point_cells, camera_cells, frame);
    let mpc = frame.metres_per_cell();
    let dx = f64::from(drawn[0]) - (point_cells[0] - camera_cells[0]);
    let dy = f64::from(drawn[1]) - (point_cells[1] - camera_cells[1]);
    let dz = f64::from(drawn[2]) - (point_cells[2] - camera_cells[2]);
    let error_m = length([dx * mpc, dy * mpc, dz * mpc]);
    let distance_m = length([
        (point_cells[0] - camera_cells[0]) * mpc,
        (point_cells[1] - camera_cells[1]) * mpc,
        (point_cells[2] - camera_cells[2]) * mpc,
    ]);
    (error_m, distance_m)
}

/// Test vertices as stored L10 cell-unit positions around `camera_cells`.
///
/// Near vertices span 1 mm to 1 km in cell units along five directions, as
/// stored vertices quantize on the `f64` cell grid; the far vertex is the
/// surface point below the camera (`to_cell` of the true surface).
fn points_around_cells(camera_cells: [f64; 3], frame: &PlanetFrame) -> Vec<[f64; 3]> {
    let mpc = frame.metres_per_cell();
    let mut points = Vec::new();
    // Near offsets in metres, stored as cell units.
    let offsets_m = [0.001, 0.01, 0.1, 1.0, 10.0, 100.0, 1_000.0];
    let dirs = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [-1.0, 0.5, 0.25],
        [0.3, -0.7, 0.6],
    ];
    for &metres in &offsets_m {
        let cells = metres / mpc;
        for dir in dirs {
            let len = length(dir);
            let unit = [dir[0] / len, dir[1] / len, dir[2] / len];
            points.push([
                camera_cells[0] + unit[0] * cells,
                camera_cells[1] + unit[1] * cells,
                camera_cells[2] + unit[2] * cells,
            ]);
        }
    }
    // The surface point below the camera through `to_cell`.
    let r_cells = length(camera_cells);
    if r_cells.is_finite() && r_cells > 0.0 {
        let up = [
            camera_cells[0] / r_cells,
            camera_cells[1] / r_cells,
            camera_cells[2] / r_cells,
        ];
        let surf_cells = frame.radius_cells();
        points.push([up[0] * surf_cells, up[1] * surf_cells, up[2] * surf_cells]);
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
            let camera_planet = [unit[0] * r, unit[1] * r, unit[2] * r];
            let camera_cells = frame.to_cell(PlanetPos(camera_planet));
            for point_cells in points_around_cells(camera_cells, frame) {
                let (error_m, distance_m) = drawn_error(point_cells, camera_cells, frame);
                if distance_m <= NEAR_M {
                    let err_mm = error_m * 1_000.0;
                    if err_mm > worst_mm {
                        worst_mm = err_mm;
                    }
                    if !err_mm.is_finite() || err_mm >= 1.0 {
                        pass = false;
                    }
                } else {
                    let err_px = if distance_m > 0.0 {
                        error_m / distance_m * focal_px()
                    } else {
                        0.0
                    };
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
        // planet; under one pixel farther out. Measured through the exact
        // render path (`l10_drawn_f32`, review #774 item 1).
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
