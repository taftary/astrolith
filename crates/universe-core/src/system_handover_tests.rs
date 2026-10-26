//! Frame handover tests and the #995 T3 probe, split under `E-FILE-SIZE`.

use super::{Frame, LEAVE_FACTOR, frame_for, from_star_axes, locate, to_star, to_star_axes};
use crate::nav::DEMO_SEED;
use crate::system_space::{
    AU_M, BodyKind, SystemSpace, distance_m, home_l9_chain, influence_radius_m, sample_l9_chains,
    system_space,
};

fn home() -> SystemSpace {
    let chain = home_l9_chain(DEMO_SEED).expect("home chain");
    system_space(DEMO_SEED, &chain).expect("home system")
}

fn unit(v: [f64; 3]) -> [f64; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / l, v[1] / l, v[2] / l]
}

fn angle_deg(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (a, b) = (unit(a), unit(b));
    let cross = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    let sin = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    let cos = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    sin.atan2(cos).to_degrees()
}

#[test]
fn a_point_survives_every_frame() {
    // #995 T3: holding a point in any frame and reading it back in the star
    // frame lands within a few `f64` steps of where it was.
    let space = home();
    let index = space
        .bodies
        .iter()
        .position(|b| matches!(b.kind, BodyKind::Planet { .. }))
        .expect("a planet");
    let earth = space.bodies.get(index).expect("planet");
    let p = [
        earth.position_m[0] + 7.0e6,
        earth.position_m[1] - 3.0e5,
        earth.position_m[2] + 1.0e6,
    ];
    for frame in [Frame::Star, Frame::Orbit(index), Frame::Spin(index)] {
        let back = to_star(&space, locate(&space, frame, p));
        assert!(distance_m(back, p) < 1e-3, "{frame:?}");
    }
    let v = [0.3, -0.5, 0.8];
    let turned = to_star_axes(
        &space,
        Frame::Spin(3),
        from_star_axes(&space, Frame::Spin(3), v),
    );
    assert!(angle_deg(turned, v) < 1e-9);
}

#[test]
fn a_zone_is_left_further_out_than_it_is_entered() {
    // #995 T3: just outside a planet's gravity zone you stay in its frame if
    // you were in it, and stay out if you were out; past `LEAVE_FACTOR` you
    // leave.
    let space = home();
    // The outermost planet: no other zone reaches it.
    let index = space
        .bodies
        .iter()
        .rposition(|b| matches!(b.kind, BodyKind::Planet { .. }))
        .expect("a planet");
    let planet = space.bodies.get(index).expect("planet");
    let r = influence_radius_m(&space, planet);
    let at = |k: f64| {
        [
            planet.position_m[0],
            planet.position_m[1] + r * k,
            planet.position_m[2],
        ]
    };
    assert_eq!(frame_for(&space, at(1.1), Frame::Star), Frame::Star);
    assert_eq!(
        frame_for(&space, at(1.1), Frame::Orbit(index)),
        Frame::Orbit(index)
    );
    assert_eq!(
        frame_for(&space, at(LEAVE_FACTOR * 1.01), Frame::Orbit(index)),
        Frame::Star
    );
    assert_eq!(frame_for(&space, at(0.9), Frame::Star), Frame::Orbit(index));
}

/// What one scripted zoom measured.
#[derive(Debug, Default)]
struct PathReport {
    handovers: usize,
    jump_m: f64,
    direction_deg: f64,
    roll_deg: f64,
    speed_change: f64,
    near_error_m: f64,
}

