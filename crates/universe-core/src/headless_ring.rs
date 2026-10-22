//! Headless `RING` line for the ground ring (#918 T2, AC2, AC4).
//!
//! Stands the camera 2 m over the plains of the demo home planet, looking
//! 10° down, and lets the culled selection with the 500 m ring settle with
//! every wanted leaf built. Then it turns the camera a full 360° in 30°
//! steps and counts:
//! - builds the turn needs inside the ring (must be 0);
//! - changes to the ring's leaves (must be 0: no split or merge near the
//!   player while the camera only turns);
//! - whether the first build of the arrival is the nearest leaf, with the
//!   rest in distance order.
//!
//! Deterministic (`E-DET-TIERS`), std only (`E-CORE-NO-BEVY`).

use crate::lod::PatchKey;
use crate::lod_cull::{Cull, View};
use crate::lod_mesh::build_patch;
use crate::lod_ring::{RING_M, box_distance_m};
use crate::lod_select::{QuadTree, UpdateScratch};
use crate::planet_frame::{EARTH_RADIUS_M, PlanetPos};
use crate::precision::{FOV_DEG, IMAGE_H_PX};
use crate::relief::Relief;
use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;

/// Camera height over the ground in metres.
const HEIGHT_M: f64 = 2.0;

/// Image width for the view's aspect (1600 x 900).
const IMAGE_W_PX: f64 = 1_600.0;

/// Turn step in degrees.
const TURN_STEP_DEG: u32 = 30;

/// Most frames to settle before the turn.
const SETTLE_FRAMES: usize = 40;

