//! Persistent scale HUD: level, distance, and scale bar (#157, ADR 0015,
//! #543). The top-left mode control lives in [`crate::mode_switch`].
//!
//! The window title already names the level, but titles never reach the
//! headless capture frames the validator reads. This overlay draws the
//! same three facts in-window (Powers of Ten pacing read down the dive,
//! never a timing change): the level name, the distance order, and a bar
//! that grows one segment per rung. Pure text from the frozen ladder, so
//! headless tests pin it without a window.

use crate::Universe;
use crate::tokens::{LABEL_HALO, LAYER_UNIVERSE, MENU_TEXT_TINT};
use bevy::prelude::*;
use bevy::ui::{Node, PositionType, Val, widget::Text};
use universe_core::coords::Level;
use universe_core::labels::{level_label, scale_label};

/// Marker for the HUD readout text block.
#[derive(Component)]
pub(crate) struct HudText;

/// Readout lines for one level: name, distance, and a scale bar (#543,
/// #729: no next line, nothing opens below any level anymore).
#[must_use]
pub(crate) fn hud_lines(level: Level) -> Vec<String> {
    let scale_bar = "=".repeat(level.get() as usize);
    vec![
        level_label(level).to_string(),
        format!("distance {}", scale_label(level)),
        format!("scale [{scale_bar}] {}", scale_label(level)),
    ]
}

/// Readout lines at the planet for a camera `height_m` above the real
/// ground (#892 T3, Q4 B, Q5 A): from L11 down to L14 the panel names the
/// height band, the height above the ground and the scale from it; above
/// 1,000 km (and off the planet, `None`) it is [`hud_lines`] as before.
#[must_use]
pub(crate) fn hud_lines_at(level: Level, height_m: Option<f64>) -> Vec<String> {
    use universe_core::labels::{ground_label, ground_level, ground_scale_label, height_line};
    let Some(h) = height_m.filter(|_| level.get() == 10) else {
        return hud_lines(level);
    };
    let band = ground_level(h);
    if band <= 10 {
        return hud_lines(level);
    }
    let scale_bar = "=".repeat(usize::from(band));
    vec![
        ground_label(band).to_string(),
        height_line(h),
        format!("scale [{scale_bar}] {}", ground_scale_label(h)),
    ]
}

/// Body names of the open cell, kept until it changes (#995 T10).
#[derive(Default)]
pub(crate) struct NameCache {
    /// Open-cell seed the names belong to.
    seed: Option<u64>,
    /// Marker and name pairs.
    names: Vec<(u32, String)>,
}

/// In one star system (#995 T10, Q2 A, Q4 A): the level the view is named
/// by (L9 near the star, else the open level) and the nearest body line,
/// from the camera's place. `None` outside a planetary system or its
/// planets.
#[must_use]
pub(crate) fn system_hud(
    universe: &universe_core::nav::Universe,
    camera: [f64; 3],
    names: &mut NameCache,
) -> Option<(Level, String)> {
    use universe_core::labels::{nearest_line, system_level};
    use universe_core::names::marker_name;
    let level = universe.level();
    if !matches!(level.get(), 8 | 10) {
        return None;
    }
    // Names cost a walk of the journey's cells: one per body per open
    // cell, never per frame (`E-HOT-NOALLOC`).
    if names.seed != Some(universe.open_seed()) {
        names.seed = Some(universe.open_seed());
        names.names.clear();
    }
    let chain = universe.path.indices();
    let mut name_of =
        |chain: &[u32], level: Level, marker: u32, point: &universe_core::r#gen::Point| {
            if let Some((_, name)) = names.names.iter().find(|(m, _)| *m == marker) {
                return name.clone();
            }
            let name = marker_name(universe.root.0, chain, level, marker, point);
            names.names.push((marker, name.clone()));
            name
        };
    let distance = |p: [f64; 3]| {
        ((camera[0] - p[0]).powi(2) + (camera[1] - p[1]).powi(2) + (camera[2] - p[2]).powi(2))
            .sqrt()
    };
    match level.get() {
        8 => {
            let mpc = universe_core::orbits::metres_per_cell(8);
            let star = universe.open.points.first().filter(|p| p.star.is_some())?;
            let named = system_level(level, distance(star.position) * mpc, star.radius * mpc);
            let (marker, point, gap) = (0u32..)
                .zip(&universe.open.points)
                .filter(|(_, p)| p.planet.is_some() || p.star.is_some())
                .map(|(m, p)| (m, p, (distance(p.position) - p.radius) * mpc))
                .min_by(|a, b| a.2.total_cmp(&b.2))?;
            let name = name_of(&chain, level, marker, point);
            Some((named, nearest_line(&name, gap)))
        }
        10 => {
            let frame = universe_core::nav::planet_frame(universe)?;
            let mpc = frame.metres_per_cell();
            let (planet_marker, close_up) = chain.split_last()?;
            let planet_name = name_of(
                close_up,
                Level::new(9).unwrap_or(level),
                *planet_marker,
                universe.open.points.first()?,
            );
            let mut nearest = (
                planet_name,
                (distance([0.0; 3]) - frame.radius_cells()) * mpc,
            );
            for point in universe.open.points.iter().filter(|p| p.moon.is_some()) {
                let gap = (distance(point.position) - point.radius) * mpc;
                if gap < nearest.1 {
                    nearest = (String::from("moon"), gap);
                }
            }
            Some((level, nearest_line(&nearest.0, nearest.1)))
        }
        _ => None,
    }
}

