//! LOD patch workers, cache and drawing (rebuild step 3, #776, T3; relief #799).
//!
//! Six roots become a quadtree of 33 x 33-point patches (core `lod`) on the
//! relief surface (core `relief`). Patches build on worker threads, the frame
//! applies at most [`PATCH_APPLY_BUDGET`] finished patches, and a parent stays
//! drawn until all four children are ready, so no hole ever appears (Q4 A).
//! Each patch entity is [`Anchored`] at its `f64` centre; vertices are metre
//! offsets narrowed to `f32`, so the narrowing stays small (Q5 A, #774).
//! Vertices carry unlit height tints from `tokens.rs` (deep, low, high, snow;
//! #799 Q3 A, Q4 A) and blend in the morph shader (`patch_material`,
//! #777 T3: parent shape plus edge-mask pinning, no per-frame mesh rebuilds).
//! `#775`'s six fixed faces are gone; moons stay in `planet.rs`.

use crate::Universe;
use crate::patch_material::{
    ATTRIBUTE_EDGE_FLAGS, ATTRIBUTE_PARENT_NORMAL, ATTRIBUTE_PARENT_POSITION, PlanetPatchMaterial,
    patch_material,
};
use crate::style::Anchored;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, Mesh3d, PrimitiveTopology};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures_lite::future::block_on};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use universe_core::lod::{PatchKey, patch_quad_sag_m};
use universe_core::lod_mesh::build_patch;
use universe_core::lod_morph::morph_band_m;
use universe_core::lod_select::{QuadTree, UpdateScratch};
use universe_core::planet_frame::PlanetFrame;
use universe_core::relief::Relief;

/// Finished patches applied per frame (Q4 A).
///
/// Upper limit from the design doc (100-200); T5 measures on the dev PC and
/// sets it from the frame time.
pub(crate) const PATCH_APPLY_BUDGET: usize = 100;

/// Bounded patch pool (Q4 A): finished meshes wait here, LRU, never unbounded.
pub(crate) const PATCH_CACHE_CAP: usize = 2048;

/// Marks one drawn LOD patch entity (`E-ENTITY-NAME` via `Name`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PatchPiece(pub PatchKey);

/// Current planet's selection tree plus its frame.
#[derive(Resource, Debug, Default)]
pub(crate) struct PatchTree {
    /// Open-cell seed the tree belongs to, if any.
    pub seed: Option<u64>,
    /// Selection, if at L10.
    pub tree: Option<QuadTree>,
    /// Planet frame for the open cell, if at L10.
    pub frame: Option<PlanetFrame>,
    /// Relief for the open cell, if at L10 (#799).
    pub relief: Option<Relief>,
}

/// Bounded LRU store of finished patch meshes.
#[derive(Resource, Debug, Default)]
pub(crate) struct PatchCache {
    /// Finished meshes by key.
    pub map: HashMap<PatchKey, universe_core::lod_mesh::PatchMesh>,
    /// Insertion order for eviction (front is oldest).
    pub order: VecDeque<PatchKey>,
}

impl PatchCache {
    /// Whether `key` is cached.
    #[must_use]
    pub(crate) fn contains(&self, key: PatchKey) -> bool {
        self.map.contains_key(&key)
    }

