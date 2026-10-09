//! Observer-frame geometry: orientation, planet radius, horizon (#394).
//!
//! Split from [`crate::nest`] under `E-FILE-SIZE` (#399): the marker path
//! owns the frame state, this module owns the pure geometry it maps
//! through. Everything here is pure and headless.

use crate::coords::{Level, ParentUnits};
use crate::nest::{OPEN_ANGLE, PREVIEW_ANGLE, SHELL_FLOOR, child_ratio};
use crate::terrain::PLANET_RADIUS_CELL;

/// Identity up: child `+Y` maps to parent `+Y` (no rotation).
pub const IDENTITY_UP: [f64; 3] = [0.0, 1.0, 0.0];

/// Orthonormal basis with `up` as its Y axis.
///
/// Returns `(x, y, z)` with `y` the normalized `up` (or identity when `up`
/// is zero or non-finite) and `x`, `z` perpendicular unit vectors, so
/// `x × y = z`. Deterministic: the same `up` always yields the same frame.
#[must_use]
pub fn basis_from_up(up: [f64; 3]) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let length = (up[0] * up[0] + up[1] * up[1] + up[2] * up[2]).sqrt();
    let y = if length.is_finite() && length > 0.0 {
        [up[0] / length, up[1] / length, up[2] / length]
    } else {
        IDENTITY_UP
    };
    let reference = if y[1].abs() < 0.9 {
        IDENTITY_UP
    } else {
        [0.0, 0.0, 1.0]
    };
    let mut x = [
        y[1] * reference[2] - y[2] * reference[1],
        y[2] * reference[0] - y[0] * reference[2],
        y[0] * reference[1] - y[1] * reference[0],
    ];
    let x_length = (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt();
    if x_length.is_finite() && x_length > 0.0 {
        x = [x[0] / x_length, x[1] / x_length, x[2] / x_length];
    } else {
        x = [1.0, 0.0, 0.0];
    }
    let z = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    (x, y, z)
}

/// Child-local vector in parent units through the `up` frame.
#[must_use]
pub fn child_to_parent(up: [f64; 3], child: [f64; 3]) -> [f64; 3] {
    let (x, y, z) = basis_from_up(up);
    [
        x[0] * child[0] + y[0] * child[1] + z[0] * child[2],
        x[1] * child[0] + y[1] * child[1] + z[1] * child[2],
        x[2] * child[0] + y[2] * child[1] + z[2] * child[2],
    ]
}

/// Parent-units vector in child units through the `up` frame.
#[must_use]
pub fn parent_to_child(up: [f64; 3], parent: [f64; 3]) -> [f64; 3] {
    let (x, y, z) = basis_from_up(up);
    [
        x[0] * parent[0] + x[1] * parent[1] + x[2] * parent[2],
        y[0] * parent[0] + y[1] * parent[1] + y[2] * parent[2],
        z[0] * parent[0] + z[1] * parent[1] + z[2] * parent[2],
    ]
}

