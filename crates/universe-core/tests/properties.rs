//! Property tests for `universe-core` (M2 for #85).
//!
//! Each test name states its property as a sentence. `proptest` is a
//! dev-dependency only, so it does not count against "std only"
//! (`E-PROTEST`). Regressions, if proptest ever writes them, are committed
//! under `proptest-regressions/`.

use proptest::prelude::*;
use universe_core::coords::OpenUnits;
use universe_core::destination::Destination;
use universe_core::nav::dive_step;
use universe_core::nest::{MarkerPath, generate_cell};
use universe_core::settings::{
    BodyColour, CameraSmoothing, Dimming, Exposure, FlightSpeed, LabelLimit, LookSensitivity,
    PlanetDetail, SavedPose, Settings, VIEW_SLOTS, ViewMode,
};
use universe_core::snapshot::snapshot_generated;
use universe_core::verify::check_border;

proptest! {
    /// Regenerating a cell twice from the same seed and chain gives
    /// identical snapshots.
    #[test]
    fn regenerating_gives_identical_snapshots(
        root in any::<u64>(),
        chain in prop::collection::vec(0u32..64, 0..14),
    ) {
        let first = snapshot_generated(&generate_cell(root, &chain));
        let second = snapshot_generated(&generate_cell(root, &chain));
        prop_assert_eq!(first, second);
    }

    /// Opening a marker and closing it returns the camera offset within
    /// 1e-12 (relative, floor 1.0).
    #[test]
    fn open_then_close_returns_the_offset_within_1e12(
        offset in (-10.0f64..10.0, -10.0f64..10.0, -10.0f64..10.0),
        marker in 0u32..128,
        position in (-0.5f64..0.5, -0.5f64..0.5, -0.5f64..0.5),
    ) {
        let mut path = MarkerPath::root([offset.0, offset.1, offset.2]);
        let before = path.offset();
        prop_assume!(path.open(marker, [position.0, position.1, position.2]));
        path.close();
        let after = path.offset();
        for (a, b) in before.iter().zip(after.iter()) {
            prop_assert!(
                (a - b).abs() <= 1e-12 * b.abs().max(1.0),
                "offset drifted: {before:?} vs {after:?}",
            );
        }
    }

    /// `snapshot_generated` ignores point order: reversing and rotating the
    /// points changes nothing.
    #[test]
    fn snapshot_generated_ignores_point_order(
        root in any::<u64>(),
        chain in prop::collection::vec(0u32..64, 0..14),
        rotate in 0usize..32,
    ) {
        let cell = generate_cell(root, &chain);
        let mut permuted = cell.points.clone();
        permuted.reverse();
        if !permuted.is_empty() {
            let shift = rotate % permuted.len();
            permuted.rotate_left(shift);
        }
        let mut shuffled = cell.clone();
        shuffled.points = permuted;
        prop_assert_eq!(
            snapshot_generated(&cell),
            snapshot_generated(&shuffled)
        );
    }

    /// `check_border(a, b)` equals `check_border(b, a)` for any sample
    /// sets and any positive epsilon.
    #[test]
    fn check_border_reads_the_same_both_ways(
        a_points in prop::collection::vec(
            (-1.0f64..1.0, -1.0f64..1.0, -1.0f64..1.0),
            0..8,
        ),
        b_points in prop::collection::vec(
            (-1.0f64..1.0, -1.0f64..1.0, -1.0f64..1.0),
            0..8,
        ),
        epsilon in 1e-12f64..1e-3,
    ) {
        let a: Vec<[f64; 3]> = a_points
            .into_iter()
            .map(|(x, y, z)| [x, y, z])
            .collect();
        let b: Vec<[f64; 3]> = b_points
            .into_iter()
            .map(|(x, y, z)| [x, y, z])
            .collect();
        prop_assert_eq!(
            check_border(&a, &b, epsilon),
            check_border(&b, &a, epsilon)
        );
    }

    /// `dive_step` never crosses the surface for any factor in (0, 1]: the
    /// stepped camera stays at least `radius` from the center.
    ///
    /// Moved with `dive_step` from `universe-render` in M3 (#85).
    #[test]
    fn dive_step_never_crosses_the_surface_for_factor_in_0_1(
        camera in (-10.0f64..10.0, -10.0f64..10.0, -10.0f64..10.0),
        center in (-10.0f64..10.0, -10.0f64..10.0, -10.0f64..10.0),
        radius in 0.001f64..10.0,
        factor in 0.0001f64..=1.0,
    ) {
        let camera = OpenUnits([camera.0, camera.1, camera.2]);
        let center = OpenUnits([center.0, center.1, center.2]);
        let next = dive_step(camera, center, radius, factor);
        let distance = next
            .0
            .iter()
            .zip(center.0.iter())
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt();
        prop_assert!(
            distance + 1e-9 >= radius,
            "crossed the surface: distance {distance} < radius {radius}",
        );
    }
}

