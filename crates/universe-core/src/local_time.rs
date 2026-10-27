//! The local hour where you stand (#981 correction round 2, Q3 A).
//!
//! On a planet the readout shows the hour of the spot under the viewer, so
//! the clock matches the sky: noon when the sun crosses the viewer's
//! meridian, sunrise around 06:00. The hour comes from the sun's hour
//! angle about the planet's spin axis; the day count follows local
//! midnights, starting from the home calendar ([`crate::clock::readout`]),
//! so it never jumps by more than the hour difference. Away from planets
//! the home calendar shows. Pure and allocation-free apart from the
//! string (`E-CORE-NO-BEVY`, `E-TRANSCENDENTAL`).

use std::f64::consts::TAU;

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// `v` with its part along unit `axis` removed.
fn flat(v: [f64; 3], axis: [f64; 3]) -> [f64; 3] {
    let along = dot(v, axis);
    [
        v[0] - axis[0] * along,
        v[1] - axis[1] * along,
        v[2] - axis[2] * along,
    ]
}

/// The local solar time of day in seconds, `0..day_s`, for a viewer whose
/// up is `up`, with the sun toward `sun` and the planet spinning about
/// `axis` (right-handed, so the ground turns east and the sun moves west).
/// 12:00 when the sun is on the viewer's meridian. `None` at the poles or
/// for degenerate input.
#[must_use]
pub fn local_time_of_day_s(sun: [f64; 3], up: [f64; 3], axis: [f64; 3], day_s: f64) -> Option<f64> {
    let norm = dot(axis, axis).sqrt();
    if !(norm.is_finite() && norm > 0.0 && day_s.is_finite() && day_s > 0.0) {
        return None;
    }
    let axis = [axis[0] / norm, axis[1] / norm, axis[2] / norm];
    let (s, u) = (flat(sun, axis), flat(up, axis));
    if dot(s, s) < 1e-18 || dot(u, u) < 1e-18 {
        return None;
    }
    // The angle the ground has turned past the sun, in the spin's sense.
    let turned = dot(cross(s, u), axis).atan2(dot(s, u));
    let tod = (0.5 + turned / TAU).rem_euclid(1.0) * day_s;
    tod.is_finite().then_some(tod)
}

/// `"Day N, HH:MM"` for the local time of day `tod_s` at home game time
/// `game_s` on a day of `day_s` seconds: the day is the home calendar's
/// day, moved to the nearest local midnight.
#[must_use]
pub fn local_readout(game_s: f64, tod_s: f64, day_s: f64) -> String {
    let since = crate::clock::START_HOUR * 3_600.0 + game_s.max(0.0);
    let home_tod = since.rem_euclid(day_s);
    let shift = (tod_s - home_tod + day_s / 2.0).rem_euclid(day_s) - day_s / 2.0;
    let local = (since + shift).max(tod_s.rem_euclid(day_s));
    let day = (local / day_s).floor();
    let minutes = ((local - day * day_s) / 60.0).floor();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "E-CAST: whole days and minutes of a non-negative clock, intended"
    )]
    let (day, hours, minutes) = (
        day as u64 + 1,
        (minutes / 60.0).floor() as u64,
        (minutes % 60.0) as u64,
    );
    format!("Day {day}, {hours:02}:{minutes:02}")
}

/// The readout at the viewer at `camera` (open-cell position; the L10 cell
/// is planet-centred): the local hour on a planet, else `None` (the caller
/// shows the home calendar).
#[must_use]
pub fn readout_at(universe: &crate::nav::Universe, camera: [f64; 3]) -> Option<String> {
    let sun = crate::light::sun_direction(universe)?;
    let day_s = universe.spin_hours.filter(|h| h.is_finite() && *h > 0.0)? * 3_600.0;
    let tod = local_time_of_day_s(sun, camera, universe.spin_axis, day_s)?;
    Some(local_readout(universe.time_s(), tod, day_s))
}

#[cfg(test)]
#[path = "local_time_tests.rs"]
mod tests;
