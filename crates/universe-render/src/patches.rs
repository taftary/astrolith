//! LOD patch workers, cache and drawing (rebuild step 3, #776, T3).
//!
//! Six roots become a quadtree of 33 x 33-point patches (core `lod`). Patches
//! build on worker threads, the frame applies at most [`PATCH_APPLY_BUDGET`]
//! finished patches, and a parent stays drawn until all four children are
//! ready, so no hole ever appears (Q4 A). Each patch entity is [`Anchored`]
//! at its `f64` centre; vertices are metre offsets narrowed to `f32`, so the
//! narrowing stays small (Q5 A, #774). `#775`'s six fixed faces are gone;
//! moons stay in `planet.rs`.

use crate::Universe;
use crate::style::Anchored;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, Mesh3d, PrimitiveTopology};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures_lite::future::block_on};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use universe_core::lod::{PatchKey, QuadTree};
use universe_core::lod_mesh::build_patch;
use universe_core::planet_frame::PlanetFrame;

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
    pub(crate) fn insert(&mut self, mesh: universe_core::lod_mesh::PatchMesh) {
        let key = mesh.key;
        if self.map.contains_key(&key) {
            return;
        }
        // Evict oldest until there is room.
        while self.map.len() >= PATCH_CACHE_CAP {
            let Some(old) = self.order.pop_front() else {
                break;
            };
            self.map.remove(&old);
        }
        self.order.push_back(key);
        self.map.insert(key, mesh);
    }

    /// Cached keys as a set (for the draw rule).
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
#[must_use]
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
/// origin (the entity's [`Anchored`] point).
#[must_use]
pub(crate) fn patch_bevy_mesh(
    mesh: &universe_core::lod_mesh::PatchMesh,
    frame: &PlanetFrame,
) -> Option<Mesh> {
    let mpc = frame.metres_per_cell();
    if !mpc.is_finite() || mpc <= 0.0 {
        return None;
    }
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(mesh.offsets.len());
    for off in mesh.offsets.iter().copied() {
        let cx = f64::from(off[0]) / mpc;
        let cy = f64::from(off[1]) / mpc;
        let cz = f64::from(off[2]) / mpc;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: patch offsets narrowed for the GPU, intended"
        )]
        let cell = [cx as f32, cy as f32, cz as f32];
        positions.push(cell);
    }
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(mesh.normals.len());
    for nrm in mesh.normals.iter().copied() {
        normals.push(nrm);
    }
    let mut bevy = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    bevy.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    bevy.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    bevy.insert_indices(Indices::U32(mesh.indices.clone()));
    Some(bevy)
}

