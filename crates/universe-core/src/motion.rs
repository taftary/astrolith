//! Gradual velocity and heading response for the player body (#756).
//!
//! Pure helpers (`E-CORE-NO-BEVY`, `E-DET-TIERS`): [`approach`] moves a
//! velocity toward its wish by at most `rate` per second, so starts, stops
//! and turns bend instead of snapping; [`turn_toward`] turns a heading toward
//! its target at a capped rate. [`body_step`](crate::body::body_step) calls
//! both; with inertia off it keeps today's instant assignments, byte for
//! byte.

/// Walking reaches its speed in about this long, seconds (#756 C1).
pub const WALK_UP_S: f64 = 0.3;

/// Walking stops in about this long, seconds (#756 C1).
pub const WALK_DOWN_S: f64 = 0.25;

/// Running reaches and leaves its speed in about this long, seconds (#756 C1).
pub const RUN_S: f64 = 0.5;

/// Flying and boosting reach and leave their speed in about this long, seconds
/// (#756 C1).
pub const FLY_S: f64 = 1.0;

/// Third-person body turn: a half turn in about this long, seconds (#756 C2).
pub const BODY_HALF_TURN_S: f64 = 0.33;

/// Body turn rate, radians per second: half a turn in [`BODY_HALF_TURN_S`].
pub const BODY_TURN_RATE: f64 = std::f64::consts::PI / BODY_HALF_TURN_S;

/// Moves `current` toward `wish` by at most `rate * dt` (cells/s).
///
/// A turn is a change of wish, so the path bends instead of snapping; a zero
/// wish brakes to a stop. Holds `current` for a non-positive or non-finite
/// `dt` or a negative or non-finite `rate`, holds `current` for a degenerate
/// `wish`, and adopts `wish` for a degenerate `current` (like
/// [`crate::flight::ease_heading`]). Pure.
#[must_use]
pub fn approach(current: [f64; 3], wish: [f64; 3], rate: f64, dt: f64) -> [f64; 3] {
    if !dt.is_finite() || dt <= 0.0 || !rate.is_finite() || rate < 0.0 {
        return current;
    }
    if !wish.iter().all(|c| c.is_finite()) {
        return current;
    }
    if !current.iter().all(|c| c.is_finite()) {
        return wish;
    }
    let diff = [
        wish[0] - current[0],
        wish[1] - current[1],
        wish[2] - current[2],
    ];
    let distance = (diff[0] * diff[0] + diff[1] * diff[1] + diff[2] * diff[2]).sqrt();
    let step = rate * dt;
    if !(step.is_finite() && distance.is_finite()) || distance <= step {
        return wish;
    }
    if distance <= 0.0 {
        return wish;
    }
    let share = step / distance;
    [
        current[0] + diff[0] * share,
        current[1] + diff[1] * share,
        current[2] + diff[2] * share,
    ]
}

/// Response rate (cells/s squared) reaching `top` cells/s in `time_s` seconds.
///
/// `top` is the mode's pace already scaled into cell units (top m/s times the
/// level metre times the speed step); the same time holds at every speed step.
/// Returns `0.0` for a non-positive or non-finite time. Pure.
#[must_use]
pub fn inertia_rate(top: f64, time_s: f64) -> f64 {
    if !top.is_finite() || !time_s.is_finite() || time_s <= 0.0 {
        return 0.0;
    }
    top / time_s
}

