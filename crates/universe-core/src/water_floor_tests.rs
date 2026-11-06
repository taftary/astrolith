//! The water is a floor (#1162 T4, Spec v2 criteria 3 and 4).

use crate::grid_tile::{build_patch_and_tile, tile_water, water_nibble, water_scale_m};
use crate::lod::PatchKey;
use crate::planet_frame::PlanetFrame;
use crate::relief::Relief;
use crate::water_places::{fibonacci_dir, lat_lon_dir};

const HOME: [u32; 9] = [7, 2, 7, 1, 2, 0, 0, 0, 5];

fn home() -> (Relief, PlanetFrame) {
    let root = crate::nav::DEMO_SEED;
    (
        Relief::for_planet(root, &HOME),
        PlanetFrame::for_planet(root, &HOME),
    )
}

/// The home world's first lake centre among spread directions.
fn a_lake(relief: &Relief) -> [f64; 3] {
    (0..200_000)
        .find_map(|i| {
            let dir = fibonacci_dir(i, 200_000);
            relief.lake(dir).map(|(lake, _)| lake.seed.centre)
        })
        .expect("a lake on home land")
}

#[test]
#[ignore = "slow (~40 s on the dev PC): runs with --include-ignored (CI, validator), #847"]
fn the_player_walks_runs_and_lands_on_the_sea_and_a_lake() {
    // Criterion 4 (Q3 A): on the sea (the #1162 shallow place, 49 m over
    // the floor) and on a lake, the feet stay on the drawn water surface
    // as on flat ground: no sinking, no floating.
    let (relief, frame) = home();
    let sea = lat_lon_dir(-22.51002, 76.35610);
    assert!(relief.ground(sea).code.is_water(), "the sea place is wet");
    let lake = a_lake(&relief);
    assert!(
        relief.ground(lake).code.is_water(),
        "the lake centre is wet"
    );
    let limit = crate::headless_ground::FEET_LIMIT_M;
    for (name, dir) in [("sea", sea), ("lake", lake)] {
        let m = crate::headless_ground::walk_metrics_at(&relief, frame, dir).expect("walk");
        assert!(m.walk_gap_m < limit, "{name} walk {m:?}");
        assert!(m.run_gap_m < limit, "{name} run {m:?}");
        assert!(m.land_gap_m < limit, "{name} land {m:?}");
        assert!(m.landings >= 1, "{name} {m:?}");
    }
}

#[test]
fn the_surface_and_water_agree_across_cube_edges() {
    // Criterion 3: the drawn ground and its water code are one function of
    // the direction, so an edge point reached from two faces agrees.
    use crate::cube_sphere::{Face, grid_point};
    let (relief, _) = home();
    let n = 256u32;
    let faces = [
        Face::PosX,
        Face::NegX,
        Face::PosY,
        Face::NegY,
        Face::PosZ,
        Face::NegZ,
    ];
    let mut seen = std::collections::HashMap::new();
    let (mut shared, mut wet) = (0, 0);
    for face in faces {
        for k in 0..=n {
            for (i, j) in [(0, k), (n, k), (k, 0), (k, n)] {
                let Some(dir) = grid_point(face, i, j, n) else {
                    continue;
                };
                #[expect(clippy::cast_possible_truncation, reason = "E-CAST: test key")]
                let key = dir.map(|c| (c * 1e9).round() as i64);
                let g = relief.ground(dir);
                let value = (g.surface_m.to_bits(), g.code);
                if let Some(other) = seen.insert(key, value) {
                    assert_eq!(value, other, "edge point {dir:?}");
                    shared += 1;
                    wet += usize::from(g.code.is_water());
                }
            }
        }
    }
    assert!(
        shared > 6 * 4 * 128 && wet > 100,
        "{shared} shared, {wet} wet"
    );
}

