use super::*;

const UP: [f64; 3] = [0.0, 1.0, 0.0];
/// The sun straight below: deep night at the viewer.
const NIGHT_SUN: [f64; 3] = [0.0, -1.0, 0.0];

#[test]
fn a_full_moon_overhead_lights_from_its_side() {
    // Full: the moon opposite the sun, and straight up at the viewer.
    let light = sky_light(NIGHT_SUN, UP, [[0.0, 1.0, 0.0]], NightLight::default());
    let moon = light.moons[0].expect("a moon light");
    assert_eq!(moon.direction, [0.0, 1.0, 0.0]);
    assert!(
        (moon.share - MOON_FULL_SHARE * 0.5).abs() < 1e-12,
        "{moon:?}"
    );
    assert!(light.moons[1].is_none());
    // The moon outshines the stars' even glow many times over.
    assert!(moon.share > 5.0 * light.ambient_share, "{light:?}");
}

#[test]
fn a_moon_below_the_horizon_or_new_gives_only_the_star_glow() {
    let night = NightLight::default();
    let below = sky_light(NIGHT_SUN, UP, [[0.0, -1.0, 0.0]], night);
    let none = sky_light(NIGHT_SUN, UP, [], night);
    assert_eq!(below, none, "a set moon lights nothing");
    // New moon: between the planet and the sun (here the sun is up).
    let day_sun = [0.0, 1.0, 0.0];
    let new = sky_light(day_sun, UP, [[0.0, 1.0, 0.0]], night);
    assert!(new.moons[0].is_none(), "{new:?}");
    // Moonless night: only the stars, which are dark.
    assert!((none.ambient_share - STAR_SHARE * 0.5).abs() < 1e-12);
    assert!(none.ambient_share < 0.1 * DAY_FILL_SHARE);
}

