//! Visibility for the LOD selection (rebuild step 5, #778 Q3 A, Q4 A).
//!
//! A [`View`](crate::lod_cull::View) is the camera in the planet frame: position, forward, up and
//! the vertical field of view with the image aspect. Each patch gets a
//! conservative bounding sphere ([`patch_bounds`](crate::lod_cull::patch_bounds)) around its cap between
//! the lowest and highest relief, widened inward by the chord sag so the
//! flat triangles, the skirts' tops and the morph's parent shape (#777) all
//! stay inside it.
//!
//! - **Frustum:** the sphere is tested against the four side planes and the
//!   camera plane; there is no far plane (Q2 A, Q6 A).
//! - **Horizon:** the occluder is the planet sphere at its lowest relief
//!   radius. A patch sphere fully inside the camera's shadow cone of that
//!   sphere and fully beyond its horizon plane is hidden. The patch sphere
//!   already reaches its highest relief, so mountain tops beyond the
//!   horizon stay (Q4 A).
//!
//! Pure `f64`, no allocation, deterministic (`E-DET-TIERS`,
//! `E-CORE-NO-BEVY`, `E-HOT-NOALLOC`).

use crate::cube_sphere::{FacePos, to_sphere};
use crate::lod::{PatchKey, patch_centre_dir, patch_uv_range};
use crate::planet_frame::PlanetPos;

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let len = length(a);
    if !len.is_finite() || len <= 0.0 {
        return None;
    }
    Some([a[0] / len, a[1] / len, a[2] / len])
}

/// The camera in the planet frame, for culling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    camera: PlanetPos,
    forward: [f64; 3],
    up: [f64; 3],
    right: [f64; 3],
    tan_half_x: f64,
    tan_half_y: f64,
}

impl View {
    /// A view at `camera` looking along `forward` with `up` roughly up,
    /// `fov_y_rad` the vertical field of view and `aspect` width / height.
    ///
    /// `None` for a non-finite input, a zero `forward`, an `up` parallel to
    /// `forward`, or a field of view outside `(0, pi)`.
    #[must_use]
    pub fn new(
        camera: PlanetPos,
        forward: [f64; 3],
        up: [f64; 3],
        fov_y_rad: f64,
        aspect: f64,
    ) -> Option<View> {
        if camera.0.iter().any(|c| !c.is_finite()) {
            return None;
        }
        if !fov_y_rad.is_finite() || fov_y_rad <= 0.0 || fov_y_rad >= std::f64::consts::PI {
            return None;
        }
        if !aspect.is_finite() || aspect <= 0.0 {
            return None;
        }
        let forward = unit(forward)?;
        let right = unit(cross(forward, up))?;
        let up = cross(right, forward);
        let tan_half_y = (fov_y_rad / 2.0).tan();
        let tan_half_x = tan_half_y * aspect;
        if !tan_half_x.is_finite() || !tan_half_y.is_finite() {
            return None;
        }
        Some(View {
            camera,
            forward,
            up,
            right,
            tan_half_x,
            tan_half_y,
        })
    }

    /// Whether `other` is this view to within `eps` in every direction
    /// component and field-of-view tangent, at the same camera (#918: a
    /// steady view need not run the selection again).
    #[must_use]
    pub fn near_same(&self, other: &View, eps: f64) -> bool {
        let close =
            |a: [f64; 3], b: [f64; 3]| a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() <= eps);
        self.camera == other.camera
            && close(self.forward, other.forward)
            && close(self.up, other.up)
            && (self.tan_half_x - other.tan_half_x).abs() <= eps
            && (self.tan_half_y - other.tan_half_y).abs() <= eps
    }

    /// The camera position.
    #[must_use]
    pub fn camera(&self) -> PlanetPos {
        self.camera
    }

    /// Whether `point` is inside the view (in front, within the four side
    /// planes); the exact test the sphere test bounds.
    #[must_use]
    pub fn sees_point(&self, point: [f64; 3]) -> bool {
        let rel = [
            point[0] - self.camera.0[0],
            point[1] - self.camera.0[1],
            point[2] - self.camera.0[2],
        ];
        let z = dot(rel, self.forward);
        if !z.is_finite() || z <= 0.0 {
            return false;
        }
        let x = dot(rel, self.right);
        let y = dot(rel, self.up);
        x.abs() <= z * self.tan_half_x && y.abs() <= z * self.tan_half_y
    }

    /// Whether a sphere touches the view: not fully behind the camera plane
    /// nor fully outside one side plane (no far plane, Q2 A).
    #[must_use]
    pub fn sees_sphere(&self, bounds: Bounds) -> bool {
        let rel = [
            bounds.centre[0] - self.camera.0[0],
            bounds.centre[1] - self.camera.0[1],
            bounds.centre[2] - self.camera.0[2],
        ];
        let r = bounds.radius;
        let z = dot(rel, self.forward);
        if !z.is_finite() || !r.is_finite() {
            return true;
        }
        if z < -r {
            return false;
        }
        let x = dot(rel, self.right);
        let y = dot(rel, self.up);
        // Signed distance to a side plane through the camera: the plane
        // normal is (±1, -tan) normalised in the (x or y, z) plane.
        let side = |offset: f64, tan: f64| (offset - z * tan) / (1.0 + tan * tan).sqrt();
        side(x, self.tan_half_x) <= r
            && side(-x, self.tan_half_x) <= r
            && side(y, self.tan_half_y) <= r
            && side(-y, self.tan_half_y) <= r
    }
}

