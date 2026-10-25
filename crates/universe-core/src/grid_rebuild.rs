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

/// `sin x` by its Taylor series to `x^9`, `f32`: within about `1e-8` of
/// the true value for `|x| < 0.9`, and exact in relative terms for tiny
/// angles. The shader runs the same series, because GPU `sin` is only
/// bounded to about `2^-11` absolute (Vulkan), far too coarse for the
/// sub-microradian steps of deep patches.
#[must_use]
pub fn poly_sin(x: f32) -> f32 {
    let x2 = x * x;
    x * (1.0 + x2 * (-1.0 / 6.0 + x2 * (1.0 / 120.0 + x2 * (-1.0 / 5_040.0 + x2 / 362_880.0))))
}

/// `cos x` by its Taylor series to `x^10`, `f32` (see [`poly_sin`]).
#[must_use]
pub fn poly_cos(x: f32) -> f32 {
    let x2 = x * x;
    1.0 + x2
        * (-0.5
            + x2 * (1.0 / 24.0 + x2 * (-1.0 / 720.0 + x2 * (1.0 / 40_320.0 - x2 / 3_628_800.0))))
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
    let (cu, cv) = (poly_cos(au), poly_cos(av));
    let (tu_c, tv_c) = (poly_sin(au) / cu, poly_sin(av) / cv);
    // tan(a) - tan(a_c) without subtracting the two tangents.
    let dtu = poly_sin(da) / (poly_cos(au + da) * cu);
    let dtv = poly_sin(db) / (poly_cos(av + db) * cv);
    offset_from_steps(frame, [tu_c, tv_c], [dtu, dtv], dh)
}

/// The offset from the centre tangents `tc` and the tangent steps `dt` to
/// the point: what [`exact_offset`] does after its trigonometry, and all the
/// shader does once a tile carries the steps (#920 T2: no GPU trig).
#[must_use]
pub fn offset_from_steps(frame: &GridFrame, tc: [f32; 2], dt: [f32; 2], dh: f32) -> [f32; 3] {
    let ([tu_c, tv_c], [dtu, dtv]) = (tc, dt);
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
/// errors in row-major order, one row of per-patch values, and two rows of
/// tangent steps (along `u` by column, along `v` by row), then eight rows
/// of packed parent-level normals at the 17 x 17 even points.
pub const TILE_H: u32 = 78;

/// First tile row of the packed parent-level normals.
const PARENT_NORMAL_ROW: u32 = 70;

/// Bits per octahedral component of a packed normal: two fit exactly in
/// the 24-bit integer range of an `f32`.
const OCT_BITS: u32 = 12;

/// `n` (unit) as two 12-bit octahedral components in one integer-valued
/// `f32`, exact in the texture (`patch_grid_common::unpack_normal`).
#[must_use]
pub fn pack_normal(n: [f32; 3]) -> f32 {
    let sum = n[0].abs() + n[1].abs() + n[2].abs();
    if sum <= 0.0 {
        return pack_normal([0.0, 0.0, 1.0]);
    }
    let (mut x, mut y) = (n[0] / sum, n[1] / sum);
    if n[2] < 0.0 {
        let (ox, oy) = (x, y);
        x = (1.0 - oy.abs()) * ox.signum();
        y = (1.0 - ox.abs()) * oy.signum();
    }
    let top = f32::from(u16::try_from((1u32 << OCT_BITS) - 1).unwrap_or(u16::MAX));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "E-CAST: clamped to 0..4095 before the cast"
    )]
    let q = |c: f32| ((c * 0.5 + 0.5).clamp(0.0, 1.0) * top).round() as u32;
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: below 2^24, exact in f32"
    )]
    let packed = (q(x) + (q(y) << OCT_BITS)) as f32;
    packed
}

/// Tile row of the tangent steps along `u`, one per ring column.
const STEP_U_ROW: u32 = 68;

/// Tile row of the tangent steps along `v`, one per ring row.
const STEP_V_ROW: u32 = 69;

/// First tile row of the morph errors.
const ERR_ROW: u32 = 37;

/// Unpacks [`pack_normal`] as `patch_grid_common::parent_normal_even`
/// does (the CPU mirror the tests hold the shader to).
#[must_use]
pub fn unpack_normal(packed: f32) -> [f32; 3] {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "E-CAST: an integer below 2^24 by construction"
    )]
    let bits = packed as u32;
    let mask = (1u32 << OCT_BITS) - 1;
    let top = f32::from(u16::try_from(mask).unwrap_or(u16::MAX));
    let comp = |q: u32| f32::from(u16::try_from(q & mask).unwrap_or(0)) / top * 2.0 - 1.0;
    let (x, y) = (comp(bits), comp(bits >> OCT_BITS));
    let mut n = [x, y, 1.0 - x.abs() - y.abs()];
    let fold = (-n[2]).max(0.0);
    n[0] += if n[0] >= 0.0 { -fold } else { fold };
    n[1] += if n[1] >= 0.0 { -fold } else { fold };
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    [n[0] / len, n[1] / len, n[2] / len]
}