    /// Cached key count.
    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }

    /// Inserts `mesh`, evicting oldest while over the cap.
    ///
    /// Roots (level 0) are never evicted: they are the ultimate fallback, so
    /// a hole can never appear (review item 4). Measured max is a few hundred
    /// leaves (see `LOD` line); the 2048 cap holds 4x headroom, so eviction
    /// never runs in practice.
    pub(crate) fn insert(&mut self, mesh: universe_core::lod_mesh::PatchMesh) {
        let key = mesh.key;
        if self.map.contains_key(&key) {
            return;
        }
        // Evict oldest non-root until there is room.
        while self.map.len() >= PATCH_CACHE_CAP {
            let mut evicted = false;
            let mut skipped: Vec<PatchKey> = Vec::new();
            while let Some(old) = self.order.pop_front() {
                if old.level == 0 {
                    // Never evict roots; keep them aside and continue.
                    skipped.push(old);
                    continue;
                }
                self.map.remove(&old);
                evicted = true;
                break;
            }
            // Roots back in order.
            for root in skipped {
                self.order.push_front(root);
            }
            if !evicted {
                break;
            }
        }
        self.order.push_back(key);
        self.map.insert(key, mesh);
    }

    /// Cached keys as a set (tests only; steady-state uses lookups).
    #[cfg(test)]
    #[must_use]
    pub(crate) fn keys(&self) -> HashSet<PatchKey> {
        self.map.keys().copied().collect()
    }
}

/// Pending worker builds plus finished-but-unapplied patches and scratch.
#[derive(Resource, Default)]
pub(crate) struct PatchTasks {
    /// Builds in flight.
    pub pending: Vec<(PatchKey, Task<Option<universe_core::lod_mesh::PatchMesh>>)>,
    /// Finished but not yet applied (throttled by the budget).
    pub completed: Vec<(PatchKey, universe_core::lod_mesh::PatchMesh)>,
    /// Wanted leaves scratch (reused, `E-HOT-NOALLOC`).
    pub wanted: Vec<PatchKey>,
    /// Draw set scratch (reused, sorted/deduped, `E-HOT-NOALLOC`).
    pub drawn: Vec<PatchKey>,
    /// Update scratch for the quadtree (reused, `E-HOT-NOALLOC`).
    pub update_scratch: UpdateScratch,
}

fn face_name(key: PatchKey) -> String {
    format!(
        "lod-patch-{}-l{}-{}-{}",
        key.face_index(),
        key.level,
        key.x,
        key.y
    )
}

