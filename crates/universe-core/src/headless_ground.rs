//! Headless `GROUND` line for near-field physics (rebuild step 6, #779
//! Q6 A).
//!
//! On the demo Earth, the body starts standing on the drawn ground over the
//! mountain slope and over a plain (Q7 A). On each it walks 100 m and runs
//! 100 m at 60 frames a second, with one jump, through the CDLOD selection
//! and the drawn surface the window uses (the eye is the camera). Every
//! frame on the ground, the gap between the feet and the drawn ground under
//! them is measured as the frame draws it (the body steps on the blend of
//! the last drawn camera, as in the window; the frame then blends from the
//! new eye); after the jump, the landing gap. Each step also runs
//! once in the planet frame, and the local tangent frame's result must
//! match it (the frame switch, re-anchors included).
//!
//! The selection updates every [`SELECT_EVERY`](crate::headless_ground::SELECT_EVERY) frames (at walking pace the
//! tree changes slowly); the morph camera, and so the blend, follows the
//! eye every frame. Deterministic (`E-DET-TIERS`), std only.

use crate::body::{Body, BodyInput, Ground, Turning, body_step, eye_height};
use crate::body_frame::{BodyAnchor, step_near_ground};
use crate::coords::Level;
use crate::drawn_surface::{DrawnPatch, DrawnSurface};
use crate::lod::PatchKey;
use crate::lod_mesh::PatchMesh;
use crate::lod_morph::morph_mask_in;
use crate::lod_range::LevelRanges;
use crate::lod_select::{QuadTree, UpdateScratch};
use crate::planet_frame::{EARTH_RADIUS_M, PlanetFrame, PlanetPos};
use crate::relief::Relief;
use std::collections::HashMap;
use std::sync::Arc;

/// Frames between selection updates on the walk.
pub const SELECT_EVERY: u32 = 10;

/// Feet-to-ground limit while walking, running and after landing (Q6 A).
pub const FEET_LIMIT_M: f64 = 0.01;

/// Frame-switch limit (AC2).
pub const SWITCH_LIMIT_M: f64 = 0.001;

/// One `GROUND` run's numbers (metres).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GroundMetrics {
    /// Worst feet gap while walking.
    pub walk_gap_m: f64,
    /// Worst feet gap while running.
    pub run_gap_m: f64,
    /// Worst feet gap on the frame after landing from a jump.
    pub land_gap_m: f64,
    /// Worst difference between the local-frame step and the planet-frame
    /// step.
    pub switch_m: f64,
    /// Jumps that landed.
    pub landings: u32,
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let len = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if len > 0.0 && len.is_finite() {
        [a[0] / len, a[1] / len, a[2] / len]
    } else {
        [0.0, 1.0, 0.0]
    }
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// The demo Earth's relief and planet frame.
fn earth() -> (Relief, PlanetFrame) {
    let journey = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
    let chain: &[u32] = journey.get(..9).unwrap_or(&[]);
    (
        Relief::for_planet(crate::nav::DEMO_SEED, chain),
        PlanetFrame::for_planet(crate::nav::DEMO_SEED, chain),
    )
}

/// The selection, its builds and the drawn surface around one camera.
struct Drawn {
    tree: QuadTree,
    scratch: UpdateScratch,
    memo: HashMap<PatchKey, (f64, f64, f64)>,
    meshes: HashMap<PatchKey, PatchMesh>,
    shared: HashMap<PatchKey, Arc<PatchMesh>>,
    quad_px: f64,
}

impl Drawn {
    fn new(relief: &Relief) -> Option<Drawn> {
        let quad_px = crate::settings::PlanetDetail::Normal.quad_px();
        let tree = QuadTree::new(EARTH_RADIUS_M)?.with_ranges(LevelRanges::new(
            relief,
            EARTH_RADIUS_M,
            quad_px,
        ));
        Some(Drawn {
            tree,
            scratch: UpdateScratch::default(),
            memo: HashMap::new(),
            meshes: HashMap::new(),
            shared: HashMap::new(),
            quad_px,
        })
    }

    /// One selection update at `camera`, then every leaf built.
    fn update(&mut self, relief: &Relief, camera: PlanetPos) {
        {
            let memo = &self.memo;
            self.tree.update_with_relief_reused(
                camera,
                self.quad_px,
                &|k| memo.get(&k).copied(),
                &mut self.scratch,
            );
        }
        let leaves = self.tree.leaves();
        crate::headless_morph::build_keys(&leaves, relief, &mut self.memo, &mut self.meshes);
    }

    /// The drawn surface of the current tree, blended from `camera`.
    fn surface(&mut self, frame: PlanetFrame, camera: PlanetPos) -> DrawnSurface {
        let leaves = self.tree.leaves();
        let is_drawn = |key: PatchKey| leaves.binary_search(&key).is_ok();
        let mut patches: Vec<DrawnPatch> = Vec::with_capacity(leaves.len());
        for key in &leaves {
            let Some(mesh) = self.meshes.get(key) else {
                continue;
            };
            let shared = self
                .shared
                .entry(*key)
                .or_insert_with(|| Arc::new(mesh.clone()))
                .clone();
            patches.push((*key, shared, morph_mask_in(*key, &is_drawn)));
        }
        DrawnSurface::new(frame, camera, self.quad_px, patches)
    }
}

