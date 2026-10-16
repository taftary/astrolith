//! Tail dive integration tests: landing, clearance, and preview context
//! through the public journey API (#430).
//!
//! Split from `nav.rs` unit tests under `E-FILE-SIZE` (`E-TEST-PLACE`:
//! journey tests through the public API live in this crate's `tests/`).
//! Integration targets compile without `cfg(test)`, so unlike the unit
//! tests these use `assert!` plus `if let` instead of `expect` (the
//! `clippy.toml` test allowances only cover `cfg(test)` code, `E-NO-PANIC`).

use universe_core::r#gen::{Form, MarkerKind};
use universe_core::ground::{GROUND_STANDOFF, ground_height};
use universe_core::nav::{
    DEMO_SEED, DiveEvent, DiveMode, MarkerIndex, Universe, WHEEL_FACTOR, preview_surface_context,
};
use universe_core::terrain::{surface_floor_radius, surface_radius};

/// Euclidean length of `v`.
fn length3(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// `v` scaled to unit length (`v` is never zero at the call sites).
fn unit3(v: [f64; 3]) -> [f64; 3] {
    let length = length3(v);
    [v[0] / length, v[1] / length, v[2] / length]
}

/// Dives the fixed journey to `level` with the replay mode, or `None` when
/// the dive backs out or a level offers no marker.
fn dive_journey_to(level: u8) -> Option<Universe> {
    let mut journey = Universe::new(DEMO_SEED);
    while journey.level().get() < level {
        let marker = journey.autopilot_target()?;
        let mut opened = false;
        for _ in 0..3000 {
            match journey.dive(Some(marker), WHEEL_FACTOR, DiveMode::Targeted) {
                DiveEvent::Opened(_) => {
                    opened = true;
                    break;
                }
                DiveEvent::Closed(_) => return None,
                DiveEvent::Moved => {}
            }
        }
        if !opened {
            return None;
        }
    }
    Some(journey)
}

#[test]
fn preview_context_previews_children_but_nothing_opens() {
    // Dive the journey to L10: the analytic preview still describes the
    // children, but no child opens and none shows a context.
    let journey = dive_journey_to(10);
    assert!(journey.is_some(), "journey must dive to L10");
    if let Some(mut journey) = journey {
        assert!(
            preview_surface_context(&journey).is_some(),
            "L10 previews children"
        );
        let target = journey.autopilot_target();
        assert!(target.is_some(), "L10 offers markers");
        if let Some(target) = target {
            assert!(!journey.open(MarkerIndex(target)), "no child opens");
        }
        assert!(
            journey.surface_context().is_none(),
            "nothing shows a context"
        );
        assert_eq!(journey.level().get(), 10);
        let root = Universe::new(DEMO_SEED);
        assert!(
            preview_surface_context(&root).is_none(),
            "L1 previews no context"
        );
    }
}

#[test]
fn landing_dive_never_cuts_the_planet_but_targeted_passes_through() {
    // Dive the journey to L10, then aim across the planet: the camera
    // starts antipodal to a region portal so the straight line cuts the
    // body.
    let journey = dive_journey_to(10);
    assert!(journey.is_some(), "journey must dive to L10");
    if let Some(journey) = journey {
        let mut portal: Option<(u32, [f64; 3], [f64; 3])> = None;
        for (index, point) in journey.open.points.iter().enumerate() {
            if point.kind != MarkerKind::Portal {
                continue;
            }
            if let Form::Patch { normal } = point.form {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "E-CAST: test index into a budgeted cell, always fits u32"
                )]
                let marker = index as u32;
                portal = Some((marker, normal, point.portal_position()));
                break;
            }
        }
        assert!(portal.is_some(), "L10 holds region portals");
        if let Some((marker, normal, at)) = portal {
            let distance = length3(at);
            let axis = unit3(at);
            for (got, want) in axis.iter().zip(normal) {
                assert!((got - want).abs() < 1e-12);
            }
            // Antipodal start, outside the surface on the far side.
            let start = [
                -axis[0] * (distance + 1.0),
                -axis[1] * (distance + 1.0),
                -axis[2] * (distance + 1.0),
            ];
            let mut cutting = journey.clone();
            cutting.path.set_offset(start);
            let mut lowest_landing = f64::INFINITY;
            for _ in 0..2000 {
                cutting.dive(Some(marker), WHEEL_FACTOR, DiveMode::Landing);
                let offset = cutting.path.offset();
                let reach = length3(offset);
                lowest_landing = lowest_landing.min(reach);
                if cutting.level().get() == 11 {
                    break;
                }
            }
            let floor = surface_floor_radius().min(surface_radius(start));
            assert!(
                lowest_landing >= floor + GROUND_STANDOFF - 1e-9,
                "landing dive cut to {lowest_landing}, floor near {floor}"
            );
            // The same line with the replay mode passes through the body, so
            // headless replays are provably unbothered by the clamp.
            let mut straight = journey.clone();
            straight.path.set_offset(start);
            let mut lowest_targeted = f64::INFINITY;
            for _ in 0..2000 {
                straight.dive(Some(marker), WHEEL_FACTOR, DiveMode::Targeted);
                lowest_targeted = lowest_targeted.min(length3(straight.path.offset()));
                if straight.level().get() == 11 {
                    break;
                }
            }
            assert!(
                lowest_targeted < floor,
                "targeted must pass through the body ({lowest_targeted} vs {floor})"
            );
        }
    }
}

#[test]
fn scrolling_out_near_the_planet_ground_stays_above() {
    let journey = dive_journey_to(10);
    assert!(journey.is_some(), "journey must dive to L10");
    if let Some(mut journey) = journey {
        // Park just above the surface, then scroll out with no lock: the clamp
        // holds the camera over the ground, across the close into L9.
        let parked = [
            0.3,
            ground_height(journey.frame.as_ref().expect("surface frame"), 0.3, 0.1)
                + GROUND_STANDOFF
                + 0.01,
            0.1,
        ];
        journey.path.set_offset(parked);
        for _ in 0..200 {
            journey.dive(None, 1.0 / WHEEL_FACTOR, DiveMode::Landing);
            let offset = journey.path.offset();
            if journey.level().get() == 10 {
                let reach =
                    (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
                let floor = surface_radius(offset) + GROUND_STANDOFF;
                assert!(
                    reach >= floor - 1e-9,
                    "L10 close sank to {reach} under {floor}"
                );
            }
            if journey.level().get() != 10 {
                break;
            }
        }
    }
}
