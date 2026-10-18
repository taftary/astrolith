//! Cube-sphere mapping for the L10 planet (rebuild step 2, #775).
//!
//! Six cube faces bent onto the sphere with a tangent-adjusted mapping
//! (Q1 A): face coordinates are bent by a tangent before normalising, which
//! runs backwards exactly and keeps the largest-to-smallest cell size ratio
//! near 1.4. The `+Y` face is the north pole. Positions on the sphere are
//! unit directions; scaling by the planet radius gives
//! [`PlanetPos`](crate::planet_frame::PlanetPos) (#774).
//!
//! A point on a face edge or corner is built by one shared function
//! ([`grid_point`](crate::cube_sphere::grid_point) folds to a canonical
//! face, Q5 A), so both faces get exactly the same bits; the same holds for
//! directions and the test-only height
//! ([`test_height`](crate::cube_sphere::test_height)).
//! [`spacing`](crate::cube_sphere::spacing) walks the grid neighbours and
//! reports the min, max, ratio, and centre/edge/corner samples (Q6 A).
//! Pure and deterministic (`E-DET-TIERS`, `E-CORE-NO-BEVY`);
//! transcendental calls live in core only (`E-TRANSCENDENTAL`).

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

/// Canonical `(face, i, j)` for a grid point (Q5 A).
///
/// Every edge and corner maps to one canonical face before any floating-point
/// work, so both faces of a seam call the same arithmetic on the same inputs
/// and get the same bits. Interior points map to themselves. Edges along `Z`
/// (`x = +-1, y = +-1`) and along `Y` (`x = +-1, z = +-1`) canonicalise to
/// the `X` face; edges along `X` (`y = +-1, z = +-1`) canonicalise to the `Y`
/// face; all eight corners canonicalise to the `X` face.
fn canonical(face: Face, i: u32, j: u32, n: u32) -> (Face, u32, u32) {
    let i_at = i == 0 || i == n;
    let j_at = j == 0 || j == n;
    if !(i_at || j_at) {
        return (face, i, j);
    }
    let corner = i_at && j_at;
    match face {
        Face::PosX | Face::NegX => (face, i, j),
        Face::PosY => {
            if corner || i_at {
                // `x = sign(i)`, `y = 1`, `z = sign(j)` -> X face.
                let cface = if i == n { Face::PosX } else { Face::NegX };
                (cface, n, j)
            } else {
                // `y = 1, z = +-1` edge, already canonical (`Y < Z`).
                (face, i, j)
            }
        }
        Face::NegY => {
            if corner || i_at {
                let cface = if i == n { Face::PosX } else { Face::NegX };
                (cface, 0, j)
            } else {
                (face, i, j)
            }
        }
        Face::PosZ => {
            if corner {
                // `x = sign(i)`, `y = sign(j)`, `z = 1` -> X face.
                let cface = if i == n { Face::PosX } else { Face::NegX };
                let ci = if j == n { n } else { 0 };
                (cface, ci, n)
            } else if i_at {
                // `x = +-1, z = 1` edge -> X face: `y` (`j`) becomes `i`.
                let cface = if i == n { Face::PosX } else { Face::NegX };
                (cface, j, n)
            } else {
                // `y = +-1, z = 1` edge -> Y face: `x` (`i`) stays `i`.
                debug_assert!(j_at);
                // `PosZ (i, j=n)` (y=1) -> `PosY (i, n)`; `PosZ (i, j=0)`
                // (y=-1) -> `NegY (i, n)` (z=1).
                if j == n {
                    (Face::PosY, i, n)
                } else {
                    (Face::NegY, i, n)
                }
            }
        }
        Face::NegZ => {
            if corner {
                let cface = if i == n { Face::PosX } else { Face::NegX };
                let ci = if j == n { n } else { 0 };
                (cface, ci, 0)
            } else if i_at {
                let cface = if i == n { Face::PosX } else { Face::NegX };
                (cface, j, 0)
            } else {
                debug_assert!(j_at);
                if j == n {
                    (Face::PosY, i, 0)
                } else {
                    (Face::NegY, i, 0)
                }
            }
        }
    }
}

