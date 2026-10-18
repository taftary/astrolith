//! Settings content, loader, and the single synced store (#473, T5).
//!
//! Everything here is `pub(crate)`: only this crate uses it.
//! [`SettingsStore`] wraps the core model and is the single resource keys
//! (`E`, `Shift+1-9`, `X`, `Ctrl+1-8`, `Alt+1-8` in [`crate::input`]) and widgets both
//! write through. [`apply_settings_store`] copies it onto the live resources
//! every frame (`ExposureLevel`, `Flight` speed, `SavedSlots`,
//! `Navigation` axes, window mode and vsync with a pending loader), so keys
//! and Settings always agree. `universe-app` saves the store on change.

use crate::camera::ExposureLevel;
use crate::fonts::TextRole;
use crate::input::{Flight, FlightMode, Navigation, SavedSlots, SavedView};
use crate::menu::Screen;
use crate::panel::{PanelContent, PanelState};
use crate::settings_rows::{
    spawn_body_colour, spawn_planet_detail, spawn_reset, spawn_section, spawn_stepper,
    spawn_toggle, spawn_view_slot,
};
use crate::tokens::MENU_DIM_TINT;
use bevy::ecs::system::Single;
use bevy::prelude::*;
use bevy::ui::widget::Text;
use bevy::window::{MonitorSelection, PresentMode, WindowMode};
use universe_core::settings::{
    Category, FlightSpeed, LOOK_MAX, LOOK_MIN, LabelLimit, LookSensitivity, SavedPose, Settings,
    ViewMode,
};

/// Single synced settings resource: keys and widgets write here.
///
/// Public because `universe-app` loads and saves it (`E-CORE-API` helper).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct SettingsStore(pub Settings);

impl Default for SettingsStore {
    /// First-launch settings as documented in Spec v1 (#473).
    fn default() -> SettingsStore {
        SettingsStore(Settings::defaults())
    }
}

impl SettingsStore {
    /// Saved slot as the live view type.
    #[must_use]
    pub(crate) fn slot_view(slot: Option<SavedPose>) -> Option<SavedView> {
        slot.map(|pose| SavedView {
            offset: pose.offset,
            destination: pose.destination,
            mode: match pose.mode {
                ViewMode::Dive => FlightMode::Dive,
                ViewMode::Free => FlightMode::Free,
                ViewMode::Normal => FlightMode::Normal,
            },
            yaw: pose.yaw,
            pitch: pose.pitch,
            step: pose.speed.get(),
        })
    }

    /// Live view as the stored pose type.
    #[must_use]
    pub(crate) fn view_pose(view: &SavedView) -> SavedPose {
        SavedPose {
            offset: view.offset,
            destination: view.destination,
            mode: match view.mode {
                FlightMode::Dive => ViewMode::Dive,
                FlightMode::Free => ViewMode::Free,
                FlightMode::Normal => ViewMode::Normal,
            },
            yaw: view.yaw,
            pitch: view.pitch,
            speed: FlightSpeed::new(view.step).unwrap_or_default(),
        }
    }
}

/// Pending window-mode switches (fullscreen, vsync).
///
/// Set when the store disagrees with the window; cleared once the window
/// reports the new mode. Widgets show a loader and ignore clicks meanwhile.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct PendingWindow {
    /// Fullscreen switch in flight.
    pub fullscreen: bool,
    /// Vsync switch in flight.
    pub vsync: bool,
}

/// On/off setting flipped by one toggle widget.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToggleField {
    /// Display > Scene: portal labels.
    Labels,
    /// Display > Scene: Mode reminder (#557).
    ModeReminder,
    /// Display > Scene: Show pull (#559).
    ShowPull,
    /// Controls > Flight: invert look (#557).
    InvertLook,
    /// Controls > Flight: dive keys hold or toggle (#557).
    DiveToggle,
    /// Controls > Flight: look button hold or toggle (#557).
    LookToggle,
    /// Controls > Flight: Flying turns, drone or plane (#558).
    PlaneTurning,
    /// Controls > Flight: Starting camera, first or third person (#558).
    StartThirdPerson,
    /// Controls > Flight: gradual starts and stops (#756, S2).
    MovementInertia,
    /// Display > Window: fullscreen (loader while pending).
    Fullscreen,
    /// Display > Window: vsync (loader while pending).
    Vsync,
    /// Developer: debug axes.
    DebugAxes,
    /// Developer: LOD debug overlay (#776, S2).
    DebugOverlay,
    /// Controls > Touch: automatic or always (#561).
    TouchAlways,
    /// Controls > Touch: right or left hand (#561).
    LeftHand,
    /// Developer: Simulate touch (#561).
    SimulateTouch,
}

