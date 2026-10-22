//! Resident ground patches and the ground memory cap (#918 T5).
//!
//! Every built patch keeps its GPU mesh while it is cached, drawn or not,
//! so ground already seen comes back with no build and no upload (Q2 A).
//! Besides the GPU mesh a patch keeps only a slim summary: its centre, its
//! relief numbers for the selection, and its edge points for the F3
//! outline. The full CPU mesh (about 91 KB) stays only near the player,
//! where the body stands on it (`drawn_surface`, #779).
//!
//! The cache counts its bytes (CPU and GPU, from the buffer sizes) against
//! the Ground memory limit (P2 A, S1). When it is over, the patches
//! farthest from the camera go first; patches the frame needs (drawn,
//! wanted or in the ring) and the six roots never go.

use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use universe_core::lod::PatchKey;
use universe_core::lod_mesh::PatchMesh;
use universe_core::lod_patch_morph::{OUTLINE_POINTS, outline_points};
use universe_core::lod_ring::box_distance_m;
use universe_core::planet_frame::PlanetPos;

/// Edge points of a patch for the F3 outline: its own places and its
/// pinned (parent) places.
type Outline = [[f32; 3]; OUTLINE_POINTS];

/// The slim part of a cached patch: what the frame needs without the mesh.
#[derive(Debug, Clone)]
pub(crate) struct PatchInfo {
    /// Patch centre on the relief surface (planet frame, metres).
    pub centre: PlanetPos,
    /// `(relief_error_m, min_h_m, max_h_m)` for the selection.
    pub relief: (f64, f64, f64),
    /// Edge points at their own places.
    pub edge_own: Box<Outline>,
    /// Edge points at their pinned places (every edge coarser).
    pub edge_pinned: Box<Outline>,
}

/// Bytes of one cached patch without its meshes: the entry and its two
/// outlines.
pub(crate) const INFO_BYTES: usize = size_of::<CachedPatch>() + 2 * size_of::<Outline>();

/// One cached patch.
#[derive(Debug, Clone)]
pub(crate) struct CachedPatch {
    /// The slim summary.
    pub info: PatchInfo,
    /// The full CPU mesh, near the player only.
    pub cpu: Option<Arc<PatchMesh>>,
    /// The GPU mesh and its culling box.
    pub gpu: Option<(Handle<Mesh>, Aabb)>,
}

/// Bytes the CPU mesh of `mesh` holds.
#[must_use]
pub(crate) fn cpu_bytes(mesh: &PatchMesh) -> usize {
    let v3 = size_of::<[f32; 3]>();
    (mesh.offsets.len()
        + mesh.normals.len()
        + mesh.parent_offsets.len()
        + mesh.parent_normals.len()
        + mesh.skirt_offsets.len()
        + mesh.skirt_parent_offsets.len()
        + mesh.skirt_normals.len()
        + mesh.skirt_parent_normals.len())
        * v3
        + (mesh.morph_err_m.len() + mesh.skirt_morph_err_m.len()) * size_of::<f32>()
        + (mesh.indices.len() + mesh.skirt_indices.len()) * size_of::<u32>()
        + size_of::<PatchMesh>()
}

/// Cached patches with their byte count.
#[derive(Resource, Debug, Default)]
pub(crate) struct PatchCache {
    /// Planet seed the cached patches belong to.
    pub seed: Option<u64>,
    /// Cached patches by key.
    pub map: HashMap<PatchKey, CachedPatch>,
    /// Bytes of the CPU meshes held.
    cpu_total: usize,
    /// GPU meshes held.
    gpu_count: usize,
    /// Eviction candidates by distance (reused, `E-HOT-NOALLOC`).
    scratch: Vec<(f64, PatchKey)>,
    /// Bumped whenever a patch comes or goes, so the selection knows when
    /// its relief lookups changed.
    pub generation: u64,
}

impl PatchCache {
    /// Whether `key` is cached.
    #[must_use]
    pub(crate) fn contains(&self, key: PatchKey) -> bool {
        self.map.contains_key(&key)
    }

    /// Cached key count.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }

    /// The selection's relief numbers of `key`, if cached.
    #[must_use]
    pub(crate) fn relief_info(&self, key: PatchKey) -> Option<(f64, f64, f64)> {
        self.map.get(&key).map(|patch| patch.info.relief)
    }

