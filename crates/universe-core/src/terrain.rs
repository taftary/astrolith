//! L11 cube-sphere terrain with level of detail (M5, sub-issue #42).
//!
//! Planets carry a heightmap on a cube-sphere: each of the six cube faces
//! samples the SAME 3D fractal field through [`height_at`], so a shared edge
//! or corner maps to identical 3D points from every adjacent face and borders
//! agree by construction. [`biome_for`] tags a sample from its height and
//! latitude, and [`lod_for`] selects the subdivision depth from the camera
//! distance (monotonic: nearer cameras never select coarser detail).

use crate::noise::fbm_3d;
use crate::r#gen::{Constraints, Generated, Generator, Point};
use std::f64::consts::PI;

/// Fractal octaves summed by [`height_at`].
///
/// Five octaves span continent swells down to mountain-range detail at
/// indicator resolution.
pub const TERRAIN_OCTAVES: u32 = 5;

/// Frequency scaling cube coordinates (`[-1.0, 1.0]`) into field space.
pub const TERRAIN_FREQUENCY: f64 = 2.0;

/// Deepest terrain subdivision (camera at or inside the surface).
pub const MAX_LOD: u8 = 8;

/// Reference camera distance, in meters, holding [`MAX_LOD`].
///
/// Each doubling of distance above this drops one LOD rung.
pub const LOD_REFERENCE_DISTANCE: f64 = 2_000.0;

/// Surface biome tag from height and latitude (see [`biome_for`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Biome {
    /// Deep water.
    Ocean,
    /// Shallow shelf at the waterline.
    Coast,
    /// Hot arid lowland.
    Desert,
    /// Temperate grassland.
    Grassland,
    /// Humid woodland.
    Forest,
    /// Cold treeless high latitude.
    Tundra,
    /// Polar ice.
    IceCap,
    /// High peaks.
    Mountain,
}

/// Samples the L11 heightmap at `(u, v)` on cube `face`.
///
/// Faces are `0..=5` (`+X`, `-X`, `+Y`, `-Y`, `+Z`, `-Z`); `u`/`v` span
/// `[0.0, 1.0]` edge to edge, with out-of-range values still mapping
/// deterministically through the global field. The `(face, u, v)` triple
/// maps to a 3D cube point sampled with fractal value noise over
/// [`TERRAIN_OCTAVES`] octaves, so the result is deterministic on every
/// platform in `[0.0, 1.0]` (`0.0` abyss, `1.0` peak). Faces outside `0..=5`
/// have no cube point and yield `0.0`; non-finite `u`/`v` yield the neutral
/// `0.5` instead of propagating `NaN`.
///
/// Edge layout (with `a = 2u - 1`, `b = 2v - 1`): face 0 is `(1, a, b)`,
/// face 1 is `(-1, a, b)`, face 2 is `(a, 1, b)`, face 3 is `(a, -1, b)`,
/// face 4 is `(a, b, 1)`, face 5 is `(a, b, -1)`.
pub fn height_at(seed: u64, face: u8, u: f64, v: f64) -> f64 {
    let Some(point) = cube_point(face, u, v) else {
        return 0.0;
    };
    fbm_3d(
        seed,
        point[0] * TERRAIN_FREQUENCY,
        point[1] * TERRAIN_FREQUENCY,
        point[2] * TERRAIN_FREQUENCY,
        TERRAIN_OCTAVES,
    )
}

/// Maps `(face, u, v)` onto the cube surface; `None` for faces above 5.
fn cube_point(face: u8, u: f64, v: f64) -> Option<[f64; 3]> {
    let a = u * 2.0 - 1.0;
    let b = v * 2.0 - 1.0;
    match face {
        0 => Some([1.0, a, b]),
        1 => Some([-1.0, a, b]),
        2 => Some([a, 1.0, b]),
        3 => Some([a, -1.0, b]),
        4 => Some([a, b, 1.0]),
        5 => Some([a, b, -1.0]),
        _ => None,
    }
}

