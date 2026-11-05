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

/// How far the fine close detail reaches from the camera, by Terrain
/// quality (S1, #1161): Low the least and the cheapest.
#[must_use]
pub(crate) fn detail_reach_m(tier: universe_core::settings::Tier) -> f32 {
    match tier {
        universe_core::settings::Tier::Low => 150.0,
        universe_core::settings::Tier::Normal => 300.0,
        universe_core::settings::Tier::High => 600.0,
    }
}

/// Strength of a lava world's own light (#1161 T5), in the game's light
/// units: 1 shows the glow colour as bright as sunlit ground at noon, so
/// 2.5 reads clearly at night and still shows by day.
pub(crate) const LAVA_GLOW: f32 = 2.5;

/// One world's ground look for the fragment shader (#1161 T4): its type's
/// palette in linear colour, and the numbers the bands read. Written once
/// per planet or Terrain quality change, never per frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType, Reflect)]
pub(crate) struct GroundLook {
    /// Low, flat ground (`w`: 1 when a palette is set, 0 for the plain tint).
    pub(crate) low: Vec4,
    /// High ground.
    pub(crate) high: Vec4,
    /// Steep slopes.
    pub(crate) steep: Vec4,
    /// Caps (`w`: how far the polar caps reach, 0 to 1).
    pub(crate) cap: Vec4,
    /// Own light of cracks and low lava fields (`w`: its strength,
    /// [`LAVA_GLOW`] on lava worlds, 0 elsewhere).
    pub(crate) glow: Vec4,
    /// `x`: planet radius (m), `y`: relief scale, `z`: close detail reach
    /// (m), `w`: crater rims and floors shaded (1 airless, 0 with air).
    pub(crate) bands: Vec4,
}

/// `srgb` (0..1) as a linear `Vec4` with `w`.
fn linear(srgb: [f64; 3], w: f32) -> Vec4 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: colour channels narrowed for the GPU, intended"
    )]
    let c = Color::srgb(srgb[0] as f32, srgb[1] as f32, srgb[2] as f32).to_linear();
    Vec4::new(c.red, c.green, c.blue, w)
}

impl GroundLook {
    /// The look of a world of `kind` with `relief`, at Terrain `tier`.
    #[must_use]
    pub(crate) fn for_world(
        kind: universe_core::world_type::WorldType,
        relief: &universe_core::relief::Relief,
        tier: universe_core::settings::Tier,
    ) -> GroundLook {
        let p = kind.palette();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: palette and band values narrowed for the GPU, intended"
        )]
        let (cap_reach, glow, radius, scale) = (
            p.cap_reach as f32,
            if p.glow.iter().any(|&c| c > 0.0) {
                LAVA_GLOW
            } else {
                0.0
            },
            relief.radius_m() as f32,
            relief.relief_scale() as f32,
        );
        GroundLook {
            low: linear(p.low, 1.0),
            high: linear(p.high, 0.0),
            steep: linear(p.steep, 0.0),
            cap: linear(p.cap, cap_reach),
            glow: linear(p.glow, glow),
            bands: Vec4::new(
                radius,
                scale,
                detail_reach_m(tier),
                if relief.is_cratered() { 1.0 } else { 0.0 },
            ),
        }
    }
}

/// The ground's own light (#1160 Q5 A, AC5): none yet. The light inputs stay
/// separate: surface colour (`base_color`), sunlight and moonlight (the
/// directional lights), the even fill (the ambient light), and this own
/// light (`emissive`). A later glow (lava, life) sets only this one.
pub(crate) const GROUND_OWN_LIGHT: LinearRgba = LinearRgba::BLACK;

/// The ground's lit base: the one ground colour (#892 T2, Q9 C; #918 T4),
/// matte, one-sided, with no own light ([`GROUND_OWN_LIGHT`]).
#[must_use]
pub(crate) fn ground_base() -> StandardMaterial {
    StandardMaterial {
        base_color: crate::tokens::RELIEF_GROUND_TINT,
        emissive: GROUND_OWN_LIGHT,
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
    #[test]
    fn the_ground_keeps_its_light_inputs_separate() {
        // #1160 AC5: colour, own light and the lights are separate inputs;
        // the own light is black for now, so the sun and moons alone light it.
        let base = super::ground_base();
        assert_eq!(base.emissive, super::GROUND_OWN_LIGHT);
        assert_eq!(base.emissive, bevy::color::LinearRgba::BLACK);
        assert_eq!(base.base_color, crate::tokens::RELIEF_GROUND_TINT);
        assert!(!base.unlit, "lit by the sun, the moons and the fill");
    }

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

#[cfg(test)]
#[path = "ground_look_tests.rs"]
mod ground_look_tests;
