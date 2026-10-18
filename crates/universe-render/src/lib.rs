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
//!
//! Navigation math, labels, and the journey replay live in `universe-core`
//! (`nav`, `labels`); this crate keeps thin resource newtypes over them.

use bevy::prelude::*;

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
mod guides_ui;
mod hud;
mod input;
mod keys_ui;
mod labels;
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
mod settings_rows;
mod settings_ui;
mod stars;
mod stream;
mod style;
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
            .init_resource::<patches::PatchTasks>()
            .init_resource::<debug_overlay::OverlayInfo>()
            .init_resource::<figure_draw::FigureState>()
            .init_resource::<BillboardState>()
            .init_resource::<ExposureLevel>()
            .init_resource::<LabelCache>()
            .add_plugins((touch_ui::TouchPlugin, player::PlayerPlugin))
            .add_systems(Startup, (spawn_hud, spawn_labels))
            .add_systems(OnExit(Screen::Universe), input::clear_hover)
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::sync_camera;
    use crate::draw::{draw_open_cell, draw_parent_siblings, draw_previews};
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

    /// Unit vector from `from` to `to`.
    fn direction(from: [f64; 3], to: [f64; 3]) -> [f64; 3] {
        universe_core::flight::ease_heading(
            [0.0; 3],
            [to[0] - from[0], to[1] - from[1], to[2] - from[2]],
            1.0,
        )
    }

    /// Largest component-wise difference between two vectors.
    fn drift(a: [f64; 3], b: [f64; 3]) -> f64 {
        a.iter()
            .zip(b)
            .map(|(x, y)| (x - y).abs())
            .fold(0.0, f64::max)
    }

    #[test]
    fn oriented_entry_carries_heading_and_up_onto_the_planet() {
        use universe_core::destination::{Destination, Way};
        use universe_core::flight::orthogonal_up;
        use universe_core::nav::MarkerIndex;
        use universe_core::nest::autopilot_path;
        let mut app = headless_app();
        // Open the fixed journey down to L9 (pure generation, no frames
        // pass): the only remaining entry is the dive onto the planet.
        for &marker in autopilot_path(DEMO_SEED)
            .get(..8)
            .expect("journey reaches L9")
        {
            assert!(
                app.world_mut()
                    .resource_mut::<Universe>()
                    .open(MarkerIndex(marker)),
                "journey marker opens"
            );
        }
        assert_eq!(app.world().resource::<Universe>().level().get(), 9);
        // The journey's planet portal: the destination aims at it, so the
        // dive opens it.
        let (target, marker_pos) = {
            let universe = app.world().resource::<Universe>();
            let marker = universe.autopilot_target().expect("planet marker");
            let pos = universe
                .marker(MarkerIndex(marker))
                .expect("portal position");
            (marker, pos)
        };
        // Park outside the open distance, looking straight at the planet
        // with a tilted up.
        let radius = app.world().resource::<Universe>().marker_radius();
        let outside = radius / OPEN_ANGLE.sin() * 1.5;
        let length = (marker_pos[0] * marker_pos[0]
            + marker_pos[1] * marker_pos[1]
            + marker_pos[2] * marker_pos[2])
            .sqrt()
            .max(1e-12);
        let start = [
            marker_pos[0] * (1.0 + outside / length),
            marker_pos[1] * (1.0 + outside / length),
            marker_pos[2] * (1.0 + outside / length),
        ];
        app.world_mut()
            .resource_mut::<Universe>()
            .path
            .set_offset(start);
        let heading = direction(start, marker_pos);
        let tilted = orthogonal_up(heading, [0.6, 0.8, 0.0]);
        // The destination is the journey itself, ending at the planet.
        let destination = Destination::seeded(DEMO_SEED, &[]);
        {
            let mut nav = app.world_mut().resource_mut::<Navigation>();
            nav.destination = Some(destination);
            nav.forward = heading;
            nav.up = tilted;
        }
        // Dive until the planet opens, tracking the pose of the frame
        // before entry.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowUp);
        let mut entered = false;
        let mut heading_before = heading;
        let mut up_before = tilted;
        for _ in 0..4000 {
            let nav = app.world().resource::<Navigation>();
            heading_before = nav.forward;
            up_before = nav.up;
            app.update();
            if app.world().resource::<Universe>().level().get() == 10 {
                entered = true;
                break;
            }
        }
        {
            let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            input.release(KeyCode::ArrowUp);
            input.clear();
        }
        assert!(entered, "the dive must enter the planet");
        let universe = app.world().resource::<Universe>();
        let nav = app.world().resource::<Navigation>();
        let opened = universe.path.entered().expect("an entry was made");
        assert_eq!(
            opened.up,
            universe_core::frame::IDENTITY_UP,
            "the planet entry keeps identity up"
        );
        // World-space continuity (#403 R1): the pose in child units, mapped
        // back through the entered frame, is the parent-units pose of the
        // frame before (the camera system never runs headless, so no
        // easing happened in between). Uncarried vectors fail by the full
        // frame rotation.
        assert!(
            drift(opened.to_parent_direction(nav.forward), heading_before) < 1e-9,
            "entry must carry the heading through the frame"
        );
        assert!(
            drift(opened.to_parent_direction(nav.up), up_before) < 1e-9,
            "entry must carry the up through the frame"
        );
        // The journey ends here (#729): the planet on the way opened, the
        // destination is kept, and arrival aims at nothing further.
        assert_eq!(opened.marker, target, "the planet on the way opened");
        assert_eq!(nav.destination, Some(destination), "entry keeps the aim");
        let next = match destination.on_the_way(&universe.path) {
            Way::Next(marker) => Some(marker),
            Way::Arrived | Way::Off => None,
        };
        assert_eq!(next, None, "arrival aims at nothing further");
        assert_eq!(nav.target, next, "entry aims at nothing further");
    }

    #[test]
    fn the_default_heading_looks_at_the_root_cell_center_upright() {
        use universe_core::frame::IDENTITY_UP;
        use universe_core::nav::START_OFFSET;
        let nav = Navigation::default();
        assert!(
            drift(nav.forward, direction(START_OFFSET, [0.0; 3])) < 1e-12,
            "the camera opens looking at the universe"
        );
        assert_eq!(nav.up, IDENTITY_UP);
    }

    #[test]
    fn manual_exit_carries_the_heading_and_keeps_aiming_at_the_marker_just_left() {
        use universe_core::destination::Destination;
        use universe_core::r#gen::MarkerKind;
        use universe_core::nav::MarkerIndex;
        let mut app = headless_app();
        // Dive four levels through autopilot opens to reach a deeper cell.
        for _ in 0..4 {
            let marker = app
                .world()
                .resource::<Universe>()
                .autopilot_target()
                .expect("dive continues");
            assert!(
                app.world_mut()
                    .resource_mut::<Universe>()
                    .open(MarkerIndex(marker)),
                "marker opens"
            );
        }
        // Lock the portal farthest from the center and settle just outside
        // its opening distance, looking straight at it.
        let (target, marker_pos) = {
            let universe = app.world().resource::<Universe>();
            let mut best: Option<(u32, [f64; 3])> = None;
            let mut farthest = 0.0f64;
            for (index, point) in universe.open.points.iter().enumerate() {
                if point.kind != MarkerKind::Portal {
                    continue;
                }
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "E-CAST: test marker index into a budgeted cell, always fits u32"
                )]
                let marker = index as u32;
                let at = universe.marker(MarkerIndex(marker)).expect("portal sits");
                let distance = (at[0] * at[0] + at[1] * at[1] + at[2] * at[2]).sqrt();
                if distance > farthest {
                    farthest = distance;
                    best = Some((marker, at));
                }
            }
            best.expect("deeper cell offers portals")
        };
        let radius = app.world().resource::<Universe>().marker_radius();
        let outside = radius / OPEN_ANGLE.sin() * 1.5;
        // Settle off to the side so the approach runs diagonally: the
        // portal ahead along the ray then differs from the nearest one.
        let start = [
            marker_pos[0] + outside,
            marker_pos[1],
            marker_pos[2] + outside,
        ];
        app.world_mut()
            .resource_mut::<Universe>()
            .path
            .set_offset(start);
        // The destination runs through the far portal (#423).
        let mut way = app.world().resource::<Universe>().path.indices();
        way.push(target);
        let destination = Destination::seeded(DEMO_SEED, &way);
        {
            let mut nav = app.world_mut().resource_mut::<Navigation>();
            nav.destination = Some(destination);
            nav.forward = direction(start, marker_pos);
        }
        // Enter through the portal on the way.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowUp);
        let mut entered = false;
        for _ in 0..2000 {
            app.update();
            if app.world().resource::<Universe>().level().get() == 6 {
                entered = true;
                break;
            }
        }
        {
            let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            input.release(KeyCode::ArrowUp);
            input.clear();
        }
        assert!(entered, "the dive must enter the next dimension");
        // Simulate a mid-glide view far from any marker, park just inside
        // the angular close distance, and dive out. (`sync_camera` needs a
        // window so it never runs headless; the contract pinned here is
        // what `handle_input` does to the heading and the target.)
        let exited = app
            .world()
            .resource::<Universe>()
            .path
            .entered()
            .expect("the entry just made");
        assert_eq!(exited.marker, target, "the portal on the way opened");
        let outward = direction([0.0; 3], [0.2, 0.9, 0.4]);
        let inside = 0.5 / CLOSE_ANGLE.sin() - 0.5;
        let gliding = direction([0.0; 3], [0.4, 0.3, 0.2]);
        let tilted = universe_core::flight::orthogonal_up(gliding, [0.0, 0.6, 0.8]);
        {
            let mut universe = app.world_mut().resource_mut::<Universe>();
            universe.path.set_offset([
                outward[0] * inside,
                outward[1] * inside,
                outward[2] * inside,
            ]);
            let mut nav = app.world_mut().resource_mut::<Navigation>();
            nav.forward = gliding;
            nav.up = tilted;
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::ArrowDown);
        }
        for _ in 0..600 {
            app.update();
            if app.world().resource::<Universe>().level().get() == 5 {
                break;
            }
        }
        {
            let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            input.release(KeyCode::ArrowDown);
            input.clear();
        }
        assert_eq!(
            app.world().resource::<Universe>().level().get(),
            5,
            "outward dives back out to the parent"
        );
        let nav = app.world().resource::<Navigation>();
        // World-space continuity (#403 R1): the heading held in child
        // units until the close, then crossed the exited frame exactly.
        assert!(
            drift(nav.forward, exited.to_parent_direction(gliding)) < 1e-9,
            "exit must carry the heading through the exited frame"
        );
        assert!(
            drift(nav.up, exited.to_parent_direction(tilted)) < 1e-9,
            "exit must carry the up through the exited frame"
        );
        // Scrolling out keeps the surface and the aim (#423): back in the
        // cell returned to, the marker on the way is the one just left.
        assert_eq!(nav.destination, Some(destination), "exit keeps the surface");
        assert_eq!(
            nav.target,
            Some(exited.marker),
            "exit aims at the marker just left"
        );
    }

    /// First root portal that is not `skip`, with its portal position.
    fn other_root_portal(app: &App, skip: u32) -> (u32, [f64; 3]) {
        use universe_core::r#gen::MarkerKind;
        let universe = app.world().resource::<Universe>();
        universe
            .open
            .points
            .iter()
            .enumerate()
            .filter(|(_, point)| point.kind == MarkerKind::Portal)
            .map(|(index, point)| {
                (
                    u32::try_from(index).expect("budgeted"),
                    point.portal_position(),
                )
            })
            .find(|&(marker, _)| marker != skip)
            .expect("root offers another portal")
    }

    /// Clicks the left mouse button on `marker` for one update.
    fn click(app: &mut App, marker: u32) {
        app.world_mut().resource_mut::<Navigation>().hover = Some(marker);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        buttons.release(MouseButton::Left);
        buttons.clear();
    }

    #[test]
    fn the_camera_starts_aimed_at_the_player() {
        // The start destination is the player (#607, Q4 A).
        let spot = universe_core::player::PlayerSpot::seeded(DEMO_SEED).expect("spot");
        let mut app = headless_app();
        app.update();
        let nav = app.world().resource::<Navigation>();
        assert_eq!(nav.destination, Some(spot.destination()));
        assert_eq!(nav.target, spot.chain().first().copied());
    }

    #[test]
    fn a_click_sets_the_seeded_surface_below_and_the_way_marker_keeps_it() {
        use universe_core::destination::Destination;
        let mut app = headless_app();
        app.update();
        let start = app.world().resource::<Navigation>().destination;
        let first = start.and_then(|held| held.markers().first().copied());
        let (other, _) = other_root_portal(&app, first.expect("journey"));
        click(&mut app, other);
        let nav = *app.world().resource::<Navigation>();
        assert_eq!(
            nav.destination,
            Some(Destination::seeded(DEMO_SEED, &[other]))
        );
        assert_eq!(nav.target, Some(other));
        click(&mut app, other);
        assert_eq!(
            app.world().resource::<Navigation>().destination,
            nav.destination,
            "clicking the marker on the way keeps the surface"
        );
    }

    #[test]
    fn scrolling_in_through_a_marker_off_the_way_does_not_enter_it() {
        use universe_core::nav::MarkerIndex;
        let mut app = headless_app();
        app.update();
        let first = app.world().resource::<Navigation>().target.expect("way");
        let at = app
            .world()
            .resource::<Universe>()
            .marker(MarkerIndex(first));
        let at = at.expect("way marker");
        let (other, other_at) = other_root_portal(&app, first);
        // Park beyond the other marker, on the line from the way marker,
        // so the dive toward the way crosses it.
        let back = direction(at, other_at);
        let radius = app.world().resource::<Universe>().marker_radius();
        let beyond = radius / OPEN_ANGLE.sin() * 1.2;
        app.world_mut().resource_mut::<Universe>().path.set_offset([
            other_at[0] + back[0] * beyond,
            other_at[1] + back[1] * beyond,
            other_at[2] + back[2] * beyond,
        ]);
        hold_until_level(&mut app, KeyCode::ArrowUp, 2, 20_000);
        let entered = app.world().resource::<Universe>().path.entered();
        assert_ne!(entered.map(|opened| opened.marker), Some(other));
        assert_eq!(entered.map(|opened| opened.marker), Some(first));
    }

    #[test]
    fn the_autopilot_stops_once_the_surface_is_reached() {
        use crate::input::Autopilot;
        use universe_core::destination::Way;
        use universe_core::nav::MarkerIndex;
        let mut app = headless_app();
        app.update();
        let way = app
            .world()
            .resource::<Navigation>()
            .destination
            .expect("set");
        for &marker in way.markers() {
            assert!(
                app.world_mut()
                    .resource_mut::<Universe>()
                    .open(MarkerIndex(marker))
            );
        }
        tap(&mut app, KeyCode::Space);
        assert_eq!(*app.world().resource::<Autopilot>(), Autopilot::Idle);
        let nav = app.world().resource::<Navigation>();
        assert_eq!(nav.target, None, "no marker left: the surface is reached");
        let path = &app.world().resource::<Universe>().path;
        assert_eq!(way.on_the_way(path), Way::Arrived);
    }

    #[test]
    fn a_saved_view_brings_its_destination_back() {
        let mut app = headless_app();
        app.update();
        let saved = app.world().resource::<Navigation>().destination;
        chord(&mut app, KeyCode::ControlLeft, KeyCode::Digit1);
        let first = saved.and_then(|held| held.markers().first().copied());
        let (other, _) = other_root_portal(&app, first.expect("journey"));
        click(&mut app, other);
        assert_ne!(app.world().resource::<Navigation>().destination, saved);
        // Plain 1 no longer recalls (#557): it enters normal mode (#558).
        player::player_here(&mut app);
        tap(&mut app, KeyCode::Digit1);
        assert_eq!(
            app.world().resource::<Flight>().mode,
            input::FlightMode::Normal,
            "key 1 enters normal mode"
        );
        chord(&mut app, KeyCode::AltLeft, KeyCode::Digit1);
        assert_eq!(app.world().resource::<Navigation>().destination, saved);
    }

    /// Holds `key` until the open level is `level` or `frames` run out.
    fn hold_until_level(app: &mut App, key: KeyCode, level: u8, frames: usize) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        for _ in 0..frames {
            app.update();
            if app.world().resource::<Universe>().level().get() == level {
                break;
            }
        }
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release(key);
        input.clear();
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
        chord(&mut app, KeyCode::AltLeft, KeyCode::Digit3);
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

    /// Counts patch-dome entities (#770: patch removed, always zero).
    fn patch_count(app: &mut App) -> usize {
        let _ = app;
        0
    }

    /// Counts LOD patch entities (#776).
    fn lod_patch_count(app: &mut App) -> usize {
        use crate::patches::PatchPiece;
        let mut state = app.world_mut().query_filtered::<Entity, With<PatchPiece>>();
        state.iter(app.world()).count()
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
        // #776 T3: LOD patches draw the sphere plus the Moon (six roots
        // synchronously, then refined by workers).
        assert!(
            lod_patch_count(&mut app) >= 6,
            "LOD roots draw at once, got {}",
            lod_patch_count(&mut app)
        );
        assert!(
            planet_mesh_count(&mut app) >= 1,
            "the moons stay, got {}",
            planet_mesh_count(&mut app)
        );
        assert_eq!(patch_count(&mut app), 0, "#770: no ground, no patch dome");
        assert_eq!(planet_rim_count(&mut app), 0, "no rim entity survives");
        // A second frame rebuilds nothing from scratch: patches persist (they
        // only refine) and moons persist.
        let patches_before = lod_patch_count(&mut app);
        let moons_before = planet_mesh_count(&mut app);
        app.update();
        assert!(
            lod_patch_count(&mut app) >= patches_before,
            "patches persist without rebuild"
        );
        assert_eq!(
            planet_mesh_count(&mut app),
            moons_before,
            "moons persist without rebuild"
        );
        // Leaving L10 despawns every planet mesh and patch.
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
        assert_eq!(
            lod_patch_count(&mut app),
            0,
            "no LOD patch survives outside L10"
        );
    }

    #[test]
    fn l9_far_planet_spawns_no_dome() {
        use universe_core::nav::MarkerIndex;
        let mut app = headless_app();
        // Dive the headless universe to the L9 system through eight
        // autopilot opens (pure generation, no frames pass).
        {
            let mut universe = app.world_mut().resource_mut::<Universe>();
            for _ in 0..8 {
                let marker = universe.autopilot_target().expect("dive continues");
                assert!(universe.open(MarkerIndex(marker)), "marker opens");
            }
            assert_eq!(universe.level().get(), 9, "the dive parks at L9");
        }
        app.update();
        app.update();
        assert_eq!(
            planet_mesh_count(&mut app),
            0,
            "far planets build no meshes"
        );
        assert_eq!(patch_count(&mut app), 0, "far planets draw no dome");
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
        let form = match index % 14 {
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
                via: [-0.1, 0.0, 0.0],
            },
            11 => Form::Sheet { normal },
            12 => Form::Patch { normal },
            _ => Form::Box { height: 0.03 },
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: test grid index below 14, exactly representable"
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
                points: (0..14).map(every_form_point).collect(),
                child_constraints: Vec::new(),
            };
        }
        // Frames draw dots, forms, previews, and siblings without a window.
        for _ in 0..5 {
            app.update();
        }
        let universe = app.world().resource::<Universe>();
        assert_eq!(universe.level(), Level::MIN, "drawing must not navigate");
        assert_eq!(universe.open.points.len(), 14, "forms must survive");
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
