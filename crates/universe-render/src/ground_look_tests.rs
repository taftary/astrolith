//! Ground look tests (#1161 T4, Spec v2 criteria 1, 6, 7 and 11).

use super::{GroundLook, detail_reach_m};
use universe_core::settings::Tier;
use universe_core::world_type::WorldType;

fn mercury() -> universe_core::relief::Relief {
    universe_core::relief::Relief::for_planet(
        universe_core::nav::DEMO_SEED,
        &[7, 2, 7, 1, 2, 0, 0, 0, 0],
    )
}

#[test]
fn terrain_quality_sets_how_far_the_close_detail_reaches() {
    // S1 (criterion 11): Low reaches the least and costs the least.
    assert!(detail_reach_m(Tier::Low) < detail_reach_m(Tier::Normal));
    assert!(detail_reach_m(Tier::Normal) < detail_reach_m(Tier::High));
    let relief = mercury();
    let low = GroundLook::for_world(WorldType::Desert, &relief, Tier::Low);
    let high = GroundLook::for_world(WorldType::Desert, &relief, Tier::High);
    assert!(low.bands.z < high.bands.z);
    assert_eq!(low.low, high.low, "only the reach changes with the tier");
}

#[test]
fn every_type_has_its_own_ground_colour() {
    // Criterion 1: the types look different; each look carries its world's
    // radius and gravity scale, and marks airless worlds.
    let relief = mercury();
    let looks: Vec<GroundLook> = WorldType::ALL
        .into_iter()
        .map(|kind| GroundLook::for_world(kind, &relief, Tier::Normal))
        .collect();
    for (i, a) in looks.iter().enumerate() {
        assert!(a.low.w > 0.5, "a palette is set");
        for b in looks.iter().skip(i + 1) {
            let gap = (a.low.truncate() - b.low.truncate()).abs().max_element();
            assert!(gap > 0.02, "two types share a ground colour: {a:?} {b:?}");
        }
    }
    let mercury_look = &looks[1];
    assert!((mercury_look.bands.x - 2_440_000.0).abs() < 20_000.0);
    assert!((mercury_look.bands.w - 1.0).abs() < f32::EPSILON, "airless");
    let lava = GroundLook::for_world(WorldType::Lava, &relief, Tier::Normal);
    assert!(lava.glow.w > 0.5);
    assert!(
        GroundLook::default().low.w < 0.5,
        "no palette before a planet opens"
    );
}

#[test]
fn the_shaders_mirror_the_rust_layout_and_constants() {
    // The WGSL `GroundLook` lists the Rust fields in order, and the close
    // detail reads the tile slots and period core writes.
    let fragment = include_str!("patch_grid_fragment.wgsl");
    let common = include_str!("patch_grid_common.wgsl");
    let start = common.find("struct GroundLook {").expect("struct");
    let body = &common[start..start + common[start..].find("};").expect("end")];
    let fields: Vec<&str> = body
        .lines()
        .skip(1)
        .filter_map(|line| line.trim().split(':').next())
        .filter(|name| !name.is_empty())
        .collect();
    assert_eq!(fields, ["low", "high", "steep", "cap", "glow", "bands"]);
    let period = format!(
        "const NOISE_PERIOD_M: f32 = {:.1};",
        universe_core::grid_tile::NOISE_PERIOD_M
    );
    assert!(fragment.contains(&period), "{period}");
    let col = format!(
        "const NOISE_COL: i32 = {};",
        universe_core::grid_tile::P_NOISE[0]
    );
    assert!(common.contains(&col), "{col}");
    assert_eq!(universe_core::grid_tile::P_NOISE, [11, 12, 13]);
}

#[test]
fn only_lava_has_its_own_light_and_it_stays_steady() {
    // T5 (criterion 8): the glow is lava's own light, set through the
    // emissive input only on lava worlds; the shader reads no time, so it
    // does not animate (Q6 A), and it lights nothing but its own ground.
    let relief = mercury();
    for kind in WorldType::ALL {
        let look = GroundLook::for_world(kind, &relief, Tier::Normal);
        if kind == WorldType::Lava {
            assert!((look.glow.w - super::LAVA_GLOW).abs() < f32::EPSILON);
            assert!(look.glow.x > look.glow.z, "orange, not white");
        } else {
            assert!(look.glow.w.abs() < f32::EPSILON, "{kind:?} glows");
        }
    }
    let fragment = include_str!("patch_grid_fragment.wgsl");
    assert!(fragment.contains("pbr_input.material.emissive"));
    assert!(
        !fragment.contains("globals"),
        "the glow must not read the clock (it would animate)"
    );
}