/// Draw set with the parents-stay rule: for every leaf, the finest cached
/// ancestor (itself when ready), deduplicated. No hole appears while roots
/// are cached.
///
/// Fills `out` in place (sorted/deduped, reused capacity, `E-HOT-NOALLOC`)
/// instead of returning a new set.
pub(crate) fn draw_selection_into(
    leaves: &[PatchKey],
    cache: &PatchCache,
    out: &mut Vec<PatchKey>,
) {
    out.clear();
    for leaf in leaves.iter().copied() {
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

/// Draw set with the parents-stay rule, allocating (tests/headless only).
#[must_use]
#[allow(dead_code)]
pub(crate) fn draw_selection(
    leaves: &[PatchKey],
    cached: &HashSet<PatchKey>,
) -> BTreeSet<PatchKey> {
    let mut out = BTreeSet::new();
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

/// Bevy mesh for `mesh`: metre offsets into cell units relative to the patch
/// origin (the entity's [`Anchored`] point), with unlit height tints (#799),
/// skirts below every edge (#777 Q4 A) and morph attributes for the shader
/// (#777 T3: parent shape plus per-vertex edge flags).
///
/// Colours come from `tokens.rs` (`E-TOKENS`) via the core tint bands
/// (deep, low, high, snow); the base material stays white and unlit.
#[must_use]
pub(crate) fn patch_bevy_mesh(
    mesh: &universe_core::lod_mesh::PatchMesh,
    frame: &PlanetFrame,
    relief_scale: f64,
) -> Option<Mesh> {
    use universe_core::relief::{ReliefTint, tint_for_height};
    let mpc = frame.metres_per_cell();
    if !mpc.is_finite() || mpc <= 0.0 {
        return None;
    }
    let radius_m = frame.radius_m();
    let total = mesh.offsets.len() + mesh.skirt_offsets.len();
    let narrow = |off: [f32; 3]| {
        let cx = f64::from(off[0]) / mpc;
        let cy = f64::from(off[1]) / mpc;
        let cz = f64::from(off[2]) / mpc;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: patch offsets narrowed for the GPU, intended"
        )]
        let cell = [cx as f32, cy as f32, cz as f32];
        cell
    };
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(total);
    for off in mesh
        .offsets
        .iter()
        .copied()
        .chain(mesh.skirt_offsets.iter().copied())
    {
        positions.push(narrow(off));
    }
    let mut parent_positions: Vec<[f32; 3]> = Vec::with_capacity(total);
    for off in mesh
        .parent_offsets
        .iter()
        .copied()
        .chain(mesh.skirt_parent_offsets.iter().copied())
    {
        parent_positions.push(narrow(off));
    }
    let mut normals: Vec<[f32; 3]> =
        Vec::with_capacity(mesh.normals.len() + mesh.skirt_normals.len());
    for nrm in mesh
        .normals
        .iter()
        .copied()
        .chain(mesh.skirt_normals.iter().copied())
    {
        normals.push(nrm);
    }
    let mut parent_normals: Vec<[f32; 3]> = Vec::with_capacity(total);
    for nrm in mesh
        .parent_normals
        .iter()
        .copied()
        .chain(mesh.skirt_parent_normals.iter().copied())
    {
        parent_normals.push(nrm);
    }
    // Edge flags for odd edge vertices (East, West, North, South as 0/1).
    // Skirts copy their edge vertex's flags so they pin together.
    let mut edge_flags: Vec<[f32; 4]> = Vec::with_capacity(total);
    let side = 33usize;
    for dj in 0..33usize {
        for di in 0..33usize {
            let on_east = di == 32 && dj % 2 == 1;
            let on_west = di == 0 && dj % 2 == 1;
            let on_north = dj == 32 && di % 2 == 1;
            let on_south = dj == 0 && di % 2 == 1;
            edge_flags.push([
                f32::from(u8::from(on_east)),
                f32::from(u8::from(on_west)),
                f32::from(u8::from(on_north)),
                f32::from(u8::from(on_south)),
            ]);
        }
    }
    let _ = side;
    // Skirt flags in South, North, West, East order (33 per edge).
    for edge_no in 0..4 {
        for k in 0..33usize {
            let (on_east, on_west, on_north, on_south) = match edge_no {
                0 => (false, false, false, k % 2 == 1),
                1 => (false, false, k % 2 == 1, false),
                2 => (false, k % 2 == 1, false, false),
                _ => (k % 2 == 1, false, false, false),
            };
            edge_flags.push([
                f32::from(u8::from(on_east)),
                f32::from(u8::from(on_west)),
                f32::from(u8::from(on_north)),
                f32::from(u8::from(on_south)),
            ]);
        }
    }
    // Height tints from the relief surface (centre plus f32 offsets).
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(total);
    for off in mesh
        .offsets
        .iter()
        .copied()
        .chain(mesh.skirt_offsets.iter().copied())
    {
        let px = mesh.centre.0[0] + f64::from(off[0]);
        let py = mesh.centre.0[1] + f64::from(off[1]);
        let pz = mesh.centre.0[2] + f64::from(off[2]);
        let r = (px * px + py * py + pz * pz).sqrt();
        let h = r - radius_m;
        let tint = tint_for_height(h, relief_scale);
        let color = match tint {
            ReliefTint::Deep => crate::tokens::RELIEF_DEEP_TINT,
            ReliefTint::Low => crate::tokens::RELIEF_LOW_TINT,
            ReliefTint::High => crate::tokens::RELIEF_HIGH_TINT,
            ReliefTint::Snow => crate::tokens::RELIEF_SNOW_TINT,
        }
        .to_linear();
        colors.push([color.red, color.green, color.blue, 1.0]);
    }
    let mut bevy = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    bevy.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    bevy.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    bevy.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    bevy.insert_attribute(ATTRIBUTE_PARENT_POSITION, parent_positions);
    bevy.insert_attribute(ATTRIBUTE_PARENT_NORMAL, parent_normals);
    bevy.insert_attribute(ATTRIBUTE_EDGE_FLAGS, edge_flags);
    let mut combined = Vec::with_capacity(mesh.indices.len() + mesh.skirt_indices.len());
    combined.extend(mesh.indices.iter().copied());
    combined.extend(mesh.skirt_indices.iter().copied());
    bevy.insert_indices(Indices::U32(combined));
    Some(bevy)
}

/// Syncs the selection, workers, cache and entities.
///
/// At L10 the tree updates from the planet-frame camera and the Planet detail
/// threshold (S1). Missing leaves spawn worker builds; finished builds apply
/// up to the budget; parents stay until children are ready; entities mirror
/// the draw set. Outside L10 everything despawns and the tree resets.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn sync_patches(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PlanetPatchMaterial>>,
    universe: Res<Universe>,
    store: Res<crate::settings_ui::SettingsStore>,
    mut tree_res: ResMut<PatchTree>,
    mut cache: ResMut<PatchCache>,
    mut tasks: ResMut<PatchTasks>,
    owned: Query<(
        Entity,
        &Mesh3d,
        &MeshMaterial3d<PlanetPatchMaterial>,
        &PatchPiece,
    )>,
    origin: Res<crate::style::RenderOrigin>,
) {
    let active = universe.level().get() == 10;
    if !active {
        if tree_res.tree.is_some() || tree_res.seed.is_some() {
            for (entity, mesh, _, _) in &owned {
                commands.entity(entity).despawn();
                meshes.remove(&mesh.0);
            }
            tree_res.tree = None;
            tree_res.seed = None;
            tree_res.frame = None;
            tree_res.relief = None;
            tasks.pending.clear();
            tasks.completed.clear();
            tasks.wanted.clear();
            tasks.drawn.clear();
        }
        return;
    }
    // Seed first: cached, no allocation. Only on a new planet do we touch the
    // path chain (one allocation per planet change, not per frame).
    let seed = universe.open_seed();
    if tree_res.seed != Some(seed)
        || tree_res.tree.is_none()
        || tree_res.frame.is_none()
        || tree_res.relief.is_none()
    {
        for (entity, mesh, _, _) in &owned {
            commands.entity(entity).despawn();
            meshes.remove(&mesh.0);
        }
        tasks.pending.clear();
        tasks.completed.clear();
        tasks.wanted.clear();
        tasks.drawn.clear();
        // One chain allocation per planet change (rare).
        let chain = universe.path.indices();
        let l10: &[u32] = chain.get(..9).unwrap_or(&[]);
        if l10.len() != 9 {
            return;
        }
        let frame = PlanetFrame::for_planet(universe.root.0, l10);
        let relief = Relief::for_planet(universe.root.0, l10);
        let radius_m = frame.radius_m();
        let Some(tree) = QuadTree::new(radius_m) else {
            return;
        };
        // Roots now: six synchronous builds (fast, no hole on arrival).
        tree.append_leaves_to(&mut tasks.wanted);
        for leaf in tasks.wanted.iter().copied() {
            if !cache.contains(leaf)
                && let Some(mesh) = build_patch(leaf, radius_m, &relief)
            {
                cache.insert(mesh);
            }
        }
        let Some(fresh) = QuadTree::new(radius_m) else {
            return;
        };
        tree_res.tree = Some(fresh);
        tree_res.seed = Some(seed);
        tree_res.frame = Some(frame);
        tree_res.relief = Some(relief);
    }
    let Some(frame) = tree_res.frame.as_ref().copied() else {
        return;
    };
    let relief_opt = tree_res.relief.clone();
    let Some(tree) = tree_res.tree.as_mut() else {
        return;
    };
    let Some(relief) = relief_opt else {
        return;
    };
    let radius_m = frame.radius_m();
    let relief_scale = relief.relief_scale();
    // Camera in the planet frame (no allocation).
    let camera_cells = universe.path.offset();
    let camera = frame.from_cell(camera_cells);
    let detail_px = store.0.planet_detail.detail_px();
    // Relief-aware selection with reused scratch (no steady-state allocation).
    // The lookup borrows the cache; missing entries read as zero.
    tree.update_with_relief_reused(
        camera,
        detail_px,
        &|key| {
            cache
                .map
                .get(&key)
                .map(|mesh| (mesh.relief_error_m, mesh.height_range_m()))
        },
        &mut tasks.update_scratch,
    );
    // Wanted leaves, reused scratch.
    tree.append_leaves_to(&mut tasks.wanted);
    // Spawn missing builds on the worker pool.
    {
        let pool = AsyncComputeTaskPool::get();
        let PatchTasks {
            pending,
            completed,
            wanted,
            ..
        } = &mut *tasks;
        for leaf in wanted.iter().copied() {
            if cache.contains(leaf) {
                continue;
            }
            if pending.iter().any(|(held, _)| *held == leaf) {
                continue;
            }
            if completed.iter().any(|(held, _)| *held == leaf) {
                continue;
            }
            let task_relief = relief.clone();
            pending.push((
                leaf,
                pool.spawn(async move { build_patch(leaf, radius_m, &task_relief) }),
            ));
        }
    }
    // Drain finished workers into completed.
    {
        let mut index = 0;
        while index < tasks.pending.len() {
            let finished = tasks
                .pending
                .get(index)
                .is_some_and(|(_, task)| task.is_finished());
            if finished {
                let (key, task) = tasks.pending.remove(index);
                if let Some(mesh) = block_on(task) {
                    tasks.completed.push((key, mesh));
                }
            } else {
                index += 1;
            }
        }
    }
    // Apply up to the budget, finest-first.
    tasks.completed.sort_by(|a, b| {
        b.0.level
            .cmp(&a.0.level)
            .then(a.0.face_index().cmp(&b.0.face_index()))
            .then(a.0.x.cmp(&b.0.x))
            .then(a.0.y.cmp(&b.0.y))
    });
    let mut applied = 0usize;
    while applied < PATCH_APPLY_BUDGET {
        if tasks.completed.is_empty() {
            break;
        }
        let (_, mesh) = tasks.completed.remove(0);
        if !cache.contains(mesh.key) {
            cache.insert(mesh);
            applied += 1;
        }
    }
    // Draw set with parents-stay, reused scratch (sorted/deduped).
    {
        let PatchTasks { wanted, drawn, .. } = &mut *tasks;
        draw_selection_into(wanted, &cache, drawn);
    }
    // Morph inputs per drawn patch (#777 T3): the CPU passes the camera
    // offset and band, the GPU computes the same factor core does
    // (no mesh rebuilds per frame).
    let morph_for = |key: PatchKey| -> ([f32; 3], f32, f32, u8) {
        let mask = tree.edge_mask(key);
        let cached = cache.map.get(&key);
        let centre = cached.map(|mesh| mesh.centre).unwrap_or(camera);
        let relief_pair = cached.map(|mesh| (mesh.relief_error_m, mesh.height_range_m()));
        let (relief_error, _) = relief_pair.unwrap_or((0.0, 0.0));
        let quad_sag = patch_quad_sag_m(key, radius_m).unwrap_or(0.0);
        let err = quad_sag.max(relief_error.max(0.0));
        let (start, end) = morph_band_m(key, radius_m, err, detail_px).unwrap_or((0.0, 0.0));
        let rel = [
            centre.0[0] - camera.0[0],
            centre.0[1] - camera.0[1],
            centre.0[2] - camera.0[2],
        ];
        // Camera offset is centre-minus-camera so length is the distance;
        // the shader only uses its length.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: camera offset narrowed for the GPU, intended"
        )]
        let rel_f32 = [rel[0] as f32, rel[1] as f32, rel[2] as f32];
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: morph band narrowed for the GPU, intended"
        )]
        let (start_f32, end_f32) = (start as f32, end as f32);
        (rel_f32, start_f32, end_f32, mask)
    };
    // Despawn entities no longer drawn and refresh morph uniforms on the
    // ones that stay (camera moves every frame, so offsets change).
    for (entity, mesh, mat, piece) in &owned {
        if tasks.drawn.binary_search(&piece.0).is_err() {
            commands.entity(entity).despawn();
            meshes.remove(&mesh.0);
            materials.remove(&mat.0);
        } else {
            let (rel, start, end, mask) = morph_for(piece.0);
            if let Some(mut material) = materials.get_mut(&mat.0) {
                material.extension.params.camera_rel = Vec4::new(rel[0], rel[1], rel[2], 0.0);
                material.extension.params.band = Vec4::new(start, end, 0.0, 0.0);
                let bit = |b: u8| {
                    if mask & (1 << b) != 0 { 1.0 } else { 0.0 }
                };
                material.extension.params.edge_mask = Vec4::new(bit(0), bit(1), bit(2), bit(3));
            }
        }
    }
    // Spawn missing drawn patches with their morph material.
    let mut have: Vec<PatchKey> = Vec::new();
    for (_, _, _, piece) in &owned {
        if tasks.drawn.binary_search(&piece.0).is_ok() {
            have.push(piece.0);
        }
    }
    have.sort();
    let at_centre = origin.cell();
    for key in tasks.drawn.iter().copied() {
        if have.binary_search(&key).is_ok() {
            continue;
        }
        let Some(cached_mesh) = cache.map.get(&key) else {
            continue;
        };
        let Some(bevy_mesh) = patch_bevy_mesh(cached_mesh, &frame, relief_scale) else {
            continue;
        };
        let (rel, start, end, mask) = morph_for(key);
        let centre_cell = frame.to_cell(cached_mesh.centre);
        commands.spawn((
            PatchPiece(key),
            Name::new(face_name(key)),
            Mesh3d(meshes.add(bevy_mesh)),
            MeshMaterial3d(materials.add(patch_material(rel, start, end, mask))),
            Transform::from_translation(at_centre),
            Anchored(centre_cell),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_tests::headless_app;
    use universe_core::lod::PatchKey;
    use universe_core::lod_select::is_ancestor;
    use universe_core::nav::MarkerIndex;

    fn covering_drawn(drawn: &BTreeSet<PatchKey>, leaf: PatchKey) -> bool {
        let mut cursor = leaf;
        loop {
            if drawn.contains(&cursor) {
                return true;
            }
            let Some(parent) = cursor.parent() else {
                return false;
            };
            cursor = parent;
        }
    }

    #[test]
    fn parents_stay_until_children_are_ready() {
        // AC3 / C3 (T3, Q4 A): after a split no frame lacks coverage, and no
        // more than the budget is applied per frame. No-window app test
        // (E-TEST-NO-WINDOW).
        let mut app = headless_app();
        {
            let mut universe = app.world_mut().resource_mut::<Universe>();
            for _ in 0..9 {
                let marker = universe.autopilot_target().expect("dive continues");
                assert!(universe.open(MarkerIndex(marker)), "marker opens");
            }
            assert_eq!(universe.level().get(), 10, "the dive ends at L10");
        }
        // First frame builds the six roots synchronously: coverage at once.
        app.update();
        let mut previous_cache = 0usize;
        for frame in 0..20 {
            app.update();
            let tree = app
                .world()
                .resource::<PatchTree>()
                .tree
                .clone()
                .expect("tree at L10");
            let cache = app.world().resource::<PatchCache>();
            // Budget: cache grows by at most the budget per frame.
            let grown = cache.len().saturating_sub(previous_cache);
            assert!(
                grown <= PATCH_APPLY_BUDGET,
                "frame {frame} applied {grown} over {PATCH_APPLY_BUDGET}"
            );
            previous_cache = cache.len();
            // Coverage: every leaf has a drawn ancestor.
            let leaves = tree.leaves();
            assert!(!leaves.is_empty(), "frame {frame} has leaves");
            let cached = cache.keys();
            let drawn = draw_selection(&leaves, &cached);
            assert!(!drawn.is_empty(), "frame {frame} draws something");
            for leaf in &leaves {
                assert!(
                    covering_drawn(&drawn, *leaf),
                    "frame {frame}: leaf {leaf:?} uncovered"
                );
            }
            // Entities mirror the draw set (no extra, no missing).
            let mut entities: HashSet<PatchKey> = HashSet::new();
            let mut query = app.world_mut().query::<&PatchPiece>();
            for piece in query.iter(app.world()) {
                entities.insert(piece.0);
            }
            assert_eq!(
                entities,
                drawn.iter().copied().collect::<HashSet<PatchKey>>(),
                "frame {frame}: entities mirror the draw set"
            );
            // Ancestor check: no drawn patch is an ancestor of another drawn
            // patch (the draw set is a cut).
            for a in drawn.iter().copied() {
                for b in drawn.iter().copied() {
                    if a != b {
                        assert!(!is_ancestor(a, b), "frame {frame}: {a:?} contains {b:?}");
                    }
                }
            }
        }
        // Leaving L10 despawns every patch with no error.
        {
            let mut universe = app.world_mut().resource_mut::<Universe>();
            assert!(universe.close().is_some(), "the dive backs out");
        }
        app.update();
        let mut query = app.world_mut().query::<&PatchPiece>();
        assert_eq!(
            query.iter(app.world()).count(),
            0,
            "no patch survives outside L10"
        );
    }

    #[test]
    fn every_patch_uses_the_morph_material() {
        // T3 done: every drawn patch uses `PlanetPatchMaterial` with the
        // core morph factor and edge mask. No-window app test.
        use crate::patch_material::PlanetPatchMaterial;
        let mut app = headless_app();
        {
            let mut universe = app.world_mut().resource_mut::<Universe>();
            for _ in 0..9 {
                let marker = universe.autopilot_target().expect("dive continues");
                assert!(universe.open(MarkerIndex(marker)), "marker opens");
            }
            assert_eq!(universe.level().get(), 10, "the dive ends at L10");
        }
        for _ in 0..10 {
            app.update();
        }
        let tree = app
            .world()
            .resource::<PatchTree>()
            .tree
            .clone()
            .expect("tree at L10");
        // Every entity carries the morph material with camera offset, band
        // and edge mask matching the tree.
        let mut count = 0usize;
        let mut query = app
            .world_mut()
            .query::<(&PatchPiece, &MeshMaterial3d<PlanetPatchMaterial>)>();
        let handles: Vec<_> = query
            .iter(app.world())
            .map(|(piece, mat)| (piece.0, mat.0.clone()))
            .collect();
        assert!(!handles.is_empty(), "draws patches");
        for (key, handle) in handles {
            let materials = app.world().resource::<Assets<PlanetPatchMaterial>>();
            let material = materials.get(&handle).expect("material exists");
            let rel = material.extension.params.camera_rel;
            assert!(
                rel.x.is_finite() && rel.y.is_finite() && rel.z.is_finite(),
                "patch {key:?} camera offset finite"
            );
            let band = material.extension.params.band;
            assert!(
                band.x.is_finite() && band.y.is_finite() && band.y >= band.x,
                "patch {key:?} band valid"
            );
            let mask = tree.edge_mask(key);
            let bit = |b: u8| {
                if mask & (1 << b) != 0 { 1.0 } else { 0.0 }
            };
            assert_eq!(
                material.extension.params.edge_mask,
                Vec4::new(bit(0), bit(1), bit(2), bit(3)),
                "patch {key:?} mask matches tree"
            );
            count += 1;
        }
        assert!(count > 0, "checked morph materials");
    }
}
