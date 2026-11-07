//! Render-store checkpoints, separate from main handles and physical GPU release.

use crate::loading_diagnostic::{LoadingDiagnostics, LoadingEvent, Producer};
use crate::patch_grid::GridAtlas;
use crate::resource_diagnostic::{AssetKey, ResourceEvent, Role};
use bevy::pbr::PreparedMaterial;
use bevy::prelude::*;
use bevy::render::erased_render_asset::ErasedRenderAssets;
use bevy::render::mesh::{RenderMesh, RenderMeshBufferInfo};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{IndexFormat, TextureFormat};
use bevy::render::texture::GpuImage;
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderSystems};

#[derive(Resource, Default)]
struct Stamp {
    frame: u64,
    context: Option<((u64, u64, u8), Producer)>,
    atlas: Option<AssetId<Image>>,
}

fn extract(
    diagnostics: Extract<Res<LoadingDiagnostics>>,
    universe: Extract<Res<crate::Universe>>,
    atlas: Extract<Option<Res<GridAtlas>>>,
    mut stamp: ResMut<Stamp>,
) {
    stamp.frame = diagnostics.frame();
    stamp.atlas = atlas.as_ref().map(|a| a.image.id());
    let key = (
        universe.root.0,
        universe.open_seed(),
        universe.level().get(),
    );
    if stamp.context.as_ref().is_none_or(|(held, _)| *held != key) {
        stamp.context = Some((key, diagnostics.context(&universe)));
    }
}

fn observe(
    diagnostics: Res<LoadingDiagnostics>,
    stamp: Res<Stamp>,
    meshes: Option<Res<RenderAssets<RenderMesh>>>,
    images: Option<Res<RenderAssets<GpuImage>>>,
    materials: Option<Res<ErasedRenderAssets<PreparedMaterial>>>,
) {
    let (Some(meshes), Some(images), Some(materials)) = (meshes, images, materials) else {
        diagnostics.resource_problem("required render-world asset store unavailable");
        return;
    };
    if let Ok(mut registry) = diagnostics.0.resources.lock() {
        let mut resident = 0;
        let (mut active, mut outgoing, mut shared) = (0, 0, 0);
        let mut problem = None;
        for entry in &mut registry.entries {
            let present = match entry.key {
                AssetKey::Image(id) => images.get(id).is_some(),
                AssetKey::Mesh(id) => meshes.get(id).is_some(),
                AssetKey::Standard(id) => materials.get(id).is_some(),
                AssetKey::Ground(id) => materials.get(id).is_some(),
            };
            resident += usize::from(present);
            if present {
                match entry.role {
                    Role::MoonProxy(_)
                    | Role::MoonMaterial
                    | Role::ParentProxy
                    | Role::ParentMaterial => {
                        if entry.source.host.is_some()
                            && entry.source.host
                                == stamp.context.as_ref().and_then(|(_, p)| p.source.host)
                        {
                            active += 1;
                        } else {
                            outgoing += 1;
                        }
                    }
                    Role::SharedAtlas if Some(entry.key) != stamp.atlas.map(AssetKey::Image) => {
                        outgoing += 1
                    }
                    _ => shared += 1,
                }
            }
            if entry.render != Some(present) {
                diagnostics.emit(
                    entry.source,
                    LoadingEvent::Resource(ResourceEvent::RenderState {
                        id: entry.id,
                        extracted_frame: stamp.frame,
                        present,
                        previously_resident: entry.was_resident,
                    }),
                );
                entry.render = Some(present);
                if let AssetKey::Mesh(id) = entry.key
                    && let Some(mesh) = meshes.get(id)
                {
                    let index_bytes = match mesh.buffer_info {
                        RenderMeshBufferInfo::NonIndexed => 0,
                        RenderMeshBufferInfo::Indexed {
                            count,
                            index_format,
                        } => {
                            u64::from(count)
                                * match index_format {
                                    IndexFormat::Uint16 => 2,
                                    IndexFormat::Uint32 => 4,
                                }
                        }
                    };
                    diagnostics.emit(
                        entry.source,
                        LoadingEvent::Resource(ResourceEvent::RenderMesh {
                            id: entry.id,
                            extracted_frame: stamp.frame,
                            vertices: mesh.vertex_count,
                            stride: mesh.layout.0.layout().array_stride,
                            index_bytes,
                        }),
                    );
                }
                if entry.role == Role::SharedAtlas
                    && let AssetKey::Image(id) = entry.key
                    && let Some(image) = images.get(id)
                {
                    diagnostics.emit(
                        entry.source,
                        LoadingEvent::Resource(ResourceEvent::RenderAtlas {
                            id: entry.id,
                            extracted_frame: stamp.frame,
                            width: image.texture.width(),
                            height: image.texture.height(),
                            layers: image.texture.depth_or_array_layers(),
                            mips: image.texture.mip_level_count(),
                            samples: image.texture.sample_count(),
                        }),
                    );
                    if image.texture.format() != TextureFormat::R32Uint {
                        // Required schema cannot silently assign R32Uint's size
                        // to another actual format. Keep evidence and fail it.
                        problem = Some("observed atlas format is not the declared R32Uint");
                    }
                }
            }
            entry.was_resident |= present;
        }
        if let Some(problem) = problem {
            registry.problem.get_or_insert_with(|| problem.to_string());
        }
        // The summary labels the actual extracted scene; per-resource events
        // retain their original creation/body identity across transitions.
        if let Some((_, producer)) = &stamp.context {
            diagnostics.emit(
                producer.source,
                LoadingEvent::Resource(ResourceEvent::RenderFrame {
                    extracted_frame: stamp.frame,
                    tracked_resident: resident,
                    meshes: meshes.iter().count(),
                    images: images.iter().count(),
                    materials: materials.iter().count(),
                    active,
                    outgoing,
                    shared,
                    current_atlas: stamp.atlas,
                }),
            );
        }
    }
}

pub(super) fn install(app: &mut App) {
    let diagnostics = app.world().resource::<LoadingDiagnostics>().clone();
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render
            .insert_resource(diagnostics)
            .init_resource::<Stamp>()
            .add_systems(ExtractSchedule, extract)
            .add_systems(
                Render,
                observe
                    .after(RenderSystems::PrepareAssets)
                    .before(RenderSystems::PrepareResources),
            );
    }
}
