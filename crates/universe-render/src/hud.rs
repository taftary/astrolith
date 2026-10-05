//! Persistent scale HUD: level, distance, and scale bar (#157, ADR 0015).
//!
//! The window title already names the level, but titles never reach the
//! headless capture frames the validator reads. This overlay draws the
//! same three facts in-window (Powers of Ten pacing read down the dive,
//! never a timing change): the level name, the distance order, and a bar
//! that grows one segment per rung. Pure text from the frozen ladder, so
//! headless tests pin it without a window.

use crate::Universe;
use bevy::prelude::*;
use bevy::ui::{Node, PositionType, Val, widget::Text};
use universe_core::coords::Level;
use universe_core::labels::{level_label, scale_label};

/// Marker for the HUD readout text block.
#[derive(Component)]
pub(crate) struct HudText;

/// Readout lines for one level: name, distance, and a scale bar.
#[must_use]
pub(crate) fn hud_lines(level: Level) -> [String; 3] {
    let scale_bar = "=".repeat(level.get() as usize);
    [
        level_label(level).to_string(),
        format!("distance {}", scale_label(level)),
        format!("scale [{scale_bar}] {}", scale_label(level)),
    ]
}

/// Spawns the HUD readout (one bottom-left text block).
pub(crate) fn spawn_hud(mut commands: Commands) {
    commands.spawn((
        HudText,
        Name::new("scale-hud"),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            bottom: Val::Px(12.0),
            ..default()
        },
        Text::new(hud_lines(Level::MIN).join("\n")),
    ));
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
    use universe_core::nav::{MAX_NAV_LEVEL, MIN_NAV_LEVEL};

    #[test]
    fn hud_names_level_distance_and_bar() {
        let level = Level::new(10).expect("L10");
        let [name, distance, bar] = hud_lines(level);
        assert!(name.contains("L10"), "level named: {name}");
        assert!(distance.contains("10^7 m"), "distance readout: {distance}");
        assert!(bar.starts_with("scale [==========]"), "ten-rung bar: {bar}");
        assert!(bar.contains("10^7 m"), "bar carries scale: {bar}");
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
