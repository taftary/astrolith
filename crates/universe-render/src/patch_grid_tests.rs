//! Shared-grid atlas tests, split from `patch_grid` under `E-FILE-SIZE`
//! (#920 T4).

use super::*;
use crate::patch_cache::PatchCache;
use crate::patches::PatchTree;

#[test]
fn freed_tiles_are_reused() {
    // A released tile goes to the next patch, and each assignment
    // queues one write for the render world.
    let mut atlas = GridAtlas::default();
    let a = PatchKey::new(universe_core::cube_sphere::Face::PosX, 1, 0, 0).expect("key");
    let b = PatchKey::new(universe_core::cube_sphere::Face::PosX, 1, 1, 0).expect("key");
    let first = atlas.assign(a, vec![0; 4]).expect("tile");
    assert_eq!(
        atlas.assign(a, vec![1; 4]),
        Some(first),
        "same key, same tile"
    );
    atlas.release(a);
    assert_eq!(atlas.tile(a), None);
    assert_eq!(
        atlas.assign(b, vec![2; 4]),
        Some(first),
        "the freed tile is reused"
    );
    assert_eq!(atlas.writes.len(), 2);
}

#[test]
fn every_cached_patch_keeps_its_tile_across_a_cache_clear() {
    // #920 T2: a cache clear (a new planet) used to release the old
    // keys only after the six roots took tiles again under the same
    // keys, so the roots lost their tiles and drew nothing.
    let mut app = crate::window_tests::headless_app();
    app.init_asset::<PatchGridMaterial>();
    app.insert_resource(GridAtlas::default());
    app.init_resource::<GridInbox>();
    {
        let mut universe = app.world_mut().resource_mut::<crate::Universe>();
        for _ in 0..9 {
            let marker = universe.autopilot_target().expect("dive continues");
            assert!(
                universe.open(universe_core::nav::MarkerIndex(marker)),
                "marker opens"
            );
        }
    }
    let check = |app: &mut App, when: &str| {
        let cache = app.world().resource::<PatchCache>();
        let atlas = app.world().resource::<GridAtlas>();
        for (key, patch) in &cache.map {
            if patch.gpu.is_some() {
                assert!(atlas.tile(*key).is_some(), "{when}: {key:?} lost its tile");
            }
        }
    };
    for _ in 0..5 {
        app.update();
    }
    check(&mut app, "settled");
    // A planet change clears the cache and rebuilds the roots.
    app.world_mut().resource_mut::<PatchCache>().seed = Some(u64::MAX);
    app.world_mut().resource_mut::<PatchTree>().tree = None;
    app.update();
    check(&mut app, "after the clear");
    app.update();
    check(&mut app, "the frame after");
}

#[test]
fn a_mask_change_writes_one_texel_after_its_tile() {
    // #920 T4: the mask lives in the tile, so a mask change is a one-texel
    // write queued after the tile's own write, never a material switch.
    let mut atlas = GridAtlas::default();
    let key = PatchKey::new(universe_core::cube_sphere::Face::PosX, 1, 0, 0).expect("key");
    let tile = atlas.assign(key, vec![0; 4]).expect("tile");
    atlas.set_mask(tile, 0x21);
    assert!(matches!(atlas.writes.first(), Some(GridWrite::Tile(t, _)) if *t == tile));
    assert!(matches!(atlas.writes.get(1), Some(GridWrite::Mask(t, 0x21)) if *t == tile));
}