/// Grid point `(i, j)` of `n` divisions on `face` as a unit direction.
///
/// `i` and `j` run `0..=n`; `n` is the quad count per edge. Edge and corner
/// indices fold to one canonical face before computing (Q5 A), so both faces
/// of a seam get exactly the same bits. Returns `None` for `n == 0` or
/// out-of-range indices.
#[must_use]
pub fn grid_point(face: Face, i: u32, j: u32, n: u32) -> Option<[f64; 3]> {
    if n == 0 || i > n || j > n {
        return None;
    }
    let (cface, ci, cj) = canonical(face, i, j, n);
    let u = grid_uv(ci, n)?;
    let v = grid_uv(cj, n)?;
    let pos = FacePos::new(cface, u, v)?;
    to_sphere(pos)
}

/// Test-only height pattern (Q3 A).
///
/// A pure function of the direction, so heights match across seams whenever
/// the directions do. Non-finite input reads as `0.0`.
#[must_use]
pub fn test_height(dir: [f64; 3]) -> f64 {
    if dir.iter().any(|c| !c.is_finite()) {
        return 0.0;
    }
    dir[0] * 1.1 + dir[1] * 1.7 + dir[2] * 2.3
}

/// Vertex spacing over the whole sphere at `n` divisions (Q6 A).
///
/// Chord distances between grid neighbours on the unit sphere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacing {
    /// Smallest neighbour distance.
    pub min: f64,
    /// Largest neighbour distance.
    pub max: f64,
    /// `max / min`.
    pub ratio: f64,
    /// Neighbour distance at a face centre.
    pub centre: f64,
    /// Neighbour distance at a face-edge middle.
    pub edge: f64,
    /// Neighbour distance at a corner along its edge.
    pub corner: f64,
}

/// Vertex spacing at `n` divisions per edge.
///
/// Walks every neighbour pair over all six faces; returns `None` for
/// `n == 0`. Pure and deterministic.
#[must_use]
pub fn spacing(n: u32) -> Option<Spacing> {
    if n == 0 {
        return None;
    }
    let mut min = f64::INFINITY;
    let mut max = 0.0f64;
    for face in Face::all() {
        for i in 0..=n {
            for j in 0..=n {
                let here = grid_point(face, i, j, n)?;
                if i < n {
                    let next = grid_point(face, i + 1, j, n)?;
                    let d = length([next[0] - here[0], next[1] - here[1], next[2] - here[2]]);
                    if d.is_finite() {
                        if d < min {
                            min = d;
                        }
                        if d > max {
                            max = d;
                        }
                    }
                }
                if j < n {
                    let next = grid_point(face, i, j + 1, n)?;
                    let d = length([next[0] - here[0], next[1] - here[1], next[2] - here[2]]);
                    if d.is_finite() {
                        if d < min {
                            min = d;
                        }
                        if d > max {
                            max = d;
                        }
                    }
                }
            }
        }
    }
    if !(min.is_finite() && max.is_finite() && min > 0.0 && max > 0.0) {
        return None;
    }
    // Samples: centre of `+Y`, middle of a `+X` edge, corner along its edge.
    let centre = neighbour_distance(Face::PosY, n / 2, n / 2, n / 2 + 1, n / 2, n)?;
    let edge = neighbour_distance(Face::PosX, n / 2, n, n / 2 + 1, n, n)?;
    let corner = neighbour_distance(Face::PosX, n - 1, n, n, n, n)?;
    Some(Spacing {
        min,
        max,
        ratio: max / min,
        centre,
        edge,
        corner,
    })
}

