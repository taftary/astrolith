//! LOD patch workers, cache and drawing (rebuild step 3, #776, T3; relief #799).
//!
//! Six roots become a quadtree of 33 x 33-point patches (core `lod`) on the
//! relief surface (core `relief`). Patches build on worker threads, the frame
//! applies at most [`PATCH_APPLY_BUDGET`] finished patches, and a parent stays
//! drawn until all four children are ready, so no hole ever appears (Q4 A).
//! Each patch entity is [`Anchored`] at its `f64` centre; vertices are metre
//! offsets narrowed to `f32`, so the narrowing stays small (Q5 A, #774).
//! Every patch draws the one shared grid with its shape from its tile in
//! the heights texture (`patch_grid`, #920), blended in the vertex shader
//! (#777: per-vertex distance, parent shape, edge mask). Patches share one
//! material per level; a mask change writes one texel of the tile, so a
//! steady frame touches no material and allocates nothing (#777 review
//! item 4).
//! `#775`'s six fixed faces are gone; moons stay in `planet.rs`.

use crate::Universe;
use crate::style::Anchored;
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::NoAutoAabb;
use bevy::mesh::{Mesh, Mesh3d};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures_lite::future::block_on};
use std::collections::{BTreeSet, HashSet};
use universe_core::lod::PatchKey;
use universe_core::lod_cull::{Cull, View};
use universe_core::lod_mesh::build_patch;
use universe_core::lod_morph::morph_mask_in;
use universe_core::lod_select::{QuadTree, UpdateScratch};
use universe_core::planet_frame::PlanetFrame;
use universe_core::relief::Relief;

/// Finished patches applied per frame (Q4 A).
///
/// Upper limit from the design doc (100-200); T5 measures on the dev PC and
/// sets it from the frame time.
pub(crate) const PATCH_APPLY_BUDGET: usize = 100;

/// Marks one drawn LOD patch entity (`E-ENTITY-NAME` via `Name`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PatchPiece(pub PatchKey);

/// The morph mask a patch entity's shared material was picked for.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PatchMask(pub u8);

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

/// The drawn ground the body stands on (#779 Q1 A): the whole tree's draw
/// set with its masks, blended from the rendered camera. Rebuilt when the
/// draw set changes; its camera follows every frame. `None` off the planet.
/// The first flag says the ground near the player is ready: every leaf in
/// the ring is built (every wanted leaf with no ring), so no coarser patch
/// stands in there and the ground under the body will not jump (#918).
/// The second says every wanted leaf is built, out to the horizon.
#[derive(Resource, Debug, Default)]
pub(crate) struct GroundSurface(
    pub Option<universe_core::drawn_surface::DrawnSurface>,
    pub bool,
    pub bool,
);

/// Whether a body may be dropped onto the ground now: off the planet at
/// once, at L10 once the drawn surface exists and its builds are done (#779).
#[must_use]
pub(crate) fn ground_ready(
    surface: Option<&GroundSurface>,
    universe: &universe_core::nav::Universe,
) -> bool {
    universe.level().get() != 10 || surface.is_some_and(|s| s.0.is_some() && s.1)
}

/// The ground under `point` (open-cell units): the drawn surface at L10 when
/// it covers the point, else the level's ground (#779).
#[must_use]
pub(crate) fn ground_under(
    surface: Option<&GroundSurface>,
    universe: &universe_core::nav::Universe,
    point: [f64; 3],
) -> Option<universe_core::body::Ground> {
    surface
        .filter(|_| universe.level().get() == 10)
        .and_then(|s| s.0.as_ref())
        .and_then(|s| s.ground_at(point))
        .or_else(|| universe_core::body::level_ground(universe, point))
}

pub(crate) use crate::patch_cache::PatchCache;

/// Box distance below which a cached patch keeps its full CPU mesh, for
/// the body standing on it (#918 T5).
pub(crate) const NEAR_CPU_M: f64 = 50.0;

/// Box distance below which a build makes the full CPU mesh (#920 T6):
/// twice [`NEAR_CPU_M`], so a patch the body walks toward rarely needs a
/// second, full build. Farther builds are tile-only.
const NEAR_BUILD_M: f64 = 2.0 * NEAR_CPU_M;

