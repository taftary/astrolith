//! Bevy indicators for the nested universe: markers, axes, and the dive camera.
//!
//! Gizmos draw every level by design (spec v3/v4: indicators only), with two
//! exceptions: the open L10 cell also builds bare planet meshes (body, moons)
//! under ADR 0016, and bright portal tints carry emissive billboards under
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
//! - `draw`: gizmo drawing of axes, cells, previews, siblings, and room outlines.
//! - `forms`: form-to-gizmo mapping for marker bodies (#384).
//! - `hud`: persistent scale readout (level, distance, bar).
//! - `input`: quit, hover, click/wheel/keys, and the autopilot.
//! - `planet`: L10 bare planet body and moons as meshes (ADR 0016).
//! - `stars`: emissive billboards for bright portal tints (ADR 0015).
//! - `stream`: background preview generation off the frame thread.
//! - `style`: era colors, render-boundary conversions, and the pick radius.
//!
//! Navigation math, labels, and the journey replay live in `universe-core`
//! (`nav`, `labels`); this crate keeps thin resource newtypes over them.

use bevy::prelude::*;

mod camera;
mod draw;
mod forms;
mod hud;
mod input;
mod planet;
mod stars;
mod stream;
mod style;

use camera::{ExposureLevel, spawn_indicator_camera, sync_camera, sync_exposure};
use draw::{draw_axes, draw_open_cell, draw_parent_siblings, draw_previews, draw_room_outlines};
use hud::{spawn_hud, sync_hud};
use input::{Autopilot, Flight, Navigation, SavedSlots};
use planet::{PlanetMeshState, draw_planets};
use stars::{BillboardState, draw_star_billboards};
use stream::StreamTasks;
use universe_core::nav::DEMO_SEED;

