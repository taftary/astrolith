//! The `HANDOVER` verify line (#995 T8): what the gravity-zone open and
//! close cost on the journey.
//!
//! The journey flies to its planet's morning-side waypoint, then in; the
//! planet opens from its gravity zone as one exact origin shift (star
//! close-up and planet together). The line measures, in metres, how far the
//! camera's place moves across the open and across a close just past the
//! zone's band, the turn of the direction to the planet, the change of the
//! distance to its surface, and the sun's height at the arrival.
//!
//! Deterministic (`E-DET-TIERS`), std only (`E-CORE-NO-BEVY`).

use crate::nav::{
    AUTOPILOT_RATE, DEMO_SEED, DiveEvent, DiveMode, Universe, ZONE_LEAVE_FACTOR, l8_planet_zone,
    length3, sub3,
};
use crate::orbits::metres_per_cell;

/// Most dive steps the journey may take to reach the planet.
const MAX_STEPS: usize = 40_000;

/// Position jump limit across an open or close (Spec AC4: no visible jump).
const JUMP_LIMIT_M: f64 = 1e-3;

/// Direction turn limit across an open or close, degrees.
const TURN_LIMIT_DEG: f64 = 0.01;

/// The camera's place in L8 units, closing every level below L8 on a copy.
fn in_system(universe: &Universe) -> Option<[f64; 3]> {
    let mut path = universe.path.clone();
    while path.level().get() > 8 {
        path.close()?;
    }
    (path.level().get() == 8).then(|| path.offset())
}

fn angle_deg(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (la, lb) = (length3(a), length3(b));
    if la <= 0.0 || lb <= 0.0 {
        return 0.0;
    }
    let cos = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]) / (la * lb);
    cos.clamp(-1.0, 1.0).acos().to_degrees()
}

/// What the handovers measured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Handover {
    /// L8 marker of the journey's planet.
    pub planet: u32,
    /// Camera place change across the open, metres.
    pub open_jump_m: f64,
    /// Camera place change across the close, metres.
    pub close_jump_m: f64,
    /// Turn of the direction to the planet across the open, degrees.
    pub turn_deg: f64,
    /// Relative change of the distance to the surface across the open.
    pub speed_change: f64,
    /// The sun's height above the horizon at the zone arrival, degrees.
    pub arrival_sun_deg: f64,
}

/// Flies the journey to its planet and measures both handovers.
#[must_use]
pub fn measure() -> Option<Handover> {
    let mut universe = Universe::new(DEMO_SEED);
    let factor = (-AUTOPILOT_RATE / 60.0).exp();
    let mut target = None;
    for _ in 0..MAX_STEPS {
        if universe.level().get() == 8 {
            break;
        }
        if target.is_none() {
            target = universe.journey_target();
        }
        match universe.dive(target, factor, DiveMode::Journey) {
            DiveEvent::Opened(_) => target = None,
            DiveEvent::Closed(opened) => target = Some(opened.marker),
            DiveEvent::Moved => {}
        }
    }
    let planet = universe.journey_target()?;
    let zone = l8_planet_zone(&universe, planet)?;
    let mut steps = 0;
    while universe.level().get() == 8 && steps < MAX_STEPS {
        steps += 1;
        let before = universe.clone();
        if let DiveEvent::Opened(_) = universe.dive(Some(planet), factor, DiveMode::Journey) {
            // The open itself: from the system at the place the step reached,
            // open the planet again on a copy and read the place back.
            let next = in_system(&universe)?;
            let mut again = before.clone();
            again.path.set_offset(next);
            if !again.open_planet(planet) {
                return None;
            }
            let at = again.path.offset();
            let centre = before.marker(crate::nav::MarkerIndex(planet))?;
            let mpc8 = metres_per_cell(8);
            let frame = crate::nav::planet_frame(&again)?;
            let open_jump_m = length3(sub3(in_system(&again)?, next)) * mpc8;
            let radius = frame.radius_m();
            let before_gap = length3(sub3(next, centre)) * mpc8 - radius;
            let after_gap = length3(at) * frame.metres_per_cell() - radius;
            let turn_deg = angle_deg(sub3(next, centre), at);
            let sun = crate::light::sun_direction(&again)?;
            let arrival_sun_deg = 90.0 - angle_deg(sun, at);
            // The close: just past the band, read before and after.
            let mut out = again.clone();
            let leave = zone * ZONE_LEAVE_FACTOR * mpc8 / frame.metres_per_cell() * 1.001;
            let d = length3(at);
            out.path
                .set_offset([at[0] / d * leave, at[1] / d * leave, at[2] / d * leave]);
            let expected = in_system(&out)?;
            let closed = matches!(out.dive(None, 1.0, DiveMode::Passing), DiveEvent::Closed(_));
            let close_jump_m = if closed && out.level().get() == 8 {
                length3(sub3(out.path.offset(), expected)) * mpc8
            } else {
                f64::INFINITY
            };
            return Some(Handover {
                planet,
                open_jump_m,
                close_jump_m,
                turn_deg,
                speed_change: (after_gap / before_gap - 1.0).abs(),
                arrival_sun_deg,
            });
        }
    }
    None
}

/// Appends the `HANDOVER` line; `true` when every measure is in its limit.
pub fn verify_handover(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let Some(h) = measure() else {
        let _ = writeln!(
            out,
            "HANDOVER journey never reached the planet's gravity zone FAIL"
        );
        return false;
    };
    let passed = h.open_jump_m < JUMP_LIMIT_M
        && h.close_jump_m < JUMP_LIMIT_M
        && h.turn_deg < TURN_LIMIT_DEG
        && h.speed_change < 1e-6
        && h.arrival_sun_deg >= 30.0;
    let _ = writeln!(
        out,
        "HANDOVER planet={} open_jump_mm={:.3} close_jump_mm={:.3} turn_deg={:.1e} surface_gap_change={:.1e} arrival_sun_deg={:.1} {}",
        h.planet,
        h.open_jump_m * 1e3,
        h.close_jump_m * 1e3,
        h.turn_deg,
        h.speed_change,
        h.arrival_sun_deg,
        if passed { "PASS" } else { "FAIL" }
    );
    passed
}
