//! Ground clearance for window dives and window free flight (#430, #632).
//!
//! Split from [`crate::terrain`] under `E-FILE-SIZE`: the terrain module
//! owns generation (heightmap, sampler, bodies), this module owns the
//! pure clamp the window shares. Since #632 the only surface is the open
//! planet (L10), and the standoff is in metres, so a dive or free flight
//! comes down to the ground through every shell level. Everything here is
//! pure and deterministic in its inputs (`E-DET-TIERS`).

use crate::surface::SurfaceFrame;
use crate::terrain::surface_radius;

/// Standoff kept above the ground, in metres (#430, #632).
///
/// Window dives and window free flight never dip below the flat-faced
/// surface plus this: low enough to stand in L14 (below 10 m), high
/// enough to stay clear of the ground.
pub const GROUND_STANDOFF_M: f64 = 1.0;

/// Camera clamped above the ground of the open planet (#430, #593, #632).
///
/// Keeps `|camera|` above the surface under it plus [`GROUND_STANDOFF_M`]
/// (radial push-out, `+Y` when on the centre). Off a planet (no frame) and
/// for non-finite inputs the camera passes through unchanged. Pure and
/// deterministic: the window dive and window free flight share it.
#[must_use]
pub fn clamp_above_ground(frame: Option<&SurfaceFrame>, camera: [f64; 3]) -> [f64; 3] {
    let Some(frame) = frame else {
        return camera;
    };
    if !camera.iter().all(|c| c.is_finite()) {
        return camera;
    }
    let distance = (camera[0] * camera[0] + camera[1] * camera[1] + camera[2] * camera[2]).sqrt();
    let floor = surface_radius(camera) + frame.standoff();
    if !floor.is_finite() || distance >= floor {
        return camera;
    }
    if distance > 1e-12 {
        let push = floor / distance;
        [camera[0] * push, camera[1] * push, camera[2] * push]
    } else {
        [0.0, floor, 0.0]
    }
}

#[cfg(test)]
mod clearance_tests {
    use super::*;
    use crate::terrain::{PLANET_RADIUS_CELL, surface_floor_radius};

    #[test]
    fn the_floor_follows_the_flat_faces() {
        for direction in [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.3, -0.5, 0.8],
        ] {
            let radius = surface_radius(direction);
            assert!(
                (surface_floor_radius()..=PLANET_RADIUS_CELL + 1e-12).contains(&radius),
                "surface radius {radius} between the faces and the sphere"
            );
        }
    }

    #[test]
    fn the_clamp_pushes_out_to_one_metre_and_leaves_the_sky_alone() {
        let planet = SurfaceFrame::planet(42);
        let standoff = planet.standoff();
        let metres = standoff * crate::shell::metres_per_unit();
        assert!(
            (metres - GROUND_STANDOFF_M).abs() < 1e-9,
            "standoff {metres} m"
        );
        let buried = [0.05, 0.02, -0.03];
        let out = clamp_above_ground(Some(&planet), buried);
        let distance = (out[0] * out[0] + out[1] * out[1] + out[2] * out[2]).sqrt();
        assert!((distance - (surface_radius(buried) + standoff)).abs() < 1e-12);
        // Radial push: direction unchanged.
        let pushed = distance
            / (buried[0] * buried[0] + buried[1] * buried[1] + buried[2] * buried[2]).sqrt();
        for (got, want) in
            out.iter()
                .zip([buried[0] * pushed, buried[1] * pushed, buried[2] * pushed])
        {
            assert!((got - want).abs() < 1e-12);
        }
        let centre = clamp_above_ground(Some(&planet), [0.0; 3]);
        assert_eq!(centre[0], 0.0);
        assert!(centre[1] >= surface_floor_radius() + standoff);
        let high = [0.0, 0.0, 3.58];
        assert_eq!(clamp_above_ground(Some(&planet), high), high);
        // Ten metres up (L14) is above the standoff: untouched.
        let up = [0.0, 0.0, 1.0];
        let ten = 10.0 / crate::shell::metres_per_unit();
        let l14 = [0.0, 0.0, surface_radius(up) + ten];
        assert_eq!(clamp_above_ground(Some(&planet), l14), l14);
    }

    #[test]
    fn clamp_passes_through_off_the_surface_and_on_bad_input() {
        let planet = SurfaceFrame::planet(42);
        let bad = [f64::NAN, 0.0, 0.0];
        let out = clamp_above_ground(Some(&planet), bad);
        assert!(out[0].is_nan() && out[1] == 0.0 && out[2] == 0.0);
        let deep = [0.0, -5.0, 0.0];
        assert_eq!(clamp_above_ground(None, deep), deep);
    }

    #[test]
    fn a_body_walks_on_the_flat_faces_across_their_edges() {
        use crate::body::{Body, BodyInput, Turning, body_step, level_ground, yaw_of};
        let chain = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
        let mut universe = crate::nav::Universe::new(crate::nav::DEMO_SEED);
        for &marker in chain.get(..9).expect("journey to L10") {
            assert!(universe.open(crate::nav::MarkerIndex(marker)));
        }
        assert_eq!(universe.level().get(), 10);
        // Start on a face, heading along a circle that crosses face edges.
        let start = [1.0, 0.3, 0.0];
        let feet = level_ground(&universe, start).expect("planet ground").point;
        let up = level_ground(&universe, feet).expect("planet ground").up;
        let mut body = Body::standing(feet, yaw_of(up, [0.0, 0.0, 1.0]));
        let forward = BodyInput {
            forward: true,
            ..BodyInput::default()
        };
        let mut faces = std::collections::HashSet::new();
        for _ in 0..400 {
            let pull = crate::gravity::pull(&universe, body.position).accel;
            body = body_step(
                body,
                forward,
                universe.level(),
                |p| level_ground(&universe, p),
                pull,
                Turning::Drone,
                16.0,
                600.0,
            );
            let radius = (body.position[0] * body.position[0]
                + body.position[1] * body.position[1]
                + body.position[2] * body.position[2])
                .sqrt();
            assert!(body.on_ground, "the body stays on the ground");
            assert!(
                (radius - surface_radius(body.position)).abs() < 1e-12,
                "the feet sit on the flat face: {radius}"
            );
            faces.insert(crate::icosphere::locate(
                body.position,
                crate::terrain::FACET_DEPTH,
            ));
        }
        assert!(faces.len() >= 2, "the walk crossed a face edge");
    }
}
