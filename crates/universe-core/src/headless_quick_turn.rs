//! Quick-turn ground coverage probe (#948 T1).
//!
//! Stands the camera 2 m over the plains of the demo home planet, settles
//! the culled selection with the 500 m ring (every wanted leaf built), then
//! replays the two fast A/D motions from the owner's report with a
//! dev-PC-scale build budget per frame:
//! - fast yaw: third-person A/D turns the body at [`BODY_TURN_RATE`];
//! - fast strafe: first-person A/D strafes at run speed, look fixed.
//!
//! Each frame counts the visible leaves with no built mesh (`holes`) and,
//! among them, the ones with no built ancestor either (`bare`: what the
//! player sees as black). Softness (coarse cover drawn instead) is allowed
//! by the spec; black is not.
//!
//! Deterministic (`E-DET-TIERS`), std only (`E-CORE-NO-BEVY`).

use crate::body::RUN_M_PER_S;
use crate::lod::PatchKey;
use crate::lod_cull::{Cull, View};
use crate::lod_mesh::build_patch;
use crate::lod_ring::RING_M;
use crate::lod_select::{QuadTree, UpdateScratch};
use crate::motion::BODY_TURN_RATE;
use crate::planet_frame::{EARTH_RADIUS_M, PlanetPos};
use crate::precision::{FOV_DEG, IMAGE_H_PX};
use crate::relief::Relief;
use std::collections::HashMap;

/// Camera height over the ground in metres.
const HEIGHT_M: f64 = 2.0;

/// Image width for the view's aspect (1600 x 900).
const IMAGE_W_PX: f64 = 1_600.0;

/// Modelled frames per second.
const FPS: f64 = 60.0;

/// Builds finished per frame: dev-PC scale (about 11 builds a frame on the
/// dev PC, #918 T8). Fixed, so runs are identical everywhere.
pub(crate) const BUILDS_PER_FRAME: usize = 12;

/// Fast-yaw frames: 20 frames at [`BODY_TURN_RATE`] turn the camera about
/// 180° (a hard A/D flick).
const YAW_FRAMES: usize = 20;

/// Fast-strafe frames: one second of run-speed strafe at 60 fps.
const STRAFE_FRAMES: usize = 60;

/// Most frames to settle before the turns.
const SETTLE_FRAMES: usize = 40;

/// One quick-turn run's numbers.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct QuickTurnMetrics {
    /// The arrival settled with every wanted leaf built.
    pub settled: bool,
    /// Most visible-but-unbuilt leaves in one fast-yaw frame.
    pub yaw_holes_max: usize,
    /// Most of them with no built ancestor (seen as black).
    pub yaw_bare_max: usize,
    /// New builds the fast yaw demanded past the settled set.
    pub yaw_new_builds: usize,
    /// Most visible-but-unbuilt leaves in one fast-strafe frame.
    pub strafe_holes_max: usize,
    /// Most of them with no built ancestor (seen as black).
    pub strafe_bare_max: usize,
    /// New builds the fast strafe demanded past the settled set.
    pub strafe_new_builds: usize,
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

/// One culled update; returns the visible leaves and the wanted leaves.
fn update(
    tree: &mut QuadTree,
    camera: PlanetPos,
    heading_deg: f64,
    built: &Built,
    bounds: (f64, f64),
    scratch: &mut UpdateScratch,
) -> Option<(Vec<PatchKey>, Vec<PatchKey>)> {
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
    let mut visible = Vec::new();
    tree.append_visible_leaves_to(&mut visible);
    let mut wanted = Vec::new();
    tree.append_wanted_to(&mut wanted);
    Some((visible, wanted))
}