/// A bounding sphere in the planet frame (metres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// Centre.
    pub centre: [f64; 3],
    /// Radius.
    pub radius: f64,
}

/// Largest angle from `key`'s centre direction to its corners and edge
/// midpoints, plus 1% (radians).
fn patch_half_angle(key: PatchKey, centre: [f64; 3]) -> Option<f64> {
    let (u0, u1, v0, v1) = patch_uv_range(key)?;
    let um = (u0 + u1) / 2.0;
    let vm = (v0 + v1) / 2.0;
    let mut worst = 0.0f64;
    for (u, v) in [
        (u0, v0),
        (u1, v0),
        (u0, v1),
        (u1, v1),
        (um, v0),
        (um, v1),
        (u0, vm),
        (u1, vm),
    ] {
        let dir = to_sphere(FacePos::new(key.face, u, v)?)?;
        let angle = dot(dir, centre).clamp(-1.0, 1.0).acos();
        worst = worst.max(angle);
    }
    let angle = worst * 1.01;
    if angle.is_finite() && angle > 0.0 && angle < std::f64::consts::FRAC_PI_2 {
        Some(angle)
    } else {
        None
    }
}

/// Conservative bounding sphere of `key` on a planet of `radius_m`, with
/// relief heights in `[min_h_m, max_h_m]`.
///
/// Every surface point of the patch lies within its half angle `theta` of
/// the centre direction, at radii from `radius + min_h` to `radius + max_h`.
/// The convex hull of that cap (the flat triangles, the parent shape and
/// the chord sag) lies in the truncated cone `inner <= a <= outer`,
/// `p <= a * tan(theta)`, with `a` along the centre direction, `p` away from
/// it, `inner = (radius + min_h) * cos(theta)` and `outer = radius + max_h`.
/// The sphere is centred on the axis halfway along and reaches the cone's
/// farthest corner. `None` for bad inputs.
#[must_use]
pub fn patch_bounds(key: PatchKey, radius_m: f64, min_h_m: f64, max_h_m: f64) -> Option<Bounds> {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return None;
    }
    if !min_h_m.is_finite() || !max_h_m.is_finite() || min_h_m > max_h_m {
        return None;
    }
    let dir = patch_centre_dir(key)?;
    let theta = patch_half_angle(key, dir)?;
    let tan_t = theta.tan();
    let outer = radius_m + max_h_m;
    let inner = (radius_m + min_h_m) * theta.cos();
    if inner <= 0.0 || outer <= inner || !tan_t.is_finite() {
        return None;
    }
    let mid = (inner + outer) / 2.0;
    let mut radius = 0.0f64;
    for (a, p) in [
        (inner, 0.0),
        (outer, 0.0),
        (inner, inner * tan_t),
        (outer, outer * tan_t),
    ] {
        let along = a - mid;
        radius = radius.max((along * along + p * p).sqrt());
    }
    let bounds = Bounds {
        centre: [dir[0] * mid, dir[1] * mid, dir[2] * mid],
        radius,
    };
    if bounds.centre.iter().all(|c| c.is_finite()) && radius.is_finite() {
        Some(bounds)
    } else {
        None
    }
}

