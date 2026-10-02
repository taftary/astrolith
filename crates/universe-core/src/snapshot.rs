//! Canonical text snapshots for `--verify` mode and validator reads.
//!
//! Floats never round-trip identically through every printer, so snapshots
//! fix the format once: points sort in a total order, every float prints
//! with [`SNAPSHOT_PRECISION`] fractional digits, and child constraints
//! follow in octant index order. Regenerating the same `(seed, constraints)`
//! yields byte-identical text; shuffling the input point order does not
//! change it.

use crate::r#gen::Generated;

/// Fractional digits printed per float in snapshots.
pub const SNAPSHOT_PRECISION: usize = 6;

/// Formats one float with [`SNAPSHOT_PRECISION`] fractional digits.
fn fixed(value: f64) -> String {
    format!("{value:.prec$}", prec = SNAPSHOT_PRECISION)
}

/// Renders `generated` as deterministic canonical text.
///
/// Points sort by `(x, y, z, radius)` with [`f64::total_cmp`] — a total
/// order, so `-0.0` and `NaN` sort deterministically too — then print with
/// fixed precision; child constraints follow in octant index order. The
/// first line is always the header `generated points=<n> children=<m>`.
pub fn snapshot_generated(generated: &Generated) -> String {
    let mut points = generated.points.clone();
    points.sort_by(|a, b| {
        a.position[0]
            .total_cmp(&b.position[0])
            .then(a.position[1].total_cmp(&b.position[1]))
            .then(a.position[2].total_cmp(&b.position[2]))
            .then(a.radius.total_cmp(&b.radius))
    });
    let point_count = points.len();
    let child_count = generated.child_constraints.len();
    let mut out = format!("generated points={point_count} children={child_count}\n");
    for point in &points {
        out.push('p');
        for value in [
            point.position[0],
            point.position[1],
            point.position[2],
            point.radius,
        ] {
            out.push(' ');
            out.push_str(&fixed(value));
        }
        out.push('\n');
    }
    for (index, child) in generated.child_constraints.iter().enumerate() {
        out.push_str(&format!(
            "c {index} {} {} {} {} {} {}\n",
            fixed(child.density_multiplier),
            child.min_count,
            child.max_count,
            fixed(child.allowed_extent[0]),
            fixed(child.allowed_extent[1]),
            fixed(child.allowed_extent[2]),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r#gen::{Constraints, Generator, Point, UniformGenerator};

    /// Builds typical mid-ladder constraints for tests.
    fn parent_constraints() -> Constraints {
        Constraints::new(1.0, 2, 64, [0.5, 0.5, 0.5]).expect("valid test constraints")
    }

    #[test]
    fn snapshot_is_deterministic() {
        let generator = UniformGenerator::new(32);
        let parent = parent_constraints();
        let first = snapshot_generated(&generator.generate(99, &parent));
        let second = snapshot_generated(&generator.generate(99, &parent));
        assert_eq!(first, second);
    }

    #[test]
    fn snapshot_sorts_points_canonically() {
        let low = Point {
            position: [-0.25, 0.0, 0.0],
            radius: 0.01,
        };
        let high = Point {
            position: [0.25, 0.0, 0.0],
            radius: 0.01,
        };
        let forward = Generated {
            points: vec![high, low],
            child_constraints: Vec::new(),
        };
        let backward = Generated {
            points: vec![low, high],
            child_constraints: Vec::new(),
        };
        assert_eq!(snapshot_generated(&forward), snapshot_generated(&backward));
        let text = snapshot_generated(&forward);
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some("generated points=2 children=0"));
        assert!(
            lines.next().unwrap_or("").starts_with("p -0.250000"),
            "lowest point did not sort first"
        );
    }

    #[test]
    fn snapshot_uses_fixed_float_precision() {
        let single = Generated {
            points: vec![Point {
                position: [0.5, -0.25, 0.0],
                radius: 0.01,
            }],
            child_constraints: vec![
                Constraints::new(0.5, 0, 32, [0.5, 0.5, 0.5]).expect("valid test constraints"),
            ],
        };
        let text = snapshot_generated(&single);
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some("generated points=1 children=1"));
        assert_eq!(lines.next(), Some("p 0.500000 -0.250000 0.000000 0.010000"));
        assert_eq!(
            lines.next(),
            Some("c 0 0.500000 0 32 0.500000 0.500000 0.500000")
        );
        for line in text.lines().skip(1) {
            for token in line.split(' ') {
                if let Some(dot) = token.find('.') {
                    assert_eq!(
                        token.len() - dot - 1,
                        SNAPSHOT_PRECISION,
                        "token without fixed precision: {token}"
                    );
                }
            }
        }
    }

    #[test]
    fn snapshot_of_empty_is_header_only() {
        let empty = Generated {
            points: Vec::new(),
            child_constraints: Vec::new(),
        };
        assert_eq!(
            snapshot_generated(&empty),
            "generated points=0 children=0\n"
        );
    }

    #[test]
    fn snapshot_distinguishes_content() {
        let generator = UniformGenerator::new(32);
        let parent = parent_constraints();
        assert_ne!(
            snapshot_generated(&generator.generate(1, &parent)),
            snapshot_generated(&generator.generate(2, &parent))
        );
    }
}
