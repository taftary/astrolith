//! Real production owner preparation and draw ownership, not input travel,
//! landing, rendered pixels, GPU retirement or performance acceptance.

use super::*;
use bevy::prelude::*;
use universe_core::coords::OpenUnits;
use universe_core::moon_catalog::PlanetIdentity;
use universe_core::nav::MarkerIndex;

fn opened() -> (App, Vec<SurfaceIdentity>) {
    let mut app = crate::window_tests::headless_app();
    // The optional resource observer requires real main-world asset stores.
    // This does not install a renderer or fabricate render-world residence.
    app.init_resource::<Assets<Image>>();
    app.add_plugins(crate::LoadingDiagnosticsPlugin);
    {
        let mut live = app.world_mut().resource_mut::<crate::Universe>();
        for id in [7, 2, 7, 1, 2, 0, 0, 0, 2] {
            assert!(live.open(MarkerIndex(id)));
        }
    }
    app.world_mut().resource_mut::<crate::input::Flight>().mode = crate::input::FlightMode::Free;
    app.world_mut()
        .resource_mut::<crate::sky::SkyClock>()
        .0
        .playing = false;
    let host = PlanetIdentity::from_open_cell(app.world().resource::<crate::Universe>()).unwrap();
    let mut owners = vec![SurfaceIdentity::Planet(host)];
    owners.extend(
        universe_core::moon_catalog::moons_for_planet(host)
            .unwrap()
            .into_iter()
            .map(|r| SurfaceIdentity::Moon(r.identity)),
    );
    assert_eq!(owners.len(), 16);
    prepare_camera(&mut app, owners[0]);
    for _ in 0..4096 {
        app.update();
        drain(&app);
        let proxies = app
            .world_mut()
            .query::<&crate::planet::PlanetMesh>()
            .iter(app.world())
            .count();
        if proxies == 15 && drawn(&app) == Some(owners[0]) {
            return (app, owners);
        }
        std::thread::yield_now();
    }
    panic!("initial real proxy/root preparation did not complete");
}

fn prepare_camera(app: &mut App, owner: SurfaceIdentity) {
    let frame = SurfaceFrame::for_owner(app.world().resource::<crate::Universe>(), owner).unwrap();
    let cell = frame
        .to_cell(
            frame
                .position([frame.radius().get() + 10.0, 0.0, 0.0])
                .unwrap(),
        )
        .unwrap();
    app.world_mut()
        .resource_mut::<crate::Universe>()
        .path
        .set_offset(cell.0);
    // Explicit preparation of a camera, not a control-driven flight/teleport UI.
    let mut origin = app.world_mut().resource_mut::<crate::style::RenderOrigin>();
    origin.camera = cell.0;
}

fn drawn(app: &App) -> Option<SurfaceIdentity> {
    app.world()
        .resource::<GroundSurface>()
        .0
        .as_ref()?
        .owner_frame()
        .map(|frame| frame.owner())
}

fn drain(app: &App) {
    let mut lines = Vec::new();
    app.world()
        .resource::<crate::LoadingDiagnostics>()
        .drain_lines(&mut lines)
        .unwrap();
    if let Some(epoch) = &app.world().resource::<PatchTasks>().epoch {
        let expected = match epoch.recipe.owner() {
            SurfaceIdentity::Planet(_) => "surface_owner=planet\t".to_owned(),
            SurfaceIdentity::Moon(moon) => format!("surface_owner=moon:{}\t", moon.marker().0),
        };
        for line in lines
            .iter()
            .filter(|line| line.contains("kind=epoch-start"))
        {
            assert!(
                line.contains(&expected),
                "actual journal epoch must name the production owner: {line}"
            );
        }
    }
    let problem = app
        .world()
        .resource::<crate::LoadingDiagnostics>()
        .problem();
    assert!(problem.is_none(), "journal problem: {problem:?}");
}

