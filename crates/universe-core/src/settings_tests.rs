//! Settings tests (split from `settings.rs` under `E-FILE-SIZE`, #776 T4).
//!
//! Unit tests for player settings live here so `settings.rs` stays under the
//! file-size guard. Pure and hermetic (`E-TEST-HERMETIC`).

use crate::destination::Destination;
use crate::settings::*;

fn pose() -> SavedPose {
    let yaw = 0.1;
    let pitch = -0.2;
    let free_forward = crate::flight::free_look_direction(yaw, pitch);
    let free_up = crate::flight::orthogonal_up(free_forward, [0.0, 1.0, 0.0]);
    SavedPose {
        offset: [0.25, -1.5, 3.0e-7],
        destination: Destination::from_chain(&[3, 1, 4]),
        mode: ViewMode::Free,
        yaw,
        pitch,
        free_forward,
        free_up,
        speed: FlightSpeed(7),
    }
}

#[test]
fn defaults_match_spec_v1() {
    let s = Settings::defaults();
    assert_eq!(s.exposure.get(), 1.0);
    assert!(s.labels);
    assert_eq!(s.label_limit.get(), 24);
    assert!(!s.fullscreen);
    assert!(s.vsync);
    assert!(s.mode_reminder, "the mode reminder starts on (#557)");
    assert!(s.show_pull, "the pull shows by default (#559)");
    assert_eq!(
        s.body_colour,
        BodyColour::GreyOrange,
        "grey and orange by default (#755)"
    );
    assert_eq!(s.look_sensitivity.get(), LOOK_DEFAULT);
    assert!(
        !s.invert_look && !s.dive_toggle && !s.look_toggle,
        "today's controls by default"
    );
    assert!(
        !s.plane_turning,
        "flying turns like a drone by default (#558)"
    );
    assert!(
        s.start_third_person,
        "normal mode starts in third person (#558)"
    );
    assert_eq!(s.speed.get(), 5);
    assert!(s.views.iter().all(Option::is_none));
    assert!(!s.debug_axes);
    assert!(!s.debug_overlay, "overlay off by default (#776)");
    assert_eq!(
        s.planet_detail,
        PlanetDetail::Normal,
        "Normal by default (#776)"
    );
    assert!(
        !s.touch_always && !s.left_hand && !s.simulate_touch,
        "touch: automatic, right hand, no simulation (#561)"
    );
}

#[test]
fn ranges_reject_out_of_range_values() {
    assert!(Exposure::new(0.2).is_none());
    assert!(Exposure::new(4.1).is_none());
    assert!(Exposure::new(f32::NAN).is_none());
    assert!(Exposure::new(0.25).is_some());
    assert!(LabelLimit::new(25).is_none());
    assert!(LabelLimit::new(0).is_some());
    assert!(FlightSpeed::new(0).is_none());
    assert!(FlightSpeed::new(10).is_none());
    assert!(FlightSpeed::new(9).is_some());
}

#[test]
fn exposure_steps_clamp_at_both_ends() {
    let mut e = Exposure::default();
    for _ in 0..20 {
        e = e.brighter();
    }
    assert_eq!(e.get(), EXPOSURE_MAX);
    for _ in 0..40 {
        e = e.darker();
    }
    assert_eq!(e.get(), EXPOSURE_MIN);
    assert_eq!(Exposure::default().brighter().get(), EXPOSURE_STEP);
}

#[test]
fn look_sensitivity_scales_around_today() {
    assert_eq!(LookSensitivity::default().scale(), 1.0);
    let slow = LookSensitivity::new(LOOK_MIN).expect("min").scale();
    let fast = LookSensitivity::new(LOOK_MAX).expect("max").scale();
    assert!(slow < 1.0 && fast > 1.0, "{slow} {fast}");
    assert!(
        (fast * slow - 1.0).abs() < 1e-12,
        "symmetric around the default"
    );
    assert!(LookSensitivity::new(0).is_none() && LookSensitivity::new(10).is_none());
}

