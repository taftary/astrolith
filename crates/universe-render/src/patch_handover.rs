//! One detailed owner with coarse cover on both sides of a transition.
//! These are main-world preparation gates, not GPU/contact/visibility verdicts.

use crate::patches::{GroundSurface, PatchCache, PatchTasks, PatchTree};
use universe_core::nav::Universe;
use universe_core::surface_frame::SurfaceFrame;
use universe_core::surface_observation::SurfaceIdentity;

/// A request cannot discard outgoing detail until both bodies have real coarse
/// entities. A missing successor keeps the current owner; no extra terrain set.
pub(crate) fn owner(
    tree: &PatchTree,
    universe: &Universe,
    has_cover: impl Fn(SurfaceIdentity) -> bool,
) -> Option<SurfaceIdentity> {
    let current = tree
        .recipe
        .map(|key| key.owner())
        .filter(|owner| SurfaceFrame::for_owner(universe, *owner).is_some());
    let Some(current) = current else {
        return tree.candidates.as_ref()?.owners().next();
    };
    let requested = tree.interest.map_or(current, |interest| interest.owner);
    if requested != current
        && SurfaceFrame::for_owner(universe, requested).is_some()
        && has_cover(current)
        && has_cover(requested)
    {
        Some(requested)
    } else {
        Some(current)
    }
}

/// Reuses the one existing cache/atlas. Retired workers retain queue slots and
/// their original witness; new generations cannot accept their output.
#[allow(
    clippy::too_many_arguments,
    reason = "E-MOD-DOMAIN: shared terrain owner rebuild boundary"
)]
pub(crate) fn rebuild(
    universe: &Universe,
    owner: SurfaceIdentity,
    quality: universe_core::settings::Tier,
    tree: &mut PatchTree,
    cache: &mut PatchCache,
    tasks: &mut PatchTasks,
    ground: &mut GroundSurface,
    grid: &mut crate::patch_grid::GridParams<'_>,
) -> bool {
    let Ok(recipe) = universe_core::surface_recipe::SurfaceRecipe::for_owner(owner) else {
        return false;
    };
    let Some(snapshot) = SurfaceFrame::for_owner(universe, owner) else {
        return false;
    };
    let frame = snapshot.patch_frame();
    let relief = recipe.relief().clone();
    let Some(fresh) = universe_core::lod_select::QuadTree::new(frame.radius_m()) else {
        return false;
    };
    let fresh = fresh.with_ranges(universe_core::lod_range::LevelRanges::new(
        &relief,
        frame.radius_m(),
        4.0,
    ));
    crate::patch_loading::abandon(tasks);
    tasks.witness = grid
        .diagnostics
        .as_ref()
        .map(|d| d.surface_epoch(universe, Some(owner)));
    grid.inbox.witness.clone_from(&tasks.witness);
    grid.atlas.witness.clone_from(&tasks.witness);
    let epoch = std::sync::Arc::new(crate::terrain_epoch::TerrainEpoch {
        recipe: recipe.key(),
        quality,
    });
    if !grid.inbox.begin(Some(epoch.clone())) {
        return false;
    }
    let seed = universe.open_seed();
    if cache.seed != Some(seed) || cache.recipe != Some(recipe.key()) {
        cache.clear();
        if let Some(witness) = &tasks.witness {
            witness.cache(cache);
        }
        cache.seed = Some(seed);
        for key in cache.released.drain(..) {
            grid.atlas.release(key);
        }
    }
    tasks.ring_m = universe_core::lod_ring::RING_M;
    tasks.last_inputs = None;
    tasks.detail_bias = 1.0;
    tasks.bias_frames = 0;
    tasks.synced.clear();
    tasks.completed.clear();
    tasks.wanted.clear();
    tasks.drawn.clear();
    tasks.cover.clear();
    tasks.near.clear();
    tasks.kept.clear();
    ground.0 = None;
    ground.1 = false;
    ground.2 = false;
    ground.3 = None;
    tasks.epoch = Some(epoch);
    cache.recipe = Some(recipe.key());
    tasks.world = Some(recipe.kind());
    tree.tree = Some(fresh);
    tree.seed = Some(seed);
    tree.recipe = Some(recipe.key());
    tree.frame = Some(frame);
    tree.relief = Some(relief);
    tree.surface_frame = Some(snapshot);
    true
}

#[cfg(test)]
#[path = "patch_handover_tests.rs"]
mod tests;
