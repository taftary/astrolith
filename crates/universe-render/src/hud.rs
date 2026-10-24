//! Persistent scale HUD: level, distance, and scale bar (#157, ADR 0015,
//! #543), plus the top-left mode reminder (#557).
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

/// Marker for the mode reminder line (#557).
#[derive(Component)]
pub(crate) struct ModeReminderText;

/// The reminder text for the current mode (#557, #558). Static text.
#[must_use]
pub(crate) fn reminder_text(mode: crate::input::FlightMode) -> &'static str {
    use universe_core::labels::{ModeName, mode_line};
    mode_line(match mode {
        crate::input::FlightMode::Normal => ModeName::Normal,
        crate::input::FlightMode::Free => ModeName::Free,
        crate::input::FlightMode::Dive => ModeName::Dive,
    })
}

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
    commands.spawn((
        ModeReminderText,
        Name::new("mode-reminder"),
        GlobalZIndex(LAYER_UNIVERSE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(12.0),
            ..default()
        },
        Text::new(reminder_text(crate::input::FlightMode::Dive)),
        TextColor(MENU_TEXT_TINT),
        BackgroundColor(LABEL_HALO),
    ));
}

/// Refreshes the mode reminder: hidden when the setting is off, otherwise
/// the current mode and its keys (#557, #558).
///
/// Writes only on a change; missing node is a no-op.
pub(crate) fn sync_mode_reminder(
    store: Res<crate::SettingsStore>,
    flight: Res<crate::input::Flight>,
    mut reminder: Query<(&mut Text, &mut Visibility), With<ModeReminderText>>,
) {
    let Ok((mut text, mut visibility)) = reminder.single_mut() else {
        return;
    };
    let want_visibility = if store.0.mode_reminder {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *visibility != want_visibility {
        *visibility = want_visibility;
    }
    let want = reminder_text(flight.mode);
    if text.0 != want {
        text.0 = want.to_string();
    }
}

/// Refreshes the HUD readout from the open level.
///
/// Hidden when the Scale HUD switch is off (#956): the whole block goes,
/// readout lines, pull line and ground-memory line together.
///
/// Missing readout (never spawned) is a no-op return, never a panic.
pub(crate) fn sync_hud(
    universe: Res<Universe>,
    flight: Res<crate::input::Flight>,
    normal: Res<crate::player_state::PlayerState>,
    store: Res<crate::SettingsStore>,
    origin: Res<crate::style::RenderOrigin>,
    overlay: Res<crate::debug_overlay::OverlayInfo>,
    mut hud: Query<(&mut Text, &mut Visibility), With<HudText>>,
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
    let mut lines = hud_lines_at(universe.level(), height);
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

    #[test]
    fn reminder_names_each_mode() {
        use crate::input::FlightMode;
        assert!(reminder_text(FlightMode::Dive).starts_with("Dive"));
        assert!(reminder_text(FlightMode::Free).starts_with("Free"));
        assert!(reminder_text(FlightMode::Normal).starts_with("Normal"));
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
}
