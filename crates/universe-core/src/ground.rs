//! Ground clearance for window dives and window free flight (#430).
//!
//! Split from [`crate::terrain`] under `E-FILE-SIZE`: the terrain module
//! owns generation (heightmap, sampler, bodies), this module owns the
//! pure clamp the window shares. Everything here is pure and deterministic
//! in its inputs (`E-DET-TIERS`); replays never call it, so the headless
//! journey stays byte-identical (`E-GOLDEN`).

use crate::coords::Level;
use crate::frame::planet_radius_cells;
use crate::terrain::surface_radius;

/// Standoff kept above the relief and the ground (#430), in open-cell units.
///
/// Window dives and window free flight never dip below the surface plus
/// this: it clears the relief jitter (`±0.005`), the polar squash (at most
/// `PLANET_RADIUS_CELL * 0.05 ~= 0.018`), and the tail ground jitter, while
/// staying far below the landing rest (about 3.58 child units above a
/// portal), so landings never touch it.
pub const GROUND_STANDOFF: f64 = 0.03;

/// Relief radius under `direction` for the L10 cell `seed` (#430, #458).
///
/// The one icosphere surface radius ([`surface_radius`]), so the clamp
/// agrees with the body on screen and with the tail surface (C2/C7).
/// Falls back to the planet radius for degenerate directions.
#[must_use]
pub fn relief_radius(seed: u64, direction: [f64; 3]) -> f64 {
    surface_radius(seed, direction)
}

/// Ground height under `(x, z)` at a tail level (#430).
///
/// The settled bowl `y = -(x^2 + z^2) / (2R)` the tail generator lays
/// points on, with `R` the true planet radius at this level's scale.
/// Returns `0.0` outside L11-L13 or when the radius is degenerate.
#[must_use]
pub fn ground_height(level: Level, x: f64, z: f64) -> f64 {
    if !(11..=13).contains(&level.get()) {
        return 0.0;
    }
    if !x.is_finite() || !z.is_finite() {
        return 0.0;
    }
    let Some(radius) = planet_radius_cells(level) else {
        return 0.0;
    };
    if !radius.is_finite() || radius <= 0.0 {
        return 0.0;
    }
    -(x * x + z * z) / (2.0 * radius)
}

/// Camera clamped above the relief and the ground (#430).
///
/// L10 keeps `|camera|` above the relief under it plus [`GROUND_STANDOFF`]
/// (radial push-out, `+Y` when on the centre); L11-L13 keep `camera.y`
/// above [`ground_height`] plus the standoff. Everywhere else, and for
/// non-finite inputs, the camera passes through unchanged. Pure and
/// deterministic: the window dive and window free flight share it, replays
/// never call it (`E-GOLDEN`).
#[must_use]
pub fn clamp_above_ground(level: Level, seed: u64, camera: [f64; 3]) -> [f64; 3] {
    clamp_above_ground_with_cap(level, seed, camera, None)
}

/// Camera clamped above the relief and the ground, with a landing cap (#430).
///
/// `floor_cap` lowers the floor to the landing point's height when set: a
/// window landing dive passes its target portal's rest height
/// ([`crate::landing::landing_distance`] above the portal), which sits
/// below the standoff at deep tail levels (L12) yet always above the
/// surface itself, so the cap keeps the dive reachable without ever
/// letting the camera under the ground. `None` keeps the full standoff
/// (centre dives, free flight, transitions). The cap is in the same units
/// as the floor: radial distance at L10, height at L11-L13.
#[must_use]
pub fn clamp_above_ground_with_cap(
    level: Level,
    seed: u64,
    camera: [f64; 3],
    floor_cap: Option<f64>,
) -> [f64; 3] {
    if !camera.iter().all(|c| c.is_finite()) {
        return camera;
    }
    match level.get() {
        10 => {
            let distance =
                (camera[0] * camera[0] + camera[1] * camera[1] + camera[2] * camera[2]).sqrt();
            let mut floor = relief_radius(seed, camera) + GROUND_STANDOFF;
            if let Some(cap) = floor_cap
                && cap.is_finite()
            {
                floor = floor.min(cap);
            }
            if !floor.is_finite() {
                return camera;
            }
            if distance < floor {
                if distance > 1e-12 {
                    let push = floor / distance;
                    [camera[0] * push, camera[1] * push, camera[2] * push]
                } else {
                    [0.0, floor, 0.0]
                }
            } else {
                camera
            }
        }
        11..=13 => {
            let mut floor = ground_height(level, camera[0], camera[2]) + GROUND_STANDOFF;
            if let Some(cap) = floor_cap
                && cap.is_finite()
            {
                floor = floor.min(cap);
            }
            if !floor.is_finite() {
                return camera;
            }
            if camera[1] < floor {
                [camera[0], floor, camera[2]]
            } else {
                camera
            }
        }
        _ => camera,
    }
}

#[cfg(test)]
mod clearance_tests {
    use super::*;
    use crate::coords::Level;
    use crate::terrain::{PLANET_RADIUS_CELL, RELIEF_RANGE_CELL};

