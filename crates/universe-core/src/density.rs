//! L1-L3 density field and cluster placement (M3, sub-issue #40).
//!
//! Levels L1-L3 describe the cosmic web as one continuous density field: the
//! same [`density_at`] value-noise function is sampled by every cell, so two
//! neighbouring cells evaluating their shared face in global coordinates
//! read bit-identical densities and borders agree by construction.
//! [`clusters_in_cell`] turns the field into indicator [`Generated`] content:
//! candidates are rejection-sampled toward overdensities (clusters) while
//! the few that miss every try stay as small void markers, so the point
//! count always honors the parent budget. [`DensityGenerator`] fixes the cell
//! for one place so the headless checks can use the [`Generator`] contract.
//!
//! [`Generated`]: crate::r#gen::Generated
//! [`Generator`]: crate::r#gen::Generator

use crate::coords::{CellPos, Level};
use crate::noise::fbm_3d;
use crate::r#gen::{Constraints, Generated, Generator, Point};
use crate::seed::{Rng, hash_cell};

/// Value-noise octaves summed by [`density_at`].
///
/// Three octaves (base lattice plus two doublings) give filament-scale
/// structure without high-frequency glitter at indicator resolution.
pub const DENSITY_OCTAVES: u32 = 3;

/// Field value at or above which a sample counts as a cluster member.
///
/// The fractal field is normalized to `[0.0, 1.0]` with a mid-grey mean, so
/// `0.5` accepts roughly half the candidates per try.
pub const CLUSTER_THRESHOLD: f64 = 0.5;

/// Baseline indicator points emitted at full parent density.
///
/// The parent budget scales this (`round(BASE * density)`, clamped to
/// `[min_count, max_count]`), exactly like the reference generator in
/// [`crate::r#gen`].
pub const DENSITY_BASE_COUNT: u32 = 32;

/// Rejection tries per point before a candidate is kept as a void marker.
///
/// Eight tries at roughly 50% acceptance keep almost every point inside a
/// cluster while bounding the work per cell.
const MAX_DENSITY_TRIES: u32 = 8;

/// Indicator radius of a cluster member, in cell units.
const CLUSTER_RADIUS: f64 = 0.02;

/// Indicator radius of a void marker, in cell units.
const VOID_RADIUS: f64 = 0.008;

/// Samples the L1-L3 cosmic-web density field at `(x, y, z)`.
///
/// Fractal value noise over [`DENSITY_OCTAVES`] octaves, hashed from `seed`
/// with no lookup tables: deterministic on every platform, returning a value
/// in `[0.0, 1.0]`. The field is `C1`-continuous, so nearby samples agree
/// within a small epsilon (neighbour continuity). Non-finite coordinates
/// yield the neutral value `0.5` instead of propagating `NaN`.
pub fn density_at(seed: u64, x: f64, y: f64, z: f64) -> f64 {
    fbm_3d(seed, x, y, z, DENSITY_OCTAVES)
}

/// Generates cluster/void indicator points for one cell.
///
/// `seed` is the density-field seed shared by neighbouring cells; `cell`
/// locates this cell in the global lattice, so a sample at local offset `o`
/// reads `density_at(seed, cell + o)` and the shared face of two adjacent
/// cells maps to identical field coordinates. Candidates scatter uniformly
/// in `parent.allowed_extent` from a stream seeded by `hash_cell`, and each
/// is re-rolled until its field value reaches [`CLUSTER_THRESHOLD`]; the
/// first try that passes is kept with cluster radius, while a candidate
/// that misses every try is kept as a small void marker so the emitted
/// count stays inside `[min_count, max_count]`. Each of the 8 child octants
/// receives half the parent density and half the count ceiling, so
/// ceiling, so [`respects`](crate::r#gen::respects) holds for every child.
/// An invalid `parent` yields empty output rather than panicking.
pub fn clusters_in_cell(seed: u64, cell: CellPos, parent: &Constraints) -> Generated {
    if !parent.is_valid() {
        return Generated {
            points: Vec::new(),
            child_constraints: Vec::new(),
        };
    }
    let wanted = (f64::from(DENSITY_BASE_COUNT) * parent.density_multiplier).round() as u32;
    let count = wanted.clamp(parent.min_count, parent.max_count);
    let cell_seed = hash_cell(seed, cell.level.get(), cell.x, cell.y, cell.z);
    let mut rng = Rng::new(cell_seed);
    let origin = [cell.x as f64, cell.y as f64, cell.z as f64];
    let mut points = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let mut position = [0.0; 3];
        let mut density = 0.0;
        for _ in 0..MAX_DENSITY_TRIES {
            for (axis, extent) in parent.allowed_extent.iter().enumerate() {
                position[axis] = (rng.next_f64() * 2.0 - 1.0) * extent;
            }
            density = density_at(
                seed,
                origin[0] + position[0],
                origin[1] + position[1],
                origin[2] + position[2],
            );
            if density >= CLUSTER_THRESHOLD {
                break;
            }
        }
        let radius = if density >= CLUSTER_THRESHOLD {
            CLUSTER_RADIUS
        } else {
            VOID_RADIUS
        };
        points.push(Point { position, radius });
    }
    let child_max = parent.max_count / 2;
    let child = Constraints {
        density_multiplier: parent.density_multiplier / 2.0,
        min_count: parent.min_count.min(child_max),
        max_count: child_max,
        allowed_extent: parent.allowed_extent,
    };
    Generated {
        points,
        child_constraints: vec![child; 8],
    }
}

