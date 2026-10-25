//! Ground patches from one shared grid (#920).
//!
//! Every ground patch draws one of two shared meshes (the 33 x 33 grid
//! plus skirts, one per triangle winding) and finds its shape in one tile
//! of a shared heights texture through its [`MeshTag`]. The vertex shaders
//! (`patch_grid*.wgsl`) rebuild the offsets, normals and parent shape from
//! that tile as `universe_core::grid_rebuild` does. Tiles are packed on
//! the worker threads ([`GridInbox`]) and written into the texture by the
//! render world, one tile at a time (no whole-texture upload). A patch's
//! graphics memory is its tile (about 10 KB) instead of its own mesh
//! (about 72 KB before #920).

use crate::patch_material::GroundScale;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, MeshTag, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{
    ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
    MaterialPlugin,
};
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, Origin3d, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureDimension, TextureFormat,
    TextureViewDescriptor, TextureViewDimension,
};
use bevy::render::renderer::RenderQueue;
use bevy::render::texture::GpuImage;
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderSystems};
use bevy::shader::ShaderRef;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use universe_core::grid_tile::{MASK_TEXEL, TILE_H, TILE_W};
use universe_core::lod::PatchKey;

/// Side of one page of the heights texture, in texels: phone-safe (GLES3
/// guarantees 2,048 and 256 layers; #920 Q5 A).
const PAGE: u32 = 1024;

/// Tiles across one page.
const PAGE_COLS: u32 = PAGE / TILE_W;

/// Tiles on one page: 27 x 16 = 432.
const PAGE_TILES: u32 = PAGE_COLS * (PAGE / TILE_H);

/// Bytes of one tile on the GPU (32-bit texels).
pub(crate) const TILE_BYTES: usize = (TILE_W * TILE_H * 4) as usize;

/// Pages the heights texture needs under a Ground memory limit of
/// `cap_bytes`: room for a tile per patch the limit can hold (#920 T6).
#[must_use]
pub(crate) fn pages_for(cap_bytes: usize) -> u32 {
    let tiles = u32::try_from(cap_bytes / TILE_BYTES).unwrap_or(u32::MAX);
    tiles.div_ceil(PAGE_TILES).max(1)
}

/// Page and texel origin of `tile` in the heights texture (as
/// `patch_grid_common::tile_of` works it out).
#[must_use]
fn tile_origin(tile: u32) -> (u32, u32, u32) {
    let (page, slot) = (tile / PAGE_TILES, tile % PAGE_TILES);
    (
        page,
        (slot % PAGE_COLS) * TILE_W,
        (slot / PAGE_COLS) * TILE_H,
    )
}

/// The heights texture with `pages` layers (contents unset: every tile is
/// written before a patch draws it).
fn heights_image(pages: u32) -> Image {
    let mut image = Image::new_uninit(
        Extent3d {
            width: PAGE,
            height: PAGE,
            depth_or_array_layers: pages,
        },
        TextureDimension::D2,
        TextureFormat::R32Uint,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    });
    image
}

/// Embedded path of the forward vertex shader.
const SHADER: &str = "embedded://universe_render/patch_grid.wgsl";

/// Embedded path of the shadow-pass vertex shader.
const PREPASS_SHADER: &str = "embedded://universe_render/patch_grid_prepass.wgsl";

/// Embedded path of the shared rebuild module both shaders import.
const COMMON_SHADER: &str = "embedded://universe_render/patch_grid_common.wgsl";

/// The ground's material: the lit ground base plus the grid extension.
pub(crate) type PatchGridMaterial = ExtendedMaterial<StandardMaterial, PatchGrid>;

/// The grid extension: the band scale, plus the heights texture.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub(crate) struct PatchGrid {
    /// The band scale every ground material shares.
    #[uniform(100)]
    pub(crate) params: GroundScale,
    /// The shared heights texture (read with `textureLoad`).
    #[texture(101, sample_type = "u_int", dimension = "2d_array")]
    pub(crate) heights: Handle<Image>,
}

