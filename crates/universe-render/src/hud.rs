//! Persistent scale HUD: level, distance, scale bar, and from the planet to
//! the plot level the next level you can dive into (#157, ADR 0015, #543),
//! plus the top-left mode reminder (#557).
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
use universe_core::labels::{level_label, next_level_line, scale_label};

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

/// Readout lines for one level: name, distance, a scale bar, and at
/// L10-L13 a fourth line naming the next level (#543).
#[must_use]
pub(crate) fn hud_lines(level: Level) -> Vec<String> {
    let scale_bar = "=".repeat(level.get() as usize);
    let mut lines = vec![
        level_label(level).to_string(),
        format!("distance {}", scale_label(level)),
        format!("scale [{scale_bar}] {}", scale_label(level)),
    ];
    lines.extend(next_level_line(level));
    lines
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
/// Missing readout (never spawned) is a no-op return, never a panic.
pub(crate) fn sync_hud(universe: Res<Universe>, mut hud: Query<&mut Text, With<HudText>>) {
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    let want = hud_lines(universe.level()).join("\n");
    if text.0 != want {
        text.0 = want;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let [name, distance, bar, next] = lines.as_slice() else {
            panic!("L10 shows four lines: {lines:?}");
        };
        assert_eq!(next, "next: L11 - Regions", "next level named");
        assert!(name.contains("L10"), "level named: {name}");
        assert!(distance.contains("10^7 m"), "distance readout: {distance}");
        assert!(bar.starts_with("scale [==========]"), "ten-rung bar: {bar}");
        assert!(bar.contains("10^7 m"), "bar carries scale: {bar}");
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