/// Patches at `level` around the home coast place (lat -17.71, lon 29.87).
fn coast_patches(level: u32) -> Vec<PatchKey> {
    let dir = lat_lon_dir(-17.70972, 29.87377);
    let pos = crate::cube_sphere::from_sphere(dir).expect("face position");
    let per = f64::from(1u32 << level);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "E-CAST: patch indices from u, v in [-1, 1]"
    )]
    let (x, y) = (
        ((pos.u + 1.0) / 2.0 * per).floor() as u32,
        ((pos.v + 1.0) / 2.0 * per).floor() as u32,
    );
    let key = PatchKey::new(
        pos.face,
        level,
        x.min((1 << level) - 1),
        y.min((1 << level) - 1),
    )
    .expect("patch under the coast");
    let mut out = vec![key];
    for (dx, dy) in [(1i64, 0i64), (0, 1), (1, 1)] {
        let (x, y) = (i64::from(key.x) + dx, i64::from(key.y) + dy);
        if let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y))
            && let Some(k) = PatchKey::new(key.face, level, x, y)
        {
            out.push(k);
        }
    }
    out
}

#[test]
fn neighbouring_patches_and_levels_share_their_coast() {
    // Criterion 3: a coast vertex shared by two patches of one level, or by
    // a patch and its parent (the even points), carries the same height
    // and water code, so the coast has no seam and holds across levels.
    let (relief, frame) = home();
    let r = frame.radius_m();
    let side = 33usize;
    for level in [9u32, 12, 15] {
        let keys = coast_patches(level);
        let meshes: Vec<_> = keys
            .iter()
            .map(|k| build_patch_and_tile(*k, r, &relief, true).expect("build"))
            .collect();
        let (a, b) = (&meshes[0].0, &meshes[1].0);
        assert_eq!(a.water.len(), side * side);
        for j in 0..side {
            // East edge of `a` is the west edge of its x + 1 neighbour.
            let (ka, kb) = (j * side + side - 1, j * side);
            for c in 0..3 {
                let pa = a.centre.0[c] + f64::from(a.offsets[ka][c]);
                let pb = b.centre.0[c] + f64::from(b.offsets[kb][c]);
                assert!((pa - pb).abs() < 0.05, "level {level} row {j}");
            }
            assert_eq!(a.water[ka], b.water[kb], "level {level} row {j}");
        }
        // Parent and child: the child's even points are the parent's.
        let child =
            PatchKey::new(keys[0].face, level + 1, keys[0].x * 2, keys[0].y * 2).expect("child");
        let (c, _) = build_patch_and_tile(child, r, &relief, true).expect("child build");
        for dj in 0..=16 {
            for di in 0..=16 {
                assert_eq!(
                    c.water[(2 * dj) * side + 2 * di],
                    a.water[dj * side + di],
                    "level {level} child even point {di} {dj}"
                );
            }
        }
        // The tile holds every vertex's water as the shader reads it.
        let tile = meshes[0].1.as_ref().expect("tile");
        let quad = crate::lod_range::edge_m(keys[0].level, r) / f64::from(crate::lod::PATCH_QUADS);
        for (k, code) in a.water.iter().enumerate() {
            let k = u32::try_from(k).expect("index");
            assert_eq!(
                tile_water(tile, k),
                Some(water_nibble(*code, water_scale_m(quad)))
            );
        }
    }
}

#[test]
fn dry_worlds_build_patches_with_no_water() {
    let root = crate::nav::DEMO_SEED;
    let desert = [7, 2, 7, 2, 3781, 1, 0, 0, 0];
    let relief = Relief::for_planet(root, &desert);
    let frame = PlanetFrame::for_planet(root, &desert);
    let key = PatchKey::new(crate::cube_sphere::Face::PosX, 8, 100, 100).expect("key");
    let (mesh, tile) = build_patch_and_tile(key, frame.radius_m(), &relief, true).expect("build");
    assert!(mesh.water.is_empty());
    let tile = tile.expect("tile");
    for k in 0..1_089 {
        assert_eq!(tile_water(&tile, k), Some(0));
    }
}