/// The tile's stored parent-level normal at even point `(2 ei, 2 ej)`.
#[must_use]
pub fn tile_parent_normal(tile: &[f32], ei: u32, ej: u32) -> Option<[f32; 3]> {
    let k = ej.checked_mul(PATCH_QUADS / 2 + 1)?.checked_add(ei)?;
    let at = PARENT_NORMAL_ROW.checked_mul(TILE_W)?.checked_add(k)?;
    tile.get(usize::try_from(at).ok()?)
        .copied()
        .map(unpack_normal)
}

/// Tile row of the per-patch values.
const PARAM_ROW: u32 = 67;

/// Tile texel `[column, row]` of the patch's morph mask (`lod_morph`
/// bits), written apart from the tile whenever the neighbours change, so
/// one material serves every mask (#920 T4).
pub const MASK_TEXEL: [u32; 2] = [9, PARAM_ROW];

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
/// has parent, tan u_c, tan v_c` (roots never morph), then the tangent
/// steps `tan u - tan u_c` per column and `tan v - tan v_c` per row, worked
/// out in `f64` so the shader needs no trigonometry.
/// `None` for a bad key or radius.
#[must_use]
pub fn grid_tile(
    mesh: &crate::lod_mesh::PatchMesh,
    radius_m: f64,
    relief: &crate::relief::Relief,
) -> Option<Vec<f32>> {
    tile_from(mesh, radius_m, relief, None)
}

/// A patch and its tile in one build (#920 T5): the tile reuses every
/// height the build sampled (the patch and its normals ring), so only ring
/// points off the face sample the relief again. The tile is the same as
/// [`grid_tile`]'s, bit for bit.
#[must_use]
pub fn build_patch_and_tile(
    key: PatchKey,
    radius_m: f64,
    relief: &crate::relief::Relief,
) -> Option<(crate::lod_mesh::PatchMesh, Option<Vec<f32>>)> {
    let (mesh, heights) = crate::lod_mesh::build_patch_with_heights(key, radius_m, relief)?;
    let tile = tile_from(&mesh, radius_m, relief, Some(&heights));
    Some((mesh, tile))
}

/// [`grid_tile`], with the heights the build sampled when known (37 x 37,
/// `None` where it sampled none).
fn tile_from(
    mesh: &crate::lod_mesh::PatchMesh,
    radius_m: f64,
    relief: &crate::relief::Relief,
    heights: Option<&[Option<f64>]>,
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
            let known = heights
                .and_then(|h| h.get(usize::try_from(rj * TILE_W + ri).ok()?).copied())
                .flatten();
            let h = match known {
                Some(h) => h,
                None => {
                    let dir =
                        ring_dir(key.face, i0 + i64::from(ri) - 2, j0 + i64::from(rj) - 2, n)?;
                    relief.height_m(dir)
                }
            };
            let slot = tile.get_mut(usize::try_from(rj * TILE_W + ri).ok()?)?;
            *slot = narrow(h - centre_h);
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
    let quarter = std::f64::consts::FRAC_PI_4;
    let (uc, vc) = crate::lod::patch_centre_uv(key)?;
    let (tu_c, tv_c) = ((uc * quarter).tan(), (vc * quarter).tan());
    let params = [
        face_code(key.face),
        frame.au,
        frame.av,
        frame.step,
        frame.radius_c,
        narrow(depth),
        if key.level > 0 { 1.0 } else { 0.0 },
        narrow(tu_c),
        narrow(tv_c),
    ];
    for r in 0..TILE_W {
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: grid indices far below 2^52, exact in f64"
        )]
        let along = |g0: i64| {
            ((-1.0 + 2.0 * (g0 + i64::from(r) - 2) as f64 / f64::from(n)) * quarter).tan()
        };
        let su = tile.get_mut(usize::try_from(STEP_U_ROW * TILE_W + r).ok()?)?;
        *su = narrow(along(i0) - tu_c);
        let sv = tile.get_mut(usize::try_from(STEP_V_ROW * TILE_W + r).ok()?)?;
        *sv = narrow(along(j0) - tv_c);
    }
    let base = usize::try_from(PARAM_ROW * TILE_W).ok()?;
    for (k, value) in params.into_iter().enumerate() {
        *tile.get_mut(base + k)? = value;
    }
    // The parent level's normals live on the even points; an odd vertex's
    // is the mean of its parent pair's, as `lod_patch_morph` builds it.
    let normals = usize::try_from(PARENT_NORMAL_ROW * TILE_W).ok()?;
    let side = usize::try_from(PATCH_QUADS + 1).ok()?;
    let half = side / 2 + 1;
    for k in 0..half * half {
        let idx = (k / half) * 2 * side + (k % half) * 2;
        *tile.get_mut(normals + k)? = pack_normal(*mesh.parent_normals.get(idx)?);
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
    let tc = [param(7)?, param(8)?];
    let dt = [at(STEP_U_ROW * TILE_W + ri)?, at(STEP_V_ROW * TILE_W + rj)?];
    Some(offset_from_steps(&frame, tc, dt, dh))
}