/// Ranged setting stepped by `-`/`+` widgets.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepField {
    /// Display > Scene: exposure.
    Exposure,
    /// Display > Scene: maximum labels.
    LabelLimit,
    /// Display > Scene: Background dimming.
    Dimming,
    /// Controls > Flight: free-flight speed.
    Speed,
    /// Controls > Flight: look sensitivity (#557).
    LookSensitivity,
    /// Controls > Flight: mouse-look smoothing (#756, S1).
    CameraSmoothing,
}

/// Marks a toggle widget button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingToggle(pub ToggleField);

/// Marks the Body colour cycle button (#755, S1): one click cycles presets.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingBodyColour;

/// Marks the Planet detail cycle button (#776, S1): one click cycles presets.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingPlanetDetail;

/// Marks a stepper widget button (`dir` is `-1` or `+1`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingStep {
    /// Ranged setting to step.
    pub field: StepField,
    /// Step direction.
    pub dir: i32,
}

/// Marks the value text of a stepper or toggle row.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingValue(pub SettingValueKind);

/// Which value a [`SettingValue`] text shows.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingValueKind {
    /// Exposure multiplier.
    Exposure,
    /// Labels on/off.
    Labels,
    /// Mode reminder on/off.
    ModeReminder,
    /// Show pull on/off.
    ShowPull,
    /// Invert look on/off.
    InvertLook,
    /// Dive keys hold or toggle.
    DiveToggle,
    /// Look button hold or toggle.
    LookToggle,
    /// Flying turns: drone or plane.
    PlaneTurning,
    /// Starting camera: first or third person.
    StartThirdPerson,
    /// Movement inertia on/off.
    MovementInertia,
    /// Camera smoothing preset.
    CameraSmoothing,
    /// Look sensitivity step.
    LookSensitivity,
    /// Maximum labels.
    LabelLimit,
    /// Background dimming percent.
    Dimming,
    /// Fullscreen on/off.
    Fullscreen,
    /// Vsync on/off.
    Vsync,
    /// Speed step.
    Speed,
    /// Debug axes on/off.
    DebugAxes,
    /// LOD debug overlay on/off (#776, S2).
    DebugOverlay,
    /// Touch controls: automatic or always.
    TouchAlways,
    /// Hand: right or left.
    LeftHand,
    /// Simulate touch on/off.
    SimulateTouch,
    /// Body colour preset (#755, S1).
    BodyColour,
    /// Planet detail preset (#776, S1).
    PlanetDetail,
    /// One saved-view slot (`Saved`/`Empty`).
    ViewSlot(usize),
}

/// Marks a Clear button for one saved-view slot.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingClear(pub usize);

/// Marks a per-category Reset button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingReset(pub Category);

/// Marks the loader text beside a pending switch.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingLoader(pub ToggleField);

/// Marks one settings content section (shown for its leaf ids).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingSection {
    /// Leaf ids showing this section (`display` shows `scene`, ...).
    pub leaves: &'static [&'static str],
}

/// Settings content and loader.
pub(crate) struct SettingsUiPlugin;

impl Plugin for SettingsUiPlugin {
    /// Registers the store, the content, and the widget systems.
    ///
    /// The two apply systems live in the `DivePlugin` camera chain (not
    /// here), so every frame applies keys and widgets before the camera.
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::keys_ui::KeysUiPlugin)
            .init_resource::<SettingsStore>()
            .init_resource::<PendingWindow>()
            .add_systems(
                Update,
                (
                    spawn_settings_content,
                    setting_toggle_click,
                    setting_step_click,
                    setting_body_colour_click,
                    setting_planet_detail_click,
                    setting_clear_click,
                    setting_reset_click,
                    sync_setting_values,
                    sync_setting_sections,
                ),
            );
    }
}

/// On/off text.
fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

/// Hold/toggle text for the input-style settings (#557).
fn hold_toggle(toggle: bool) -> &'static str {
    if toggle { "toggle" } else { "hold" }
}

