//! One patch's tile in the shared grid's heights texture (#920).
//!
//! A shared-grid patch keeps only its tile: `TILE_W x TILE_H` texels of 32
//! bits (`R32Uint`), read in the vertex stage (`patch_grid_common.wgsl`)
//! and rebuilt there as [`crate::grid_rebuild`] does. Rows:
//!
//! - `0..37`: heights over the centre height on the 37 x 37 grid of the
//!   patch and a ring of two points around it (`f32` bits);
//! - `ERR_ROW..`: the 1,089 morph errors as 16-bit floats, two a texel,
//!   each a share of the patch's quad length (so coarse levels, whose
//!   errors reach hundreds of kilometres, stay in the half range);
//! - `PARAM_ROW`: the per-patch values (face, frame, skirt depth, parent
//!   flag, centre tangents, morph mask, quad length);
//! - `STEP_U_ROW`, `STEP_V_ROW`: the tangent steps per ring column and
//!   row, worked out in `f64` so the shader needs no trigonometry;
//! - `PARENT_NORMAL_ROW..`: the parent level's normals at the 17 x 17 even
//!   points, octahedral, 16 bits a component.
//!
//! 37 x 63 texels of 4 bytes: 9,324 bytes a patch (#920 AC5: at most 10
//! KB). Pure and deterministic (`E-CORE-NO-BEVY`, `E-DET-TIERS`).

use crate::cube_sphere::Face;
use crate::grid_rebuild::{GridFrame, face_axes, narrow, offset_from_steps};
use crate::lod::{PATCH_QUADS, PatchKey};

/// Width of one tile: the 33-point patch side plus a ring of two points
/// on each side.
pub const TILE_W: u32 = 37;

/// Height of one tile (see the module docs).
pub const TILE_H: u32 = 63;

/// First tile row of the morph errors (16-bit pairs).
pub const ERR_ROW: u32 = 37;

/// Tile row of the per-patch values.
pub const PARAM_ROW: u32 = 52;

/// Tile row of the tangent steps along `u`, one per ring column.
pub const STEP_U_ROW: u32 = 53;

/// Tile row of the tangent steps along `v`, one per ring row.
pub const STEP_V_ROW: u32 = 54;

/// First tile row of the packed parent-level normals.
pub const PARENT_NORMAL_ROW: u32 = 55;

/// Tile texel `[column, row]` of the patch's morph mask (`lod_morph`
/// bits), written apart from the tile whenever the neighbours change, so
/// one material serves every mask (#920 T4).
pub const MASK_TEXEL: [u32; 2] = [9, PARAM_ROW];

/// Per-patch value slots in `PARAM_ROW`.
const P_FACE: u32 = 0;
const P_AU: u32 = 1;
const P_AV: u32 = 2;
const P_STEP: u32 = 3;
const P_RADIUS: u32 = 4;
const P_DEPTH: u32 = 5;
const P_PARENT: u32 = 6;
const P_TU: u32 = 7;
const P_TV: u32 = 8;
/// Per-patch value slot of the quad length, the morph errors' unit.
pub const P_QUAD: u32 = 10;

/// Points along one patch side.
const SIDE: usize = 33;

/// Even points along one side: the parent grid.
const HALF: usize = 17;

