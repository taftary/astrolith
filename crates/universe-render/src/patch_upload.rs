//! Production tile extraction/queue writes, with optional scalar witnesses.
//! Pending writes keep their existing retargeting behavior; observations do not
//! discard stale work or change when the texture becomes ready.

use crate::loading_diagnostic::{LoadingDiagnostics, Producer};
use crate::patch_grid::{GridAtlas, GridWrite, tile_origin};
use crate::resource_diagnostic::ResourceEvent;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    Extent3d, Origin3d, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect,
};
use bevy::render::renderer::RenderQueue;
use bevy::render::texture::GpuImage;
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderSystems};
use universe_core::grid_tile::{MASK_TEXEL, TILE_H, TILE_W};

#[derive(Resource, Debug, Default)]
struct RenderTileWrites {
    image: Option<AssetId<Image>>,
    pending: Vec<GridWrite>,
    witness: Option<Producer>,
    extracted_frame: u64,
}

fn extract_tile_writes(
    atlas: Extract<Option<Res<GridAtlas>>>,
    diagnostics: Extract<Option<Res<LoadingDiagnostics>>>,
    mut writes: ResMut<RenderTileWrites>,
) {
    if let Some(atlas) = atlas.as_ref() {
        let previous_image = writes.image;
        let before = writes.pending.len();
        writes.image = Some(atlas.image.id());
        writes.pending.extend(atlas.writes.iter().cloned());
        writes.witness.clone_from(&atlas.witness);
        if let (Some(witness), Some(diagnostics)) = (&writes.witness, diagnostics.as_ref()) {
            let frame = diagnostics.frame();
            witness.resource(ResourceEvent::ExtractWrites {
                extracted_frame: frame,
                image: atlas.image.id(),
                previous_image,
                before,
                added: atlas.writes.len(),
                pending_bytes: writes.pending.iter().map(GridWrite::bytes).sum(),
            });
            writes.extracted_frame = frame;
        }
    }
}

fn write_tiles(
    mut writes: ResMut<RenderTileWrites>,
    images: Res<RenderAssets<GpuImage>>,
    queue: Res<RenderQueue>,
) {
    let Some(gpu) = writes.image.and_then(|id| images.get(id)) else {
        return;
    };
    let started = writes.witness.as_ref().map(|_| std::time::Instant::now());
    let count = writes.pending.len();
    let mut submitted_bytes = 0;
    let texture = gpu.texture.clone();
    for write in writes.pending.drain(..) {
        let (tile, at, size, bytes) = match write {
            GridWrite::Tile(tile, texels) => {
                let mut bytes = Vec::with_capacity(texels.len() * 4);
                for value in texels.iter() {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
                (tile, [0, 0], [TILE_W, TILE_H], bytes)
            }
            GridWrite::Mask(tile, mask) => (
                tile,
                MASK_TEXEL,
                [1, 1],
                u32::from(mask).to_le_bytes().to_vec(),
            ),
        };
        submitted_bytes += bytes.len();
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: {
                    let (page, x, y) = tile_origin(tile);
                    Origin3d {
                        x: x + at[0],
                        y: y + at[1],
                        z: page,
                    }
                },
                aspect: TextureAspect::All,
            },
            &bytes,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size[0] * 4),
                rows_per_image: Some(size[1]),
            },
            Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
        );
    }
    if let (Some(witness), Some(started), Some(image)) = (&writes.witness, started, writes.image) {
        witness.resource(ResourceEvent::SubmitWrites {
            extracted_frame: writes.extracted_frame,
            image,
            writes: count,
            bytes: submitted_bytes,
            elapsed_ns: u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        });
    }
}

pub(crate) fn install(app: &mut App) {
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render
            .init_resource::<RenderTileWrites>()
            .add_systems(ExtractSchedule, extract_tile_writes)
            .add_systems(Render, write_tiles.in_set(RenderSystems::PrepareResources));
    }
}