    /// The four children of `key` when every one is cached: the ground a
    /// merged leaf still draws until its own patch is built (#918 T5).
    #[must_use]
    pub(crate) fn cached_children(&self, key: PatchKey) -> Option<[PatchKey; 4]> {
        key.children()
            .filter(|children| children.iter().all(|child| self.contains(*child)))
    }

    /// The selection's relief numbers of `key`: its own when cached, else
    /// its four cached children's together (#918 T5). Splits cascade, so a
    /// parent is often never built; without these numbers it could never
    /// merge, and under the memory cap the far ground could not coarsen.
    #[must_use]
    pub(crate) fn relief_or_children(&self, key: PatchKey) -> Option<(f64, f64, f64)> {
        if let Some(known) = self.relief_info(key) {
            return Some(known);
        }
        let children = self.cached_children(key)?;
        let mut out = (0.0f64, f64::INFINITY, f64::NEG_INFINITY);
        for child in children {
            let (error, min_h, max_h) = self.relief_info(child)?;
            out = (out.0.max(error), out.1.min(min_h), out.2.max(max_h));
        }
        Some(out)
    }

    /// The height range of `key`, if cached.
    #[must_use]
    pub(crate) fn heights(&self, key: PatchKey) -> Option<(f64, f64)> {
        self.map
            .get(&key)
            .map(|patch| (patch.info.relief.1, patch.info.relief.2))
    }

    /// The full CPU mesh of `key`, if held.
    #[must_use]
    pub(crate) fn cpu(&self, key: PatchKey) -> Option<&Arc<PatchMesh>> {
        self.map.get(&key).and_then(|patch| patch.cpu.as_ref())
    }

    /// Bytes held: the slim summaries, the CPU meshes and the GPU meshes.
    #[must_use]
    pub(crate) fn bytes(&self) -> usize {
        self.map.len() * INFO_BYTES
            + self.cpu_total
            + self.gpu_count * crate::patches::gpu_bytes_per_patch()
    }

    /// Bytes held by the patches in `keys` (sorted): what the frame needs.
    #[must_use]
    pub(crate) fn bytes_of(&self, keys: &[PatchKey]) -> usize {
        keys.iter()
            .filter_map(|key| self.map.get(key))
            .map(|patch| {
                INFO_BYTES
                    + patch.cpu.as_ref().map_or(0, |mesh| cpu_bytes(mesh))
                    + if patch.gpu.is_some() {
                        crate::patches::gpu_bytes_per_patch()
                    } else {
                        0
                    }
            })
            .sum()
    }

    /// Caches `mesh` with its GPU mesh; keeps the CPU mesh too.
    pub(crate) fn insert(&mut self, mesh: PatchMesh, gpu: Option<(Handle<Mesh>, Aabb)>) {
        let key = mesh.key;
        if self.map.contains_key(&key) {
            return;
        }
        let (Some(edge_own), Some(edge_pinned)) =
            (outline_points(&mesh, 0), outline_points(&mesh, 0x0F))
        else {
            return;
        };
        let info = PatchInfo {
            centre: mesh.centre,
            relief: mesh.relief_info(),
            edge_own: Box::new(edge_own),
            edge_pinned: Box::new(edge_pinned),
        };
        self.cpu_total += cpu_bytes(&mesh);
        self.gpu_count += usize::from(gpu.is_some());
        self.generation += 1;
        self.map.insert(
            key,
            CachedPatch {
                info,
                cpu: Some(Arc::new(mesh)),
                gpu,
            },
        );
    }

    /// Gives a cached patch its CPU mesh back (a rebuild near the player).
    pub(crate) fn set_cpu(&mut self, mesh: PatchMesh) {
        if let Some(patch) = self.map.get_mut(&mesh.key)
            && patch.cpu.is_none()
        {
            self.cpu_total += cpu_bytes(&mesh);
            patch.cpu = Some(Arc::new(mesh));
        }
    }

    /// Fills `out` (sorted) with the `cover` patches (sorted, the drawn
    /// ground) that are cached within `near_m` of `camera` by their box
    /// distance: the ground the body may stand on (#918 T5).
    pub(crate) fn near_keys_into(
        &self,
        cover: &[PatchKey],
        camera: PlanetPos,
        radius_m: f64,
        near_m: f64,
        out: &mut Vec<PatchKey>,
    ) {
        out.clear();
        let lookup = |key: PatchKey| self.relief_info(key);
        out.extend(cover.iter().copied().filter(|key| {
            self.contains(*key)
                && box_distance_m(*key, camera, radius_m, &lookup).is_some_and(|d| d < near_m)
        }));
    }

