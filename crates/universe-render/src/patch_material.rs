//! LOD patch morph material (rebuild step 4, #777).
//!
//! `PlanetPatchMaterial = ExtendedMaterial<StandardMaterial, PatchMorph>`
//! with embedded WGSL vertex shader, the pattern of ADR 0027 (#626). The
//! base stays white and unlit; per-vertex colours carry the height tint
//! (#799). The extension blends each vertex between its own position and
//! its parent-level position (and normals) by its own distance to the
//! camera (CDLOD, Q2 A, Q6 A, Q7 A, core `lod_morph::vertex_factor`),
//! pinning or holding edges by the morph mask (Q3 A).
//!
//! Nothing changes per patch per frame (#777 review item 4): the distance
//! comes from the vertex and view positions in the shader, the band from
//! the per-vertex morph error, and the mask picks one of the shared
//! materials in [`PatchMaterials`] (one per mask value, made on first use).
//! Only a Planet detail change rewrites their band scale. Pure morph math
//! lives in core; this module only wires Bevy assets.

use bevy::mesh::{Mesh, MeshVertexAttribute, MeshVertexBufferLayoutRef};
use bevy::pbr::{
    ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
    MaterialPlugin,
};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError, VertexFormat,
};
use bevy::shader::ShaderRef;
use std::collections::HashMap;
use universe_core::lod_morph::{mask_coarser, mask_finer};

/// Patch material: white unlit base plus the morph extension.
pub(crate) type PlanetPatchMaterial = ExtendedMaterial<StandardMaterial, PatchMorph>;

/// Embedded path of the morph vertex shader.
const SHADER: &str = "embedded://universe_render/patch_morph.wgsl";

// High random ids for custom attributes (avoid collisions, like the
// Bevy custom-attribute example).
/// Parent-level position (same cell units as `ATTRIBUTE_POSITION`).
pub(crate) const ATTRIBUTE_PARENT_POSITION: MeshVertexAttribute =
    MeshVertexAttribute::new("ParentPosition", 989543917, VertexFormat::Float32x3);
/// Parent-level normal (unit).
pub(crate) const ATTRIBUTE_PARENT_NORMAL: MeshVertexAttribute =
    MeshVertexAttribute::new("ParentNormal", 989543918, VertexFormat::Float32x3);
/// Edge flags for odd edge vertices: East, West, North, South as 0/1.
pub(crate) const ATTRIBUTE_EDGE_FLAGS: MeshVertexAttribute =
    MeshVertexAttribute::new("EdgeFlags", 989543919, VertexFormat::Float32x4);
/// Morph error in metres (`PatchMesh::morph_err_m`): the band end is
/// `err x focal / detail`.
pub(crate) const ATTRIBUTE_MORPH_ERR: MeshVertexAttribute =
    MeshVertexAttribute::new("MorphErr", 989543920, VertexFormat::Float32);

/// Morph inputs for the shader, shared by every patch with one mask.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType, Reflect)]
pub(crate) struct PatchMorphUniform {
    /// Edges touching a coarser neighbour as 0/1: East, West, North, South.
    pub(crate) coarser: Vec4,
    /// Edges touching a finer neighbour as 0/1: East, West, North, South.
    pub(crate) finer: Vec4,
    /// `x`: metres per world unit, `y`: `focal_px / detail_px`.
    pub(crate) scale: Vec4,
}

/// The morph extension of the patch material.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub(crate) struct PatchMorph {
    /// Everything the vertex shader reads.
    #[uniform(100)]
    pub(crate) params: PatchMorphUniform,
}

impl MaterialExtension for PatchMorph {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Forward locations: 0 position, 1 normal, 5 colour; custom at
        // 17-20 (the grass article's pattern).
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(5),
            ATTRIBUTE_PARENT_POSITION.at_shader_location(17),
            ATTRIBUTE_PARENT_NORMAL.at_shader_location(18),
            ATTRIBUTE_EDGE_FLAGS.at_shader_location(19),
            ATTRIBUTE_MORPH_ERR.at_shader_location(20),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

/// Edge set (East, West, North, South bits) as 0/1 lanes.
fn lanes(edges: u8) -> Vec4 {
    let bit = |b: u8| {
        if edges & (1 << b) != 0 { 1.0 } else { 0.0 }
    };
    Vec4::new(bit(0), bit(1), bit(2), bit(3))
}

/// The morph uniform for one mask and band scale.
///
/// `metres_per_unit` turns world distances into metres; `band_scale` is
/// `focal_px / detail_px`, so `err x band_scale` is the band end core uses.
#[must_use]
pub(crate) fn morph_uniform(mask: u8, metres_per_unit: f32, band_scale: f32) -> PatchMorphUniform {
    PatchMorphUniform {
        coarser: lanes(mask_coarser(mask)),
        finer: lanes(mask_finer(mask)),
        scale: Vec4::new(metres_per_unit, band_scale, 0.0, 0.0),
    }
}

/// One patch material for `mask` at the given scale.
#[must_use]
pub(crate) fn patch_material(
    mask: u8,
    metres_per_unit: f32,
    band_scale: f32,
) -> PlanetPatchMaterial {
    ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            cull_mode: None,
            ..default()
        },
        extension: PatchMorph {
            params: morph_uniform(mask, metres_per_unit, band_scale),
        },
    }
}