/// Spawns every settings section into the Settings content area.
///
/// One-shot: runs until the panel shell exists (Startup order across plugins
/// is unspecified), then parks.
pub(crate) fn spawn_settings_content(
    mut commands: Commands,
    content: Query<Entity, With<PanelContent>>,
    panel: Query<&PanelContent>,
    mut spawned: Local<bool>,
) {
    if *spawned {
        return;
    }
    let Some(content) = content.iter().find(|entity| {
        panel
            .get(*entity)
            .is_ok_and(|content| content.0 == Screen::Settings)
    }) else {
        return;
    };
    *spawned = true;
    commands.entity(content).with_children(|sections| {
        spawn_section(sections, &["scene"], "Scene", |rows| {
            spawn_stepper(rows, StepField::Exposure, "Exposure");
            spawn_toggle(rows, ToggleField::Labels, "Portal labels");
            spawn_stepper(rows, StepField::LabelLimit, "Maximum labels");
            spawn_stepper(rows, StepField::Dimming, "Background dimming");
            spawn_toggle(rows, ToggleField::ModeReminder, "Mode reminder");
            spawn_toggle(rows, ToggleField::ShowPull, "Show pull");
            spawn_body_colour(rows);
            spawn_planet_detail(rows);
            spawn_reset(rows, Category::Display);
        });
        spawn_section(sections, &["window"], "Window", |rows| {
            spawn_toggle(rows, ToggleField::Fullscreen, "Fullscreen");
            spawn_toggle(rows, ToggleField::Vsync, "Vsync");
            spawn_reset(rows, Category::Display);
        });
        spawn_section(sections, &["flight"], "Flight", |rows| {
            spawn_stepper(rows, StepField::Speed, "Free-flight speed");
            spawn_stepper(rows, StepField::LookSensitivity, "Look sensitivity");
            spawn_toggle(rows, ToggleField::InvertLook, "Invert look");
            spawn_toggle(rows, ToggleField::DiveToggle, "Dive: hold or toggle");
            spawn_toggle(rows, ToggleField::LookToggle, "Look: hold or toggle");
            spawn_toggle(rows, ToggleField::PlaneTurning, "Flying turns");
            spawn_toggle(rows, ToggleField::StartThirdPerson, "Starting camera");
            spawn_stepper(rows, StepField::CameraSmoothing, "Camera smoothing");
            spawn_toggle(rows, ToggleField::MovementInertia, "Movement inertia");
            spawn_reset(rows, Category::Controls);
        });
        spawn_section(sections, &["keys"], "Keys", |rows| {
            crate::keys_ui::spawn_keys_page(rows);
        });
        spawn_section(sections, &["touch"], "Touch", |rows| {
            spawn_toggle(rows, ToggleField::TouchAlways, "Touch controls");
            spawn_toggle(rows, ToggleField::LeftHand, "Hand");
            spawn_reset(rows, Category::Controls);
        });
        spawn_section(sections, &["views"], "Views", |rows| {
            for slot in 0..universe_core::settings::VIEW_SLOTS {
                spawn_view_slot(rows, slot);
            }
            crate::settings_rows::spawn_player_row(rows);
            spawn_reset(rows, Category::Views);
        });
        spawn_section(sections, &["accessibility"], "Accessibility", |rows| {
            rows.spawn((
                Text::new("Coming soon"),
                TextRole::Note,
                TextColor(MENU_DIM_TINT),
            ));
        });
        spawn_section(sections, &["developer"], "Developer", |rows| {
            spawn_toggle(rows, ToggleField::DebugAxes, "Debug axes");
            spawn_toggle(rows, ToggleField::DebugOverlay, "Debug overlay");
            spawn_toggle(rows, ToggleField::SimulateTouch, "Simulate touch");
            spawn_reset(rows, Category::Developer);
        });
    });
}

/// Copies the store onto every live resource, every frame.
///
/// Keys and widgets both write the store; this keeps `ExposureLevel`,
/// `Flight` speed, `SavedSlots`, and the debug axes in agreement with it.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input; the store fans out to every live resource in one place"
)]
pub(crate) fn apply_settings_live(
    store: Res<SettingsStore>,
    mut exposure: ResMut<ExposureLevel>,
    mut flight: ResMut<Flight>,
    mut slots: ResMut<SavedSlots>,
    mut nav: ResMut<Navigation>,
) {
    exposure.0 = store.0.exposure.get();
    flight.step = store.0.speed.get();
    nav.show_axes = store.0.debug_axes;
    nav.show_overlay = store.0.debug_overlay;
    for (slot, view) in slots.slots.iter_mut().zip(store.0.views.iter()) {
        *slot = SettingsStore::slot_view(*view);
    }
}

