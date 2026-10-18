//! Quadtree LOD per cube face (rebuild step 3, #776).
//!
//! Six roots (one per [`Face`](crate::cube_sphere::Face)), each node a patch
//! of 33 x 33 points (32 x 32 quads, Q1 A) on #775's mapping. Selection is
//! by screen-space error (Q2 A) with a hysteresis band (Q3 A): splits above
//! the threshold, merges only below half of it, so small moves never flip a
//! patch. Neighbours stay within one level (restricted quadtree, cross-face
//! through #775's canonical folding). Depth caps at about 1 m between
//! vertices (Q7 A, Earth 18). Pure and deterministic (`E-DET-TIERS`,
//! `E-CORE-NO-BEVY`); transcendental calls stay in core (`E-TRANSCENDENTAL`).

use crate::cube_sphere::{Face, FacePos, to_sphere};
use crate::planet_frame::PlanetPos;

/// Quads per patch edge (Q1 A: 33 x 33 vertices).
pub const PATCH_QUADS: u32 = 32;

/// Vertices per patch edge.
pub const PATCH_VERTS: u32 = 33;

/// Face edge in metres for `radius_m`: a quarter of the circumference.
#[must_use]
pub fn face_edge_m(radius_m: f64) -> f64 {
    radius_m * std::f64::consts::FRAC_PI_2
}

/// Deepest level giving about 1 m between vertices (Q7 A).
///
/// `floor(log2(face_edge / (32 * 1 m)))`; Earth gives 18. Returns `0` for a
/// non-finite or non-positive radius.
#[must_use]
pub fn max_level_for_radius(radius_m: f64) -> u32 {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return 0;
    }
    let edge = face_edge_m(radius_m);
    if !edge.is_finite() || edge <= 0.0 {
        return 0;
    }
    let per_patch = edge / f64::from(PATCH_QUADS);
    if !per_patch.is_finite() || per_patch <= 0.0 {
        return 0;
    }
    let levels = per_patch.log2().floor();
    if !levels.is_finite() || levels < 0.0 {
        return 0;
    }
    if levels > 24.0 {
        24
    } else {
        // Levels are small (Earth 18); truncation is the floor.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "E-CAST: lod levels are 0..=24, exactly representable"
        )]
        let level = levels as u32;
        level
    }
}

/// One patch: `face` root subdivided `level` times, cell `(x, y)`.
///
/// `level` 0 is the whole face; `x` and `y` run `0..2^level`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PatchKey {
    /// Which cube face.
    pub face: Face,
    /// Subdivision depth.
    pub level: u32,
    /// Cell column.
    pub x: u32,
    /// Cell row.
    pub y: u32,
}

impl PatchKey {
    /// Patch holding `face`, `level`, `x`, `y`, or `None` when out of range.
    #[must_use]
    pub fn new(face: Face, level: u32, x: u32, y: u32) -> Option<PatchKey> {
        let per_edge = if level >= 32 {
            return None;
        } else {
            1u32.checked_shl(level)?
        };
        if x >= per_edge || y >= per_edge {
            return None;
        }
        Some(PatchKey { face, level, x, y })
    }

    /// Whole-face root.
    #[must_use]
    pub const fn root(face: Face) -> PatchKey {
        PatchKey {
            face,
            level: 0,
            x: 0,
            y: 0,
        }
    }

    /// Patches per edge at this key's level.
    #[must_use]
    pub fn per_edge(self) -> Option<u32> {
        1u32.checked_shl(self.level)
    }

    /// Four children, or `None` on overflow.
    #[must_use]
    pub fn children(self) -> Option<[PatchKey; 4]> {
        let next = self.level.checked_add(1)?;
        let (x2, y2) = (self.x.checked_mul(2)?, self.y.checked_mul(2)?);
        Some([
            PatchKey {
                face: self.face,
                level: next,
                x: x2,
                y: y2,
            },
            PatchKey {
                face: self.face,
                level: next,
                x: x2.checked_add(1)?,
                y: y2,
            },
            PatchKey {
                face: self.face,
                level: next,
                x: x2,
                y: y2.checked_add(1)?,
            },
            PatchKey {
                face: self.face,
                level: next,
                x: x2.checked_add(1)?,
                y: y2.checked_add(1)?,
            },
        ])
    }

