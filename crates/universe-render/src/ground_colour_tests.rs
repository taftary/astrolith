//! Ground colour and patch size tests (#892 T2, #918 T4, #920), split from
//! `patches` under `E-FILE-SIZE`.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use universe_core::lod::PatchKey;
use universe_core::lod_mesh::build_patch;
use universe_core::planet_frame::PlanetFrame;
use universe_core::relief::Relief;

fn home() -> (PlanetFrame, Relief) {
    use universe_core::nav::DEMO_SEED;
    let journey = universe_core::nest::autopilot_path(DEMO_SEED);
    let chain = journey.get(..9).expect("home chain");
    (
        PlanetFrame::for_planet(DEMO_SEED, chain),
        Relief::for_planet(DEMO_SEED, chain),
    )
}

/// Edge bits of local vertex `v` (East, West, North, South, core
/// `odd_edge_bits`) in the shared grid's vertex order, as
/// `patch_grid_common::grid_at` and `edge_flags` work them out: the 33 x
/// 33 grid row by row, then the skirts South, North, West, East, 33 each,
/// which pin with their edge point.
fn vertex_edge_bits(v: u32) -> u8 {
    let side = 33u32;
    let (di, dj) = if v < side * side {
        (v % side, v / side)
    } else {
        let s = v - side * side;
        let k = s % side;
        match s / side {
            0 => (k, 0),
            1 => (k, side - 1),
            2 => (0, k),
            _ => (side - 1, k),
        }
    };
    universe_core::lod_morph::odd_edge_bits(di, dj)
}

#[test]
fn ground_has_one_colour() {
    // C3 (#892 T2, Q9 C): every ground patch takes the one ground token,
    // over basins and mountains alike. The colour is the materials' base
    // colour, and the shared grid carries no vertex colours (#918 T4, #920).
    use universe_core::cube_sphere::Face;
    let (frame, relief) = home();
    let (low, high) = relief.height_bounds_m();
    assert!(
        low < -1_000.0 && high > 1_000.0,
        "basins and mountains exist"
    );
    assert_eq!(
        crate::patch_material::ground_base().base_color,
        crate::tokens::RELIEF_GROUND_TINT
    );
    for face in Face::all() {
        let mesh =
            build_patch(PatchKey::root(face), frame.radius_m(), &relief).expect("root patch");
        let shared = crate::patch_grid::shared_mesh(&mesh).expect("shared mesh");
        assert!(
            shared.attribute(Mesh::ATTRIBUTE_COLOR).is_none(),
            "no vertex colours on face {face:?}"
        );
    }
}

#[test]
fn a_drawn_patch_costs_its_tile() {
    // #920 AC5: a patch's graphics memory is its tile in the shared heights
    // texture; the one shared grid mesh is made once per winding and keeps
    // no main-world copy.
    let (frame, relief) = home();
    let key = PatchKey::root(universe_core::cube_sphere::Face::PosY)
        .children()
        .expect("children")[0];
    let mesh = build_patch(key, frame.radius_m(), &relief).expect("patch");
    let shared = crate::patch_grid::shared_mesh(&mesh).expect("shared mesh");
    assert_eq!(shared.asset_usage, RenderAssetUsages::RENDER_WORLD);
    let cache = crate::patch_cache::PatchCache::default();
    assert_eq!(cache.gpu_each(), crate::patch_grid::TILE_BYTES);
    let tile = universe_core::grid_tile::grid_tile(&mesh, frame.radius_m(), &relief).expect("tile");
    assert_eq!(tile.len() * 4, crate::patch_grid::TILE_BYTES);
}

#[test]
fn the_default_limit_holds_far_more_patches() {
    // #920 AC6: at the default 64 MB a patch away from the player costs its
    // tile and its slim info, so over 5,000 fit, against under 900 with the
    // per-patch meshes (about 72 KB each, #918).
    use universe_core::settings::GroundMemory;
    let limit = GroundMemory::default().bytes();
    let each = crate::patch_grid::TILE_BYTES + crate::patch_cache::INFO_BYTES;
    let before = limit / (72 * 1024 + crate::patch_cache::INFO_BYTES);
    let fit = limit / each;
    assert!(fit > 5_000 && fit > 5 * before, "{fit} against {before}");
}

#[test]
fn edge_flags_follow_the_vertex_order() {
    // #918 T4: the shader works the edge flags out from the vertex index;
    // the order is the shared mesh's (grid row by row, then the skirts
    // South, North, West, East), so every skirt point pins with its edge
    // point.
    use universe_core::lod_morph::odd_edge_bits;
    let mut want = Vec::new();
    for dj in 0..=32u32 {
        for di in 0..=32u32 {
            want.push(odd_edge_bits(di, dj));
        }
    }
    for edge_no in 0..4u32 {
        for k in 0..=32u32 {
            let (di, dj) = match edge_no {
                0 => (k, 0),
                1 => (k, 32),
                2 => (0, k),
                _ => (32, k),
            };
            want.push(odd_edge_bits(di, dj));
        }
    }
    let got: Vec<u8> = (0..u32::try_from(want.len()).expect("count"))
        .map(vertex_edge_bits)
        .collect();
    assert_eq!(got, want);
}