/// Requests window switches from the store with a pending loader.
///
/// Sets the window mode and present mode when they disagree with the store,
/// and marks them pending until the window reports the new mode (skipped
/// without exactly one window).
pub(crate) fn apply_settings_window(
    store: Res<SettingsStore>,
    mut pending: ResMut<PendingWindow>,
    mut window: Single<&mut Window>,
) {
    let windowed = matches!(window.mode, WindowMode::Windowed);
    if store.0.fullscreen == windowed {
        window.mode = if store.0.fullscreen {
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        } else {
            WindowMode::Windowed
        };
        pending.fullscreen = true;
    } else {
        pending.fullscreen = false;
    }
    let want_vsync = if store.0.vsync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };
    if window.present_mode != want_vsync {
        window.present_mode = want_vsync;
        pending.vsync = true;
    } else {
        pending.vsync = false;
    }
}

/// Flips one toggle setting (pending switches ignore clicks).
pub(crate) fn setting_toggle_click(
    mut store: ResMut<SettingsStore>,
    pending: Res<PendingWindow>,
    buttons: Query<(&Interaction, &SettingToggle), Changed<Interaction>>,
) {
    for (interaction, toggle) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match toggle.0 {
            ToggleField::Labels => store.0.labels = !store.0.labels,
            ToggleField::ModeReminder => store.0.mode_reminder = !store.0.mode_reminder,
            ToggleField::ShowPull => store.0.show_pull = !store.0.show_pull,
            ToggleField::InvertLook => store.0.invert_look = !store.0.invert_look,
            ToggleField::DiveToggle => store.0.dive_toggle = !store.0.dive_toggle,
            ToggleField::LookToggle => store.0.look_toggle = !store.0.look_toggle,
            ToggleField::PlaneTurning => store.0.plane_turning = !store.0.plane_turning,
            ToggleField::StartThirdPerson => {
                store.0.start_third_person = !store.0.start_third_person;
            }
            ToggleField::MovementInertia => {
                store.0.movement_inertia = !store.0.movement_inertia;
            }
            ToggleField::Fullscreen => {
                if !pending.fullscreen {
                    store.0.fullscreen = !store.0.fullscreen;
                }
            }
            ToggleField::Vsync => {
                if !pending.vsync {
                    store.0.vsync = !store.0.vsync;
                }
            }
            ToggleField::DebugAxes => store.0.debug_axes = !store.0.debug_axes,
            ToggleField::DebugOverlay => store.0.debug_overlay = !store.0.debug_overlay,
            ToggleField::TouchAlways => store.0.touch_always = !store.0.touch_always,
            ToggleField::LeftHand => store.0.left_hand = !store.0.left_hand,
            ToggleField::SimulateTouch => store.0.simulate_touch = !store.0.simulate_touch,
        }
    }
}

/// Steps one ranged setting within its range.
pub(crate) fn setting_step_click(
    mut store: ResMut<SettingsStore>,
    buttons: Query<(&Interaction, &SettingStep), Changed<Interaction>>,
) {
    for (interaction, step) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match step.field {
            StepField::Exposure => {
                store.0.exposure = if step.dir > 0 {
                    store.0.exposure.brighter()
                } else {
                    store.0.exposure.darker()
                };
            }
            StepField::LabelLimit => {
                let next = store
                    .0
                    .label_limit
                    .get()
                    .saturating_add_signed(step.dir as isize);
                if let Some(limit) = LabelLimit::new(next) {
                    store.0.label_limit = limit;
                }
            }
            StepField::Dimming => {
                store.0.dimming = if step.dir > 0 {
                    store.0.dimming.darker()
                } else {
                    store.0.dimming.lighter()
                };
            }
            StepField::Speed => {
                let next = store.0.speed.get().saturating_add_signed(step.dir);
                if let Some(speed) = FlightSpeed::new(next.clamp(1, 9)) {
                    store.0.speed = speed;
                }
            }
            StepField::LookSensitivity => {
                let next = store
                    .0
                    .look_sensitivity
                    .get()
                    .saturating_add_signed(step.dir);
                if let Some(look) = LookSensitivity::new(next.clamp(LOOK_MIN, LOOK_MAX)) {
                    store.0.look_sensitivity = look;
                }
            }
            StepField::CameraSmoothing => {
                store.0.camera_smoothing = if step.dir > 0 {
                    store.0.camera_smoothing.higher()
                } else {
                    store.0.camera_smoothing.lower()
                };
            }
        }
    }
}

/// Cycles the Body colour presets (#755, S1).
pub(crate) fn setting_body_colour_click(
    mut store: ResMut<SettingsStore>,
    buttons: Query<&Interaction, (Changed<Interaction>, With<SettingBodyColour>)>,
) {
    for interaction in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        store.0.body_colour = store.0.body_colour.next();
    }
}

