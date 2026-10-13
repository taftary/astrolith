//! Window-path dives for the continuous camera (#430).
//!
//! Test-only module (`E-FILE-SIZE`: `lib.rs` is past the split line).
//! Headless apps driving the real input pipeline: manual wheel dives land
//! down the tail with bounded roll and turn and above the ground, free
//! flight holds the standoff, and the autopilot keeps the replay mode.
//! `sync_camera` never runs headless, so the tests replicate its easing
//! (`next_heading` at `LOOK_RATE`, `next_up` toward `local_up` at
//! `UP_RATE`) at a fixed 60 Hz beside the driven camera path.
//!
//! Marker design-system acceptance (#446) lives here too: portal rings at
//! three rungs, and ranked click cycling with selection rings at the floor.

use super::*;
use bevy::asset::AssetPlugin;
use bevy::ecs::message::Messages;
use bevy::gizmos::{
    GizmoAsset, GizmoHandles,
    config::{DefaultGizmoConfigGroup, GizmoConfigStore},
};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::input::touch::TouchPhase;
use bevy::mesh::Mesh;
use bevy::pbr::StandardMaterial;
use bevy::prelude::{App, AppExit, ButtonInput, Entity, KeyCode, MinimalPlugins, MouseButton};
use universe_core::coords::Level;
use universe_core::destination::Destination;
use universe_core::flight::ease_heading;
use universe_core::frame::local_up;
use universe_core::nav::{DEMO_SEED, DiveMode, MarkerIndex};

use crate::DivePlugin;
use crate::Universe;
use crate::camera::{look_point, next_heading, next_up};
use crate::input::{Autopilot, ClickCycle, CursorPortals, Flight, FlightMode, Navigation};
use crate::tokens::{CYCLE_CAP, SELECTION_RING_SCALE, SURFACE_EDGE_SQUARES_TINT};

/// Fixed step for the replicated camera easing: a 60 Hz window.
const DT: f64 = 1.0 / 60.0;

/// Headless app running the dive pipeline without a window or GPU.
///
/// Same plugin set as the `lib.rs` harness: `MinimalPlugins` plus
/// `AssetPlugin`, the exit/wheel/motion messages, defaulted button inputs,
/// gizmo storage for the buffered draw param, and `DivePlugin`.
pub(crate) fn headless_app() -> App {
    use bevy::input::mouse::MouseMotion;
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

/// Angle between two vectors, in radians.
fn angle_between(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dot = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
    let la = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    let lb = (b[0] * b[0] + b[1] * b[1] + b[2] * b[2]).sqrt();
    if !la.is_finite() || !lb.is_finite() || la <= 0.0 || lb <= 0.0 {
        return 0.0;
    }
    dot.acos()
}

/// Scrolls `y` wheel lines inward for exactly one update.
///
/// Fractional lines take fine steps (`y = 1.0` is one full notch, `0.75`
/// of the gap); the tail dive below uses small steps so the replicated
/// 60 Hz easing tracks a smooth camera like the real window.
fn notch_in(app: &mut App, y: f32) {
    app.world_mut()
        .resource_mut::<Messages<MouseWheel>>()
        .write(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y,
            window: Entity::PLACEHOLDER,
            phase: TouchPhase::Moved,
        });
    app.update();
}

/// Level of the app's open cell.
fn level(app: &App) -> u8 {
    app.world().resource::<Universe>().level().get()
}