#[test]
fn grid_materials_are_made_one_a_frame_finest_level_first() {
    // #920 T4: the GPU draws material batches in the order the materials
    // were prepared, so they are made one a frame, finest level first, and
    // every patch drawn meanwhile moves to its own level's once all exist.
    let mut app = App::new();
    app.add_plugins(AssetPlugin::default())
        .init_asset::<PatchGridMaterial>()
        .insert_resource(GridAtlas::default())
        .add_systems(Update, grow_materials);
    let early = {
        let world = app.world_mut();
        world.resource_scope(|world, mut atlas: Mut<GridAtlas>| {
            let mut materials = world.resource_mut::<Assets<PatchGridMaterial>>();
            atlas.material(3, &mut materials)
        })
    };
    let key = PatchKey::new(universe_core::cube_sphere::Face::PosX, 3, 0, 0).expect("key");
    let piece = app
        .world_mut()
        .spawn((
            crate::patches::PatchPiece(key),
            MeshMaterial3d(early.clone()),
        ))
        .id();
    let made = |app: &App| app.world().resource::<GridAtlas>().materials.len();
    assert_eq!(made(&app), 1, "the first patch makes one stand-in");
    let mut frames = 0;
    while made(&app) < LEVEL_MATERIALS {
        let before = made(&app);
        app.update();
        frames += 1;
        assert!(made(&app) <= before + 1, "at most one material a frame");
        assert!(frames < 100, "materials keep coming");
    }
    let atlas = app.world().resource::<GridAtlas>();
    assert_eq!(
        atlas.materials.first(),
        Some(&early),
        "the finest level's came first"
    );
    let own = atlas
        .materials
        .get(LEVEL_MATERIALS - 1 - 3)
        .cloned()
        .expect("level 3");
    app.update();
    let held = app
        .world()
        .get::<MeshMaterial3d<PatchGridMaterial>>(piece)
        .expect("material");
    assert_eq!(held.0, own, "the patch moved to its own level's material");
}

#[test]
fn the_shader_reads_the_core_tile_layout() {
    // #920 T6: the WGSL constants are the core tile layout's
    // (`universe_core::grid_tile`) and the page layout's here; a drift
    // would read the wrong texels with no error anywhere.
    use universe_core::grid_tile::{
        ERR_ROW, MASK_TEXEL, P_QUAD, PARAM_ROW, PARENT_NORMAL_ROW, STEP_U_ROW, STEP_V_ROW,
    };
    let shader = include_str!("patch_grid_common.wgsl");
    for line in [
        format!("const TILE_W: i32 = {TILE_W};"),
        format!("const TILE_H: i32 = {TILE_H};"),
        format!("const ERR_ROW: i32 = {ERR_ROW};"),
        format!("const PARAM_ROW: i32 = {PARAM_ROW};"),
        format!("const MASK_COL: i32 = {};", MASK_TEXEL[0]),
        format!("const QUAD_COL: i32 = {P_QUAD};"),
        format!("const STEP_U_ROW: i32 = {STEP_U_ROW};"),
        format!("const STEP_V_ROW: i32 = {STEP_V_ROW};"),
        format!("const PARENT_NORMAL_ROW: i32 = {PARENT_NORMAL_ROW};"),
        format!("const PAGE_COLS: u32 = {PAGE_COLS}u;"),
        format!("const PAGE_TILES: u32 = {PAGE_TILES}u;"),
    ] {
        assert!(
            shader.contains(&line),
            "patch_grid_common.wgsl must hold {line:?}"
        );
    }
    assert_eq!(MASK_TEXEL[1], PARAM_ROW);
}

#[test]
fn pages_follow_the_ground_memory_limit() {
    // #920 T6: the heights texture has room for a tile per patch the limit
    // holds, on 1,024-texel pages (phone-safe), and every tile lies inside
    // its page.
    use universe_core::settings::GroundMemory;
    for memory in GroundMemory::ALL {
        let pages = pages_for(memory.bytes());
        let room = usize::try_from(pages * PAGE_TILES).expect("tiles") * TILE_BYTES;
        assert!(room >= memory.bytes(), "{memory:?}: {pages} pages");
        assert!(room < memory.bytes() + PAGE_TILES as usize * TILE_BYTES);
        assert!(pages <= 256, "GLES3 layers");
    }
    assert_eq!(pages_for(0), 1);
    for tile in [
        0,
        1,
        26,
        27,
        PAGE_TILES - 1,
        PAGE_TILES,
        5 * PAGE_TILES + 100,
    ] {
        let (page, x, y) = tile_origin(tile);
        assert_eq!(page, tile / PAGE_TILES);
        assert!(x + TILE_W <= PAGE && y + TILE_H <= PAGE, "tile {tile}");
    }
}