    fn level(n: u8) -> Level {
        Level::new(n).expect("ladder level")
    }

    #[test]
    fn relief_radius_tracks_the_relief_range() {
        let seed = 42u64;
        for direction in [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.3, -0.5, 0.8],
        ] {
            let radius = relief_radius(seed, direction);
            assert!(
                (PLANET_RADIUS_CELL - RELIEF_RANGE_CELL..=PLANET_RADIUS_CELL + RELIEF_RANGE_CELL)
                    .contains(&radius),
                "relief radius {radius} out of range"
            );
        }
        assert_eq!(
            relief_radius(seed, [0.0; 3]),
            relief_radius(seed, [1.0, 0.0, 0.0])
        );
        assert_eq!(
            relief_radius(seed, [1.0, 0.0, 0.0]),
            relief_radius(seed, [1.0, 0.0, 0.0])
        );
    }

    #[test]
    fn l10_clamp_pushes_out_and_leaves_height_alone() {
        let seed = 42u64;
        let l10 = level(10);
        let buried = [0.05, 0.02, -0.03];
        let out = clamp_above_ground(l10, seed, buried);
        let distance = (out[0] * out[0] + out[1] * out[1] + out[2] * out[2]).sqrt();
        assert!(distance >= relief_radius(seed, buried) + GROUND_STANDOFF - 1e-12);
        // Radial push: direction unchanged.
        let pushed = distance
            / (buried[0] * buried[0] + buried[1] * buried[1] + buried[2] * buried[2]).sqrt();
        for (got, want) in
            out.iter()
                .zip([buried[0] * pushed, buried[1] * pushed, buried[2] * pushed])
        {
            assert!((got - want).abs() < 1e-12);
        }
        let centre = clamp_above_ground(l10, seed, [0.0; 3]);
        assert_eq!(centre[0], 0.0);
        assert!(centre[1] >= PLANET_RADIUS_CELL - RELIEF_RANGE_CELL + GROUND_STANDOFF);
        let high = [0.0, 0.0, 3.58];
        assert_eq!(clamp_above_ground(l10, seed, high), high);
    }

    #[test]
    fn tail_clamp_raises_to_the_bowl_plus_standoff() {
        let l11 = level(11);
        let sag = ground_height(l11, 0.4, 0.0);
        assert!(sag < 0.0, "the bowl sags below zero");
        let under = [0.4, sag - 0.5, 0.0];
        let out = clamp_above_ground(l11, 42, under);
        assert_eq!(out[0], under[0]);
        assert_eq!(out[2], under[2]);
        assert!((out[1] - (sag + GROUND_STANDOFF)).abs() < 1e-12);
        let above = [0.4, sag + 1.0, 0.0];
        assert_eq!(clamp_above_ground(l11, 42, above), above);
        assert_eq!(ground_height(level(10), 0.4, 0.0), 0.0);
        assert_eq!(ground_height(level(14), 0.4, 0.0), 0.0);
    }

    #[test]
    fn clamp_passes_through_elsewhere_and_on_bad_input() {
        let bad = [f64::NAN, 0.0, 0.0];
        let out = clamp_above_ground(level(10), 42, bad);
        assert!(out[0].is_nan() && out[1] == 0.0 && out[2] == 0.0);
        let deep = [0.0, -5.0, 0.0];
        assert_eq!(clamp_above_ground(level(5), 42, deep), deep);
        assert_eq!(clamp_above_ground(level(14), 42, deep), deep);
    }

    #[test]
    fn standoff_sits_far_below_the_landing_rest() {
        // Landings end ~3.58 child units above a portal; the standoff must
        // never touch them. In L10 parent units the rest is the marker
        // radius over sin(OPEN_ANGLE), an order above the standoff.
        const { assert!(GROUND_STANDOFF < 0.1) };
        let l10 = level(10);
        let rest_parent =
            0.5 * crate::nest::child_ratio(l10).expect("L10 ratio") / crate::nest::OPEN_ANGLE.sin();
        assert!(rest_parent > GROUND_STANDOFF * 2.0);
    }

    #[test]
    fn clearance_follows_the_icosphere_surface() {
        use crate::terrain::surface_radius;
        let seed = 1234u64;
        let l10 = level(10);
        for direction in [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.4, 0.5, -0.6]] {
            let floor = surface_radius(seed, direction) + GROUND_STANDOFF;
            let inside = [
                direction[0] * 0.05,
                direction[1] * 0.05,
                direction[2] * 0.05,
            ];
            let out = clamp_above_ground(l10, seed, inside);
            let distance = (out[0] * out[0] + out[1] * out[1] + out[2] * out[2]).sqrt();
            assert!(
                (distance - floor).abs() < 1e-9,
                "clearance floor {distance} is not the surface {floor}"
            );
            assert_eq!(
                relief_radius(seed, direction).to_bits(),
                surface_radius(seed, direction).to_bits()
            );
        }
    }
}