/// Tags a surface sample with a biome from height and latitude.
///
/// `height` is the [`height_at`] value clamped to `[0.0, 1.0]`; `latitude`
/// is radians clamped to `[-PI/2, PI/2]` (positive is north). Non-finite
/// heights read as shoreline, non-finite latitudes as the equator.
///
/// | Height | Latitude | Biome |
/// |---|---|---|
/// | `< 0.40` | any | [`Biome::Ocean`] |
/// | `< 0.45` | any | [`Biome::Coast`] |
/// | `>= 0.45` | `\|lat\| > 1.15` | [`Biome::IceCap`] |
/// | `> 0.80` | cooler than ice | [`Biome::Mountain`] |
/// | `>= 0.45` | `\|lat\| > 0.90` | [`Biome::Tundra`] |
/// | `< 0.55` | `\|lat\| < 0.35` | [`Biome::Desert`] |
/// | `< 0.62` | milder | [`Biome::Grassland`] |
/// | otherwise | otherwise | [`Biome::Forest`] |
pub fn biome_for(height: f64, latitude: f64) -> Biome {
    let clamped_height = if height.is_finite() {
        height.clamp(0.0, 1.0)
    } else {
        0.42
    };
    let clamped_latitude = if latitude.is_finite() {
        latitude.clamp(-PI / 2.0, PI / 2.0)
    } else {
        0.0
    };
    let polar = clamped_latitude.abs();
    if clamped_height < 0.40 {
        Biome::Ocean
    } else if clamped_height < 0.45 {
        Biome::Coast
    } else if polar > 1.15 {
        Biome::IceCap
    } else if clamped_height > 0.80 {
        Biome::Mountain
    } else if polar > 0.90 {
        Biome::Tundra
    } else if clamped_height < 0.55 && polar < 0.35 {
        Biome::Desert
    } else if clamped_height < 0.62 {
        Biome::Grassland
    } else {
        Biome::Forest
    }
}

