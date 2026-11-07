//! Owner/recipe and generation boundaries for the one terrain pipeline (#1251).
//!
//! An Arc's identity is the generation token: while an old worker holds it, a
//! later visit cannot reuse it (including a return to the same recipe/quality).
//! The single inbox is cleared under one lock; workers never retain a private
//! cache or publish results into a successor's generation.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use universe_core::lod::PatchKey;
use universe_core::lod_reuse::HeightCache;
use universe_core::surface_recipe::SurfaceRecipeKey;

#[derive(Debug)]
pub(crate) struct TerrainEpoch {
    pub recipe: SurfaceRecipeKey,
    pub quality: universe_core::settings::Tier,
}

pub(crate) type Epoch = Arc<TerrainEpoch>;

pub(crate) fn same_epoch(a: Option<&Epoch>, b: Option<&Epoch>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

#[derive(Debug, Default)]
pub(crate) struct InboxState {
    pub epoch: Option<Epoch>,
    pub tiles: HashMap<PatchKey, crate::patch_grid::PackedTile>,
    pub heights: HeightCache,
}

/// One shared bounded inbox. Clones capture a generation but not another store.
#[derive(bevy::prelude::Resource, Debug, Default, Clone)]
pub(crate) struct GridInbox {
    pub state: Arc<Mutex<InboxState>>,
    pub epoch: Option<Epoch>,
    pub witness: Option<crate::loading_diagnostic::Producer>,
}

impl GridInbox {
    /// Serial main-world boundary. Invalidate old workers and drop old fine
    /// heights/packed tiles atomically, even for a same-body return or resize.
    pub(crate) fn begin(&mut self, epoch: Option<Epoch>) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        if let Some(witness) = &self.witness {
            witness.emit(crate::loading_diagnostic::LoadingEvent::InboxCleared {
                entries: state.tiles.len(),
                bytes: state
                    .tiles
                    .values()
                    .map(|t| t.texels.len() * size_of::<u32>())
                    .sum(),
            });
        }
        state.tiles.clear();
        state.heights = HeightCache::default();
        state.epoch.clone_from(&epoch);
        self.epoch = epoch;
        true
    }
}

#[derive(Debug)]
pub(crate) struct BuiltPatch {
    pub epoch: Option<Epoch>,
    pub mesh: universe_core::lod_mesh::PatchMesh,
}

pub(crate) struct PendingBuild {
    pub task: bevy::tasks::Task<Option<BuiltPatch>>,
    pub witness: Option<crate::loading_diagnostic::Producer>,
}

impl PendingBuild {
    pub(crate) fn is_finished(&self) -> bool {
        self.task.is_finished()
    }
}

#[cfg(test)]
#[path = "terrain_epoch_tests.rs"]
mod tests;

pub(crate) fn spawn(
    pool: &bevy::tasks::AsyncComputeTaskPool,
    key: PatchKey,
    radius_m: f64,
    relief: universe_core::relief::Relief,
    inbox: GridInbox,
    full: bool,
) -> PendingBuild {
    let witness = inbox.witness.clone();
    PendingBuild {
        task: pool.spawn(crate::patch_worker::build_task(
            key, radius_m, relief, inbox, full,
        )),
        witness,
    }
}