#[test]
fn controls_reset_restores_the_new_controls() {
    let mut s = Settings::defaults();
    s.look_sensitivity = LookSensitivity(2);
    s.invert_look = true;
    s.dive_toggle = true;
    s.look_toggle = true;
    s.mode_reminder = false;
    let controls = s.reset(Category::Controls);
    assert_eq!(controls.look_sensitivity, LookSensitivity::default());
    assert!(!controls.invert_look && !controls.dive_toggle && !controls.look_toggle);
    assert!(!controls.mode_reminder, "Display keeps its own values");
    assert!(s.reset(Category::Display).mode_reminder);
}

#[test]
fn round_trip_keeps_every_value() {
    let mut s = Settings::defaults();
    s.exposure = Exposure(2.4414062);
    s.labels = false;
    s.label_limit = LabelLimit(3);
    s.mode_reminder = false;
    s.show_pull = false;
    s.body_colour = BodyColour::Blue;
    s.look_sensitivity = LookSensitivity(8);
    s.invert_look = true;
    s.dive_toggle = true;
    s.look_toggle = true;
    s.plane_turning = true;
    s.start_third_person = false;
    s.keymap.bind(
        crate::keymap::Action::Forward,
        crate::keymap::Binding::named("KeyI").expect("bindable"),
    );
    s.fullscreen = true;
    s.vsync = false;
    s.speed = FlightSpeed(9);
    s.debug_axes = true;
    s.debug_overlay = true;
    s.planet_detail = PlanetDetail::High;
    s.touch_always = true;
    s.left_hand = true;
    s.simulate_touch = true;
    s.player = Some((
        42,
        crate::player::PlayerSpot::new(&[1, 2, 3], [0.1, -0.2, 0.3], 1.5).expect("spot"),
    ));
    s.views[2] = Some(pose());
    s.views[7] = Some(SavedPose {
        destination: None,
        mode: ViewMode::Dive,
        ..pose()
    });
    assert_eq!(Settings::parse(&s.format()), s);
}

#[test]
fn ground_guides_setting_removed() {
    // #770 S1: the row and stored value are gone; old files still load.
    let s = Settings::defaults();
    assert!(!s.format().contains("ground_guides"), "no stored value");
    // Old file with the key still parses (ignored).
    let old = format!("{}\nground_guides = on\n", s.format());
    assert_eq!(Settings::parse(&old), s, "old key ignored");
}

#[test]
fn empty_or_garbage_text_gives_defaults() {
    assert_eq!(Settings::parse(""), Settings::defaults());
    assert_eq!(
        Settings::parse("\u{0}\u{1}random ### text = = =\nexposure\n= 3"),
        Settings::defaults()
    );
}

#[test]
fn bad_lines_keep_defaults_and_good_lines_still_apply() {
    let s = Settings::parse(
        "exposure = 99\nlabels = maybe\nlabel_limit = -1\nspeed = 12\n\
         view9 = dive, 0, 0, 0, 0, 0, 5, -\nview1 = dive, x, 0, 0, 0, 0, 5, -\n\
         view2 = dive, 0, 0, 0, 0, 0, 5, -, extra\nunknown = 1\nvsync = off\n",
    );
    let mut want = Settings::defaults();
    want.vsync = false;
    assert_eq!(s, want);
}

#[test]
fn reset_touches_only_its_category() {
    let mut s = Settings::defaults();
    s.exposure = Exposure(2.0);
    s.speed = FlightSpeed(2);
    s.views[0] = Some(pose());
    s.debug_axes = true;
    s.debug_overlay = true;
    s.planet_detail = PlanetDetail::Low;
    let d = s.reset(Category::Display);
    assert_eq!(d.exposure, Exposure::default());
    assert_eq!(d.speed.get(), 2);
    let v = s.reset(Category::Views);
    assert!(v.views.iter().all(Option::is_none));
    assert!(v.debug_axes);
    assert_eq!(s.reset(Category::Accessibility), s);
    assert!(!s.reset(Category::Developer).debug_axes);
    assert!(!s.reset(Category::Developer).debug_overlay);
    assert_eq!(s.reset(Category::Controls).speed, FlightSpeed::default());
}