/// Selects the terrain subdivision for a camera `distance`, in meters.
///
/// [`MAX_LOD`] at or below [`LOD_REFERENCE_DISTANCE`], minus one rung per
/// doubling above it, floored at `0`. Monotonic non-increasing in distance:
/// nearer cameras never select coarser detail. Non-finite or non-positive
/// distances (camera at or inside the surface) request [`MAX_LOD`].
pub fn lod_for(distance: f64) -> u8 {
    if !distance.is_finite() || distance <= 0.0 {
        return MAX_LOD;
    }
    let mut lod = MAX_LOD;
    let mut threshold = LOD_REFERENCE_DISTANCE;
    while distance > threshold && lod > 0 {
        threshold *= 2.0;
        lod -= 1;
    }
    lod
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_is_deterministic_and_seed_sensitive() {
        let seed = 0xCAFE_F00D;
        assert_eq!(
            height_at(seed, 4, 0.25, 0.75).to_bits(),
            height_at(seed, 4, 0.25, 0.75).to_bits()
        );
        assert_ne!(
            height_at(seed, 4, 0.25, 0.75).to_bits(),
            height_at(seed ^ 0x1, 4, 0.25, 0.75).to_bits()
        );
    }

    #[test]
    fn height_stays_in_unit_range() {
        let seed = 4242;
        for face in 0..6u8 {
            for t in [0.0, 0.25, 0.5, 0.75, 1.0, -0.5, 1.5] {
                let height = height_at(seed, face, t, 1.0 - t);
                assert!((0.0..=1.0).contains(&height), "out of range: {height}");
            }
        }
        assert_eq!(
            height_at(seed, 0, f64::NAN, 0.5).to_bits(),
            0.5f64.to_bits()
        );
    }

    #[test]
    fn shared_edges_and_corners_agree_across_faces() {
        let seed = 0xCAFE_F00D;
        // Edge (1, 1, b) shared by faces 0 (+X, u=1) and 2 (+Y, u=1).
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let from_plus_x = height_at(seed, 0, 1.0, t);
            let from_plus_y = height_at(seed, 2, 1.0, t);
            assert!(
                (from_plus_x - from_plus_y).abs() <= 1e-12,
                "edge mismatch at t={t}: {from_plus_x} vs {from_plus_y}"
            );
        }
        // Edge (-1, -1, b) shared by faces 1 (-X, u=0) and 3 (-Y, u=0).
        for t in [0.0, 0.5, 1.0] {
            let from_minus_x = height_at(seed, 1, 0.0, t);
            let from_minus_y = height_at(seed, 3, 0.0, t);
            assert!(
                (from_minus_x - from_minus_y).abs() <= 1e-12,
                "edge mismatch at t={t}: {from_minus_x} vs {from_minus_y}"
            );
        }
        // Corner (1, 1, 1) shared by faces 0, 2, and 4.
        let corner = [
            height_at(seed, 0, 1.0, 1.0),
            height_at(seed, 2, 1.0, 1.0),
            height_at(seed, 4, 1.0, 1.0),
        ];
        for pair in corner.windows(2) {
            assert!(
                (pair[0] - pair[1]).abs() <= 1e-12,
                "corner mismatch: {} vs {}",
                pair[0],
                pair[1]
            );
        }
        // Corner (-1, -1, -1) shared by faces 1, 3, and 5.
        let opposite = [
            height_at(seed, 1, 0.0, 0.0),
            height_at(seed, 3, 0.0, 0.0),
            height_at(seed, 5, 0.0, 0.0),
        ];
        for pair in opposite.windows(2) {
            assert!(
                (pair[0] - pair[1]).abs() <= 1e-12,
                "corner mismatch: {} vs {}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn unknown_face_yields_zero() {
        assert_eq!(height_at(7, 6, 0.5, 0.5).to_bits(), 0.0f64.to_bits());
        assert_eq!(
            height_at(7, u8::MAX, 0.5, 0.5).to_bits(),
            0.0f64.to_bits()
        );
    }

    #[test]
    fn biome_thresholds_match_documented_table() {
        assert_eq!(biome_for(0.20, 0.0), Biome::Ocean);
        assert_eq!(biome_for(0.43, 0.0), Biome::Coast);
        assert_eq!(biome_for(0.90, 0.0), Biome::Mountain);
        assert_eq!(biome_for(0.60, 1.30), Biome::IceCap);
        assert_eq!(biome_for(0.60, 1.00), Biome::Tundra);
        assert_eq!(biome_for(0.50, 0.00), Biome::Desert);
        assert_eq!(biome_for(0.60, 0.50), Biome::Grassland);
        assert_eq!(biome_for(0.70, 0.20), Biome::Forest);
        // Degenerate inputs degrade gracefully, never panic.
        assert_eq!(biome_for(f64::NAN, 0.0), Biome::Coast);
        assert_eq!(biome_for(0.60, f64::NAN), Biome::Grassland);
        assert_eq!(biome_for(5.0, 0.0), Biome::Mountain);
        assert_eq!(biome_for(-3.0, 0.0), Biome::Ocean);
    }

    #[test]
    fn lod_is_monotonic_in_distance() {
        assert_eq!(lod_for(0.0), MAX_LOD);
        assert_eq!(lod_for(-100.0), MAX_LOD);
        assert_eq!(lod_for(f64::NAN), MAX_LOD);
        assert_eq!(lod_for(f64::INFINITY), MAX_LOD);
        assert_eq!(lod_for(LOD_REFERENCE_DISTANCE), MAX_LOD);
        assert_eq!(lod_for(LOD_REFERENCE_DISTANCE + 1.0), MAX_LOD - 1);
        assert_eq!(lod_for(1.0e12), 0);
        let mut previous = MAX_LOD;
        let mut distance = 1.0;
        while distance < 1.0e12 {
            let lod = lod_for(distance);
            assert!(lod <= previous, "LOD rose with distance: {lod} after {previous}");
            previous = lod;
            distance *= 1.5;
        }
        assert_eq!(previous, 0);
    }
}

/// Grid samples per cube-face axis taken by [`TerrainSampler`].
///
/// Six faces of `SAMPLER_GRID x SAMPLER_GRID` give 54 candidates; the sampler
/// emits at most the parent budget from this fixed order.
pub const SAMPLER_GRID: u32 = 3;

/// Generator adapter sampling terrain heightmap points (M5 demo/`--verify`).
///
/// Maps `(face, u, v)` samples of [`height_at`] to cell-local indicator
/// points: `[u - 0.5, (h - 0.5) * 0.25, v - 0.5]`, all inside the cell bounds.
/// Children receive halved budgets, so `respects` holds for every child
/// against the parent the cell was generated with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainSampler;

impl TerrainSampler {
    /// Creates a terrain point sampler (count comes from the parent budget).
    pub const fn new() -> TerrainSampler {
        TerrainSampler
    }
}

impl Default for TerrainSampler {
    /// Creates a default terrain point sampler.
    fn default() -> TerrainSampler {
        TerrainSampler::new()
    }
}

impl Generator for TerrainSampler {
    /// Samples the heightmap grid up to `parent.max_count` points.
    ///
    /// Returns empty output for an invalid parent rather than panicking.
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        if !parent.is_valid() {
            return Generated {
                points: Vec::new(),
                child_constraints: Vec::new(),
            };
        }
        let mut points = Vec::new();
        for face in 0..6u8 {
            for iu in 0..SAMPLER_GRID {
                for iv in 0..SAMPLER_GRID {
                    if points.len() >= parent.max_count as usize {
                        break;
                    }
                    let u = (f64::from(iu) + 0.5) / f64::from(SAMPLER_GRID);
                    let v = (f64::from(iv) + 0.5) / f64::from(SAMPLER_GRID);
                    let height = height_at(seed, face, u, v);
                    points.push(Point {
                        position: [u - 0.5, (height - 0.5) * 0.25, v - 0.5],
                        radius: 0.01,
                    });
                }
            }
        }
        let child = Constraints {
            density_multiplier: parent.density_multiplier / 2.0,
            min_count: parent.min_count.min(parent.max_count / 2),
            max_count: parent.max_count / 2,
            allowed_extent: parent.allowed_extent,
        };
        Generated {
            points,
            child_constraints: vec![child; 8],
        }
    }
}

