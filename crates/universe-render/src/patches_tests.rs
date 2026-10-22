//! Patch worker, cache and drawing tests, split from `patches` under
//! `E-FILE-SIZE` (#918 T5).

use super::*;
use crate::patch_cache::{BIAS_DOWN_FRAMES, BIAS_MAX, BIAS_STEP, BIAS_UP_FRAMES, bias_step};
use crate::window_tests::headless_app;
use universe_core::lod::PatchKey;
use universe_core::lod_select::is_ancestor;
use universe_core::nav::MarkerIndex;

#[test]
fn builds_in_flight_stay_few() {
    // #819, #918 T8: six per worker thread, never below two, so the pool
    // keeps room for a screenshot's read-back.
    assert_eq!(max_builds(0), 2);
    assert_eq!(max_builds(1), 6);
    assert_eq!(max_builds(2), 12);
    assert_eq!(max_builds(4), 24);
}

fn covering_drawn(drawn: &BTreeSet<PatchKey>, leaf: PatchKey) -> bool {
    let mut cursor = leaf;
    loop {
        if drawn.contains(&cursor) {
            return true;
        }
        let Some(parent) = cursor.parent() else {
            return false;
        };
        cursor = parent;
    }
}

/// Drawn patch keys and the keys that have an entity, both sorted.
fn drawn_and_present(app: &mut App) -> (Vec<PatchKey>, Vec<PatchKey>) {
    let drawn = app.world().resource::<PatchTasks>().synced.clone();
    let mut present: Vec<PatchKey> = app
        .world_mut()
        .query::<&PatchPiece>()
        .iter(app.world())
        .map(|piece| piece.0)
        .collect();
    present.sort();
    (drawn, present)
}

#[test]
fn rebuild_never_touches_despawned_patches() {
    // C1 / C2 (#851): a rebuild despawns every patch, and in the same
    // frame the fresh draw set gets new entities, never the despawned
    // ones; every drawn patch has exactly one entity afterwards.
    // No-window app test (E-TEST-NO-WINDOW).
    let mut app = headless_app();
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        for _ in 0..9 {
            let marker = universe.autopilot_target().expect("dive continues");
            assert!(universe.open(MarkerIndex(marker)), "marker opens");
        }
    }
    app.update();
    app.update();
    let (drawn, present) = drawn_and_present(&mut app);
    assert!(!drawn.is_empty(), "patches drawn at the planet");
    assert_eq!(present, drawn, "one entity per drawn patch");
    // Force a rebuild with patches on screen (as a planet change does).
    app.world_mut().resource_mut::<PatchTree>().tree = None;
    app.update();
    let (drawn, present) = drawn_and_present(&mut app);
    assert!(!drawn.is_empty(), "the rebuild draws again at once");
    assert_eq!(present, drawn, "no drawn patch lost its entity");
    app.update();
    let (drawn, present) = drawn_and_present(&mut app);
    assert_eq!(present, drawn, "and none the frame after");
}

#[test]
fn parents_stay_until_children_are_ready() {
    // AC3 / C3 (T3, Q4 A): after a split no frame lacks coverage, and no
    // more than the budget is applied per frame. No-window app test
    // (E-TEST-NO-WINDOW).
    let mut app = headless_app();
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        for _ in 0..9 {
            let marker = universe.autopilot_target().expect("dive continues");
            assert!(universe.open(MarkerIndex(marker)), "marker opens");
        }
        assert_eq!(universe.level().get(), 10, "the dive ends at L10");
    }
    // First frame builds the six roots synchronously: coverage at once.
    app.update();
    let mut previous_cache = 0usize;
    for frame in 0..20 {
        app.update();
        let tree = app
            .world()
            .resource::<PatchTree>()
            .tree
            .clone()
            .expect("tree at L10");
        let cache = app.world().resource::<PatchCache>();
        // Budget: cache grows by at most the budget per frame.
        let grown = cache.len().saturating_sub(previous_cache);
        assert!(
            grown <= PATCH_APPLY_BUDGET,
            "frame {frame} applied {grown} over {PATCH_APPLY_BUDGET}"
        );
        previous_cache = cache.len();
        // Coverage: every leaf has a drawn ancestor.
        let leaves = tree.leaves();
        assert!(!leaves.is_empty(), "frame {frame} has leaves");
        let cached = cache.keys();
        let drawn = draw_selection(&leaves, &cached);
        assert!(!drawn.is_empty(), "frame {frame} draws something");
        for leaf in &leaves {
            assert!(
                covering_drawn(&drawn, *leaf),
                "frame {frame}: leaf {leaf:?} uncovered"
            );
        }
        // Entities mirror the draw set (no extra, no missing).
        let mut entities: HashSet<PatchKey> = HashSet::new();
        let mut query = app.world_mut().query::<&PatchPiece>();
        for piece in query.iter(app.world()) {
            entities.insert(piece.0);
        }
        assert_eq!(
            entities,
            drawn.iter().copied().collect::<HashSet<PatchKey>>(),
            "frame {frame}: entities mirror the draw set"
        );
        // Ancestor check: no drawn patch is an ancestor of another drawn
        // patch (the draw set is a cut).
        for a in drawn.iter().copied() {
            for b in drawn.iter().copied() {
                if a != b {
                    assert!(!is_ancestor(a, b), "frame {frame}: {a:?} contains {b:?}");
                }
            }
        }
    }
    // Leaving L10 despawns every patch with no error.
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        assert!(universe.close().is_some(), "the dive backs out");
    }
    app.update();
    let mut query = app.world_mut().query::<&PatchPiece>();
    assert_eq!(
        query.iter(app.world()).count(),
        0,
        "no patch survives outside L10"
    );
}