/// View change under which the selection is not run again: about 0.0006°
/// (#918 T5), far below one pixel at any detail.
const STEADY_VIEW_EPS: f64 = 1e-5;

/// Builds in flight per worker thread (#819, #918 T8). The worker pool is
/// shared (Bevy hands screenshots back through it too), so the ground keeps
/// its queue short instead of spawning every wanted leaf at once: a long
/// queue held a screenshot back past its frame cap. Six keep a worker busy
/// for about one frame (a build takes about 6 ms in release, #918 T8).
const BUILDS_PER_WORKER: usize = 6;

/// Builds in flight for a pool of `threads` workers (pure, at least 2).
#[must_use]
pub(crate) const fn max_builds(threads: usize) -> usize {
    let cap = threads * BUILDS_PER_WORKER;
    if cap < 2 { 2 } else { cap }
}

/// Pending worker builds plus finished-but-unapplied patches and scratch.
#[derive(Resource, Default)]
pub(crate) struct PatchTasks {
    /// Builds in flight.
    pub pending: Vec<(PatchKey, Task<Option<universe_core::lod_mesh::PatchMesh>>)>,
    /// Finished but not yet applied (throttled by the budget).
    pub completed: Vec<(PatchKey, universe_core::lod_mesh::PatchMesh)>,
    /// Visible leaves, the ones drawn (reused, `E-HOT-NOALLOC`).
    pub wanted: Vec<PatchKey>,
    /// Leaves to build, nearest first: the visible ones plus the ground
    /// ring (reused, #918 T2).
    pub build: Vec<PatchKey>,
    /// Hidden cover with no built mesh, queued after `build`: background
    /// work so a fast turn finds soft ground instead of black (#948 T2).
    pub tail: Vec<PatchKey>,
    /// `tail` with each patch's distance, for the sort (reused, #920 T9).
    pub tail_order: Vec<(f64, PatchKey)>,
    /// The visible leaves' draw set: what is on screen (reused,
    /// sorted/deduped, `E-HOT-NOALLOC`). Entities mirror `cover` instead.
    pub drawn: Vec<PatchKey>,
    /// Update scratch for the quadtree (reused, `E-HOT-NOALLOC`).
    pub update_scratch: UpdateScratch,
    /// Cover the entities (and their masks) were last synced to (reused).
    pub synced: Vec<PatchKey>,
    /// Existing entity keys on a refinement frame (reused).
    pub have: Vec<PatchKey>,
    /// Every leaf of the tree, hidden ones included (reused, #817).
    pub all_leaves: Vec<PatchKey>,
    /// The whole tree's draw set (parents-stay), the edge masks' reference:
    /// a hidden coarser neighbour still pins a visible edge (#817). Every
    /// cover patch has an entity (#948 R2): the selection follows last
    /// frame's camera, so a fast turn must find the hidden ground already
    /// drawn; the GPU culls off-screen patches with the current camera.
    pub cover: Vec<PatchKey>,
    /// Patches the frame needs, sorted: never evicted (reused, #918 T5).
    pub kept: Vec<PatchKey>,
    /// Cached patches near the player, sorted: they keep their CPU mesh
    /// (reused, #918 T5).
    pub near: Vec<PatchKey>,
    /// Current ground ring radius: 500 m, less while over the memory cap
    /// (#918 P1 B).
    pub ring_m: f64,
    /// Ground memory limit in effect, in bytes: the Ground memory limit
    /// setting, read every frame so a change applies at once (#918 S1).
    pub cap_bytes: usize,
    /// A limit in bytes overriding the setting, 0 for none (tests).
    pub cap_override: usize,
    /// The selection's last inputs: camera, view, detail, ring and cache
    /// generation (#918 T5).
    pub last_inputs: Option<SelectionInputs>,
    /// Detail coarsening for the memory cap, 1 for none (#918 T5).
    pub detail_bias: f64,
    /// Frames in a row the cap held builds back, or had room to spare.
    pub bias_frames: i32,
}

/// What one selection update reads (#918 T5).
pub(crate) type SelectionInputs = (
    universe_core::planet_frame::PlanetPos,
    Option<View>,
    f64,
    f64,
    u64,
);

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
/// are cached. A merged leaf not built yet whose four children are all
/// cached draws them instead, so merging never pops to coarser ground
/// (children-stay, #918 T5).
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

