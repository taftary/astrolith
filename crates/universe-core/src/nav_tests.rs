//! `nav` tests, split from `nav` under `E-FILE-SIZE` (#995).
use super::*;
use crate::nest::{CLOSE_ANGLE, OPEN_ANGLE, PREVIEW_CAP};
use crate::stream::STREAM_CAP;

#[test]
fn dive_step_closes_and_never_crosses_surface() {
    let mut camera = OpenUnits([0.0, 0.0, 4.0]);
    let center = OpenUnits([0.0, 0.0, 0.0]);
    let radius = 0.2;
    let mut previous = 4.0;
    for step in 0..200 {
        camera = dive_step(camera, center, radius, WHEEL_FACTOR);
        let distance = length3(camera.0);
        if step < 20 {
            assert!(distance < previous, "did not approach at step {step}");
        } else {
            assert!(distance <= previous, "pulled away at step {step}");
        }
        assert!(distance > radius, "crossed the surface");
        previous = distance;
    }
    let away = dive_step(camera, center, radius, 1.0 / WHEEL_FACTOR);
    assert!(length3(away.0) > previous);
}

#[test]
fn thresholds_have_hysteresis() {
    const { assert!(OPEN_ANGLE > CLOSE_ANGLE) };
    let open_distance = 0.5 / OPEN_ANGLE.sin();
    assert!(should_open(0.5, open_distance * 0.99));
    assert!(!should_open(0.5, open_distance * 1.01));
    assert!(!should_close(open_distance));
    assert!(should_close(0.5 / CLOSE_ANGLE.sin() * 1.01));
}

#[test]
fn small_moves_after_entry_stay_inside_and_the_slack_is_the_same_for_keys_and_wheel() {
    // Entry rests at 0.5 / sin(OPEN_ANGLE) child units and the cell
    // closes past 0.5 / sin(CLOSE_ANGLE); the slack between them is a
    // pure distance ratio, so wheel notches and held-key frames spend
    // it identically (#403 AC3, replacing the dive-counted settle).
    let slack = OPEN_ANGLE.sin() / CLOSE_ANGLE.sin();
    assert!(
        slack > 1.0 / WHEEL_FACTOR.powi(3),
        "entry must survive three outward wheel notches"
    );
    let mut universe = Universe::new(DEMO_SEED);
    let marker = universe.autopilot_target().expect("root has markers");
    let mut event = DiveEvent::Moved;
    for _ in 0..500 {
        event = universe.dive(Some(marker), WHEEL_FACTOR, DiveMode::Targeted);
        if event != DiveEvent::Moved {
            break;
        }
    }
    assert_eq!(event, DiveEvent::Opened(marker));
    let rest = universe.path.distance_to_center();
    for notch in 1..=3 {
        assert_eq!(
            universe.dive(None, 1.0 / WHEEL_FACTOR, DiveMode::Targeted),
            DiveEvent::Moved,
            "outward wheel notch {notch} must stay inside"
        );
    }
    // The same slack spent by a held key: 1/60 s frames at KEY_RATE.
    let mut again = Universe::new(DEMO_SEED);
    for _ in 0..500 {
        if again.dive(Some(marker), WHEEL_FACTOR, DiveMode::Targeted) != DiveEvent::Moved {
            break;
        }
    }
    assert_eq!(again.level().get(), 2);
    assert_eq!(
        again.path.distance_to_center(),
        rest,
        "same journey, same rest"
    );
    let per_frame = KEY_RATE / 60.0;
    let frame = per_frame.exp();
    let mut frames = 0u32;
    while again.dive(None, frame, DiveMode::Targeted) == DiveEvent::Moved {
        frames += 1;
        assert!(frames < 10_000, "a held key must eventually close");
    }
    // Moved frames before the close: the log-distance from the rest to
    // the close distance, spent at KEY_RATE per second.
    let close_distance = 0.5 / CLOSE_ANGLE.sin();
    let expected = ((close_distance / rest).ln() / per_frame).floor();
    assert!(
        (f64::from(frames) - expected).abs() <= 1.0,
        "held-key slack {frames} frames must match the distance slack ({expected})"
    );
    // A deliberate long way out still closes at the entered marker
    // (#403 AC4).
    let mut closed = None;
    for _ in 0..500 {
        if let DiveEvent::Closed(opened) =
            universe.dive(None, 1.0 / WHEEL_FACTOR, DiveMode::Targeted)
        {
            closed = Some(opened);
            break;
        }
    }
    assert_eq!(closed.map(|o| o.marker), Some(marker));
    assert_eq!(universe.level(), Level::MIN);
}

