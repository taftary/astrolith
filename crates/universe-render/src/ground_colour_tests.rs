//! Ground colour and patch size tests (#892 T2, #918 T4), split from
//! `patches` under `E-FILE-SIZE`.

use crate::patches::{patch_bevy_mesh, vertex_edge_bits};
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

#[test]
fn ground_has_one_colour() {
    // C3 (#892 T2, Q9 C): every ground patch takes the one ground token,
    // over basins and mountains alike. Since #918 T4 the colour is the
    // material's base colour and no patch mesh carries vertex colours that
    // could differ.
    use universe_core::cube_sphere::Face;
    let (frame, relief) = home();
    let (low, high) = relief.height_bounds_m();
    assert!(
        low < -1_000.0 && high > 1_000.0,
        "basins and mountains exist"
    );
    let material = crate::patch_material::patch_material(0, 1.0, 1.0);
    assert_eq!(material.base.base_color, crate::tokens::RELIEF_GROUND_TINT);
    for face in Face::all() {
        let mesh =
            build_patch(PatchKey::root(face), frame.radius_m(), &relief).expect("root patch");
        let bevy_mesh = patch_bevy_mesh(&mesh, &frame).expect("bevy mesh");
        assert!(
            bevy_mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_none(),
            "no vertex colours on face {face:?}"
        );
    }
}

#[test]
fn drawn_patch_is_small() {
    // C6 (#918 AC6): a drawn patch's GPU buffers stay at about 75 KB or
    // less, and the mesh keeps no main-world copy.
    let (frame, relief) = home();
    let key = PatchKey::root(universe_core::cube_sphere::Face::PosY)
        .children()
        .expect("children")[0];
    let mesh = build_patch(key, frame.radius_m(), &relief).expect("patch");
    let bevy_mesh = patch_bevy_mesh(&mesh, &frame).expect("bevy mesh");
    assert_eq!(bevy_mesh.asset_usage, RenderAssetUsages::RENDER_WORLD);
    let vertex_bytes: usize = bevy_mesh
        .attributes()
        .map(|(_, values)| values.get_bytes().len())
        .sum();
    let index_bytes = bevy_mesh.indices().map_or(0, |indices| match indices {
        bevy::mesh::Indices::U16(i) => i.len() * 2,
        bevy::mesh::Indices::U32(i) => i.len() * 4,
    });
    let total = vertex_bytes + index_bytes;
    assert!(total <= 75_000, "a drawn patch is {total} bytes");
    assert_eq!(crate::patches::gpu_bytes_per_patch(), total);
}

#[test]
fn edge_flags_follow_the_vertex_order() {
    // #918 T4: the shader works the edge flags out from the vertex index;
    // the order is the mesh's (grid row by row, then the skirts South,
    // North, West, East), so every skirt point pins with its edge point.
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
