//! The `ORBIT` and `CLOCK` verify lines (#981 T3): the sky keeps its
//! periods and the planet spins under its sun.
//!
//! `ORBIT` turns the journey's planet a quarter period around its star and
//! its first moon half a period around the planet, and reports the angles.
//! `CLOCK` reads the start time, the real seconds of one Earth day at 1x,
//! and how far the sun turns about the spin axis in a quarter of the planet's
//! day while the camera keeps its place on the ground.
//!
//! Deterministic (`E-DET-TIERS`), std only (`E-CORE-NO-BEVY`).

use crate::clock::{Clock, DAY_S, readout};
use crate::frame::turn_about;
use crate::nav::{DEMO_SEED, MarkerIndex, Universe, length3, sub3};

/// Angle limit for a period's share, degrees.
const ANGLE_LIMIT_DEG: f64 = 1e-6;

fn angle_deg(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (la, lb) = (length3(a), length3(b));
    if la <= 0.0 || lb <= 0.0 {
        return 0.0;
    }
    let cos = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]) / (la * lb);
    cos.clamp(-1.0, 1.0).acos().to_degrees()
}

/// The demo journey at its planet: the system (L8) and the planet's
/// marker.
fn system() -> Option<(Universe, u32)> {
    let mut universe = Universe::new(DEMO_SEED);
    for &marker in crate::nest::autopilot_path(DEMO_SEED).get(..7)? {
        if !universe.open(MarkerIndex(marker)) {
            return None;
        }
    }
    let planet = universe.journey_target()?;
    Some((universe, planet))
}

/// The planet opened from just above its centre.
fn on_planet(mut universe: Universe, planet: u32) -> Option<Universe> {
    let at = universe.marker(MarkerIndex(planet))?;
    universe.path.set_offset([at[0], at[1] + 1e-6, at[2]]);
    universe.open_planet(planet).then_some(universe)
}

/// Quarter-period turn of the planet and half-period turn of its first
/// moon, degrees.
fn orbit_angles() -> Option<(f64, f64)> {
    let (mut universe, planet) = system()?;
    let period = universe
        .open
        .points
        .get(planet as usize)?
        .planet?
        .period_days;
    let start = universe.marker(MarkerIndex(planet))?;
    universe.advance(period * DAY_S / 4.0);
    let quarter = angle_deg(start, universe.marker(MarkerIndex(planet))?);
    let mut moons = on_planet(system()?.0, planet)?;
    let (index, moon) = moons
        .open
        .points
        .iter()
        .enumerate()
        .find_map(|(i, p)| p.moon.map(|m| (i, m)))?;
    let first = moons.open.points.get(index)?.position;
    moons.advance(moon.period_days * DAY_S / 2.0);
    // The planet's cell turns with its ground: undo the spin.
    let entry = moons.path.entered()?;
    let back = turn_about(
        moons.open.points.get(index)?.position,
        entry.axis,
        entry.spin,
    );
    let half = angle_deg(first, back);
    Some((quarter, half))
}

/// Appends the `ORBIT` line; `true` when both turns match their periods.
pub fn verify_orbit(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let Some((quarter, half)) = orbit_angles() else {
        let _ = writeln!(out, "ORBIT the journey's planet or moon is missing FAIL");
        return false;
    };
    let passed = (quarter - 90.0).abs() < ANGLE_LIMIT_DEG && (half - 180.0).abs() < 1e-3;
    let _ = writeln!(
        out,
        "ORBIT planet_quarter_deg={quarter:.6} moon_half_deg={half:.3} {}",
        if passed { "PASS" } else { "FAIL" }
    );
    passed
}

/// What the clock measured.
struct Spin {
    /// Real seconds of one Earth day at 1x.
    day_real_s: f64,
    /// The sun's turn about the pole in a quarter of the planet's day.
    sun_turn_deg: f64,
    /// Camera place change on the ground meanwhile, metres.
    drift_m: f64,
}

fn spin() -> Option<Spin> {
    let mut clock = Clock::default();
    let mut ticks = 0.0;
    while clock.game_s < DAY_S {
        clock.tick(1.0);
        ticks += 1.0;
    }
    let (universe, planet) = system()?;
    let mut universe = on_planet(universe, planet)?;
    let hours = universe.spin_hours?;
    let before = universe.path.offset();
    let sun = crate::light::sun_direction(&universe)?;
    universe.advance(hours * 3_600.0 / 4.0);
    let now = crate::light::sun_direction(&universe)?;
    // Across the spin axis only.
    let axis = universe.spin_axis;
    let flat = |v: [f64; 3]| {
        let along = v[0] * axis[0] + v[1] * axis[1] + v[2] * axis[2];
        sub3(v, [axis[0] * along, axis[1] * along, axis[2] * along])
    };
    let drift_m = length3(sub3(universe.path.offset(), before))
        * crate::nav::planet_frame(&universe)?.metres_per_cell();
    Some(Spin {
        day_real_s: ticks,
        sun_turn_deg: angle_deg(flat(sun), flat(now)),
        drift_m,
    })
}

/// Appends the `CLOCK` line; `true` when the start, the day and the spin
/// hold.
pub fn verify_clock(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let start = readout(0.0, 24.0);
    let Some(spin) = spin() else {
        let _ = writeln!(out, "CLOCK the journey's planet is missing FAIL");
        return false;
    };
    // The planet also moves a little on its orbit in a quarter day, so the
    // sun turns a little less than a quarter turn.
    let passed = start == "Day 1, 08:00"
        && spin.day_real_s == 600.0
        && spin.sun_turn_deg < 90.0
        && spin.sun_turn_deg > 89.0
        && spin.drift_m < 1e-3;
    let _ = writeln!(
        out,
        "CLOCK start=\"{start}\" day_real_s={:.0} sun_quarter_day_deg={:.2} drift_mm={:.3} {}",
        spin.day_real_s,
        spin.sun_turn_deg,
        spin.drift_m * 1e3,
        if passed { "PASS" } else { "FAIL" }
    );
    passed
}