/// Planet radius in cell units at a surface level (#394).
///
/// The L10 body radius ([`PLANET_RADIUS_CELL`]) carried down by the true
/// ratios: L10 is the body itself, L11 divides by the L10 ratio, and so on
/// through L14. Returns `None` outside L10-L14. Pure: the same ladder
/// always yields the same radii (about 9.0 at L11, 450 at L12, 1.4e5 at
/// L13 for the R11 anchors).
#[must_use]
pub fn planet_radius_cells(level: Level) -> Option<f64> {
    match level.get() {
        10 => Some(PLANET_RADIUS_CELL),
        11..=14 => {
            let mut radius = PLANET_RADIUS_CELL;
            for n in 10..level.get() {
                let rung = Level::new(n)?;
                radius /= child_ratio(rung)?;
            }
            if radius.is_finite() && radius > 0.0 {
                Some(radius)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Horizon distance on a sphere of `radius` seen from `height` above it.
///
/// `sqrt(2Rh + h^2)` in the same units as the inputs: the ground circle
/// where the curved surface leaves the view. Returns `0.0` for
/// non-finite or non-positive inputs (#394).
#[must_use]
pub fn horizon_distance(radius: f64, height: f64) -> f64 {
    if !radius.is_finite() || !height.is_finite() || radius <= 0.0 || height <= 0.0 {
        return 0.0;
    }
    (2.0 * radius * height + height * height).sqrt()
}

/// Euclidean length of `v`.
pub(crate) fn length(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Angular radius (radians) of a sphere of `radius` at `distance`.
///
/// Returns `pi/2` when the camera is inside the sphere.
#[must_use]
pub fn angular_radius(radius: f64, distance: f64) -> f64 {
    if distance <= radius {
        std::f64::consts::FRAC_PI_2
    } else {
        (radius / distance).asin()
    }
}

/// World position (parent-cell units) of a child drawn inside a marker.
///
/// `marker` is the marker position, `ratio` the child/parent size ratio,
/// `child_local` the child's position in child-cell units (passed straight
/// from cell storage; no arithmetic is done on it outside this function).
/// This is the exact inverse of the offset map in
/// [`crate::nest::MarkerPath::open`], so a child drawn before opening sits
/// where the open cell's marker appears after opening.
#[must_use]
pub fn child_world_position(marker: ParentUnits, ratio: f64, child_local: [f64; 3]) -> ParentUnits {
    child_world_position_oriented(marker, ratio, child_local, IDENTITY_UP)
}

/// World position of a child drawn inside an oriented marker (#394).
///
/// Identity `up` behaves exactly like [`child_world_position`]; a patch
/// normal `up` turns the child frame so the region's ground lands tangent
/// to the planet. Exact inverse of
/// [`crate::nest::MarkerPath::open_oriented`].
#[must_use]
pub fn child_world_position_oriented(
    marker: ParentUnits,
    ratio: f64,
    child_local: [f64; 3],
    up: [f64; 3],
) -> ParentUnits {
    let scaled = [
        child_local[0] * ratio,
        child_local[1] * ratio,
        child_local[2] * ratio,
    ];
    let turned = child_to_parent(up, scaled);
    ParentUnits([
        marker.0[0] + turned[0],
        marker.0[1] + turned[1],
        marker.0[2] + turned[2],
    ])
}

/// Smoothstep of `x` between `edge0` and `edge1`, clamped.
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Brightness of a marker's shell (its dot) at `angular_radius`.
///
/// `1.0` up to [`PREVIEW_ANGLE`], easing down to [`SHELL_FLOOR`] at
/// [`OPEN_ANGLE`] while the children resolve, then easing to `0.0` as the
/// camera passes inside the sphere (`pi/2`). Continuous and non-increasing,
/// so neither opening nor closing produces a brightness step.
#[must_use]
pub fn shell_brightness(angular_radius: f64) -> f64 {
    if !angular_radius.is_finite() {
        return 0.0;
    }
    let resolve = smoothstep(PREVIEW_ANGLE, OPEN_ANGLE, angular_radius);
    let enter = smoothstep(OPEN_ANGLE, std::f64::consts::FRAC_PI_2, angular_radius);
    (1.0 - (1.0 - SHELL_FLOOR) * resolve) * (1.0 - enter)
}

/// Brightness of previewed children at the parent marker's `angular_radius`.
///
/// Complement of the shell's resolve phase: `0.0` at [`PREVIEW_ANGLE`],
/// `1.0` from [`OPEN_ANGLE`] on, so children are fully lit by the time the
/// marker opens and the open cell draws them at full brightness.
#[must_use]
pub fn children_brightness(angular_radius: f64) -> f64 {
    if !angular_radius.is_finite() {
        return 0.0;
    }
    smoothstep(PREVIEW_ANGLE, OPEN_ANGLE, angular_radius)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::Level;

    fn level(n: u8) -> Level {
        Level::new(n).expect("ladder level")
    }

    #[test]
    fn identity_up_leaves_the_frame_put() {
        let (x, y, z) = basis_from_up(IDENTITY_UP);
        assert_eq!(x, [1.0, 0.0, 0.0]);
        assert_eq!(y, [0.0, 1.0, 0.0]);
        assert_eq!(z, [0.0, 0.0, 1.0]);
        let local = [0.2, -0.3, 0.1];
        assert_eq!(child_to_parent(IDENTITY_UP, local), local);
        assert_eq!(parent_to_child(IDENTITY_UP, local), local);
    }

    #[test]
    fn planet_radius_grows_down_the_tail_by_true_ratios() {
        use crate::terrain::PLANET_RADIUS_CELL;
        let l10 = planet_radius_cells(level(10)).expect("L10 body");
        assert_eq!(l10, PLANET_RADIUS_CELL);
        let l11 = planet_radius_cells(level(11)).expect("L11 ground");
        let l12 = planet_radius_cells(level(12)).expect("L12 ground");
        let l13 = planet_radius_cells(level(13)).expect("L13 ground");
        assert!((l11 - 9.0).abs() < 0.5, "L11 radius near 9: {l11}");
        assert!((l12 - 450.0).abs() < 20.0, "L12 radius near 450: {l12}");
        assert!(l13 > 1.0e5 && l13 < 2.0e5, "L13 radius near 1.4e5: {l13}");
        let l14 = planet_radius_cells(level(14)).expect("L14 ground");
        assert!(l14 > l13, "the room sits deepest in the planet frame");
        assert!(planet_radius_cells(level(9)).is_none());
    }

    #[test]
    fn horizon_distance_follows_sqrt_two_rh() {
        assert_eq!(horizon_distance(9.0, 0.0), 0.0);
        assert_eq!(horizon_distance(f64::NAN, 0.1), 0.0);
        let horizon = horizon_distance(9.0, 0.1);
        let expected: f64 = (2.0f64 * 9.0f64 * 0.1f64 + 0.1f64 * 0.1f64).sqrt();
        assert!((horizon - expected).abs() < 1e-12);
        assert!(horizon > 0.0 && horizon < 9.0);
    }

    #[test]
    fn angular_radius_is_monotonic_and_saturates() {
        assert_eq!(angular_radius(1.0, 0.5), std::f64::consts::FRAC_PI_2);
        let far = angular_radius(0.5, 10.0);
        let near = angular_radius(0.5, 2.0);
        assert!(far < near && far > 0.0);
    }

    #[test]
    fn brightness_curves_are_continuous_and_monotonic() {
        use crate::nest::{CLOSE_ANGLE, OPEN_ANGLE, PREVIEW_ANGLE, SHELL_FLOOR};
        assert_eq!(shell_brightness(0.0), 1.0);
        assert_eq!(shell_brightness(PREVIEW_ANGLE), 1.0);
        assert!((shell_brightness(OPEN_ANGLE) - SHELL_FLOOR).abs() < 1e-12);
        assert!(shell_brightness(std::f64::consts::FRAC_PI_2) < 1e-12);
        assert_eq!(children_brightness(PREVIEW_ANGLE), 0.0);
        assert_eq!(children_brightness(OPEN_ANGLE), 1.0);
        let eps = 1e-6;
        for edge in [PREVIEW_ANGLE, OPEN_ANGLE, CLOSE_ANGLE] {
            assert!((shell_brightness(edge - eps) - shell_brightness(edge + eps)).abs() < 1e-4);
            assert!(
                (children_brightness(edge - eps) - children_brightness(edge + eps)).abs() < 1e-4
            );
        }
        let mut prev_shell = f64::INFINITY;
        let mut prev_children = f64::NEG_INFINITY;
        let mut angle = 0.0;
        while angle <= std::f64::consts::FRAC_PI_2 {
            let shell = shell_brightness(angle);
            let children = children_brightness(angle);
            assert!(shell <= prev_shell + 1e-12 && (0.0..=1.0).contains(&shell));
            assert!(children >= prev_children - 1e-12 && (0.0..=1.0).contains(&children));
            prev_shell = shell;
            prev_children = children;
            angle += 0.001;
        }
        assert_eq!(shell_brightness(f64::NAN), 0.0);
    }
}
