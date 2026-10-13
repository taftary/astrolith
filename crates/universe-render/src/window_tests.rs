//! Window-path dives for the continuous camera (#430, #632).
//!
//! Test-only module (`E-FILE-SIZE`: `lib.rs` is past the split line).
//! Headless apps driving the real input pipeline: manual wheel dives come
//! down to the planet's ground through every shell level and above the
//! ground, the level follows the height in every mode, free flight passes
//! through the ground, and the autopilot keeps the replay mode.
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
use universe_core::destination::Destination;
use universe_core::nav::{DEMO_SEED, DiveMode, MarkerIndex};

use crate::DivePlugin;
use crate::Universe;
use crate::input::{Autopilot, ClickCycle, CursorPortals, Flight, FlightMode, Navigation};
use crate::tokens::{CYCLE_CAP, SELECTION_RING_SCALE, SURFACE_EDGE_SQUARES_TINT};

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

/// Level shown to the player: the height shell on a planet (#632).
fn shown(app: &App) -> u8 {
    app.world().resource::<Universe>().shown_level().get()
}

/// Camera above the planet's ground (#430, #632): never under it.
fn above_floor(app: &App) -> bool {
    use universe_core::terrain::surface_radius;
    let universe = app.world().resource::<Universe>();
    let offset = universe.path.offset();
    if !offset.iter().all(|c| c.is_finite()) {
        return false;
    }
    if universe.level().get() != 10 {
        return true;
    }
    let reach = (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
    reach + 1e-12 >= surface_radius(offset)
}

/// Headless app on the journey's planet (L10), the camera `metres` above
/// the ground at the north pole, aimed at the planet's own destination.
fn planet_app(metres: f64) -> App {
    use universe_core::frame::IDENTITY_UP;
    use universe_core::nest::autopilot_path;
    let mut app = headless_app();
    let chain = autopilot_path(DEMO_SEED);
    let planet = chain.get(..9).expect("journey reaches the planet");
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        for &marker in planet {
            assert!(universe.open(MarkerIndex(marker)), "journey marker opens");
        }
        assert_eq!(universe.level().get(), 10);
        let ground = universe_core::terrain::surface_radius([0.0, 1.0, 0.0]);
        let lift = metres / universe_core::shell::metres_per_unit();
        universe.path.set_offset([0.0, ground + lift, 0.0]);
    }
    {
        let mut nav = app.world_mut().resource_mut::<Navigation>();
        nav.destination = Destination::from_chain(planet);
        nav.up = IDENTITY_UP;
        nav.forward = [0.0, -0.6, -0.8];
    }
    app
}

#[test]
fn manual_dives_land_but_the_autopilot_keeps_the_replay_line() {
    use crate::input::dive_mode;
    assert_eq!(dive_mode(&Autopilot::Flying), DiveMode::Targeted);
    assert_eq!(dive_mode(&Autopilot::Idle), DiveMode::Landing);
}

#[test]
fn window_dive_comes_down_through_every_shell_and_back_out() {
    // AC1, AC5, AC8 of #632: the wheel on the planet comes down to the
    // ground through L11, L12, L13 and L14 in order, never under the
    // ground, and rests there; the dive out climbs back through the same
    // levels to L10 and leaves the planet.
    let mut app = planet_app(5.0e6);
    assert_eq!(shown(&app), 10, "far above the shells");
    let mut seen = Vec::new();
    for _ in 0..4000 {
        notch_in(&mut app, 0.25);
        assert!(above_floor(&app), "the window dive stays above the ground");
        assert_eq!(level(&app), 10, "nothing opens below the planet");
        let now = shown(&app);
        if seen.last() != Some(&now) && now > 10 {
            seen.push(now);
        }
        if now == 14 {
            break;
        }
    }
    assert_eq!(seen, vec![11, 12, 13, 14], "every shell, in order");
    let height = app
        .world()
        .resource::<Universe>()
        .path
        .height_above_ground_m()
        .expect("on the planet");
    assert!((1.0..10.0).contains(&height), "rests in L14: {height} m");
    let mut back = Vec::new();
    for _ in 0..4000 {
        notch_in(&mut app, -0.25);
        assert!(above_floor(&app), "rising stays above the ground");
        if level(&app) != 10 {
            break;
        }
        let now = shown(&app);
        if back.last() != Some(&now) {
            back.push(now);
        }
    }
    assert_eq!(
        back,
        vec![14, 13, 12, 11, 10],
        "back up through every shell"
    );
    assert_eq!(level(&app), 9, "past the close angle the planet closes");
}

#[test]
fn window_free_flight_passes_through_the_ground() {
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;
    // Start 20 m over the planet's north pole (the buildings shell), on
    // fixed frames, so the flight covers the same ground on every machine.
    let mut app = planet_app(20.0);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        16,
    )));
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
        assert_eq!(universe.level().get(), 10, "free flight never opens");
        if universe
            .path
            .height_above_ground_m()
            .is_some_and(|height| height < 0.0)
        {
            below = true;
            break;
        }
    }
    assert!(below, "free mode flies under the ground");
}