/// The F3 ground memory line (#918 AC5): bytes held over the limit, in
/// megabytes of 2^20 bytes, as the limit is set. Rounded down, so the line
/// never shows more than is held (63.99 MB reads 63.9, not 64.0).
#[must_use]
pub(crate) fn ground_memory_line(bytes: usize, cap_bytes: usize) -> String {
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: byte counts shown to 0.1 MB, intended"
    )]
    let mb = |b: usize| b as f64 / f64::from(1u32 << 20);
    let held = (mb(bytes) * 10.0).floor() / 10.0;
    format!("ground memory {held:.1} / {:.0} MB", mb(cap_bytes))
}

/// Height of the rendered camera above the real ground at L10 (#892 T3),
/// in every mode (dive, free, and walking: the posed eye). Pure.
#[must_use]
pub(crate) fn camera_height_m(
    universe: &universe_core::nav::Universe,
    camera: [f64; 3],
) -> Option<f64> {
    if universe.level().get() != 10 {
        return None;
    }
    let frame = universe_core::nav::planet_frame(universe)?;
    let relief = universe_core::nav::planet_relief(universe)?;
    let h = relief.height_above_m(frame.from_cell(camera));
    h.is_finite().then_some(h)
}

/// Spawns the HUD readout (one bottom-left text block).
pub(crate) fn spawn_hud(mut commands: Commands) {
    // White text on a halo: the default text color is dark and vanishes
    // against the near-black sky (owner report); the halo keeps it readable
    // over bright markers too. Both tints are existing tokens.
    commands.spawn((
        HudText,
        Name::new("scale-hud"),
        GlobalZIndex(LAYER_UNIVERSE),
        Visibility::Hidden,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            bottom: Val::Px(12.0),
            ..default()
        },
        Text::new(hud_lines(Level::MIN).join("\n")),
        TextColor(MENU_TEXT_TINT),
        BackgroundColor(LABEL_HALO),
    ));
}

