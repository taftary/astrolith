//! Shared-grid rebuild study (#920 T1).
//!
//! A shared-grid patch keeps only its heights; the vertex shader rebuilds
//! every offset from the patch centre. This module is that rebuild written
//! in plain Rust with `f32` arithmetic only, the operations a WGSL shader
//! would run, so its error against today's `f64`-built offsets
//! ([`crate::lod_mesh::build_patch`]) can be measured headlessly.
//!
//! Two forms:
//! - [`naive_offset`]: `(R + h) x dir - centre` straight in `f32`. Two
//!   planet-sized numbers are subtracted, so the result is off by about the
//!   `f32` spacing at the planet radius (decimetres).
//! - [`exact_offset`]: only small differences ever meet. The face tangent
//!   step comes from `tan a - tan b = sin(a - b) / (cos a cos b)` with
//!   `a - b` an exact multiple of the grid step, the direction change from
//!   the cube point's length change, and the height as `h - h_centre`
//!   (stored relative to the centre, so mountains keep their millimetres).
//!
//! Pure and deterministic (`E-CORE-NO-BEVY`, `E-DET-TIERS`).

use crate::cube_sphere::Face;
use crate::lod::{PATCH_QUADS, PatchKey};

/// Index of the centre vertex along one patch side (the middle of 33).
pub(crate) const CENTRE_INDEX: i32 = 16;

/// The per-patch values a shader would get as uniforms, all `f32`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GridFrame {
    /// Cube axis the face is on (0, 1, 2).
    dom: usize,
    /// Sign of that axis (`1` or `-1`).
    sign: f32,
    /// Axis of the first tangent `u`.
    ua: usize,
    /// Axis of the second tangent `v`.
    va: usize,
    /// Centre angle along `u`: `u_c x pi / 4`.
    au: f32,
    /// Centre angle along `v`.
    av: f32,
    /// Angle of one grid step: `pi / (2 n)`.
    step: f32,
    /// Centre radius `R + h_c` in metres.
    radius_c: f32,
}

/// Face axes: dominant axis, its sign, then the `u` and `v` axes, as
/// `cube_sphere::FacePos` lays them out.
fn face_axes(face: Face) -> (usize, f32, usize, usize) {
    match face {
        Face::PosX => (0, 1.0, 1, 2),
        Face::NegX => (0, -1.0, 1, 2),
        Face::PosY => (1, 1.0, 0, 2),
        Face::NegY => (1, -1.0, 0, 2),
        Face::PosZ => (2, 1.0, 0, 1),
        Face::NegZ => (2, -1.0, 0, 1),
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "E-CAST: the study narrows to f32 on purpose, as the GPU would"
)]
fn narrow(x: f64) -> f32 {
    x as f32
}

impl GridFrame {
    /// The frame of `key` with centre radius `radius_c_m` (`R + h_c`).
    pub(crate) fn new(key: PatchKey, radius_c_m: f64) -> Option<GridFrame> {
        let (u, v) = crate::lod::patch_centre_uv(key)?;
        let n = f64::from(key.per_edge()?.checked_mul(PATCH_QUADS)?);
        let (dom, sign, ua, va) = face_axes(key.face);
        let quarter = std::f64::consts::FRAC_PI_4;
        Some(GridFrame {
            dom,
            sign,
            ua,
            va,
            au: narrow(u * quarter),
            av: narrow(v * quarter),
            step: narrow(std::f64::consts::FRAC_PI_2 / n),
            radius_c: narrow(radius_c_m),
        })
    }

    /// Cube point components `(dominant, u, v)` placed on their axes.
    fn place(&self, d: f32, u: f32, v: f32) -> [f32; 3] {
        let mut out = [0.0f32; 3];
        if let Some(slot) = out.get_mut(self.dom) {
            *slot = d;
        }
        if let Some(slot) = out.get_mut(self.ua) {
            *slot = u;
        }
        if let Some(slot) = out.get_mut(self.va) {
            *slot = v;
        }
        out
    }
}

/// Offset from the patch centre of grid point `(k, l)` steps away from the
/// centre vertex, at height `dh` metres over the centre height. Only small
/// numbers are ever subtracted (see the module docs).
pub(crate) fn exact_offset(frame: &GridFrame, k: i32, l: i32, dh: f32) -> [f32; 3] {
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: grid steps are small integers, exact in f32"
    )]
    let (da, db) = (k as f32 * frame.step, l as f32 * frame.step);
    let (au, av) = (frame.au, frame.av);
    let (tu_c, tv_c) = (au.tan(), av.tan());
    // tan(a) - tan(a_c) without subtracting the two tangents.
    let dtu = da.sin() / ((au + da).cos() * au.cos());
    let dtv = db.sin() / ((av + db).cos() * av.cos());
    let (tu, tv) = (tu_c + dtu, tv_c + dtv);
    let len_c2 = 1.0 + tu_c * tu_c + tv_c * tv_c;
    // |c|^2 - |c_c|^2 from the small steps only.
    let dlen2 = dtu * (tu + tu_c) + dtv * (tv + tv_c);
    let len_c = len_c2.sqrt();
    let len = (len_c2 + dlen2).sqrt();
    let cc = frame.place(frame.sign, tu_c, tv_c);
    let dc = frame.place(0.0, dtu, dtv);
    let shrink = dlen2 / (len * len_c * (len + len_c));
    let mut out = [0.0f32; 3];
    for (axis, slot) in out.iter_mut().enumerate() {
        let (c, d) = (
            cc.get(axis).copied().unwrap_or(0.0),
            dc.get(axis).copied().unwrap_or(0.0),
        );
        // Direction change d - d_c, then (R_c + dh) (d - d_c) + dh d_c.
        let dd = d / len - c * shrink;
        *slot = (frame.radius_c + dh) * dd + dh * (c / len_c);
    }
    out
}

/// The same point the straightforward way, all in `f32`: the planet-frame
/// position minus the centre (`centre` narrowed), as a shader without care
/// would compute it.
pub(crate) fn naive_offset(
    frame: &GridFrame,
    k: i32,
    l: i32,
    dh: f32,
    centre: [f32; 3],
) -> [f32; 3] {
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: grid steps are small integers, exact in f32"
    )]
    let (a, b) = (
        frame.au + k as f32 * frame.step,
        frame.av + l as f32 * frame.step,
    );
    let c = frame.place(frame.sign, a.tan(), b.tan());
    let len = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
    let r = frame.radius_c + dh;
    [
        r * c[0] / len - centre[0],
        r * c[1] / len - centre[1],
        r * c[2] / len - centre[2],
    ]
}

/// Grid offset `(k, l)` from the centre vertex for row-major vertex `index`
/// of a 33 x 33 patch.
#[must_use]
pub(crate) fn steps_of(index: usize) -> Option<(i32, i32)> {
    let side = usize::try_from(PATCH_QUADS + 1).ok()?;
    let (di, dj) = (
        i32::try_from(index % side).ok()?,
        i32::try_from(index / side).ok()?,
    );
    Some((di - CENTRE_INDEX, dj - CENTRE_INDEX))
}