#[cfg(test)]
mod sampler_tests {
    use super::*;
    use crate::r#gen::respects;
    use crate::snapshot::snapshot_generated;

    /// Builds the standard demo budget (mirrors the render demo for L11).
    fn demo_budget() -> Constraints {
        Constraints::new(1.0, 8, 24, [0.5, 0.5, 0.5]).expect("valid test budget")
    }

    #[test]
    fn sampler_is_deterministic() {
        let sampler = TerrainSampler::new();
        let first = snapshot_generated(&sampler.generate(42, &demo_budget()));
        let second = snapshot_generated(&sampler.generate(42, &demo_budget()));
        assert_eq!(first, second);
        assert!(first.contains("points=24"));
    }

    #[test]
    fn sampler_honors_parent_budget() {
        let sampler = TerrainSampler::new();
        let tight = Constraints::new(1.0, 2, 10, [0.5, 0.5, 0.5]).expect("valid");
        let out = sampler.generate(7, &tight);
        assert!((tight.min_count as usize..=tight.max_count as usize).contains(&out.points.len()));
        for point in &out.points {
            for axis in point.position {
                assert!((-0.5..0.5).contains(&axis), "out of cell: {axis}");
            }
            assert!(point.radius > 0.0);
        }
    }

    #[test]
    fn sampler_children_respect_parent() {
        let sampler = TerrainSampler::new();
        let parent = demo_budget();
        for child in &sampler.generate(7, &parent).child_constraints {
            assert!(respects(child, &parent));
        }
    }

    #[test]
    fn sampler_rejects_invalid_parent() {
        let sampler = TerrainSampler::new();
        let bad = Constraints {
            density_multiplier: 2.0,
            min_count: 9,
            max_count: 3,
            allowed_extent: [0.5, 0.5, 0.5],
        };
        assert!(!bad.is_valid());
        assert!(sampler.generate(7, &bad).points.is_empty());
    }
}
