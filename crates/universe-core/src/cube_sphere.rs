//! Cube-sphere mapping for the L10 planet (rebuild step 2, #775).
//!
//! Six cube faces bent onto the sphere with a tangent-adjusted mapping
//! (Q1 A): face coordinates are bent by a tangent before normalising, which
//! runs backwards exactly and keeps the largest-to-smallest cell size ratio
//! near 1.4. The `+Y` face is the north pole. Positions on the sphere are
//! unit directions; scaling by the planet radius gives [`PlanetPos`] (#774).
//!
//! Seams are bit-identical later (T2 canonical folding); this module owns the
//! forward ([`to_sphere`]) and backward ([`from_sphere`]) steps plus the grid
//! helper ([`grid_point`]). Pure and deterministic (`E-DET-TIERS`,
//! `E-CORE-NO-BEVY`); transcendental calls live in core only
//! (`E-TRANSCENDENTAL`).

use crate::planet_frame::PlanetPos;
use std::f64::consts::FRAC_PI_4;

/// One cube face; the `+Y` face is the north pole (Q1 A).
///
/// Two-variant-per-axis enum, never a bare `bool` (`E-ENUM-BOOL`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    /// Normal `+X`.
    PosX,
    /// Normal `-X`.
    NegX,
    /// Normal `+Y` (north pole).
    PosY,
    /// Normal `-Y` (south pole).
    NegY,
    /// Normal `+Z`.
    PosZ,
    /// Normal `-Z`.
    NegZ,
}

impl Face {
    /// Every face, in discriminant order.
    #[must_use]
    pub const fn all() -> [Face; 6] {
        [
            Face::PosX,
            Face::NegX,
            Face::PosY,
            Face::NegY,
            Face::PosZ,
            Face::NegZ,
        ]
    }

    /// Outward unit normal of the face in planet axes.
    #[must_use]
    pub const fn normal(self) -> [f64; 3] {
        match self {
            Face::PosX => [1.0, 0.0, 0.0],
            Face::NegX => [-1.0, 0.0, 0.0],
            Face::PosY => [0.0, 1.0, 0.0],
            Face::NegY => [0.0, -1.0, 0.0],
            Face::PosZ => [0.0, 0.0, 1.0],
            Face::NegZ => [0.0, 0.0, -1.0],
        }
    }
}

/// A place on one cube face: `u` and `v` in `[-1, 1]`.
///
/// `u` runs along the face's first tangent axis, `v` along the second:
/// `PosX`/`NegX` map `(u, v)` to `(Y, Z)`, `PosY`/`NegY` to `(X, Z)`,
/// `PosZ`/`NegZ` to `(X, Y)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FacePos {
    /// Which face.
    pub face: Face,
    /// First tangent coordinate in `[-1, 1]`.
    pub u: f64,
    /// Second tangent coordinate in `[-1, 1]`.
    pub v: f64,
}

impl FacePos {
    /// Builds a face position, returning `None` for out-of-range or
    /// non-finite coordinates.
    #[must_use]
    pub fn new(face: Face, u: f64, v: f64) -> Option<FacePos> {
        if !u.is_finite() || !v.is_finite() {
            return None;
        }
        if !(-1.0..=1.0).contains(&u) || !(-1.0..=1.0).contains(&v) {
            return None;
        }
        Some(FacePos { face, u, v })
    }
}

/// Tangent of the scaled coordinate: `tan(u * pi/4)`.
///
/// At `u = +-1` this is `tan(+-pi/4) = +-1`, so face edges land on the cube
/// edges before normalising.
fn tan_scaled(t: f64) -> f64 {
    (t * FRAC_PI_4).tan()
}