impl MaterialExtension for PatchGrid {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }

    fn prepass_vertex_shader() -> ShaderRef {
        PREPASS_SHADER.into()
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        true
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

/// Grid materials, one per patch level (deeper levels share the finest).
/// The GPU draws material batches in the order their materials were
/// prepared, so the materials are made one a frame, finest (nearest)
/// level first: near ground is drawn first and hides what lies behind it
/// (#920 T4: one shared batch drew in a run-dependent order and shaded up
/// to 50% more pixels; materials made in one frame are prepared in hash
/// order). A later scale change keeps each material's place, since Bevy
/// frees and re-takes the same slot.
const LEVEL_MATERIALS: usize = 32;

/// Frames before the first material, so the heights texture is on the GPU
/// and no material has to wait a frame (which would change its place).
const MATERIAL_DELAY_FRAMES: u32 = 4;

/// Makes one grid material a frame (`LEVEL_MATERIALS`); once all exist,
/// moves every patch drawn meanwhile onto its own level's material.
fn grow_materials(
    mut commands: Commands,
    atlas: Option<ResMut<GridAtlas>>,
    materials: Option<ResMut<Assets<PatchGridMaterial>>>,
    pieces: Query<(
        Entity,
        &crate::patches::PatchPiece,
        &MeshMaterial3d<PatchGridMaterial>,
    )>,
    mut frames: Local<u32>,
) {
    let (Some(mut atlas), Some(mut materials)) = (atlas, materials) else {
        return;
    };
    if atlas.materials.len() >= LEVEL_MATERIALS {
        return;
    }
    *frames += 1;
    if *frames < MATERIAL_DELAY_FRAMES {
        return;
    }
    atlas.add_material(&mut materials);
    if atlas.materials.len() < LEVEL_MATERIALS {
        return;
    }
    for (entity, piece, held) in &pieces {
        let own = atlas.material(piece.0.level, &mut materials);
        if held.0 != own {
            commands.entity(entity).insert(MeshMaterial3d(own));
        }
    }
}

/// One write into the heights texture, applied in order.
#[derive(Debug, Clone)]
pub(crate) enum GridWrite {
    /// A whole tile.
    Tile(u32, Arc<Vec<u32>>),
    /// The morph mask texel of a tile.
    Mask(u32, u8),
}

/// Tiles packed on the worker threads, waiting for their patch to apply.
#[derive(Resource, Debug, Default, Clone)]
pub(crate) struct GridInbox(pub Arc<Mutex<HashMap<PatchKey, Vec<u32>>>>);

/// The heights texture, the shared meshes, the tile table and materials.
#[derive(Resource, Debug)]
pub(crate) struct GridAtlas {
    /// The shared heights texture.
    pub(crate) image: Handle<Image>,
    /// Shared meshes by winding `[left-handed, right-handed]`, made from
    /// the first patch built on such a face.
    meshes: [Option<Handle<Mesh>>; 2],
    /// The tile of each patch that has one.
    tiles: HashMap<PatchKey, u32>,
    /// Freed tiles, reused first.
    free: Vec<u32>,
    /// Next never-used tile.
    next: u32,
    /// Tile writes for the render world this frame, in order.
    pub(crate) writes: Vec<GridWrite>,
    /// One material per level, finest level first (`LEVEL_MATERIALS`).
    materials: Vec<Handle<PatchGridMaterial>>,
    /// Scale the materials were made or last updated with.
    scale: (f32, f32),
    /// The shared rebuild module, kept loaded for the imports.
    _common: Handle<Shader>,
    /// Pages (texture layers) of the heights texture.
    pub(crate) pages: u32,
    /// The texture ran out of tiles at least once.
    full: bool,
    /// Uploads that gave no GPU side (diagnostic).
    pub(crate) failed: u32,
    /// Drawn patches skipped for want of a tile (diagnostic).
    pub(crate) untiled: u32,
}

impl GridAtlas {
    /// The tile of `key`, if it has one.
    #[must_use]
    pub(crate) fn tile(&self, key: PatchKey) -> Option<u32> {
        self.tiles.get(&key).copied()
    }

    /// Gives `key` a tile holding `texels`; `None` when the texture is full.
    pub(crate) fn assign(&mut self, key: PatchKey, texels: Vec<u32>) -> Option<u32> {
        if let Some(tile) = self.tiles.get(&key).copied() {
            return Some(tile);
        }
        let tile = match self.free.pop() {
            Some(tile) => tile,
            None if self.next < PAGE_TILES * self.pages => {
                self.next += 1;
                self.next - 1
            }
            None => {
                self.full = true;
                return None;
            }
        };
        self.tiles.insert(key, tile);
        self.writes.push(GridWrite::Tile(tile, Arc::new(texels)));
        Some(tile)
    }

    /// Sets the morph mask of the patch in `tile` (`MASK_TEXEL`).
    pub(crate) fn set_mask(&mut self, tile: u32, mask: u8) {
        self.writes.push(GridWrite::Mask(tile, mask));
    }

    /// Frees the tile of `key`.
    pub(crate) fn release(&mut self, key: PatchKey) {
        if let Some(tile) = self.tiles.remove(&key) {
            self.free.push(tile);
        }
    }

    /// Remakes the heights texture with `pages` layers (the Ground memory
    /// limit changed): every tile goes, so the caller rebuilds the ground.
    /// The materials keep their places (Bevy re-prepares each in its own
    /// slot) and point at the new texture.
    pub(crate) fn resize(
        &mut self,
        pages: u32,
        images: Option<&mut Assets<Image>>,
        materials: &mut Assets<PatchGridMaterial>,
    ) {
        self.pages = pages;
        self.tiles.clear();
        self.free.clear();
        self.next = 0;
        self.writes.clear();
        self.full = false;
        let Some(images) = images else {
            return;
        };
        images.remove(&self.image);
        self.image = images.add(heights_image(pages));
        for handle in &self.materials {
            if let Some(mut material) = materials.get_mut(handle) {
                material.extension.heights = self.image.clone();
            }
        }
    }

    /// The shared mesh for `built`'s face winding, made from it on first
    /// use; `None` for a mesh with too many vertices for 16-bit indices.
    pub(crate) fn mesh(
        &mut self,
        built: &universe_core::lod_mesh::PatchMesh,
        meshes: &mut Assets<Mesh>,
    ) -> Option<Handle<Mesh>> {
        let right = universe_core::lod::face_grid_right_handed(built.key.face);
        let slot = self.meshes.get_mut(usize::from(right))?;
        if slot.is_none() {
            *slot = Some(meshes.add(shared_mesh(built)?));
        }
        slot.clone()
    }

    /// The material for patches of `level`. Materials are made one a
    /// frame, finest level first (`grow_materials`); until a level's
    /// exists, the coarsest one made so far stands in, and
    /// `grow_materials` rebinds every patch once all exist.
    pub(crate) fn material(
        &mut self,
        level: u32,
        materials: &mut Assets<PatchGridMaterial>,
    ) -> Handle<PatchGridMaterial> {
        if self.materials.is_empty() {
            self.add_material(materials);
        }
        let finest = LEVEL_MATERIALS - 1;
        let made = finest - usize::try_from(level).unwrap_or(usize::MAX).min(finest);
        self.materials
            .get(made.min(self.materials.len() - 1))
            .cloned()
            .unwrap_or_default()
    }

    /// Makes the next material, finer levels first.
    fn add_material(&mut self, materials: &mut Assets<PatchGridMaterial>) {
        let (mpu, band) = self.scale;
        self.materials.push(materials.add(ExtendedMaterial {
            base: crate::patch_material::ground_base(),
            extension: PatchGrid {
                params: GroundScale::new(mpu, band),
                heights: self.image.clone(),
            },
        }));
    }

    /// Updates the band scale on every material when it changed.
    pub(crate) fn set_scale(
        &mut self,
        metres_per_unit: f32,
        band_scale: f32,
        materials: &mut Assets<PatchGridMaterial>,
    ) {
        if self.scale == (metres_per_unit, band_scale) {
            return;
        }
        self.scale = (metres_per_unit, band_scale);
        for handle in &self.materials {
            if let Some(mut material) = materials.get_mut(handle) {
                material.extension.params = GroundScale::new(metres_per_unit, band_scale);
            }
        }
    }
}

/// One shared mesh: zero positions and normals (the shader rebuilds them)
/// and `built`'s main plus skirt triangles (every patch of a face with the
/// same winding has the same ones).
pub(crate) fn shared_mesh(built: &universe_core::lod_mesh::PatchMesh) -> Option<Mesh> {
    let total = built.total_vertex_count();
    let mut indices: Vec<u16> = Vec::with_capacity(built.indices.len() + built.skirt_indices.len());
    for index in built.indices.iter().chain(built.skirt_indices.iter()) {
        indices.push(u16::try_from(*index).ok()?);
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32; 3]; total]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; total]);
    mesh.insert_indices(Indices::U16(indices));
    Some(mesh)
}