/// Cycles the Planet detail presets (#776, S1).
pub(crate) fn setting_planet_detail_click(
    mut store: ResMut<SettingsStore>,
    buttons: Query<&Interaction, (Changed<Interaction>, With<SettingPlanetDetail>)>,
) {
    for interaction in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        store.0.planet_detail = store.0.planet_detail.next();
    }
}

/// Clears one saved-view slot.
pub(crate) fn setting_clear_click(
    mut store: ResMut<SettingsStore>,
    buttons: Query<(&Interaction, &SettingClear), Changed<Interaction>>,
) {
    for (interaction, clear) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if let Some(slot) = store.0.views.get_mut(clear.0) {
            *slot = None;
        }
    }
}

/// Resets one category to its defaults (Views clears all slots).
pub(crate) fn setting_reset_click(
    mut store: ResMut<SettingsStore>,
    buttons: Query<(&Interaction, &SettingReset), Changed<Interaction>>,
) {
    for (interaction, reset) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        store.0 = store.0.reset(reset.0);
    }
}

/// Rewrites every value text (plus loaders) from the store.
pub(crate) fn sync_setting_values(
    store: Res<SettingsStore>,
    pending: Res<PendingWindow>,
    mut values: Query<(&SettingValue, &mut Text), Without<SettingLoader>>,
    mut loaders: Query<(&SettingLoader, &mut Text), Without<SettingValue>>,
) {
    if !store.is_changed() && !pending.is_changed() {
        return;
    }
    for (value, mut text) in &mut values {
        let want: String = match value.0 {
            // Value only: the row label beside it names the setting.
            SettingValueKind::Exposure => format!("{:.2}", store.0.exposure.get()),
            SettingValueKind::Labels => on_off(store.0.labels).to_string(),
            SettingValueKind::ModeReminder => on_off(store.0.mode_reminder).to_string(),
            SettingValueKind::ShowPull => on_off(store.0.show_pull).to_string(),
            SettingValueKind::InvertLook => on_off(store.0.invert_look).to_string(),
            SettingValueKind::DiveToggle => hold_toggle(store.0.dive_toggle).to_string(),
            SettingValueKind::LookToggle => hold_toggle(store.0.look_toggle).to_string(),
            SettingValueKind::PlaneTurning => if store.0.plane_turning {
                "plane"
            } else {
                "drone"
            }
            .to_string(),
            SettingValueKind::StartThirdPerson => if store.0.start_third_person {
                "third person"
            } else {
                "first person"
            }
            .to_string(),
            SettingValueKind::MovementInertia => on_off(store.0.movement_inertia).to_string(),
            SettingValueKind::CameraSmoothing => store.0.camera_smoothing.display().to_string(),
            SettingValueKind::LookSensitivity => store.0.look_sensitivity.get().to_string(),
            SettingValueKind::LabelLimit => store.0.label_limit.get().to_string(),
            SettingValueKind::Fullscreen => on_off(store.0.fullscreen).to_string(),
            SettingValueKind::Vsync => on_off(store.0.vsync).to_string(),
            SettingValueKind::Dimming => format!("{}%", store.0.dimming.get()),
            SettingValueKind::Speed => store.0.speed.get().to_string(),
            SettingValueKind::DebugAxes => on_off(store.0.debug_axes).to_string(),
            SettingValueKind::DebugOverlay => on_off(store.0.debug_overlay).to_string(),
            SettingValueKind::TouchAlways => if store.0.touch_always {
                "always"
            } else {
                "automatic"
            }
            .to_string(),
            SettingValueKind::LeftHand => {
                if store.0.left_hand { "left" } else { "right" }.to_string()
            }
            SettingValueKind::SimulateTouch => on_off(store.0.simulate_touch).to_string(),
            SettingValueKind::BodyColour => store.0.body_colour.display().to_string(),
            SettingValueKind::PlanetDetail => store.0.planet_detail.display().to_string(),
            SettingValueKind::ViewSlot(slot) => {
                let state = match store.0.views.get(slot) {
                    Some(Some(_)) => "Saved",
                    _ => "Empty",
                };
                format!("Slot {}: {state}", slot + 1)
            }
        };
        if text.0 != want {
            text.0 = want;
        }
    }
    for (loader, mut text) in &mut loaders {
        let busy = match loader.0 {
            ToggleField::Fullscreen => pending.fullscreen,
            ToggleField::Vsync => pending.vsync,
            ToggleField::Labels
            | ToggleField::ModeReminder
            | ToggleField::ShowPull
            | ToggleField::InvertLook
            | ToggleField::DiveToggle
            | ToggleField::LookToggle
            | ToggleField::PlaneTurning
            | ToggleField::StartThirdPerson
            | ToggleField::MovementInertia
            | ToggleField::DebugAxes
            | ToggleField::DebugOverlay
            | ToggleField::TouchAlways
            | ToggleField::LeftHand
            | ToggleField::SimulateTouch => false,
        };
        let want = if busy { "..." } else { "" };
        if text.0 != want {
            text.0 = want.to_string();
        }
    }
}