/// `x` as an IEEE half (round to nearest, ties to even): what WGSL's
/// `unpack2x16float` reads back.
#[must_use]
pub fn f16_bits(x: f32) -> u16 {
    let bits = x.to_bits();
    let sign = u16::try_from((bits >> 16) & 0x8000).unwrap_or(0);
    let exp = i32::try_from((bits >> 23) & 0xFF).unwrap_or(0);
    let man = bits & 0x007F_FFFF;
    if exp == 0xFF {
        // Infinity or NaN.
        return sign | 0x7C00 | if man == 0 { 0 } else { 0x0200 };
    }
    let e = exp - 127 + 15;
    if e >= 0x1F {
        return sign | 0x7C00;
    }
    if e <= 0 {
        // Subnormal half (or zero): shift the full mantissa in.
        if e < -10 {
            return sign;
        }
        let full = man | 0x0080_0000;
        let shift = u32::try_from(14 - e).unwrap_or(31);
        let half = full >> shift;
        let rest = full & ((1 << shift) - 1);
        let mid = 1 << (shift - 1);
        let up = rest > mid || (rest == mid && half & 1 == 1);
        return sign | u16::try_from(half + u32::from(up)).unwrap_or(0);
    }
    let half = (u32::try_from(e).unwrap_or(0) << 10) | (man >> 13);
    let rest = man & 0x1FFF;
    let up = rest > 0x1000 || (rest == 0x1000 && half & 1 == 1);
    // A carry out of the mantissa steps the exponent, as it should.
    sign | u16::try_from(half + u32::from(up)).unwrap_or(0x7C00)
}

/// The value of half `h` (the CPU mirror of `unpack2x16float`).
#[must_use]
pub fn f16_value(h: u16) -> f32 {
    let sign = if h & 0x8000 == 0 { 1.0f32 } else { -1.0 };
    let exp = i32::from((h >> 10) & 0x1F);
    let man = f32::from(h & 0x03FF);
    match exp {
        0 => sign * man * 2f32.powi(-24),
        0x1F => {
            if man == 0.0 {
                sign * f32::INFINITY
            } else {
                f32::NAN
            }
        }
        _ => sign * (1.0 + man / 1024.0) * 2f32.powi(exp - 15),
    }
}

/// `n` (unit) as two 16-bit octahedral components in one texel
/// (`patch_grid_common::parent_normal_even` unpacks it).
#[must_use]
pub fn pack_normal(n: [f32; 3]) -> u32 {
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
    let top = f32::from(u16::MAX);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "E-CAST: clamped to 0..65535 before the cast"
    )]
    let q = |c: f32| ((c * 0.5 + 0.5).clamp(0.0, 1.0) * top).round() as u32;
    q(x) | (q(y) << 16)
}

/// Unpacks [`pack_normal`] as the shader does (the CPU mirror the tests
/// hold the shader to).
#[must_use]
pub fn unpack_normal(packed: u32) -> [f32; 3] {
    let top = f32::from(u16::MAX);
    let comp = |q: u32| f32::from(u16::try_from(q & 0xFFFF).unwrap_or(0)) / top * 2.0 - 1.0;
    let (x, y) = (comp(packed), comp(packed >> 16));
    let mut n = [x, y, 1.0 - x.abs() - y.abs()];
    let fold = (-n[2]).max(0.0);
    n[0] += if n[0] >= 0.0 { -fold } else { fold };
    n[1] += if n[1] >= 0.0 { -fold } else { fold };
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    [n[0] / len, n[1] / len, n[2] / len]
}

