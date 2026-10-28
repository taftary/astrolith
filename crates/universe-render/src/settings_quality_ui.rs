//! Graphics quality in Settings (#1022 T11, Spec v4 S1-S9).
//!
//! Display > Performance holds the mode (S1), the target (S2), the Auto
//! ceiling (S3), the Manual 3D resolution (S4) and the read-only status
//! (S9). Display > Scene gains Atmosphere (S7) and Bloom (S8) beside the
//! existing Planet detail, Shadows and Marker detail (S5, S6): in Manual
//! they show and change the saved choice; in Auto they show the effective
//! value, marked Auto-managed, and a click changes nothing. Display >
//! Window carries the vsync note (S2).

use crate::SettingsStore;
use crate::auto_quality::AutoQuality;
use crate::effective::EffectiveGraphics;
use crate::fonts::TextRole;
use crate::frame_cap::PacingStatus;
use crate::render_scale::RenderScale;
use crate::settings_rows::{spawn_label, spawn_row, spawn_wide_button};
use crate::tokens::{MENU_DIM_TINT, MENU_TEXT_TINT};
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use universe_core::quality::{EffectiveQuality, scaled_size};
use universe_core::quality_controller::Restriction;
use universe_core::settings::{QualityMode, Resolution3d, Settings};

/// Seconds between refreshes of the status while nothing else changed.
const STATUS_REFRESH_S: f32 = 0.5;

/// Lines of the read-only status (S9).
pub(crate) const STATUS_LINES: usize = 5;

/// The suffix of an effective value in Auto (S5-S8).
pub(crate) const AUTO_MANAGED: &str = " (Auto-managed)";

/// A quality setting's button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QualityButton(pub QualityField);

/// A quality setting's value text.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QualityText(pub QualityField);

/// One line of the status.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QualityStatus(pub usize);

/// The quality settings with a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QualityField {
    /// S1 Auto or Manual.
    Mode,
    /// S2 30 or 60 FPS.
    Target,
    /// S3 the Auto ceiling (changes in Auto).
    Ceiling,
    /// S4 the Manual 3D resolution (changes in Manual).
    Resolution,
    /// S7 atmosphere (changes in Manual).
    Atmosphere,
    /// S8 bloom (changes in Manual).
    Bloom,
}

/// Spawns one row: a label and a wide button showing the value.
fn spawn_quality_row(rows: &mut ChildSpawnerCommands, field: QualityField, label: &str, id: &str) {
    spawn_row(rows, |row| {
        spawn_label(row, label);
        let face = |face: &mut ChildSpawnerCommands| {
            face.spawn((
                QualityText(field),
                Text::new(""),
                TextRole::Body,
                TextColor(MENU_TEXT_TINT),
            ));
        };
        spawn_wide_button(row, QualityButton(field), format!("cycle-{id}"), face);
    });
}

/// Display > Performance: S1-S4 and the status (S9).
pub(crate) fn spawn_performance_rows(rows: &mut ChildSpawnerCommands) {
    spawn_quality_row(rows, QualityField::Mode, "Quality", "quality-mode");
    spawn_quality_row(
        rows,
        QualityField::Target,
        "Target frame rate",
        "target-fps",
    );
    spawn_quality_row(
        rows,
        QualityField::Ceiling,
        "Auto quality ceiling",
        "auto-ceiling",
    );
    spawn_quality_row(
        rows,
        QualityField::Resolution,
        "3D resolution",
        "resolution-3d",
    );
    for line in 0..STATUS_LINES {
        rows.spawn((
            QualityStatus(line),
            Text::new(""),
            TextRole::Note,
            TextColor(MENU_DIM_TINT),
        ));
    }
}

/// Display > Scene: Atmosphere (S7) and Bloom (S8).
pub(crate) fn spawn_scene_rows(rows: &mut ChildSpawnerCommands) {
    spawn_quality_row(rows, QualityField::Atmosphere, "Atmosphere", "atmosphere");
    spawn_quality_row(rows, QualityField::Bloom, "Bloom", "bloom");
}