    /// Parent, or `None` at the roots.
    #[must_use]
    pub fn parent(self) -> Option<PatchKey> {
        if self.level == 0 {
            return None;
        }
        Some(PatchKey {
            face: self.face,
            level: self.level - 1,
            x: self.x / 2,
            y: self.y / 2,
        })
    }

    /// Face discriminant in `Face::all()` order, for ordering.
    #[must_use]
    pub fn face_index(&self) -> usize {
        match self.face {
            Face::PosX => 0,
            Face::NegX => 1,
            Face::PosY => 2,
            Face::NegY => 3,
            Face::PosZ => 4,
            Face::NegZ => 5,
        }
    }
}

impl PartialOrd for PatchKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PatchKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.face_index()
            .cmp(&other.face_index())
            .then(self.level.cmp(&other.level))
            .then(self.x.cmp(&other.x))
            .then(self.y.cmp(&other.y))
    }
}

/// `(u0, u1, v0, v1)` bounds of `key` in face `[-1, 1]` coordinates.
///
/// Returns `None` when the level overflows.
#[must_use]
pub fn patch_uv_range(key: PatchKey) -> Option<(f64, f64, f64, f64)> {
    let per_edge = key.per_edge()?;
    if per_edge == 0 {
        return None;
    }
    let per = f64::from(per_edge);
    let x = f64::from(key.x);
    let y = f64::from(key.y);
    let u0 = -1.0 + 2.0 * x / per;
    let u1 = -1.0 + 2.0 * (x + 1.0) / per;
    let v0 = -1.0 + 2.0 * y / per;
    let v1 = -1.0 + 2.0 * (y + 1.0) / per;
    if !(u0.is_finite() && u1.is_finite() && v0.is_finite() && v1.is_finite()) {
        return None;
    }
    Some((u0, u1, v0, v1))
}

/// Centre `(u, v)` of `key`.
#[must_use]
pub fn patch_centre_uv(key: PatchKey) -> Option<(f64, f64)> {
    let (u0, u1, v0, v1) = patch_uv_range(key)?;
    Some(((u0 + u1) / 2.0, (v0 + v1) / 2.0))
}

/// Whether `face`'s grid is right-handed seen from outside: `+u` then `+v`
/// turn counter-clockwise around the outward direction (#778 T3, Q5 A).
///
/// On such a face the grid order `a, c, b` winds inward, so the mesh
/// reverses it. Exact sign from the face centre's mapping.
#[must_use]
pub fn face_grid_right_handed(face: Face) -> bool {
    let at = |u: f64, v: f64| FacePos::new(face, u, v).and_then(to_sphere);
    let (Some(o), Some(pu), Some(pv)) = (at(0.0, 0.0), at(0.1, 0.0), at(0.0, 0.1)) else {
        return false;
    };
    let du = [pu[0] - o[0], pu[1] - o[1], pu[2] - o[2]];
    let dv = [pv[0] - o[0], pv[1] - o[1], pv[2] - o[2]];
    let n = [
        du[1] * dv[2] - du[2] * dv[1],
        du[2] * dv[0] - du[0] * dv[2],
        du[0] * dv[1] - du[1] * dv[0],
    ];
    n[0] * o[0] + n[1] * o[1] + n[2] * o[2] > 0.0
}

/// Centre direction of `key` on the unit sphere.
#[must_use]
pub fn patch_centre_dir(key: PatchKey) -> Option<[f64; 3]> {
    let (u, v) = patch_centre_uv(key)?;
    let pos = FacePos::new(key.face, u, v)?;
    to_sphere(pos)
}

