//! Drawn patch cover, morph/culling view and entity naming (#1251 split).
//! Parents/children stay until replacement geometry exists; no policy change.

use crate::patch_cache::PatchCache;
use bevy::prelude::*;
use universe_core::lod::PatchKey;
use universe_core::lod_cull::View;

/// Six actual uploaded roots are the minimum replacement for a body's proxy.
/// CPU build completion/recipe intent alone cannot authorize hiding coarse cover.
/// This checks main-world handles/tiles, not GPU completion or pixel visibility;
/// material/proxy handover remains a separate requirement.
pub(crate) fn roots_ready(cache: &PatchCache, atlas: &crate::patch_grid::GridAtlas) -> bool {
    universe_core::cube_sphere::Face::all()
        .into_iter()
        .all(|face| {
            let key = PatchKey::root(face);
            cache.map.get(&key).is_some_and(|entry| entry.gpu.is_some())
                && atlas.tile(key).is_some()
        })
}

pub(crate) fn face_name(key: PatchKey) -> String {
    format!(
        "lod-patch-{}-l{}-{}-{}",
        key.face_index(),
        key.level,
        key.x,
        key.y
    )
}

/// Finest built ancestors, or cached children while a merged parent builds.
/// Reuses storage; hidden neighbours remain in the mask reference cover.
pub(crate) fn draw_selection_into(
    leaves: &[PatchKey],
    cache: &PatchCache,
    out: &mut Vec<PatchKey>,
) {
    out.clear();
    for leaf in leaves.iter().copied() {
        if !cache.contains(leaf)
            && let Some(children) = cache.cached_children(leaf)
        {
            out.extend(children);
            continue;
        }
        let mut cursor = leaf;
        loop {
            if cache.contains(cursor) {
                out.push(cursor);
                break;
            }
            let Some(parent) = cursor.parent() else {
                break;
            };
            cursor = parent;
        }
    }
    out.sort();
    out.dedup();
}

#[cfg(test)]
pub(crate) fn draw_selection(
    leaves: &[PatchKey],
    cached: &std::collections::HashSet<PatchKey>,
) -> std::collections::BTreeSet<PatchKey> {
    let mut out = std::collections::BTreeSet::new();
    for leaf in leaves.iter().copied() {
        let mut cursor = leaf;
        loop {
            if cached.contains(&cursor) {
                out.insert(cursor);
                break;
            }
            let Some(parent) = cursor.parent() else {
                break;
            };
            cursor = parent;
        }
    }
    out
}

pub(crate) fn upload(
    grid: &mut crate::patch_grid::GridParams<'_>,
    mesh: &universe_core::lod_mesh::PatchMesh,
    frame: &universe_core::planet_frame::PlanetFrame,
    relief: &universe_core::relief::Relief,
    meshes: &mut Assets<Mesh>,
) -> Option<(Handle<Mesh>, bevy::camera::primitives::Aabb)> {
    crate::patch_grid::upload(
        mesh,
        frame.radius_m(),
        relief,
        &mut grid.atlas,
        &grid.inbox,
        meshes,
    )
}

/// The rendered camera's owner-centred view; axes remain the production paint
/// axes. No culling in no-window tests or without a perspective projection.
pub(crate) fn camera_view(
    camera: universe_core::planet_frame::PlanetPos,
    nav: Option<&crate::input::Navigation>,
    projections: &Query<&Projection, With<Camera3d>>,
    windows: &Query<&Window>,
) -> Option<View> {
    let nav = nav?;
    let fov = projections.iter().find_map(|projection| match projection {
        Projection::Perspective(p) => Some(f64::from(p.fov)),
        _ => None,
    })?;
    let window = windows.iter().next()?;
    let (w, h) = (f64::from(window.width()), f64::from(window.height()));
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    View::new(camera, nav.forward, nav.up, fov, w / h)
}

/// One effective detail from the shared terrain tier and memory pressure bias.
#[must_use]
pub(crate) fn terrain_detail_px(tier: universe_core::settings::Tier, bias: f64) -> f64 {
    universe_core::quality::planet_detail(tier).quad_px() * bias.max(1.0)
}
