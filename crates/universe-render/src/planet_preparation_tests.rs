//! Real mesh preparation and bounded resource tests, not travel/landing or GPU proof.

use super::*;
use bevy::ecs::system::RunSystemOnce;
use universe_core::nav::MarkerIndex;

const RICH: [u32; 9] = [7, 2, 7, 1, 2, 0, 0, 0, 2];

fn universe() -> universe_core::nav::Universe {
    let mut universe = universe_core::nav::Universe::new(42);
    for id in RICH {
        assert!(universe.open(MarkerIndex(id)));
    }
    assert_eq!(
        universe
            .open
            .with_ids()
            .filter(|(_, p)| p.moon.is_some())
            .count(),
        15
    );
    universe
}

fn world() -> World {
    let mut world = World::new();
    world.insert_resource(Universe(universe()));
    world.insert_resource(RenderOrigin::default());
    world.insert_resource(PlanetMeshState::default());
    world.insert_resource(crate::patches::PatchTasks::default());
    world.insert_resource(crate::patches::PatchTree::default());
    world.insert_resource(Assets::<Mesh>::default());
    world.insert_resource(Assets::<StandardMaterial>::default());
    world.insert_resource(crate::LoadingDiagnostics::default());
    world
}

/// An application interval supplied as a production scheduler input. Zero time
/// with one earlier unit is spent; zero time with no units still admits one.
fn interval(world: &mut World, units: usize) {
    let mut tasks = world.resource_mut::<crate::patches::PatchTasks>();
    tasks.apply_started = Some(std::time::Instant::now());
    tasks.apply_units = units;
    tasks.apply_budget_ms = 0.0;
    tasks.apply_work_share = 0.10;
}

fn step(world: &mut World) {
    world.run_system_once(draw_planets).unwrap();
}

#[test]
fn spent_shared_interval_defers_proxies_and_each_fresh_interval_prepares_at_most_one() {
    let mut world = world();
    interval(&mut world, 1);
    step(&mut world);
    assert!(world.resource::<Assets<Mesh>>().is_empty());
    assert!(world.resource::<Assets<StandardMaterial>>().is_empty());
    for index in 0..15 {
        interval(&mut world, 0);
        step(&mut world);
        assert_eq!(world.resource::<Assets<Mesh>>().len(), index + 1);
        assert_eq!(world.resource::<Assets<StandardMaterial>>().len(), 1);
        assert_eq!(
            world.resource::<crate::patches::PatchTasks>().apply_units,
            1
        );
        // No fresh budget for a second call in the same interval.
        step(&mut world);
        assert_eq!(world.resource::<Assets<Mesh>>().len(), index + 1);
    }
    assert!(world.resource::<PlanetMeshState>().complete);
    let host = world.resource::<PlanetMeshState>().host.unwrap();
    let records = universe_core::moon_catalog::moons_for_planet(host).unwrap();
    let mut query = world.query::<(&PlanetMesh, &Mesh3d, &MeshMaterial3d<StandardMaterial>)>();
    let mut material = None;
    for (proxy, mesh, mat) in query.iter(&world) {
        let record = records
            .iter()
            .find(|record| record.identity.marker() == proxy.marker)
            .unwrap();
        assert_eq!(proxy.host, Some(host));
        assert_eq!(proxy.kind, record.kind);
        assert_eq!(proxy.paint_seed, record.paint_seed);
        assert!(
            world
                .resource::<Assets<Mesh>>()
                .get(&mesh.0)
                .unwrap()
                .count_vertices()
                <= 20_000
        );
        assert_eq!(*material.get_or_insert(mat.id()), mat.id());
    }
    let mut lines = Vec::new();
    world
        .resource::<crate::LoadingDiagnostics>()
        .drain_lines(&mut lines)
        .unwrap();
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.contains("role=moon-proxy"))
            .count(),
        15
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.contains("role=moon-material"))
            .count(),
        1
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.contains("kind=proxy-apply-unit"))
            .count(),
        15
    );
}

#[test]
fn approached_moon_prepares_first_without_renumbering_paint_seed_or_marker() {
    let mut world = world();
    let host =
        universe_core::moon_catalog::PlanetIdentity::from_open_cell(world.resource::<Universe>())
            .unwrap();
    let records = universe_core::moon_catalog::moons_for_planet(host).unwrap();
    let target = records.last().unwrap();
    world.resource_mut::<crate::patches::PatchTree>().interest =
        Some(universe_core::surface_interest::SurfaceInterest {
            owner: universe_core::surface_observation::SurfaceIdentity::Moon(target.identity),
            kind: universe_core::surface_interest::SurfaceInterestKind::PlayerApproach,
        });
    interval(&mut world, 0);
    step(&mut world);
    let mut query = world.query::<&PlanetMesh>();
    let proxy = query.single(&world).unwrap();
    assert_eq!(proxy.marker, target.identity.marker());
    assert_eq!(proxy.paint_seed, target.paint_seed);
    assert_eq!(proxy.kind, target.kind);
}

