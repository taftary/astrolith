//! Ground colour test (#892 T2), split from `patches` under `E-FILE-SIZE`.

use crate::patches::patch_bevy_mesh;
use bevy::prelude::*;
use universe_core::lod::PatchKey;
use universe_core::lod_mesh::build_patch;
use universe_core::planet_frame::PlanetFrame;
use universe_core::relief::Relief;

#[test]
fn ground_has_one_colour() {
    // C3 (#892 T2, Q9 C): every ground vertex, skirts included, takes
    // the one ground token, over basins and mountains alike (the six
    // root patches cover the whole home planet).
    use bevy::mesh::VertexAttributeValues;
    use universe_core::cube_sphere::Face;
    use universe_core::nav::DEMO_SEED;
    let journey = universe_core::nest::autopilot_path(DEMO_SEED);
    let chain = journey.get(..9).expect("home chain");
    let frame = PlanetFrame::for_planet(DEMO_SEED, chain);
    let relief = Relief::for_planet(DEMO_SEED, chain);
    let (low, high) = relief.height_bounds_m();
    assert!(
        low < -1_000.0 && high > 1_000.0,
        "basins and mountains exist"
    );
    let want = crate::tokens::RELIEF_GROUND_TINT.to_linear();
    let want = [want.red, want.green, want.blue, 1.0];
    for face in [
        Face::PosX,
        Face::NegX,
        Face::PosY,
        Face::NegY,
        Face::PosZ,
        Face::NegZ,
    ] {
        let mesh =
            build_patch(PatchKey::root(face), frame.radius_m(), &relief).expect("root patch");
        let bevy_mesh = patch_bevy_mesh(&mesh, &frame).expect("bevy mesh");
        let Some(VertexAttributeValues::Float32x4(colors)) =
            bevy_mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("colour attribute");
        };
        assert_eq!(colors.len(), mesh.offsets.len() + mesh.skirt_offsets.len());
        assert!(
            colors.iter().all(|c| *c == want),
            "one ground colour on face {face:?}"
        );
    }
}