/// Makes the heights texture for the Ground memory limit (window app).
fn setup_atlas(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    assets: Res<AssetServer>,
    store: Res<crate::settings_ui::SettingsStore>,
) {
    let pages = pages_for(store.0.ground_memory.bytes());
    commands.insert_resource(GridAtlas {
        image: images.add(heights_image(pages)),
        meshes: Default::default(),
        tiles: HashMap::new(),
        free: Vec::new(),
        next: 0,
        writes: Vec::new(),
        materials: Vec::new(),
        scale: (0.0, 0.0),
        _common: assets.load(COMMON_SHADER),
        pages,
        full: false,
        failed: 0,
        untiled: 0,
    });
}

impl Default for GridAtlas {
    /// An atlas with no texture behind it (the headless app: tiles are
    /// handed out and patches drawn, with nothing on a GPU).
    fn default() -> GridAtlas {
        GridAtlas {
            image: Handle::default(),
            meshes: Default::default(),
            tiles: HashMap::new(),
            free: Vec::new(),
            next: 0,
            writes: Vec::new(),
            materials: Vec::new(),
            scale: (0.0, 0.0),
            _common: Handle::default(),
            pages: pages_for(universe_core::settings::GroundMemory::default().bytes()),
            full: false,
            failed: 0,
            untiled: 0,
        }
    }
}

