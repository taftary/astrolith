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
pub const CENTRE_INDEX: i32 = 16;

/// The per-patch values a shader would get as uniforms, all `f32`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridFrame {
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
    #[must_use]
    pub fn new(key: PatchKey, radius_c_m: f64) -> Option<GridFrame> {
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
#[must_use]
pub fn exact_offset(frame: &GridFrame, k: i32, l: i32, dh: f32) -> [f32; 3] {
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
#[must_use]
pub fn naive_offset(frame: &GridFrame, k: i32, l: i32, dh: f32, centre: [f32; 3]) -> [f32; 3] {
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
pub fn steps_of(index: usize) -> Option<(i32, i32)> {
    let side = usize::try_from(PATCH_QUADS + 1).ok()?;
    let (di, dj) = (
        i32::try_from(index % side).ok()?,
        i32::try_from(index / side).ok()?,
    );
    Some((di - CENTRE_INDEX, dj - CENTRE_INDEX))
}

/// Width of one patch tile in the shared heights texture (#920 T2): the
/// 33-point patch side plus a ring of two points on each side.
pub const TILE_W: u32 = 37;

/// Height of one tile: 37 rows of heights, 30 rows holding the 1,089 morph
/// errors in row-major order, and one row of per-patch values.
pub const TILE_H: u32 = 68;

/// First tile row of the morph errors.
const ERR_ROW: u32 = 37;

/// Tile row of the per-patch values.
const PARAM_ROW: u32 = 67;

/// Face code the shader reads: `PosX, NegX, PosY, NegY, PosZ, NegZ` as
/// 0 to 5.
fn face_code(face: Face) -> f32 {
    match face {
        Face::PosX => 0.0,
        Face::NegX => 1.0,
        Face::PosY => 2.0,
        Face::NegY => 3.0,
        Face::PosZ => 4.0,
        Face::NegZ => 5.0,
    }
}

/// Unit direction of grid point `(gi, gj)` of `n` on `face`, also beyond the
/// face edge (the ring of a patch on a face edge): on the face it is
/// exactly [`crate::cube_sphere::grid_point`]; beyond, the face's tangent
/// plane continues.
fn ring_dir(face: Face, gi: i64, gj: i64, n: u32) -> Option<[f64; 3]> {
    let n64 = i64::from(n);
    if (0..=n64).contains(&gi) && (0..=n64).contains(&gj) {
        return crate::cube_sphere::grid_point(
            face,
            u32::try_from(gi).ok()?,
            u32::try_from(gj).ok()?,
            n,
        );
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: grid indices far below 2^52, exact in f64"
    )]
    let (u, v) = (
        -1.0 + 2.0 * gi as f64 / f64::from(n),
        -1.0 + 2.0 * gj as f64 / f64::from(n),
    );
    let quarter = std::f64::consts::FRAC_PI_4;
    let (tu, tv) = ((u * quarter).tan(), (v * quarter).tan());
    let (dom, sign, ua, va) = face_axes(face);
    let mut c = [0.0f64; 3];
    *c.get_mut(dom)? = f64::from(sign);
    *c.get_mut(ua)? = tu;
    *c.get_mut(va)? = tv;
    let len = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
    (len.is_finite() && len > 0.0).then(|| [c[0] / len, c[1] / len, c[2] / len])
}

/// The texels of one patch tile, row-major `TILE_W x TILE_H` (#920 T2):
/// heights over the centre height on the 37 x 37 ring grid, the morph
/// errors, then `face, u-angle, v-angle, step, R + h_c, skirt depth,
/// has parent` (roots never morph).
/// `None` for a bad key or radius.
#[must_use]
pub fn grid_tile(
    mesh: &crate::lod_mesh::PatchMesh,
    radius_m: f64,
    relief: &crate::relief::Relief,
) -> Option<Vec<f32>> {
    let key = mesh.key;
    let centre_dir = crate::lod::patch_centre_dir(key)?;
    let centre_h = relief.height_m(centre_dir);
    let radius_c = radius_m + centre_h;
    let frame = GridFrame::new(key, radius_c)?;
    let n = key.per_edge()?.checked_mul(PATCH_QUADS)?;
    let (i0, j0) = (
        i64::from(key.x.checked_mul(PATCH_QUADS)?),
        i64::from(key.y.checked_mul(PATCH_QUADS)?),
    );
    let size = usize::try_from(TILE_W * TILE_H).ok()?;
    let mut tile = vec![0.0f32; size];
    for rj in 0..TILE_W {
        for ri in 0..TILE_W {
            let dir = ring_dir(key.face, i0 + i64::from(ri) - 2, j0 + i64::from(rj) - 2, n)?;
            let slot = tile.get_mut(usize::try_from(rj * TILE_W + ri).ok()?)?;
            *slot = narrow(relief.height_m(dir) - centre_h);
        }
    }
    let first = usize::try_from(ERR_ROW * TILE_W).ok()?;
    for (k, err) in mesh.morph_err_m.iter().copied().enumerate() {
        *tile.get_mut(first + k)? = err;
    }
    // Skirt depth as the build sets it: max(4 x patch error, 1 m).
    let sag = crate::lod::patch_quad_sag_m(key, radius_m).unwrap_or(0.0);
    let err = sag.max(if mesh.relief_error_m.is_finite() {
        mesh.relief_error_m.max(0.0)
    } else {
        0.0
    });
    let depth = (4.0 * err).max(1.0);
    let params = [
        face_code(key.face),
        frame.au,
        frame.av,
        frame.step,
        frame.radius_c,
        narrow(depth),
        if key.level > 0 { 1.0 } else { 0.0 },
    ];
    let base = usize::try_from(PARAM_ROW * TILE_W).ok()?;
    for (k, value) in params.into_iter().enumerate() {
        *tile.get_mut(base + k)? = value;
    }
    Some(tile)
}

/// The tile's own vertex `(di, dj)` (0..33 each) rebuilt from the tile alone,
/// exactly as `patch_grid.wgsl` reads it: the CPU mirror the tests hold the
/// shader's inputs to. `None` outside the tile.
#[must_use]
pub fn tile_offset(tile: &[f32], di: i32, dj: i32) -> Option<[f32; 3]> {
    let at = |i: u32| tile.get(usize::try_from(i).ok()?).copied();
    let param = |k: u32| at(PARAM_ROW * TILE_W + k);
    let face = match param(0)? {
        c if c < 0.5 => Face::PosX,
        c if c < 1.5 => Face::NegX,
        c if c < 2.5 => Face::PosY,
        c if c < 3.5 => Face::NegY,
        c if c < 4.5 => Face::PosZ,
        _ => Face::NegZ,
    };
    let (dom, sign, ua, va) = face_axes(face);
    let frame = GridFrame {
        dom,
        sign,
        ua,
        va,
        au: param(1)?,
        av: param(2)?,
        step: param(3)?,
        radius_c: param(4)?,
    };
    let (ri, rj) = (u32::try_from(di + 2).ok()?, u32::try_from(dj + 2).ok()?);
    if ri >= TILE_W || rj >= TILE_W {
        return None;
    }
    let dh = at(rj * TILE_W + ri)?;
    Some(exact_offset(
        &frame,
        di - CENTRE_INDEX,
        dj - CENTRE_INDEX,
        dh,
    ))
}

#[cfg(test)]
mod tests {
    use super::{TILE_H, TILE_W, grid_tile, tile_offset};
    use crate::cube_sphere::Face;
    use crate::lod::PatchKey;
    use crate::lod_mesh::build_patch;
    use crate::planet_frame::EARTH_RADIUS_M;

    #[test]
    fn a_tile_rebuilds_its_patch_under_a_millimetre() {
        // #920 T2: the packed tile alone (heights, frame values) gives back
        // today's offsets, on every face, at a coarse and a deep level, and
        // carries the morph errors unchanged.
        let relief = crate::headless_morph::earth_relief();
        for face in [
            Face::PosX,
            Face::NegX,
            Face::PosY,
            Face::NegY,
            Face::PosZ,
            Face::NegZ,
        ] {
            for (level, x, y) in [(3u32, 2u32, 5u32), (14, 9_000, 7_001)] {
                let key = PatchKey::new(face, level, x, y).expect("key");
                let mesh = build_patch(key, EARTH_RADIUS_M, &relief).expect("mesh");
                let tile = grid_tile(&mesh, EARTH_RADIUS_M, &relief).expect("tile");
                assert_eq!(tile.len(), (TILE_W * TILE_H) as usize);
                let quad = crate::lod_range::edge_m(level, EARTH_RADIUS_M) / 32.0;
                for (index, today) in mesh.offsets.iter().enumerate() {
                    let (k, l) = super::steps_of(index).expect("in patch");
                    let (di, dj) = (k + super::CENTRE_INDEX, l + super::CENTRE_INDEX);
                    let got = tile_offset(&tile, di, dj).expect("in tile");
                    let gap = ((got[0] - today[0]).powi(2)
                        + (got[1] - today[1]).powi(2)
                        + (got[2] - today[2]).powi(2))
                    .sqrt();
                    let limit = if level >= 14 { 1e-3 } else { 1e-5 * quad };
                    assert!(f64::from(gap) < limit, "{key:?} vertex {index}: {gap} m");
                }
                let errs = &tile[(37 * TILE_W) as usize..][..1089];
                assert_eq!(errs, mesh.morph_err_m.as_slice(), "{key:?} morph errors");
            }
        }
    }
}
