//! The ground's material (rebuild step 4, #777; one shared grid, #920).
//!
//! Every ground patch draws the one shared grid with
//! `patch_grid::PatchGridMaterial`, `ExtendedMaterial<StandardMaterial,
//! PatchGrid>` with embedded WGSL (the pattern of ADR 0027, #626). This
//! module holds what the grid's materials share: the base (the one ground
//! colour, #892; matte, one-sided) and the band scale uniform. The vertex
//! shader blends each vertex between its own and its parent-level shape by
//! its own distance to the camera (CDLOD, #777 Q2 A, Q6 A, Q7 A, core
//! `lod_morph::vertex_factor`), pinning or holding edges by the patch's
//! morph mask, which lives in its tile (Q3 A, #920 T4).
//!
//! Nothing changes per patch per frame (#777 review item 4): only a Planet
//! detail change rewrites the band scale. Pure morph math lives in core;
//! this module only wires Bevy assets.

use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;

/// The band scale every ground material shares.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType, Reflect)]
pub(crate) struct GroundScale {
    /// `x`: metres per world unit, `y`: `focal_px / detail_px`, so
    /// `err x y` is the band end core uses.
    pub(crate) scale: Vec4,
}

impl GroundScale {
    /// The scale for `metres_per_unit` and `band_scale`.
    #[must_use]
    pub(crate) fn new(metres_per_unit: f32, band_scale: f32) -> GroundScale {
        GroundScale {
            scale: Vec4::new(metres_per_unit, band_scale, 0.0, 0.0),
        }
    }
}

/// The ground's lit base: the one ground colour (#892 T2, Q9 C; #918 T4),
/// matte, one-sided.
#[must_use]
pub(crate) fn ground_base() -> StandardMaterial {
    StandardMaterial {
        base_color: crate::tokens::RELIEF_GROUND_TINT,
        // Lit by the one sunlight through the morphed terrain normals
        // (#780 Q1 A): matte ground, no shine.
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        // One-sided (#778 Q5 A): core winds every triangle and skirt to
        // face out, so only the far side of the ground is dropped.
        cull_mode: Some(bevy::render::render_resource::Face::Back),
        ..default()
    }
}

/// Largest mesh slab (bytes). Bevy packs meshes into shared GPU slabs that
/// grow by 1.5x up to 512 MiB, copying the whole slab on each growth; the
/// per-patch meshes before #920 froze single frames for 190-380 ms near
/// the ground on the dev PC that way. Capped slabs start a new one
/// instead, so a growth copies at most this much.
const MESH_SLAB_MAX_BYTES: u64 = 16 * 1024 * 1024;

/// Registers the ground's material and its embedded shaders (window app).
pub struct PatchMaterialPlugin;

impl Plugin for PatchMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::patch_grid::PatchGridPlugin);
        if let Some(render) = app.get_sub_app_mut(bevy::render::RenderApp) {
            let mut settings = bevy::render::mesh::allocator::MeshAllocatorSettings::default();
            settings.slab_allocator_settings.max_slab_size = MESH_SLAB_MAX_BYTES;
            settings.slab_allocator_settings.large_threshold = MESH_SLAB_MAX_BYTES / 2;
            render.insert_resource(settings);
        }
    }
}

#[cfg(test)]
mod tests {
    use universe_core::lod_morph::{morph_end_m, morph_factor, morph_mask, vertex_factor};
    use universe_core::precision::focal_px;

    /// The WGSL `grid_factor`, run on the CPU in `f32`: the band from the
    /// error, then the edge pins of the tile's mask texel, decoded bit by
    /// bit as `tile_of` decodes it.
    fn shader_factor(distance_m: f32, err: f32, band_scale: f32, flags: [f32; 4], mask: u8) -> f32 {
        let end = err * band_scale;
        let mut f = if end <= 0.0 {
            0.0
        } else {
            let zone = end * 0.3;
            let start = end - zone;
            if distance_m <= start {
                0.0
            } else if distance_m >= end {
                1.0
            } else {
                ((distance_m - start) / zone).clamp(0.0, 1.0)
            }
        };
        let lane = |bit: u8| f32::from((mask >> bit) & 1);
        let coarser = [lane(0), lane(1), lane(2), lane(3)];
        let finer = [lane(4), lane(5), lane(6), lane(7)];
        let dot = |a: [f32; 4], b: [f32; 4]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
        if dot(flags, coarser) > 0.5 {
            f = 1.0;
        } else if dot(flags, finer) > 0.5 {
            f = 0.0;
        }
        f
    }

    #[test]
    fn ground_shader_agrees_with_core() {
        // Review item 3 (#777): the shader and core run one rule. The WGSL
        // keeps the core constants and tokens, and its formula (mirrored
        // above in `f32`) gives the core factor on sample distances, errors
        // and masks, the mask read from the tile (#920 T4).
        let shader = include_str!("patch_grid_common.wgsl");
        for token in [
            "0.3",
            "morph_err",
            "t.coarser",
            "t.finer",
            "mask >> 4u",
            "parent_position",
        ] {
            assert!(
                shader.contains(token),
                "patch_grid_common.wgsl must contain {token:?}"
            );
        }
        let main = include_str!("patch_grid.wgsl");
        for token in ["mix(", "view.world_position", "grid_factor("] {
            assert!(
                main.contains(token),
                "patch_grid.wgsl must contain {token:?}"
            );
        }
        let detail = 1.5;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: test band scale narrowed for the GPU, intended"
        )]
        let band_scale = (focal_px() / detail) as f32;
        let err = 2.5f64;
        let end = morph_end_m(err, detail);
        for mask in [
            0u8,
            morph_mask(1, 0),
            morph_mask(0, 1),
            morph_mask(0b1010, 0b0101),
        ] {
            for edge in [0u8, 1, 2, 4, 8] {
                let lane = |bit: u8| f32::from((edge >> bit) & 1);
                let flags = [lane(0), lane(1), lane(2), lane(3)];
                for share in [0.1, 0.7, 0.8, 0.85, 0.95, 1.0, 1.5] {
                    let d = end * share;
                    let want = vertex_factor(d, err, detail, edge, mask);
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "E-CAST: test inputs narrowed for the GPU, intended"
                    )]
                    let got = shader_factor(d as f32, err as f32, band_scale, flags, mask);
                    assert!(
                        (f64::from(got) - want).abs() < 1e-4,
                        "mask {mask:#x} edge {edge} at {share}: shader {got}, core {want}"
                    );
                }
            }
        }
        assert_eq!(morph_factor(end, end), 1.0);
    }

    #[test]
    fn the_shader_uniform_is_the_ground_scale() {
        // #920 T6: the WGSL `PatchGrid` struct binds `GroundScale` (one
        // vec4); a field left over in either fails pipeline creation at
        // run time (the uniform's size no longer matches).
        let shader = include_str!("patch_grid_common.wgsl");
        let start = shader.find("struct PatchGrid {").expect("struct");
        let body = &shader[start..];
        let body = &body[..body.find("};").expect("end")];
        let fields = body
            .lines()
            .filter(|line| line.contains(": vec4<f32>"))
            .count();
        assert_eq!(fields, 1, "one vec4 field: {body}");
        assert!(body.contains("scale: vec4<f32>"), "{body}");
        assert_eq!(size_of::<super::GroundScale>(), 16);
    }
}