/// Turns `yaw` toward `target` by at most `max_rate * dt` radians, along the
/// shorter arc.
///
/// Holds `yaw` for a non-finite `target`, a negative or non-finite rate, or a
/// non-positive or non-finite `dt`, and adopts a finite `target` for a
/// non-finite `yaw`. Pure.
#[must_use]
pub fn turn_toward(yaw: f64, target: f64, max_rate: f64, dt: f64) -> f64 {
    if !yaw.is_finite() {
        return target;
    }
    if !target.is_finite() || !max_rate.is_finite() || max_rate < 0.0 {
        return yaw;
    }
    if !dt.is_finite() || dt <= 0.0 {
        return yaw;
    }
    let two_pi = 2.0 * std::f64::consts::PI;
    let mut diff = (target - yaw) % two_pi;
    if diff > std::f64::consts::PI {
        diff -= two_pi;
    } else if diff < -std::f64::consts::PI {
        diff += two_pi;
    }
    let step = max_rate * dt;
    if diff.abs() <= step {
        target
    } else {
        yaw + diff.signum() * step
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approach_reaches_the_wish_within_rate_times_distance() {
        let current = [0.0, 0.0, 0.0];
        let wish = [1.4, 0.0, 0.0];
        // Rate 1.4 / 0.3: full speed in about 0.3 s.
        let rate = inertia_rate(1.4, WALK_UP_S);
        let mut velocity = current;
        for _ in 0..18 {
            velocity = approach(velocity, wish, rate, 1.0 / 60.0);
        }
        assert!(
            (velocity[0] - 1.4).abs() < 1.4 * 0.05,
            "about full speed after 0.3 s: {velocity:?}"
        );
        // Braking at 1.4 / 0.25: stopped in about 0.25 s.
        let brake = inertia_rate(1.4, WALK_DOWN_S);
        for _ in 0..15 {
            velocity = approach(velocity, [0.0; 3], brake, 1.0 / 60.0);
        }
        assert!(
            velocity[0].abs() < 1.4 * 0.05,
            "about stopped after 0.25 s: {velocity:?}"
        );
    }

    #[test]
    fn approach_bends_turns_instead_of_snapping() {
        let rate = inertia_rate(1.4, WALK_UP_S);
        let full = [0.0, 0.0, -1.4];
        // Full speed ahead, then a right turn: one step bends, not snaps.
        let bent = approach(full, [1.4, 0.0, 0.0], rate, 1.0 / 60.0);
        assert!(bent[0] > 0.0, "turns toward the new wish: {bent:?}");
        assert!(bent[2] < 0.0, "keeps momentum through the turn: {bent:?}");
    }

    #[test]
    fn approach_holds_on_bad_inputs() {
        let current = [1.0, 2.0, 3.0];
        let wish = [4.0, 5.0, 6.0];
        assert_eq!(approach(current, wish, 1.0, 0.0), current);
        assert_eq!(approach(current, wish, 1.0, f64::NAN), current);
        assert_eq!(approach(current, wish, -1.0, 1.0), current);
        assert_eq!(approach(current, [f64::NAN, 0.0, 0.0], 1.0, 1.0), current);
        assert_eq!(
            approach([f64::NAN, 0.0, 0.0], wish, 1.0, 1.0),
            wish,
            "a lost velocity adopts the wish"
        );
        // Zero distance arrives exactly.
        assert_eq!(approach(wish, wish, 1.0, 1.0), wish);
    }

    #[test]
    fn turn_toward_caps_the_rate_and_takes_the_shorter_arc() {
        // Half a turn at the body rate takes about 0.33 s: 20 frames.
        let mut yaw = 0.0;
        for _ in 0..20 {
            yaw = turn_toward(yaw, std::f64::consts::PI, BODY_TURN_RATE, 1.0 / 60.0);
        }
        assert!(
            (yaw - std::f64::consts::PI).abs() < 0.05,
            "about a half turn after 0.33 s: {yaw}"
        );
        // The shorter arc: from 0 to -PI/2 turns right, not left around.
        let right = turn_toward(0.0, -std::f64::consts::PI / 2.0, BODY_TURN_RATE, 1.0 / 60.0);
        assert!(right < 0.0, "turns right: {right}");
        let left = turn_toward(0.0, std::f64::consts::PI / 2.0, BODY_TURN_RATE, 1.0 / 60.0);
        assert!(left > 0.0, "turns left: {left}");
        // Small gaps arrive exactly.
        assert_eq!(turn_toward(1.0, 1.01, BODY_TURN_RATE, 1.0), 1.01);
        // Bad inputs hold.
        assert_eq!(turn_toward(1.0, f64::NAN, BODY_TURN_RATE, 1.0), 1.0);
        assert_eq!(turn_toward(1.0, 2.0, BODY_TURN_RATE, 0.0), 1.0);
        assert_eq!(turn_toward(f64::NAN, 2.0, BODY_TURN_RATE, 1.0), 2.0);
    }
}