/// Display > Window: the vsync note (S2).
pub(crate) fn spawn_vsync_note(rows: &mut ChildSpawnerCommands) {
    rows.spawn((
        Text::new(
            "Vsync matches frames to your display. Its refresh rate can keep the frame rate target from being paced evenly; Performance explains when.",
        ),
        TextRole::Note,
        TextColor(MENU_DIM_TINT),
    ));
}

/// Whether `field` changes in `mode` (pure): the ceiling in Auto, the
/// resolution and the scene effects in Manual, the mode and target always.
#[must_use]
pub(crate) const fn editable(field: QualityField, mode: QualityMode) -> bool {
    match field {
        QualityField::Mode | QualityField::Target => true,
        QualityField::Ceiling => matches!(mode, QualityMode::Auto),
        QualityField::Resolution | QualityField::Atmosphere | QualityField::Bloom => {
            matches!(mode, QualityMode::Manual)
        }
    }
}

/// Steps `field` of `saved` once, for a window whose shorter side is
/// `native_short` pixels; nothing when it does not change in this mode.
pub(crate) fn click(saved: &mut Settings, field: QualityField, native_short: u32) {
    if !editable(field, saved.quality_mode) {
        return;
    }
    match field {
        QualityField::Mode => saved.quality_mode = saved.quality_mode.next(),
        QualityField::Target => saved.target_fps = saved.target_fps.next(),
        QualityField::Ceiling => saved.auto_ceiling = saved.auto_ceiling.next(),
        QualityField::Resolution => {
            saved.resolution_3d = saved.resolution_3d.next_offered(native_short);
        }
        QualityField::Atmosphere => saved.atmosphere = saved.atmosphere.next(),
        QualityField::Bloom => saved.bloom = !saved.bloom,
    }
}

/// On or off.
const fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

/// A scene value: the saved one in Manual, the effective one marked
/// Auto-managed in Auto (S5-S8, pure).
#[must_use]
pub(crate) fn scene_value(mode: QualityMode, saved: &str, effective: &str) -> String {
    match mode {
        QualityMode::Manual => saved.to_string(),
        QualityMode::Auto => format!("{effective}{AUTO_MANAGED}"),
    }
}

/// The physical size a Manual `choice` draws at in a `native` window.
#[must_use]
pub(crate) fn resolution_text(choice: Resolution3d, native: (u32, u32)) -> String {
    let short = native.0.min(native.1).max(1);
    let scale = crate::render_scale::manual_scale(short, choice);
    let (w, h) = scaled_size(native, scale);
    match choice {
        Resolution3d::Native => format!("Native ({w} x {h})"),
        _ if scale >= 1.0 => format!("Native ({w} x {h})"),
        _ => format!("{w} x {h}"),
    }
}

/// The value text of `field`.
fn field_text(
    field: QualityField,
    saved: &Settings,
    effective: &EffectiveQuality,
    native: (u32, u32),
) -> String {
    let mode = saved.quality_mode;
    match field {
        QualityField::Mode => saved.quality_mode.display().to_string(),
        QualityField::Target => saved.target_fps.display().to_string(),
        QualityField::Ceiling => saved.auto_ceiling.display().to_string(),
        QualityField::Resolution => resolution_text(saved.resolution_3d, native),
        QualityField::Atmosphere => scene_value(
            mode,
            saved.atmosphere.display(),
            effective.atmosphere.display(),
        ),
        QualityField::Bloom => scene_value(mode, on_off(saved.bloom), on_off(effective.bloom)),
    }
}

/// What the status reads from Auto: the restriction and the limit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AutoFacts {
    /// What holds the quality below the ceiling.
    pub restriction: Restriction,
    /// Every useful reduction is used up.
    pub limited: bool,
    /// At the picture floor.
    pub at_floor: bool,
}

