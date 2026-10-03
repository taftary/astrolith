//! Bevy indicators for the nested universe: markers, axes, and the dive camera.
//!
//! No meshes, materials, or textures live here by design (spec v3/v4:
//! indicators only). `UniverseRenderPlugin` owns the camera and the axis
//! indicators; `DivePlugin` (R6, sub-issue #60) owns the nested navigation:
//! the open cell is the render origin, its markers are the next dimension,
//! hovering highlights a marker, clicking targets it, the wheel dives with
//! `v = k * h`, the target opens into its interior at `theta_min`, and
//! diving out closes it back into the marker you came from. The largest
//! markers on screen preview their interior before you enter (R7, #63), so
//! opening and closing change nothing on screen. Every system draws with
//! gizmos only.
//!
//! Modules, one line each:
//!
//! - `camera`: indicator camera spawn and per-frame sync.
//! - `draw`: gizmo drawing of axes, cells, previews, and siblings.
//! - `input`: quit, hover, click/wheel/keys, and the autopilot.
//! - `style`: era colors, render-boundary conversions, and the pick radius.
//!
//! Navigation math, labels, and the journey replay live in `universe-core`
//! (`nav`, `labels`); this crate keeps thin resource newtypes over them.

use bevy::prelude::*;

mod camera;
mod draw;
mod input;
mod style;

use camera::{spawn_indicator_camera, sync_camera};
use draw::{draw_axes, draw_open_cell, draw_parent_siblings, draw_previews, sync_previews};
use input::{Autopilot, Navigation};
use universe_core::nav::DEMO_SEED;

pub use universe_core::nest::{CLOSE_ANGLE, OPEN_ANGLE, PREVIEW_ANGLE, PREVIEW_CAP};

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
pub struct PreviewCache(pub universe_core::nav::PreviewCache);

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
        app.add_systems(Startup, spawn_indicator_camera)
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
        app.insert_resource(Universe(universe_core::nav::Universe::new(DEMO_SEED)))
            .init_resource::<Navigation>()
            .init_resource::<Autopilot>()
            .init_resource::<PreviewCache>()
            .add_systems(
                Update,
                (
                    (input::handle_quit, input::pick_hover, input::handle_input)
                        .chain()
                        .in_set(DiveSystems::Input),
                    (sync_previews, sync_camera)
                        .chain()
                        .in_set(DiveSystems::Camera),
                    (draw_open_cell, draw_previews, draw_parent_siblings)
                        .chain()
                        .in_set(DiveSystems::Draw),
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::sync_camera;
    use crate::draw::{draw_open_cell, draw_parent_siblings, draw_previews, sync_previews};
    use crate::input::{handle_input, handle_quit, pick_hover};
    use bevy::asset::AssetPlugin;
    use bevy::gizmos::{
        GizmoAsset, GizmoHandles,
        config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    };
    use bevy::input::{ButtonInput, mouse::MouseWheel};
    use bevy::prelude::{App, AppExit, MinimalPlugins, ResMut, Resource};
    use universe_core::coords::Level;

    /// Headless app running the dive pipeline without a window or GPU.
    ///
    /// Headless app running the dive pipeline without a window or GPU.
    ///
    /// Minimal plugin set `DivePlugin` needs: `MinimalPlugins` (task pool,
    /// time, schedules) plus `AssetPlugin` for asset storage; the `AppExit`
    /// and `MouseWheel` messages; defaulted button inputs; an empty gizmo
    /// config store with its default group plus gizmo mesh asset storage
    /// and handles for the `Gizmos` draw param (buffered, never drawn);
    /// and `DivePlugin` itself. Deliberately no `GizmoPlugin`: it adds
    /// render-world mesh systems that cannot run headless.
    fn headless_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.add_message::<AppExit>();
        app.add_message::<MouseWheel>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.init_resource::<GizmoConfigStore>();
        app.init_gizmo_group::<DefaultGizmoConfigGroup>();
        app.init_asset::<GizmoAsset>();
        app.init_resource::<GizmoHandles>();
        app.add_plugins(DivePlugin);
        app
    }

    #[test]
    fn window_boots_headless_and_stays_at_root() {
        let mut app = headless_app();
        for _ in 0..3 {
            app.update();
        }
        let universe = app.world().resource::<Universe>();
        assert_eq!(universe.level(), Level::MIN);
    }

    /// Pipeline order probe, one spy per gap between consecutive systems.
    #[derive(Resource, Default)]
    struct OrderLog(Vec<&'static str>);

    fn spy_after_quit(mut log: ResMut<OrderLog>) {
        log.0.push("after-quit");
    }

    fn spy_after_hover(mut log: ResMut<OrderLog>) {
        log.0.push("after-hover");
    }

    fn spy_after_input(mut log: ResMut<OrderLog>) {
        log.0.push("after-input");
    }

    fn spy_after_previews(mut log: ResMut<OrderLog>) {
        log.0.push("after-previews");
    }

    fn spy_after_camera(mut log: ResMut<OrderLog>) {
        log.0.push("after-camera");
    }

    fn spy_after_draw_open(mut log: ResMut<OrderLog>) {
        log.0.push("after-draw-open");
    }

    fn spy_after_draw_previews(mut log: ResMut<OrderLog>) {
        log.0.push("after-draw-previews");
    }

    #[test]
    fn dive_pipeline_runs_in_chain_order() {
        let mut app = headless_app();
        app.init_resource::<OrderLog>();
        app.add_systems(
            Update,
            (
                spy_after_quit.after(handle_quit).before(pick_hover),
                spy_after_hover.after(pick_hover).before(handle_input),
                spy_after_input.after(handle_input).before(sync_previews),
                spy_after_previews.after(sync_previews).before(sync_camera),
                spy_after_camera.after(sync_camera).before(draw_open_cell),
                spy_after_draw_open
                    .after(draw_open_cell)
                    .before(draw_previews),
                spy_after_draw_previews
                    .after(draw_previews)
                    .before(draw_parent_siblings),
            ),
        );
        app.update();
        assert_eq!(
            app.world().resource::<OrderLog>().0,
            [
                "after-quit",
                "after-hover",
                "after-input",
                "after-previews",
                "after-camera",
                "after-draw-open",
                "after-draw-previews",
            ],
        );
    }
}
