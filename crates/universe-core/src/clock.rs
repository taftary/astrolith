//! The sky's clock (#981, Spec v2): one number of game seconds drives every
//! moving body.
//!
//! At 1x one Earth day passes in ten minutes of real time
//! ([`GAME_PER_REAL`] = 144, Q2 B), and the speed steps by tens up to
//! 10,000x (Q3 A). Every position is a pure function of the game time:
//! an orbit angle is its phase plus `2 pi t / period`, a spin angle is
//! `2 pi t / day`, so nothing is stored or rebuilt per frame.
//!
//! Pure and deterministic (`E-CORE-NO-BEVY`, `E-DET-TIERS`); the
//! trigonometry stays in core (`E-TRANSCENDENTAL`).

use std::f64::consts::TAU;

/// Game seconds per real second at 1x: one Earth day (24 h) in ten minutes.
pub const GAME_PER_REAL: f64 = 144.0;

/// The speed steps, slowest first (Q3 A).
pub const SPEED_STEPS: [f64; 5] = [1.0, 10.0, 100.0, 1_000.0, 10_000.0];

/// Seconds in a day.
pub const DAY_S: f64 = 86_400.0;

/// The home planet's local hour at game time zero: the seeded start is in
/// the morning (#780 F1 A).
pub const START_HOUR: f64 = 8.0;

/// The sky's clock: game time, the speed step and whether it plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clock {
    /// Game seconds since the start.
    pub game_s: f64,
    /// Index into [`SPEED_STEPS`].
    pub step: usize,
    /// Whether the sky moves (Q4 A: pause stops only the sky).
    pub playing: bool,
}

impl Default for Clock {
    /// Playing at 1x from game time zero (Q4 A).
    fn default() -> Clock {
        Clock {
            game_s: 0.0,
            step: 0,
            playing: true,
        }
    }
}

impl Clock {
    /// The speed multiplier, `1` to `10,000`.
    #[must_use]
    pub fn speed(&self) -> f64 {
        SPEED_STEPS
            .get(self.step)
            .copied()
            .unwrap_or(SPEED_STEPS[0])
    }

    /// Advances by `real_dt` seconds of real time when playing. A
    /// non-finite or negative step changes nothing.
    pub fn tick(&mut self, real_dt: f64) {
        if self.playing && real_dt.is_finite() && real_dt > 0.0 {
            self.game_s += real_dt * GAME_PER_REAL * self.speed();
        }
    }

    /// One speed step faster, stopping at the fastest.
    pub fn faster(&mut self) {
        self.step = (self.step + 1).min(SPEED_STEPS.len() - 1);
    }

    /// One speed step slower, stopping at 1x.
    pub fn slower(&mut self) {
        self.step = self.step.saturating_sub(1);
    }

    /// Plays or pauses the sky.
    pub fn toggle(&mut self) {
        self.playing = !self.playing;
    }
}

/// The speed label shown between the buttons: `"1×"` to `"10,000×"`.
#[must_use]
pub fn speed_label(speed: f64) -> String {
    match speed {
        s if s >= 10_000.0 => String::from("10,000×"),
        s if s >= 1_000.0 => String::from("1,000×"),
        s if s >= 100.0 => String::from("100×"),
        s if s >= 10.0 => String::from("10×"),
        _ => String::from("1×"),
    }
}

/// An orbit angle (radians) at `game_s`: `phase` plus one turn per
/// `period_days`. A non-positive period keeps the phase.
#[must_use]
pub fn orbit_angle_at(phase: f64, period_days: f64, game_s: f64) -> f64 {
    if !(period_days.is_finite() && period_days > 0.0) {
        return phase;
    }
    phase + TAU * (game_s / (period_days * DAY_S)).rem_euclid(1.0)
}

/// A spin angle (radians) at `game_s` for a day of `spin_hours`. A
/// non-positive day keeps zero.
#[must_use]
pub fn spin_angle_at(spin_hours: f64, game_s: f64) -> f64 {
    if !(spin_hours.is_finite() && spin_hours > 0.0) {
        return 0.0;
    }
    TAU * (game_s / (spin_hours * 3_600.0)).rem_euclid(1.0)
}

/// The home calendar readout for `game_s` on a planet whose day lasts
/// `spin_hours` (Q7 A): `"Day 12, 08:40"`. Day one starts at
/// [`START_HOUR`].
#[must_use]
pub fn readout(game_s: f64, spin_hours: f64) -> String {
    let day_s = if spin_hours.is_finite() && spin_hours > 0.0 {
        spin_hours * 3_600.0
    } else {
        DAY_S
    };
    let since = START_HOUR * 3_600.0 + game_s.max(0.0);
    let day = (since / day_s).floor();
    let within = since - day * day_s;
    let minutes = (within / 60.0).floor();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_earth_day_takes_ten_minutes_at_1x() {
        // #981 Q2 B: 1x is 144 times real time.
        let mut clock = Clock::default();
        clock.tick(600.0);
        assert!((clock.game_s - DAY_S).abs() < 1e-9);
        assert!(clock.playing, "the game starts playing (Q4 A)");
    }

    #[test]
    fn speeds_step_by_tens_and_pause_stops_the_clock() {
        // #981 Q3 A, Q4 A.
        let mut clock = Clock::default();
        for want in ["10×", "100×", "1,000×", "10,000×", "10,000×"] {
            clock.faster();
            assert_eq!(speed_label(clock.speed()), want);
        }
        clock.slower();
        assert_eq!(clock.speed(), 1_000.0);
        clock.toggle();
        let before = clock.game_s;
        clock.tick(10.0);
        assert_eq!(clock.game_s, before, "paused");
        for _ in 0..9 {
            clock.slower();
        }
        assert_eq!(clock.speed(), 1.0);
    }

    #[test]
    fn angles_turn_once_per_period() {
        // #981 Q1 C: orbits and spins as pure functions of time.
        let quarter = orbit_angle_at(0.5, 365.25, 365.25 * DAY_S / 4.0);
        assert!((quarter - (0.5 + TAU / 4.0)).abs() < 1e-12);
        assert!((orbit_angle_at(0.5, 365.25, 365.25 * DAY_S) - 0.5).abs() < 1e-9);
        assert!((spin_angle_at(24.0, 6.0 * 3_600.0) - TAU / 4.0).abs() < 1e-12);
        assert_eq!(orbit_angle_at(0.5, 0.0, 1e9), 0.5);
        assert_eq!(spin_angle_at(-1.0, 1e9), 0.0);
    }

    #[test]
    fn the_readout_counts_days_and_minutes() {
        // #981 Q7 A: day one starts in the morning.
        assert_eq!(readout(0.0, 24.0), "Day 1, 08:00");
        assert_eq!(readout(16.0 * 3_600.0 + 40.0 * 60.0, 24.0), "Day 2, 00:40");
        assert_eq!(readout(11.0 * DAY_S, 24.0), "Day 12, 08:00");
    }
}