/// Face code the shader reads: `PosX, NegX, PosY, NegY, PosZ, NegZ` as
/// 0 to 5.
fn face_code(face: Face) -> u32 {
    match face {
        Face::PosX => 0,
        Face::NegX => 1,
        Face::PosY => 2,
        Face::NegY => 3,
        Face::PosZ => 4,
        Face::NegZ => 5,
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

/// Quad length of `key`'s level, the unit of its stored morph errors.
fn quad_m(key: PatchKey, radius_m: f64) -> f64 {
    crate::lod_range::edge_m(key.level, radius_m) / f64::from(PATCH_QUADS)
}

/// The texels of `mesh`'s tile, row-major `TILE_W x TILE_H` (see the
/// module docs). `None` for a bad key or radius.
#[must_use]
pub fn grid_tile(
    mesh: &crate::lod_mesh::PatchMesh,
    radius_m: f64,
    relief: &crate::relief::Relief,
) -> Option<Vec<u32>> {
    tile_from(mesh, radius_m, relief, None)
}

/// A patch and its tile in one build (#920 T5): the tile reuses every
/// height the build sampled (the patch and its normals ring), so only ring
/// points off the face sample the relief again. The tile is the same as
/// [`grid_tile`]'s, bit for bit. Without `full`, the patch is tile-only
/// ([`crate::lod_mesh::PatchMesh::full`]): what the tile and the cache need,
/// no normals (#920 T6).
#[must_use]
pub fn build_patch_and_tile(
    key: PatchKey,
    radius_m: f64,
    relief: &crate::relief::Relief,
    full: bool,
) -> Option<(crate::lod_mesh::PatchMesh, Option<Vec<u32>>)> {
    build_patch_and_tile_reusing(key, radius_m, relief, full, None)
        .map(|(mesh, tile, _)| (mesh, tile))
}

/// [`build_patch_and_tile`] reading the `parent`'s fine grid where it
/// covers the patch's own points, and returning the patch's fine grid for
/// its children (#994 T3). The same patch and tile, bit for bit.
#[must_use]
pub fn build_patch_and_tile_reusing(
    key: PatchKey,
    radius_m: f64,
    relief: &crate::relief::Relief,
    full: bool,
    parent: Option<&crate::lod_reuse::FineGrid>,
) -> Option<(
    crate::lod_mesh::PatchMesh,
    Option<Vec<u32>>,
    Option<crate::lod_reuse::FineGrid>,
)> {
    let (mesh, heights, fine) =
        crate::lod_mesh::build_patch_reusing(key, radius_m, relief, full, parent)?;
    let tile = tile_from(&mesh, radius_m, relief, Some(&heights));
    let fine = crate::lod_reuse::FineGrid::new(key, relief, radius_m, fine);
    Some((mesh, tile, fine))
}

/// [`grid_tile`], with the heights the build sampled when known (37 x 37,
/// `None` where it sampled none).
fn tile_from(
    mesh: &crate::lod_mesh::PatchMesh,
    radius_m: f64,
    relief: &crate::relief::Relief,
    heights: Option<&[Option<f64>]>,
) -> Option<Vec<u32>> {
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
    let mut tile = vec![0u32; usize::try_from(TILE_W * TILE_H).ok()?];
    let mut put = |col: u32, row: u32, value: u32| -> Option<()> {
        *tile.get_mut(usize::try_from(row * TILE_W + col).ok()?)? = value;
        Some(())
    };
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
            put(ri, rj, narrow(h - centre_h).to_bits())?;
        }
    }
    // Morph errors as shares of the quad length, two halves a texel.
    let quad = quad_m(key, radius_m);
    let share = |k: usize| {
        mesh.morph_err_m
            .get(k)
            .map_or(0, |err| f16_bits(narrow(f64::from(*err) / quad)))
    };
    for k in 0..mesh.morph_err_m.len().div_ceil(2) {
        let pair = u32::from(share(2 * k)) | (u32::from(share(2 * k + 1)) << 16);
        let k = u32::try_from(k).ok()?;
        put(k % TILE_W, ERR_ROW + k / TILE_W, pair)?;
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
    for (slot, value) in [
        (P_FACE, face_code(key.face)),
        (P_AU, frame.au.to_bits()),
        (P_AV, frame.av.to_bits()),
        (P_STEP, frame.step.to_bits()),
        (P_RADIUS, frame.radius_c.to_bits()),
        (P_DEPTH, narrow(depth).to_bits()),
        (P_PARENT, u32::from(key.level > 0)),
        (P_TU, narrow(tu_c).to_bits()),
        (P_TV, narrow(tv_c).to_bits()),
        (P_QUAD, narrow(quad).to_bits()),
    ] {
        put(slot, PARAM_ROW, value)?;
    }
    for r in 0..TILE_W {
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: grid indices far below 2^52, exact in f64"
        )]
        let along = |g0: i64| {
            ((-1.0 + 2.0 * (g0 + i64::from(r) - 2) as f64 / f64::from(n)) * quarter).tan()
        };
        put(r, STEP_U_ROW, narrow(along(i0) - tu_c).to_bits())?;
        put(r, STEP_V_ROW, narrow(along(j0) - tv_c).to_bits())?;
    }
    // The parent level's normals live on the even points; an odd vertex's
    // is the mean of its parent pair's, as `lod_patch_morph` builds it.
    for k in 0..HALF * HALF {
        let idx = (k / HALF) * 2 * SIDE + (k % HALF) * 2;
        let k = u32::try_from(k).ok()?;
        put(
            k % TILE_W,
            PARENT_NORMAL_ROW + k / TILE_W,
            pack_normal(*mesh.parent_normals.get(idx)?),
        )?;
    }
    Some(tile)
}

