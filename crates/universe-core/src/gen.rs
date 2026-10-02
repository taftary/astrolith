//! Pure procedural generation contracts.
//!
//! The ladder rule `(contents, child_constraints) = G(seed, parent)`
//! (`docs/universe/ladder.md`) becomes the [`Generator`] trait: a pure
//! function of a `u64` seed plus the parent [`Constraints`], returning
//! [`Generated`] content. [`respects`] checks that a child budget fits inside
//! its parent budget. [`UniformGenerator`] is the reference implementation
//! used by the headless checks; content levels (M3+) provide their own.

use crate::coords::HALF_BOUND;
use crate::seed::Rng;

/// Budget passed from a parent cell to its children (`K` in the notion).
///
/// Plain data: a density multiplier plus allowed count and extent ranges.
/// Units are per-cell (positions live in `[-HALF_BOUND, HALF_BOUND)`), so the
/// same struct describes every level without rescaling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraints {
    /// Fraction of the baseline content to emit, in `[0.0, 1.0]`.
    pub density_multiplier: f64,
    /// Fewest points a generation of this cell may emit.
    pub min_count: u32,
    /// Most points a generation of this cell may emit.
    pub max_count: u32,
    /// Half-extent of the allowed region per axis, each in `(0.0, 0.5]`.
    pub allowed_extent: [f64; 3],
}

impl Constraints {
    /// Builds constraints, returning `None` when any field is out of range
    /// (density outside `[0.0, 1.0]`, `min_count > max_count`, an extent
    /// outside `(0.0, 0.5]`, or any non-finite float).
    pub fn new(
        density_multiplier: f64,
        min_count: u32,
        max_count: u32,
        allowed_extent: [f64; 3],
    ) -> Option<Constraints> {
        let candidate = Constraints {
            density_multiplier,
            min_count,
            max_count,
            allowed_extent,
        };
        if candidate.is_valid() {
            Some(candidate)
        } else {
            None
        }
    }

    /// Returns `true` when every field is in its documented range.
    pub fn is_valid(&self) -> bool {
        self.density_multiplier.is_finite()
            && (0.0..=1.0).contains(&self.density_multiplier)
            && self.min_count <= self.max_count
            && self
                .allowed_extent
                .iter()
                .all(|extent| extent.is_finite() && *extent > 0.0 && *extent <= HALF_BOUND)
    }
}

/// One generated indicator point: a position plus a marker size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// Cell-local position in `[-HALF_BOUND, HALF_BOUND)` per axis.
    pub position: [f64; 3],
    /// Marker radius in cell units; always positive.
    pub radius: f64,
}

/// Output of generating one cell: indicator points plus child budgets.
///
/// `child_constraints[i]` is the budget for octant `i` (bit `4*x + 2*y + z`
/// of the octant `[u8; 3]`); [`respects`] must hold for each against the
/// parent constraints the cell was generated with.
#[derive(Clone, Debug, PartialEq)]
pub struct Generated {
    /// Indicator points (grids, vectors, points) inside this cell.
    pub points: Vec<Point>,
    /// One budget per child octant, index `4*x + 2*y + z`.
    pub child_constraints: Vec<Constraints>,
}

/// Pure generator contract `G(seed, parent) -> Generated`.
///
/// Implementations must be pure functions of their arguments: no global or
/// interior-mutable state, no I/O, no clock reads. The same `(seed, parent)`
/// must yield byte-identical output on every call and every platform (see
/// [`crate::verify::check_determinism`]). When `parent` is invalid, return an
/// empty [`Generated`] rather than panicking.
pub trait Generator {
    /// Generates the cell contents for `seed` under the `parent` budget.
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated;
}

/// Returns `true` when `child` fits inside the budget `parent` allows.
///
/// The rule: both sides valid, the child density does not exceed the parent
/// density, and the child count ceiling does not exceed the parent ceiling.
/// (Extents are per-cell units, so they are range-checked by validity rather
/// than compared across levels.)
pub fn respects(child: &Constraints, parent: &Constraints) -> bool {
    child.is_valid()
        && parent.is_valid()
        && child.density_multiplier <= parent.density_multiplier
        && child.max_count <= parent.max_count
}

/// Reference generator scattering uniform indicator points.
///
/// Emits `round(density * base_count)` points clamped to `[min, max]`, with
/// positions uniform in `[-extent, extent]` per axis drawn from an [`Rng`]
/// seeded by the cell seed. Each of the 8 child octants receives half the
/// parent density and half the count ceiling, so [`respects`] holds for every
/// child against the parent the cell was generated with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UniformGenerator {
    /// Baseline point count at full density.
    pub base_count: u32,
}