#[test]
fn dimming_is_60_to_95_in_fives_default_80() {
    assert_eq!(Settings::defaults().dimming.get(), 80);
    assert!(Dimming::new(55).is_none());
    assert!(Dimming::new(100).is_none());
    assert!(Dimming::new(72).is_none(), "off-step");
    assert!(Dimming::new(60).is_some() && Dimming::new(95).is_some());
    let mut d = Dimming::default();
    for _ in 0..20 {
        d = d.darker();
    }
    assert_eq!(d.get(), DIMMING_MAX);
    for _ in 0..20 {
        d = d.lighter();
    }
    assert_eq!(d.get(), DIMMING_MIN);
    assert_eq!(Dimming::default().darker().get(), 85);
    assert!((Dimming(70).fraction() - 0.70).abs() < 1e-6);
}

#[test]
fn dimming_saves_loads_and_falls_back_to_80() {
    let mut s = Settings::defaults();
    s.dimming = Dimming(70);
    assert!(s.format().contains("dimming = 70"));
    assert_eq!(Settings::parse(&s.format()).dimming.get(), 70);
    // A file written before this setting existed has no dimming line.
    let old = "exposure = 2\nlabels = off\nspeed = 3\n";
    let loaded = Settings::parse(old);
    assert_eq!(loaded.dimming.get(), DIMMING_DEFAULT);
    assert!(!loaded.labels, "the rest of the old file still loads");
    assert_eq!(Settings::parse("dimming = 99").dimming.get(), 80);
}

#[test]
fn display_reset_puts_dimming_back_to_80() {
    let mut s = Settings::defaults();
    s.dimming = Dimming(60);
    assert_eq!(s.reset(Category::Display).dimming.get(), 80);
    assert_eq!(s.reset(Category::Controls).dimming.get(), 60);
}

#[test]
fn body_colour_setting_cycles_and_saves() {
    // Default grey and orange, cycling white, blue, red and back.
    assert_eq!(BodyColour::default(), BodyColour::GreyOrange);
    assert_eq!(BodyColour::ALL.len(), 4);
    let mut colour = BodyColour::GreyOrange;
    for want in [
        BodyColour::White,
        BodyColour::Blue,
        BodyColour::Red,
        BodyColour::GreyOrange,
    ] {
        colour = colour.next();
        assert_eq!(colour, want);
    }
    assert_eq!(
        BodyColour::from_id("grey-orange"),
        Some(BodyColour::GreyOrange)
    );
    assert_eq!(BodyColour::from_id("white"), Some(BodyColour::White));
    assert_eq!(BodyColour::from_id("blue"), Some(BodyColour::Blue));
    assert_eq!(BodyColour::from_id("red"), Some(BodyColour::Red));
    assert_eq!(BodyColour::from_id("purple"), None);
    // Saved, restored and reset with Display.
    for preset in BodyColour::ALL {
        let mut s = Settings::defaults();
        s.body_colour = preset;
        assert!(
            s.format()
                .contains(&format!("body_colour = {}", preset.id()))
        );
        assert_eq!(Settings::parse(&s.format()).body_colour, preset);
    }
    // Damaged entry keeps its default.
    assert_eq!(
        Settings::parse("body_colour = purple").body_colour,
        BodyColour::GreyOrange
    );
    let old = "exposure = 2\nlabels = off\nspeed = 3\n";
    assert_eq!(
        Settings::parse(old).body_colour,
        BodyColour::GreyOrange,
        "a file before this setting keeps the default"
    );
    let mut s = Settings::defaults();
    s.body_colour = BodyColour::Red;
    s.speed = FlightSpeed(2);
    assert_eq!(
        s.reset(Category::Display).body_colour,
        BodyColour::GreyOrange
    );
    assert_eq!(s.reset(Category::Controls).body_colour, BodyColour::Red);
    assert_eq!(s.reset(Category::Display).speed.get(), 2);
}