/// Shared patch materials, one per morph mask (#777 review item 4).
///
/// Every patch with the same mask uses the same handle, so patches batch
/// and no material is touched per frame. `scale` is the current
/// `(metres_per_unit, focal / detail)`; [`PatchMaterials::set_scale`]
/// rewrites every material only when it changes.
#[derive(Resource, Debug, Default)]
pub(crate) struct PatchMaterials {
    /// Handle per mask value, made on first use.
    pub(crate) handles: HashMap<u8, Handle<PlanetPatchMaterial>>,
    /// Scale the handles were made or last updated with.
    pub(crate) scale: (f32, f32),
}

impl PatchMaterials {
    /// The shared handle for `mask`, made on first use.
    pub(crate) fn handle(
        &mut self,
        mask: u8,
        materials: &mut Assets<PlanetPatchMaterial>,
    ) -> Handle<PlanetPatchMaterial> {
        let (mpu, band) = self.scale;
        self.handles
            .entry(mask)
            .or_insert_with(|| materials.add(patch_material(mask, mpu, band)))
            .clone()
    }

    /// Updates the band scale on every shared material when it changed.
    pub(crate) fn set_scale(
        &mut self,
        metres_per_unit: f32,
        band_scale: f32,
        materials: &mut Assets<PlanetPatchMaterial>,
    ) {
        if self.scale == (metres_per_unit, band_scale) {
            return;
        }
        self.scale = (metres_per_unit, band_scale);
        for (mask, handle) in &self.handles {
            if let Some(mut material) = materials.get_mut(handle) {
                material.extension.params = morph_uniform(*mask, metres_per_unit, band_scale);
            }
        }
    }
}

/// Largest mesh slab (bytes). Bevy packs meshes into shared GPU slabs that
/// grow by 1.5x up to 512 MiB, copying the whole slab on each growth; with
/// the morph attributes (#777, about 84 bytes a vertex) that copy froze
/// single frames for 190-380 ms near the ground on the dev PC. Capped
/// slabs start a new one instead, so a growth copies at most this much.
const MESH_SLAB_MAX_BYTES: u64 = 16 * 1024 * 1024;

/// Registers the patch material and its embedded shader (window app).
pub struct PatchMaterialPlugin;

impl Plugin for PatchMaterialPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "patch_morph.wgsl");
        app.add_plugins(MaterialPlugin::<PlanetPatchMaterial>::default());
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
    use super::*;
    use universe_core::lod_morph::{morph_end_m, morph_factor, morph_mask, vertex_factor};
    use universe_core::precision::focal_px;

    /// The WGSL `morph_factor` and edge rule, run on the CPU in `f32`.
    fn shader_factor(
        distance_m: f32,
        err: f32,
        band_scale: f32,
        flags: Vec4,
        u: &PatchMorphUniform,
    ) -> f32 {
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
        if flags.dot(u.coarser) > 0.5 {
            f = 1.0;
        } else if flags.dot(u.finer) > 0.5 {
            f = 0.0;
        }
        f
    }

    #[test]
    fn morph_material_agrees_with_core() {
        // Review item 3: the shader and core run one rule. The WGSL keeps
        // the core constants and tokens, and its formula (mirrored above in
        // `f32`) gives the core factor on sample distances, errors and masks.
        let shader = include_str!("patch_morph.wgsl");
        for token in [
            "mix(",
            "0.3",
            "view.world_position",
            "morph_err",
            "patch_morph.coarser",
            "patch_morph.finer",
            "parent_position",
        ] {
            assert!(
                shader.contains(token),
                "patch_morph.wgsl must contain {token:?}"
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
            let u = morph_uniform(mask, 1.0, band_scale);
            for edge in [0u8, 1, 2, 4, 8] {
                let flags = lanes(edge);
                for share in [0.1, 0.7, 0.8, 0.85, 0.95, 1.0, 1.5] {
                    let d = end * share;
                    let want = vertex_factor(d, err, detail, edge, mask);
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "E-CAST: test inputs narrowed for the GPU, intended"
                    )]
                    let got = shader_factor(d as f32, err as f32, band_scale, flags, &u);
                    assert!(
                        (f64::from(got) - want).abs() < 1e-4,
                        "mask {mask:#x} edge {edge} at {share}: shader {got}, core {want}"
                    );
                }
            }
        }
        assert_eq!(morph_factor(end, end), 1.0);
        // Mask nibbles land in East, West, North, South lanes.
        let u = morph_uniform(morph_mask(0b0010, 0b1000), 2.0, 3.0);
        assert_eq!(u.coarser, Vec4::new(0.0, 1.0, 0.0, 0.0));
        assert_eq!(u.finer, Vec4::new(0.0, 0.0, 0.0, 1.0));
        assert_eq!(u.scale, Vec4::new(2.0, 3.0, 0.0, 0.0));
    }
}
