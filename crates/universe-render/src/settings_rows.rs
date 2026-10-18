//! Settings row builders in the instrument-panel look (#473 Spec v3).
//!
//! Split out of `settings_ui.rs` (`E-FILE-SIZE`). Each setting is one
//! horizontal row: its label, then double-border chrome buttons and the
//! current value. The components stay in `settings_ui.rs`, which owns the
//! clicks and the value text.

use crate::chrome;
use crate::fonts::TextRole;
use crate::settings_ui::{
    SettingBodyColour, SettingClear, SettingLoader, SettingReset, SettingSection, SettingStep,
    SettingToggle, SettingValue, SettingValueKind, StepField, ToggleField,
};
use crate::tokens::{
    MENU_DIM_TINT, MENU_MARGIN_PX, MENU_TEXT_TINT, RESET_BUTTON_WIDTH_PX, ROW_GAP_PX,
    SCREEN_BUTTON_HEIGHT_PX, SETTING_LABEL_WIDTH_PX, SMALL_BUTTON_WIDTH_PX,
};
use crate::views_ui::{SettingGoView, SettingSaveView};
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use universe_core::settings::Category;

/// Spawns one section (hidden until its leaf is selected) with its rows.
///
/// Hidden means out of layout too (`Display::None`): visibility alone keeps
/// the section's space and stacks every page (#518).
pub(crate) fn spawn_section(
    sections: &mut ChildSpawnerCommands,
    leaves: &'static [&'static str],
    name: &str,
    rows: impl FnOnce(&mut ChildSpawnerCommands),
) {
    sections
        .spawn((
            SettingSection { leaves },
            Name::new(format!("settings-{name}")),
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(ROW_GAP_PX),
                padding: UiRect::all(Val::Px(MENU_MARGIN_PX)),
                display: Display::None,
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|section| {
            rows(section);
        });
}

/// Spawns one horizontal row and fills it.
pub(crate) fn spawn_row(
    rows: &mut ChildSpawnerCommands,
    fill: impl FnOnce(&mut ChildSpawnerCommands),
) {
    rows.spawn((
        Name::new("settings-row"),
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(ROW_GAP_PX),
            min_height: Val::Px(SCREEN_BUTTON_HEIGHT_PX),
            ..default()
        },
    ))
    .with_children(fill);
}

/// Row label text at the fixed label width.
fn spawn_label(row: &mut ChildSpawnerCommands, label: &str) {
    row.spawn((
        Text::new(label),
        TextRole::Body,
        TextColor(MENU_TEXT_TINT),
        Node {
            width: Val::Px(SETTING_LABEL_WIDTH_PX),
            ..default()
        },
    ));
}

/// Small double-border button carrying `face` content.
fn spawn_small_button(
    row: &mut ChildSpawnerCommands,
    marker: impl Bundle,
    name: String,
    face: impl FnOnce(&mut ChildSpawnerCommands),
) {
    row.spawn((
        marker,
        Name::new(name),
        chrome::button(
            Val::Px(SMALL_BUTTON_WIDTH_PX),
            Val::Px(SCREEN_BUTTON_HEIGHT_PX),
            false,
        ),
    ))
    .with_children(|button| {
        button.spawn(chrome::button_face(false)).with_children(face);
    });
}

/// Wider double-border button (Save here), carrying `face` content.
fn spawn_wide_button(
    row: &mut ChildSpawnerCommands,
    marker: impl Bundle,
    name: String,
    face: impl FnOnce(&mut ChildSpawnerCommands),
) {
    row.spawn((
        marker,
        Name::new(name),
        chrome::button(
            Val::Px(2.0 * SMALL_BUTTON_WIDTH_PX),
            Val::Px(SCREEN_BUTTON_HEIGHT_PX),
            false,
        ),
    ))
    .with_children(|button| {
        button.spawn(chrome::button_face(false)).with_children(face);
    });
}

/// Spawns one toggle row: label, on/off button, and loader note.
pub(crate) fn spawn_toggle(rows: &mut ChildSpawnerCommands, field: ToggleField, label: &str) {
    let kind = match field {
        ToggleField::Labels => SettingValueKind::Labels,
        ToggleField::GroundGuides => SettingValueKind::GroundGuides,
        ToggleField::ModeReminder => SettingValueKind::ModeReminder,
        ToggleField::ShowPull => SettingValueKind::ShowPull,
        ToggleField::InvertLook => SettingValueKind::InvertLook,
        ToggleField::DiveToggle => SettingValueKind::DiveToggle,
        ToggleField::LookToggle => SettingValueKind::LookToggle,
        ToggleField::PlaneTurning => SettingValueKind::PlaneTurning,
        ToggleField::StartThirdPerson => SettingValueKind::StartThirdPerson,
        ToggleField::MovementInertia => SettingValueKind::MovementInertia,
        ToggleField::Fullscreen => SettingValueKind::Fullscreen,
        ToggleField::Vsync => SettingValueKind::Vsync,
        ToggleField::DebugAxes => SettingValueKind::DebugAxes,
        ToggleField::TouchAlways => SettingValueKind::TouchAlways,
        ToggleField::LeftHand => SettingValueKind::LeftHand,
        ToggleField::SimulateTouch => SettingValueKind::SimulateTouch,
    };
    spawn_row(rows, |row| {
        spawn_label(row, label);
        let face = |face: &mut ChildSpawnerCommands| {
            face.spawn((
                SettingValue(kind),
                Text::new(""),
                TextRole::Body,
                TextColor(MENU_TEXT_TINT),
            ));
        };
        let name = format!("toggle-{label}");
        // "automatic" needs the wider button; Hand matches it (#561).
        if matches!(field, ToggleField::TouchAlways | ToggleField::LeftHand) {
            spawn_wide_button(row, SettingToggle(field), name, face);
        } else {
            spawn_small_button(row, SettingToggle(field), name, face);
        }
        if matches!(field, ToggleField::Fullscreen | ToggleField::Vsync) {
            row.spawn((
                SettingLoader(field),
                Text::new(""),
                TextRole::Note,
                TextColor(MENU_DIM_TINT),
            ));
        }
    });
}