/// Density-field generator fixed to one cell (M3 [`Generator`] adapter).
///
/// Delegates to [`clusters_in_cell`]: `seed` stays the shared field seed
/// while the stored cell provides the global lattice offset.
///
/// [`Generator`]: crate::r#gen::Generator
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DensityGenerator {
    /// Level of the cell this generator is fixed to.
    pub level: Level,
    /// Integer grid index of the cell along x.
    pub x: i64,
    /// Integer grid index of the cell along y.
    pub y: i64,
    /// Integer grid index of the cell along z.
    pub z: i64,
}

impl DensityGenerator {
    /// Fixes a density generator to the cell `(level, x, y, z)`.
    pub const fn new(level: Level, x: i64, y: i64, z: i64) -> DensityGenerator {
        DensityGenerator { level, x, y, z }
    }
}

impl Generator for DensityGenerator {
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        clusters_in_cell(
            seed,
            CellPos::new(self.level, self.x, self.y, self.z),
            parent,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r#gen::respects;

    /// Builds typical mid-ladder constraints for tests.
    fn parent_constraints() -> Constraints {
        Constraints::new(1.0, 2, 64, [0.5, 0.5, 0.5]).expect("valid test constraints")
    }

    /// Builds an L2 test cell at `(x, y, z)`.
    fn cell(x: i64, y: i64, z: i64) -> CellPos {
        CellPos::new(Level::new(2).expect("valid test level"), x, y, z)
    }

    #[test]
    fn density_is_deterministic_and_seed_sensitive() {
        let seed = 0x1234_5678_9ABC_DEF0;
        let (x, y, z) = (1.25, -0.5, 3.0);
        assert_eq!(
            density_at(seed, x, y, z).to_bits(),
            density_at(seed, x, y, z).to_bits()
        );
        assert_ne!(
            density_at(seed, x, y, z).to_bits(),
            density_at(seed ^ 0x1, x, y, z).to_bits()
        );
    }

    #[test]
    fn density_stays_in_unit_range() {
        let seed = 777;
        for x in [-256.0, -1.5, 0.0, 0.5, 40.0, 1.0e6] {
            for y in [-3.25, 0.0, 2.0] {
                let density = density_at(seed, x, y, x - y);
                assert!((0.0..=1.0).contains(&density), "out of range: {density}");
            }
        }
        assert_eq!(
            density_at(seed, f64::NAN, 0.0, 0.0).to_bits(),
            0.5f64.to_bits()
        );
        assert_eq!(
            density_at(seed, 0.0, f64::INFINITY, 0.0).to_bits(),
            0.5f64.to_bits()
        );
    }

    #[test]
    fn shared_face_samples_agree_across_neighbors() {
        let seed = 0x1234_5678_9ABC_DEF0;
        // Shared face between cells x=0 and x=1: local +0.5 meets local -0.5.
        for u in [-0.375, -0.125, 0.125, 0.375] {
            for v in [-0.375, -0.125, 0.125, 0.375] {
                let from_left = density_at(seed, 0.0 + 0.5, u, v);
                let from_right = density_at(seed, 1.0 - 0.5, u, v);
                assert!(
                    (from_left - from_right).abs() <= 1e-12,
                    "face mismatch at ({u}, {v}): {from_left} vs {from_right}"
                );
            }
        }
        // Field smoothness: millistep moves change the value only slightly.
        let base = density_at(seed, 1.25, -0.5, 3.0);
        for step in [[1e-3, 0.0, 0.0], [0.0, 1e-3, 0.0], [0.0, 0.0, 1e-3]] {
            let near = density_at(seed, 1.25 + step[0], -0.5 + step[1], 3.0 + step[2]);
            assert!(
                (base - near).abs() <= 0.05,
                "field jumps too fast: {base} vs {near}"
            );
        }
    }

    #[test]
    fn clusters_are_deterministic_and_seed_sensitive() {
        let parent = parent_constraints();
        let place = cell(3, -4, 5);
        assert_eq!(
            clusters_in_cell(99, place, &parent),
            clusters_in_cell(99, place, &parent)
        );
        assert_ne!(
            clusters_in_cell(99, place, &parent),
            clusters_in_cell(100, place, &parent)
        );
        let generator = DensityGenerator::new(place.level, place.x, place.y, place.z);
        assert_eq!(
            generator.generate(99, &parent),
            clusters_in_cell(99, place, &parent)
        );
    }

    #[test]
    fn clusters_honor_parent_and_children_respect_parent() {
        let parent = parent_constraints();
        let out = clusters_in_cell(99, cell(3, -4, 5), &parent);
        assert!(
            (parent.min_count as usize..=parent.max_count as usize).contains(&out.points.len())
        );
        for point in &out.points {
            assert!(point.radius > 0.0, "non-positive radius");
            for (axis, extent) in parent.allowed_extent.iter().enumerate() {
                assert!(
                    point.position[axis].abs() <= *extent,
                    "point outside allowed extent"
                );
            }
        }
        assert_eq!(out.child_constraints.len(), 8);
        for child in &out.child_constraints {
            assert!(respects(child, &parent));
        }
    }

    #[test]
    fn invalid_parent_yields_empty_output() {
        let bad = Constraints {
            density_multiplier: 2.0,
            min_count: 5,
            max_count: 2,
            allowed_extent: [0.5; 3],
        };
        assert!(!bad.is_valid());
        let out = clusters_in_cell(7, cell(0, 0, 0), &bad);
        assert!(out.points.is_empty());
        assert!(out.child_constraints.is_empty());
    }
}