impl UniformGenerator {
    /// Creates a uniform generator emitting up to `base_count` points.
    pub const fn new(base_count: u32) -> UniformGenerator {
        UniformGenerator { base_count }
    }
}

impl Generator for UniformGenerator {
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        if !parent.is_valid() {
            return Generated {
                points: Vec::new(),
                child_constraints: Vec::new(),
            };
        }
        let wanted = (f64::from(self.base_count) * parent.density_multiplier).round() as u32;
        let count = wanted.clamp(parent.min_count, parent.max_count);
        let mut rng = Rng::new(seed);
        let mut points = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let mut position = [0.0; 3];
            for (axis, extent) in parent.allowed_extent.iter().enumerate() {
                position[axis] = (rng.next_f64() * 2.0 - 1.0) * extent;
            }
            points.push(Point {
                position,
                radius: 0.01,
            });
        }
        let child_density = parent.density_multiplier / 2.0;
        let child_max = parent.max_count / 2;
        let child_min = parent.min_count.min(child_max);
        // SAFETY: halving keeps every field in range when the parent is valid.
        let child = Constraints {
            density_multiplier: child_density,
            min_count: child_min,
            max_count: child_max,
            allowed_extent: parent.allowed_extent,
        };
        Generated {
            points,
            child_constraints: vec![child; 8],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds typical mid-ladder constraints for tests.
    fn parent_constraints() -> Constraints {
        Constraints::new(1.0, 2, 64, [0.5, 0.5, 0.5]).expect("valid test constraints")
    }

    #[test]
    fn same_seed_generates_identical_output() {
        let generator = UniformGenerator::new(32);
        let parent = parent_constraints();
        assert_eq!(
            generator.generate(7, &parent),
            generator.generate(7, &parent)
        );
    }

    #[test]
    fn generation_order_does_not_matter() {
        let generator = UniformGenerator::new(32);
        let parent = parent_constraints();
        let seeds = [1u64, 2, 3, 99, 1024];
        let forward: Vec<_> = seeds
            .iter()
            .map(|s| generator.generate(*s, &parent))
            .collect();
        let backward: Vec<_> = seeds
            .iter()
            .rev()
            .map(|s| generator.generate(*s, &parent))
            .collect();
        let mut backward_sorted = backward.clone();
        backward_sorted.reverse();
        assert_eq!(forward, backward_sorted);
        // Distinct seeds give distinct content.
        assert_ne!(forward[0], forward[1]);
    }

    #[test]
    fn respects_accepts_tighter_and_rejects_looser_children() {
        let parent = parent_constraints();
        let tighter = Constraints::new(0.5, 0, 32, [0.25; 3]).expect("valid");
        assert!(respects(&tighter, &parent));
        // Denser than the parent: rejected.
        let denser = Constraints {
            density_multiplier: 1.5,
            ..tighter
        };
        assert!(!respects(&denser, &parent));
        // Higher ceiling than the parent: rejected (bypasses `new` on purpose).
        let hungrier = Constraints {
            density_multiplier: 0.5,
            min_count: 0,
            max_count: parent.max_count + 1,
            allowed_extent: [0.25; 3],
        };
        assert!(!respects(&hungrier, &parent));
        assert!(!respects(&parent, &tighter));
    }

    #[test]
    fn uniform_output_honors_parent_ranges() {
        let generator = UniformGenerator::new(32);
        let parent = parent_constraints();
        let out = generator.generate(11, &parent);
        assert!(
            (parent.min_count as usize..=parent.max_count as usize).contains(&out.points.len())
        );
        for point in &out.points {
            assert!(point.radius > 0.0);
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
        let generator = UniformGenerator::new(32);
        let bad = Constraints {
            density_multiplier: 2.0,
            min_count: 5,
            max_count: 2,
            allowed_extent: [0.5; 3],
        };
        assert!(!bad.is_valid());
        assert_eq!(generator.generate(3, &bad).points.len(), 0);
        assert_eq!(Constraints::new(2.0, 0, 1, [0.5; 3]), None);
        assert_eq!(Constraints::new(0.5, 9, 1, [0.5; 3]), None);
        assert_eq!(Constraints::new(0.5, 0, 1, [0.0; 3]), None);
    }
}