/// Camera above the bare surface at the app's open cell (#430).
///
/// The hard requirement behind the clearance: never under the ground. Uses the surface itself rather than the standoff, since a
/// landing rest can sit below the standoff at deep tail levels.
fn above_floor(app: &App) -> bool {
    use universe_core::ground::ground_height;
    use universe_core::terrain::surface_radius;
    let universe = app.world().resource::<Universe>();
    let offset = universe.path.offset();
    if !offset.iter().all(|c| c.is_finite()) {
        return false;
    }
    match universe.level().get() {
        10 => {
            let reach =
                (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
            reach + 1e-9 >= surface_radius(offset)
        }
        11..=13 => universe
            .frame
            .as_ref()
            .is_some_and(|frame| offset[1] + 1e-9 >= ground_height(frame, offset[0], offset[2])),
        _ => true,
    }
}

#[test]
fn manual_dives_land_but_the_autopilot_keeps_the_replay_line() {
    use crate::input::dive_mode;
    assert_eq!(dive_mode(&Autopilot::Flying), DiveMode::Targeted);
    assert_eq!(dive_mode(&Autopilot::Idle), DiveMode::Landing);
}

#[test]
fn window_dive_lands_down_the_tail_with_bounded_roll_and_turn() {
    use universe_core::destination::Way;
    use universe_core::frame::IDENTITY_UP;
    use universe_core::nest::{OPEN_ANGLE, autopilot_path};
    let mut app = headless_app();
    // Open the fixed journey down to the L10 planet cell (pure generation,
    // no frames pass), park at the entry rest above the relief, then aim at
    // the journey's room. Direct opens never move the camera, so without
    // the parking the first dive would close straight back out.
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        for &marker in autopilot_path(DEMO_SEED)
            .get(..9)
            .expect("journey reaches L10")
        {
            assert!(universe.open(MarkerIndex(marker)), "journey marker opens");
        }
        assert_eq!(universe.level().get(), 10);
        universe.path.set_offset([0.0, 0.0, 3.0]);
    }
    let destination = Destination::seeded(DEMO_SEED, &[]);
    {
        let mut nav = app.world_mut().resource_mut::<Navigation>();
        nav.destination = Some(destination);
        nav.up = IDENTITY_UP;
    }
    // One dry frame derives the marker on the way; aim at it before the
    // loop, the way the window always flies aimed (boot aims at the root
    // center, and every dive keeps its aim, so the test starts settled).
    notch_in(&mut app, 0.0);
    {
        let (camera, target) = {
            let universe = app.world().resource::<Universe>();
            let nav = app.world().resource::<Navigation>();
            (
                universe.path.offset(),
                nav.target.and_then(|m| universe.marker(MarkerIndex(m))),
            )
        };
        let at = target.expect("the dive aims at the next marker on the way");
        let aimed = ease_heading(
            [0.0; 3],
            [at[0] - camera[0], at[1] - camera[1], at[2] - camera[2]],
            1.0,
        );
        app.world_mut().resource_mut::<Navigation>().forward = aimed;
    }
    // Replicated window pose beside the driven camera: carried through
    // entries like the input system, eased like the camera system. Starts
    // perpendicular, the steady state the window holds after boot (the
    // first perpendicular snap has no previous frame to bound it).
    let mut forward = app.world().resource::<Navigation>().forward;
    let mut up =
        universe_core::flight::orthogonal_up(forward, app.world().resource::<Navigation>().up);
    let mut seen = [false; 4];
    let mut previous_level = 10u8;
    let mut max_roll = 0.0f64;
    let mut max_turn = 0.0f64;
    let mut max_entry_turn = 0.0f64;
    for _ in 0..6000 {
        notch_in(&mut app, 0.05);
        let universe = app.world().resource::<Universe>().0.clone();
        let nav = *app.world().resource::<Navigation>();
        let now = universe.level().get();
        assert!(now >= previous_level, "scrolling in must never close");
        let entered = now != previous_level;
        if entered {
            // Entry: carry the replicated pose through the entered frame
            // first (exact, like the input system), then ease below.
            if let Some(opened) = universe.path.entered() {
                forward = opened.to_child_direction(forward);
                up = opened.to_child_direction(up);
            }
            previous_level = now;
            // A landing ends on the surface normal at the entry rest: in
            // child units that is straight up at about 3.58 units, through
            // the real window path at every tail level.
            let offset = universe.path.offset();
            let reach =
                (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
            let rest = 0.5 / OPEN_ANGLE.sin();
            assert!(
                (reach - rest).abs() / rest < 0.05,
                "L{now} entry rests at about 3.58 child units: {reach} vs {rest}"
            );
            assert!(
                offset[1] / reach > 0.99,
                "L{now} entry stands on the normal: {offset:?}"
            );
        }
        if (11..=14).contains(&now) {
            seen[usize::from(now - 11)] = true;
        }
        // Replicate one 60 Hz camera frame toward this frame's aim.
        let arrived = destination.on_the_way(&universe.path) == Way::Arrived;
        let target = look_point(
            nav.target.and_then(|m| universe.marker(MarkerIndex(m))),
            arrived,
        );
        let camera = universe.path.offset();
        let next_forward = next_heading(forward, camera, target, DT);
        let turn = angle_between(forward, next_forward);
        max_turn = max_turn.max(turn);
        if entered {
            max_entry_turn = max_entry_turn.max(turn);
        }
        forward = next_forward;
        let next_up = next_up(
            forward,
            up,
            local_up(Level::new(now).expect("tail level"), camera),
            DT,
        );
        max_roll = max_roll.max(angle_between(up, next_up));
        up = next_up;
        assert!(above_floor(&app), "the window dive stays above the ground");
        if now == 14 {
            break;
        }
    }
    // No snap anywhere: the easing moves a fraction per frame, so one frame
    // stays an order below a radian. The roll bound carries slack for the
    // perpendicular coupling against the faster heading; the turn bound is
    // the LOOK_RATE fraction of the largest retarget seen (entry frames
    // settle fast by design, still far from a snap).
    assert!(max_roll < 0.05, "no up step over the dive: {max_roll}");
    assert!(max_turn < 0.15, "no heading snap over the dive: {max_turn}");
    assert!(
        max_entry_turn < 0.15,
        "entry hands over without a snap: {max_entry_turn}"
    );
    assert_eq!(seen, [true; 4], "the window dive opens L11-L14 in order");
    assert_eq!(level(&app), 14, "the dive reaches the room");
}

#[test]
fn window_free_flight_passes_through_the_ground() {
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;
    use universe_core::ground::ground_height;
    use universe_core::nest::autopilot_path;
    let mut app = headless_app();
    // Fixed frames, so the flight covers the same ground on every machine.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        16,
    )));
    // Open the fixed journey down to the L11 region cell.
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        for &marker in autopilot_path(DEMO_SEED)
            .get(..10)
            .expect("journey reaches L11")
        {
            assert!(universe.open(MarkerIndex(marker)), "journey marker opens");
        }
        assert_eq!(universe.level().get(), 11);
    }
    let frame = app
        .world()
        .resource::<Universe>()
        .frame
        .clone()
        .expect("surface frame");
    let start = [0.2, ground_height(&frame, 0.2, 0.1) + 0.02, 0.1];
    app.world_mut()
        .resource_mut::<Universe>()
        .path
        .set_offset(start);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyF);
    app.update();
    {
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release(KeyCode::KeyF);
        input.clear();
    }
    assert_eq!(app.world().resource::<Flight>().mode, FlightMode::Free);
    {
        let mut flight = app.world_mut().resource_mut::<Flight>();
        flight.pitch = -1.0;
        flight.step = 9;
    }
    // Fly down: free mode passes through everything, the ground included
    // (#558, Q7), and never opens a cell.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    let mut below = false;
    for _ in 0..500 {
        app.update();
        let universe = app.world().resource::<Universe>();
        assert_eq!(universe.level().get(), 11, "free flight never opens");
        let offset = universe.path.offset();
        if offset[1] < ground_height(&frame, offset[0], offset[2]) {
            below = true;
            break;
        }
    }
    assert!(below, "free mode flies under the ground");
}