/// Whether `bounds` shows above the horizon of an occluding sphere of
/// `occluder_m` (the lowest relief radius) seen from `camera`.
///
/// Hidden only when the whole sphere is beyond the horizon plane and inside
/// the shadow cone; anything else counts as visible (conservative).
#[must_use]
pub fn above_horizon(camera: PlanetPos, bounds: Bounds, occluder_m: f64) -> bool {
    let d = length(camera.0);
    if !d.is_finite() || !occluder_m.is_finite() || occluder_m <= 0.0 || d <= occluder_m {
        return true;
    }
    let Some(up) = unit(camera.0) else {
        return true;
    };
    // Horizon plane: points with `q · up` below `occluder² / d` lie beyond it.
    let plane = occluder_m * occluder_m / d;
    if dot(bounds.centre, up) + bounds.radius >= plane {
        return true;
    }
    // Shadow cone: apex at the camera, axis to the centre, half-angle
    // `asin(occluder / d)`. The sphere must sit fully inside it.
    let to = [
        bounds.centre[0] - camera.0[0],
        bounds.centre[1] - camera.0[1],
        bounds.centre[2] - camera.0[2],
    ];
    let dist = length(to);
    if !dist.is_finite() || dist <= bounds.radius {
        return true;
    }
    let half = (occluder_m / d).asin();
    let axis = [-up[0], -up[1], -up[2]];
    let off_axis = (dot(to, axis) / dist).clamp(-1.0, 1.0).acos();
    let spread = (bounds.radius / dist).asin();
    off_axis + spread > half
}

/// Whether `key` may be seen from `view`: inside the view and above the
/// horizon (Q3 A, Q4 A). Bad geometry counts as visible.
#[must_use]
pub fn patch_visible(
    view: &View,
    key: PatchKey,
    radius_m: f64,
    heights_m: (f64, f64),
    occluder_m: f64,
) -> bool {
    let Some(bounds) = patch_bounds(key, radius_m, heights_m.0, heights_m.1) else {
        return true;
    };
    view.sees_sphere(bounds) && above_horizon(view.camera, bounds, occluder_m)
}

/// Culling inputs for one selection update (#778 T2).
///
/// `heights` gives a built patch's `(min_h_m, max_h_m)`. An unbuilt patch
/// takes its nearest built ancestor's range, grown by half that range plus
/// [`UNBUILT_PAD_M`] (#918), clamped to `fallback` (the planet's
/// [`crate::relief::Relief::height_bounds_m`]), which it uses when no
/// ancestor is built. `occluder_m` is the planet radius plus the lowest
/// relief.
pub struct Cull<'a> {
    /// The camera view.
    pub view: View,
    /// Built patch height range, `None` when unbuilt.
    pub heights: &'a dyn Fn(PatchKey) -> Option<(f64, f64)>,
    /// Height range for unbuilt patches.
    pub fallback: (f64, f64),
    /// Horizon occluder radius in metres.
    pub occluder_m: f64,
}

/// Height margin in metres around an unbuilt patch's ancestor range (#918).
pub const UNBUILT_PAD_M: f64 = 200.0;

impl Cull<'_> {
    /// Whether `key` may be seen on a planet of `radius_m`.
    #[must_use]
    pub fn sees(&self, key: PatchKey, radius_m: f64) -> bool {
        patch_visible(
            &self.view,
            key,
            radius_m,
            self.heights_of(key),
            self.occluder_m,
        )
    }

    /// Height range for `key`: its own when built, else its nearest built
    /// ancestor's with a margin (#918: the whole planet's range made every
    /// unbuilt patch near a jump look visible, so about half the builds of
    /// an arrival were thrown away), else `fallback`.
    #[must_use]
    pub fn heights_of(&self, key: PatchKey) -> (f64, f64) {
        if let Some(own) = (self.heights)(key) {
            return own;
        }
        let mut cursor = key;
        while let Some(parent) = cursor.parent() {
            if let Some((min_h, max_h)) = (self.heights)(parent) {
                let pad = (max_h - min_h).max(0.0) * 0.5 + UNBUILT_PAD_M;
                return (
                    (min_h - pad).max(self.fallback.0),
                    (max_h + pad).min(self.fallback.1),
                );
            }
            cursor = parent;
        }
        self.fallback
    }
}

impl std::fmt::Debug for Cull<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cull")
            .field("view", &self.view)
            .field("fallback", &self.fallback)
            .field("occluder_m", &self.occluder_m)
            .finish_non_exhaustive()
    }
}
