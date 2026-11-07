//! Real recipe/worker boundaries; no window, GPU or replacement test mechanism.

use super::*;
use universe_core::moon_catalog::{MoonIdentity, PlanetIdentity};
use universe_core::nav::{MarkerIndex, Seed};
use universe_core::settings::Tier;
use universe_core::surface_observation::SurfaceIdentity;
use universe_core::surface_recipe::SurfaceRecipe;

fn recipes() -> Vec<SurfaceRecipe> {
    let host = PlanetIdentity::new(Seed(42), &[7, 2, 7, 1, 2, 0, 0, 0, 2]).unwrap();
    [
        SurfaceIdentity::Planet(host),
        SurfaceIdentity::Moon(MoonIdentity::new(host, MarkerIndex(1)).unwrap()),
        SurfaceIdentity::Moon(MoonIdentity::new(host, MarkerIndex(2)).unwrap()),
    ]
    .into_iter()
    .map(|owner| SurfaceRecipe::for_owner(owner).unwrap())
    .collect()
}

fn epoch(recipe: &SurfaceRecipe, quality: Tier) -> Epoch {
    Arc::new(TerrainEpoch {
        recipe: recipe.key(),
        quality,
    })
}

#[test]
fn returning_to_same_owner_and_quality_cannot_resurrect_an_old_visit() {
    let recipes = recipes();
    let mut inbox = GridInbox::default();
    let key = PatchKey::root(universe_core::cube_sphere::Face::PosX);
    // Planet -> moon -> moon -> planet, followed by the same planet at a new
    // quality and a same-quality resize/rebuild. Each captured worker is real.
    for index in [0, 1, 2, 0] {
        let recipe = &recipes[index];
        let current = epoch(recipe, Tier::Normal);
        assert!(inbox.begin(Some(current.clone())));
        let future = crate::patch_worker::build_task(
            key,
            recipe.relief().radius_m(),
            recipe.relief().clone(),
            inbox.clone(),
            false,
        );
        // Interrupt before polling: queued jobs from the prior generation must
        // refuse even an identical recipe/radius on the next visit.
        let successor = epoch(recipe, Tier::Normal);
        assert!(!same_epoch(Some(&current), Some(&successor)));
        assert!(inbox.begin(Some(successor)));
        assert!(bevy::tasks::block_on(future).is_none());
        let state = inbox.state.lock().unwrap();
        assert!(state.tiles.is_empty());
        assert!(state.heights.is_empty());
    }
}

#[test]
fn completed_real_work_keeps_its_generation_and_current_worker_uses_shared_store() {
    let recipes = recipes();
    let recipe = &recipes[1];
    let mut inbox = GridInbox::default();
    let first = epoch(recipe, Tier::Normal);
    assert!(inbox.begin(Some(first.clone())));
    let key = PatchKey::root(universe_core::cube_sphere::Face::PosX);
    let completed = bevy::tasks::block_on(crate::patch_worker::build_task(
        key,
        recipe.relief().radius_m(),
        recipe.relief().clone(),
        inbox.clone(),
        false,
    ))
    .unwrap();
    assert!(same_epoch(completed.epoch.as_ref(), Some(&first)));
    {
        let state = inbox.state.lock().unwrap();
        assert_eq!(state.tiles.len(), 1);
        assert_eq!(state.heights.len(), 1);
        assert!(same_epoch(state.tiles[&key].epoch.as_ref(), Some(&first)));
    }
    let old_store = inbox.state.clone();
    let next = epoch(recipe, Tier::High);
    assert!(inbox.begin(Some(next.clone())));
    assert!(
        Arc::ptr_eq(&old_store, &inbox.state),
        "no per-visited-owner inbox/cache"
    );
    assert!(!same_epoch(completed.epoch.as_ref(), Some(&next)));
    let state = inbox.state.lock().unwrap();
    assert!(state.tiles.is_empty());
    assert!(state.heights.is_empty());
}