/// Angular edge size of `key` in radians (face spans `pi/2`).
#[must_use]
pub fn patch_edge_angle(key: PatchKey) -> Option<f64> {
    let per_edge = key.per_edge()?;
    if per_edge == 0 {
        return None;
    }
    let angle = std::f64::consts::FRAC_PI_2 / f64::from(per_edge);
    if angle.is_finite() && angle > 0.0 {
        Some(angle)
    } else {
        None
    }
}

/// Geometric error of `key`: chord sag in metres on a sphere of `radius_m`.
///
/// `R * (1 - cos(theta / 2))` with `theta` the edge angle; exact for the
/// sphere, the distance the flat triangles sit inside the true surface.
/// This is the whole-patch sag (kept for reference); selection uses the
/// drawn quad's sag below.
#[must_use]
pub fn patch_sag_m(key: PatchKey, radius_m: f64) -> Option<f64> {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return None;
    }
    let theta = patch_edge_angle(key)?;
    let half = theta / 2.0;
    let sag = radius_m * (1.0 - half.cos());
    if sag.is_finite() && sag >= 0.0 {
        Some(sag)
    } else {
        None
    }
}

/// Geometric error of one drawn quad: the patch is drawn as 32 x 32 quads
/// that follow the sphere, so the drawn error is the sag of one quad,
/// about 1000 times smaller than the whole-patch sag above.
#[must_use]
pub fn patch_quad_sag_m(key: PatchKey, radius_m: f64) -> Option<f64> {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return None;
    }
    let theta = patch_edge_angle(key)?;
    let quad = theta / f64::from(PATCH_QUADS);
    if !quad.is_finite() || quad <= 0.0 {
        return None;
    }
    let half = quad / 2.0;
    let sag = radius_m * (1.0 - half.cos());
    if sag.is_finite() && sag >= 0.0 {
        Some(sag)
    } else {
        None
    }
}

fn length(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Distance in metres from `camera` to the centre of `key`.
#[must_use]
pub fn patch_distance_m(key: PatchKey, camera: PlanetPos, radius_m: f64) -> Option<f64> {
    let dir = patch_centre_dir(key)?;
    let centre = [dir[0] * radius_m, dir[1] * radius_m, dir[2] * radius_m];
    let d = [
        centre[0] - camera.0[0],
        centre[1] - camera.0[1],
        centre[2] - camera.0[2],
    ];
    let dist = length(d);
    if dist.is_finite() && dist >= 0.0 {
        Some(dist)
    } else {
        None
    }
}

/// Bounding radius of `key` in metres: the farthest corner from the centre.
///
/// The patch's four corners come from the same canonical grid as the build,
/// so the radius is exact for the sphere. Used for the nearest-point distance
/// below.
#[must_use]
pub fn patch_bounding_radius_m(key: PatchKey, radius_m: f64) -> Option<f64> {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return None;
    }
    let (u0, u1, v0, v1) = patch_uv_range(key)?;
    let centre_dir = patch_centre_dir(key)?;
    let mut worst = 0.0f64;
    for (u, v) in [(u0, v0), (u1, v0), (u0, v1), (u1, v1)] {
        let pos = FacePos::new(key.face, u, v)?;
        let dir = to_sphere(pos)?;
        let chord = length([
            dir[0] - centre_dir[0],
            dir[1] - centre_dir[1],
            dir[2] - centre_dir[2],
        ]) * radius_m;
        if !chord.is_finite() {
            return None;
        }
        if chord > worst {
            worst = chord;
        }
    }
    Some(worst)
}