/// Height of `position` (cells) above the drawn ground, in metres.
fn feet_gap_m(frame: &PlanetFrame, surface: &DrawnSurface, position: [f64; 3]) -> Option<f64> {
    let g = surface.ground_at(position)?;
    let p = frame.from_cell(position).0;
    let q = frame.from_cell(g.point).0;
    Some((p[0] - q[0]) * g.up[0] + (p[1] - q[1]) * g.up[1] + (p[2] - q[2]) * g.up[2])
}

/// Walks and runs over the ground at unit `dir`, folding into `m`.
fn walk_at(
    relief: &Relief,
    frame: PlanetFrame,
    dir: [f64; 3],
    m: &mut GroundMetrics,
) -> Option<()> {
    let level = Level::new(10)?;
    let mpc = frame.metres_per_cell();
    let up = unit(dir);
    let eye_m = eye_height(level) * mpc;
    let r = EARTH_RADIUS_M + relief.height_m(up) + eye_m;
    let mut camera = PlanetPos([up[0] * r, up[1] * r, up[2] * r]);
    let mut drawn = Drawn::new(relief)?;
    for _ in 0..40 {
        drawn.update(relief, camera);
    }
    let mut surface = drawn.surface(frame, camera);
    // Start standing on the drawn ground (Q7 A).
    let feet = surface.place_on_ground(frame.to_cell(camera))?;
    let mut body = Body::standing(feet, 0.4);
    let start = frame.from_cell(feet).0;
    let mut anchor = BodyAnchor::default();
    let dt = 1.0 / 60.0;
    let mut jumped = false;
    let mut was_airborne = false;
    for frame_no in 0..20_000u32 {
        let walked = dist(frame.from_cell(body.position).0, start);
        if walked >= 200.0 {
            break;
        }
        let running = walked >= 100.0;
        // One jump, a quarter of the way along.
        let jump = !jumped && walked >= 50.0 && body.on_ground;
        jumped |= jump;
        let input = BodyInput {
            forward: true,
            fast: running,
            space_pressed: jump,
            ..BodyInput::default()
        };
        // As in the window: the body steps on the surface blended for the
        // last drawn camera; the frame is then drawn from the new eye.
        if frame_no % SELECT_EVERY == 0 {
            drawn.update(relief, camera);
            surface = drawn.surface(frame, camera);
        } else {
            surface.set_camera(camera);
        }
        let ground = |p: [f64; 3]| -> Option<Ground> { surface.ground_at(p) };
        let down = unit(frame.from_cell(body.position).0);
        let g = 9.81 / mpc;
        let pull = [-down[0] * g, -down[1] * g, -down[2] * g];
        let before = body;
        body = step_near_ground(
            body,
            input,
            level,
            &frame,
            &mut anchor,
            ground,
            pull,
            Turning::Drone,
            1.0,
            true,
            0.4,
            false,
            dt,
        );
        let plain = body_step(
            before,
            input,
            level,
            ground,
            pull,
            Turning::Drone,
            1.0,
            true,
            0.4,
            false,
            dt,
        );
        m.switch_m = m.switch_m.max(dist(
            frame.from_cell(body.position).0,
            frame.from_cell(plain.position).0,
        ));
        // The eye is the camera: the drawn blend follows it.
        let eye = frame.from_cell(body.position).0;
        let eye_up = unit(eye);
        camera = PlanetPos([
            eye[0] + eye_up[0] * eye_m,
            eye[1] + eye_up[1] * eye_m,
            eye[2] + eye_up[2] * eye_m,
        ]);
        if body.on_ground {
            // Feet against the ground as drawn this frame (new camera).
            surface.set_camera(camera);
            let gap = feet_gap_m(&frame, &surface, body.position)?.abs();
            if was_airborne {
                m.land_gap_m = m.land_gap_m.max(gap);
                m.landings += 1;
            } else if running {
                m.run_gap_m = m.run_gap_m.max(gap);
            } else {
                m.walk_gap_m = m.walk_gap_m.max(gap);
            }
        }
        was_airborne = !body.on_ground;
    }
    Some(())
}

/// Runs the walks over the mountain slope and over a plain.
#[must_use]
pub fn ground_metrics() -> GroundMetrics {
    let (relief, frame) = earth();
    let (mountain, plains) = crate::headless_morph::earth_mountain_and_plains_dirs();
    let mut m = GroundMetrics::default();
    for dir in [mountain, plains] {
        if walk_at(&relief, frame, dir, &mut m).is_none() {
            m.walk_gap_m = f64::INFINITY;
        }
    }
    m
}

/// Checks the walks and prints one `GROUND` line (millimetres).
pub fn verify_ground(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let m = ground_metrics();
    let passed = m.walk_gap_m < FEET_LIMIT_M
        && m.run_gap_m < FEET_LIMIT_M
        && m.land_gap_m < FEET_LIMIT_M
        && m.landings >= 2
        && m.switch_m < SWITCH_LIMIT_M;
    let _ = writeln!(
        out,
        "GROUND walk_gap_mm={:.3} run_gap_mm={:.3} land_gap_mm={:.3} landings={} frame_switch_mm={:.3} {}",
        m.walk_gap_m * 1000.0,
        m.run_gap_m * 1000.0,
        m.land_gap_m * 1000.0,
        m.landings,
        m.switch_m * 1000.0,
        if passed { "PASS" } else { "FAIL" },
    );
    passed
}