/// Moon-mesh entities live in the headless app (the planet body is the
/// surface mesh since ADR 0024).
fn planet_mesh_count(app: &mut App) -> usize {
    use crate::planet::PlanetMesh;
    let mut state = app.world_mut().query_filtered::<Entity, With<PlanetMesh>>();
    state.iter(app.world()).count()
}

/// Surface-mesh entities: exactly one on L10-L13, none elsewhere (ADR 0024).
fn surface_count(app: &mut App) -> usize {
    use crate::surface_mesh::SurfaceMesh;
    let mut state = app
        .world_mut()
        .query_filtered::<Entity, With<SurfaceMesh>>();
    state.iter(app.world()).count()
}

/// Spawned surface edge-line entities (#593).
fn edge_line_count(app: &mut App) -> usize {
    use crate::surface_mesh::SurfaceEdges;
    let mut state = app
        .world_mut()
        .query_filtered::<Entity, With<SurfaceEdges>>();
    state.iter(app.world()).count()
}

/// Expected surface meshes at `level`: one on the surface levels.
fn surface_expected(level: u8) -> usize {
    usize::from((10..=13).contains(&level))
}

/// Headless app opened down the fixed journey to `depth` markers, parked at
/// the entry rest and aimed at the journey's room.
fn journey_app(depth: usize) -> App {
    use universe_core::destination::Destination;
    use universe_core::frame::IDENTITY_UP;
    use universe_core::nest::autopilot_path;
    let mut app = headless_app();
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        for &marker in autopilot_path(DEMO_SEED)
            .get(..depth)
            .expect("journey reaches its depth")
        {
            assert!(universe.open(MarkerIndex(marker)), "journey marker opens");
        }
        universe.path.set_offset([0.0, 0.0, 3.0]);
    }
    {
        let mut nav = app.world_mut().resource_mut::<Navigation>();
        nav.destination = Some(Destination::seeded(DEMO_SEED, &[]));
        nav.up = IDENTITY_UP;
    }
    // One dry frame derives the marker on the way; aim at it, the way the
    // window always flies aimed.
    notch_in(&mut app, 0.0);
    {
        let (camera, target) = {
            let universe = app.world().resource::<Universe>();
            let nav = app.world().resource::<Navigation>();
            (
                universe.path.offset(),
                nav.target.and_then(|m| universe.marker(MarkerIndex(m))),
            )
        };
        let at = target.expect("the dive aims at the next marker on the way");
        app.world_mut().resource_mut::<Navigation>().forward = ease_heading(
            [0.0; 3],
            [at[0] - camera[0], at[1] - camera[1], at[2] - camera[2]],
            1.0,
        );
    }
    app
}