#[test]
fn patches_share_one_material_per_mask() {
    // Review item 4: every drawn patch uses the shared material for its
    // morph mask (against the draw set), patches with one mask share one
    // handle, and a steady frame changes no material. No-window app test.
    use crate::patch_material::{PatchMaterials, PlanetPatchMaterial};
    use universe_core::lod_morph::morph_mask_in;
    let mut app = headless_app();
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        for _ in 0..9 {
            let marker = universe.autopilot_target().expect("dive continues");
            assert!(universe.open(MarkerIndex(marker)), "marker opens");
        }
        assert_eq!(universe.level().get(), 10, "the dive ends at L10");
    }
    for _ in 0..10 {
        app.update();
    }
    let collect = |app: &mut App| {
        let mut query = app.world_mut().query::<(
            &PatchPiece,
            &PatchMask,
            &MeshMaterial3d<PlanetPatchMaterial>,
        )>();
        let mut rows: Vec<(PatchKey, u8, AssetId<PlanetPatchMaterial>)> = query
            .iter(app.world())
            .map(|(piece, mask, mat)| (piece.0, mask.0, mat.0.id()))
            .collect();
        rows.sort_by_key(|row| row.0);
        rows
    };
    let rows = collect(&mut app);
    assert!(!rows.is_empty(), "draws patches");
    let drawn: Vec<PatchKey> = rows.iter().map(|row| row.0).collect();
    let shared = app.world().resource::<PatchMaterials>();
    for (key, mask, id) in &rows {
        let want = morph_mask_in(*key, &|k| drawn.binary_search(&k).is_ok());
        assert_eq!(*mask, want, "patch {key:?} mask matches the draw set");
        let handle = shared.handles.get(mask).expect("shared handle for mask");
        assert_eq!(*id, handle.id(), "patch {key:?} uses the shared material");
    }
    assert!(
        shared.handles.len() <= rows.len(),
        "no more materials than patches"
    );
    let scale = shared.scale;
    assert!(scale.0 > 0.0 && scale.1 > 0.0, "band scale set {scale:?}");
    // The shared material count is bounded by masks, not patches.
    let materials = app.world().resource::<Assets<PlanetPatchMaterial>>();
    assert!(materials.len() <= 256, "one material per mask at most");
}

/// An app at the demo planet (L10), a few frames in.
fn planet_app() -> App {
    let mut app = headless_app();
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        for _ in 0..9 {
            let marker = universe.autopilot_target().expect("dive continues");
            assert!(universe.open(MarkerIndex(marker)), "marker opens");
        }
    }
    // Two metres over the ground, where the tree refines.
    app.world_mut()
        .resource_scope(|world, mut universe: Mut<Universe>| {
            let mut nav = world.resource_mut::<crate::input::Navigation>();
            crate::screens_sky::place_above_ground(&mut universe, &mut nav, 2_000);
        });
    app
}

#[test]
fn ground_memory_stays_under_the_cap() {
    // C5 (#918 AC5, P2 A): with a small Ground memory limit, every frame
    // keeps the cache under it, or holds only patches the frame needs (and
    // the ring shrinks); full CPU meshes stay near the player only.
    let mut app = planet_app();
    let cap = 6 << 20;
    app.world_mut().resource_mut::<PatchTasks>().cap_bytes = cap;
    for frame in 0..40 {
        app.update();
        let tasks = app.world().resource::<PatchTasks>();
        let cache = app.world().resource::<PatchCache>();
        let over = cache.bytes() > cap;
        if over {
            for key in cache.map.keys() {
                assert!(
                    key.level == 0 || tasks.kept.binary_search(key).is_ok(),
                    "frame {frame}: over the cap with an unneeded patch {key:?}"
                );
            }
        }
        for (key, patch) in &cache.map {
            if patch.cpu.is_some() && patch.gpu.is_some() && frame > 0 {
                assert!(
                    tasks.near.binary_search(key).is_ok(),
                    "frame {frame}: {key:?} keeps a CPU mesh far from the player"
                );
            }
        }
    }
    let cache = app.world().resource::<PatchCache>();
    let tasks = app.world().resource::<PatchTasks>();
    assert!(
        cache.bytes() > cap / 2,
        "the test presses on the cap: {} bytes",
        cache.bytes()
    );
    assert!(
        cache.len() > 6,
        "the ground builds under the cap: {} cached, {} bytes, build {} pending {}",
        cache.len(),
        cache.bytes(),
        tasks.build.len(),
        tasks.pending.len()
    );
}