#[test]
fn the_light_follows_the_phase_and_the_height() {
    let night = NightLight::default();
    // A half moon (sun and moon 90 degrees apart) gives half the full share.
    let half = phase([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    assert!((half - 0.5).abs() < 1e-12);
    assert!((phase(NIGHT_SUN, UP) - 1.0).abs() < 1e-12);
    assert!(phase(UP, UP).abs() < 1e-12);
    // Lower in the sky, less light on flat ground; none just under the
    // horizon, rising smoothly through it.
    let share =
        |moon: [f64; 3]| sky_light(NIGHT_SUN, UP, [moon], night).moons[0].map_or(0.0, |m| m.share);
    let high = share([0.0, 1.0, 0.0]);
    let low = share([0.9, 0.44, 0.0]);
    let rim = share([1.0, 0.02, 0.0]);
    assert!(high > low && low > rim && rim >= 0.0, "{high} {low} {rim}");
    assert!(share([1.0, -0.06, 0.0]) == 0.0);
}

#[test]
fn two_moons_add_up_brightest_first() {
    let night = NightLight::default();
    let bright = [0.0, 1.0, 0.0];
    let dim = [0.6, 0.8, 0.0];
    let light = sky_light(NIGHT_SUN, UP, [dim, bright], night);
    let (a, b) = (
        light.moons[0].expect("first"),
        light.moons[1].expect("second"),
    );
    assert_eq!(a.direction, bright);
    assert!(a.share > b.share && b.share > 0.0);
    assert!((light.moon_share() - (a.share + b.share)).abs() < 1e-12);
    // More moons than lights: the brightest four are kept.
    let many = sky_light(NIGHT_SUN, UP, [dim, dim, bright, dim, dim, dim], night);
    assert_eq!(many.moons.iter().flatten().count(), MAX_MOON_LIGHTS);
    assert_eq!(many.moons[0].map(|m| m.direction), Some(bright));
}

#[test]
fn the_day_keeps_its_fill_and_night_light_scales_the_night_only() {
    let day = sky_light(UP, UP, [], NightLight::default());
    assert!(
        (day.ambient_share - DAY_FILL_SHARE).abs() < 1e-12,
        "day as before"
    );
    let off = NightLight::new(0).expect("0%");
    let full = NightLight::new(100).expect("100%");
    assert_eq!(sky_light(UP, UP, [], off).ambient_share, day.ambient_share);
    let dark = sky_light(NIGHT_SUN, UP, [UP], off);
    assert_eq!((dark.ambient_share, dark.moon_share()), (0.0, 0.0));
    let bright = sky_light(NIGHT_SUN, UP, [UP], full);
    assert!((bright.moon_share() - MOON_FULL_SHARE).abs() < 1e-12);
}

#[test]
fn night_light_steps_by_ten_within_its_range() {
    assert_eq!(NightLight::default().get(), 50);
    assert_eq!(NightLight::new(55), None);
    assert_eq!(NightLight::new(110), None);
    let top = NightLight::new(100).expect("100");
    assert_eq!(top.brighter(), top);
    assert_eq!(NightLight::new(0).expect("0").darker().get(), 0);
    assert_eq!(NightLight::default().brighter().get(), 60);
}

#[test]
fn the_home_planet_has_moonlit_and_moonless_hours() {
    // The planet's own moons through the clock: some hour of the first
    // days has a moon up lighting the ground, and some night has none.
    use crate::nav::{DEMO_SEED, MarkerIndex, Universe};
    let chain = crate::nest::autopilot_path(DEMO_SEED);
    let mut universe = Universe::new(DEMO_SEED);
    for &marker in chain.iter().take(9) {
        assert!(universe.open(MarkerIndex(marker)), "journey opens");
    }
    let moons = universe
        .open
        .points
        .iter()
        .filter(|p| p.moon.is_some())
        .count();
    assert!(moons > 0, "the home planet has a moon");
    let viewer = [0.0, 1.0, 0.0];
    let (mut lit, mut dark) = (false, false);
    for hour in 0..(24 * 30) {
        universe.advance(f64::from(hour) * 3_600.0);
        let light = sky_light_at(&universe, viewer, NightLight::default()).expect("on the planet");
        let sun = crate::light::sun_direction(&universe).expect("sun");
        if sun[1] < -0.2 {
            lit |= light.moon_share() > 0.05;
            dark |= light.moon_share() == 0.0;
        }
    }
    assert!(lit && dark, "moonlit {lit}, moonless {dark}");
}

#[test]
fn night_light_is_saved_loaded_and_reset_with_display() {
    use crate::settings::{Category, Settings};
    let mut settings = Settings::defaults();
    assert_eq!(settings.night_light, NightLight::default());
    settings.night_light = NightLight::new(80).expect("80");
    let text = settings.format();
    assert!(text.contains("night_light = 80"), "{text}");
    assert_eq!(Settings::parse(&text).night_light.get(), 80);
    // A wrong value keeps the default.
    assert_eq!(
        Settings::parse("night_light = 33\n").night_light,
        NightLight::default()
    );
    assert_eq!(
        settings.reset(Category::Display).night_light,
        NightLight::default()
    );
}

#[test]
fn the_eyes_see_noon_as_before_and_a_moonlit_night_brighter_than_a_moonless_one() {
    let night = NightLight::default();
    let noon = sky_light(UP, UP, [], night);
    assert!((view_share(UP, UP, &noon) - 1.0).abs() < 1e-12);
    let moonlit = sky_light(NIGHT_SUN, UP, [UP], night);
    let moonless = sky_light(NIGHT_SUN, UP, [], night);
    let (lit, dark) = (
        view_share(NIGHT_SUN, UP, &moonlit),
        view_share(NIGHT_SUN, UP, &moonless),
    );
    assert!(lit > 4.0 * dark && dark > 0.0, "{lit} {dark}");
    let black = sky_light(NIGHT_SUN, UP, [], NightLight::new(0).expect("0"));
    assert!(view_share(NIGHT_SUN, UP, &black) > 0.0);
}
