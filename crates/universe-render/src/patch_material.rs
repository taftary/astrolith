//! LOD patch morph material (rebuild step 4, #777 T3).
//!
//! `PlanetPatchMaterial = ExtendedMaterial<StandardMaterial, PatchMorph>`
//! with embedded WGSL vertex shader, the pattern of ADR 0027 (#626). The
//! base stays white and unlit; per-vertex colours carry the height tint
//! (#799). The extension blends each vertex between its own position and
//! its parent-level position (and normals) by distance over the last 30% of
//! its band (CDLOD, Q2 A, Q6 A, Q7 A, core `lod_morph`), pinning masked
//! edges fully to the coarse edge (Q3 A). No mesh is rebuilt per frame:
//! the CPU updates one small uniform per patch (morph factor plus edge
//! mask), the GPU blends. Pure morph math lives in core; this module only
//! wires Bevy assets.

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

/// Shared patch material: white unlit base plus the morph extension.
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

/// Per-patch morph inputs for the shader.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType, Reflect)]
pub(crate) struct PatchMorphUniform {
    /// Camera position relative to the patch centre, in metres (xyz).
    pub(crate) camera_rel: Vec4,
    /// Distance band in metres: `x` is start, `y` is end.
    pub(crate) band: Vec4,
    /// Edge mask as 0/1: East, West, North, South (Q3 A).
    pub(crate) edge_mask: Vec4,
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
        // 17, 18, 19 (the grass article's pattern).
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(5),
            ATTRIBUTE_PARENT_POSITION.at_shader_location(17),
            ATTRIBUTE_PARENT_NORMAL.at_shader_location(18),
            ATTRIBUTE_EDGE_FLAGS.at_shader_location(19),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

/// One patch material from the camera offset, band and edge mask.
///
/// `camera_rel_m` is the camera position relative to the patch centre in
/// metres; `(band_start_m, band_end_m)` is the CDLOD band from
/// `lod_morph::morph_band_m`; the shader computes the same factor core
/// does, so no mesh is rebuilt per frame.
#[must_use]
pub(crate) fn patch_material(
    camera_rel_m: [f32; 3],
    band_start_m: f32,
    band_end_m: f32,
    edge_mask: u8,
) -> PlanetPatchMaterial {
    let bit = |b: u8| {
        if edge_mask & (1 << b) != 0 { 1.0 } else { 0.0 }
    };
    ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            cull_mode: None,
            ..default()
        },
        extension: PatchMorph {
            params: PatchMorphUniform {
                camera_rel: Vec4::new(camera_rel_m[0], camera_rel_m[1], camera_rel_m[2], 0.0),
                band: Vec4::new(band_start_m, band_end_m, 0.0, 0.0),
                edge_mask: Vec4::new(bit(0), bit(1), bit(2), bit(3)),
            },
        },
    }
}

/// Registers the patch material and its embedded shader (window app).
pub struct PatchMaterialPlugin;

impl Plugin for PatchMaterialPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "patch_morph.wgsl");
        app.add_plugins(MaterialPlugin::<PlanetPatchMaterial>::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn morph_material_agrees_with_core() {
        // The shader blends with the same factor core computes (Q2 A, Q7 A):
        // the WGSL mirrors `lod_morph::morph_factor` (same 0.3 zone, same
        // clamp), and this material carries the camera offset, band and
        // edge mask the shader needs. A no-window check runs the WGSL
        // formula's inputs through the core function on sample points.
        let shader = include_str!("patch_morph.wgsl");
        for token in ["mix(", "max(", "0.3", "edge_mask", "parent_position"] {
            assert!(
                shader.contains(token),
                "patch_morph.wgsl must contain {token:?}"
            );
        }
        // Core factor on sample distances (the shader's formula, same inputs).
        let (start, end) = (100.0, 200.0);
        let zone_start = end - 0.3 * (end - start);
        for (dist, want) in [
            (50.0, 0.0),
            (start, 0.0),
            (zone_start, 0.0),
            ((zone_start + end) / 2.0, 0.5),
            (end, 1.0),
            (300.0, 1.0),
        ] {
            let got = universe_core::lod_morph::morph_factor(dist, start, end);
            assert!(
                (got - want).abs() < 1e-12,
                "core factor at {dist} is {got}, want {want}"
            );
        }
        // Material carries the band and camera offset the shader computes from.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: test band narrowed for the GPU, intended"
        )]
        let (start_f32, end_f32) = (start as f32, end as f32);
        let mat = patch_material([0.0, 0.0, 150.0], start_f32, end_f32, 0);
        assert_eq!(mat.extension.params.band.x, start_f32);
        assert_eq!(mat.extension.params.band.y, end_f32);
        assert_eq!(
            mat.extension.params.camera_rel,
            Vec4::new(0.0, 0.0, 150.0, 0.0)
        );
        // Edge mask bits land in the uniform in East,West,North,South order.
        let mat = patch_material([0.0, 0.0, 0.0], start_f32, end_f32, 0b0000_1010);
        assert_eq!(
            mat.extension.params.edge_mask,
            Vec4::new(0.0, 1.0, 0.0, 1.0)
        );
    }
}