pub use universe_core::nest::{CLOSE_ANGLE, OPEN_ANGLE, PREVIEW_ANGLE, PREVIEW_CAP};
pub use universe_core::stream::STREAM_CAP;

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
            .init_resource::<StreamTasks>()
            .init_resource::<Flight>()
            .init_resource::<SavedSlots>()
            .init_resource::<PlanetMeshState>()
            .init_resource::<BillboardState>()
            .init_resource::<ExposureLevel>()
            .add_systems(Startup, spawn_hud)
            .add_systems(
                Update,
                (
                    (input::handle_quit, input::pick_hover, input::handle_input)
                        .chain()
                        .in_set(DiveSystems::Input),
                    (stream::sync_previews, sync_camera, sync_exposure)
                        .chain()
                        .in_set(DiveSystems::Camera),
                    (
                        draw_open_cell,
                        draw_room_outlines,
                        draw_previews,
                        draw_parent_siblings,
                        draw_planets,
                        draw_star_billboards,
                        sync_hud,
                    )
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
    use crate::draw::{draw_open_cell, draw_parent_siblings, draw_previews, draw_room_outlines};
    use crate::input::{handle_input, handle_quit, pick_hover};
    use crate::stream::sync_previews;
    use bevy::asset::AssetPlugin;
    use bevy::gizmos::{
        GizmoAsset, GizmoHandles,
        config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    };
    use bevy::input::{ButtonInput, mouse::MouseMotion, mouse::MouseWheel};
    use bevy::mesh::Mesh;
    use bevy::pbr::StandardMaterial;
    use bevy::prelude::{App, AppExit, KeyCode, MinimalPlugins, ResMut, Resource};
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
        app.add_message::<MouseMotion>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.init_resource::<GizmoConfigStore>();
        app.init_gizmo_group::<DefaultGizmoConfigGroup>();
        app.init_asset::<GizmoAsset>();
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
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

    #[test]
    fn background_previews_match_synchronous_sync() {
        use crate::stream::StreamTasks;
        use universe_core::nav::MarkerIndex;
        use universe_core::stream::STREAM_CAP;
        let mut app = headless_app();
        // Open L2 directly so previews exist on the first frame.
        {
            let mut universe = app.world_mut().resource_mut::<Universe>();
            let marker = universe.autopilot_target().expect("marker");
            assert!(universe.open(MarkerIndex(marker)));
        }
        // Step until the background tasks merge the full preview set
        // (bounded: generation is pure, so completion always lands).
        for _ in 0..600 {
            app.update();
            let universe = app.world().resource::<Universe>().0.clone();
            let mut synchronous = PreviewCache::default();
            synchronous.sync(&universe);
            let cache = &app.world().resource::<PreviewCache>().0;
            if cache.entries() == synchronous.entries() {
                let ledger = &app.world().resource::<StreamTasks>().ledger;
                assert!(ledger.len() <= STREAM_CAP, "ledger over the cap");
                return;
            }
        }
        panic!("background previews never settled");
    }

    /// Presses `key` for exactly one update.
    ///
    /// Nothing clears edge-triggered input headless, so the helper flushes
    /// it: without the flush a tap would keep firing on later updates.
    fn tap(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release(key);
        input.clear();
    }

    /// Holds `key` for `frames` updates.
    fn hold(app: &mut App, key: KeyCode, frames: usize) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        for _ in 0..frames {
            app.update();
        }
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release(key);
        input.clear();
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
    }

    #[test]
    fn f_toggles_dive_and_free_flight() {
        use crate::input::{Autopilot, Flight, FlightMode};
        use universe_core::flight::nearest_portal;
        let mut app = headless_app();
        // Start the autopilot so the toggle also proves it cancels.
        tap(&mut app, KeyCode::Space);
        assert_eq!(*app.world().resource::<Autopilot>(), Autopilot::Flying);
        // `F` enters free flight: camera kept, target dropped, no steering.
        let before = app.world().resource::<Universe>().path.offset();
        tap(&mut app, KeyCode::KeyF);
        let flight = *app.world().resource::<Flight>();
        assert_eq!(flight.mode, FlightMode::Free);
        assert_eq!(app.world().resource::<Navigation>().target, None);
        assert_eq!(*app.world().resource::<Autopilot>(), Autopilot::Idle);
        assert_eq!(app.world().resource::<Universe>().path.offset(), before);
        // `F` leaves it: the nearest portal is targeted for the dive.
        tap(&mut app, KeyCode::KeyF);
        let universe = app.world().resource::<Universe>();
        let want = nearest_portal(universe.path.offset(), &universe.open);
        assert_eq!(app.world().resource::<Flight>().mode, FlightMode::Dive);
        assert!(want.is_some(), "root cell offers portals");
        assert_eq!(app.world().resource::<Navigation>().target, want);
    }

    #[test]
    fn space_in_free_flight_returns_to_the_dive() {
        use crate::input::{Autopilot, Flight, FlightMode};
        use universe_core::flight::nearest_portal;
        let mut app = headless_app();
        tap(&mut app, KeyCode::KeyF);
        assert_eq!(app.world().resource::<Flight>().mode, FlightMode::Free);
        tap(&mut app, KeyCode::Space);
        assert_eq!(app.world().resource::<Flight>().mode, FlightMode::Dive);
        assert_eq!(*app.world().resource::<Autopilot>(), Autopilot::Flying);
        let universe = app.world().resource::<Universe>();
        let want = nearest_portal(universe.path.offset(), &universe.open);
        assert!(want.is_some(), "root cell offers portals");
        assert_eq!(app.world().resource::<Navigation>().target, want);
    }

    #[test]
    fn shift_digit_steps_speed_and_slots_round_trip() {
        use crate::input::{Flight, FlightMode};
        let mut app = headless_app();
        tap(&mut app, KeyCode::KeyF);
        assert_eq!(app.world().resource::<Flight>().mode, FlightMode::Free);
        chord(&mut app, KeyCode::ShiftLeft, KeyCode::Digit7);
        assert_eq!(app.world().resource::<Flight>().step, 7);
        // Save slot 3, fly forward, recall: the saved view returns exactly.
        chord(&mut app, KeyCode::ControlLeft, KeyCode::Digit3);
        let saved = app.world().resource::<Universe>().path.offset();
        hold(&mut app, KeyCode::KeyW, 60);
        let flown = app.world().resource::<Universe>().path.offset();
        assert_ne!(flown, saved, "free flight must move the camera");
        assert!(
            flown[2] < saved[2],
            "with zero yaw, forward flies toward -Z"
        );
        tap(&mut app, KeyCode::Digit3);
        assert_eq!(
            app.world().resource::<Universe>().path.offset(),
            saved,
            "recall must restore the saved view exactly"
        );
        assert_eq!(
            app.world().resource::<Flight>().mode,
            FlightMode::Free,
            "recall restores the saved mode"
        );
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

    fn spy_after_draw_room(mut log: ResMut<OrderLog>) {
        log.0.push("after-draw-room");
    }

    fn spy_after_draw_previews(mut log: ResMut<OrderLog>) {
        log.0.push("after-draw-previews");
    }

    /// Counts planet-mesh entities through a detached query state.
    fn planet_mesh_count(app: &mut App) -> usize {
        use crate::planet::PlanetMesh;
        let mut state = app.world_mut().query_filtered::<Entity, With<PlanetMesh>>();
        state.iter(app.world()).count()
    }

    /// Counts rim entities by name: always zero since #375 removed the rim.
    fn planet_rim_count(app: &mut App) -> usize {
        use crate::planet::PlanetMesh;
        let mut state = app.world_mut().query_filtered::<&Name, With<PlanetMesh>>();
        state
            .iter(app.world())
            .filter(|name| name.as_str() == "planet-rim")
            .count()
    }

    #[test]
    fn l10_open_cell_spawns_planet_meshes_and_leaving_despawns_them() {
        use universe_core::nav::MarkerIndex;
        let mut app = headless_app();
        // Dive the headless universe to the L10 planet cell through nine
        // autopilot opens (pure generation, no frames pass).
        {
            let mut universe = app.world_mut().resource_mut::<Universe>();
            for _ in 0..9 {
                let marker = universe.autopilot_target().expect("dive continues");
                assert!(universe.open(MarkerIndex(marker)), "marker opens");
            }
            assert_eq!(universe.level().get(), 10, "the dive ends at L10");
        }
        app.update();
        assert_eq!(planet_mesh_count(&mut app), 2, "body plus one Moon, no rim");
        assert_eq!(planet_rim_count(&mut app), 0, "no rim entity survives");
        // A second frame rebuilds nothing.
        app.update();
        assert_eq!(
            planet_mesh_count(&mut app),
            2,
            "meshes persist without rebuild"
        );
        // Leaving L10 despawns every planet mesh.
        {
            let mut universe = app.world_mut().resource_mut::<Universe>();
            assert!(universe.close().is_some(), "the dive backs out");
        }
        app.update();
        assert_eq!(
            planet_mesh_count(&mut app),
            0,
            "no planet mesh survives outside L10"
        );
    }

    #[test]
    fn l14_room_spawns_no_meshes() {
        use universe_core::nav::MarkerIndex;
        let mut app = headless_app();
        // Dive the headless universe all the way to the L14 room through
        // thirteen autopilot opens (pure generation, no frames pass).
        {
            let mut universe = app.world_mut().resource_mut::<Universe>();
            for _ in 0..13 {
                let marker = universe.autopilot_target().expect("dive continues");
                assert!(universe.open(MarkerIndex(marker)), "marker opens");
            }
            assert_eq!(universe.level().get(), 14, "the dive ends at L14");
            assert!(
                universe.autopilot_target().is_none(),
                "nothing opens past L14"
            );
        }
        app.update();
        app.update();
        assert_eq!(
            planet_mesh_count(&mut app),
            0,
            "the room builds no meshes: furniture is outlines only"
        );
    }

    #[test]
    fn x_toggles_debug_axes_off_by_default() {
        let mut app = headless_app();
        assert!(
            !app.world().resource::<Navigation>().show_axes,
            "axes start hidden"
        );
        tap(&mut app, KeyCode::KeyX);
        assert!(
            app.world().resource::<Navigation>().show_axes,
            "X shows the axes"
        );
        tap(&mut app, KeyCode::KeyX);
        assert!(
            !app.world().resource::<Navigation>().show_axes,
            "X hides them again"
        );
    }

    /// One portal per form variant, drawn headless through the real systems.
    fn every_form_point(index: usize) -> universe_core::r#gen::Point {
        use universe_core::r#gen::{Form, MarkerKind, Point};
        let normal = [0.0, 0.6, 0.8];
        let form = match index % 16 {
            0 => Form::Dot,
            1 => Form::Body,
            2 => Form::Disk {
                normal,
                barred: true,
            },
            3 => Form::Spheroid,
            4 => Form::Clump,
            5 => Form::Ring {
                normal,
                inner: 0.02,
                outer: 0.05,
            },
            6 => Form::Shell,
            7 => Form::Orbit { normal },
            8 => Form::Arc { normal, span: 1.2 },
            9 => Form::Arm { normal, phase: 0.7 },
            10 => Form::Thread {
                to: [0.1, 0.0, 0.0],
            },
            11 => Form::Sheet { normal },
            12 => Form::Patch { normal },
            13 => Form::Grid {
                normal,
                curvature: 0.4,
            },
            14 => Form::Rect { half: [0.03, 0.02] },
            _ => Form::Box { height: 0.03 },
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: test grid index below 16, exactly representable"
        )]
        let lane = index as f64;
        Point {
            position: [-0.3 + 0.15 * (lane % 4.0), 0.2 - 0.1 * lane, 0.0],
            radius: 0.05,
            kind: MarkerKind::Portal,
            form,
            ..Point::bare([0.0; 3], 0.05, MarkerKind::Portal)
        }
    }

    #[test]
    fn every_form_draws_headless_through_the_pipeline() {
        use universe_core::r#gen::Generated;
        let mut app = headless_app();
        {
            let mut universe = app.world_mut().resource_mut::<Universe>();
            universe.open = Generated {
                points: (0..16).map(every_form_point).collect(),
                child_constraints: Vec::new(),
            };
        }
        // Frames draw dots, forms, previews, and siblings without a window.
        for _ in 0..5 {
            app.update();
        }
        let universe = app.world().resource::<Universe>();
        assert_eq!(universe.level(), Level::MIN, "drawing must not navigate");
        assert_eq!(universe.open.points.len(), 16, "forms must survive");
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
                    .before(draw_room_outlines),
                spy_after_draw_room
                    .after(draw_room_outlines)
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
                "after-draw-room",
                "after-draw-previews",
            ],
        );
    }
}