#[cfg(test)]
mod tests {
    use super::{TILE_H, TILE_W, grid_tile, tile_offset, tile_parent_normal};
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

    #[test]
    fn stored_parent_normals_match_todays_within_a_twentieth_of_a_degree() {
        // #920 T4: the parent level's normals are stored packed at the 17 x
        // 17 even points; an odd vertex's is the mean of its pair's, as
        // `lod_patch_morph` builds it. Both stay within 0.05 degrees of the
        // f32 normals today's meshes carry.
        let relief = crate::headless_morph::earth_relief();
        let angle = |a: [f32; 3], b: [f32; 3]| {
            let dot = f64::from(a[0] * b[0] + a[1] * b[1] + a[2] * b[2]);
            dot.clamp(-1.0, 1.0).acos().to_degrees()
        };
        let unit = |v: [f32; 3]| {
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            [v[0] / len, v[1] / len, v[2] / len]
        };
        for face in [Face::PosX, Face::NegY, Face::PosZ] {
            for (level, x, y) in [(3u32, 2u32, 5u32), (14, 9_000, 7_001)] {
                let key = PatchKey::new(face, level, x, y).expect("key");
                let mesh = build_patch(key, EARTH_RADIUS_M, &relief).expect("mesh");
                let tile = grid_tile(&mesh, EARTH_RADIUS_M, &relief).expect("tile");
                let stored =
                    |i: u32, j: u32| tile_parent_normal(&tile, i / 2, j / 2).expect("stored");
                let mut worst = 0.0f64;
                for (index, today) in mesh.parent_normals.iter().enumerate() {
                    let i = u32::try_from(index % 33).expect("column");
                    let j = u32::try_from(index / 33).expect("row");
                    let got = match (i % 2 == 1, j % 2 == 1) {
                        (false, false) => stored(i, j),
                        (false, true) => {
                            let (a, b) = (stored(i, j - 1), stored(i, j + 1));
                            unit([a[0] + b[0], a[1] + b[1], a[2] + b[2]])
                        }
                        (true, false) => {
                            let (a, b) = (stored(i - 1, j), stored(i + 1, j));
                            unit([a[0] + b[0], a[1] + b[1], a[2] + b[2]])
                        }
                        (true, true) => {
                            let (a, b) = (stored(i + 1, j - 1), stored(i - 1, j + 1));
                            unit([a[0] + b[0], a[1] + b[1], a[2] + b[2]])
                        }
                    };
                    worst = worst.max(angle(got, *today));
                }
                assert!(worst < 0.05, "{key:?}: {worst} degrees");
            }
        }
    }

    #[test]
    fn a_tile_built_with_its_patch_matches_one_packed_afterwards() {
        // #920 T5: reusing the build's own heights changes no texel.
        let relief = crate::headless_morph::earth_relief();
        for (face, level, x, y) in [
            (Face::PosY, 0u32, 0u32, 0u32),
            (Face::NegZ, 9, 300, 17),
            (Face::PosX, 14, 9_000, 7_001),
        ] {
            let key = PatchKey::new(face, level, x, y).expect("key");
            let (mesh, tile) =
                super::build_patch_and_tile(key, EARTH_RADIUS_M, &relief).expect("build");
            let after = grid_tile(&mesh, EARTH_RADIUS_M, &relief).expect("tile");
            let tile = tile.expect("tile with the build");
            assert_eq!(tile.len(), after.len());
            assert!(
                tile.iter()
                    .zip(&after)
                    .all(|(a, b)| a.to_bits() == b.to_bits()),
                "{key:?}: a texel differs"
            );
        }
    }
}