#[test]
fn old_completed_result_cannot_replace_cpu_or_upload_even_with_identical_recipe() {
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;
    let recipes = recipes();
    let recipe = &recipes[1];
    let mut inbox = GridInbox::default();
    let old = epoch(recipe, Tier::Normal);
    assert!(inbox.begin(Some(old)));
    let key = PatchKey::root(universe_core::cube_sphere::Face::PosX);
    let result = bevy::tasks::block_on(crate::patch_worker::build_task(
        key,
        recipe.relief().radius_m(),
        recipe.relief().clone(),
        inbox.clone(),
        true,
    ))
    .unwrap();
    let next = epoch(recipe, Tier::Normal);
    assert!(inbox.begin(Some(next.clone())));
    let mut world = World::new();
    world.insert_resource(inbox);
    world.insert_resource(crate::patch_grid::GridAtlas::default());
    world.insert_resource(Assets::<crate::patch_grid::PatchGridMaterial>::default());
    world.insert_resource(Assets::<Mesh>::default());
    let mut tasks = crate::patches::PatchTasks {
        epoch: Some(next),
        ..Default::default()
    };
    tasks.completed.push((key, result));
    let mut cache = crate::patch_cache::PatchCache::default();
    cache.recipe = Some(recipe.key());
    let radius =
        universe_core::surface_frame::SurfaceRadiusMetres::new(recipe.relief().radius_m()).unwrap();
    let frame = universe_core::planet_frame::PlanetFrame::for_radius_metres(radius);
    let relief = recipe.relief().clone();
    world
        .run_system_once(
            move |mut grid: crate::patch_grid::GridParams, mut meshes: ResMut<Assets<Mesh>>| {
                crate::patch_loading::apply(
                    &mut tasks,
                    &mut cache,
                    &mut grid,
                    &frame,
                    &relief,
                    &mut meshes,
                    2.0,
                    0.10,
                );
                assert!(tasks.completed.is_empty());
                assert!(!cache.contains(key));
                assert!(grid.atlas.writes.is_empty());
                assert_eq!(meshes.len(), 0);
            },
        )
        .unwrap();
}

#[test]
fn production_rebuild_resize_and_quality_changes_keep_one_store_and_shared_queue_cap() {
    use crate::patches::{PatchCache, PatchTasks, PatchTree};
    use universe_core::settings::{GroundMemory, QualityMode};
    let mut app = crate::window_tests::headless_app();
    {
        let mut universe = app.world_mut().resource_mut::<crate::Universe>();
        for &id in &[7, 2, 7, 1, 2, 0, 0, 0, 5] {
            assert!(universe.open(MarkerIndex(id)));
        }
    }
    app.update();
    let original = app.world().resource::<GridInbox>().state.clone();
    let mut last_epoch = app.world().resource::<PatchTasks>().epoch.clone().unwrap();
    let cap = crate::patches::max_builds(bevy::tasks::AsyncComputeTaskPool::get().thread_num());
    for (quality, memory) in [
        (Tier::High, GroundMemory::Mb8),
        (Tier::Low, GroundMemory::Mb128),
        (Tier::Normal, GroundMemory::Mb64),
    ] {
        {
            let mut store = app
                .world_mut()
                .resource_mut::<crate::settings_ui::SettingsStore>();
            store.0.quality_mode = QualityMode::Manual;
            store.0.ground_memory = memory;
            store.0.planet_detail = match quality {
                Tier::Low => universe_core::settings::PlanetDetail::Low,
                Tier::High => universe_core::settings::PlanetDetail::High,
                _ => universe_core::settings::PlanetDetail::Normal,
            };
        }
        // Force a same-owner rebuild too, not just a seed/radius change.
        app.world_mut().resource_mut::<PatchTree>().tree = None;
        for _ in 0..3 {
            app.update();
            let tasks = app.world().resource::<PatchTasks>();
            assert!(tasks.pending.len() + tasks.completed.len() <= cap);
            assert!(
                tasks
                    .completed
                    .iter()
                    .all(|(_, result)| same_epoch(result.epoch.as_ref(), tasks.epoch.as_ref()))
            );
            let inbox = app.world().resource::<GridInbox>();
            assert!(Arc::ptr_eq(&original, &inbox.state));
            let state = inbox.state.lock().unwrap();
            assert!(same_epoch(state.epoch.as_ref(), tasks.epoch.as_ref()));
            assert!(
                state
                    .tiles
                    .values()
                    .all(|tile| same_epoch(tile.epoch.as_ref(), state.epoch.as_ref()))
            );
            assert!(state.heights.len() <= state.heights.capacity());
            let cache = app.world().resource::<PatchCache>();
            assert_eq!(cache.recipe, tasks.epoch.as_ref().map(|epoch| epoch.recipe));
        }
        let epoch = app.world().resource::<PatchTasks>().epoch.clone().unwrap();
        assert!(!same_epoch(Some(&epoch), Some(&last_epoch)));
        last_epoch = epoch;
    }
    // Leaving retires the store contents without dropping still-running jobs
    // from queue accounting. They can only drain, never repopulate this store.
    app.world_mut().resource_mut::<crate::Universe>().close();
    app.update();
    let tasks = app.world().resource::<PatchTasks>();
    assert!(tasks.epoch.is_none());
    assert!(tasks.completed.is_empty());
    assert!(tasks.pending.len() <= cap);
    let inbox = app.world().resource::<GridInbox>();
    let state = inbox.state.lock().unwrap();
    assert!(state.epoch.is_none());
    assert!(state.tiles.is_empty() && state.heights.is_empty());
}