/// Spawns one stepper row: label, `-`, the value, `+`.
pub(crate) fn spawn_stepper(rows: &mut ChildSpawnerCommands, field: StepField, label: &str) {
    let kind = match field {
        StepField::Exposure => SettingValueKind::Exposure,
        StepField::LabelLimit => SettingValueKind::LabelLimit,
        StepField::Dimming => SettingValueKind::Dimming,
        StepField::Speed => SettingValueKind::Speed,
        StepField::LookSensitivity => SettingValueKind::LookSensitivity,
        StepField::CameraSmoothing => SettingValueKind::CameraSmoothing,
    };
    spawn_row(rows, |row| {
        spawn_label(row, label);
        for (name, dir) in [("minus", -1), ("plus", 1)] {
            spawn_small_button(
                row,
                SettingStep { field, dir },
                format!("step-{label}-{name}"),
                |face| {
                    face.spawn((
                        Text::new(if dir > 0 { "+" } else { "-" }),
                        TextRole::Body,
                        TextColor(MENU_TEXT_TINT),
                    ));
                },
            );
        }
        row.spawn((
            SettingValue(kind),
            Text::new(""),
            TextRole::Body,
            TextColor(MENU_TEXT_TINT),
        ));
    });
}

/// Spawns the Body colour row (#755, S1): label and one button cycling presets.
pub(crate) fn spawn_body_colour(rows: &mut ChildSpawnerCommands) {
    spawn_row(rows, |row| {
        spawn_label(row, "Body colour");
        let face = |face: &mut ChildSpawnerCommands| {
            face.spawn((
                SettingValue(SettingValueKind::BodyColour),
                Text::new(""),
                TextRole::Body,
                TextColor(MENU_TEXT_TINT),
            ));
        };
        spawn_wide_button(
            row,
            SettingBodyColour,
            "cycle-body-colour".to_string(),
            face,
        );
    });
}

/// Spawns one saved-view row: the slot text and its Clear button.
pub(crate) fn spawn_view_slot(rows: &mut ChildSpawnerCommands, slot: usize) {
    spawn_row(rows, |row| {
        row.spawn((
            SettingValue(SettingValueKind::ViewSlot(slot)),
            Text::new(""),
            TextRole::Body,
            TextColor(MENU_TEXT_TINT),
            Node {
                width: Val::Px(SETTING_LABEL_WIDTH_PX),
                ..default()
            },
        ));
        // Save here and Go reach the views by touch (#561, Q8 A).
        for (text, kind) in [("Save here", 0), ("Go", 1), ("Clear", 2)] {
            let face = |face: &mut ChildSpawnerCommands| {
                face.spawn((Text::new(text), TextRole::Body, TextColor(MENU_TEXT_TINT)));
            };
            let name = format!("views-{}-{slot}", text.to_lowercase().replace(' ', "-"));
            match kind {
                0 => spawn_wide_button(row, SettingSaveView(slot), name, face),
                1 => spawn_small_button(row, SettingGoView(slot), name, face),
                _ => spawn_small_button(row, SettingClear(slot), name, face),
            }
        }
    });
}

/// Spawns the player row of the Views page: Go to player (#607, Q6 A) and
/// Reset player (S2).
pub(crate) fn spawn_player_row(rows: &mut ChildSpawnerCommands) {
    spawn_row(rows, |row| {
        row.spawn((
            Text::new("Player"),
            TextRole::Body,
            TextColor(MENU_TEXT_TINT),
            Node {
                width: Val::Px(SETTING_LABEL_WIDTH_PX),
                ..default()
            },
        ));
        let face = |face: &mut ChildSpawnerCommands| {
            face.spawn((
                Text::new("Go to player"),
                TextRole::Body,
                TextColor(MENU_TEXT_TINT),
            ));
        };
        spawn_wide_button(
            row,
            crate::player::SettingGoPlayer,
            "views-go-to-player".to_string(),
            face,
        );
        let face = |face: &mut ChildSpawnerCommands| {
            face.spawn((
                Text::new("Reset player"),
                TextRole::Body,
                TextColor(MENU_TEXT_TINT),
            ));
        };
        spawn_wide_button(
            row,
            crate::player::SettingResetPlayer,
            "views-reset-player".to_string(),
            face,
        );
    });
}

/// Spawns one per-category Reset button.
pub(crate) fn spawn_reset(rows: &mut ChildSpawnerCommands, category: Category) {
    let label = match category {
        Category::Display => "Reset Display",
        Category::Controls => "Reset Controls",
        Category::Views => "Reset Views",
        Category::Accessibility => "Reset Accessibility",
        Category::Developer => "Reset Developer",
    };
    rows.spawn((
        SettingReset(category),
        Name::new(format!("reset-{label}")),
        chrome::button(
            Val::Px(RESET_BUTTON_WIDTH_PX),
            Val::Px(SCREEN_BUTTON_HEIGHT_PX),
            false,
        ),
    ))
    .with_children(|button| {
        button
            .spawn(chrome::button_face(false))
            .with_children(|face| {
                face.spawn((Text::new(label), TextRole::Body, TextColor(MENU_TEXT_TINT)));
            });
    });
}
