//! Canonical text snapshots for `--verify` mode and validator reads.
//!
//! Floats never round-trip identically through every printer, so snapshots
//! fix the format once: points sort in a total order, every float prints
//! with [`SNAPSHOT_PRECISION`](crate::snapshot::SNAPSHOT_PRECISION) fractional digits, and child constraints
//! follow in octant index order. Regenerating the same `(seed, constraints)`
//! yields byte-identical text; shuffling the input point order does not
//! change it.

use crate::r#gen::Generated;
use std::cmp::Ordering;

/// Fractional digits printed per float in snapshots.
pub const SNAPSHOT_PRECISION: usize = 6;

/// Formats one float with [`SNAPSHOT_PRECISION`] fractional digits.
fn fixed(value: f64) -> String {
    format!("{value:.prec$}", prec = SNAPSHOT_PRECISION)
}

/// Tag rank for the canonical order (spiral, elliptical, irregular).
fn galaxy_tag(kind: crate::r#gen::GalaxyKind) -> u8 {
    match kind {
        crate::r#gen::GalaxyKind::Spiral => 0,
        crate::r#gen::GalaxyKind::Elliptical => 1,
        crate::r#gen::GalaxyKind::Irregular => 2,
    }
}

/// Canonical order over galaxy shape tokens (total: `total_cmp` floats).
///
/// `None` sorts before any shape; shapes compare by tag, orientation,
/// bar flag, then far-view flag. Points that already differed on
/// `(x, y, z, radius, kind)` never reorder on this key.
fn cmp_galaxy(
    a: &Option<crate::r#gen::GalaxyInfo>,
    b: &Option<crate::r#gen::GalaxyInfo>,
) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(x), Some(y)) => galaxy_tag(x.kind)
            .cmp(&galaxy_tag(y.kind))
            .then(x.orientation[0].total_cmp(&y.orientation[0]))
            .then(x.orientation[1].total_cmp(&y.orientation[1]))
            .then(x.orientation[2].total_cmp(&y.orientation[2]))
            .then(x.barred.cmp(&y.barred))
            .then(x.far_view.cmp(&y.far_view)),
    }
}

/// Renders `generated` as deterministic canonical text.
///
/// Points sort by `(x, y, z, radius, kind, galaxy)` with [`f64::total_cmp`] — a total
/// order, so `-0.0` and `NaN` sort deterministically too — then print with
/// fixed precision; portal lines start with `p`, population lines with `o`;
/// L4 galaxy points append their shape tokens (`S`/`E`/`I`, orientation at
/// fixed precision, `bar`/`nobar`, `far`/`near`); child constraints follow in
/// octant index order. The
/// first line is always the header `generated points=<n> children=<m>`.
#[must_use]
pub fn snapshot_generated(generated: &Generated) -> String {
    use crate::r#gen::MarkerKind;
    let mut points = generated.points.clone();
    points.sort_by(|a, b| {
        a.position[0]
            .total_cmp(&b.position[0])
            .then(a.position[1].total_cmp(&b.position[1]))
            .then(a.position[2].total_cmp(&b.position[2]))
            .then(a.radius.total_cmp(&b.radius))
            .then(a.kind.cmp(&b.kind))
            .then(cmp_galaxy(&a.galaxy, &b.galaxy))
    });
    let point_count = points.len();
    let child_count = generated.child_constraints.len();
    let mut out = format!("generated points={point_count} children={child_count}\n");
    for point in &points {
        out.push(if point.kind == MarkerKind::Portal {
            'p'
        } else {
            'o'
        });
        for value in [
            point.position[0],
            point.position[1],
            point.position[2],
            point.radius,
        ] {
            out.push(' ');
            out.push_str(&fixed(value));
        }
        if let Some(galaxy) = point.galaxy {
            out.push(' ');
            out.push(match galaxy.kind {
                crate::r#gen::GalaxyKind::Spiral => 'S',
                crate::r#gen::GalaxyKind::Elliptical => 'E',
                crate::r#gen::GalaxyKind::Irregular => 'I',
            });
            for value in galaxy.orientation {
                out.push(' ');
                out.push_str(&fixed(value));
            }
            out.push_str(if galaxy.barred { " bar" } else { " nobar" });
            out.push_str(if galaxy.far_view { " far" } else { " near" });
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
        use crate::r#gen::MarkerKind;
        let low = Point {
            position: [-0.25, 0.0, 0.0],
            radius: 0.01,
            kind: MarkerKind::Portal,
            galaxy: None,
        };
        let high = Point {
            position: [0.25, 0.0, 0.0],
            radius: 0.01,
            kind: MarkerKind::Population,
            galaxy: None,
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
        use crate::r#gen::MarkerKind;
        let single = Generated {
            points: vec![Point {
                position: [0.5, -0.25, 0.0],
                radius: 0.01,
                kind: MarkerKind::Portal,
                galaxy: None,
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

    #[test]
    fn snapshot_marks_populations_and_breaks_kind_ties() {
        use crate::r#gen::MarkerKind;
        let portal = Point {
            position: [0.0, 0.0, 0.0],
            radius: 0.01,
            kind: MarkerKind::Portal,
            galaxy: None,
        };
        let population = Point {
            position: [0.0, 0.0, 0.0],
            radius: 0.01,
            kind: MarkerKind::Population,
            galaxy: None,
        };
        let text = snapshot_generated(&Generated {
            points: vec![population, portal],
            child_constraints: Vec::new(),
        });
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some("generated points=2 children=0"));
        assert_eq!(lines.next(), Some("p 0.000000 0.000000 0.000000 0.010000"));
        assert_eq!(lines.next(), Some("o 0.000000 0.000000 0.000000 0.010000"));
    }

    #[test]
    fn snapshot_prints_galaxy_shape_tokens() {
        use crate::r#gen::{GalaxyInfo, GalaxyKind, MarkerKind};
        let spiral = Point {
            position: [0.1, -0.2, 0.3],
            radius: 0.05,
            kind: MarkerKind::Portal,
            galaxy: Some(GalaxyInfo {
                kind: GalaxyKind::Spiral,
                orientation: [0.0, 0.6, 0.8],
                barred: true,
                far_view: false,
            }),
        };
        let elliptical = Point {
            position: [0.1, -0.2, 0.3],
            radius: 0.05,
            kind: MarkerKind::Portal,
            galaxy: Some(GalaxyInfo {
                kind: GalaxyKind::Elliptical,
                orientation: [0.0, 0.0, 1.0],
                barred: false,
                far_view: true,
            }),
        };
        let text = snapshot_generated(&Generated {
            points: vec![elliptical, spiral],
            child_constraints: Vec::new(),
        });
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some("generated points=2 children=0"));
        assert_eq!(
            lines.next(),
            Some("p 0.100000 -0.200000 0.300000 0.050000 S 0.000000 0.600000 0.800000 bar near")
        );
        assert_eq!(
            lines.next(),
            Some("p 0.100000 -0.200000 0.300000 0.050000 E 0.000000 0.000000 1.000000 nobar far")
        );
        // Galaxy data never changes the sort: ties keep generator order.
        let swapped = snapshot_generated(&Generated {
            points: vec![spiral, elliptical],
            child_constraints: Vec::new(),
        });
        assert_eq!(text, swapped);
    }
}