/// Syncs the selection, workers, cache and entities.
///
/// At L10 the tree updates from the planet-frame camera and the Normal
/// detail (1.5 px; Planet detail arrives in T4). Missing leaves spawn worker
/// builds; finished builds apply up to the budget; parents stay until
/// children are ready; entities mirror the draw set. Outside L10 everything
/// despawns and the tree resets.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn sync_patches(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    universe: Res<Universe>,
    mut tree_res: ResMut<PatchTree>,
    mut cache: ResMut<PatchCache>,
    mut tasks: ResMut<PatchTasks>,
    owned: Query<(
        Entity,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &PatchPiece,
    )>,
    origin: Res<crate::style::RenderOrigin>,
) {
    let active = universe.level().get() == 10;
    if !active {
        if tree_res.tree.is_some() || tree_res.seed.is_some() {
            for (entity, mesh, material, _) in &owned {
                commands.entity(entity).despawn();
                meshes.remove(&mesh.0);
                materials.remove(&material.0);
            }
            tree_res.tree = None;
            tree_res.seed = None;
            tree_res.frame = None;
            tasks.pending.clear();
            tasks.completed.clear();
        }
        return;
    }
    // Planet frame for the open cell (one source, #774).
    let chain = universe.path.indices();
    let l10: &[u32] = chain.get(..9).unwrap_or(&[]);
    if l10.len() != 9 {
        return;
    }
    let frame = PlanetFrame::for_planet(universe.root.0, l10);
    let seed = universe.open_seed();
    let radius_m = frame.radius_m();
    // New planet: fresh tree, roots built synchronously so something draws.
    if tree_res.seed != Some(seed) || tree_res.tree.is_none() {
        for (entity, mesh, material, _) in &owned {
            commands.entity(entity).despawn();
            meshes.remove(&mesh.0);
            materials.remove(&material.0);
        }
        tasks.pending.clear();
        tasks.completed.clear();
        let Some(tree) = QuadTree::new(radius_m) else {
            return;
        };
        // Roots now: six synchronous builds (fast, no hole on arrival).
        for leaf in tree.leaves() {
            if !cache.contains(leaf)
                && let Some(mesh) = build_patch(leaf, radius_m)
            {
                cache.insert(mesh);
            }
        }
        let _ = tree;
        // Rebuild the tree for this planet (fresh roots).
        let Some(fresh) = QuadTree::new(radius_m) else {
            return;
        };
        tree_res.tree = Some(fresh);
        tree_res.seed = Some(seed);
        tree_res.frame = Some(frame);
        // Fall through to the per-frame update below.
    }
    let Some(tree) = tree_res.tree.as_mut() else {
        return;
    };
    // Camera in the planet frame.
    let camera_cells = universe.path.offset();
    let camera = frame.from_cell(camera_cells);
    // Detail threshold: Normal 1.5 px until Planet detail (T4).
    tree.update(camera, 1.5);
    // Wanted leaves (reused scratch).
    tasks.wanted.clear();
    tasks.wanted.extend(tree.leaves());
    // Spawn missing builds on the worker pool.
    {
        let pool = AsyncComputeTaskPool::get();
        let PatchTasks {
            pending,
            completed,
            wanted,
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
            pending.push((leaf, pool.spawn(async move { build_patch(leaf, radius_m) })));
        }
    }
    // Drain finished workers into completed (deterministic order later).
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
    // Apply up to the budget, finest-first for stability (sorted by level
    // descending, then key for determinism).
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
        // Take from the end after finest-first sort? The sort above puts
        // finest first, so remove(0) takes finest. Use swap_remove for
        // determinism with the sorted order: pop from front via remove(0)
        // is O(n) but n is small (<= budget + a few).
        let (_, mesh) = tasks.completed.remove(0);
        if !cache.contains(mesh.key) {
            cache.insert(mesh);
            applied += 1;
        }
    }
    // Draw set with parents-stay.
    let leaves = tree.leaves();
    let cached = cache.keys();
    let drawn = draw_selection(&leaves, &cached);
    // Despawn entities no longer drawn.
    for (entity, mesh, material, piece) in &owned {
        if !drawn.contains(&piece.0) {
            commands.entity(entity).despawn();
            meshes.remove(&mesh.0);
            materials.remove(&material.0);
        }
    }
    // Spawn missing drawn patches.
    let mut have: HashSet<PatchKey> = HashSet::new();
    for (_, _, _, piece) in &owned {
        have.insert(piece.0);
    }
    // One flat-colour unlit material for the sphere (Q2 A; light in #780).
    let planet_colour = crate::style::point_color_for_level(
        universe_core::coords::Level::new(10).unwrap_or(universe_core::coords::Level::MIN),
    );
    let material = materials.add(StandardMaterial {
        base_color: planet_colour,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let at_centre = origin.cell();
    for key in drawn {
        if have.contains(&key) {
            continue;
        }
        let Some(cached_mesh) = cache.map.get(&key) else {
            continue;
        };
        let Some(bevy_mesh) = patch_bevy_mesh(cached_mesh, &frame) else {
            continue;
        };
        let centre_cell = frame.to_cell(cached_mesh.centre);
        commands.spawn((
            PatchPiece(key),
            Name::new(face_name(key)),
            Mesh3d(meshes.add(bevy_mesh)),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(at_centre),
            Anchored(centre_cell),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_tests::headless_app;
    use universe_core::lod::{PatchKey, is_ancestor};
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
}