/// Refreshes the HUD readout from the open level.
///
/// Hidden when the Scale HUD switch is off (#956): the whole block goes,
/// readout lines, pull line and ground-memory line together.
///
/// Missing readout (never spawned) is a no-op return, never a panic.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn sync_hud(
    universe: Res<Universe>,
    flight: Res<crate::input::Flight>,
    normal: Res<crate::player_state::PlayerState>,
    store: Res<crate::SettingsStore>,
    origin: Res<crate::style::RenderOrigin>,
    overlay: Res<crate::debug_overlay::OverlayInfo>,
    mut hud: Query<(&mut Text, &mut Visibility), With<HudText>>,
    mut names: Local<NameCache>,
) {
    let Ok((mut text, mut visibility)) = hud.single_mut() else {
        return;
    };
    let want_visibility = if store.0.scale_hud {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *visibility != want_visibility {
        *visibility = want_visibility;
    }
    // At the planet the panel follows the rendered camera's height above
    // the real ground (#892 T3).
    let height = camera_height_m(&universe, origin.camera);
    // In one star system the view is named by distance and the nearest
    // body is shown (#995 T10); the ground bands below keep their lines.
    let system = if store.0.scale_hud {
        system_hud(&universe, origin.camera, &mut names)
    } else {
        None
    };
    let named = system
        .as_ref()
        .map_or(universe.level(), |(level, _)| *level);
    let mut lines = hud_lines_at(named, height);
    if let Some((_, nearest)) = system
        && lines.first().map(String::as_str) == Some(level_label(named))
    {
        lines.push(nearest);
    }
    // The strongest pull in normal mode (#559), hidden when almost zero or
    // when Show pull is off.
    if flight.mode == crate::input::FlightMode::Normal
        && normal.active
        && store.0.show_pull
        && normal.pull_strength >= universe_core::gravity::FAINT_PULL
    {
        lines.push(universe_core::labels::pull_line(
            &normal.pull_name,
            normal.pull_strength,
        ));
    }
    // With F3 on at the planet, the ground memory and its limit (#918 AC5).
    if overlay.enabled && universe.level().get() == 10 {
        lines.push(ground_memory_line(
            overlay.ground_bytes,
            overlay.ground_cap_bytes,
        ));
    }
    let want = lines.join("\n");
    if text.0 != want {
        text.0 = want;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_memory_line_reads_in_megabytes() {
        // #918 AC5: the F3 line the owner reads the cap from.
        assert_eq!(
            ground_memory_line(43_253_760, 64 << 20),
            "ground memory 41.2 / 64 MB"
        );
        assert_eq!(ground_memory_line(0, 32 << 20), "ground memory 0.0 / 32 MB");
        assert_eq!(
            ground_memory_line((64 << 20) - 10_000, 64 << 20),
            "ground memory 63.9 / 64 MB",
            "under the limit never reads as the limit"
        );
    }
    use universe_core::nav::{MAX_NAV_LEVEL, MIN_NAV_LEVEL};

    #[test]
    fn hud_names_level_distance_and_bar() {
        let level = Level::new(10).expect("L10");
        let lines = hud_lines(level);
        let [name, distance, bar] = lines.as_slice() else {
            panic!("L10 shows three lines, no next: {lines:?}");
        };
        assert!(name.contains("L10"), "level named: {name}");
        assert!(distance.contains("10^7 m"), "distance readout: {distance}");
        assert!(bar.starts_with("scale [==========]"), "ten-rung bar: {bar}");
        assert!(bar.contains("10^7 m"), "bar carries scale: {bar}");
    }

    #[test]
    fn panel_follows_the_height() {
        // C4 (#892 T3, Q4 B, Q5 A): at 2,000 km, 500 km, 50 km, 500 m and
        // 5 m above the ground the panel shows L10 to L14 with the names,
        // the height and the scale; the same rule serves every mode since
        // it reads the rendered camera (dive, free, the walking eye).
        let l10 = Level::new(10).expect("L10");
        let today = hud_lines(l10);
        assert_eq!(
            hud_lines_at(l10, Some(2_000_000.0)),
            today,
            "above 1,000 km as today"
        );
        assert_eq!(hud_lines_at(l10, None), today, "off the planet as today");
        let cases = [
            (
                500_000.0,
                "L11 - High above the planet",
                "height 500 km above ground",
                "10^6 m",
                11,
            ),
            (
                50_000.0,
                "L12 - Above the land",
                "height 50.0 km above ground",
                "10^5 m",
                12,
            ),
            (
                500.0,
                "L13 - Close to the ground",
                "height 500 m above ground",
                "10^3 m",
                13,
            ),
            (
                5.0,
                "L14 - On the ground",
                "height 5 m above ground",
                "10^1 m",
                14,
            ),
        ];
        for (h, name, height, scale, rungs) in cases {
            let lines = hud_lines_at(l10, Some(h));
            let [n, hl, bar] = lines.as_slice() else {
                panic!("three lines at {h} m: {lines:?}");
            };
            assert_eq!(n, name);
            assert_eq!(hl, height);
            assert!(
                bar.starts_with(&format!("scale [{}]", "=".repeat(rungs))),
                "{bar}"
            );
            assert!(bar.ends_with(scale), "{bar}");
        }
        // Other levels never read a height.
        let l9 = Level::new(9).expect("L9");
        assert_eq!(hud_lines_at(l9, Some(5.0)), hud_lines(l9));
        // The camera height comes from the real ground at L10.
        let (_, universe) =
            universe_core::nav::replay_autopilot(universe_core::nav::DEMO_SEED, 1.0 / 60.0, 600.0);
        let frame = universe_core::nav::planet_frame(&universe).expect("frame");
        let relief = universe_core::nav::planet_relief(&universe).expect("relief");
        let dir = [0.0, 1.0, 0.0];
        let r = (frame.radius_m() + relief.height_m(dir) + 500.0) / frame.metres_per_cell();
        let h = camera_height_m(&universe, [0.0, r, 0.0]).expect("height at L10");
        assert!((h - 500.0).abs() < 1e-3, "500 m above the ground: {h}");
    }

    #[test]
    fn scale_hud_starts_hidden_and_the_switch_shows_it() {
        // #956 T3: the whole block follows the switch with no restart.
        use crate::window_tests::headless_app;
        let mut app = headless_app();
        app.update();
        let read = |app: &mut App| {
            app.world_mut()
                .query_filtered::<&Visibility, With<HudText>>()
                .single(app.world())
                .copied()
                .expect("one readout")
        };
        assert_eq!(read(&mut app), Visibility::Hidden, "hidden by default");
        app.world_mut()
            .resource_mut::<crate::SettingsStore>()
            .0
            .scale_hud = true;
        app.update();
        assert_eq!(read(&mut app), Visibility::Inherited, "on shows the block");
        app.world_mut()
            .resource_mut::<crate::SettingsStore>()
            .0
            .scale_hud = false;
        app.update();
        assert_eq!(read(&mut app), Visibility::Hidden, "off hides everything");
    }

    #[test]
    fn hud_text_is_white_on_a_halo() {
        // The readout floats over the near-black sky and bright markers: the
        // default dark text color vanishes, so white-on-halo is required.
        let mut app = App::new();
        app.add_systems(Startup, spawn_hud);
        app.update();
        let found: Vec<(TextColor, BackgroundColor)> = app
            .world_mut()
            .query_filtered::<(&TextColor, &BackgroundColor), With<HudText>>()
            .iter(app.world())
            .map(|(color, halo)| (*color, *halo))
            .collect();
        assert_eq!(found.len(), 1, "one hud readout exists");
        assert_eq!(
            found[0].0,
            TextColor(MENU_TEXT_TINT),
            "hud reads white on the dark sky"
        );
        assert_eq!(
            found[0].1,
            BackgroundColor(LABEL_HALO),
            "halo keeps it readable over bright markers"
        );
    }

    #[test]
    fn hud_covers_every_rung_without_reserved_words() {
        for n in MIN_NAV_LEVEL..=MAX_NAV_LEVEL {
            let level = Level::new(n).expect("ladder level");
            let lines = hud_lines(level).join("\n");
            assert!(lines.contains(&format!("L{n}")), "rung named: {lines}");
            for word in ["error", "panic", "warn", "deprecat", "FAILED"] {
                assert!(!lines.contains(word), "reserved word leaked: {word}");
            }
        }
    }

    #[test]
    fn the_system_hud_names_the_level_by_distance_and_the_nearest_body() {
        // #995 T10 (Q2 A, Q4 A): beside Earth in the planetary system the
        // view is L8 and Earth is nearest; close to the Sun it is L9.
        use universe_core::nav::{DEMO_SEED, MarkerIndex, Universe as Core};
        let mut universe = Core::new(DEMO_SEED);
        for &marker in universe_core::nest::autopilot_path(DEMO_SEED)
            .get(..7)
            .expect("journey reaches L8")
        {
            assert!(universe.open(MarkerIndex(marker)));
        }
        let planet = universe.journey_target().expect("Earth");
        let at = universe.marker(MarkerIndex(planet)).expect("Earth");
        let beside = [at[0], at[1] + 1e-4, at[2]];
        let mut names = NameCache::default();
        let (level, line) = system_hud(&universe, beside, &mut names).expect("system");
        assert_eq!(level.get(), 8);
        assert!(line.starts_with("nearest Earth "), "{line}");
        let star = universe.open.points.first().expect("Sun").radius;
        let (level, line) =
            system_hud(&universe, [0.0, 5.0 * star, 0.0], &mut names).expect("system");
        assert_eq!(level.get(), 9);
        assert!(line.starts_with("nearest Sun "), "{line}");
    }

    #[test]
    fn the_readout_names_each_level_passed_in_and_out() {
        // #998 Spec v2 (Q2 A): no portal, the level name is a readout that
        // follows the level being passed, in order, both ways. Labels stay
        // on the nearest level (the open cell's portals, up to the label
        // limit; see `labels`).
        use universe_core::nav::{AUTOPILOT_RATE, DEMO_SEED, DiveEvent, DiveMode};
        let mut universe = universe_core::nav::Universe::new(DEMO_SEED);
        let factor = (-AUTOPILOT_RATE / 60.0).exp();
        let mut seen = vec![hud_lines(universe.level())[0].clone()];
        let mut target = None;
        for _ in 0..40_000 {
            if universe.level().get() >= 7 {
                break;
            }
            if target.is_none() {
                target = universe.journey_target();
            }
            match universe.dive(target, factor, DiveMode::Journey) {
                DiveEvent::Opened(_) => {
                    target = None;
                    seen.push(hud_lines(universe.level())[0].clone());
                }
                DiveEvent::Closed(opened) => target = Some(opened.marker),
                DiveEvent::Moved => {}
            }
        }
        while !universe.path.is_at_root() {
            if let DiveEvent::Closed(_) = universe.dive(None, 1.0 / factor, DiveMode::Passing) {
                seen.push(hud_lines(universe.level())[0].clone());
            }
        }
        let names: Vec<String> = [1u8, 2, 3, 4, 5, 6, 7, 6, 5, 4, 3, 2, 1]
            .iter()
            .map(|n| level_label(Level::new(*n).expect("level")).to_string())
            .collect();
        assert_eq!(seen, names);
    }
}