#[test]
fn a_merge_keeps_its_children_until_built() {
    // #918 T5: splits cascade, so a parent is often never built. Its four
    // cached children give it relief numbers (so it can merge under the
    // memory cap), and it draws them until its own patch is built.
    use universe_core::cube_sphere::Face;
    use universe_core::lod_mesh::build_patch;
    use universe_core::nav::DEMO_SEED;
    let journey = universe_core::nest::autopilot_path(DEMO_SEED);
    let chain = journey.get(..9).expect("home chain");
    let relief = Relief::for_planet(DEMO_SEED, chain);
    let radius_m = PlanetFrame::for_planet(DEMO_SEED, chain).radius_m();
    let parent = PatchKey::new(Face::PosX, 3, 2, 5).expect("parent");
    let children = parent.children().expect("children");
    let mut cache = PatchCache::default();
    for child in children.iter().skip(1) {
        cache.insert(build_patch(*child, radius_m, &relief).expect("child"), None);
    }
    assert!(
        cache.relief_or_children(parent).is_none(),
        "three children say nothing"
    );
    let mut drawn = Vec::new();
    draw_selection_into(&[parent], &cache, &mut drawn);
    assert!(
        drawn.is_empty(),
        "no root cached, no child set: nothing drawn"
    );
    cache.insert(
        build_patch(children[0], radius_m, &relief).expect("child"),
        None,
    );
    let (error, min_h, max_h) = cache.relief_or_children(parent).expect("four children");
    for child in children {
        let (e, lo, hi) = cache.relief_info(child).expect("cached");
        assert!(
            e <= error && min_h <= lo && hi <= max_h,
            "{child:?} inside the union"
        );
    }
    draw_selection_into(&[parent], &cache, &mut drawn);
    let mut expect = children.to_vec();
    expect.sort();
    assert_eq!(drawn, expect, "the children stay drawn");
    cache.insert(
        build_patch(parent, radius_m, &relief).expect("parent"),
        None,
    );
    assert_eq!(cache.relief_or_children(parent), cache.relief_info(parent));
    draw_selection_into(&[parent], &cache, &mut drawn);
    assert_eq!(drawn, vec![parent], "built: the parent replaces them");
}

#[test]
fn the_memory_bias_steps_slowly() {
    // #918 T5: the detail coarsens one step after a run of blocked frames,
    // refines only after a long run with room, and stays in [1, 4].
    let cap = 100;
    let (mut bias, mut frames) = (1.0, 0);
    for _ in 0..BIAS_UP_FRAMES - 1 {
        (bias, frames) = bias_step(bias, frames, true, cap, cap);
    }
    assert!(
        (bias - 1.0).abs() < f64::EPSILON,
        "no step before the run ends"
    );
    (bias, frames) = bias_step(bias, frames, true, cap, cap);
    assert!(
        (bias - BIAS_STEP).abs() < 1e-12 && frames == 0,
        "one step up"
    );
    for _ in 0..BIAS_DOWN_FRAMES - 1 {
        (bias, frames) = bias_step(bias, frames, false, cap / 2, cap);
    }
    assert!(
        (bias - BIAS_STEP).abs() < 1e-12,
        "no step down before the run ends"
    );
    (bias, _) = bias_step(bias, frames, false, cap / 2, cap);
    assert!((bias - 1.0).abs() < 1e-12, "one step down, never below 1");
    assert_eq!(
        bias_step(1.0, -5, false, cap, cap),
        (1.0, 0),
        "near the cap: steady"
    );
    assert!(bias_step(BIAS_MAX, BIAS_UP_FRAMES, true, cap, cap).0 <= BIAS_MAX);
}

#[test]
fn the_body_stands_on_held_ground() {
    // #918 T5: the ground is ready near the player only when every near
    // drawn patch holds its CPU mesh, so the body never stands on a patch
    // without one.
    let mut app = planet_app();
    for _ in 0..30 {
        app.update();
        let ground = app.world().resource::<GroundSurface>();
        if !ground.1 {
            continue;
        }
        let tasks = app.world().resource::<PatchTasks>();
        let cache = app.world().resource::<PatchCache>();
        for key in &tasks.cover {
            if tasks.near.binary_search(key).is_ok() {
                assert!(cache.cpu(*key).is_some(), "{key:?} near without a CPU mesh");
            }
        }
    }
}