#[test]
fn planet_detail_and_debug_overlay_settings() {
    // AC9 / C9 (T4, S1, S2): Planet detail (Low 2 px, Normal 1.5 px, High 1 px;
    // default Normal) sets the split and merge thresholds, and Debug overlay
    // (default off) stays in step with F3; both are saved, restored and reset.
    assert_eq!(PlanetDetail::default(), PlanetDetail::Normal);
    assert_eq!(PlanetDetail::ALL.len(), 3);
    assert_eq!(PlanetDetail::Low.detail_px(), 2.0);
    assert_eq!(PlanetDetail::Normal.detail_px(), 1.5);
    assert_eq!(PlanetDetail::High.detail_px(), 1.0);
    let mut detail = PlanetDetail::Low;
    for want in [PlanetDetail::Normal, PlanetDetail::High, PlanetDetail::Low] {
        detail = detail.next();
        assert_eq!(detail, want);
    }
    assert_eq!(PlanetDetail::from_id("low"), Some(PlanetDetail::Low));
    assert_eq!(PlanetDetail::from_id("normal"), Some(PlanetDetail::Normal));
    assert_eq!(PlanetDetail::from_id("high"), Some(PlanetDetail::High));
    assert_eq!(PlanetDetail::from_id("ultra"), None);
    for preset in PlanetDetail::ALL {
        let mut s = Settings::defaults();
        s.planet_detail = preset;
        assert!(
            s.format()
                .contains(&format!("planet_detail = {}", preset.id()))
        );
        let loaded = Settings::parse(&s.format());
        assert_eq!(loaded.planet_detail, preset);
        // Thresholds follow the choice; merge stays at half.
        assert_eq!(loaded.planet_detail.detail_px(), preset.detail_px());
    }
    assert_eq!(
        Settings::parse("planet_detail = ultra").planet_detail,
        PlanetDetail::Normal
    );
    let old = "exposure = 2\nlabels = off\nspeed = 3\n";
    assert_eq!(
        Settings::parse(old).planet_detail,
        PlanetDetail::Normal,
        "a file before this setting keeps Normal"
    );
    // Debug overlay: default off, saved, restored and reset with Developer.
    assert!(!Settings::defaults().debug_overlay);
    let mut s = Settings::defaults();
    s.debug_overlay = true;
    assert!(s.format().contains("debug_overlay = on"));
    assert!(Settings::parse(&s.format()).debug_overlay);
    assert!(!Settings::parse("debug_overlay = maybe").debug_overlay);
    assert!(!Settings::parse(old).debug_overlay);
    let mut s = Settings::defaults();
    s.debug_overlay = true;
    s.planet_detail = PlanetDetail::High;
    s.speed = FlightSpeed(2);
    assert!(!s.reset(Category::Developer).debug_overlay);
    assert_eq!(
        s.reset(Category::Developer).planet_detail,
        PlanetDetail::High,
        "Display keeps its own detail"
    );
    assert_eq!(
        s.reset(Category::Display).planet_detail,
        PlanetDetail::Normal
    );
    assert!(s.reset(Category::Display).debug_overlay);
    assert_eq!(s.reset(Category::Display).speed.get(), 2);
}

#[test]
fn shadows_setting_defaults_on_and_saves() {
    // C8 (#780 S1): Display > Scene > Shadows is on by default, saved and
    // restored, a file before it keeps it on, and Display reset turns it on.
    assert!(Settings::defaults().shadows);
    let mut s = Settings::defaults();
    s.shadows = false;
    assert!(s.format().contains("shadows = off"));
    assert!(!Settings::parse(&s.format()).shadows);
    assert!(Settings::parse("shadows = maybe").shadows);
    assert!(
        Settings::parse("exposure = 2\nlabels = off\n").shadows,
        "a file before this setting keeps shadows on"
    );
    assert!(s.reset(Category::Display).shadows);
    assert!(!s.reset(Category::Controls).shadows);
}