/// Tile writes copied into the render world, kept until the texture exists.
#[derive(Resource, Debug, Default)]
struct RenderTileWrites {
    image: Option<AssetId<Image>>,
    pending: Vec<GridWrite>,
}

/// Copies this frame's tile writes into the render world.
fn extract_tile_writes(
    atlas: Extract<Option<Res<GridAtlas>>>,
    mut writes: ResMut<RenderTileWrites>,
) {
    if let Some(atlas) = atlas.as_ref() {
        writes.image = Some(atlas.image.id());
        writes.pending.extend(atlas.writes.iter().cloned());
    }
}

/// Writes pending tiles into the heights texture, one region each.
fn write_tiles(
    mut writes: ResMut<RenderTileWrites>,
    images: Res<RenderAssets<GpuImage>>,
    queue: Res<RenderQueue>,
) {
    let Some(gpu) = writes.image.and_then(|id| images.get(id)) else {
        return;
    };
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
}

/// Clears the main world's writes once the render world has copied them
/// (both apps: the headless one has no render world to copy them).
pub(crate) fn clear_tile_writes(mut atlas: ResMut<GridAtlas>) {
    atlas.writes.clear();
}

/// Registers the ground's material, shaders and heights texture (window
/// app; the headless app keeps `DivePlugin`'s texture-less atlas).
pub(crate) struct PatchGridPlugin;

