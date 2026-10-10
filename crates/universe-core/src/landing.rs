//! Window-only landing approach for surface portals (#430).
//!
//! Pure bend-to-land math shared by the window dive: far from a surface
//! portal the step is bit-identical to [`crate::nav::dive_step`], near it
//! the direction bends toward the portal's surface normal and ends on the
//! normal axis at the open distance. Replays ([`crate::nav::DiveMode::Targeted`])
//! never call this module, so the headless journey stays byte-identical
//! (`E-GOLDEN`, `E-REPLAY-CORE`). Everything here is pure and deterministic
//! in its inputs (`E-DET-TIERS`).

use crate::coords::{Level, OpenUnits};
use crate::frame::IDENTITY_UP;
use crate::r#gen::{Form, MarkerKind};
use crate::nav::{add3, div3, dot3, length3, mul3, sub3};
use crate::nest::OPEN_ANGLE;

/// How far out the bend starts, as a multiple of the landing distance.
///
/// The path is straight (bit-identical to `dive_step`) beyond this, so
/// aiming still feels direct and only the final approach bends.
pub const BEND_START_MULTIPLE: f64 = 4.0;

/// Surface normal for a landing approach at `level`, if `form`/`kind` mark
/// a surface portal (#430).
///
/// L10 region patches land along their patch normal; L11 city, L12 building
/// and L13 room portals land along the ground up (`+Y`, rooms approached
/// from above the roof). Anything else (galaxies, stars, planets from orbit,
/// populations) holds no normal and keeps the straight line.
#[must_use]
pub fn landing_normal_for(level: Level, kind: MarkerKind, form: Form) -> Option<[f64; 3]> {
    if kind != MarkerKind::Portal {
        return None;
    }
    match level.get() {
        10 => match form {
            Form::Patch { normal } => Some(normal),
            _ => None,
        },
        11..=13 => Some(IDENTITY_UP),
        _ => None,
    }
}

/// Landing distance from the portal centre in parent-cell units.
///
/// Exactly the open distance (`radius / sin(OPEN_ANGLE)`), so the bend ends
/// where [`crate::nav::should_open`] fires and entry rests at today's
/// distance (about 3.58 child units), now measured along the normal.
#[must_use]
pub fn landing_distance(radius: f64) -> f64 {
    radius / OPEN_ANGLE.sin()
}

/// Floor cap for a landing dive at its target portal (#430).
///
/// The landing point's height in clearance-floor units, a hair under the
/// open boundary: radial distance at L10, height at L11-L13. The rest
/// clears every portal, so the cap always sits above the surface itself:
/// capping the clearance floor with it keeps the dive reachable (the rest
/// can sit below the standoff at deep tail levels) without ever letting
/// the camera under the ground. The hair (1% under the rest) matters: the
/// open rule needs the distance strictly inside the boundary, and a cap
/// exactly on it would pin the dive there forever. Returns `None` when the
/// target holds no landing normal or a level outside L10-L13.
#[must_use]
pub fn landing_floor_cap(
    level: Level,
    portal: [f64; 3],
    normal: [f64; 3],
    radius: f64,
) -> Option<f64> {
    let rest = landing_distance(radius);
    if !rest.is_finite() || rest <= 0.0 {
        return None;
    }
    // Strictly inside the open boundary, so the crossing step fires it.
    let reach = rest * 0.99;
    match level.get() {
        10 => {
            let length = length3(normal);
            if !length.is_finite() || length <= 0.0 {
                return None;
            }
            let unit = div3(normal, length);
            let at = add3(portal, mul3(unit, reach));
            let cap = length3(at);
            if cap.is_finite() { Some(cap) } else { None }
        }
        11..=13 => {
            let cap = portal[1] + reach;
            if cap.is_finite() { Some(cap) } else { None }
        }
        _ => None,
    }
}