/// Bounding radius of `key` in metres with relief (#799).
///
/// Sphere bound from [`patch_bounding_radius_m`] plus the patch's height
/// range, so high relief grows the bounding sphere and shortens the nearest
/// distance. Returns `None` for a bad radius or range.
#[must_use]
pub fn patch_bounding_radius_with_relief_m(
    key: PatchKey,
    radius_m: f64,
    height_range_m: f64,
) -> Option<f64> {
    let sphere = patch_bounding_radius_m(key, radius_m)?;
    let range = if height_range_m.is_finite() && height_range_m > 0.0 {
        height_range_m
    } else {
        0.0
    };
    let grown = sphere + range;
    if grown.is_finite() && grown >= 0.0 {
        Some(grown)
    } else {
        None
    }
}
/// Distance in metres from `camera` to the nearest point of `key`.
///
/// Centre distance minus the bounding radius (the bounding sphere), clamped to
/// a hair above zero; inside the sphere reads as a large error (split). This
/// is the plan's "nearest point" rule: a close camera sees a large error even
/// when the centre is a patch-width away.
#[must_use]
pub fn patch_nearest_distance_m(key: PatchKey, camera: PlanetPos, radius_m: f64) -> Option<f64> {
    let centre = patch_distance_m(key, camera, radius_m)?;
    let bound = patch_bounding_radius_m(key, radius_m)?;
    if !centre.is_finite() || !bound.is_finite() {
        return None;
    }
    let nearest = centre - bound;
    if !nearest.is_finite() {
        return None;
    }
    if nearest < 1e-6 {
        return Some(1e-6);
    }
    Some(nearest)
}

/// Screen-space error of `key` in pixels (Q2 A).
///
/// The drawn quad's sag divided by the distance to the patch's nearest point
/// (its bounding sphere), times `focal_px` from `precision` (1600x900, 70 deg
/// vertical). Near-zero distances read as a large error (split).
#[must_use]
pub fn patch_screen_error_px(
    key: PatchKey,
    camera: PlanetPos,
    radius_m: f64,
    focal: f64,
) -> Option<f64> {
    let sag = patch_quad_sag_m(key, radius_m)?;
    let dist = patch_nearest_distance_m(key, camera, radius_m)?;
    if !focal.is_finite() || focal <= 0.0 {
        return None;
    }
    if !dist.is_finite() {
        return None;
    }
    if dist < 1e-6 {
        return Some(f64::INFINITY);
    }
    let px = sag / dist * focal;
    if px.is_finite() && px >= 0.0 {
        Some(px)
    } else {
        None
    }
}

/// Distance to the nearest point of `key` with relief (#799).
///
/// Like [`patch_nearest_distance_m`], but the bounding sphere grows by the
/// patch's height range, so high relief shortens the distance.
#[must_use]
pub fn patch_nearest_distance_with_relief_m(
    key: PatchKey,
    camera: PlanetPos,
    radius_m: f64,
    height_range_m: f64,
) -> Option<f64> {
    let centre = patch_distance_m(key, camera, radius_m)?;
    let bound = patch_bounding_radius_with_relief_m(key, radius_m, height_range_m)?;
    if !centre.is_finite() || !bound.is_finite() {
        return None;
    }
    let nearest = centre - bound;
    if !nearest.is_finite() {
        return None;
    }
    if nearest < 1e-6 {
        return Some(1e-6);
    }
    Some(nearest)
}

/// Screen-space error of `key` in pixels with relief (#799, AC5).
///
/// `max(quad_sag, relief_error)` over the relief-grown nearest distance,
/// times `focal`. Mountains (large `relief_error_m`) split earlier than flat
/// plains at the same distance; with `(0, 0)` it equals
/// [`patch_screen_error_px`].
#[must_use]
pub fn patch_screen_error_with_relief_px(
    key: PatchKey,
    camera: PlanetPos,
    radius_m: f64,
    focal: f64,
    relief_error_m: f64,
    height_range_m: f64,
) -> Option<f64> {
    let sag = patch_quad_sag_m(key, radius_m)?;
    let relief = if relief_error_m.is_finite() && relief_error_m > 0.0 {
        relief_error_m
    } else {
        0.0
    };
    let err_m = sag.max(relief);
    let dist = patch_nearest_distance_with_relief_m(key, camera, radius_m, height_range_m)?;
    if !focal.is_finite() || focal <= 0.0 {
        return None;
    }
    if !dist.is_finite() {
        return None;
    }
    if dist < 1e-6 {
        return Some(f64::INFINITY);
    }
    let px = err_m / dist * focal;
    if px.is_finite() && px >= 0.0 {
        Some(px)
    } else {
        None
    }
}