/// The spawned surface mesh entity, if any.
fn surface_entity(app: &mut App) -> Option<Entity> {
    use crate::surface_mesh::SurfaceMesh;
    let mut state = app
        .world_mut()
        .query_filtered::<Entity, With<SurfaceMesh>>();
    state.iter(app.world()).next()
}

#[test]
fn the_surface_rebuilds_on_a_turn_only_when_past_ten_degrees() {
    use crate::input::Navigation;
    let mut app = headless_app();
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        for _ in 0..10 {
            let marker = universe.autopilot_target().expect("dive continues");
            assert!(universe.open(MarkerIndex(marker)), "marker opens");
        }
    }
    app.update();
    let first = surface_entity(&mut app).expect("a surface");
    app.update();
    assert_eq!(
        surface_entity(&mut app),
        Some(first),
        "a still camera rebuilds nothing"
    );
    // A small turn (5 degrees) keeps the mesh; a 20 degree turn rebuilds.
    let turn = |app: &mut App, degrees: f64| {
        let mut nav = app.world_mut().resource_mut::<Navigation>();
        let [x, y, z] = nav.forward;
        let (sin, cos) = degrees.to_radians().sin_cos();
        nav.forward = [x * cos - z * sin, y, x * sin + z * cos];
    };
    turn(&mut app, 5.0);
    app.update();
    assert_eq!(
        surface_entity(&mut app),
        Some(first),
        "a small turn keeps it"
    );
    turn(&mut app, 15.0);
    app.update();
    let turned = surface_entity(&mut app).expect("a surface");
    assert_ne!(turned, first, "a turn past 10 degrees rebuilds");
    // The target no longer matters to the split (#598 Spec v2); the turn
    // rule itself is `surface_mesh::view_changed` (unit-tested there).
    app.update();
    assert_eq!(surface_entity(&mut app), Some(turned), "settled again");
}

#[test]
fn an_approached_planet_shows_its_surface_only_in_the_last_part_of_the_approach() {
    let mut app = headless_app();
    let (at, ratio) = {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        for _ in 0..8 {
            let marker = universe.autopilot_target().expect("dive continues");
            assert!(universe.open(MarkerIndex(marker)), "marker opens");
        }
        assert_eq!(universe.level().get(), 9, "the level above the planet");
        let marker = universe.autopilot_target().expect("a planet");
        let at = universe.open.points[marker as usize].portal_position();
        let ratio = universe_core::nest::child_ratio(universe.level()).expect("ratio");
        (at, ratio)
    };
    let park = |app: &mut App, share: f64| {
        let distance = 0.5 * ratio / (OPEN_ANGLE * share).sin();
        app.world_mut().resource_mut::<Universe>().path.set_offset([
            at[0],
            at[1],
            at[2] + distance,
        ]);
        app.update();
    };
    park(&mut app, 0.3);
    assert_eq!(surface_count(&mut app), 0, "far away: only the marker");
    park(&mut app, 0.8);
    assert_eq!(
        surface_count(&mut app),
        1,
        "mid-way: the planet surface fades in"
    );
    assert_eq!(edge_line_count(&mut app), 1, "with its edge lines");
    park(&mut app, 0.3);
    assert_eq!(surface_count(&mut app), 0, "backing out hides it again");
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        assert!(universe.close().is_some(), "out to L8");
    }
    app.update();
    assert_eq!(surface_count(&mut app), 0, "no planet surface at L8");
}

