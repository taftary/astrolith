//! Ground clearance for window dives and window free flight (#430).
//!
//! Split from [`crate::terrain`] under `E-FILE-SIZE`: the terrain module
//! owns generation (heightmap, sampler, bodies), this module owns the
//! pure clamp the window shares. Everything here is pure and deterministic
//! in its inputs (`E-DET-TIERS`); replays never call it, so the headless
//! journey stays byte-identical (`E-GOLDEN`).

use crate::surface::SurfaceFrame;
use crate::terrain::surface_radius;

/// Standoff kept above the ground (#430), in open-cell units.
///
/// Window dives and window free flight never dip below the flat-faced
/// surface plus this, while staying far below the landing rest (about 3.58
/// child units above a portal), so landings never touch it.
pub const GROUND_STANDOFF: f64 = 0.03;

/// Ground height under `(x, z)` in the open tail cell (#430, #458).
///
/// The flat-faced surface of the one icosphere under the cell column
/// ([`SurfaceFrame::ground_height`]), so the floor agrees with the drawn
/// mesh and the portals (C2/C7). Returns `0.0` outside L11-L13.
#[must_use]
pub fn ground_height(frame: &SurfaceFrame, x: f64, z: f64) -> f64 {
    if !(11..=13).contains(&frame.level().get()) {
        return 0.0;
    }
    frame.ground_height(x, z)
}

/// Camera clamped above the ground (#430, #458, #593).
///
/// L10 keeps `|camera|` above the surface under it plus [`GROUND_STANDOFF`]
/// (radial push-out, `+Y` when on the centre); L11-L13 keep `camera.y`
/// above [`ground_height`] plus the standoff. Off the surface (no frame)
/// and for non-finite inputs the camera passes through unchanged. Pure and
/// deterministic: the window dive and window free flight share it,
/// replays never call it (`E-GOLDEN`).
#[must_use]
pub fn clamp_above_ground(frame: Option<&SurfaceFrame>, camera: [f64; 3]) -> [f64; 3] {
    clamp_above_ground_with_cap(frame, camera, None)
}

