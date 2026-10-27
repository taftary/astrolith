use super::*;
use crate::grid_tile::build_patch_and_tile_reusing;
use crate::patch_bench::{bench_keys, home_relief};

/// The four children of `key`.
fn children(key: PatchKey) -> Vec<PatchKey> {
    let mut out = Vec::new();
    for (cx, cy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        if let Some(child) = PatchKey::new(key.face, key.level + 1, key.x * 2 + cx, key.y * 2 + cy)
        {
            out.push(child);
        }
    }
    out
}

#[test]
fn a_child_built_from_its_parents_grid_is_the_same_bit_for_bit() {
    // #994 AC1 with the cache warm: patch and tile identical to a cold build.
    let (relief, radius_m) = home_relief();
    let mut reused = 0usize;
    for parent in bench_keys().into_iter().filter(|k| k.level < 18) {
        let (_, _, grid) =
            build_patch_and_tile_reusing(parent, radius_m, &relief, true, None).expect("parent");
        let grid = grid.expect("fine grid");
        assert_eq!(grid.key(), parent);
        for child in children(parent) {
            assert!(
                grid.parents(child, &relief, radius_m),
                "{child:?} reads its parent"
            );
            // Its whole own vertex grid lies in the parent's fine grid.
            let (i0, j0) = (child.x * PATCH_QUADS, child.y * PATCH_QUADS);
            assert!(grid.at(i0, j0).is_some() && grid.at(i0 + 32, j0 + 32).is_some());
            for full in [true, false] {
                let cold = build_patch_and_tile_reusing(child, radius_m, &relief, full, None)
                    .expect("cold");
                let warm =
                    build_patch_and_tile_reusing(child, radius_m, &relief, full, Some(&grid))
                        .expect("warm");
                assert_eq!(cold.0, warm.0, "{child:?} mesh");
                assert_eq!(cold.1, warm.1, "{child:?} tile");
                assert_eq!(cold.2, warm.2, "{child:?} fine grid");
            }
            reused += 1;
        }
    }
    assert!(reused >= 160, "{reused} children");
}

#[test]
fn a_grid_is_used_only_by_its_own_children_on_its_own_planet() {
    let (relief, radius_m) = home_relief();
    let parent = bench_keys()[3];
    let (_, _, grid) =
        build_patch_and_tile_reusing(parent, radius_m, &relief, true, None).expect("parent");
    let grid = Arc::new(grid.expect("fine grid"));
    let mut cache = HeightCache::new(2);
    cache.insert(grid.clone());
    let child = children(parent)[0];
    assert!(cache.parent_of(child, &relief, radius_m).is_some());
    let other = Relief::for_recipe(relief.seed() ^ 1, 1.0);
    assert!(
        cache.parent_of(child, &other, radius_m).is_none(),
        "other planet"
    );
    assert!(
        cache.parent_of(child, &relief, radius_m + 1.0).is_none(),
        "other radius"
    );
    assert!(
        cache.parent_of(parent, &relief, radius_m).is_none(),
        "not its own parent"
    );
    let grandchild = children(child)[0];
    assert!(
        cache.parent_of(grandchild, &relief, radius_m).is_none(),
        "two levels down"
    );
}

#[test]
fn the_cache_keeps_at_most_its_cap() {
    let (relief, radius_m) = home_relief();
    let mut cache = HeightCache::new(3);
    let keys = bench_keys();
    for key in keys.iter().take(5) {
        let (_, _, grid) =
            build_patch_and_tile_reusing(*key, radius_m, &relief, false, None).expect("build");
        cache.insert(Arc::new(grid.expect("fine grid")));
        assert!(cache.len() <= 3);
    }
    assert_eq!(cache.len(), 3);
    // The oldest went first: the first key's children find nothing.
    assert!(
        cache
            .parent_of(children(keys[0])[0], &relief, radius_m)
            .is_none()
    );
    assert!(
        cache
            .parent_of(children(keys[4])[0], &relief, radius_m)
            .is_some()
    );
    // The same patch again replaces its grid instead of adding one.
    let (_, _, again) =
        build_patch_and_tile_reusing(keys[4], radius_m, &relief, false, None).expect("again");
    cache.insert(Arc::new(again.expect("fine grid")));
    assert_eq!(cache.len(), 3);
    assert!(HeightCache::new(0).is_empty());
}