/// Moon-mesh entities live in the headless app (the planet body is the
/// Surface-mesh entities: exactly one on a planet, none elsewhere (ADR 0024).
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
    let mut app = planet_app(2.0e5);
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
fn the_planet_keeps_one_surface_down_to_the_ground_and_leaving_despawns_it() {
    // One icosphere surface from orbit to 5 m over the ground (#632), and
    // leaving the planet despawns it with its edge lines.
    let mut app = planet_app(5.0);
    app.update();
    assert_eq!(shown(&app), 14, "5 m up is the room shell");
    assert_eq!(
        surface_count(&mut app),
        1,
        "one icosphere surface, no discs"
    );
    assert_eq!(edge_line_count(&mut app), 1, "its edges show as lines");
    app.update();
    assert_eq!(surface_count(&mut app), 1, "the mesh persists");
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        assert!(universe.close().is_some(), "the dive backs out to L9");
    }
    // Far from the planet in the level above, the surface is gone.
    app.world_mut()
        .resource_mut::<Universe>()
        .path
        .set_offset([0.0, 0.0, 3.0]);
    app.update();
    assert_eq!(level(&app), 9);
    assert_eq!(surface_count(&mut app), 0, "no surface mesh off the planet");
    assert_eq!(edge_line_count(&mut app), 0, "and no edge lines");
}

#[test]
fn the_level_is_the_same_in_every_mode() {
    // AC3 of #632: the height alone picks the level, whatever the mode.
    for (mode, metres, want) in [
        (FlightMode::Dive, 5.0e4, 12u8),
        (FlightMode::Free, 500.0, 13),
        (FlightMode::Free, 2.0e6, 10),
    ] {
        let mut app = planet_app(metres);
        app.world_mut().resource_mut::<Flight>().mode = mode;
        app.update();
        assert_eq!(shown(&app), want, "{mode:?} at {metres} m");
    }
    // Normal mode: the body stands on the ground, its eye in L14.
    let mut app = planet_app(0.5);
    player::player_here(&mut app);
    app.world_mut().resource_mut::<Flight>().mode = FlightMode::Normal;
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(level(&app), 10, "walking opens nothing");
    assert_eq!(shown(&app), 14, "the body's eye is in the room shell");
}

#[test]
fn a_level_change_moves_nothing_on_screen() {
    // AC5 of #632: crossing a shell height changes only the HUD level.
    // Just above 1.5 km (L12) and just below 1 km (L13): the same surface
    // mesh stays, the open planet and its bodies stay the same.
    let mut app = planet_app(1_000.4);
    app.update();
    assert_eq!(shown(&app), 12);
    let before = surface_entity(&mut app).expect("a surface");
    let points_before = app.world().resource::<Universe>().open.points.clone();
    let step = 0.8 / universe_core::shell::metres_per_unit();
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        let [x, y, z] = universe.path.offset();
        universe.path.set_offset([x, y - step, z]);
    }
    app.update();
    assert_eq!(shown(&app), 13, "crossed into the buildings shell");
    assert_eq!(
        surface_entity(&mut app),
        Some(before),
        "the same surface mesh, nothing rebuilt or swapped"
    );
    let universe = app.world().resource::<Universe>();
    assert_eq!(universe.level().get(), 10, "the same open planet");
    assert_eq!(universe.open.points, points_before, "the same drawn bodies");
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
fn portal_rings_draw_headless_at_l1_l6_and_l9() {
    use crate::draw::portal_ring;
    use universe_core::r#gen::MarkerKind;
    use universe_core::nav::MarkerIndex;
    let mut app = headless_app();
    // Walk the journey, drawing rings at three rungs: portals carry a ring
    // at their true position, populations carry none (#446 T4).
    for want_level in [1u8, 6, 9] {
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
fn the_planet_look_switches_the_surface_to_squares_and_back() {
    // #626 AC1, AC2, AC5: the Squares look turns the shader's squares on
    // and the edge lines orange; Normal puts both back.
    use crate::planet_squares::SurfaceMaterial;
    use crate::surface_mesh::{SurfaceEdges, SurfaceMesh};
    use bevy::gizmos::retained::Gizmo;
    use bevy::pbr::MeshMaterial3d;
    let mut app = planet_app(500.0);
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

#[test]
fn the_readout_and_title_name_the_shell_level() {
    // AC6 of #632: on the planet the readout names the height shell, its
    // 10^N m readout and the next level; the window title follows.
    use crate::hud::HudText;
    for (metres, name, scale, next) in [
        (5.0e5, "L11 - Regions", "10^6 m", Some("next: L12")),
        (
            5.0e4,
            "L12 - Cities and landscapes",
            "10^5 m",
            Some("next: L13"),
        ),
        (500.0, "L13 - Buildings", "10^3 m", Some("next: L14")),
        (5.0, "L14 - Room", "10^1 m", None),
    ] {
        let mut app = planet_app(metres);
        app.update();
        let text = app
            .world_mut()
            .query_filtered::<&Text, With<HudText>>()
            .single(app.world())
            .map(|text| text.0.clone())
            .expect("one readout");
        assert!(text.starts_with(name), "{metres} m: {text}");
        assert!(text.contains(scale), "{metres} m: {text}");
        match next {
            Some(line) => assert!(text.contains(line), "{metres} m: {text}"),
            None => assert!(!text.contains("next:"), "{metres} m: {text}"),
        }
    }
}
