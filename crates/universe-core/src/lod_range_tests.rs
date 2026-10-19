//! Range tests (#817 T2): ranges follow patch size and detail; the scale is
//! continuous across seams and bounded by the patch maximum.

use crate::cube_sphere::{Face, FacePos, to_sphere};
use crate::lod::PatchKey;
use crate::lod_range::{LevelRanges, MAX_SCALE, edge_m, scale_at_dir};
use crate::planet_frame::EARTH_RADIUS_M;
use crate::precision::focal_px;
use crate::settings::PlanetDetail;

#[test]
fn ranges_follow_patch_size_and_detail() {
    let relief = crate::headless_morph::earth_relief();
    for detail in PlanetDetail::ALL {
        let ranges = LevelRanges::new(&relief, EARTH_RADIUS_M, detail.quad_px());
        for level in 0..18u32 {
            let coarse = ranges.range_m(level, 1.0);
            let fine = ranges.range_m(level + 1, 1.0);
            assert!(
                (coarse / fine - 2.0).abs() < 1e-12,
                "level {level}: ranges do not double"
            );
            // At the end of a range one quad covers quad_px pixels.
            let quad_m = edge_m(level, EARTH_RADIUS_M) / 32.0;
            let px = quad_m / coarse * focal_px();
            assert!(
                (px - detail.quad_px()).abs() < 1e-9,
                "{detail:?} level {level}: {px} px per quad"
            );
            // Scale multiplies the range.
            assert!((ranges.range_m(level, 3.0) / coarse - 3.0).abs() < 1e-12);
        }
    }
    assert_eq!(PlanetDetail::High.quad_px(), 2.0);
    assert_eq!(PlanetDetail::Normal.quad_px(), 4.0);
    assert_eq!(PlanetDetail::Low.quad_px(), 8.0);
}

#[test]
fn scale_is_continuous_and_bounded() {
    let relief = crate::headless_morph::earth_relief();
    let ranges = LevelRanges::new(&relief, EARTH_RADIUS_M, 4.0);
    // Lattice values equal the on-the-fly ones, inside [1, 4], and some
    // ground is rough (mountain belts).
    let mut rough = 0usize;
    for face in Face::all() {
        for k in 0..200u32 {
            let u = -0.999 + 1.998 * f64::from(k) / 199.0;
            let v = (f64::from(k) * 0.618).fract() * 1.998 - 0.999;
            let dir = to_sphere(FacePos::new(face, u, v).expect("pos")).expect("dir");
            let s = ranges.scale_at(dir);
            assert!((s - scale_at_dir(&relief, dir)).abs() < 1e-12);
            assert!((1.0..=MAX_SCALE).contains(&s), "scale {s}");
            if s > 1.5 {
                rough += 1;
            }
        }
    }
    assert!(rough > 10, "no rough ground found ({rough})");
    // Continuous across face seams: just either side of an edge.
    for face in Face::all() {
        for k in 1..50u32 {
            let t = -1.0 + 2.0 * f64::from(k) / 50.0;
            let edge = to_sphere(FacePos::new(face, 1.0, t).expect("pos")).expect("dir");
            let inside = to_sphere(FacePos::new(face, 1.0 - 1e-9, t).expect("pos")).expect("dir");
            let across = [
                edge[0] * 2.0 - inside[0],
                edge[1] * 2.0 - inside[1],
                edge[2] * 2.0 - inside[2],
            ];
            let gap = (ranges.scale_at(inside) - ranges.scale_at(across)).abs();
            assert!(gap < 1e-6, "{face:?} seam jump {gap}");
        }
    }
    // A patch's maximum bounds every point in it.
    for (level, x, y) in [(2u32, 1u32, 2u32), (6, 40, 17), (12, 3000, 1000)] {
        for face in Face::all() {
            let key = PatchKey::new(face, level, x, y).expect("key");
            let max = ranges.scale_max(key);
            let (u0, u1, v0, v1) = crate::lod::patch_uv_range(key).expect("uv");
            for a in 0..=8u32 {
                for b in 0..=8u32 {
                    let u = u0 + (u1 - u0) * f64::from(a) / 8.0;
                    let v = v0 + (v1 - v0) * f64::from(b) / 8.0;
                    let dir = to_sphere(FacePos::new(face, u, v).expect("pos")).expect("dir");
                    assert!(
                        ranges.scale_at(dir) <= max + 1e-12,
                        "{key:?} point above its max"
                    );
                }
            }
        }
    }
}