#[test]
fn l11_open_cell_spawns_one_surface_and_leaving_despawns_it() {
    let mut app = headless_app();
    // Dive the headless universe to the L11 region cell through ten
    // autopilot opens (pure generation, no frames pass).
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        for _ in 0..10 {
            let marker = universe.autopilot_target().expect("dive continues");
            assert!(universe.open(MarkerIndex(marker)), "marker opens");
        }
        assert_eq!(universe.level().get(), 11, "the dive ends at L11");
    }
    app.update();
    assert_eq!(
        surface_count(&mut app),
        1,
        "one icosphere surface, no discs"
    );
    assert_eq!(edge_line_count(&mut app), 1, "its edges show as lines");
    // A second frame with a still camera rebuilds nothing.
    app.update();
    assert_eq!(surface_count(&mut app), 1, "the mesh persists");
    // Backing out to L10 keeps one surface (the planet), and leaving the
    // surface levels despawns it.
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        assert!(universe.close().is_some(), "the dive backs out");
    }
    app.update();
    assert_eq!(
        surface_count(&mut app),
        1,
        "the planet is the same mesh kind"
    );
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        assert!(universe.close().is_some(), "the dive backs out to L9");
    }
    app.update();
    assert_eq!(surface_count(&mut app), 0, "no surface mesh off L10-L13");
    assert_eq!(edge_line_count(&mut app), 0, "and no edge lines");
}

#[test]
fn tail_round_trips_keep_parent_context_and_no_new_meshes() {
    // Each pair dives in and back out through the real window path: the
    // entries land, the exits re-aim at the marker just left, parent
    // context and the surface hold every frame, moons mesh only at L10,
    // and the one icosphere surface is the only mesh at L10-L13 (no discs,
    // no separate body; the room builds none).
    for (depth, inside) in [(9usize, 11u8), (11, 13), (12, 14)] {
        let mut app = journey_app(depth);
        let home = level(&app);
        assert_eq!(planet_mesh_count(&mut app), usize::from(home == 10));
        assert_eq!(surface_count(&mut app), surface_expected(home));
        for _ in 0..600 {
            notch_in(&mut app, 0.5);
            assert!(above_floor(&app), "the dive stays above the ground");
            if level(&app) == inside {
                break;
            }
        }
        assert_eq!(level(&app), inside, "the window dive opens L{inside}");
        assert_eq!(planet_mesh_count(&mut app), 0, "no moon outside L10");
        assert_eq!(surface_count(&mut app), surface_expected(inside));
        let entered = app
            .world()
            .resource::<Universe>()
            .path
            .entered()
            .expect("the entry just made");
        for _ in 0..60 {
            notch_in(&mut app, -1.0);
            assert!(above_floor(&app), "backing out stays above the ground");
            if level(&app) == home {
                break;
            }
        }
        assert_eq!(level(&app), home, "the dive backs out to L{home}");
        assert_eq!(
            app.world().resource::<Navigation>().target,
            Some(entered.marker),
            "exit aims at the marker just left"
        );
        assert_eq!(planet_mesh_count(&mut app), usize::from(home == 10));
        assert_eq!(surface_count(&mut app), surface_expected(home));
    }
}

/// Builds a cycle-cap order holding `markers` first (#446 T7).
fn staged_order(markers: &[u32]) -> [u32; CYCLE_CAP] {
    let mut order = [0; CYCLE_CAP];
    for (slot, &marker) in order.iter_mut().zip(markers) {
        *slot = marker;
    }
    order
}

/// Presses the left mouse button for one update (cursor order staged).
fn press(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    buttons.release(MouseButton::Left);
    buttons.clear();
}

