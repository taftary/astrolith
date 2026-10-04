//! Free-flight synthesis and the scripted headless leg (#152).
//!
//! Pure steering math shared by the window and the headless proof: look
//! directions, distance-scaled speed rungs, the per-step integrator, and the
//! fixed-program leg replayed after the dive. Everything here is
//! deterministic in its inputs (`E-DET-TIERS`).

use crate::coords::Level;
use crate::r#gen::{Generated, MarkerKind};
use crate::nav::{AUTOPILOT_RATE, DiveEvent, DiveMode, Universe, add3, dot3, length3, mul3, sub3};
use crate::preview::PreviewCache;
use crate::stream::{StreamLedger, working_chains};

/// Default free-flight speed step (`Shift+1-9`): the surface-distance rate.
pub const FREE_SPEED_DEFAULT: u32 = 5;

/// Free-flight pitch clamp, in radians: 89 degrees either way (#152).
///
/// The look direction never runs parallel to up, so the strafe basis stays
/// exact in closed form.
pub const FREE_PITCH_LIMIT: f64 = 89.0 * std::f64::consts::PI / 180.0;

/// Floor for the surface-distance speed scale, in open-cell units (#152).
///
/// Keeps flight moving when parked exactly on a marker surface.
pub const MIN_FREE_GAP: f64 = 1e-3;

/// Nearest portal marker to `camera` in `open`, if any (#152).
///
/// Leaving free flight targets it. Populations never qualify. Pure and
/// deterministic: ties keep the smallest index.
#[must_use]
pub fn nearest_portal(camera: [f64; 3], open: &Generated) -> Option<u32> {
    let mut best: Option<(u32, f64)> = None;
    for (index, point) in open.points.iter().enumerate() {
        if point.kind != MarkerKind::Portal {
            continue;
        }
        let distance = length3(sub3(camera, point.position));
        let closer = match best {
            None => true,
            Some((_, held)) => distance < held,
        };
        if closer {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: marker index into a budgeted cell, always fits u32"
            )]
            let marker = index as u32;
            best = Some((marker, distance));
        }
    }
    best.map(|(marker, _)| marker)
}

/// Distance from `camera` to the nearest marker surface in `open` (#152).
///
/// Falls back to the distance to the cell center when no marker is closer;
/// never negative. Drives the distance-scaled flight speed and the free
/// camera's near plane.
#[must_use]
pub fn nearest_surface_distance(camera: [f64; 3], open: &Generated, radius: f64) -> f64 {
    let mut nearest = length3(camera);
    for point in &open.points {
        let gap = length3(sub3(camera, point.position)) - radius;
        if gap < nearest {
            nearest = gap;
        }
    }
    nearest.max(0.0)
}

/// Free-flight speed multiplier for step `1..=9` (#152).
///
/// Exponential rungs doubling per step, so step 5 flies at the
/// surface-distance rate. Out-of-range steps clamp to the ends.
#[must_use]
pub fn free_speed_multiplier(step: u32) -> f64 {
    match step.clamp(1, 9) {
        1 => 0.0625,
        2 => 0.125,
        3 => 0.25,
        4 => 0.5,
        5 => 1.0,
        6 => 2.0,
        7 => 4.0,
        8 => 8.0,
        _ => 16.0,
    }
}

/// Forward vector for free-flight look angles (#152).
///
/// `yaw` circles around Y (zero faces `-Z`, the Bevy camera forward) and
/// `pitch` rises toward `+Y`, clamped to [`FREE_PITCH_LIMIT`].
#[must_use]
pub fn free_look_direction(yaw: f64, pitch: f64) -> [f64; 3] {
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let (sin_pitch, cos_pitch) = pitch.clamp(-FREE_PITCH_LIMIT, FREE_PITCH_LIMIT).sin_cos();
    [-sin_yaw * cos_pitch, sin_pitch, -cos_yaw * cos_pitch]
}

/// Free-flight camera pose integrated by [`free_flight_step`] (#152).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreePose {
    /// Camera position in open-cell units.
    pub camera: [f64; 3],
    /// Look yaw around Y; zero faces `-Z`.
    pub yaw: f64,
    /// Look pitch toward `+Y`.
    pub pitch: f64,
}

/// Held steering keys for one free-flight step (#152).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FreeKeys {
    /// Forward along the look direction (`W`).
    pub forward: bool,
    /// Back along the look direction (`S`).
    pub back: bool,
    /// Strafe left (`A`).
    pub left: bool,
    /// Strafe right (`D`).
    pub right: bool,
}

