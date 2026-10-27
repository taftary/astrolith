use super::*;

const AXIS: [f64; 3] = [0.0, 1.0, 0.0];
const DAY: f64 = 86_400.0;

#[test]
fn noon_under_the_sun_midnight_opposite_and_dawn_and_dusk_between() {
    let sun = [1.0, 0.0, 0.0];
    let hour = |up: [f64; 3]| local_time_of_day_s(sun, up, AXIS, DAY).expect("hour") / 3_600.0;
    assert!((hour([1.0, 0.0, 0.0]) - 12.0).abs() < 1e-9, "noon");
    assert!(
        (hour([-1.0, 0.0, 0.0]) - 0.0).abs() < 1e-9 || (hour([-1.0, 0.0, 0.0]) - 24.0).abs() < 1e-9
    );
    // Turning about +Y right-handed carries +X toward -Z: a spot 90 degrees
    // further along the spin has passed noon by six hours.
    assert!((hour([0.0, 0.0, -1.0]) - 18.0).abs() < 1e-9, "evening");
    assert!((hour([0.0, 0.0, 1.0]) - 6.0).abs() < 1e-9, "morning");
    // Latitude does not change the hour.
    assert!((hour([0.7, 0.7, 0.0]) - 12.0).abs() < 1e-9);
    assert_eq!(local_time_of_day_s(sun, AXIS, AXIS, DAY), None, "pole");
}

#[test]
fn the_readout_keeps_the_home_day_near_and_follows_local_midnight() {
    // Home 08:00 on day 1 at game time 0.
    assert_eq!(local_readout(0.0, 8.0 * 3_600.0, DAY), "Day 1, 08:00");
    // Four hours further east: 12:00 the same day.
    assert_eq!(local_readout(0.0, 12.0 * 3_600.0, DAY), "Day 1, 12:00");
    // Ten hours west at home 08:00 on day 2: still the night before, day 1.
    assert_eq!(local_readout(DAY, 22.0 * 3_600.0, DAY), "Day 1, 22:00");
    // Never before day 1.
    assert_eq!(local_readout(0.0, 23.0 * 3_600.0, DAY), "Day 1, 23:00");
}

#[test]
fn on_the_home_planet_the_local_hour_runs_with_the_sky() {
    use crate::nav::{DEMO_SEED, MarkerIndex, Universe};
    let chain = crate::nest::autopilot_path(DEMO_SEED);
    let mut universe = Universe::new(DEMO_SEED);
    for &marker in chain.iter().take(9) {
        assert!(universe.open(MarkerIndex(marker)), "journey opens");
    }
    let up = [0.3, 0.2, 0.93];
    let day_s = universe.spin_hours.expect("a day") * 3_600.0;
    let mut last: Option<f64> = None;
    for step in 0..48 {
        universe.advance(f64::from(step) * day_s / 48.0);
        let sun = crate::light::sun_direction(&universe).expect("sun");
        let tod = local_time_of_day_s(sun, up, universe.spin_axis, day_s).expect("hour");
        // The sun is high near local noon and below the horizon near
        // local midnight.
        let height = sun[0] * up[0] + sun[1] * up[1] + sun[2] * up[2];
        let hours = tod / 3_600.0 * 24.0 * 3_600.0 / day_s;
        if (hours - 12.0).abs() < 1.0 {
            assert!(height > 0.3, "noon at {hours:.2} h: height {height}");
        }
        if !(1.0..=23.0).contains(&hours) {
            assert!(height < -0.3, "midnight at {hours:.2} h: height {height}");
        }
        // Half an hour of sky is about half an hour on the clock.
        if let Some(prev) = last {
            let ran = (tod - prev).rem_euclid(day_s);
            assert!((ran - day_s / 48.0).abs() < day_s / 400.0, "ran {ran}");
        }
        last = Some(tod);
        assert!(readout_at(&universe, up).is_some());
    }
    assert_eq!(
        readout_at(&Universe::new(DEMO_SEED), up),
        None,
        "off the planet"
    );
}