/// The status lines (S9, pure): the drawn size, the effective tiers, the
/// target, the floor and ceiling, what limits the quality, and the
/// display's pacing note.
#[must_use]
pub(crate) fn status_lines(
    saved: &Settings,
    effective: &EffectiveQuality,
    drawn: (u32, u32),
    native: (u32, u32),
    auto: Option<AutoFacts>,
    note: Option<&str>,
) -> [String; STATUS_LINES] {
    let size = format!(
        "Drawing at {} x {} of {} x {}.",
        drawn.0, drawn.1, native.0, native.1
    );
    let tiers = format!(
        "Now: terrain {}, markers {}, atmosphere {}, shadows {}, bloom {}.",
        effective.terrain.display(),
        effective.markers.display(),
        effective.atmosphere.display(),
        on_off(effective.shadows),
        on_off(effective.bloom)
    );
    let target = format!("Target {} FPS.", saved.target_fps.fps());
    let (state, limits) = match (saved.quality_mode, auto) {
        (QualityMode::Manual, _) => (
            "Manual: your saved choices.".to_string(),
            "Ground-memory protection still applies.".to_string(),
        ),
        (QualityMode::Auto, None) => (
            "Auto: not measuring in this run.".to_string(),
            String::new(),
        ),
        (QualityMode::Auto, Some(facts)) => {
            let state = if facts.at_floor {
                "Auto: at the minimum picture quality."
            } else if facts.restriction == Restriction::None {
                "Auto: at the ceiling."
            } else {
                "Auto: between the minimum and the ceiling."
            };
            let held = match facts.restriction {
                Restriction::None => "Nothing is limiting the quality.",
                Restriction::Memory => "Limited by memory.",
                Restriction::Performance if facts.limited => {
                    "Limited by performance: no further reduction helps."
                }
                Restriction::Performance => "Limited by performance.",
                Restriction::Both => "Limited by performance and memory.",
            };
            (state.to_string(), held.to_string())
        }
    };
    let target = match note {
        Some(note) => format!("{target} {note}"),
        None => target,
    };
    [size, tiers, target, state, limits]
}

/// Steps a quality setting on a press.
fn quality_click(
    mut store: ResMut<SettingsStore>,
    windows: Query<&Window, With<PrimaryWindow>>,
    buttons: Query<(&Interaction, &QualityButton), Changed<Interaction>>,
) {
    let short = windows
        .single()
        .map_or(0, |w| w.physical_width().min(w.physical_height()));
    for (interaction, button) in &buttons {
        if *interaction == Interaction::Pressed {
            click(&mut store.0, button.0, short);
        }
    }
}

/// What the status reads besides the settings: the drawn scale, the
/// display's pacing and Auto's controller (each absent in some runs).
type StatusSources<'w> = (
    Option<Res<'w, RenderScale>>,
    Option<Res<'w, PacingStatus>>,
    Option<Res<'w, AutoQuality>>,
);

/// Rewrites the value texts and the status when anything they show
/// changed, and the status at most twice a second otherwise.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
fn sync_quality_texts(
    store: Res<SettingsStore>,
    effective: Res<EffectiveGraphics>,
    (scale, pacing, auto): StatusSources,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time<Real>>,
    mut since: Local<f32>,
    mut values: Query<(&QualityText, &mut Text), Without<QualityStatus>>,
    mut status: Query<(&QualityStatus, &mut Text), Without<QualityText>>,
) {
    *since += time.delta_secs();
    let changed = store.is_changed()
        || effective.is_changed()
        || scale.as_ref().is_some_and(|s| s.is_changed());
    if !changed && *since < STATUS_REFRESH_S {
        return;
    }
    *since = 0.0;
    let native = windows
        .single()
        .map_or((0, 0), |w| (w.physical_width(), w.physical_height()));
    for (value, mut text) in &mut values {
        let want = field_text(value.0, &store.0, &effective.0, native);
        if text.0 != want {
            text.0 = want;
        }
    }
    let facts = auto.as_ref().map(|a| AutoFacts {
        restriction: a.controller.restriction(),
        limited: a.controller.performance_limited(),
        at_floor: a.controller.quality().at_floor(a.controller.limits()),
    });
    let note = pacing.as_ref().and_then(|p| p.note.as_deref());
    let lines = status_lines(
        &store.0,
        &effective.0,
        scaled_size(native, scale.map_or(1.0, |s| s.0)),
        native,
        facts,
        note,
    );
    for (line, mut text) in &mut status {
        let want = lines.get(line.0).cloned().unwrap_or_default();
        if text.0 != want {
            text.0 = want;
        }
    }
}

