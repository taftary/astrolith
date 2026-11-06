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
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .filter_map(|line| line.split(':').next())
        .filter(|name| !name.is_empty())
        .collect();
    assert_eq!(
        fields,
        [
            "low",
            "high",
            "steep",
            "cap",
            "glow",
            "bands",
            "water",
            "sea_deep",
            "sea_shallow",
            "waves"
        ]
    );
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
    // Only the #1162 waves read the clock; the glow does not.
    let start = fragment.find("fn lava_glow(").expect("lava_glow");
    let glow = &fragment[start
        ..start
            + fragment[start..]
                .find(
                    "
}",
                )
                .expect("end")];
    assert!(
        !glow.contains("globals") && !glow.contains("wave_time"),
        "the glow must not read the clock (it would animate)"
    );
}

#[test]
fn only_ice_draws_cracks() {
    // Criterion 1: ice worlds draw crack lines (colour only, no own light).
    let relief = mercury();
    for kind in WorldType::ALL {
        let look = GroundLook::for_world(kind, &relief, Tier::Normal);
        let cracks = look.steep.w > 0.5;
        assert_eq!(cracks, kind == WorldType::Ice, "{kind:?}");
        if kind == WorldType::Ice {
            assert!(look.glow.w.abs() < f32::EPSILON, "ice cracks do not glow");
        }
    }
}

#[test]
fn terrain_quality_sets_how_far_the_waves_reach_on_wet_worlds_only() {
    // #1162 S1 (criterion 11): the waves' reach grows from Low to High,
    // Earth-like seas are liquid and ice-world seas frozen, and dry worlds
    // draw no water.
    use super::wave_reach_m;
    assert!(wave_reach_m(Tier::Low) < wave_reach_m(Tier::Normal));
    assert!(wave_reach_m(Tier::Normal) < wave_reach_m(Tier::High));
    let root = universe_core::nav::DEMO_SEED;
    let home = universe_core::relief::Relief::for_planet(root, &[7, 2, 7, 1, 2, 0, 0, 0, 5]);
    let ice = universe_core::relief::Relief::for_planet(root, &[7, 2, 7, 1, 303, 1, 0, 0, 1]);
    for tier in [Tier::Low, Tier::Normal, Tier::High] {
        let look = GroundLook::for_world(WorldType::EarthLike, &home, tier);
        assert!((look.water.x - 1.0).abs() < f32::EPSILON);
        assert!((look.water.z - wave_reach_m(tier)).abs() < f32::EPSILON);
        assert!(
            look.water.y > 0.9 && look.water.y < 0.95,
            "sea ice edge {}",
            look.water.y
        );
    }
    let frozen = GroundLook::for_world(WorldType::Ice, &ice, Tier::Normal);
    assert!((frozen.water.x - 2.0).abs() < f32::EPSILON);
    let dry = GroundLook::for_world(WorldType::Desert, &mercury(), Tier::Normal);
    assert!(dry.water.x.abs() < f32::EPSILON);
}