/// Advances a free-flight camera by one step (#152).
///
/// The wish direction combines the held keys in the look frame, scaled by
/// the surface-distance rate times the speed step over `dt`. Deterministic
/// in every input (`E-DET-TIERS`): the window and the headless leg share
/// this synthesis. No keys, or a non-positive `dt`, holds position;
/// non-finite inputs hold position.
#[must_use]
pub fn free_flight_step(
    pose: FreePose,
    keys: FreeKeys,
    speed_step: u32,
    open: &Generated,
    radius: f64,
    dt: f64,
) -> [f64; 3] {
    let FreePose { camera, yaw, pitch } = pose;
    if !dt.is_finite() || dt <= 0.0 {
        return camera;
    }
    let forward = free_look_direction(yaw, pitch);
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let right = [cos_yaw, 0.0, -sin_yaw];
    let mut wish = [0.0; 3];
    if keys.forward {
        wish = add3(wish, forward);
    }
    if keys.back {
        wish = sub3(wish, forward);
    }
    if keys.right {
        wish = add3(wish, right);
    }
    if keys.left {
        wish = sub3(wish, right);
    }
    let squared = dot3(wish, wish);
    if !squared.is_finite() || squared <= 0.0 {
        return camera;
    }
    let surface = nearest_surface_distance(camera, open, radius).max(MIN_FREE_GAP);
    let travelled = surface * free_speed_multiplier(speed_step) * dt;
    if !travelled.is_finite() {
        return camera;
    }
    let moved = add3(camera, mul3(wish, travelled / squared.sqrt()));
    if moved.iter().all(|component| component.is_finite()) {
        moved
    } else {
        camera
    }
}

/// One recorded sample of the scripted free-flight leg (#152).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreeLegPoint {
    /// Seconds since the leg started.
    pub elapsed: f64,
    /// Camera offset in open-cell units.
    pub offset: [f64; 3],
    /// Working-set cells live this sample.
    pub alive: usize,
}

/// Scripted free-flight leg replayed headlessly (#152).
///
/// Steers a fixed program after the dive with the same synthesis the window
/// uses ([`free_flight_step`]), so the leg is byte-identical every run.
#[derive(Debug, Clone, PartialEq)]
pub struct FreeLegReplay {
    /// Level the leg steers in (the dive's L5 cell).
    pub level: Level,
    /// Snapshot of the leg's cell: equals the dive's own L5 snapshot.
    pub start_snapshot: String,
    /// Camera offset the leg steered from.
    pub start_offset: [f64; 3],
    /// One sample per steering step.
    pub points: Vec<FreeLegPoint>,
    /// Most live cells in any sample (bound proof).
    pub alive_max: usize,
}

/// Fixed steering program: keys, yaw, pitch, speed step, step count (#152).
const FREE_LEG_SCRIPT: [(FreeKeys, f64, f64, u32, usize); 3] = [
    (
        FreeKeys {
            forward: true,
            back: false,
            left: false,
            right: false,
        },
        0.0,
        0.0,
        5,
        120,
    ),
    (
        FreeKeys {
            forward: true,
            back: false,
            left: false,
            right: false,
        },
        0.6,
        0.1,
        6,
        120,
    ),
    (
        FreeKeys {
            forward: false,
            back: false,
            left: false,
            right: true,
        },
        0.6,
        0.1,
        4,
        60,
    ),
];