#[test]
fn portal_rings_draw_headless_at_l1_l6_and_l11() {
    use crate::draw::portal_ring;
    use universe_core::r#gen::MarkerKind;
    use universe_core::nav::MarkerIndex;
    let mut app = headless_app();
    // Walk the journey, drawing rings at three rungs: portals carry a ring
    // at their true position, populations carry none (#446 T4).
    for want_level in [1u8, 6, 11] {
        while app.world().resource::<Universe>().level().get() < want_level {
            let marker = app
                .world()
                .resource::<Universe>()
                .autopilot_target()
                .expect("journey continues");
            assert!(
                app.world_mut()
                    .resource_mut::<Universe>()
                    .open(MarkerIndex(marker)),
                "journey marker opens"
            );
        }
        for _ in 0..3 {
            app.update();
        }
        let universe = app.world().resource::<Universe>();
        assert_eq!(universe.level().get(), want_level);
        let mut portals = 0u32;
        for point in &universe.open.points {
            if point.kind == MarkerKind::Portal {
                portals += 1;
                let (centre, _) = portal_ring(point, 0.05, 3.0).expect("portal carries a ring");
                assert_eq!(centre, point.portal_position());
            } else {
                assert_eq!(portal_ring(point, 0.05, 3.0), None);
            }
        }
        assert!(portals > 0, "L{want_level} offers portals");
    }
}

#[test]
fn repeated_clicks_cycle_ranked_portals_and_rings_draw_at_the_floor() {
    use universe_core::r#gen::MarkerKind;
    let mut app = headless_app();
    app.update();
    // Two portals in the open cell: the ranked head wins first (#446 T7).
    let portals: Vec<u32> = app
        .world()
        .resource::<Universe>()
        .open
        .points
        .iter()
        .enumerate()
        .filter(|(_, point)| point.kind == MarkerKind::Portal)
        .map(|(index, _)| u32::try_from(index).expect("budgeted"))
        .take(3)
        .collect();
    assert!(portals.len() >= 2, "root offers portals to cycle");
    let (first, second) = (portals[0], portals[1]);
    let at = [100.0, 100.0];
    // Fresh click on [second, first] picks the ranked head.
    app.world_mut()
        .resource_mut::<CursorPortals>()
        .clone_from(&CursorPortals {
            pos: at,
            order: staged_order(&[second, first]),
            len: 2,
            valid: true,
        });
    press(&mut app);
    assert_eq!(app.world().resource::<Navigation>().target, Some(second));
    let cycle = *app.world().resource::<ClickCycle>();
    assert_eq!(
        (cycle.pos, cycle.picked, cycle.armed),
        (at, Some(second), true)
    );
    // Same spot cycles to the next portal, then wraps around.
    press(&mut app);
    assert_eq!(app.world().resource::<Navigation>().target, Some(first));
    press(&mut app);
    assert_eq!(app.world().resource::<Navigation>().target, Some(second));
    // Moving the pointer away resets to the ranked head.
    let away = [300.0, 300.0];
    app.world_mut()
        .resource_mut::<CursorPortals>()
        .clone_from(&CursorPortals {
            pos: away,
            order: staged_order(&[first, second]),
            len: 2,
            valid: true,
        });
    press(&mut app);
    assert_eq!(app.world().resource::<Navigation>().target, Some(first));
    // Selection and hover rings draw through the pipeline without
    // navigating: the level holds while target and hover are set.
    const {
        assert!(SELECTION_RING_SCALE > 1.0);
        assert!(CYCLE_CAP >= 2);
    }
    let open = app.world().resource::<Universe>().level();
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(app.world().resource::<Universe>().level(), open);
}