fn pose_strategy() -> impl Strategy<Value = Option<SavedPose>> {
    prop::option::of((
        prop::array::uniform3(-1.0e12f64..1.0e12),
        prop::option::of(prop::collection::vec(any::<u32>(), 0..14)),
        any::<bool>(),
        -10.0f64..10.0,
        -10.0f64..10.0,
        1u32..=9,
    ))
    .prop_map(|v| {
        v.and_then(|(offset, chain, free, yaw, pitch, speed)| {
            Some(SavedPose {
                offset,
                destination: match chain {
                    None => None,
                    Some(c) => Some(Destination::from_chain(&c)?),
                },
                mode: if free { ViewMode::Free } else { ViewMode::Dive },
                yaw,
                pitch,
                speed: FlightSpeed::new(speed)?,
            })
        })
    })
}

proptest! {
    /// Writing settings to text and reading the text back gives the same
    /// settings (#473).
    #[test]
    fn settings_survive_a_file_round_trip(
        exposure in 0.25f32..=4.0,
        labels in any::<bool>(),
        limit in 0usize..=24,
        fullscreen in any::<bool>(),
        vsync in any::<bool>(),
        speed in 1u32..=9,
        look in 1u32..=9,
        toggles in prop::array::uniform4(any::<bool>()),
        debug_axes in any::<bool>(),
        debug_overlay in any::<bool>(),
        dimming in 12u8..=19,
        body_colour in 0u8..4,
        planet_detail in 0u8..3,
        movement_inertia in any::<bool>(),
        smoothing in 0u8..4,
        views in prop::array::uniform8(pose_strategy()),
    ) {
        let s = Settings {
            exposure: Exposure::new(exposure).unwrap_or_default(),
            labels,
            label_limit: LabelLimit::new(limit).unwrap_or_default(),
            dimming: Dimming::new(dimming * 5).unwrap_or_default(),
            mode_reminder: toggles[0],
            show_pull: !toggles[2],
            body_colour: match body_colour {
                0 => BodyColour::GreyOrange,
                1 => BodyColour::White,
                2 => BodyColour::Blue,
                _ => BodyColour::Red,
            },
            planet_detail: match planet_detail {
                0 => PlanetDetail::Low,
                1 => PlanetDetail::Normal,
                _ => PlanetDetail::High,
            },
            camera_smoothing: match smoothing {
                0 => CameraSmoothing::Off,
                1 => CameraSmoothing::Low,
                2 => CameraSmoothing::Medium,
                _ => CameraSmoothing::High,
            },
            fullscreen,
            vsync,
            speed: FlightSpeed::new(speed).unwrap_or_default(),
            look_sensitivity: LookSensitivity::new(look).unwrap_or_default(),
            invert_look: toggles[1],
            dive_toggle: toggles[2],
            look_toggle: toggles[3],
            plane_turning: !toggles[0],
            start_third_person: !toggles[1],
            movement_inertia,
            keymap: universe_core::keymap::Keymap::default(),
            touch_always: toggles[3],
            left_hand: !toggles[3],
            views,
            debug_axes,
            debug_overlay,
            simulate_touch: !debug_axes,
            player: labels.then(|| {
                let spot = universe_core::player::PlayerSpot::new(
                    &[3, 1, 4],
                    [f64::from(exposure), -0.1, 0.2],
                    f64::from(look) * 0.3,
                );
                (u64::from(speed), spot.expect("short chain"))
            }),
        };
        prop_assert_eq!(views.len(), VIEW_SLOTS);
        prop_assert_eq!(Settings::parse(&s.format()), s);
    }

    /// Reading any text at all never panics and always yields settings
    /// inside their ranges (#473).
    #[test]
    fn any_text_parses_to_valid_settings(text in ".{0,400}") {
        let s = Settings::parse(&text);
        prop_assert!((0.25..=4.0).contains(&s.exposure.get()));
        prop_assert!(s.label_limit.get() <= 24);
        prop_assert!((1..=9).contains(&s.speed.get()));
        prop_assert!((60..=95).contains(&s.dimming.get()) && s.dimming.get().is_multiple_of(5));
    }
}
