//! Bevy indicators for the nested universe: markers, axes, and the dive camera.
//!
//! Gizmos draw every level by design (spec v3/v4: indicators only), with two
//! exceptions: open surface cells build filled meshes (the L10 planet body
//! and moons under ADR 0016, the L11-L13 tail ground plus highlighted tiles
//! under ADR 0020), and bright portal tints carry emissive billboards under
//! ADR 0015. `UniverseRenderPlugin` owns the camera and the axis
//! indicators; `DivePlugin` (R6, sub-issue #60) owns the nested navigation:
//! the open cell is the render origin, its markers are the next dimension,
//! hovering highlights a marker, clicking targets it, the wheel dives with
//! `v = k * h`, the target opens into its interior at `theta_min`, and
//! diving out closes it back into the marker you came from. The largest
//! markers on screen preview their interior before you enter (R7, #63), so
//! opening and closing change nothing on screen. Every system draws with
//! gizmos, plus the L10 planet meshes, the star billboards, and the HUD.
//!
//! Modules, one line each:
//!
//! - `camera`: indicator camera spawn and per-frame sync.
//! - `draw`: gizmo drawing of axes, cells, previews, and siblings.
//! - `forms`: form-to-gizmo mapping for marker bodies (#384).
//! - `hud`: persistent scale readout (level, distance, bar).
//! - `input`: quit, hover, click/wheel/keys, and the autopilot.
//! - `menu`: menu button, Menu screen, Esc rule, pause and blocking (#473).
//! - `normal_mode`: the player body in normal mode, walk, fly, cameras (#558).
//! - `player_camera`: one function for the player's camera pose (#754).
//! - `player_state`: one player state: spot, glide, and body (leave, follow, enter, #754).
//! - `panel`: shared Guides/Settings two-column layout (#473).
//! - `figure_draw`: one lit low-poly figure for waiting and active (#755).
//! - `screens`: screenshot mode sequencer for `--screens` (#492).
//! - `settings_ui`: Settings content, loader, and the synced store (#473).
//! - `guides_ui`: Guides content with the approved text (#473).
//! - `planet`: L10 moons as meshes; the sphere is LOD patches (#776).
//! - `patches`: LOD patch workers, cache and drawing (#776).
//! - `patch_material`: CDLOD morph material for LOD patches (#777 T3).
//! - `debug_overlay`: F3 LOD overlay with the seven items (#776).
//! - `stars`: emissive billboards for bright portal tints (ADR 0015).
//! - `stream`: background preview generation off the frame thread.
//! - `style`: era colors, camera-relative render origin (ADR 0026), and the pick radius.
//! - `labels`: portal label pool, cached names, largest-first cap (#446).
//! - `tokens`: design tokens: every visual value named by role (#446).
//! - `window_tests`: window-path dives for the continuous camera (#430, tests only).
//! - `lib_tests`: dive pipeline tests (split from `lib.rs` under `E-FILE-SIZE`, #884).
//! - `settings_ui_tests`: Settings UI widget tests (split from `settings_ui.rs`, #884).
//!
//! Navigation math, labels, and the journey replay live in `universe-core`
//! (`nav`, `labels`); this crate keeps thin resource newtypes over them.

use bevy::prelude::*;

mod air;
mod bindings;
mod camera;
mod chrome;
#[cfg(test)]
mod control_tests;
mod debug_overlay;
mod draw;
mod figure_draw;
mod fonts;
mod forms;
#[cfg(test)]
mod ground_colour_tests;
mod guides_ui;
mod hud;
mod input;
mod keys_ui;
mod labels;
#[cfg(test)]
mod lib_tests;
mod menu;
mod normal_mode;
mod panel;
mod patch_material;
mod patches;
mod planet;
mod player;
mod player_camera;
mod player_state;
mod quit_popup;
mod screens;
mod screens_select;
mod screens_sky;
mod settings_rows;
mod settings_ui;
#[cfg(test)]
mod settings_ui_tests;
mod shot_file;
mod shot_place;
mod stars;
mod stream;
mod style;
mod sun;
mod timing;
mod tokens;
mod touch;
mod touch_ui;
mod views_ui;
#[cfg(test)]
mod window_tests;

use camera::{ExposureLevel, spawn_indicator_camera, sync_camera, sync_exposure};
use draw::{draw_axes, draw_open_cell, draw_parent_siblings, draw_previews};
use guides_ui::GuidesUiPlugin;
use hud::{spawn_hud, sync_hud, sync_mode_reminder};
use input::{Autopilot, ClickCycle, CursorPortals, Flight, Latches, Navigation, SavedSlots};
use labels::{LabelCache, spawn_labels, sync_labels};
use menu::{MenuPlugin, Screen};
use panel::PanelPlugin;
use planet::{PlanetMeshState, draw_planets};
use settings_ui::{SettingsUiPlugin, apply_settings_live, apply_settings_window};
use stars::{BillboardState, draw_star_billboards};
use stream::StreamTasks;
use tokens::apply_gizmo_width;
use universe_core::nav::DEMO_SEED;

pub use universe_core::nest::{CLOSE_ANGLE, OPEN_ANGLE, PREVIEW_ANGLE, PREVIEW_CAP};
pub use universe_core::stream::STREAM_CAP;

/// Synced settings store, loaded and saved by `universe-app`.
pub use settings_ui::SettingsStore;

/// Patch morph material plugin (`--screens`, #777 T3).
pub use patch_material::PatchMaterialPlugin;