/// Every portal label slot is hidden and the hover is cleared behind each
/// screen, even when a portal was hovered and its label showing (#473 Spec
/// v4, criterion 19).
#[test]
fn no_label_shows_behind_any_screen_even_when_hovered() {
    use crate::labels::PortalLabel;
    use crate::menu::Screen;
    use bevy::prelude::{NextState, Visibility, With};
    for screen in [Screen::Menu, Screen::Settings, Screen::Guides] {
        let mut app = headless_app();
        app.update();
        app.world_mut().resource_mut::<Navigation>().hover = Some(0);
        let mut slots = app
            .world_mut()
            .query_filtered::<&mut Visibility, With<PortalLabel>>();
        for mut visibility in slots.iter_mut(app.world_mut()) {
            *visibility = Visibility::Visible;
        }
        app.world_mut()
            .resource_mut::<NextState<Screen>>()
            .set(screen);
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<Navigation>().hover,
            None,
            "{screen:?}: hover cleared on leaving the universe"
        );
        let mut slots = app
            .world_mut()
            .query_filtered::<&Visibility, With<PortalLabel>>();
        assert!(slots.iter(app.world()).count() > 0, "label pool exists");
        assert!(
            slots
                .iter(app.world())
                .all(|visibility| *visibility == Visibility::Hidden),
            "{screen:?}: every label slot hidden"
        );
    }
}

/// UI roots stack by layer, not by spawn order: universe-view nodes (HUD,
/// labels, menu button) sit below every screen card (#473 Spec v3, criterion 19).
#[test]
fn ui_roots_stack_universe_below_screens() {
    use crate::tokens::{LAYER_SCREEN, LAYER_UNIVERSE};
    use bevy::prelude::{GlobalZIndex, Name};
    let mut app = headless_app();
    app.update();
    let mut query = app.world_mut().query::<(&Name, &GlobalZIndex)>();
    let roots: Vec<(String, i32)> = query
        .iter(app.world())
        .map(|(name, z)| (name.as_str().to_string(), z.0))
        .collect();
    let layer = |prefix: &str| -> Vec<i32> {
        roots
            .iter()
            .filter(|(name, _)| name.starts_with(prefix))
            .map(|(_, z)| *z)
            .collect()
    };
    for name in ["scale-hud", "portal-label", "menu-button"] {
        let found = layer(name);
        assert!(!found.is_empty(), "{name} has a layer");
        assert!(found.iter().all(|z| *z == LAYER_UNIVERSE), "{name}");
    }
    for name in ["menu-screen", "panel-"] {
        let found = layer(name);
        assert!(!found.is_empty(), "{name} has a layer");
        assert!(found.iter().all(|z| *z == LAYER_SCREEN), "{name}");
    }
    // Pop-ups: veil, then card, both above every screen.
    use crate::tokens::{LAYER_DIM, LAYER_POPUP, LAYER_POPUP_DIM};
    assert_eq!(layer("dim-veil"), [LAYER_DIM]);
    assert_eq!(layer("quit-veil"), [LAYER_POPUP_DIM]);
    assert_eq!(layer("quit-popup"), [LAYER_POPUP]);
    const { assert!(LAYER_UNIVERSE < LAYER_DIM && LAYER_DIM < LAYER_SCREEN) };
    const { assert!(LAYER_SCREEN < LAYER_POPUP_DIM && LAYER_POPUP_DIM < LAYER_POPUP) };
}

/// Every text on the Menu, Settings, and Guides screens has a role and is set
/// in a bundled font (Inter, or Lucide for icons), and no placeholder letter
/// remains (#473 Spec v3, criteria 22-23).
#[test]
fn screen_texts_use_bundled_fonts_and_icons() {
    use crate::fonts::{TextRole, UiFonts};
    use crate::menu::{MenuButton, MenuRoot, Screen};
    use crate::panel::PanelRoot;
    use bevy::prelude::{ChildOf, NextState, Or, Text, With};
    use bevy::text::{Font, FontSource, TextFont};
    for screen in [Screen::Menu, Screen::Settings, Screen::Guides] {
        let mut app = headless_app();
        app.init_asset::<Font>();
        app.update();
        app.world_mut()
            .resource_mut::<NextState<Screen>>()
            .set(screen);
        app.update();
        app.update();
        let fonts = app.world().resource::<UiFonts>().clone();
        assert_ne!(fonts.text, fonts.icons, "fonts were loaded");
        let world = app.world_mut();
        let mut roots = world
            .query_filtered::<Entity, Or<(With<MenuRoot>, With<PanelRoot>, With<MenuButton>)>>();
        let roots: Vec<Entity> = roots.iter(world).collect();
        let mut links = world.query::<(Entity, &ChildOf)>();
        let parents: Vec<(Entity, Entity)> = links
            .iter(world)
            .map(|(child, link)| (child, link.parent()))
            .collect();
        let under_screen = |mut entity: Entity| loop {
            if roots.contains(&entity) {
                return true;
            }
            match parents.iter().find(|(child, _)| *child == entity) {
                Some((_, parent)) => entity = *parent,
                None => return false,
            }
        };
        let mut texts = world.query::<(
            Entity,
            &Text,
            Option<&TextRole>,
            &TextFont,
            Has<keys_ui::KeyText>,
        )>();
        let mut checked = 0;
        for (entity, text, role, font, key) in texts.iter(world) {
            if !under_screen(entity) {
                continue;
            }
            checked += 1;
            let role = role.unwrap_or_else(|| panic!("{screen:?}: {:?} has a role", text.0));
            let want = match role {
                TextRole::Icon => &fonts.icons,
                TextRole::Title => &fonts.title,
                _ => &fonts.text,
            };
            assert_eq!(font.font, FontSource::Handle(want.clone()), "{:?}", text.0);
            // A key name ("X" for Debug axes) is not a placeholder (#560).
            assert!(
                key || !matches!(text.0.as_str(), "M" | "X" | "<"),
                "{screen:?}: placeholder {:?} remains",
                text.0
            );
        }
        assert!(checked > 3, "{screen:?}: screen texts found ({checked})");
    }
}