/// One `RING` run's numbers.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RingMetrics {
    /// Leaves inside the ring after settling.
    pub ring_patches: usize,
    /// Leaves wanted (visible plus ring) after settling.
    pub wanted: usize,
    /// Leaves inside the ring the turn had to build (must be 0).
    pub turn_builds: usize,
    /// Ring leaves that split or merged during the turn (must be 0).
    pub ring_changes: usize,
    /// The arrival's wanted list starts with the nearest leaf and is in
    /// distance order.
    pub nearest_first: bool,
    /// Every wanted leaf was built before the turn.
    pub settled: bool,
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let len = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if len > 0.0 && len.is_finite() {
        [a[0] / len, a[1] / len, a[2] / len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// A view at `camera` looking 10° down toward compass `heading_deg`.
fn view_at(camera: PlanetPos, heading_deg: f64) -> Option<View> {
    let up = unit(camera.0);
    let helper = if up[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let east = unit(cross(helper, up));
    let north = cross(up, east);
    let (h, tilt) = (heading_deg.to_radians(), 10f64.to_radians());
    let flat = [
        north[0] * h.cos() + east[0] * h.sin(),
        north[1] * h.cos() + east[1] * h.sin(),
        north[2] * h.cos() + east[2] * h.sin(),
    ];
    let forward = [
        flat[0] * tilt.cos() - up[0] * tilt.sin(),
        flat[1] * tilt.cos() - up[1] * tilt.sin(),
        flat[2] * tilt.cos() - up[2] * tilt.sin(),
    ];
    View::new(
        camera,
        forward,
        up,
        FOV_DEG.to_radians(),
        IMAGE_W_PX / IMAGE_H_PX,
    )
}

/// Built patches' relief info and height range, by key.
type Built = HashMap<PatchKey, ((f64, f64, f64), (f64, f64))>;

/// One culled update at `heading_deg`; returns the wanted leaves.
fn update(
    tree: &mut QuadTree,
    camera: PlanetPos,
    heading_deg: f64,
    built: &Built,
    bounds: (f64, f64),
    scratch: &mut UpdateScratch,
) -> Option<Vec<PatchKey>> {
    let view = view_at(camera, heading_deg)?;
    let lookup = |key: PatchKey| built.get(&key).map(|b| b.0);
    let heights = |key: PatchKey| built.get(&key).map(|b| b.1);
    let cull = Cull {
        view,
        heights: &heights,
        fallback: bounds,
        occluder_m: EARTH_RADIUS_M + bounds.0,
    };
    let quad_px = crate::settings::PlanetDetail::Normal.quad_px();
    tree.update_culled_reused(camera, quad_px, &lookup, Some(&cull), scratch);
    let mut wanted = Vec::new();
    tree.append_wanted_to(&mut wanted);
    Some(wanted)
}

/// Builds every key of `keys` not built yet, on every core.
fn build_missing(keys: &[PatchKey], relief: &Relief, built: &mut Built) {
    let missing: Vec<PatchKey> = keys
        .iter()
        .copied()
        .filter(|key| !built.contains_key(key))
        .collect();
    let meshes = crate::headless_morph_measure::par_chunks(&missing, |chunk| {
        chunk
            .iter()
            .filter_map(|key| build_patch(*key, EARTH_RADIUS_M, relief))
            .map(|mesh| (mesh.key, (mesh.relief_info(), (mesh.min_h_m, mesh.max_h_m))))
            .collect::<Vec<_>>()
    });
    built.extend(meshes.into_iter().flatten());
}

/// The ring's leaves at `camera`.
fn ring_set(tree: &QuadTree, camera: PlanetPos, built: &Built) -> BTreeSet<PatchKey> {
    let lookup = |key: PatchKey| built.get(&key).map(|b| b.0);
    tree.leaves()
        .into_iter()
        .filter(|key| {
            box_distance_m(*key, camera, EARTH_RADIUS_M, &lookup).is_some_and(|d| d < RING_M)
        })
        .collect()
}

/// Runs the arrival and the turn (see the module docs).
#[must_use]
pub fn ring_metrics() -> RingMetrics {
    let relief = crate::headless_morph::earth_relief();
    let (_, plains_dir) = crate::headless_morph::earth_mountain_and_plains_dirs();
    let camera = crate::headless_morph::camera_for(&relief, HEIGHT_M, plains_dir);
    let bounds = relief.height_bounds_m();
    let quad_px = crate::settings::PlanetDetail::Normal.quad_px();
    let ranges = crate::lod_range::LevelRanges::new(&relief, EARTH_RADIUS_M, quad_px);
    let Some(mut tree) = QuadTree::new(EARTH_RADIUS_M).map(|t| t.with_ranges(ranges)) else {
        return RingMetrics::default();
    };
    tree.set_ring_m(RING_M);
    let mut scratch = UpdateScratch::default();
    let mut built = Built::new();
    let mut metrics = RingMetrics::default();
    // Arrival: settle with every wanted leaf built each frame.
    for frame in 0..SETTLE_FRAMES {
        let Some(wanted) = update(&mut tree, camera, 0.0, &built, bounds, &mut scratch) else {
            return metrics;
        };
        if frame == 0 {
            let lookup = |key: PatchKey| built.get(&key).map(|b| b.0);
            let distances: Vec<f64> = wanted
                .iter()
                .map(|key| {
                    box_distance_m(*key, camera, EARTH_RADIUS_M, &lookup).unwrap_or(f64::INFINITY)
                })
                .collect();
            metrics.nearest_first = !distances.is_empty()
                && distances
                    .iter()
                    .zip(distances.iter().skip(1))
                    .all(|(a, b)| a <= b);
        }
        if wanted.iter().all(|key| built.contains_key(key)) {
            metrics.settled = true;
            metrics.wanted = wanted.len();
            break;
        }
        build_missing(&wanted, &relief, &mut built);
    }
    let before = ring_set(&tree, camera, &built);
    metrics.ring_patches = before.len();
    // The turn: count ring builds and ring changes, then build what the new
    // view wants beyond the ring.
    for step in 1..=360 / TURN_STEP_DEG {
        let heading = f64::from(step * TURN_STEP_DEG);
        let Some(wanted) = update(&mut tree, camera, heading, &built, bounds, &mut scratch) else {
            return metrics;
        };
        let now = ring_set(&tree, camera, &built);
        metrics.ring_changes += before.symmetric_difference(&now).count();
        metrics.turn_builds += now.iter().filter(|key| !built.contains_key(key)).count();
        build_missing(&wanted, &relief, &mut built);
    }
    metrics
}

/// Checks the ring and prints one `RING` line.
pub fn verify_ring(out: &mut String) -> bool {
    let m = ring_metrics();
    let pass = m.settled
        && m.nearest_first
        && m.turn_builds == 0
        && m.ring_changes == 0
        && m.ring_patches > 0;
    let _ = writeln!(
        out,
        "RING ring_m={RING_M:.0} ring_patches={} wanted={} turn_builds={} ring_changes={} nearest_first={} {}",
        m.ring_patches,
        m.wanted,
        m.turn_builds,
        m.ring_changes,
        if m.nearest_first { "yes" } else { "no" },
        if pass { "PASS" } else { "FAIL" }
    );
    pass
}