#[test]
fn dive_orbit_speed_setting_round_trips() {
    // AC8/C8 (#873 T5 S1): Controls > Flight > Dive orbit speed, 0.5..=2.0,
    // default 1.0, saved and restored, Display reset keeps it, Controls
    // reset restores 1.0; 2.0 orbits twice as fast as 1.0.
    use crate::dive_orbit::orbit_rate;
    use crate::settings::{DIVE_ORBIT_SPEED_DEFAULT, DIVE_ORBIT_SPEED_MAX, DIVE_ORBIT_SPEED_MIN};
    assert_eq!(DIVE_ORBIT_SPEED_DEFAULT, 1.0);
    assert_eq!(DIVE_ORBIT_SPEED_MIN, 0.5);
    assert_eq!(DIVE_ORBIT_SPEED_MAX, 2.0);
    assert_eq!(Settings::defaults().dive_orbit_speed.get(), 1.0);
    assert!(DiveOrbitSpeed::new(0.5).is_some());
    assert!(DiveOrbitSpeed::new(2.0).is_some());
    assert!(DiveOrbitSpeed::new(0.49).is_none());
    assert!(DiveOrbitSpeed::new(2.01).is_none());
    assert!(DiveOrbitSpeed::new(f32::NAN).is_none());
    assert!(DiveOrbitSpeed::new(f32::INFINITY).is_none());
    // 2.0 orbits twice as fast as 1.0.
    let slow = orbit_rate(0.5, 1.0, 1.0);
    let fast = orbit_rate(0.5, 1.0, 2.0);
    assert!((fast - 2.0 * slow).abs() < 1e-12);
    // Saved and restored.
    let mut s = Settings::defaults();
    s.dive_orbit_speed = DiveOrbitSpeed::new(1.5).expect("in range");
    assert!(s.format().contains("dive_orbit_speed = 1.5"));
    assert_eq!(
        Settings::parse(&s.format()).dive_orbit_speed,
        s.dive_orbit_speed
    );
    assert_eq!(
        Settings::parse("dive_orbit_speed = bogus").dive_orbit_speed,
        DiveOrbitSpeed::default()
    );
    assert_eq!(
        Settings::parse("exposure = 2\nlabels = off\n").dive_orbit_speed,
        DiveOrbitSpeed::default(),
        "a file before this setting keeps the default"
    );
    // Reset.
    assert_eq!(
        s.reset(Category::Controls).dive_orbit_speed,
        DiveOrbitSpeed::default()
    );
    s.dive_orbit_speed = DiveOrbitSpeed::new(1.5).expect("in range");
    assert_eq!(
        s.reset(Category::Display).dive_orbit_speed.get(),
        1.5,
        "Display reset keeps the flight speed"
    );
}

#[test]
fn exposure_moves_to_b() {
    // AC4/C4 (#884 T3, Q7 A): B darkens and Shift+B brightens, E no longer
    // changes exposure, Q/E are the roll defaults, and a saved map with the
    // old E for exposure moves to B.
    use crate::keymap::{Action, Binding};
    let map = crate::keymap::Keymap::default();
    assert_eq!(map.binding(Action::Exposure).name(), "KeyB");
    assert_eq!(map.binding(Action::RollLeft).name(), "KeyQ");
    assert_eq!(map.binding(Action::RollRight).name(), "KeyE");
    assert_eq!(
        map.action_for(Binding::named("KeyB").expect("bindable")),
        Some(Action::Exposure)
    );
    assert_eq!(
        map.action_for(Binding::named("KeyQ").expect("bindable")),
        Some(Action::RollLeft)
    );
    assert_eq!(
        map.action_for(Binding::named("KeyE").expect("bindable")),
        Some(Action::RollRight),
        "E rolls right, no longer changes exposure"
    );
    // Old file: exposure on E, no roll entries (defaults Q/E) -> moves to B.
    let old = Settings::parse("key.exposure = KeyE\n");
    assert_eq!(
        old.keymap.binding(Action::Exposure).name(),
        "KeyB",
        "old E moves to B"
    );
    assert_eq!(old.keymap.binding(Action::RollRight).name(), "KeyE");
    // Player's own rebind (exposure on E, roll elsewhere) is kept.
    let custom = Settings::parse("key.exposure = KeyE\nkey.roll_right = KeyB\n");
    assert_eq!(
        custom.keymap.binding(Action::Exposure).name(),
        "KeyE",
        "custom choice kept"
    );
    // New file stays.
    let current = Settings::parse("key.exposure = KeyB\n");
    assert_eq!(current.keymap.binding(Action::Exposure).name(), "KeyB");
    // Exposure steps still work: darker then brighter returns.
    let e = Exposure::default();
    assert!(e.darker().get() < e.get());
    assert_eq!(e.darker().brighter().get(), e.get());
}