/// Replays the scripted free-flight leg after the dive (#152).
///
/// Re-dives the fixed journey to its L5 cell (target-only, exactly like
/// [`crate::nav::replay_autopilot`]; the starting snapshot proves the two agree), then
/// steers the fixed program with [`free_flight_step`]: no opens, no closes,
/// previews and the ledger tracked every step. Returns the leg replay and
/// the steered universe.
#[must_use]
pub fn replay_free_leg(root: u64, dt: f64, max_secs: f64) -> (FreeLegReplay, Universe) {
    let mut universe = Universe::new(root);
    let mut target = None;
    let mut elapsed = 0.0;
    let factor = (-AUTOPILOT_RATE * dt).exp();
    while universe.level().get() < 5 && elapsed < max_secs {
        if target.is_none() {
            target = universe.autopilot_target();
            if target.is_none() {
                break;
            }
        }
        elapsed += dt;
        match universe.dive(target, factor, 0.0, DiveMode::Targeted) {
            DiveEvent::Opened(_) => target = None,
            DiveEvent::Closed(opened) => target = Some(opened.marker),
            DiveEvent::Moved => {}
        }
    }
    let mut previews = PreviewCache::default();
    let mut ledger = StreamLedger::default();
    let mut points: Vec<FreeLegPoint> = Vec::new();
    let mut alive_max = 0usize;
    let mut leg_elapsed = 0.0;
    let start_offset = universe.path.offset();
    for (keys, yaw, pitch, step, count) in FREE_LEG_SCRIPT {
        for _ in 0..count {
            let pose = FreePose {
                camera: universe.path.offset(),
                yaw,
                pitch,
            };
            let next = free_flight_step(
                pose,
                keys,
                step,
                &universe.open,
                universe.marker_radius(),
                dt,
            );
            universe.path.set_offset(next);
            previews.sync(&universe);
            let chains = working_chains(&universe.path, &previews);
            ledger.sync(root, &chains);
            leg_elapsed += dt;
            alive_max = alive_max.max(ledger.len());
            points.push(FreeLegPoint {
                elapsed: leg_elapsed,
                offset: universe.path.offset(),
                alive: ledger.len(),
            });
        }
    }
    let replay = FreeLegReplay {
        level: universe.level(),
        start_snapshot: universe.snapshot(),
        start_offset,
        points,
        alive_max,
    };
    (replay, universe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::{DEMO_SEED, replay_autopilot};
    use crate::stream::STREAM_CAP;

    #[test]
    fn nearest_portal_skips_populations_and_ties_keep_order() {
        use crate::r#gen::Point;
        let open = Generated {
            points: vec![
                Point {
                    position: [0.4, 0.0, 0.0],
                    radius: 0.008,
                    kind: MarkerKind::Portal,
                },
                Point {
                    position: [0.1, 0.0, 0.0],
                    radius: 0.008,
                    kind: MarkerKind::Population,
                },
                Point {
                    position: [-0.4, 0.0, 0.0],
                    radius: 0.008,
                    kind: MarkerKind::Portal,
                },
            ],
            child_constraints: Vec::new(),
        };
        assert_eq!(nearest_portal([0.0; 3], &open), Some(0));
        assert_eq!(nearest_portal([-0.3, 0.0, 0.0], &open), Some(2));
        let empty = Generated {
            points: Vec::new(),
            child_constraints: Vec::new(),
        };
        assert_eq!(nearest_portal([0.0; 3], &empty), None);
    }

    #[test]
    fn free_speed_rungs_double_and_clamp() {
        assert_eq!(free_speed_multiplier(5), 1.0);
        for step in 1..9 {
            assert_eq!(
                free_speed_multiplier(step + 1),
                2.0 * free_speed_multiplier(step)
            );
        }
        assert_eq!(free_speed_multiplier(0), free_speed_multiplier(1));
        assert_eq!(free_speed_multiplier(99), free_speed_multiplier(9));
    }

    #[test]
    fn free_look_faces_minus_z_and_clamps_pitch() {
        let forward = free_look_direction(0.0, 0.0);
        assert!((forward[0]).abs() < 1e-15);
        assert!((forward[1]).abs() < 1e-15);
        assert_eq!(forward[2], -1.0);
        let clamped = free_look_direction(0.0, 10.0);
        let limited = free_look_direction(0.0, FREE_PITCH_LIMIT);
        assert_eq!(clamped, limited);
    }

    #[test]
    fn free_flight_step_moves_with_the_keys_deterministically() {
        use crate::r#gen::Point;
        let open = Generated {
            points: vec![Point {
                position: [0.0, 0.0, -0.4],
                radius: 0.01,
                kind: MarkerKind::Portal,
            }],
            child_constraints: Vec::new(),
        };
        let camera = [0.0, 0.0, 2.0];
        let keys = FreeKeys {
            forward: true,
            ..FreeKeys::default()
        };
        let moved = free_flight_step(
            FreePose {
                camera,
                yaw: 0.0,
                pitch: 0.0,
            },
            keys,
            5,
            &open,
            0.01,
            1.0 / 60.0,
        );
        assert!(moved[2] < camera[2], "forward must approach -Z");
        assert_eq!(moved[0], camera[0]);
        assert_eq!(moved[1], camera[1]);
        let again = free_flight_step(
            FreePose {
                camera,
                yaw: 0.0,
                pitch: 0.0,
            },
            keys,
            5,
            &open,
            0.01,
            1.0 / 60.0,
        );
        assert_eq!(moved, again, "same inputs, same step");
        assert_eq!(
            free_flight_step(
                FreePose {
                    camera,
                    yaw: 0.0,
                    pitch: 0.0,
                },
                FreeKeys::default(),
                5,
                &open,
                0.01,
                1.0 / 60.0
            ),
            camera,
            "no keys holds position"
        );
        assert_eq!(
            free_flight_step(
                FreePose {
                    camera,
                    yaw: 0.0,
                    pitch: 0.0,
                },
                keys,
                5,
                &open,
                0.01,
                0.0
            ),
            camera,
            "no time holds position"
        );
        assert_eq!(
            free_flight_step(
                FreePose {
                    camera,
                    yaw: 0.0,
                    pitch: 0.0,
                },
                keys,
                5,
                &open,
                0.01,
                f64::NAN
            ),
            camera,
            "bad time holds position"
        );
    }

    #[test]
    fn free_leg_is_fixed_seed_and_byte_identical() {
        let (leg, universe) = replay_free_leg(DEMO_SEED, 1.0 / 60.0, 600.0);
        let (again, _) = replay_free_leg(DEMO_SEED, 1.0 / 60.0, 600.0);
        assert_eq!(leg, again, "the leg must be identical every run");
        assert_eq!(leg.level.get(), 5, "the leg steers the L5 cell");
        assert_eq!(leg.points.len(), 300, "the scripted program is fixed");
        assert!(leg.alive_max <= STREAM_CAP, "working set over the cap");
        assert_eq!(universe.level().get(), 5, "steering never opens or closes");
        // The leg starts where the dive was: the same L5 snapshot.
        let (steps, _) = replay_autopilot(DEMO_SEED, 1.0 / 60.0, 600.0);
        let dived = steps
            .iter()
            .find(|step| step.level == leg.level)
            .expect("the dive opens L5");
        assert_eq!(leg.start_snapshot, dived.snapshot);
    }
}