/// Builds up to `budget` keys of `wanted` (in order) not built yet.
fn build_up_to(wanted: &[PatchKey], budget: usize, relief: &Relief, built: &mut Built) {
    let missing: Vec<PatchKey> = wanted
        .iter()
        .copied()
        .filter(|key| !built.contains_key(key))
        .take(budget)
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

/// Counts visible leaves with no built mesh, and among them the ones with
/// no built ancestor either.
fn count_holes(visible: &[PatchKey], built: &Built) -> (usize, usize) {
    let mut holes = 0usize;
    let mut bare = 0usize;
    for key in visible.iter().copied() {
        if built.contains_key(&key) {
            continue;
        }
        holes += 1;
        let mut ancestor = key.parent();
        let mut covered = false;
        while let Some(parent) = ancestor {
            if built.contains_key(&parent) {
                covered = true;
                break;
            }
            ancestor = parent.parent();
        }
        if !covered {
            bare += 1;
        }
    }
    (holes, bare)
}

/// Runs the arrival and the two fast turns (see the module docs).
#[must_use]
pub(crate) fn quick_turn_metrics() -> QuickTurnMetrics {
    let relief = crate::headless_morph::earth_relief();
    let (_, plains_dir) = crate::headless_morph::earth_mountain_and_plains_dirs();
    let start = crate::headless_morph::camera_for(&relief, HEIGHT_M, plains_dir);
    let bounds = relief.height_bounds_m();
    let quad_px = crate::settings::PlanetDetail::Normal.quad_px();
    let ranges = crate::lod_range::LevelRanges::new(&relief, EARTH_RADIUS_M, quad_px);
    let Some(mut tree) = QuadTree::new(EARTH_RADIUS_M).map(|t| t.with_ranges(ranges)) else {
        return QuickTurnMetrics::default();
    };
    tree.set_ring_m(RING_M);
    let mut scratch = UpdateScratch::default();
    let mut built = Built::new();
    let mut metrics = QuickTurnMetrics::default();
    // Arrival: settle with every wanted leaf built each frame.
    for _ in 0..SETTLE_FRAMES {
        let Some((_, wanted)) = update(&mut tree, start, 0.0, &built, bounds, &mut scratch) else {
            return metrics;
        };
        if wanted.iter().all(|key| built.contains_key(key)) {
            metrics.settled = true;
            break;
        }
        build_up_to(&wanted, usize::MAX, &relief, &mut built);
    }
    if !metrics.settled {
        return metrics;
    }
    // Fast yaw: a hard third-person A/D flick at BODY_TURN_RATE.
    let yaw_step_deg = BODY_TURN_RATE.to_degrees() / FPS;
    let mut heading = 0.0f64;
    let built_before = built.len();
    for _ in 0..YAW_FRAMES {
        heading += yaw_step_deg;
        let Some((visible, wanted)) =
            update(&mut tree, start, heading, &built, bounds, &mut scratch)
        else {
            return metrics;
        };
        let (holes, bare) = count_holes(&visible, &built);
        metrics.yaw_holes_max = metrics.yaw_holes_max.max(holes);
        metrics.yaw_bare_max = metrics.yaw_bare_max.max(bare);
        build_up_to(&wanted, BUILDS_PER_FRAME, &relief, &mut built);
    }
    metrics.yaw_new_builds = built.len().saturating_sub(built_before);
    // Fast strafe: one second of first-person A/D run-strafe, look fixed.
    let up = unit(start.0);
    let helper = if up[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let east = unit(cross(helper, up));
    let step = RUN_M_PER_S / FPS;
    let mut camera = start;
    let built_before = built.len();
    for _ in 0..STRAFE_FRAMES {
        let moved = [
            camera.0[0] + east[0] * step,
            camera.0[1] + east[1] * step,
            camera.0[2] + east[2] * step,
        ];
        let len = (moved[0] * moved[0] + moved[1] * moved[1] + moved[2] * moved[2]).sqrt();
        let keep = if len > 0.0 && len.is_finite() {
            (EARTH_RADIUS_M + HEIGHT_M) / len
        } else {
            1.0
        };
        camera = PlanetPos([moved[0] * keep, moved[1] * keep, moved[2] * keep]);
        let Some((visible, wanted)) = update(&mut tree, camera, 0.0, &built, bounds, &mut scratch)
        else {
            return metrics;
        };
        let (holes, bare) = count_holes(&visible, &built);
        metrics.strafe_holes_max = metrics.strafe_holes_max.max(holes);
        metrics.strafe_bare_max = metrics.strafe_bare_max.max(bare);
        build_up_to(&wanted, BUILDS_PER_FRAME, &relief, &mut built);
    }
    metrics.strafe_new_builds = built.len().saturating_sub(built_before);
    metrics
}

#[cfg(test)]
mod tests {
    use super::{BUILDS_PER_FRAME, quick_turn_metrics};
    use std::sync::OnceLock;

    /// One arrival and two fast turns, shared by the tests below.
    fn metrics() -> &'static super::QuickTurnMetrics {
        static METRICS: OnceLock<super::QuickTurnMetrics> = OnceLock::new();
        METRICS.get_or_init(quick_turn_metrics)
    }

    #[test]
    fn fast_yaw_keeps_side_ground_built() {
        // #948 AC1/AC2 (yaw half): a hard A/D flick must find the side
        // ground built: no visible-but-unbuilt leaf, and nothing bare.
        let m = metrics();
        assert!(m.settled, "the arrival settles: {m:?}");
        assert_eq!(m.yaw_holes_max, 0, "{m:?}");
        assert_eq!(m.yaw_bare_max, 0, "{m:?}");
    }

    #[test]
    fn fast_strafe_keeps_side_ground_built() {
        // #948 AC1 (strafe half): a second of run-speed A/D strafe must
        // find the side ground built too.
        let m = metrics();
        assert!(m.settled, "the arrival settles: {m:?}");
        assert_eq!(m.strafe_holes_max, 0, "{m:?}");
        assert_eq!(m.strafe_bare_max, 0, "{m:?}");
    }

    #[test]
    fn probe_models_a_real_frame_budget() {
        // The probe is only meaningful with a dev-PC-scale budget: it must
        // be far below settling a whole turn at once.
        const _: () = assert!(BUILDS_PER_FRAME <= 24);
    }
}
