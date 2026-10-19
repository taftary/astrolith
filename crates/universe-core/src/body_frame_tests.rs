//! Local frame tests (#779 T2): the frame switch moves the body under a
//! millimetre, and the body stays upright with its feet on slopes.

use crate::body::{Body, BodyInput, Ground, Turning, body_step};
use crate::body_frame::{BodyAnchor, LOCAL_BELOW_M, step_near_ground};
use crate::coords::Level;
use crate::drawn_surface::DrawnSurface;
use crate::drawn_surface_tests::{earth, surface_at};
use crate::planet_frame::{PlanetFrame, PlanetPos};

fn unit(a: [f64; 3]) -> [f64; 3] {
    let len = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    [a[0] / len, a[1] / len, a[2] / len]
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// Gravity toward the centre in open-cell units per second squared.
fn pull_at(frame: &PlanetFrame, position: [f64; 3]) -> [f64; 3] {
    let down = unit(position);
    let g = 9.81 / frame.metres_per_cell();
    [-down[0] * g, -down[1] * g, -down[2] * g]
}

/// A body `height_m` above the drawn ground under the camera of `surface`.
fn body_above(
    surface: &DrawnSurface,
    frame: &PlanetFrame,
    camera: PlanetPos,
    height_m: f64,
    flying: bool,
) -> Body {
    let (ground, up) = surface.ground_at_planet(camera).expect("ground");
    let p = PlanetPos([
        ground.0[0] + up[0] * height_m,
        ground.0[1] + up[1] * height_m,
        ground.0[2] + up[2] * height_m,
    ]);
    let mut body = Body::standing(frame.to_cell(p), 0.3);
    body.flying = flying;
    body
}

#[test]
fn local_frame_switch_moves_under_a_millimetre() {
    let (relief, frame) = earth();
    let (mountain, _) = crate::headless_morph::earth_mountain_and_plains_dirs();
    let (surface, camera) = surface_at(&relief, frame, mountain, 40.0);
    let ground = |p: [f64; 3]| -> Option<Ground> { surface.ground_at(p) };
    let level = Level::new(10).expect("level");
    let dt = 1.0 / 60.0;
    // Fly fast just above the ground for 30 s (several kilometres, several
    // re-anchors), then climb through 100 m and come back down.
    let mut local = body_above(&surface, &frame, camera, 40.0, true);
    let mut plain = local;
    let mut anchor = BodyAnchor::default();
    let mut origins = Vec::new();
    let mut worst = 0.0f64;
    let mut crossed_up = false;
    for frame_no in 0..2_400u32 {
        let input = BodyInput {
            forward: true,
            fast: true,
            space_held: (600..900).contains(&frame_no),
            down_held: (900..1_300).contains(&frame_no),
            ..BodyInput::default()
        };
        let pull = pull_at(&frame, local.position);
        local = step_near_ground(
            local,
            input,
            level,
            &frame,
            &mut anchor,
            ground,
            pull,
            Turning::Drone,
            64.0,
            true,
            0.3,
            false,
            dt,
        );
        plain = body_step(
            plain,
            input,
            level,
            ground,
            pull,
            Turning::Drone,
            64.0,
            true,
            0.3,
            false,
            dt,
        );
        if let Some(a) = anchor.0
            && origins
                .last()
                .is_none_or(|o: &[f64; 3]| dist(*o, a.origin.0) > 1.0)
        {
            origins.push(a.origin.0);
        }
        if anchor.0.is_none() {
            crossed_up = true;
        }
        let gap = dist(
            frame.from_cell(local.position).0,
            frame.from_cell(plain.position).0,
        );
        worst = worst.max(gap);
        // Keep both on the same path: the comparison is per step.
        plain = local;
    }
    assert!(origins.len() >= 3, "only {} anchors", origins.len());
    assert!(crossed_up, "never rose above {LOCAL_BELOW_M} m");
    assert!(
        worst < 1e-3,
        "a step differs by {worst} m from the planet frame"
    );
}

#[test]
fn body_stays_upright_on_slopes() {
    let (relief, frame) = earth();
    let (mountain, _) = crate::headless_morph::earth_mountain_and_plains_dirs();
    let (surface, camera) = surface_at(&relief, frame, mountain, 1.7);
    let ground = |p: [f64; 3]| -> Option<Ground> { surface.ground_at(p) };
    let level = Level::new(10).expect("level");
    let dt = 1.0 / 60.0;
    let mut body = body_above(&surface, &frame, camera, 0.0, false);
    let mut anchor = BodyAnchor::default();
    let mut climbed = 0.0f64;
    let start = frame.from_cell(body.position);
    for _ in 0..600u32 {
        let input = BodyInput {
            forward: true,
            ..BodyInput::default()
        };
        let pull = pull_at(&frame, body.position);
        body = step_near_ground(
            body,
            input,
            level,
            &frame,
            &mut anchor,
            ground,
            pull,
            Turning::Drone,
            1.0,
            true,
            0.3,
            false,
            dt,
        );
        let g = surface.ground_at(body.position).expect("ground");
        // Up is radial, so the body stands upright.
        let radial = unit(frame.from_cell(body.position).0);
        let along = g.up[0] * radial[0] + g.up[1] * radial[1] + g.up[2] * radial[2];
        assert!((along - 1.0).abs() < 1e-9, "up not radial");
        // Feet on the drawn ground.
        let p = frame.from_cell(body.position).0;
        let q = frame.from_cell(g.point).0;
        let gap = (p[0] - q[0]) * g.up[0] + (p[1] - q[1]) * g.up[1] + (p[2] - q[2]) * g.up[2];
        assert!(gap.abs() < 0.01, "feet {gap} m off the ground");
        climbed = climbed.max((dist(p, [0.0; 3]) - dist(start.0, [0.0; 3])).abs());
    }
    assert!(
        climbed > 0.05,
        "the walk never left level ground ({climbed} m)"
    );
}

#[test]
fn player_starts_standing_on_the_ground() {
    // Q7 A: the seeded spot is dropped onto the drawn ground; the body
    // stands there (not flying), feet on the surface.
    let (relief, frame) = earth();
    let spot = crate::player::PlayerSpot::seeded(crate::nav::DEMO_SEED).expect("spot");
    let dir = frame.from_cell(spot.position).0;
    let (surface, _) = surface_at(&relief, frame, dir, 1.7);
    let feet = surface
        .place_on_ground(spot.position)
        .expect("ground under the spot");
    let body = Body::standing(feet, spot.yaw);
    assert!(!body.flying, "starts flying");
    let g = surface.ground_at(body.position).expect("ground");
    let gap = dist(frame.from_cell(body.position).0, frame.from_cell(g.point).0);
    assert!(gap < 1e-3, "feet {gap} m off the drawn ground");
}
