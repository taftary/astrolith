//! Terrain worker with generation-safe publication and optional witnesses.
//! Observation never changes cancellation, cache, build or tile decisions.

use crate::loading_diagnostic::LoadingEvent;
use crate::patch_grid::{GridInbox, PackedTile};
use std::sync::Arc;
use universe_core::lod::PatchKey;

/// Dispatch is outside the future, so an unpolled task still has a real ticket.
pub(crate) fn build_task(
    key: PatchKey,
    radius_m: f64,
    relief: universe_core::relief::Relief,
    inbox: GridInbox,
    full: bool,
) -> impl Future<Output = Option<crate::terrain_epoch::BuiltPatch>> {
    let mut ticket = inbox.witness.as_ref().map(|w| w.job(key, full, false));
    async move {
        if let Some(ticket) = &mut ticket {
            ticket.start();
        }
        let result = build(key, radius_m, &relief, &inbox, full);
        if let Some(ticket) = ticket {
            ticket.finish(result.as_ref());
        }
        result.map(|mesh| crate::terrain_epoch::BuiltPatch {
            epoch: inbox.epoch.clone(),
            mesh,
        })
    }
}

fn build(
    key: PatchKey,
    radius_m: f64,
    relief: &universe_core::relief::Relief,
    inbox: &GridInbox,
    full: bool,
) -> Option<universe_core::lod_mesh::PatchMesh> {
    let parent = {
        let state = inbox.state.lock().ok()?;
        if !crate::terrain_epoch::same_epoch(state.epoch.as_ref(), inbox.epoch.as_ref()) {
            return None;
        }
        state.heights.parent_of(key, relief, radius_m)
    };
    let (built, tile, fine) = universe_core::grid_tile::build_patch_and_tile_reusing(
        key,
        radius_m,
        relief,
        full,
        parent.as_deref(),
    )?;
    // A generation may change while expensive pure sampling runs. Never publish
    // stale fine heights or tiles, including an old same-body visit's output.
    let mut state = inbox.state.lock().ok()?;
    if !crate::terrain_epoch::same_epoch(state.epoch.as_ref(), inbox.epoch.as_ref()) {
        return None;
    }
    if let Some(fine) = fine {
        let cache = &mut state.heights;
        cache.insert(Arc::new(fine));
        if let Some(witness) = &inbox.witness {
            witness.emit(LoadingEvent::HeightCache {
                entries: cache.len(),
                cap: cache.capacity(),
                logical_bytes: cache.logical_bytes(),
            });
        }
    }
    if let Some(tile) = tile {
        let map = &mut state.tiles;
        let replaced = map.insert(
            key,
            PackedTile {
                epoch: inbox.epoch.clone(),
                seed: relief.seed(),
                radius_bits: radius_m.to_bits(),
                texels: tile,
            },
        );
        if let Some(witness) = &inbox.witness {
            witness.emit(LoadingEvent::Packed {
                key,
                replaced: replaced.is_some(),
                entries: map.len(),
                bytes: map
                    .values()
                    .map(|t| t.texels.len() * size_of::<u32>())
                    .sum(),
            });
        }
    }
    Some(built)
}