#[test]
fn failed_application_does_not_cache_unusable_cover_and_same_generation_can_retry() {
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;
    let recipes = recipes();
    let recipe = &recipes[1];
    let current = epoch(recipe, Tier::Normal);
    let mut inbox = GridInbox::default();
    assert!(inbox.begin(Some(current.clone())));
    let key = PatchKey::root(universe_core::cube_sphere::Face::PosX);
    let mut world = World::new();
    world.insert_resource(inbox);
    world.insert_resource(crate::patch_grid::GridAtlas::default());
    world.insert_resource(Assets::<crate::patch_grid::PatchGridMaterial>::default());
    world.insert_resource(Assets::<Mesh>::default());
    world.insert_resource(crate::patches::PatchTasks {
        epoch: Some(current),
        ..Default::default()
    });
    let mut cache = crate::patch_cache::PatchCache::default();
    cache.recipe = Some(recipe.key());
    world.insert_resource(cache);
    let radius =
        universe_core::surface_frame::SurfaceRadiusMetres::new(recipe.relief().radius_m()).unwrap();
    let frame = universe_core::planet_frame::PlanetFrame::for_radius_metres(radius);
    for full_atlas in [true, false] {
        // A separate application frame starts a fresh shared interval.
        {
            let mut tasks = world.resource_mut::<crate::patches::PatchTasks>();
            tasks.apply_started = None;
            tasks.apply_units = 0;
            tasks.apply_uploads = 0;
        }
        let result = bevy::tasks::block_on(crate::patch_worker::build_task(
            key,
            recipe.relief().radius_m(),
            recipe.relief().clone(),
            world.resource::<GridInbox>().clone(),
            true,
        ))
        .unwrap();
        world
            .resource_mut::<crate::patches::PatchTasks>()
            .completed
            .push((key, result));
        world.resource_mut::<crate::patch_grid::GridAtlas>().pages = u32::from(!full_atlas);
        let relief = recipe.relief().clone();
        world
            .run_system_once(
                move |mut tasks: ResMut<crate::patches::PatchTasks>,
                      mut cache: ResMut<crate::patch_cache::PatchCache>,
                      mut grid: crate::patch_grid::GridParams,
                      mut meshes: ResMut<Assets<Mesh>>| {
                    // A preceding coarse unit has already spent this frame's
                    // shared budget: root uploads do not receive a fresh share.
                    tasks.apply_started = Some(std::time::Instant::now());
                    tasks.apply_units = 1;
                    crate::patch_loading::apply(
                        &mut tasks,
                        &mut cache,
                        &mut grid,
                        &frame,
                        &relief,
                        &mut meshes,
                        0.0,
                        0.10,
                    );
                    assert_eq!(tasks.completed.len(), 1);
                    assert!(!cache.contains(key));
                    assert!(grid.atlas.tile(key).is_none());
                    tasks.apply_started = None;
                    tasks.apply_units = 0;
                    crate::patch_loading::apply(
                        &mut tasks,
                        &mut cache,
                        &mut grid,
                        &frame,
                        &relief,
                        &mut meshes,
                        2.0,
                        0.10,
                    );
                    assert!(tasks.completed.is_empty());
                    assert_eq!(cache.contains(key), !full_atlas);
                    assert_eq!(grid.atlas.tile(key).is_some(), !full_atlas);
                    assert!(!crate::patch_cover::roots_ready(&cache, &grid.atlas));
                    if !full_atlas {
                        assert!(cache.map[&key].gpu.is_some());
                        assert!(cache.cpu(key).is_some());
                    }
                },
            )
            .unwrap();
    }
}
