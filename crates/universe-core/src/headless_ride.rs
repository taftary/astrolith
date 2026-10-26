//! The `RIDE` verify line (#981 T2): what the moving sky costs the camera.
//!
//! Inside the journey's planet, a month of game time carries the planet
//! along its orbit while the camera keeps its place on the planet; in the
//! planetary system, the planet's gravity zone sweeps over a still camera
//! and opens the planet as one exact origin shift. The line reports how far
//! the planet travelled, how far the camera drifted on the planet, and the
//! camera's jump across the zone open.
//!
//! Deterministic (`E-DET-TIERS`), std only (`E-CORE-NO-BEVY`).

use crate::clock::DAY_S;
use crate::nav::{DEMO_SEED, DiveEvent, MarkerIndex, Universe, l8_planet_zone, length3, sub3};
use crate::orbits::metres_per_cell;

/// Drift and jump limit, metres (Spec: no visible jump).
const LIMIT_M: f64 = 1e-3;

/// Steps the zone may take to sweep over the waiting camera.
const SWEEP_STEPS: u32 = 200;

/// The camera's place in L8 units, closing every level below L8 on a copy.
fn in_system(universe: &Universe) -> Option<[f64; 3]> {
    let mut path = universe.path.clone();
    while path.level().get() > 8 {
        path.close()?;
    }
    (path.level().get() == 8).then(|| path.offset())
}

/// What the moving sky measured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ride {
    /// L8 marker of the journey's planet.
    pub planet: u32,
    /// How far the planet carried the camera through the system, km.
    pub travelled_km: f64,
    /// Camera place change on the planet over the month, metres.
    pub drift_m: f64,
    /// Camera place change across the zone open, metres.
    pub open_jump_m: f64,
}

/// The demo journey opened down to the planetary system (L8).
fn system() -> Option<Universe> {
    let mut universe = Universe::new(DEMO_SEED);
    for &marker in crate::nest::autopilot_path(DEMO_SEED).get(..7)? {
        if !universe.open(MarkerIndex(marker)) {
            return None;
        }
    }
    Some(universe)
}

/// Rides a month on the journey's planet, then lets its zone reach a still
/// camera.
#[must_use]
pub fn measure() -> Option<Ride> {
    let start = system()?;
    let planet = start.journey_target()?;
    let period = start.open.points.get(planet as usize)?.planet?.period_days;
    let mpc8 = metres_per_cell(8);
    // The ride: a camera just above the planet's centre in its cell.
    let mut riding = start.clone();
    let at = riding.marker(MarkerIndex(planet))?;
    riding.path.set_offset([at[0], at[1] + 1e-6, at[2]]);
    if !riding.open_planet(planet) {
        return None;
    }
    let (on_planet, place) = (riding.path.offset(), in_system(&riding)?);
    riding.advance(30.0 * DAY_S);
    let drift_m = length3(sub3(riding.path.offset(), on_planet))
        * crate::nav::planet_frame(&riding)?.metres_per_cell();
    let travelled_km = length3(sub3(in_system(&riding)?, place)) * mpc8 / 1e3;
    // The sweep: the camera waits on the orbit a few zones ahead.
    let mut waiting = start;
    let zone = l8_planet_zone(&waiting, planet)?;
    let r = at[0].hypot(at[2]);
    let ahead = at[2].atan2(at[0]) + 3.0 * zone / r;
    let wait = [r * ahead.cos(), 0.0, r * ahead.sin()];
    waiting.path.set_offset(wait);
    let step = period * DAY_S * (0.1 * zone / r) / std::f64::consts::TAU;
    for n in 1..SWEEP_STEPS {
        waiting.advance(f64::from(n) * step);
        if let Some(DiveEvent::Opened(_)) = waiting.sync_zones() {
            return Some(Ride {
                planet,
                travelled_km,
                drift_m,
                open_jump_m: length3(sub3(in_system(&waiting)?, wait)) * mpc8,
            });
        }
    }
    None
}

/// Appends the `RIDE` line; `true` when every measure is in its limit.
pub fn verify_ride(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let Some(ride) = measure() else {
        let _ = writeln!(out, "RIDE the planet's zone never reached the camera FAIL");
        return false;
    };
    let passed = ride.travelled_km > 1.0 && ride.drift_m < LIMIT_M && ride.open_jump_m < LIMIT_M;
    let _ = writeln!(
        out,
        "RIDE planet={} travelled_km={:.0} drift_mm={:.3} zone_open_jump_mm={:.3} {}",
        ride.planet,
        ride.travelled_km,
        ride.drift_m * 1e3,
        ride.open_jump_m * 1e3,
        if passed { "PASS" } else { "FAIL" }
    );
    passed
}