impl Plugin for PatchGridPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "patch_grid.wgsl");
        bevy::asset::embedded_asset!(app, "patch_grid_prepass.wgsl");
        bevy::asset::embedded_asset!(app, "patch_grid_common.wgsl");
        app.add_plugins(MaterialPlugin::<PatchGridMaterial>::default())
            .add_systems(Startup, setup_atlas)
            .add_systems(Update, grow_materials);
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .init_resource::<RenderTileWrites>()
                .add_systems(ExtractSchedule, extract_tile_writes)
                .add_systems(Render, write_tiles.in_set(RenderSystems::PrepareResources));
        }
    }
}

/// The tag a patch entity carries: its tile.
#[must_use]
pub(crate) fn tag(tile: u32) -> MeshTag {
    MeshTag(tile)
}

/// The grid's resources for `sync_patches`.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct GridParams<'w> {
    /// The images (absent in the headless app).
    pub(crate) images: Option<ResMut<'w, Assets<Image>>>,
    /// The heights texture and tiles.
    pub(crate) atlas: ResMut<'w, GridAtlas>,
    /// The ground materials.
    pub(crate) materials: ResMut<'w, Assets<PatchGridMaterial>>,
    /// Tiles packed by the workers.
    pub(crate) inbox: Res<'w, GridInbox>,
}

/// Culling box of `built` from its own and parent offsets, skirts included:
/// the morph moves points between them (#777), so the engine's frustum
/// check never drops a morphing patch (#778 Q3 A). `None` when empty.
#[must_use]
pub(crate) fn aabb_of(
    built: &universe_core::lod_mesh::PatchMesh,
) -> Option<bevy::camera::primitives::Aabb> {
    let mut lo = Vec3::splat(f32::INFINITY);
    let mut hi = Vec3::splat(f32::NEG_INFINITY);
    for p in built
        .offsets
        .iter()
        .chain(&built.skirt_offsets)
        .chain(&built.parent_offsets)
        .chain(&built.skirt_parent_offsets)
    {
        let v = Vec3::from_array(*p);
        lo = lo.min(v);
        hi = hi.max(v);
    }
    (lo.is_finite() && hi.is_finite()).then(|| bevy::camera::primitives::Aabb::from_min_max(lo, hi))
}

/// Gives `built` a tile (packed by a worker, else here) and returns the
/// shared mesh and its culling box; `None` when the texture is full.
pub(crate) fn upload(
    built: &universe_core::lod_mesh::PatchMesh,
    radius_m: f64,
    relief: &universe_core::relief::Relief,
    atlas: &mut GridAtlas,
    inbox: &GridInbox,
    meshes: &mut Assets<Mesh>,
) -> Option<(Handle<Mesh>, bevy::camera::primitives::Aabb)> {
    let packed = inbox
        .0
        .lock()
        .ok()
        .and_then(|mut map| map.remove(&built.key));
    let texels = packed.or_else(|| universe_core::grid_tile::grid_tile(built, radius_m, relief))?;
    let out = (|| {
        atlas.assign(built.key, texels)?;
        let handle = atlas.mesh(built, meshes)?;
        Some((handle, aabb_of(built)?))
    })();
    if out.is_none() {
        atlas.failed += 1;
    }
    out
}

/// A worker build: the patch, and its tile into `inbox`, packed from the
/// heights the build sampled (#920 T5).
pub(crate) async fn build_task(
    key: PatchKey,
    radius_m: f64,
    relief: universe_core::relief::Relief,
    inbox: GridInbox,
) -> Option<universe_core::lod_mesh::PatchMesh> {
    let (built, tile) = universe_core::grid_tile::build_patch_and_tile(key, radius_m, &relief)?;
    if let Some(tile) = tile
        && let Ok(mut map) = inbox.0.lock()
    {
        map.insert(key, tile);
    }
    Some(built)
}

#[cfg(test)]
#[path = "patch_grid_tests.rs"]
mod tests;
