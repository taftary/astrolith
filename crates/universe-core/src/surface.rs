//! One icosphere surface from planet to building, per level (#458, ADR 0024).
//!
//! The planet is the core icosphere split once (80 triangles). Each surface
//! level L10-L13 reads the same triangles split to its own depth, chosen so
//! a triangle edge is about a quarter of the open cell under the ladder
//! sizes. Everything here is pure `f64` and deterministic (`E-DET-TIERS`).

use crate::coords::Level;
use crate::nest::child_ratio;
use crate::terrain::PLANET_RADIUS_CELL;

/// Icosphere depth per surface level L10..=L13 (ADR 0024).
///
/// L10 is the default single split (Q2). L11-L13 follow the R11 ladder
/// sizes: about 5, 11, and 19 splits put a triangle edge near a quarter of
/// the cell (tested against [`child_ratio`]).
pub const SURFACE_DEPTH: [u8; 4] = [1, 5, 11, 19];

/// Edge of a base icosahedron face over its circumradius.
const BASE_EDGE: f64 = 1.051_462_224_238_267_3;

/// Icosphere depth of the surface at `level`, or `None` outside L10-L13.
#[must_use]
pub fn surface_depth(level: Level) -> Option<u8> {
    let index = usize::from(level.get().checked_sub(10)?);
    SURFACE_DEPTH.get(index).copied()
}

/// Open-cell size at `level` in L10 units (`None` outside L10-L13).
///
/// The product of the true ladder ratios from L10 down (one at L10).
#[must_use]
pub fn cell_span_l10(level: Level) -> Option<f64> {
    if !(10..=13).contains(&level.get()) {
        return None;
    }
    let mut span = 1.0;
    for n in 10..level.get() {
        span *= child_ratio(Level::new(n)?)?;
    }
    Some(span)
}

/// Nominal triangle edge at `depth`, in L10 units.
#[must_use]
pub fn edge_l10(depth: u8) -> f64 {
    BASE_EDGE * PLANET_RADIUS_CELL / 2f64.powi(i32::from(depth))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(n: u8) -> Level {
        Level::new(n).expect("valid level")
    }

    #[test]
    fn planet_defaults_to_one_split() {
        assert_eq!(surface_depth(level(10)), Some(1), "Q2: 80 triangles");
        assert!(surface_depth(level(9)).is_none());
        assert!(surface_depth(level(14)).is_none());
    }

    #[test]
    fn tail_depths_follow_the_ladder_sizes() {
        for n in 11..=13u8 {
            let depth = surface_depth(level(n)).expect("tail depth");
            let span = cell_span_l10(level(n)).expect("tail span");
            let ratio = edge_l10(depth) / span;
            assert!(
                (0.125..=0.5).contains(&ratio),
                "L{n}: edge/cell {ratio} outside [1/8, 1/2]"
            );
        }
        let depths: Vec<u8> = (10..=13u8)
            .filter_map(|n| surface_depth(level(n)))
            .collect();
        assert!(
            depths.windows(2).all(|w| w[0] < w[1]),
            "depth grows per level"
        );
        assert!(
            depths.iter().all(|d| *d <= crate::icosphere::MAX_DEPTH),
            "addressable"
        );
    }
}