/// Camera clamped above the ground, with a landing cap (#430).
///
/// `floor_cap` lowers the floor to the landing point's height when set: a
/// window landing dive passes its target portal's rest height
/// ([`crate::landing::landing_distance`] above the portal), which can sit
/// below the standoff at deep tail levels, yet always above the surface
/// itself, so the cap keeps the dive reachable without ever letting the
/// camera under the ground: the floor never drops below the surface under
/// the camera, whatever the cap. `None` keeps the full standoff (centre dives,
/// free flight, transitions). The cap is in the same units as the floor:
/// radial distance at L10, height at L11-L13.
#[must_use]
pub fn clamp_above_ground_with_cap(
    frame: Option<&SurfaceFrame>,
    camera: [f64; 3],
    floor_cap: Option<f64>,
) -> [f64; 3] {
    let Some(frame) = frame else {
        return camera;
    };
    if !camera.iter().all(|c| c.is_finite()) {
        return camera;
    }
    // The cap lowers the standoff toward the landing rest but never below
    // the surface itself: the rest height of a far portal can sit under
    // the ground beneath the camera (#458).
    let capped = |surface: f64| match floor_cap {
        Some(cap) if cap.is_finite() => (surface + GROUND_STANDOFF).min(cap).max(surface),
        _ => surface + GROUND_STANDOFF,
    };
    match frame.level().get() {
        10 => {
            let distance =
                (camera[0] * camera[0] + camera[1] * camera[1] + camera[2] * camera[2]).sqrt();
            let floor = capped(surface_radius(camera));
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
        11..=13 => {
            let floor = capped(ground_height(frame, camera[0], camera[2]));
            if floor.is_finite() && camera[1] < floor {
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
    use crate::r#gen::MarkerKind;
    use crate::terrain::{PLANET_RADIUS_CELL, surface_floor_radius};

    /// The demo journey's surface frame at level `n` (L10-L13).
    fn frame(n: u8) -> SurfaceFrame {
        let depth = usize::from(n) - 1;
        let mut chain = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
        // The journey stops at L10; deeper frames extend it with the L10
        // cell's first portal point, then the first enterable cluster.
        while chain.len() < depth {
            if chain.len() == 9 {
                let cell = crate::nest::generate_cell(crate::nav::DEMO_SEED, &chain);
                let portal = cell
                    .points
                    .iter()
                    .position(|point| point.kind == MarkerKind::Portal)
                    .expect("L10 portal");
                chain.push(u32::try_from(portal).expect("marker fits u32"));
            } else {
                chain.push(0);
            }
        }
        let named = chain.get(..depth).expect("journey reaches the surface");
        SurfaceFrame::for_path(crate::nav::DEMO_SEED, named).expect("surface frame")
    }

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
    fn l10_clamp_pushes_out_and_leaves_height_alone() {
        let planet = SurfaceFrame::planet(42);
        let buried = [0.05, 0.02, -0.03];
        let out = clamp_above_ground(Some(&planet), buried);
        let distance = (out[0] * out[0] + out[1] * out[1] + out[2] * out[2]).sqrt();
        assert!(distance >= surface_radius(buried) + GROUND_STANDOFF - 1e-12);
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
        assert!(centre[1] >= surface_floor_radius() + GROUND_STANDOFF);
        let high = [0.0, 0.0, 3.58];
        assert_eq!(clamp_above_ground(Some(&planet), high), high);
    }

    #[test]
    fn tail_clamp_raises_to_the_surface_plus_standoff() {
        for n in 11..=13u8 {
            let surface = frame(n);
            let ground = ground_height(&surface, 0.3, -0.1);
            let under = [0.3, ground - 0.5, -0.1];
            let out = clamp_above_ground(Some(&surface), under);
            assert_eq!(out[0], under[0]);
            assert_eq!(out[2], under[2]);
            assert!(
                (out[1] - (ground + GROUND_STANDOFF)).abs() < 1e-12,
                "L{n} clamp holds at the standoff"
            );
            let above = [0.3, ground + 1.0, -0.1];
            assert_eq!(clamp_above_ground(Some(&surface), above), above);
        }
        assert_eq!(ground_height(&frame(10), 0.1, 0.0), 0.0, "L10 is radial");
    }

    #[test]
    fn landing_cap_never_drops_the_floor_under_the_surface() {
        for n in 11..=13u8 {
            let surface = frame(n);
            let ground = ground_height(&surface, 0.3, -0.1);
            let under = [0.3, ground - 2.0, -0.1];
            // A cap far below the ground here (a lower portal elsewhere).
            let out = clamp_above_ground_with_cap(Some(&surface), under, Some(ground - 1.0));
            assert!(
                (out[1] - ground).abs() < 1e-12,
                "L{n} floor holds at the surface"
            );
            // A cap between the surface and the standoff wins.
            let mid = ground + GROUND_STANDOFF * 0.5;
            let out = clamp_above_ground_with_cap(Some(&surface), under, Some(mid));
            assert!((out[1] - mid).abs() < 1e-12, "L{n} cap lowers the standoff");
        }
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
    fn standoff_sits_far_below_the_landing_rest() {
        // Landings end ~3.58 child units above a portal; the standoff must
        // never touch them. In L10 parent units the rest is the marker
        // radius over sin(OPEN_ANGLE), an order above the standoff.
        const { assert!(GROUND_STANDOFF < 0.1) };
        let l10 = crate::coords::Level::new(10).expect("L10");
        let rest_parent =
            0.5 * crate::nest::child_ratio(l10).expect("L10 ratio") / crate::nest::OPEN_ANGLE.sin();
        assert!(rest_parent > GROUND_STANDOFF * 2.0);
    }

    #[test]
    fn clearance_follows_the_icosphere_surface() {
        let planet = SurfaceFrame::planet(1234);
        for direction in [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.4, 0.5, -0.6]] {
            let floor = surface_radius(direction) + GROUND_STANDOFF;
            let inside = [
                direction[0] * 0.05,
                direction[1] * 0.05,
                direction[2] * 0.05,
            ];
            let out = clamp_above_ground(Some(&planet), inside);
            let distance = (out[0] * out[0] + out[1] * out[1] + out[2] * out[2]).sqrt();
            assert!(
                (distance - floor).abs() < 1e-9,
                "clearance floor {distance} is not the surface {floor}"
            );
        }
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
                false,
                body.yaw,
                false,
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
