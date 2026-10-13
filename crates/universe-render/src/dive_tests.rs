//! Dive pipeline tests: plugin wiring, system order and input (moved
//! out of `lib.rs` for its size limit, #626).

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
    // Scrolling out keeps the room and the aim (#423): back in the
    // cell returned to, the marker on the way is the one just left.
    assert_eq!(nav.destination, Some(destination), "exit keeps the room");
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
fn a_click_sets_the_seeded_room_below_and_the_way_marker_keeps_it() {
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
        "clicking the marker on the way keeps the room"
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
fn the_autopilot_stops_once_the_room_is_reached() {
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
    assert_eq!(nav.target, None, "no marker left: the room is reached");
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

/// Counts surface-mesh entities through a detached query state.
fn surface_count(app: &mut App) -> usize {
    use crate::surface_mesh::SurfaceMesh;
    let mut state = app
        .world_mut()
        .query_filtered::<Entity, With<SurfaceMesh>>();
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
    assert_eq!(planet_mesh_count(&mut app), 1, "one Moon, no rim");
    assert_eq!(
        surface_count(&mut app),
        1,
        "the body is the icosphere surface"
    );
    assert_eq!(planet_rim_count(&mut app), 0, "no rim entity survives");
    // A second frame rebuilds nothing.
    app.update();
    assert_eq!(
        planet_mesh_count(&mut app),
        1,
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