/// Screenshot mode (`--screens`, #492): plugin, captured-picture queue, shot count.
pub use screens::{CapturedShot, ScreensPlugin, ScreensQueue, shot_count};
pub use screens_select::{SMOKE, ShotGroup, ShotSelection, shot_count_for};
pub use shot_file::{ShotFile, ShotLook, ShotPlace, ShotSpec, parse_shot_file};
pub use timing::{FrameSummary, TIMING_FRAMES, TimingRun, summarise, timing_line};

/// Window-side handle to the navigation state.
///
/// Moved to [`universe_core::nav`] in M3 (#85); this newtype derefs to it so
/// system bodies keep working unchanged.
#[derive(Resource, Deref, DerefMut, Debug, Clone, PartialEq)]
pub struct Universe(pub universe_core::nav::Universe);

/// Window-side handle to the pre-entry preview cache.
///
/// Moved to [`universe_core::nav`] in M3 (#85); this newtype derefs to it so
/// system bodies keep working unchanged.
#[derive(Resource, Deref, DerefMut, Debug, Clone, PartialEq, Default)]
pub struct PreviewCache(pub universe_core::preview::PreviewCache);

/// Dive-pipeline system sets, fixing today's run order (M4, #85).
///
/// Chained `Input` then `Camera` then `Draw`, so the saved order is quit,
/// hover, input, previews, camera, draw open cell, draw previews, draw
/// siblings. The order test below asserts every consecutive pair.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DiveSystems {
    /// Quit, hover, and input handling.
    Input,
    /// Preview sync and camera sync.
    Camera,
    /// Marker drawing.
    Draw,
}

/// Root plugin: the indicator camera plus the open cell's RGB axes.
pub struct UniverseRenderPlugin;

impl Plugin for UniverseRenderPlugin {
    /// Registers the camera and the axis drawing system.
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_indicator_camera, apply_gizmo_width))
            .add_systems(Update, draw_axes);
    }
}

/// Nested dive navigation (R6, sub-issue #60).
///
/// Owns the `Universe`, hover and target state, the autopilot, wheel and
/// key diving, open/close, camera sync, and marker drawing. Added beside
/// `UniverseRenderPlugin`.
pub struct DivePlugin;

impl Plugin for DivePlugin {
    /// Registers navigation state and the input, dive, camera, draw chain.
    fn build(&self, app: &mut App) {
        app.add_plugins((fonts::FontsPlugin, chrome::ChromePlugin, MenuPlugin));
        app.add_plugins((PanelPlugin, SettingsUiPlugin, GuidesUiPlugin));
        // Without the window's material plugin (headless), keep the store.
        if !app
            .world()
            .contains_resource::<Assets<patch_material::PlanetPatchMaterial>>()
        {
            app.init_asset::<patch_material::PlanetPatchMaterial>();
        }
        app.insert_resource(Universe(universe_core::nav::Universe::new(DEMO_SEED)))
            .init_resource::<Navigation>()
            .init_resource::<Autopilot>()
            .init_resource::<PreviewCache>()
            .init_resource::<StreamTasks>()
            .init_resource::<Flight>()
            .init_resource::<SavedSlots>()
            .init_resource::<CursorPortals>()
            .init_resource::<ClickCycle>()
            .init_resource::<player_state::PlayerState>()
            .init_resource::<Latches>()
            .init_resource::<style::RenderOrigin>()
            .init_resource::<PlanetMeshState>()
            .init_resource::<patches::PatchTree>()
            .init_resource::<patches::PatchCache>()
            .init_resource::<patches::GroundSurface>()
            .init_resource::<patch_material::PatchMaterials>()
            .init_resource::<patches::PatchTasks>()
            .init_resource::<debug_overlay::OverlayInfo>()
            .init_resource::<figure_draw::FigureState>()
            .init_resource::<air::AirState>()
            .init_resource::<BillboardState>()
            .init_resource::<ExposureLevel>()
            .init_resource::<LabelCache>()
            .add_plugins((touch_ui::TouchPlugin, player::PlayerPlugin))
            .add_systems(Startup, (spawn_hud, spawn_labels, sun::spawn_sun))
            .add_systems(OnExit(Screen::Universe), input::clear_hover)
            // Stale planet sky (#880): after Bevy's own extract, drop the
            // prepared sky from render cameras that lost their settings.
            // Headless apps have no render sub-app, so nothing is added there.
            .add_systems(
                Update,
                (
                    (
                        touch::read_touches,
                        input::handle_quit,
                        input::pick_hover,
                        input::handle_input,
                        player_state::leave_player,
                        player_state::follow_player,
                        normal_mode::normal_mode_step,
                        player_state::enter_player,
                    )
                        .chain()
                        .run_if(in_state(Screen::Universe))
                        .in_set(DiveSystems::Input),
                    (
                        apply_settings_live,
                        apply_settings_window,
                        stream::sync_previews,
                        patches::sync_patches,
                        air::sync_air,
                        sync_camera,
                        style::place_anchored,
                        sync_exposure,
                    )
                        .chain()
                        .in_set(DiveSystems::Camera),
                    (
                        draw_open_cell,
                        draw_previews,
                        draw_parent_siblings,
                        draw_planets,
                        debug_overlay::sync_overlay,
                        normal_mode::draw_body,
                        figure_draw::draw_figure,
                        sun::sync_sun,
                        air::sync_sky_mode,
                        draw_star_billboards,
                        sync_labels,
                        sync_hud,
                        sync_mode_reminder,
                    )
                        .chain()
                        .in_set(DiveSystems::Draw),
                )
                    .chain(),
            );
        if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
            render_app.add_systems(
                ExtractSchedule,
                air::clear_stale_sky.after(bevy::pbr::extract_atmosphere),
            );
        }
    }
}