/// Cube point before normalising for `pos`.
///
/// Layout per [`FacePos`]: the dominant axis is `+-1`, the other two are the
/// tangent values.
fn cube_point(pos: FacePos) -> [f64; 3] {
    let tx = tan_scaled(pos.u);
    let ty = tan_scaled(pos.v);
    match pos.face {
        Face::PosX => [1.0, tx, ty],
        Face::NegX => [-1.0, tx, ty],
        Face::PosY => [tx, 1.0, ty],
        Face::NegY => [tx, -1.0, ty],
        Face::PosZ => [tx, ty, 1.0],
        Face::NegZ => [tx, ty, -1.0],
    }
}

fn length(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn normalize(v: [f64; 3]) -> Option<[f64; 3]> {
    let len = length(v);
    if !len.is_finite() || len <= 0.0 {
        return None;
    }
    Some([v[0] / len, v[1] / len, v[2] / len])
}

/// Bends one cube face onto the unit sphere (Q1 A).
///
/// Returns `None` for out-of-range or non-finite inputs.
#[must_use]
pub fn to_sphere(pos: FacePos) -> Option<[f64; 3]> {
    if !pos.u.is_finite() || !pos.v.is_finite() {
        return None;
    }
    if !(-1.0..=1.0).contains(&pos.u) || !(-1.0..=1.0).contains(&pos.v) {
        return None;
    }
    normalize(cube_point(pos))
}

/// Exact backward step: a direction to its face and face place (Q1 A).
///
/// The dominant axis picks the face; `atan` inverts the tangent exactly, so
/// `from_sphere(to_sphere(p))` returns `p` up to floating-point round-off.
/// Returns `None` for the zero vector or non-finite input.
#[must_use]
pub fn from_sphere(dir: [f64; 3]) -> Option<FacePos> {
    if dir.iter().any(|c| !c.is_finite()) {
        return None;
    }
    let ax = dir[0].abs();
    let ay = dir[1].abs();
    let az = dir[2].abs();
    if !(ax > 0.0 || ay > 0.0 || az > 0.0) {
        return None;
    }
    // Dominant axis picks the face; ties fall to X, then Y, then Z, so edge
    // points map deterministically.
    if ax >= ay && ax >= az {
        let face = if dir[0] > 0.0 { Face::PosX } else { Face::NegX };
        let denom = ax;
        if !(denom.is_finite() && denom > 0.0) {
            return None;
        }
        // `tx = y / |x|`, `ty = z / |x|`: recovers the tangent values.
        let tx = dir[1] / denom;
        let ty = dir[2] / denom;
        if !tx.is_finite() || !ty.is_finite() {
            return None;
        }
        let u = tx.atan() / FRAC_PI_4;
        let v = ty.atan() / FRAC_PI_4;
        FacePos::new(face, u, v)
    } else if ay >= az {
        let face = if dir[1] > 0.0 { Face::PosY } else { Face::NegY };
        let denom = ay;
        if !(denom.is_finite() && denom > 0.0) {
            return None;
        }
        let tx = dir[0] / denom;
        let ty = dir[2] / denom;
        if !tx.is_finite() || !ty.is_finite() {
            return None;
        }
        let u = tx.atan() / FRAC_PI_4;
        let v = ty.atan() / FRAC_PI_4;
        FacePos::new(face, u, v)
    } else {
        let face = if dir[2] > 0.0 { Face::PosZ } else { Face::NegZ };
        let denom = az;
        if !(denom.is_finite() && denom > 0.0) {
            return None;
        }
        let tx = dir[0] / denom;
        let ty = dir[1] / denom;
        if !tx.is_finite() || !ty.is_finite() {
            return None;
        }
        let u = tx.atan() / FRAC_PI_4;
        let v = ty.atan() / FRAC_PI_4;
        FacePos::new(face, u, v)
    }
}

/// Face-grid coordinate in `[-1, 1]` for index `i` of `n` divisions.
fn grid_uv(i: u32, n: u32) -> Option<f64> {
    if n == 0 || i > n {
        return None;
    }
    let fi = f64::from(i);
    let fn_ = f64::from(n);
    Some(-1.0 + 2.0 * fi / fn_)
}

/// Grid point `(i, j)` of `n` divisions on `face` as a unit direction.
///
/// `i` and `j` run `0..=n`; `n` is the quad count per edge (T2 adds the
/// canonical edge folding for bit-identical seams). Returns `None` for
/// `n == 0` or out-of-range indices.
#[must_use]
pub fn grid_point(face: Face, i: u32, j: u32, n: u32) -> Option<[f64; 3]> {
    let u = grid_uv(i, n)?;
    let v = grid_uv(j, n)?;
    let pos = FacePos::new(face, u, v)?;
    to_sphere(pos)
}

/// Unit direction at `radius_m` metres as a planet-frame position.
#[must_use]
pub fn to_planet(dir: [f64; 3], radius_m: f64) -> Option<PlanetPos> {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return None;
    }
    if dir.iter().any(|c| !c.is_finite()) {
        return None;
    }
    let len = length(dir);
    if !(len.is_finite() && len > 0.0) {
        return None;
    }
    // Normalise first so near-unit inputs still land on the sphere.
    let unit = [dir[0] / len, dir[1] / len, dir[2] / len];
    Some(PlanetPos([
        unit[0] * radius_m,
        unit[1] * radius_m,
        unit[2] * radius_m,
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_frame::{EARTH_RADIUS_M, PlanetFrame};

    fn earth_radius_m() -> f64 {
        EARTH_RADIUS_M
    }

    #[test]
    fn cube_sphere_maps_and_inverts() {
        // AC1 / C1 (T1): every face centre, edge, corner, plus 1,000 seeded
        // random points round-trip within 1 µm on Earth.
        let radius_m = earth_radius_m();
        let mut cases: Vec<[f64; 3]> = Vec::new();
        for face in Face::all() {
            for (u, v) in [
                (0.0, 0.0),
                (1.0, 0.0),
                (-1.0, 0.0),
                (0.0, 1.0),
                (0.0, -1.0),
                (1.0, 1.0),
                (1.0, -1.0),
                (-1.0, 1.0),
                (-1.0, -1.0),
            ] {
                let pos = FacePos::new(face, u, v).expect("case in range");
                let dir = to_sphere(pos).expect("maps");
                cases.push(dir);
            }
        }
        // 1,000 deterministic random directions (fixed seed, hermetic).
        let mut rng = crate::seed::Rng::new(0xC0BE_7757_51E_ED00);
        for _ in 0..1000 {
            let dir = [
                rng.next_f64() * 2.0 - 1.0,
                rng.next_f64() * 2.0 - 1.0,
                rng.next_f64() * 2.0 - 1.0,
            ];
            let len = length(dir);
            if !(len.is_finite() && len > 1e-9) {
                continue;
            }
            let unit = [dir[0] / len, dir[1] / len, dir[2] / len];
            cases.push(unit);
        }
        assert!(cases.len() >= 1000, "random cases ran");
        let mut worst_um = 0.0f64;
        for dir in cases {
            let back = from_sphere(dir).expect("inverts");
            let dir2 = to_sphere(back).expect("re-maps");
            let err = length([dir2[0] - dir[0], dir2[1] - dir[1], dir2[2] - dir[2]]) * radius_m;
            let err_um = err * 1e6;
            if err_um > worst_um {
                worst_um = err_um;
            }
            assert!(
                err < 1e-6,
                "round trip {err} m ({err_um} µm) for {dir:?} via {back:?}"
            );
        }
        // Deterministic: same inputs give byte-identical directions.
        let pos = FacePos::new(Face::PosY, 0.25, -0.5).expect("pos");
        assert_eq!(to_sphere(pos), to_sphere(pos), "same input, same bits");
        let frame = PlanetFrame::for_radius_earth(1.0);
        let planet = to_planet([0.0, 1.0, 0.0], frame.radius_m()).expect("planet");
        assert!((planet.radius() - frame.radius_m()).abs() < 1e-6);
    }
}