#[test]
fn open_refuses_populations_but_opens_portals() {
    let mut universe = Universe::new(DEMO_SEED);
    let population = universe
        .open
        .points
        .iter()
        .position(|point| point.kind == MarkerKind::Population);
    let portal = universe
        .open
        .points
        .iter()
        .position(|point| point.kind == MarkerKind::Portal);
    if let Some(index) = population {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: test index into a budgeted cell, always fits u32"
        )]
        let marker = MarkerIndex(index as u32);
        assert!(!universe.open(marker), "population marker must not open");
        assert_eq!(universe.level(), Level::MIN, "failed open must not move");
    }
    if let Some(index) = portal {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: test index into a budgeted cell, always fits u32"
        )]
        let marker = MarkerIndex(index as u32);
        assert!(universe.open(marker), "portal marker must open");
        assert_eq!(universe.level().get(), 2);
    }
}

#[test]
fn opening_keeps_camera_outside_child_cell() {
    let mut universe = Universe::new(DEMO_SEED);
    let marker = universe.autopilot_target().expect("root has markers");
    let mut target = Some(marker);
    let mut event = DiveEvent::Moved;
    for _ in 0..500 {
        event = universe.dive(target, WHEEL_FACTOR, DiveMode::Targeted);
        if event != DiveEvent::Moved {
            break;
        }
    }
    assert_eq!(event, DiveEvent::Opened(marker));
    assert_eq!(universe.level().get(), 2);
    assert!(universe.path.distance_to_center() > 0.5);
    assert!(universe.parent.is_some());
    target = None;
    let mut closed = None;
    for _ in 0..500 {
        if let DiveEvent::Closed(opened) =
            universe.dive(target, 1.0 / WHEEL_FACTOR, DiveMode::Targeted)
        {
            closed = Some(opened);
            break;
        }
    }
    assert_eq!(
        closed.map(|o| o.marker),
        Some(marker),
        "close must re-target"
    );
    assert_eq!(universe.level(), Level::MIN);
}

#[test]
fn autopilot_stops_at_the_planet_surface() {
    let (steps, universe) = replay_autopilot(DEMO_SEED, 1.0 / 60.0, 600.0);
    assert_eq!(universe.level().get(), MAX_NAV_LEVEL);
    // #995 T8: L2..L8 open one by one, then the planet's gravity zone
    // opens the star close-up and the planet (L10) in one step.
    assert_eq!(steps.len(), usize::from(MAX_NAV_LEVEL - 2));
    for (index, step) in steps.iter().enumerate() {
        let want = if index + 2 >= 9 { 10 } else { index + 2 };
        assert_eq!(usize::from(step.level.get()), want);
        if want == 10 {
            continue;
        }
        assert!(
            step.preview_count >= 1,
            "target was not previewed before opening"
        );
        assert!(step.preview_count <= PREVIEW_CAP);
        assert!(step.alive <= STREAM_CAP, "working set over the cap");
        assert!(
            step.preview_error <= 1e-9,
            "preview drifted: {}",
            step.preview_error
        );
    }
    let (again, _) = replay_autopilot(DEMO_SEED, 1.0 / 60.0, 600.0);
    assert_eq!(steps, again, "journey must be the same every run");
    assert!(
        steps.last().expect("steps").elapsed < 120.0,
        "journey too long"
    );
}

#[test]
fn crossed_portals_walk_the_ray_in_order_without_populations() {
    use crate::r#gen::Point;
    let open = Generated {
        points: vec![
            Point::bare([0.7, 0.0, 0.0], 0.008, MarkerKind::Portal),
            Point::bare([0.6, 0.0, 0.0], 0.008, MarkerKind::Population),
            Point::bare([0.45, 0.0, 0.0], 0.008, MarkerKind::Portal),
            Point::bare([0.0, 0.4, 0.0], 0.008, MarkerKind::Portal),
        ],
        child_constraints: Vec::new(),
    };
    let order = crossed_portals([2.0, 0.0, 0.0], [0.5, 0.0, 0.0], &open, 0.05);
    assert_eq!(order, vec![0, 2], "ray order, populations excluded");
    assert!(
        crossed_portals([0.5, 0.0, 0.0], [0.5, 0.0, 0.0], &open, 0.05).is_empty(),
        "a still camera crosses nothing"
    );
}