/// Shows the section matching the panel selection.
///
/// Hidden sections also leave layout (`Display::None`): `Visibility::Hidden`
/// alone keeps their space, which stacked every page and pushed the selected
/// one out of view (#518).
pub(crate) fn sync_setting_sections(
    selection: Res<PanelState>,
    screen: Res<State<Screen>>,
    mut sections: Query<(&SettingSection, &mut Visibility, &mut Node), Without<PanelContent>>,
) {
    if !selection.is_changed() && !screen.is_changed() {
        return;
    }
    let open = *screen.get() == Screen::Settings;
    for (section, mut visibility, mut node) in &mut sections {
        let selected = match selection.selected.as_str() {
            "display" => section.leaves.contains(&"scene"),
            "controls" => section.leaves.contains(&"flight"),
            other => section.leaves.contains(&other),
        };
        let show = open && selected;
        let want_visibility = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != want_visibility {
            *visibility = want_visibility;
        }
        let want_display = if show { Display::Flex } else { Display::None };
        if node.display != want_display {
            node.display = want_display;
        }
    }
}

/// Stepper buttons for one ranged setting (spawned beside its value text).
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::AssetPlugin;
    use bevy::gizmos::{
        GizmoAsset, GizmoHandles,
        config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    };
    use bevy::input::mouse::{MouseMotion, MouseWheel};
    use bevy::mesh::Mesh;
    use bevy::pbr::StandardMaterial;

    /// Headless app with the full dive pipeline (store included).
    fn headless_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.add_message::<AppExit>();
        app.add_message::<MouseWheel>();
        app.add_message::<MouseMotion>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.init_resource::<GizmoConfigStore>();
        app.init_gizmo_group::<DefaultGizmoConfigGroup>();
        app.init_asset::<GizmoAsset>();
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.init_resource::<GizmoHandles>();
        app.add_plugins(crate::DivePlugin);
        app
    }

    /// Taps `key` for exactly one update (plus the state-transition frame).
    fn tap(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release(key);
        input.clear();
        app.update();
    }

    /// Holds `held` while tapping `tap_key` for one update.
    fn chord(app: &mut App, held: KeyCode, tap_key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(held);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(tap_key);
        app.update();
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release(tap_key);
        input.release(held);
        input.clear();
        app.update();
    }

    #[test]
    fn only_the_selected_section_keeps_layout_space() {
        use crate::panel::PanelState;
        let mut app = headless_app();
        app.update();
        app.world_mut()
            .resource_mut::<NextState<Screen>>()
            .set(Screen::Settings);
        app.update();
        app.update();
        for leaf in [
            "scene",
            "window",
            "flight",
            "keys",
            "views",
            "accessibility",
            "developer",
        ] {
            app.world_mut().resource_mut::<PanelState>().select(leaf);
            app.update();
            let mut shown = 0;
            for (section, node) in app
                .world_mut()
                .query::<(&SettingSection, &Node)>()
                .iter(app.world())
            {
                if section.leaves.contains(&leaf) {
                    assert_eq!(node.display, Display::Flex, "{leaf} shows its section");
                    shown += 1;
                } else {
                    assert_eq!(node.display, Display::None, "{leaf} collapses the rest");
                }
            }
            assert_eq!(shown, 1, "{leaf} shows exactly one section");
        }
    }

    #[test]
    fn store_defaults_match_the_spec_table() {
        let store = SettingsStore::default();
        assert_eq!(store.0.exposure.get(), 1.0);
        assert!(store.0.labels);
        assert_eq!(store.0.label_limit.get(), 24);
        assert!(!store.0.fullscreen);
        assert!(store.0.vsync);
        assert_eq!(store.0.speed.get(), 5);
        assert!(store.0.views.iter().all(Option::is_none));
        assert!(!store.0.debug_axes);
    }

    #[test]
    fn keys_write_through_the_store_and_stay_in_sync() {
        let mut app = headless_app();
        app.update();
        // `E` dims the store exposure, and the camera level follows.
        tap(&mut app, KeyCode::KeyE);
        assert!(
            app.world().resource::<SettingsStore>().0.exposure.get() < 1.0,
            "E dims the store exposure"
        );
        assert_eq!(
            app.world().resource::<ExposureLevel>().0,
            app.world().resource::<SettingsStore>().0.exposure.get(),
            "the camera level follows the store"
        );
        // `Shift+7` steps the store speed, and flight follows.
        chord(&mut app, KeyCode::ShiftLeft, KeyCode::Digit7);
        assert_eq!(app.world().resource::<SettingsStore>().0.speed.get(), 7);
        assert_eq!(app.world().resource::<Flight>().step, 7);
        // `X` flips the store axes, and navigation follows.
        tap(&mut app, KeyCode::KeyX);
        assert!(app.world().resource::<SettingsStore>().0.debug_axes);
        assert!(app.world().resource::<Navigation>().show_axes);
        // `Ctrl+3` saves slot 3 in the store and the live slots.
        chord(&mut app, KeyCode::ControlLeft, KeyCode::Digit3);
        assert!(
            app.world().resource::<SettingsStore>().0.views[2].is_some(),
            "slot 3 saved in the store"
        );
        assert!(
            app.world().resource::<SavedSlots>().slots[2].is_some(),
            "slot 3 saved live"
        );
    }

    #[test]
    fn views_clear_and_reset_touch_only_their_slots() {
        let mut app = headless_app();
        app.update();
        chord(&mut app, KeyCode::ControlLeft, KeyCode::Digit2);
        assert!(app.world().resource::<SettingsStore>().0.views[1].is_some());
        // Clear slot 2 through the store (widget path below covers clicks).
        app.world_mut().resource_mut::<SettingsStore>().0.views[1] = None;
        app.update();
        assert!(app.world().resource::<SettingsStore>().0.views[1].is_none());
        assert!(
            app.world().resource::<SavedSlots>().slots[1].is_none(),
            "cleared slots recall to nothing"
        );
        // Resetting Display keeps the speed the keys set.
        chord(&mut app, KeyCode::ShiftLeft, KeyCode::Digit9);
        app.world_mut().resource_mut::<SettingsStore>().0 = app
            .world()
            .resource::<SettingsStore>()
            .0
            .reset(Category::Display);
        app.update();
        let store = app.world().resource::<SettingsStore>().0;
        assert_eq!(store.exposure.get(), 1.0);
        assert_eq!(store.speed.get(), 9, "other categories are untouched");
    }

    #[test]
    fn fullscreen_request_pends_until_the_window_reports_back() {
        let mut app = headless_app();
        app.world_mut().spawn((Window::default(),));
        app.update();
        let count = app.world_mut().query::<&Window>().iter(app.world()).count();
        assert_eq!(count, 1, "exactly one window for the Single param");
        assert!(!app.world().resource::<PendingWindow>().fullscreen);
        app.world_mut().resource_mut::<SettingsStore>().0.fullscreen = true;
        app.update();
        assert!(
            app.world().resource::<PendingWindow>().fullscreen,
            "the switch pends while the window changes"
        );
        app.update();
        assert!(
            !app.world().resource::<PendingWindow>().fullscreen,
            "the loader clears once the window reports back"
        );
        assert!(
            app.world_mut()
                .query::<&Window>()
                .iter(app.world())
                .all(|window| !matches!(window.mode, WindowMode::Windowed)),
            "the window went fullscreen"
        );
    }

    #[test]
    fn accessibility_shows_coming_soon() {
        let mut app = headless_app();
        app.update();
        let texts: Vec<String> = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|text| text.0.clone())
            .collect();
        assert!(
            texts.iter().any(|text| text == "Coming soon"),
            "accessibility shows its note"
        );
        assert!(
            texts
                .iter()
                .any(|text| text == crate::keys_ui::FIXED_KEYS_NOTE),
            "the Keys page names the fixed keys"
        );
    }

    /// Presses one widget button for one update.
    fn press_widget<T: Component>(app: &mut App, matches: impl Fn(&T) -> bool) {
        let entity = app
            .world_mut()
            .query::<(Entity, &T)>()
            .iter(app.world())
            .find(|(_, widget)| matches(widget))
            .map(|(entity, _)| entity)
            .expect("widget exists");
        app.world_mut()
            .entity_mut(entity)
            .insert(Interaction::Pressed);
        app.update();
        app.world_mut().entity_mut(entity).insert(Interaction::None);
        app.update();
    }

    #[test]
    fn widgets_write_through_the_store() {
        let mut app = headless_app();
        app.update();
        // Speed `+` steps the store speed, and flight follows.
        press_widget(&mut app, |step: &SettingStep| {
            step.field == StepField::Speed && step.dir > 0
        });
        assert_eq!(app.world().resource::<SettingsStore>().0.speed.get(), 6);
        assert_eq!(app.world().resource::<Flight>().step, 6);
        // Labels toggle flips the store.
        press_widget(&mut app, |toggle: &SettingToggle| {
            toggle.0 == ToggleField::Labels
        });
        assert!(!app.world().resource::<SettingsStore>().0.labels);
        // #770: Ground guides removed.
        // The new controls rows write through the store too (#557).
        press_widget(&mut app, |step: &SettingStep| {
            step.field == StepField::LookSensitivity && step.dir > 0
        });
        assert_eq!(
            app.world()
                .resource::<SettingsStore>()
                .0
                .look_sensitivity
                .get(),
            6
        );
        for field in [
            ToggleField::InvertLook,
            ToggleField::DiveToggle,
            ToggleField::LookToggle,
            ToggleField::ModeReminder,
        ] {
            press_widget(&mut app, |toggle: &SettingToggle| toggle.0 == field);
        }
        let settings = app.world().resource::<SettingsStore>().0;
        assert!(settings.invert_look && settings.dive_toggle && settings.look_toggle);
        assert!(!settings.mode_reminder, "the reminder turns off");
        // Views reset clears every slot.
        chord(&mut app, KeyCode::ControlLeft, KeyCode::Digit2);
        assert!(
            app.world().resource::<SettingsStore>().0.views[1].is_some(),
            "slot 2 saved first"
        );
        press_widget(&mut app, |reset: &SettingReset| reset.0 == Category::Views);
        assert!(
            app.world()
                .resource::<SettingsStore>()
                .0
                .views
                .iter()
                .all(Option::is_none),
            "reset clears all slots"
        );
        assert!(
            app.world()
                .resource::<SavedSlots>()
                .slots
                .iter()
                .all(Option::is_none),
            "cleared slots recall to nothing"
        );
    }

    #[test]
    fn smoothing_and_inertia_widgets_write_through_the_store() {
        // #756 C6 (S1, S2): the Flight page steps camera smoothing and
        // toggles movement inertia through the store.
        use universe_core::settings::CameraSmoothing;
        let mut app = headless_app();
        app.update();
        assert_eq!(
            app.world().resource::<SettingsStore>().0.camera_smoothing,
            CameraSmoothing::Low
        );
        assert!(app.world().resource::<SettingsStore>().0.movement_inertia);
        // Both rows exist with their labels.
        let texts: Vec<String> = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|text| text.0.clone())
            .collect();
        for label in ["Camera smoothing", "Movement inertia"] {
            assert!(texts.iter().any(|text| text == label), "{label} row exists");
        }
        press_widget(&mut app, |step: &SettingStep| {
            step.field == StepField::CameraSmoothing && step.dir > 0
        });
        press_widget(&mut app, |step: &SettingStep| {
            step.field == StepField::CameraSmoothing && step.dir > 0
        });
        assert_eq!(
            app.world().resource::<SettingsStore>().0.camera_smoothing,
            CameraSmoothing::High
        );
        press_widget(&mut app, |step: &SettingStep| {
            step.field == StepField::CameraSmoothing && step.dir < 0
        });
        assert_eq!(
            app.world().resource::<SettingsStore>().0.camera_smoothing,
            CameraSmoothing::Medium
        );
        press_widget(&mut app, |toggle: &SettingToggle| {
            toggle.0 == ToggleField::MovementInertia
        });
        assert!(!app.world().resource::<SettingsStore>().0.movement_inertia);
        press_widget(&mut app, |toggle: &SettingToggle| {
            toggle.0 == ToggleField::MovementInertia
        });
        assert!(app.world().resource::<SettingsStore>().0.movement_inertia);
        app.world_mut().resource_mut::<SettingsStore>().0 = app
            .world()
            .resource::<SettingsStore>()
            .0
            .reset(Category::Controls);
        app.update();
        let store = app.world().resource::<SettingsStore>().0;
        assert_eq!(store.camera_smoothing, CameraSmoothing::Low);
        assert!(store.movement_inertia);
    }
}