#[test]
fn region_level_labels_carry_their_tile_count() {
    // Ten journey opens land in the L11 region cell (#543): every portal
    // label ends with the size of its cluster.
    let mut app = journey_app(10);
    app.update();
    let cache = app.world().resource::<LabelCache>();
    let texts = cache.texts();
    assert!(!texts.is_empty(), "L11 holds labelled portals");
    for text in texts {
        assert!(
            text.ends_with(" tiles") || text.ends_with(" tile"),
            "L11 label without its size: {text}"
        );
    }
}

#[test]
fn the_planet_look_switches_the_surface_to_squares_and_back() {
    // #626 AC1, AC2, AC5: the Squares look turns the shader's squares on
    // and the edge lines orange; Normal puts both back.
    use crate::planet_squares::SurfaceMaterial;
    use crate::surface_mesh::{SurfaceEdges, SurfaceMesh};
    use bevy::gizmos::retained::Gizmo;
    use bevy::pbr::MeshMaterial3d;
    let mut app = headless_app();
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        for _ in 0..10 {
            let marker = universe.autopilot_target().expect("dive continues");
            assert!(universe.open(MarkerIndex(marker)), "marker opens");
        }
    }
    let look = |app: &mut App| {
        let mut meshes = app
            .world_mut()
            .query_filtered::<&MeshMaterial3d<SurfaceMaterial>, With<SurfaceMesh>>();
        let handle = meshes
            .iter(app.world())
            .next()
            .expect("a surface")
            .0
            .clone();
        let squares_on = app
            .world()
            .resource::<Assets<SurfaceMaterial>>()
            .get(&handle)
            .expect("its material")
            .extension
            .squares
            .look
            .x;
        let mut lines = app
            .world_mut()
            .query_filtered::<&Gizmo, With<SurfaceEdges>>();
        let gizmo = lines
            .iter(app.world())
            .next()
            .expect("edge lines")
            .handle
            .clone();
        let tint = app
            .world()
            .resource::<Assets<GizmoAsset>>()
            .get(&gizmo)
            .expect("the edge gizmo")
            .buffer()
            .list_colors
            .first()
            .copied()
            .expect("an edge colour");
        (squares_on, tint)
    };
    app.update();
    let (off, normal_tint) = look(&mut app);
    assert_eq!(off, 0.0, "Normal by default");
    app.world_mut()
        .resource_mut::<SettingsStore>()
        .0
        .planet_squares = true;
    app.update();
    let (on, squares_tint) = look(&mut app);
    assert_eq!(on, 1.0, "the shader draws the squares");
    assert_eq!(
        squares_tint,
        SURFACE_EDGE_SQUARES_TINT.to_linear(),
        "orange edges"
    );
    assert_ne!(squares_tint, normal_tint);
    app.world_mut()
        .resource_mut::<SettingsStore>()
        .0
        .planet_squares = false;
    app.update();
    let (back, tint) = look(&mut app);
    assert_eq!((back, tint), (0.0, normal_tint), "Normal again");
}