fn inspect(app: &mut App, owners: &[SurfaceIdentity]) {
    let detailed = drawn(app);
    let mut cover = vec![0usize; owners.len()];
    let mut parent_query = app
        .world_mut()
        .query::<(&crate::parent_proxy::ParentProxy, &Visibility)>();
    for (proxy, visibility) in parent_query.iter(app.world()) {
        assert_eq!(proxy.0, owners[0].host());
        if *visibility != Visibility::Hidden {
            cover[0] += 1;
        }
    }
    let mut moon_query = app
        .world_mut()
        .query::<(&crate::planet::PlanetMesh, &Visibility)>();
    assert_eq!(
        moon_query.iter(app.world()).count(),
        15,
        "proxy identities/slots are retained"
    );
    for (proxy, visibility) in moon_query.iter(app.world()) {
        let index = owners.iter().position(|owner| matches!(owner,
            SurfaceIdentity::Moon(moon) if Some(moon.planet()) == proxy.host && moon.marker() == proxy.marker)).unwrap();
        if *visibility != Visibility::Hidden {
            cover[index] += 1;
        }
    }
    for (index, owner) in owners.iter().enumerate() {
        assert_eq!(
            cover[index],
            usize::from(detailed != Some(*owner)),
            "every body has exactly one main-world coarse OR detailed representation: {owner:?}"
        );
    }
    let tasks = app.world().resource::<PatchTasks>();
    let cap = crate::patches::max_builds(bevy::tasks::AsyncComputeTaskPool::get().thread_num());
    assert!(tasks.pending.len() + tasks.completed.len() <= cap);
    assert!(tasks.apply_uploads <= crate::patches::PATCH_APPLY_BUDGET);
    assert!(
        tasks
            .completed
            .iter()
            .all(|(_, result)| crate::terrain_epoch::same_epoch(
                result.epoch.as_ref(),
                tasks.epoch.as_ref()
            ))
    );
    assert_eq!(
        app.world().resource::<PatchCache>().recipe,
        tasks.epoch.as_ref().map(|epoch| epoch.recipe)
    );
    assert!(app.world().resource::<Assets<StandardMaterial>>().len() <= 2);
    assert!(
        app.world()
            .resource::<Assets<crate::patch_grid::PatchGridMaterial>>()
            .len()
            <= 32
    );
    let snapshot = app.world().resource::<PatchTree>().surface_frame.unwrap();
    if let Some(detailed) = detailed {
        assert_eq!(detailed, snapshot.owner());
    }
    let mut patches = app.world_mut().query::<(
        &crate::patches::PatchPiece,
        &crate::style::Anchored,
        &Transform,
        &bevy::camera::primitives::Aabb,
    )>();
    let count = patches.iter(app.world()).count();
    assert_eq!(count > 0, detailed.is_some());
    for (piece, anchor, transform, bounds) in patches.iter(app.world()) {
        let cached = &app.world().resource::<PatchCache>().map[&piece.0];
        let expected = crate::patch_owner::cell_at(snapshot, cached.info.centre).unwrap();
        assert_eq!(
            anchor.0, expected,
            "translated patch centre, never parent-centred alias"
        );
        assert_eq!(
            transform.translation,
            app.world()
                .resource::<crate::style::RenderOrigin>()
                .at(expected)
        );
        assert_eq!(transform.scale, Vec3::ONE);
        let cached_bounds = &cached.gpu.as_ref().unwrap().1;
        assert_eq!(bounds.center, cached_bounds.center);
        assert_eq!(bounds.half_extents, cached_bounds.half_extents);
        assert!(bounds.center.is_finite() && bounds.half_extents.is_finite());
    }
    drain(app);
}

fn wait_for(app: &mut App, owners: &[SurfaceIdentity], target: SurfaceIdentity) {
    for _ in 0..4096 {
        app.update();
        inspect(app, owners);
        if drawn(app) == Some(target) {
            return;
        }
        std::thread::yield_now();
    }
    panic!("requested actual owner did not publish: {target:?}");
}

#[test]
fn planet_moon_moon_planet_transitions_reuse_one_store_and_cover_every_retired_body() {
    let (mut app, owners) = opened();
    let original = app
        .world()
        .resource::<crate::patch_grid::GridInbox>()
        .state
        .clone();
    let host = owners[0].host();
    let frozen = app.world().resource::<crate::Universe>().time_s();
    for target in [owners[1], owners[2], owners[0], owners[15], owners[0]] {
        prepare_camera(&mut app, target);
        wait_for(&mut app, &owners, target);
        assert!(std::sync::Arc::ptr_eq(
            &original,
            &app.world().resource::<crate::patch_grid::GridInbox>().state
        ));
        assert_eq!(
            PlanetIdentity::from_open_cell(app.world().resource::<crate::Universe>()).unwrap(),
            host
        );
        assert_eq!(app.world().resource::<crate::Universe>().time_s(), frozen);
        let frame = app.world().resource::<PatchTree>().surface_frame.unwrap();
        assert_eq!(frame.owner(), target);
        let universe = app.world().resource::<crate::Universe>();
        let round_trip = frame
            .to_cell(frame.from_cell(OpenUnits(universe.path.offset())).unwrap())
            .unwrap();
        let delta = round_trip
            .0
            .into_iter()
            .zip(universe.path.offset())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(delta * universe_core::planet_frame::metres_per_cell_l10() <= 0.01);
    }
}

