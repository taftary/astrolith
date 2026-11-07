//! Production scheduling/ownership checks, not GPU visibility or contact proof.

use super::*;

fn opened() -> App {
    let mut app = crate::window_tests::headless_app();
    {
        let mut universe = app.world_mut().resource_mut::<crate::Universe>();
        for &id in &[7, 2, 7, 1, 2, 0, 0, 0, 5] {
            assert!(universe.open(universe_core::nav::MarkerIndex(id)));
        }
    }
    app
}

fn counts(app: &mut App) -> (usize, usize) {
    let parent = app
        .world_mut()
        .query::<&ParentProxy>()
        .iter(app.world())
        .count();
    let patches = app
        .world_mut()
        .query::<&crate::patches::PatchPiece>()
        .iter(app.world())
        .count();
    (parent, patches)
}

#[test]
fn asynchronous_roots_keep_non_contact_cover_then_retire_it_without_duplicate_entities() {
    let mut app = opened();
    app.update();
    assert_eq!(counts(&mut app), (1, 0));
    let ground = app.world().resource::<GroundSurface>();
    assert!(ground.0.is_none() && !ground.1 && !ground.2 && ground.3.is_none());
    let tasks = app.world().resource::<PatchTasks>();
    assert!(
        !tasks.pending.is_empty(),
        "roots are worker jobs, not synchronous builds"
    );
    assert_eq!(
        tasks.apply_units, 1,
        "coarse application consumes the shared interval"
    );
    let (mesh_id, material_id) = {
        let mut query = app
            .world_mut()
            .query_filtered::<(&Mesh3d, &MeshMaterial3d<StandardMaterial>), With<ParentProxy>>();
        let (mesh, material) = query.single(app.world()).unwrap();
        assert!(
            app.world()
                .resource::<Assets<Mesh>>()
                .get(&mesh.0)
                .unwrap()
                .count_vertices()
                <= 20_000
        );
        (mesh.id(), material.id())
    };
    let cap = crate::patches::max_builds(bevy::tasks::AsyncComputeTaskPool::get().thread_num());
    let mut published = false;
    for _ in 0..4096 {
        app.update();
        let (parent, patches) = counts(&mut app);
        assert!(
            parent == 1 && patches == 0 || parent == 0 && patches >= 6,
            "one coarse body or complete detailed cover, never a partial/duplicate sphere"
        );
        let tasks = app.world().resource::<PatchTasks>();
        assert!(tasks.pending.len() + tasks.completed.len() <= cap);
        assert!(tasks.apply_uploads <= crate::patches::PATCH_APPLY_BUDGET);
        if patches > 0 {
            assert!(app.world().resource::<GroundSurface>().0.is_some());
            assert!(!app.world().resource::<Assets<Mesh>>().contains(mesh_id));
            assert!(
                !app.world()
                    .resource::<Assets<StandardMaterial>>()
                    .contains(material_id)
            );
            published = true;
            break;
        }
        assert!(!app.world().resource::<GroundSurface>().1);
        std::thread::yield_now();
    }
    assert!(published);
}

#[test]
fn interrupted_root_preparation_retires_coarse_assets_and_does_not_publish_ground() {
    let mut app = opened();
    app.update();
    let ids = {
        let mut query = app
            .world_mut()
            .query_filtered::<(&Mesh3d, &MeshMaterial3d<StandardMaterial>), With<ParentProxy>>();
        let (mesh, material) = query.single(app.world()).unwrap();
        (mesh.id(), material.id())
    };
    app.world_mut().resource_mut::<crate::Universe>().close();
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(counts(&mut app), (0, 0));
    assert!(!app.world().resource::<Assets<Mesh>>().contains(ids.0));
    assert!(
        !app.world()
            .resource::<Assets<StandardMaterial>>()
            .contains(ids.1)
    );
    assert!(app.world().resource::<GroundSurface>().0.is_none());
    assert!(app.world().resource::<PatchTasks>().completed.is_empty());
    assert!(app.world().resource::<PatchTasks>().epoch.is_none());
}

#[test]
fn memory_resize_keeps_one_coarse_or_detailed_main_world_cover_and_records_real_root_jobs() {
    let mut app = opened();
    app.add_plugins(crate::LoadingDiagnosticsPlugin);
    crate::window_tests::wait_for_detailed_cover(&mut app);
    let mut lines = Vec::new();
    app.world()
        .resource::<crate::LoadingDiagnostics>()
        .drain_lines(&mut lines)
        .unwrap();
    assert!(lines.iter().any(|line| line.contains("role=parent-proxy")));
    assert!(
        lines
            .iter()
            .any(|line| line.contains("role=parent-material"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("kind=job-queued") && line.contains("synchronous=false"))
    );
    assert!(!lines.iter().any(|line| line.contains("synchronous=true")));
    // Main-world draw ownership only; no grounded-player or GPU-retirement claim.
    for memory in [
        universe_core::settings::GroundMemory::Mb8,
        universe_core::settings::GroundMemory::Mb64,
    ] {
        app.world_mut()
            .resource_mut::<crate::SettingsStore>()
            .0
            .ground_memory = memory;
        app.update();
        assert_eq!(counts(&mut app), (1, 0));
        assert!(!app.world().resource::<GroundSurface>().1);
        for _ in 0..4096 {
            app.update();
            let (parent, patches) = counts(&mut app);
            assert!(parent == 1 && patches == 0 || parent == 0 && patches >= 6);
            if patches > 0 {
                break;
            }
            std::thread::yield_now();
        }
        assert_eq!(counts(&mut app).0, 0);
        assert!(app.world().resource::<GroundSurface>().0.is_some());
        // One shared moon material, no accumulating parent material per rebuild.
        assert!(app.world().resource::<Assets<StandardMaterial>>().len() <= 2);
    }
}
