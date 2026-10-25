//! Tile format tests (#920), split from `grid_tile` under `E-FILE-SIZE`.

use super::{
    TILE_H, TILE_W, build_patch_and_tile, f16_bits, f16_value, grid_tile, tile_morph_err,
    tile_offset, tile_parent_normal,
};
use crate::cube_sphere::Face;
use crate::grid_rebuild::{CENTRE_INDEX, steps_of};
use crate::lod::PatchKey;
use crate::lod_mesh::build_patch;
use crate::planet_frame::EARTH_RADIUS_M;

/// A coarse and a deep patch on every face.
fn sample_keys() -> Vec<PatchKey> {
    let mut keys = Vec::new();
    for face in Face::all() {
        for (level, x, y) in [(3u32, 2u32, 5u32), (14, 9_000, 7_001)] {
            keys.push(PatchKey::new(face, level, x, y).expect("key"));
        }
    }
    keys
}

#[test]
fn a_tile_is_at_most_ten_kilobytes() {
    // #920 AC5: each drawn ground patch uses at most 10 KB of graphics
    // memory: its tile of 32-bit texels.
    let bytes = TILE_W * TILE_H * 4;
    assert!(bytes <= 10 * 1024, "a tile is {bytes} bytes");
}

#[test]
fn a_tile_rebuilds_its_patch_under_a_millimetre() {
    // #920 T2: the packed tile alone (heights, frame values) gives back
    // today's offsets, on every face, at a coarse and a deep level, and
    // its morph errors within a half's precision (#920 T6: 16-bit shares
    // of the quad length).
    let relief = crate::headless_morph::earth_relief();
    for key in sample_keys() {
        let mesh = build_patch(key, EARTH_RADIUS_M, &relief).expect("mesh");
        let tile = grid_tile(&mesh, EARTH_RADIUS_M, &relief).expect("tile");
        assert_eq!(tile.len(), (TILE_W * TILE_H) as usize);
        let quad = crate::lod_range::edge_m(key.level, EARTH_RADIUS_M) / 32.0;
        for (index, today) in mesh.offsets.iter().enumerate() {
            let (k, l) = steps_of(index).expect("in patch");
            let (di, dj) = (k + CENTRE_INDEX, l + CENTRE_INDEX);
            let got = tile_offset(&tile, di, dj).expect("in tile");
            let gap = ((got[0] - today[0]).powi(2)
                + (got[1] - today[1]).powi(2)
                + (got[2] - today[2]).powi(2))
            .sqrt();
            let limit = if key.level >= 14 { 1e-3 } else { 1e-5 * quad };
            assert!(f64::from(gap) < limit, "{key:?} vertex {index}: {gap} m");
        }
        for (index, today) in mesh.morph_err_m.iter().enumerate() {
            let got = tile_morph_err(&tile, u32::try_from(index).expect("index")).expect("err");
            let gap = (got - today).abs();
            assert!(
                gap <= today.abs() * 2f32.powi(-10),
                "{key:?} morph error {index}: {got} against {today}"
            );
        }
    }
}

#[test]
fn stored_parent_normals_match_todays_within_a_twentieth_of_a_degree() {
    // #920 T4: the parent level's normals are stored packed at the 17 x 17
    // even points (16 bits a component since T6); an odd vertex's is the
    // mean of its pair's, as `lod_patch_morph` builds it (in f64 there,
    // f32 here: about 0.03 degrees at coarse levels). Both stay within 0.05
    // degrees of the f32 normals today's meshes carry.
    let relief = crate::headless_morph::earth_relief();
    let angle = |a: [f32; 3], b: [f32; 3]| {
        let dot = f64::from(a[0] * b[0] + a[1] * b[1] + a[2] * b[2]);
        dot.clamp(-1.0, 1.0).acos().to_degrees()
    };
    let unit = |v: [f32; 3]| {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        [v[0] / len, v[1] / len, v[2] / len]
    };
    for key in sample_keys() {
        let mesh = build_patch(key, EARTH_RADIUS_M, &relief).expect("mesh");
        let tile = grid_tile(&mesh, EARTH_RADIUS_M, &relief).expect("tile");
        let stored = |i: u32, j: u32| tile_parent_normal(&tile, i / 2, j / 2).expect("stored");
        let mut worst = 0.0f64;
        for (index, today) in mesh.parent_normals.iter().enumerate() {
            let i = u32::try_from(index % 33).expect("column");
            let j = u32::try_from(index / 33).expect("row");
            let got = match (i % 2 == 1, j % 2 == 1) {
                (false, false) => stored(i, j),
                (false, true) => {
                    let (a, b) = (stored(i, j - 1), stored(i, j + 1));
                    unit([a[0] + b[0], a[1] + b[1], a[2] + b[2]])
                }
                (true, false) => {
                    let (a, b) = (stored(i - 1, j), stored(i + 1, j));
                    unit([a[0] + b[0], a[1] + b[1], a[2] + b[2]])
                }
                (true, true) => {
                    let (a, b) = (stored(i + 1, j - 1), stored(i - 1, j + 1));
                    unit([a[0] + b[0], a[1] + b[1], a[2] + b[2]])
                }
            };
            worst = worst.max(angle(got, *today));
        }
        assert!(worst < 0.05, "{key:?}: {worst} degrees");
    }
}

#[test]
fn a_tile_built_with_its_patch_matches_one_packed_afterwards() {
    // #920 T5: reusing the build's own heights changes no texel.
    let relief = crate::headless_morph::earth_relief();
    for (face, level, x, y) in [
        (Face::PosY, 0u32, 0u32, 0u32),
        (Face::NegZ, 9, 300, 17),
        (Face::PosX, 14, 9_000, 7_001),
    ] {
        let key = PatchKey::new(face, level, x, y).expect("key");
        let (mesh, tile) = build_patch_and_tile(key, EARTH_RADIUS_M, &relief).expect("build");
        let after = grid_tile(&mesh, EARTH_RADIUS_M, &relief).expect("tile");
        assert_eq!(tile.expect("tile with the build"), after, "{key:?}");
    }
}

#[test]
fn halves_round_to_nearest_like_the_gpu() {
    // #920 T6: the CPU half matches IEEE binary16 (what WGSL's
    // `unpack2x16float` reads): exact values stay exact, others round to
    // nearest within half a unit in the last place, out of range goes to
    // infinity, tiny values to subnormals or zero.
    for (x, bits) in [
        (0.0f32, 0x0000u16),
        (1.0, 0x3C00),
        (-2.0, 0xC000),
        (0.5, 0x3800),
        (65_504.0, 0x7BFF),
        (65_520.0, 0x7C00),
        (2f32.powi(-14), 0x0400),
        (2f32.powi(-24), 0x0001),
        (2f32.powi(-26), 0x0000),
        (f32::INFINITY, 0x7C00),
    ] {
        assert_eq!(f16_bits(x), bits, "{x}");
    }
    let mut x = 1e-4f32;
    while x < 6e4 {
        let back = f16_value(f16_bits(x));
        assert!(
            (back - x).abs() <= x * 2f32.powi(-11),
            "{x} came back as {back}"
        );
        assert_eq!(f16_value(f16_bits(back)), back, "{back} is exact");
        x *= 1.013;
    }
}