/// One landing step toward a portal sphere (#430).
///
/// `camera` steps toward `center` (radius `radius`) with `factor`, bending
/// toward `normal` when present. With no normal the result is bit-identical
/// to [`crate::nav::dive_step`]; above the bend start
/// (`BEND_START_MULTIPLE` landing distances out) it is also identical.
/// Below it the direction eases from the straight line to the normal axis
/// while the distance matches the straight step exactly, so the gap stays
/// monotone and the path ends on `center + normal * landing_distance`.
/// A camera on the centre steps along the normal.
#[must_use]
pub fn landing_step(
    camera: OpenUnits,
    center: OpenUnits,
    radius: f64,
    normal: Option<[f64; 3]>,
    factor: f64,
) -> OpenUnits {
    let straight = crate::nav::dive_step(camera, center, radius, factor);
    let Some(normal) = normal else {
        return straight;
    };
    if !normal.iter().all(|c| c.is_finite()) {
        return straight;
    }
    let normal_length = length3(normal);
    if !normal_length.is_finite() || normal_length <= 0.0 {
        return straight;
    }
    let unit = div3(normal, normal_length);
    let rest = landing_distance(radius);
    if !rest.is_finite() || rest <= 0.0 {
        return straight;
    }
    let start = rest * BEND_START_MULTIPLE;
    let to_straight = sub3(straight.0, center.0);
    let straight_distance = length3(to_straight);
    if !straight_distance.is_finite() || straight_distance <= 0.0 {
        return OpenUnits(add3(center.0, mul3(unit, rest)));
    }
    if straight_distance >= start {
        return straight;
    }
    let span = start - rest;
    if !span.is_finite() || span <= 0.0 {
        return straight;
    }
    let raw = 1.0 - (straight_distance - rest) / span;
    let t = raw.clamp(0.0, 1.0);
    let ease = t * t * (3.0 - 2.0 * t);
    if ease <= 0.0 {
        return straight;
    }
    let direction = div3(to_straight, straight_distance);
    let mixed = add3(mul3(direction, 1.0 - ease), mul3(unit, ease));
    let mixed_length = length3(mixed);
    let bent = if mixed_length.is_finite() && mixed_length > 1e-12 {
        div3(mixed, mixed_length)
    } else if ease >= 0.5 {
        unit
    } else {
        direction
    };
    let dot = dot3(direction, unit).clamp(-1.0, 1.0);
    if dot >= 1.0 - 1e-12 && ease >= 1.0 - 1e-12 {
        return OpenUnits(add3(center.0, mul3(unit, straight_distance)));
    }
    OpenUnits(add3(center.0, mul3(bent, straight_distance)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::Level;
    use crate::nav::{DEMO_SEED, DiveEvent, DiveMode, Universe, WHEEL_FACTOR};

    fn level(n: u8) -> Level {
        Level::new(n).expect("ladder level")
    }

    #[test]
    fn no_normal_matches_the_straight_step_exactly() {
        let camera = OpenUnits([1.2, 0.4, -0.6]);
        let center = OpenUnits([0.1, 0.0, 0.2]);
        for factor in [WHEEL_FACTOR, 1.0 / WHEEL_FACTOR, 0.5] {
            assert_eq!(
                landing_step(camera, center, 0.02, None, factor),
                crate::nav::dive_step(camera, center, 0.02, factor),
                "galaxies, stars and planets keep the straight line"
            );
        }
    }

    #[test]
    fn straight_above_the_bend_start() {
        let center = OpenUnits([0.0; 3]);
        let radius = 0.02;
        let rest = landing_distance(radius);
        let normal = Some([0.0, 1.0, 0.0]);
        let far = OpenUnits([0.0, 0.0, rest * (BEND_START_MULTIPLE + 2.0)]);
        assert_eq!(
            landing_step(far, center, radius, normal, WHEEL_FACTOR),
            crate::nav::dive_step(far, center, radius, WHEEL_FACTOR),
        );
    }

    #[test]
    fn bend_ends_on_the_normal_at_the_landing_distance() {
        let center = OpenUnits([0.05, -0.02, 0.03]);
        let radius = 0.02;
        let rest = landing_distance(radius);
        let normal = [0.0, 1.0, 0.0];
        let mut camera = OpenUnits([0.6, 0.5, 0.6]);
        let mut previous = length3(sub3(camera.0, center.0));
        for _ in 0..2000 {
            camera = landing_step(camera, center, radius, Some(normal), WHEEL_FACTOR);
            let gap = length3(sub3(camera.0, center.0));
            assert!(gap < previous + 1e-12, "gap stays monotone");
            previous = gap;
            if (gap - rest).abs() / rest < 0.02 {
                break;
            }
        }
        let offset = sub3(camera.0, center.0);
        let distance = length3(offset);
        assert!(
            (distance - rest).abs() / rest < 0.02,
            "ends at about 3.58 child units: {distance} vs {rest}"
        );
        let dot = dot3(div3(offset, distance), normal);
        assert!(dot > 0.999, "on the normal axis: dot {dot}");
    }

    #[test]
    fn same_inputs_give_the_same_path() {
        let camera = OpenUnits([0.8, 0.3, -0.4]);
        let center = OpenUnits([0.0; 3]);
        let normal = Some([0.2, 0.9, 0.1]);
        let first = landing_step(camera, center, 0.02, normal, WHEEL_FACTOR);
        let second = landing_step(camera, center, 0.02, normal, WHEEL_FACTOR);
        assert_eq!(first, second);
    }

    #[test]
    fn landing_normals_cover_only_surface_portals() {
        assert!(
            landing_normal_for(level(10), MarkerKind::Portal, Form::Patch { normal: [0.0, 1.0, 0.0] })
                .is_some()
        );
        assert!(landing_normal_for(level(10), MarkerKind::Portal, Form::Dot).is_none());
        assert!(landing_normal_for(level(9), MarkerKind::Portal, Form::Dot).is_none());
        for n in [11u8, 12, 13] {
            assert_eq!(
                landing_normal_for(level(n), MarkerKind::Portal, Form::Dot),
                Some(IDENTITY_UP),
                "L{n} ground portals land along +Y"
            );
        }
        assert!(landing_normal_for(level(11), MarkerKind::Population, Form::Dot).is_none());
        assert!(landing_normal_for(level(14), MarkerKind::Portal, Form::Dot).is_none());
    }

    #[test]
    fn window_landing_matches_targeted_off_the_tail() {
        // L1 dives aim at non-surface portals: Landing must fly the replay
        // line exactly.
        for seed in [DEMO_SEED, 7u64] {
            let mut targeted = Universe::new(seed);
            let mut landing = Universe::new(seed);
            let marker = targeted.autopilot_target().expect("root marker");
            for _ in 0..300 {
                let left = targeted.dive(Some(marker), WHEEL_FACTOR, DiveMode::Targeted);
                let right = landing.dive(Some(marker), WHEEL_FACTOR, DiveMode::Landing);
                assert_eq!(landing.path.offset(), targeted.path.offset());
                assert_eq!(right, left);
                if left != DiveEvent::Moved {
                    break;
                }
            }
        }
    }

    #[test]
    fn tail_landings_open_on_the_normal_at_rest() {
        use crate::nav::MarkerIndex;
        // Fine steps (about 1% per step) so the first crossing lands within
        // the ±2% band around the rest distance, like the window's key dive.
        let fine = 0.99;
        let mut journey = Universe::new(DEMO_SEED);
        while journey.level().get() < 10 {
            let marker = journey.autopilot_target().expect("journey marker");
            let mut opened = false;
            for _ in 0..2000 {
                match journey.dive(Some(marker), WHEEL_FACTOR, DiveMode::Targeted) {
                    DiveEvent::Opened(_) => {
                        opened = true;
                        break;
                    }
                    DiveEvent::Closed(_) => panic!("backed out"),
                    DiveEvent::Moved => {}
                }
            }
            assert!(opened);
        }
        for n in 10u8..=13 {
            assert_eq!(journey.level().get(), n);
            let radius = journey.marker_radius();
            let rest = landing_distance(radius);
            let target = journey.autopilot_target().expect("tail marker");
            let point = journey.open.points.get(target as usize).cloned().expect("portal");
            let normal = landing_normal_for(journey.level(), point.kind, point.form)
                .expect("tail portals land");
            let center = journey.marker(MarkerIndex(target)).expect("portal position");
            let mut camera = OpenUnits(journey.path.offset());
            for _ in 0..20000 {
                let next = landing_step(camera, OpenUnits(center), radius, Some(normal), fine);
                journey.path.set_offset(next.0);
                camera = next;
                let gap = length3(sub3(next.0, center));
                if crate::nav::should_open(radius, gap) {
                    break;
                }
            }
            let offset = sub3(camera.0, center);
            let distance = length3(offset);
            assert!(
                (distance - rest).abs() / rest < 0.02,
                "L{n} opens at about 3.58 child units: {distance} vs {rest}"
            );
            let dot = dot3(div3(offset, distance), div3(normal, length3(normal)));
            assert!(dot > 0.999, "L{n} opens on the normal: dot {dot}");
            assert!(journey.open(MarkerIndex(target)), "L{n} portal opens");
        }
    }

    #[test]
    fn landing_reaches_a_low_portal_below_the_standoff() {
        use crate::terrain::GROUND_STANDOFF;
        // The L12 rest sits below the standoff, so a low portal's rest is
        // under the floor: the landing cap must still let the dive open
        // (regression: the cap-less clamp held the camera above the rest
        // and froze the dive).
        let mut journey = Universe::new(DEMO_SEED);
        while journey.level().get() < 12 {
            let marker = journey.autopilot_target().expect("journey marker");
            let mut opened = false;
            for _ in 0..3000 {
                match journey.dive(Some(marker), WHEEL_FACTOR, DiveMode::Targeted) {
                    DiveEvent::Opened(_) => {
                        opened = true;
                        break;
                    }
                    DiveEvent::Closed(_) => panic!("backed out"),
                    DiveEvent::Moved => {}
                }
            }
            assert!(opened);
        }
        assert_eq!(journey.level().get(), 12);
        let radius = journey.marker_radius();
        let rest = landing_distance(radius);
        assert!(rest < GROUND_STANDOFF, "test premise: rest under the standoff");
        let target = journey.autopilot_target().expect("tail marker");
        let mut event = DiveEvent::Moved;
        for _ in 0..20000 {
            event = journey.dive(Some(target), 0.99, DiveMode::Landing);
            if event != DiveEvent::Moved {
                break;
            }
        }
        assert_eq!(event, DiveEvent::Opened(target), "the low dive must open");
        assert_eq!(journey.level().get(), 13);
    }

    #[test]
    fn replays_keep_their_straight_line() {
        let (steps, universe) = crate::nav::replay_autopilot(DEMO_SEED, 1.0 / 60.0, 600.0);
        assert_eq!(universe.level().get(), crate::nav::MAX_NAV_LEVEL);
        assert_eq!(steps.len(), usize::from(crate::nav::MAX_NAV_LEVEL - 1));
        let (leg, _) = crate::flight::replay_free_leg(DEMO_SEED, 1.0 / 60.0, 600.0);
        assert_eq!(leg.points.len(), 300);
    }
}
