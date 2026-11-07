//! Existing terrain worker plus optional scalar loading witnesses (#1238 T5).
//! No cancellation, cache, build or tile decision is changed by observation.

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
) -> impl Future<Output = Option<universe_core::lod_mesh::PatchMesh>> {
    let mut ticket = inbox.2.as_ref().map(|w| w.job(key, full, false));
    async move {
        if let Some(ticket) = &mut ticket {
            ticket.start();
        }
        let result = build(key, radius_m, &relief, &inbox, full);
        if let Some(ticket) = ticket {
            ticket.finish(result.as_ref());
        }
        result
    }
}

fn build(
    key: PatchKey,
    radius_m: f64,
    relief: &universe_core::relief::Relief,
    inbox: &GridInbox,
    full: bool,
) -> Option<universe_core::lod_mesh::PatchMesh> {
    let parent = inbox
        .1
        .lock()
        .ok()
        .and_then(|cache| cache.parent_of(key, relief, radius_m));
    let (built, tile, fine) = universe_core::grid_tile::build_patch_and_tile_reusing(
        key,
        radius_m,
        relief,
        full,
        parent.as_deref(),
    )?;
    if let Some(fine) = fine
        && let Ok(mut cache) = inbox.1.lock()
    {
        cache.insert(Arc::new(fine));
        if let Some(witness) = &inbox.2 {
            witness.emit(LoadingEvent::HeightCache {
                entries: cache.len(),
                cap: cache.capacity(),
                logical_bytes: cache.logical_bytes(),
            });
        }
    }
    if let Some(tile) = tile
        && let Ok(mut map) = inbox.0.lock()
    {
        let replaced = map.insert(
            key,
            PackedTile {
                seed: relief.seed(),
                radius_bits: radius_m.to_bits(),
                texels: tile,
            },
        );
        if let Some(witness) = &inbox.2 {
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