/// The GPU side of a new cached patch: its tile and the shared mesh
/// (#920), with its culling box.
fn upload(
    grid: &mut crate::patch_grid::GridParams<'_>,
    mesh: &universe_core::lod_mesh::PatchMesh,
    frame: &PlanetFrame,
    relief: &Relief,
    meshes: &mut Assets<Mesh>,
) -> Option<(Handle<Mesh>, Aabb)> {
    crate::patch_grid::upload(
        mesh,
        frame.radius_m(),
        relief,
        &mut grid.atlas,
        &grid.inbox,
        meshes,
    )
}

/// The camera's view in the planet frame for culling (#778 Q3 A): its
/// forward and up (cell axes are planet axes), the projection's vertical
/// field of view and the window's aspect. `None` without a window or
/// perspective camera (headless app tests), so nothing is culled there.
fn camera_view(
    camera: universe_core::planet_frame::PlanetPos,
    nav: Option<&crate::input::Navigation>,
    projections: &Query<&Projection, With<Camera3d>>,
    windows: &Query<&Window>,
) -> Option<View> {
    let nav = nav?;
    let fov = projections.iter().find_map(|projection| match projection {
        Projection::Perspective(perspective) => Some(f64::from(perspective.fov)),
        _ => None,
    })?;
    let window = windows.iter().next()?;
    let (w, h) = (f64::from(window.width()), f64::from(window.height()));
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    View::new(camera, nav.forward, nav.up, fov, w / h)
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
    universe: Res<Universe>,
    store: Res<crate::settings_ui::SettingsStore>,
    mut tree_res: ResMut<PatchTree>,
    mut cache: ResMut<PatchCache>,
    mut tasks: ResMut<PatchTasks>,
    owned: Query<(Entity, &Mesh3d, &PatchPiece, &PatchMask)>,
    origin: Res<crate::style::RenderOrigin>,
    nav: Option<Res<crate::input::Navigation>>,
    player: Option<Res<crate::player_state::PlayerState>>,
    (projections, windows): (Query<&Projection, With<Camera3d>>, Query<&Window>),
    mut ground: ResMut<GroundSurface>,
    mut grid: crate::patch_grid::GridParams<'_>,
) {
    tasks.cap_bytes = if tasks.cap_override == 0 {
        store.0.ground_memory.bytes()
    } else {
        tasks.cap_override
    };
    // The heights texture holds a tile per patch the limit allows: a new
    // limit remakes it, and the ground rebuilds (#920 T6).
    let pages = crate::patch_grid::pages_for(tasks.cap_bytes);
    if grid.atlas.pages != pages {
        let crate::patch_grid::GridParams {
            atlas,
            materials,
            images,
            ..
        } = &mut grid;
        atlas.resize(pages, images.as_deref_mut(), materials);
        cache.seed = None;
        tree_res.tree = None;
    }
    let active = universe.level().get() == 10;
    if !active {
        if tree_res.tree.is_some() || tree_res.seed.is_some() {
            // The meshes stay with the cache (#918 T5).
            for (entity, _, _, _) in &owned {
                commands.entity(entity).despawn();
            }
            tasks.synced.clear();
            tree_res.tree = None;
            tree_res.seed = None;
            tree_res.frame = None;
            tree_res.relief = None;
            tasks.pending.clear();
            tasks.completed.clear();
            tasks.wanted.clear();
            tasks.drawn.clear();
        }
        ground.0 = None;
        ground.1 = false;
        ground.2 = false;
        return;
    }
    // Seed first: cached, no allocation. Only on a new planet do we touch the
    // path chain (one allocation per planet change, not per frame).
    let seed = universe.open_seed();
    // Set when this call despawns every patch for a new planet: the owned
    // query still lists them until the commands apply, so the loop below
    // must not touch them again (#851).
    let mut rebuilt = false;
    if tree_res.seed != Some(seed)
        || tree_res.tree.is_none()
        || tree_res.frame.is_none()
        || tree_res.relief.is_none()
    {
        rebuilt = true;
        for (entity, _, _, _) in &owned {
            commands.entity(entity).despawn();
        }
        // Keys do not name their planet: another planet's patches go
        // (#918 T5).
        if cache.seed != Some(seed) {
            cache.clear();
            cache.seed = Some(seed);
            // Their tiles go free now, before the roots below take tiles
            // again under the same keys (#920 T2).
            for key in cache.released.drain(..) {
                grid.atlas.release(key);
            }
        }
        tasks.ring_m = universe_core::lod_ring::RING_M;
        tasks.last_inputs = None;
        tasks.detail_bias = 1.0;
        tasks.bias_frames = 0;
        tasks.synced.clear();
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
                let gpu = upload(&mut grid, &mesh, &frame, &relief, &mut meshes);
                cache.insert(mesh, gpu);
            }
        }
        let Some(fresh) = QuadTree::new(radius_m) else {
            return;
        };
        let fresh = fresh.with_ranges(universe_core::lod_range::LevelRanges::new(
            &relief, radius_m, 4.0,
        ));
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
    // Camera in the planet frame (no allocation).
    // The rendered camera (`RenderOrigin`, last frame's pose; #778): the
    // path offset in dive and free flight, the posed eye or third-person
    // camera in normal mode, so detail and the view follow what is drawn.
    let camera_cells = if origin.l10.is_some() {
        origin.camera
    } else {
        universe.path.offset()
    };
    let camera = frame.from_cell(camera_cells);
    // The ground under the player's feet: the body if walking, else his
    // spot (#919 T2), dropped along the radius onto the relief. The ground
    // there keeps its CPU mesh at any height of the camera or the body
    // (#920 T6: a figure shot drops the body about 1.4 km while the camera
    // is still high, and the body stands on the drawn ground straight
    // below it).
    let feet = player.as_deref().map(|state| {
        let at = frame.from_cell(if state.active {
            state.body.position
        } else {
            state.spot.position
        });
        let len = (at.0[0] * at.0[0] + at.0[1] * at.0[1] + at.0[2] * at.0[2]).sqrt();
        if !len.is_finite() || len <= 0.0 {
            return at;
        }
        let dir = [at.0[0] / len, at.0[1] / len, at.0[2] / len];
        let r = radius_m + relief.height_m(dir);
        universe_core::planet_frame::PlanetPos([dir[0] * r, dir[1] * r, dir[2] * r])
    });
    let anchors_buf = [camera, feet.unwrap_or(camera)];
    let anchors: &[universe_core::planet_frame::PlanetPos] = &anchors_buf;
    // Pixels per quad at the end of each level's range (#817 F1 A).
    // The Planet detail times the memory bias: over the Ground memory limit
    // the whole selection coarsens a step instead of leaving far patches
    // unbuilt under their drawn parents (#918 T5).
    let detail_px = store.0.planet_detail.quad_px() * tasks.detail_bias.max(1.0);
    // Relief-aware, culled selection with reused scratch (no steady-state
    // allocation, #778 Q3 A, Q4 A). The lookups borrow the cache; missing
    // entries read as zero error and the planet's relief bounds.
    {
        let view = camera_view(camera, nav.as_deref(), &projections, &windows);
        let heights = |key: PatchKey| cache.heights(key);
        let bounds = relief.height_bounds_m();
        let cull = view.map(|view| Cull {
            view,
            heights: &heights,
            fallback: bounds,
            occluder_m: radius_m + bounds.0,
        });
        // The ground ring keeps the ground near the player built in every
        // direction (#918 T2).
        tree.set_ring_m(tasks.ring_m);
        // A steady frame (same camera, view, detail, ring and cache) gives
        // the same selection: skip it (#918 T5, `E-HOT-NOALLOC`).
        let inputs = (camera, view, detail_px, tasks.ring_m, cache.generation);
        let same = tasks.last_inputs.is_some_and(|last| {
            let views = match (last.1, view) {
                (Some(a), Some(b)) => a.near_same(&b, STEADY_VIEW_EPS),
                (None, None) => true,
                _ => false,
            };
            views
                && last.0 == inputs.0
                && last.2 == inputs.2
                && last.3 == inputs.3
                && last.4 == inputs.4
        });
        if !same {
            tasks.last_inputs = Some(inputs);
            tree.update_culled_reused(
                camera,
                detail_px,
                &|key| cache.relief_or_children(key),
                cull.as_ref(),
                &mut tasks.update_scratch,
            );
        }
    }
    // Drawn: the visible leaves. Built: those plus the ring, nearest first
    // (#918 T2), so hidden ring ground is ready before the camera turns.
    tree.append_visible_leaves_to(&mut tasks.wanted);
    tree.append_wanted_to(&mut tasks.build);
    // The patch under the player builds first, even when the camera is
    // elsewhere (#919 T2, AC4): the body if walking, else his spot.
    if let Some(feet) = feet {
        universe_core::lod_ring::promote_player_leaf(&mut tasks.build, feet, radius_m);
    }
    // Hidden cover with no built mesh queues last: a fast turn reveals it
    // before the workers catch up, and a built coarse leaf draws soft
    // instead of black (#948 T2). It never holds back wanted builds: the
    // room gate below spends leftover room on it without raising `blocked`
    // (the memory bias must not coarsen the far view for background work).
    let wanted_len = tasks.build.len();
    tree.append_unbuilt_cover_to(&|key| cache.contains(key), &mut tasks.tail);
    // Nearest hidden cover builds first: a turn reveals what is close
    // (reused buffers, deterministic ties). Each distance is worked out
    // once: in the comparisons it cost up to 25 ms a frame on arrival
    // (#920 T9, AC8).
    {
        let lookup = |key: PatchKey| cache.relief_or_children(key);
        let PatchTasks {
            build,
            tail,
            tail_order,
            ..
        } = &mut *tasks;
        tail_order.clear();
        tail_order.extend(tail.drain(..).map(|key| {
            let distance = universe_core::lod_ring::box_distance_m(key, camera, radius_m, &lookup)
                .unwrap_or(f64::INFINITY);
            (distance, key)
        }));
        tail_order.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        build.extend(tail_order.iter().map(|&(_, key)| key));
    }
    let mut missing: usize;
    // Drain finished workers into completed first, so their slots refill
    // this frame (#918 T8: draining after spawning left them idle a frame).
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
    // Spawn missing builds on the worker pool, nearest first, up to
    // `max_builds` in flight.
    let memory_cap = tasks.cap_bytes;
    let ring_len = tree.ring_len();
    let mut blocked = false;
    let mut ring_missing = 0usize;
    {
        let pool = AsyncComputeTaskPool::get();
        let cap = max_builds(pool.thread_num());
        // Tiles go back with the builds (#920 T2).
        let inbox = grid.inbox.clone();
        let PatchTasks {
            pending,
            completed,
            build,
            near,
            kept,
            ..
        } = &mut *tasks;
        missing = 0usize;
        // Near patches that dropped their CPU mesh rebuild first: the body
        // stands on them (#918 T5).
        for key in near.iter().copied() {
            if pending.len() >= cap {
                break;
            }
            if cache.cpu(key).is_some()
                || pending.iter().any(|(held, _)| *held == key)
                || completed.iter().any(|(held, _)| *held == key)
            {
                continue;
            }
            let task_relief = relief.clone();
            pending.push((
                key,
                pool.spawn(crate::patch_grid::build_task(
                    key,
                    radius_m,
                    task_relief,
                    inbox.clone(),
                    true,
                )),
            ));
        }
        // Room under the Ground memory limit for builds beyond the ring,
        // counting those in flight (#918 AC5): the cap wins over far detail.
        // Only what the frame needs counts: patches it no longer needs are
        // evicted to make room, so they must not hold builds back.
        let per_patch = cache.gpu_each() + crate::patch_cache::INFO_BYTES;
        let mut room = memory_cap
            .saturating_sub(cache.bytes_of(kept))
            .saturating_sub((pending.len() + completed.len()) * per_patch);
        for (index, leaf) in build.iter().copied().enumerate() {
            if cache.contains(leaf) {
                continue;
            }
            // A wanted leaf not built yet: the ground is not ready (#918).
            // Background cover past `wanted_len` never counts: it is best
            // effort, and readiness signals keep their wanted-only meaning.
            if index < wanted_len {
                missing += 1;
            }
            if index < ring_len {
                ring_missing += 1;
            }
            if pending.len() >= cap {
                continue;
            }
            // Ring leaves always build (the ring shrinks to fit instead), and
            // so does a merged leaf over its four cached children: it frees
            // three patches' worth once built.
            if index >= ring_len && cache.cached_children(leaf).is_none() {
                // The ring first: workers stay free for ring leaves the
                // refining tree still adds (#918 AC3).
                if ring_missing > 0 {
                    continue;
                }
                if room < per_patch {
                    // Background cover (#948 T2) spends leftover room only:
                    // it never reports the cap as holding builds back, so
                    // the bias below cannot coarsen the far view for it.
                    if index < wanted_len {
                        blocked = true;
                    }
                    continue;
                }
                // Background cover also keeps one flick of headroom: a fast
                // turn's new builds (about 15 MB measured, #950) must fit
                // without tripping the bias the moment they arrive.
                if index >= wanted_len && room < memory_cap / 5 {
                    continue;
                }
                room -= per_patch;
            }
            if pending.iter().any(|(held, _)| *held == leaf) {
                continue;
            }
            if completed.iter().any(|(held, _)| *held == leaf) {
                continue;
            }
            // Full builds only where the body may stand (#920 T6): the rest
            // make just their tile and the selection numbers.
            let lookup = |key: PatchKey| cache.relief_info(key);
            let full = anchors.iter().any(|point| {
                universe_core::lod_ring::box_distance_m(leaf, *point, radius_m, &lookup)
                    .is_some_and(|d| d < NEAR_BUILD_M)
            });
            let task_relief = relief.clone();
            pending.push((
                leaf,
                pool.spawn(crate::patch_grid::build_task(
                    leaf,
                    radius_m,
                    task_relief,
                    inbox.clone(),
                    full,
                )),
            ));
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
        if cache.contains(mesh.key) {
            cache.set_cpu(mesh);
        } else {
            let gpu = upload(&mut grid, &mesh, &frame, &relief, &mut meshes);
            cache.insert(mesh, gpu);
            applied += 1;
        }
    }
    // Draw set with parents-stay, reused scratch (sorted/deduped).
    {
        let PatchTasks {
            wanted,
            drawn,
            all_leaves,
            cover,
            ..
        } = &mut *tasks;
        draw_selection_into(wanted, &cache, drawn);
        tree.append_leaves_to(all_leaves);
        draw_selection_into(all_leaves, &cache, cover);
    }
    // Memory (#918 T5): full CPU meshes near the player only; then, while
    // over the Ground memory limit, the farthest patches the frame does not
    // need go, and the ring shrinks while what it needs is over the limit.
    {
        let cap = memory_cap;
        let PatchTasks {
            build,
            cover,
            kept,
            near,
            ring_m,
            detail_bias,
            bias_frames,
            ..
        } = &mut *tasks;
        cache.near_keys_into(cover, anchors, radius_m, NEAR_CPU_M, near);
        cache.drop_far_cpu(near);
        kept.clear();
        kept.extend(build.iter().copied());
        kept.extend(cover.iter().copied());
        kept.sort();
        kept.dedup();
        cache.evict_farthest(cap, kept, camera, radius_m);
        // Evicted or cleared patches free their tiles (#920 T2).
        for key in cache.released.drain(..) {
            grid.atlas.release(key);
        }
        // The ring follows what the frame needs, not what the cache still
        // holds beyond it: the cap evicts that first.
        *ring_m = universe_core::lod_ring::next_ring_m(
            *ring_m,
            universe_core::lod_ring::RING_M,
            cache.bytes_of(kept),
            cap,
        );
        // Coarsen a step after the cap held builds back for a while; refine
        // a step after a long while with room (#918 T5). Not while the ring
        // builds: the bias coarsens the ring too, and its ground comes first
        // (AC3).
        let needed = cache.bytes_of(kept);
        let blocked = blocked && ring_missing == 0;
        let (bias, frames) =
            crate::patch_cache::bias_step(*detail_bias, *bias_frames, blocked, needed, cap);
        *detail_bias = bias;
        *bias_frames = frames;
    }
    // Band scale for the shared materials: metres per world unit and
    // `focal / detail`, rewritten only when Planet detail changes.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: morph scale narrowed for the GPU, intended"
    )]
    // World units are metres at the planet (#780 F2 A).
    let (mpu, band_scale) = (
        1.0f32,
        (universe_core::precision::focal_px() / detail_px) as f32,
    );
    {
        let crate::patch_grid::GridParams {
            atlas, materials, ..
        } = &mut grid;
        atlas.set_scale(mpu, band_scale, materials);
    }
    // Fast path: the draw set is the one the entities mirror, so masks and
    // entities are unchanged (no scan, no allocation, `E-HOT-NOALLOC`).
    // The ground's blend follows the rendered camera every frame (#779).
    if let Some(surface) = ground.0.as_mut() {
        surface.set_camera(camera);
    }
    // Ready only when every wanted leaf is built, not merely when no build
    // runs this frame (#918 T8). Near the player: the ring's leaves, which
    // lead the build list (#918 AC3).
    let all_ready = missing == 0 && tasks.pending.is_empty() && tasks.completed.is_empty();
    let ring_built = if tree.ring_m() > 0.0 {
        tasks
            .build
            .iter()
            .take(tree.ring_len())
            .all(|key| cache.contains(*key))
    } else {
        all_ready
    };
    // The body stands on the near drawn ground: it needs those CPU meshes
    // (#918 T5).
    let near_ready = ring_built
        && tasks
            .cover
            .iter()
            .filter(|key| tasks.near.binary_search(key).is_ok())
            .all(|key| cache.cpu(*key).is_some());
    if ground.1 != near_ready {
        ground.1 = near_ready;
    }
    if ground.2 != all_ready {
        ground.2 = all_ready;
    }
    if tasks.cover == tasks.synced {
        return;
    }
    let PatchTasks {
        synced,
        have,
        cover,
        ..
    } = &mut *tasks;
    // Masks against the whole tree, not only the drawn patches (#817): a
    // hidden coarser neighbour still pins the edge, so culling never snaps
    // an edge when the view moves.
    let mask_of = |key: PatchKey| morph_mask_in(key, &|k| cover.binary_search(&k).is_ok());
    // Despawn entities no longer in the cover; write the mask of the ones
    // whose mask changed into their tile.
    have.clear();
    for (entity, _, piece, held) in owned.iter().filter(|_| !rebuilt) {
        if cover.binary_search(&piece.0).is_err() {
            // The mesh stays with the cache (#918 T5).
            commands.entity(entity).despawn();
            continue;
        }
        have.push(piece.0);
        let mask = mask_of(piece.0);
        if mask != held.0 {
            if let Some(tile) = grid.atlas.tile(piece.0) {
                grid.atlas.set_mask(tile, mask);
            }
            commands.entity(entity).insert(PatchMask(mask));
        }
    }
    have.sort();
    // Spawn missing cover patches with their level's material and their
    // mask in their tile: hidden ones too, so a fast turn never reveals a
    // strip with nothing drawn (#948 R2; the `Aabb` lets the GPU cull them
    // off screen).
    let at_centre = origin.cell();
    for key in cover.iter().copied() {
        if have.binary_search(&key).is_ok() {
            continue;
        }
        let Some(cached) = cache.map.get(&key) else {
            continue;
        };
        let Some((mesh_handle, aabb)) = cached.gpu.clone() else {
            continue;
        };
        let mask = mask_of(key);
        let centre_cell = frame.to_cell(cached.info.centre);
        let crate::patch_grid::GridParams {
            atlas, materials, ..
        } = &mut grid;
        let Some(tile) = atlas.tile(key) else {
            atlas.untiled += 1;
            continue;
        };
        atlas.set_mask(tile, mask);
        let handle = atlas.material(key.level, materials);
        commands.spawn((
            PatchPiece(key),
            PatchMask(mask),
            Name::new(face_name(key)),
            Mesh3d(mesh_handle),
            MeshMaterial3d(handle),
            crate::patch_grid::tag(tile),
            Transform::from_translation(at_centre),
            Anchored(centre_cell),
            aabb,
            NoAutoAabb,
        ));
    }
    synced.clone_from(cover);
    // The drawn ground for the body: the whole tree's draw set (hidden
    // patches included, so the ground under a first-person body is there),
    // masks as drawn (#779 Q1 A), where the full CPU mesh is held: near the
    // player (#918 T5).
    let patches: Vec<universe_core::drawn_surface::DrawnPatch> = cover
        .iter()
        .filter_map(|key| {
            let mesh = cache.cpu(*key)?;
            Some((*key, std::sync::Arc::clone(mesh), mask_of(*key)))
        })
        .collect();
    ground.0 = Some(universe_core::drawn_surface::DrawnSurface::new(
        frame, camera, detail_px, patches,
    ));
}

#[cfg(test)]
#[path = "patches_tests.rs"]
mod tests;