#[test]
fn the_open_cell_closes_on_the_angular_rule_alone() {
    // Past the old horizon floor (4.0) but inside the angular close
    // distance a still camera stays in its cell (#403): no second
    // close currency fights the dive.
    let mut universe = Universe::new(DEMO_SEED);
    let marker = universe.autopilot_target().expect("marker");
    assert!(universe.open(MarkerIndex(marker)), "open L2 directly");
    universe.path.set_offset([4.5, 0.0, 0.0]);
    assert_eq!(
        universe.dive(None, 1.0, DiveMode::Passing),
        DiveEvent::Moved,
        "inside the angular close distance nothing closes"
    );
    assert_eq!(universe.level().get(), 2);
    universe
        .path
        .set_offset([0.5 / CLOSE_ANGLE.sin() * 1.01, 0.0, 0.0]);
    match universe.dive(None, 1.0, DiveMode::Passing) {
        DiveEvent::Closed(opened) => assert_eq!(opened.marker, marker),
        DiveEvent::Moved => panic!("the angular rule should have closed the cell"),
        DiveEvent::Opened(_) => panic!("nothing to open while backing out"),
    }
    assert_eq!(universe.level(), Level::MIN);
}

#[test]
fn passthrough_opens_the_crossed_portal_first() {
    // Forced crossing (#152): the camera starts on the far side of one
    // portal, aimed at another, so the segment crosses it exactly. The
    // crossed portal opens first while the stored target value is never
    // touched (it arrives by value and is only read).
    let mut universe = Universe::new(DEMO_SEED);
    let portals: Vec<(u32, [f64; 3])> = universe
        .open
        .points
        .iter()
        .enumerate()
        .filter(|(_, point)| point.kind == MarkerKind::Portal)
        .map(|(index, point)| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: marker index into a budgeted cell, always fits u32"
            )]
            let marker = index as u32;
            (marker, point.portal_position())
        })
        .collect();
    assert!(portals.len() >= 2, "root cell needs two portals");
    let (crossed, crossed_pos) = portals[0];
    let (target, target_pos) = portals[1];
    let mut axis = [
        target_pos[0] - crossed_pos[0],
        target_pos[1] - crossed_pos[1],
        target_pos[2] - crossed_pos[2],
    ];
    let length = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    assert!(length > 0.0, "portals must not coincide");
    axis = [axis[0] / length, axis[1] / length, axis[2] / length];
    universe.path.set_offset([
        crossed_pos[0] - axis[0] * 2.0,
        crossed_pos[1] - axis[1] * 2.0,
        crossed_pos[2] - axis[2] * 2.0,
    ]);
    let mut opened = None;
    for _ in 0..2000 {
        match universe.dive(Some(target), WHEEL_FACTOR, DiveMode::Passing) {
            DiveEvent::Opened(m) => {
                opened = Some(m);
                break;
            }
            DiveEvent::Closed(_) => panic!("backed out while approaching"),
            DiveEvent::Moved => {}
        }
    }
    assert_eq!(opened, Some(crossed), "the crossed portal opens first");
    assert_eq!(universe.level().get(), 2);
}

#[test]
fn root_cannot_rise_past_cap_or_close() {
    let mut universe = Universe::new(DEMO_SEED);
    for _ in 0..100 {
        assert_eq!(
            universe.dive(None, 1.0 / WHEEL_FACTOR, DiveMode::Targeted),
            DiveEvent::Moved
        );
    }
    assert!(universe.path.distance_to_center() <= ROOT_MAX_DISTANCE + 1e-9);
    assert_eq!(universe.level(), Level::MIN);
}

#[test]
fn siblings_land_one_over_ratio_away() {
    let entered = Opened {
        marker: 0,
        position: [0.1, 0.0, 0.0],
        ratio: 0.01,
        anonymous: false,
        up: crate::frame::IDENTITY_UP,
    };
    let (position, radius) = sibling_in_open_units(entered, ParentUnits([0.2, 0.0, 0.0]));
    assert!((position.0[0] - 10.0).abs() < 1e-9);
    assert_eq!(radius, 0.5);
    assert!(drawn_radius(1e-6, 10.0) > 1e-6);
    assert_eq!(drawn_radius(1.0, 10.0), 1.0);
}

#[test]
fn nothing_opens_below_l10() {
    // #770: region portals removed; L10 holds no portals at all.
    let (_, universe) = replay_autopilot(DEMO_SEED, 1.0 / 60.0, 600.0);
    assert_eq!(universe.level().get(), MAX_NAV_LEVEL);
    assert!(!universe.path.can_open(), "L10 markers never open");
    let portals: Vec<MarkerIndex> = universe
        .open
        .points
        .iter()
        .enumerate()
        .filter(|(_, point)| point.kind == MarkerKind::Portal)
        .filter_map(|(index, _)| u32::try_from(index).ok().map(MarkerIndex))
        .collect();
    assert!(portals.is_empty(), "#770: no region portals");
    assert_eq!(universe.level().get(), MAX_NAV_LEVEL);
}