#[test]
fn interrupted_moon_builds_and_resize_refuse_stale_generations_without_losing_coarse_cover() {
    let (mut app, owners) = opened();
    prepare_camera(&mut app, owners[1]);
    for _ in 0..4096 {
        app.update();
        inspect(&mut app, &owners);
        if app.world().resource::<PatchTree>().recipe.unwrap().owner() == owners[1] {
            break;
        }
    }
    let retired = app.world().resource::<PatchTasks>().epoch.clone().unwrap();
    assert_eq!(retired.recipe.owner(), owners[1]);
    assert!(
        drawn(&app).is_none(),
        "new roots are still worker preparation, not contact"
    );
    prepare_camera(&mut app, owners[2]);
    wait_for(&mut app, &owners, owners[2]);
    assert!(!crate::terrain_epoch::same_epoch(
        Some(&retired),
        app.world().resource::<PatchTasks>().epoch.as_ref()
    ));
    app.world_mut()
        .resource_mut::<crate::SettingsStore>()
        .0
        .ground_memory = universe_core::settings::GroundMemory::Mb8;
    wait_for(&mut app, &owners, owners[2]);
    prepare_camera(&mut app, owners[0]);
    wait_for(&mut app, &owners, owners[0]);
}

#[test]
fn missing_coarse_cover_holds_the_current_owner_instead_of_discarding_it() {
    let (app, owners) = opened();
    let tree = PatchTree {
        recipe: app.world().resource::<PatchTree>().recipe,
        interest: Some(universe_core::surface_interest::SurfaceInterest {
            owner: owners[1],
            kind: universe_core::surface_interest::SurfaceInterestKind::Camera,
        }),
        ..Default::default()
    };
    let universe = app.world().resource::<crate::Universe>();
    assert_eq!(owner(&tree, universe, |_| false), Some(owners[0]));
    assert_eq!(
        owner(&tree, universe, |body| body == owners[0]),
        Some(owners[0])
    );
    assert_eq!(
        owner(&tree, universe, |body| body == owners[1]),
        Some(owners[0])
    );
    assert_eq!(owner(&tree, universe, |_| true), Some(owners[1]));
}

#[test]
fn moving_moon_refreshes_actual_centres_without_relabeling_or_rebuilding_its_terrain_epoch() {
    let (mut app, owners) = opened();
    let moon = owners[1];
    prepare_camera(&mut app, moon);
    wait_for(&mut app, &owners, moon);
    let epoch = app.world().resource::<PatchTasks>().epoch.clone().unwrap();
    let old = app.world().resource::<PatchTree>().surface_frame.unwrap();
    let next_time = app.world().resource::<crate::Universe>().time_s() + 86_400.0;
    app.world_mut()
        .resource_mut::<crate::Universe>()
        .advance(next_time);
    app.world_mut()
        .resource_mut::<crate::sky::SkyClock>()
        .0
        .game_s = next_time;
    // Separate explicit camera preparation at the new centre. This does NOT
    // exercise automatic player transport, self-propelled travel or a frozen
    // conversion between different times.
    prepare_camera(&mut app, moon);
    app.update();
    inspect(&mut app, &owners);
    let now = app.world().resource::<PatchTree>().surface_frame.unwrap();
    assert_eq!(now.owner(), old.owner());
    assert_ne!(now.centre(), old.centre());
    assert_eq!(now.time_s(), next_time);
    assert!(crate::terrain_epoch::same_epoch(
        Some(&epoch),
        app.world().resource::<PatchTasks>().epoch.as_ref()
    ));
    assert_eq!(
        app.world()
            .resource::<GroundSurface>()
            .0
            .as_ref()
            .unwrap()
            .owner_frame(),
        Some(now)
    );
}

#[test]
fn production_moon_query_uses_the_same_owned_mesh_morph_triangles_not_a_reference_sphere() {
    let (mut app, owners) = opened();
    let target = owners[1];
    prepare_camera(&mut app, target);
    wait_for(&mut app, &owners, target);
    let mut queried = false;
    for _ in 0..4096 {
        app.update();
        inspect(&mut app, &owners);
        let point = app.world().resource::<crate::Universe>().path.offset();
        let surface = app.world().resource::<GroundSurface>().0.as_ref().unwrap();
        let Some(ground) = surface.ground_at(point) else {
            std::thread::yield_now();
            continue;
        };
        let centred = surface.patch_position(OpenUnits(point)).unwrap();
        let triangle = surface.ground_at_planet(centred).unwrap();
        assert_eq!(ground.point, surface.cell_position(triangle.0).unwrap().0);
        assert_eq!(ground.up, triangle.1);
        assert_eq!(surface.owner_frame().unwrap().owner(), target);
        assert_eq!(app.world().resource::<GroundSurface>().3, Some(target));
        queried = true;
        break;
    }
    assert!(
        queried,
        "the actual held production mesh supplies the moon query"
    );
    // No body contact flag is manufactured here. A geometry query is not landing.
}