/// The graphics quality settings' systems.
pub(crate) struct SettingsQualityPlugin;

impl Plugin for SettingsQualityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (quality_click, sync_quality_texts).chain());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use universe_core::settings::Tier;

    #[test]
    fn each_setting_changes_only_in_its_mode() {
        let mut s = Settings::defaults();
        assert_eq!(s.quality_mode, QualityMode::Auto);
        click(&mut s, QualityField::Resolution, 900);
        click(&mut s, QualityField::Bloom, 900);
        click(&mut s, QualityField::Atmosphere, 900);
        assert_eq!(s.resolution_3d, Resolution3d::Native, "Manual only");
        assert!(!s.bloom);
        assert_eq!(s.atmosphere, Tier::Normal);
        click(&mut s, QualityField::Ceiling, 900);
        assert_eq!(s.auto_ceiling, Tier::High, "the ceiling changes in Auto");
        click(&mut s, QualityField::Target, 900);
        assert_eq!(s.target_fps.fps(), 30);
        click(&mut s, QualityField::Mode, 900);
        assert_eq!(s.quality_mode, QualityMode::Manual);
        click(&mut s, QualityField::Ceiling, 900);
        assert_eq!(s.auto_ceiling, Tier::High, "not in Manual");
        // 1600 x 900 offers 720 and Native (Q25).
        click(&mut s, QualityField::Resolution, 900);
        assert_eq!(s.resolution_3d, Resolution3d::Short720);
        click(&mut s, QualityField::Resolution, 900);
        assert_eq!(s.resolution_3d, Resolution3d::Native);
        click(&mut s, QualityField::Bloom, 900);
        assert!(s.bloom);
    }

    #[test]
    fn values_show_the_saved_choice_in_manual_and_the_effective_in_auto() {
        assert_eq!(scene_value(QualityMode::Manual, "High", "Low"), "High");
        assert_eq!(
            scene_value(QualityMode::Auto, "High", "Low"),
            "Low (Auto-managed)"
        );
        assert_eq!(
            resolution_text(Resolution3d::Short720, (1600, 900)),
            "1280 x 720"
        );
        assert_eq!(
            resolution_text(Resolution3d::Native, (1600, 900)),
            "Native (1600 x 900)"
        );
        assert_eq!(
            resolution_text(Resolution3d::Short1080, (1600, 900)),
            "Native (1600 x 900)",
            "no size above native"
        );
    }

    #[test]
    fn the_status_names_size_tiers_target_and_limits() {
        let saved = Settings::defaults();
        let q = crate::effective::manual_quality(&saved);
        let facts = AutoFacts {
            restriction: Restriction::Both,
            limited: false,
            at_floor: false,
        };
        let lines = status_lines(
            &saved,
            &q,
            (1280, 720),
            (1600, 900),
            Some(facts),
            Some("Uneven."),
        );
        assert_eq!(lines[0], "Drawing at 1280 x 720 of 1600 x 900.");
        assert!(
            lines[1].starts_with("Now: terrain Normal, markers Normal"),
            "{}",
            lines[1]
        );
        assert_eq!(lines[2], "Target 60 FPS. Uneven.");
        assert_eq!(lines[3], "Auto: between the minimum and the ceiling.");
        assert_eq!(lines[4], "Limited by performance and memory.");
        let manual = Settings {
            quality_mode: QualityMode::Manual,
            ..saved
        };
        let lines = status_lines(&manual, &q, (1600, 900), (1600, 900), None, None);
        assert_eq!(lines[3], "Manual: your saved choices.");
    }
}