#[test]
fn interrupted_partial_preparation_removes_each_mesh_and_the_one_shared_material() {
    let mut world = world();
    for _ in 0..3 {
        interval(&mut world, 0);
        step(&mut world);
    }
    assert_eq!(world.resource::<Assets<Mesh>>().len(), 3);
    world.resource_mut::<Universe>().close();
    interval(&mut world, 1);
    step(&mut world);
    assert!(world.resource::<Assets<Mesh>>().is_empty());
    assert!(world.resource::<Assets<StandardMaterial>>().is_empty());
    assert!(world.resource::<PlanetMeshState>().built.is_empty());
    assert!(world.resource::<PlanetMeshState>().host.is_none());
    let mut lines = Vec::new();
    world
        .resource::<crate::LoadingDiagnostics>()
        .drain_lines(&mut lines)
        .unwrap();
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.contains("kind=main-remove-request")
                && line.contains("removed_from_store=true"))
            .count(),
        4
    );
}

#[test]
fn cached_seed_match_does_not_override_the_permanent_containing_identity() {
    let mut world = world();
    interval(&mut world, 0);
    step(&mut world);
    let old_meshes: Vec<_> = world.resource::<Assets<Mesh>>().ids().collect();
    let old_host = world.resource::<PlanetMeshState>().host.unwrap();
    let mut next = universe_core::nav::Universe::new(42);
    for id in [7, 2, 7, 1, 2, 0, 0, 0, 5] {
        assert!(next.open(MarkerIndex(id)));
    }
    // Corrupt only the scheduler's cached seed to match the incoming cell. This
    // is not a claim that the real generated fixtures have a hash collision.
    world.resource_mut::<PlanetMeshState>().seed = Some(next.open_seed());
    world.resource_mut::<Universe>().0 = next;
    interval(&mut world, 1);
    step(&mut world);
    assert_ne!(world.resource::<PlanetMeshState>().host.unwrap(), old_host);
    assert!(world.resource::<PlanetMeshState>().built.is_empty());
    assert!(world.resource::<Assets<StandardMaterial>>().is_empty());
    for id in old_meshes {
        assert!(!world.resource::<Assets<Mesh>>().contains(id));
    }
}

#[test]
fn already_built_proxies_follow_the_actual_clock_snapshot_without_rebuilding_assets() {
    let mut world = world();
    for _ in 0..16 {
        interval(&mut world, 0);
        step(&mut world);
    }
    let mut query = world.query::<(&PlanetMesh, &Mesh3d, &crate::style::Anchored, &Transform)>();
    let before: Vec<_> = query
        .iter(&world)
        .map(|(p, m, a, _)| (p.marker, m.id(), a.0))
        .collect();
    world.resource_mut::<Universe>().advance(86_400.0);
    let snapshot = world.resource::<Universe>().0.clone();
    interval(&mut world, 1);
    step(&mut world);
    let mut moved = 0;
    for (proxy, mesh, anchor, transform) in query.iter(&world) {
        let old = before
            .iter()
            .find(|(id, _, _)| *id == proxy.marker)
            .unwrap();
        let actual = snapshot.open.point_by_id(proxy.marker.0).unwrap().position;
        assert_eq!(mesh.id(), old.1);
        assert_eq!(anchor.0, actual);
        assert_eq!(
            transform.translation,
            world.resource::<RenderOrigin>().at(actual)
        );
        moved += usize::from(anchor.0 != old.2);
    }
    assert!(moved > 0);
    assert_eq!(
        world.resource::<Universe>().0,
        snapshot,
        "drawing does not advance time or travel"
    );
    assert_eq!(world.resource::<Assets<Mesh>>().len(), 15);
}

#[test]
fn full_production_schedule_completes_rich_host_without_an_all_moons_frame_spike() {
    let mut app = crate::window_tests::headless_app();
    app.world_mut().resource_mut::<Universe>().0 = universe();
    let mut previous = 0;
    for _ in 0..4096 {
        app.update();
        let count = app
            .world_mut()
            .query::<&PlanetMesh>()
            .iter(app.world())
            .count();
        assert!(count >= previous && count <= previous + 1);
        assert!(app.world().resource::<Assets<StandardMaterial>>().len() <= 2);
        previous = count;
        if app.world().resource::<PlanetMeshState>().complete {
            break;
        }
        std::thread::yield_now();
    }
    assert_eq!(previous, 15);
    assert!(app.world().resource::<PlanetMeshState>().complete);
}