/// Texel `(col, row)` of `tile`.
fn texel(tile: &[u32], col: u32, row: u32) -> Option<u32> {
    tile.get(usize::try_from(row.checked_mul(TILE_W)?.checked_add(col)?).ok()?)
        .copied()
}

/// Per-patch value `slot` read as an `f32`.
fn param(tile: &[u32], slot: u32) -> Option<f32> {
    texel(tile, slot, PARAM_ROW).map(f32::from_bits)
}

/// The tile's own vertex `(di, dj)` (0..33 each) rebuilt from the tile alone,
/// exactly as `patch_grid_common.wgsl` reads it: the CPU mirror the tests
/// hold the shader's inputs to. `None` outside the tile.
#[must_use]
pub fn tile_offset(tile: &[u32], di: i32, dj: i32) -> Option<[f32; 3]> {
    let face = match texel(tile, P_FACE, PARAM_ROW)? {
        0 => Face::PosX,
        1 => Face::NegX,
        2 => Face::PosY,
        3 => Face::NegY,
        4 => Face::PosZ,
        _ => Face::NegZ,
    };
    let frame = GridFrame::from_parts(
        face,
        [
            param(tile, P_AU)?,
            param(tile, P_AV)?,
            param(tile, P_STEP)?,
            param(tile, P_RADIUS)?,
        ],
    );
    let (ri, rj) = (u32::try_from(di + 2).ok()?, u32::try_from(dj + 2).ok()?);
    if ri >= TILE_W || rj >= TILE_W {
        return None;
    }
    let dh = f32::from_bits(texel(tile, ri, rj)?);
    let tc = [param(tile, P_TU)?, param(tile, P_TV)?];
    let dt = [
        f32::from_bits(texel(tile, ri, STEP_U_ROW)?),
        f32::from_bits(texel(tile, rj, STEP_V_ROW)?),
    ];
    Some(offset_from_steps(&frame, tc, dt, dh))
}

/// The morph error (metres) of row-major vertex `index`, as the shader
/// reads it: the stored half share times the quad length.
#[must_use]
pub fn tile_morph_err(tile: &[u32], index: u32) -> Option<f32> {
    let k = index / 2;
    let pair = texel(tile, k % TILE_W, ERR_ROW + k / TILE_W)?;
    let half = if index.is_multiple_of(2) {
        pair
    } else {
        pair >> 16
    };
    Some(param(tile, P_QUAD)? * f16_value(u16::try_from(half & 0xFFFF).ok()?))
}

/// The tile's stored parent-level normal at even point `(2 ei, 2 ej)`.
#[must_use]
pub fn tile_parent_normal(tile: &[u32], ei: u32, ej: u32) -> Option<[f32; 3]> {
    let k = ej.checked_mul(u32::try_from(HALF).ok()?)?.checked_add(ei)?;
    texel(tile, k % TILE_W, PARENT_NORMAL_ROW + k / TILE_W).map(unpack_normal)
}

#[cfg(test)]
#[path = "grid_tile_tests.rs"]
mod tests;