/// Zooms from 20 AU above the ecliptic down to `floor_m` above body
/// `target` and back out, holding the camera in its current frame and
/// handing it over as [`frame_for`] says.
fn zoom(space: &SystemSpace, target: usize, floor_m: f64) -> PathReport {
    let body = *space.bodies.get(target).expect("target");
    let dir = unit([0.3, 1.0, 0.2]);
    let mut report = PathReport::default();
    let start_h = 20.0 * AU_M;
    let mut heights = Vec::new();
    let mut h = start_h;
    while h > floor_m {
        heights.push(h);
        h *= 0.85;
    }
    heights.push(floor_m);
    let back: Vec<f64> = heights.iter().rev().copied().collect();
    heights.extend(back);
    let mut frame = Frame::Star;
    for h in heights {
        let reach = body.radius_m + h;
        let centre = locate(space, frame, body.position_m).local_m;
        let d = from_star_axes(space, frame, dir);
        let mut at = super::Located {
            frame,
            local_m: [
                centre[0] + d[0] * reach,
                centre[1] + d[1] * reach,
                centre[2] + d[2] * reach,
            ],
        };
        let near = (0..3)
            .map(|i| (at.local_m[i] - centre[i]).powi(2))
            .sum::<f64>()
            .sqrt();
        if h < 1_000.0 {
            report.near_error_m = report.near_error_m.max((near - reach).abs());
        }
        let next = frame_for(space, to_star(space, at), frame);
        if next != frame {
            report.handovers += 1;
            let before = to_star(space, at);
            let look_before = to_star_axes(space, frame, from_star_axes(space, frame, dir));
            let gap_before = near - body.radius_m;
            at = locate(space, next, before);
            let after = to_star(space, at);
            let look_after = to_star_axes(space, next, from_star_axes(space, next, look_before));
            let centre_after = locate(space, next, body.position_m).local_m;
            let gap_after = distance_m(at.local_m, centre_after) - body.radius_m;
            report.jump_m = report.jump_m.max(distance_m(before, after));
            report.direction_deg = report.direction_deg.max(angle_deg(look_before, look_after));
            report.roll_deg = report.roll_deg.max(angle_deg(look_before, look_after));
            report.speed_change = report
                .speed_change
                .max((gap_after / gap_before - 1.0).abs());
            frame = next;
        }
    }
    report
}

#[test]
fn zooms_hand_over_without_a_jump() {
    // #995 T3 probe: from the system view down to the home planet's ground,
    // the star, the Moon and every sample system's first planet, and back
    // out. Each handover keeps position within 1 mm, direction and up within
    // 0.01 degrees and the speed (distance to the surface) within 1e-6; the
    // camera's distance to the target near the ground stays within 1 cm; a
    // path crosses each boundary once each way (no flicker).
    let mut systems = vec![("home", home())];
    for (n, chain) in sample_l9_chains(DEMO_SEED, 2).into_iter().enumerate() {
        let name = if n == 0 { "sample-a" } else { "sample-b" };
        systems.push((name, system_space(DEMO_SEED, &chain).expect("system")));
    }
    for (name, space) in &systems {
        let mut targets = vec![(String::from("star"), 0usize, 1_000.0)];
        if let Some(first) = space
            .bodies
            .iter()
            .position(|b| matches!(b.kind, BodyKind::Planet { .. }))
        {
            targets.push((String::from("planet"), first, 2.0));
        }
        if let Some((index, moon)) = space
            .bodies
            .iter()
            .enumerate()
            .find(|(_, b)| matches!(b.kind, BodyKind::Moon { .. }))
        {
            let BodyKind::Moon { planet, .. } = moon.kind else {
                unreachable!("a moon")
            };
            targets.push((String::from("moon-planet"), planet, 2.0));
            targets.push((String::from("moon"), index, 2.0));
        }
        for (label, index, floor) in targets {
            let r = zoom(space, index, floor);
            println!(
                "SYSTEM-HANDOVER system={name} target={label} handovers={} jump_mm={:.4} dir_deg={:.2e} roll_deg={:.2e} speed_change={:.2e} near_error_mm={:.4}",
                r.handovers,
                r.jump_m * 1e3,
                r.direction_deg,
                r.roll_deg,
                r.speed_change,
                r.near_error_m * 1e3
            );
            assert!(r.jump_m < 1e-3, "{name} {label}: {r:?}");
            assert!(
                r.direction_deg < 0.01 && r.roll_deg < 0.01,
                "{name} {label}: {r:?}"
            );
            assert!(r.speed_change < 1e-6, "{name} {label}: {r:?}");
            assert!(r.near_error_m < 1e-2, "{name} {label}: {r:?}");
            assert!(r.handovers <= 6, "{name} {label}: flicker {r:?}");
        }
    }
}

#[test]
fn star_frame_precision_by_distance() {
    // #995 T3: the spacing of `f64` star-frame positions at 1 AU, at the
    // heliopause (L8, 120 AU) and at the Oort edge (L7, 100,000 AU). Inside
    // a gravity zone positions are held from the body instead.
    for (label, au, limit_m) in [
        ("1au", 1.0, 1e-4),
        ("120au", 120.0, 1e-2),
        ("oort", 100_000.0, 4.0),
    ] {
        let x = au * AU_M;
        let step = x.next_up() - x;
        println!("SYSTEM-PRECISION at={label} step_mm={:.4}", step * 1e3);
        assert!(step < limit_m, "{label}: {step}");
    }
}