    /// Drops the CPU mesh of every patch with a GPU mesh that is not in
    /// `near` (sorted).
    pub(crate) fn drop_far_cpu(&mut self, near: &[PatchKey]) {
        for (key, patch) in &mut self.map {
            if patch.gpu.is_none() || near.binary_search(key).is_ok() {
                continue;
            }
            if let Some(mesh) = patch.cpu.take() {
                self.cpu_total = self.cpu_total.saturating_sub(cpu_bytes(&mesh));
            }
        }
    }

    /// Evicts the patches farthest from `camera` while over `cap_bytes`,
    /// never one in `kept` (sorted) or a root, and frees their GPU meshes.
    /// Returns the count evicted.
    pub(crate) fn evict_farthest(
        &mut self,
        cap_bytes: usize,
        kept: &[PatchKey],
        camera: PlanetPos,
        radius_m: f64,
        meshes: &mut Assets<Mesh>,
    ) -> usize {
        if self.bytes() <= cap_bytes {
            return 0;
        }
        let mut scratch = std::mem::take(&mut self.scratch);
        scratch.clear();
        {
            let lookup = |key: PatchKey| self.relief_info(key);
            for key in self.map.keys().copied() {
                if key.level > 0 && kept.binary_search(&key).is_err() {
                    let d = box_distance_m(key, camera, radius_m, &lookup).unwrap_or(f64::INFINITY);
                    scratch.push((d, key));
                }
            }
        }
        // Farthest first; ties on the key, so the order is deterministic.
        scratch.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut evicted = 0;
        for &(_, key) in &scratch {
            if self.bytes() <= cap_bytes {
                break;
            }
            if let Some(patch) = self.map.remove(&key) {
                if let Some(mesh) = patch.cpu {
                    self.cpu_total = self.cpu_total.saturating_sub(cpu_bytes(&mesh));
                }
                if let Some((handle, _)) = patch.gpu {
                    self.gpu_count = self.gpu_count.saturating_sub(1);
                    meshes.remove(&handle);
                }
                evicted += 1;
                self.generation += 1;
            }
        }
        self.scratch = scratch;
        evicted
    }

    /// Empties the cache and frees every GPU mesh (a new planet).
    pub(crate) fn clear(&mut self, meshes: &mut Assets<Mesh>) {
        for (_, patch) in self.map.drain() {
            if let Some((handle, _)) = patch.gpu {
                meshes.remove(&handle);
            }
        }
        self.cpu_total = 0;
        self.gpu_count = 0;
        self.generation += 1;
    }

    /// Cached keys as a set (tests only; steady-state uses lookups).
    #[cfg(test)]
    #[must_use]
    pub(crate) fn keys(&self) -> std::collections::HashSet<PatchKey> {
        self.map.keys().copied().collect()
    }
}

/// Frames the cap must hold builds back before the detail coarsens a step.
pub(crate) const BIAS_UP_FRAMES: i32 = 10;

/// Frames with room to spare before the detail refines a step.
pub(crate) const BIAS_DOWN_FRAMES: i32 = 60;

/// Detail coarsening per step, and the most it may reach.
pub(crate) const BIAS_STEP: f64 = 1.15;
pub(crate) const BIAS_MAX: f64 = 4.0;

/// Share of the cap under which the detail may refine again.
pub(crate) const BIAS_ROOM: f64 = 0.7;

/// Next `(bias, frames)` of the memory detail bias (#918 T5, pure): a run
/// of `blocked` frames (the cap held a build back) coarsens a step; a long
/// run with `needed_bytes` well under `cap_bytes` refines a step.
#[must_use]
pub(crate) fn bias_step(
    bias: f64,
    frames: i32,
    blocked: bool,
    needed_bytes: usize,
    cap_bytes: usize,
) -> (f64, i32) {
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: byte counts compared as a share, intended"
    )]
    let roomy = (needed_bytes as f64) < (cap_bytes as f64) * BIAS_ROOM;
    let bias = bias.clamp(1.0, BIAS_MAX);
    if blocked {
        let frames = frames.max(0) + 1;
        if frames >= BIAS_UP_FRAMES {
            return ((bias * BIAS_STEP).min(BIAS_MAX), 0);
        }
        return (bias, frames);
    }
    if roomy && bias > 1.0 {
        let frames = frames.min(0) - 1;
        if -frames >= BIAS_DOWN_FRAMES {
            return ((bias / BIAS_STEP).max(1.0), 0);
        }
        return (bias, frames);
    }
    (bias, 0)
}