/// Chord distance between two grid points on one face.
fn neighbour_distance(face: Face, i0: u32, j0: u32, i1: u32, j1: u32, n: u32) -> Option<f64> {
    let a = grid_point(face, i0, j0, n)?;
    let b = grid_point(face, i1, j1, n)?;
    let d = length([b[0] - a[0], b[1] - a[1], b[2] - a[2]]);
    if d.is_finite() { Some(d) } else { None }
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

    #[test]
    fn seams_match_bit_for_bit() {
        // AC2 / C2 (T2, Q5 A): zero mismatches along every face seam and the
        // 8 corners, for positions, normals (the direction itself on a
        // sphere), and test heights, compared bit for bit.
        const N: u32 = 64;
        let mut mismatches = 0u32;
        let mut checked = 0u32;
        fn bits(v: [f64; 3]) -> [u64; 3] {
            [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()]
        }
        let mut check_pair = |a: Option<[f64; 3]>, b: Option<[f64; 3]>| {
            let (pa, pb) = (a.expect("seam point"), b.expect("seam point"));
            checked += 1;
            // Positions bit for bit.
            if bits(pa) != bits(pb) {
                mismatches += 1;
            }
            // Normals: the direction itself on a sphere.
            if bits(pa) != bits(pb) {
                mismatches += 1;
            }
            // Test heights bit for bit.
            if test_height(pa).to_bits() != test_height(pb).to_bits() {
                mismatches += 1;
            }
        };
        // Edges along `Z` (`x = +-1, y = +-1`): `X` face vs `Y` face.
        for k in 0..=N {
            // `x = 1, y = 1`: `PosX (n, k)` vs `PosY (n, k)`.
            check_pair(
                grid_point(Face::PosX, N, k, N),
                grid_point(Face::PosY, N, k, N),
            );
            // `x = 1, y = -1`: `PosX (0, k)` vs `NegY (n, k)`.
            check_pair(
                grid_point(Face::PosX, 0, k, N),
                grid_point(Face::NegY, N, k, N),
            );
            // `x = -1, y = 1`: `NegX (n, k)` vs `PosY (0, k)`.
            check_pair(
                grid_point(Face::NegX, N, k, N),
                grid_point(Face::PosY, 0, k, N),
            );
            // `x = -1, y = -1`: `NegX (0, k)` vs `NegY (0, k)`.
            check_pair(
                grid_point(Face::NegX, 0, k, N),
                grid_point(Face::NegY, 0, k, N),
            );
            // Edges along `Y` (`x = +-1, z = +-1`): `X` face vs `Z` face.
            // `x = 1, z = 1`: `PosX (k, n)` vs `PosZ (n, k)`.
            check_pair(
                grid_point(Face::PosX, k, N, N),
                grid_point(Face::PosZ, N, k, N),
            );
            // `x = 1, z = -1`: `PosX (k, 0)` vs `NegZ (n, k)`.
            check_pair(
                grid_point(Face::PosX, k, 0, N),
                grid_point(Face::NegZ, N, k, N),
            );
            // `x = -1, z = 1`: `NegX (k, n)` vs `PosZ (0, k)`.
            check_pair(
                grid_point(Face::NegX, k, N, N),
                grid_point(Face::PosZ, 0, k, N),
            );
            // `x = -1, z = -1`: `NegX (k, 0)` vs `NegZ (0, k)`.
            check_pair(
                grid_point(Face::NegX, k, 0, N),
                grid_point(Face::NegZ, 0, k, N),
            );
            // Edges along `X` (`y = +-1, z = +-1`): `Y` face vs `Z` face.
            // `y = 1, z = 1`: `PosY (k, n)` vs `PosZ (k, n)`.
            check_pair(
                grid_point(Face::PosY, k, N, N),
                grid_point(Face::PosZ, k, N, N),
            );
            // `y = 1, z = -1`: `PosY (k, 0)` vs `NegZ (k, n)`.
            check_pair(
                grid_point(Face::PosY, k, 0, N),
                grid_point(Face::NegZ, k, N, N),
            );
            // `y = -1, z = 1`: `NegY (k, n)` vs `PosZ (k, 0)`.
            check_pair(
                grid_point(Face::NegY, k, N, N),
                grid_point(Face::PosZ, k, 0, N),
            );
            // `y = -1, z = -1`: `NegY (k, 0)` vs `NegZ (k, 0)`.
            check_pair(
                grid_point(Face::NegY, k, 0, N),
                grid_point(Face::NegZ, k, 0, N),
            );
        }
        // The 8 corners across their three faces.
        let corners = [
            [
                grid_point(Face::PosX, N, N, N),
                grid_point(Face::PosY, N, N, N),
                grid_point(Face::PosZ, N, N, N),
            ],
            [
                grid_point(Face::PosX, N, 0, N),
                grid_point(Face::PosY, N, 0, N),
                grid_point(Face::NegZ, N, N, N),
            ],
            [
                grid_point(Face::PosX, 0, N, N),
                grid_point(Face::NegY, N, N, N),
                grid_point(Face::PosZ, N, 0, N),
            ],
            [
                grid_point(Face::PosX, 0, 0, N),
                grid_point(Face::NegY, N, 0, N),
                grid_point(Face::NegZ, N, 0, N),
            ],
            [
                grid_point(Face::NegX, N, N, N),
                grid_point(Face::PosY, 0, N, N),
                grid_point(Face::PosZ, 0, N, N),
            ],
            [
                grid_point(Face::NegX, N, 0, N),
                grid_point(Face::PosY, 0, 0, N),
                grid_point(Face::NegZ, 0, N, N),
            ],
            [
                grid_point(Face::NegX, 0, N, N),
                grid_point(Face::NegY, 0, N, N),
                grid_point(Face::PosZ, 0, 0, N),
            ],
            [
                grid_point(Face::NegX, 0, 0, N),
                grid_point(Face::NegY, 0, 0, N),
                grid_point(Face::NegZ, 0, 0, N),
            ],
        ];
        for triple in corners {
            check_pair(triple[0], triple[1]);
            check_pair(triple[1], triple[2]);
        }
        assert_eq!(
            mismatches, 0,
            "seam mismatches {mismatches} over {checked} pairs"
        );
        // Review item 6: independent check that folding lands on the right
        // place, not only on the same bits. Every seam index folded must sit
        // within 1e-12 of the unfolded `to_sphere` of its own face position
        // (1-ulp `tan(pi/4)` differences allowed; transposition bugs give
        // order-1 errors).
        let mut independent = 0u32;
        let mut worst = 0.0f64;
        let mut check_folded = |face: Face, i: u32, j: u32| {
            let folded = grid_point(face, i, j, N).expect("folded");
            let u = grid_uv(i, N).expect("u");
            let v = grid_uv(j, N).expect("v");
            let pos = FacePos::new(face, u, v).expect("face pos");
            let unfolded = to_sphere(pos).expect("unfolded");
            let d = length([
                folded[0] - unfolded[0],
                folded[1] - unfolded[1],
                folded[2] - unfolded[2],
            ]);
            if d > worst {
                worst = d;
            }
            independent += 1;
            assert!(
                d < 1e-12,
                "folded {folded:?} vs unfolded {unfolded:?} at {face:?} ({i},{j}): {d}"
            );
        };
        for k in 0..=N {
            for (face, i, j) in [
                (Face::PosX, N, k),
                (Face::PosY, N, k),
                (Face::PosX, 0, k),
                (Face::NegY, N, k),
                (Face::NegX, N, k),
                (Face::PosY, 0, k),
                (Face::NegX, 0, k),
                (Face::NegY, 0, k),
                (Face::PosX, k, N),
                (Face::PosZ, N, k),
                (Face::PosX, k, 0),
                (Face::NegZ, N, k),
                (Face::NegX, k, N),
                (Face::PosZ, 0, k),
                (Face::NegX, k, 0),
                (Face::NegZ, 0, k),
                (Face::PosY, k, N),
                (Face::PosZ, k, N),
                (Face::PosY, k, 0),
                (Face::NegZ, k, N),
                (Face::NegY, k, N),
                (Face::PosZ, k, 0),
                (Face::NegY, k, 0),
                (Face::NegZ, k, 0),
            ] {
                check_folded(face, i, j);
            }
        }
        assert!(independent > 0, "independent folding checks ran");
    }

    #[test]
    fn spacing_stays_under_one_and_a_half() {
        // AC3 / C3 (T2, Q6 A): largest/smallest under 1.5 over the whole
        // sphere at the draw resolution, with centre/edge/corner reported.
        let s = spacing(64).expect("spacing at 64");
        assert!(
            s.ratio < 1.5,
            "spacing ratio {} over 1.5: min {} max {} centre {} edge {} corner {}",
            s.ratio,
            s.min,
            s.max,
            s.centre,
            s.edge,
            s.corner
        );
        assert!(s.min > 0.0 && s.max > 0.0);
        assert!(s.centre > 0.0 && s.edge > 0.0 && s.corner > 0.0);
        // Deterministic: same divisions give byte-identical spacing.
        assert_eq!(spacing(64), spacing(64), "same grid, same spacing");
    }
}
