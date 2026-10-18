//! Where the player stands in the universe (#607).
//!
//! A [`PlayerSpot`](crate::player::PlayerSpot) is the named marker chain of
//! the cell the body stands in, its feet there, and its heading. The start
//! spot comes from the seed ([`PlayerSpot::seeded`](crate::player::PlayerSpot::seeded),
//! owner answer Q1 B): a planet and a spot on its ground (L10), the same
//! for the same seed. The dive's default final target is the player ([`PlayerSpot::destination`](crate::player::PlayerSpot::destination),
//! Q4 A), and its journey ends at
//! [`arrival_pose`](crate::player::PlayerSpot::arrival_pose), just behind
//! and above the body (Q3 A). Pure and deterministic (`E-DET-TIERS`), and
//! `Copy` like [`Destination`](crate::destination::Destination), so it rides in the settings without
//! allocating.

use crate::coords::Level;
use crate::destination::{DESTINATION_SLOTS, Destination};
use crate::nest::{MarkerPath, autopilot_candidates, generate_cell, path_level, path_seed};
use crate::seed::hash_triple;

/// Domain tag of the spawn picks, so they never follow the autopilot's.
const SPAWN_STREAM_TAG: u64 = 0x5A3E_1607_5A3E_1607;

/// Named chain length of the planet cell (L10): nine opens to the planet,
/// and the spot sits on its ground (Q2 A, re-anchored #729).
pub const SPAWN_CHAIN: usize = 9;

/// The spawn spot stays this far inside the region cell's middle.
pub const SPAWN_REACH: f64 = 0.25;

/// Third-person camera distance behind the eye, in body heights: the
/// arrival pose and the third-person view share this one definition
/// (#754, Q4 A).
pub const THIRD_BACK: f64 = 3.0;

/// Third-person camera rise above the eye, in body heights (#754, Q4 A).
pub const THIRD_UP: f64 = 0.6;

/// Third-person camera position for `eye` looking along `forward` with
/// `up`, in open-cell units: [`THIRD_BACK`] body heights behind the eye
/// and [`THIRD_UP`] above it. Pure (`E-DET-TIERS`).
#[must_use]
pub fn third_person_camera(
    eye: [f64; 3],
    forward: [f64; 3],
    up: [f64; 3],
    height: f64,
) -> [f64; 3] {
    [
        eye[0] - forward[0] * THIRD_BACK * height + up[0] * THIRD_UP * height,
        eye[1] - forward[1] * THIRD_BACK * height + up[1] * THIRD_UP * height,
        eye[2] - forward[2] * THIRD_BACK * height + up[2] * THIRD_UP * height,
    ]
}

/// Where the player's body stands (#607).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerSpot {
    markers: [u32; DESTINATION_SLOTS],
    len: u8,
    /// Feet position in the cell's units.
    pub position: [f64; 3],
    /// Heading around the cell's up, radians.
    pub yaw: f64,
}

impl PlayerSpot {
    /// The body standing at `position` with heading `yaw` in the cell at
    /// `chain`, or `None` for a chain longer than the slots.
    #[must_use]
    pub fn new(chain: &[u32], position: [f64; 3], yaw: f64) -> Option<PlayerSpot> {
        if chain.len() > DESTINATION_SLOTS {
            return None;
        }
        let mut markers = [0u32; DESTINATION_SLOTS];
        for (slot, &marker) in markers.iter_mut().zip(chain) {
            *slot = marker;
        }
        Some(PlayerSpot {
            markers,
            len: u8::try_from(chain.len()).ok()?,
            position,
            yaw,
        })
    }

    /// A body at the root cell's middle: the fallback when no branch of a
    /// universe reaches a region.
    #[must_use]
    pub fn at_root() -> PlayerSpot {
        PlayerSpot {
            markers: [0; DESTINATION_SLOTS],
            len: 0,
            position: [0.0; 3],
            yaw: 0.0,
        }
    }

    /// The named chain of the body's cell.
    #[must_use]
    pub fn chain(&self) -> &[u32] {
        self.markers
            .get(..usize::from(self.len))
            .unwrap_or(&self.markers)
    }

    /// The level of the body's cell.
    #[must_use]
    pub fn level(&self) -> Level {
        Level::new(self.len.saturating_add(1)).unwrap_or(Level::MAX)
    }

    /// The dive destination that leads to the player: its cell's chain,
    /// anchored so leaving the way rises back out instead of re-aiming.
    #[must_use]
    pub fn destination(&self) -> Destination {
        Destination::anchored(self.chain()).unwrap_or_else(|| Destination::seeded(0, &[]))
    }

    /// Whether the open cell of `path` is the body's cell (Q7 A: pressing
    /// 1 there takes the body over). Reads the chain in place.
    #[must_use]
    pub fn is_near(&self, path: &MarkerPath) -> bool {
        let mut named = path.chain().iter().filter(|opened| !opened.anonymous);
        let same = self
            .chain()
            .iter()
            .all(|marker| named.next().is_some_and(|opened| opened.marker == *marker));
        same && named.next().is_none()
    }

    /// Camera point and the point it looks at when the dive arrives at the
    /// player (Q3 A): [`THIRD_BACK`] body heights behind the eye and
    /// [`THIRD_UP`] above it, looking at the eye. Cell units, `+Y` up.
    /// Shares [`third_person_camera`] with the third-person view (#754).
    #[must_use]
    pub fn arrival_pose(&self) -> ([f64; 3], [f64; 3]) {
        let height = crate::body::body_height(self.level());
        let up = [0.0, 1.0, 0.0];
        let (ahead, _) = crate::body::heading_basis(up, self.yaw);
        let eye = [
            self.position[0],
            self.position[1] + crate::body::eye_height(self.level()),
            self.position[2],
        ];
        (third_person_camera(eye, ahead, up, height), eye)
    }

    /// The start spot for `root` (Q1 B, Q2 A re-anchored #729): the same
    /// seed always gives the same planet and spot.
    ///
    /// From L1, each level picks a portal by hash among the autopilot's
    /// candidates, falling back to the next candidates when a branch never
    /// reaches the planet. The spot is a hashed point within
    /// [`SPAWN_REACH`] of the planet cell's middle, on its ground, with a
    /// hashed heading. `None` only if no branch reaches a region.
    #[must_use]
    pub fn seeded(root: u64) -> Option<PlayerSpot> {
        let mut chain = Vec::with_capacity(SPAWN_CHAIN);
        if !spawn_walk(root, &mut chain) {
            return None;
        }
        let frame = crate::surface::SurfaceFrame::for_path(root, &chain)?;
        let seed = path_seed(root, &chain);
        let unit = |index: u64| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: a 53-bit hash share as a unit float, intended"
            )]
            let share =
                (hash_triple(seed, SPAWN_STREAM_TAG, index) >> 11) as f64 / (1u64 << 53) as f64;
            share
        };
        let x = (unit(1) * 2.0 - 1.0) * SPAWN_REACH;
        let z = (unit(2) * 2.0 - 1.0) * SPAWN_REACH;
        let yaw = unit(3) * std::f64::consts::TAU;
        let y = frame.ground_height(x, z);
        PlayerSpot::new(&chain, [x, y, z], yaw)
    }
}

/// Extends `chain` to [`SPAWN_CHAIN`] markers by hashed picks, backing up
/// to the next candidate when a branch dead-ends. `true` on success.
fn spawn_walk(root: u64, chain: &mut Vec<u32>) -> bool {
    if chain.len() == SPAWN_CHAIN {
        return true;
    }
    let level = path_level(chain);
    let seed = path_seed(root, chain);
    let cell = generate_cell(root, chain);
    let candidates = autopilot_candidates(level, seed, &cell.points);
    if candidates.is_empty() {
        return false;
    }
    let count = u64::try_from(candidates.len()).unwrap_or(1).max(1);
    let start = usize::try_from(hash_triple(seed, SPAWN_STREAM_TAG, 0) % count).unwrap_or(0);
    for step in 0..candidates.len() {
        let Some(&marker) = candidates.get((start + step) % candidates.len()) else {
            continue;
        };
        chain.push(marker);
        if spawn_walk(root, chain) {
            return true;
        }
        chain.pop();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::{DEMO_SEED, MarkerIndex, Universe};

    #[test]
    fn the_same_seed_always_spawns_at_the_same_spot_on_the_planet_ground() {
        let first = PlayerSpot::seeded(DEMO_SEED).expect("a spot");
        let again = PlayerSpot::seeded(DEMO_SEED).expect("a spot");
        assert_eq!(first, again, "fixed per seed (Q1 B)");
        assert_eq!(first.chain().len(), SPAWN_CHAIN);
        assert_eq!(first.level().get(), 10, "the planet (Q2 A re-anchored)");
        let frame =
            crate::surface::SurfaceFrame::for_path(DEMO_SEED, first.chain()).expect("planet frame");
        let ground = frame.ground_height(first.position[0], first.position[2]);
        assert!((first.position[1] - ground).abs() < 1e-12, "on the ground");
        let other = PlayerSpot::seeded(DEMO_SEED ^ 0x55).expect("a spot");
        assert_ne!(first, other, "another seed, another spot");
    }

    #[test]
    fn the_destination_leads_to_the_body_and_is_anchored() {
        let spot = PlayerSpot::seeded(DEMO_SEED).expect("a spot");
        let destination = spot.destination();
        assert!(destination.is_anchored());
        assert_eq!(destination.markers(), spot.chain());
        let mut universe = Universe::new(DEMO_SEED);
        for &marker in spot.chain() {
            assert!(!spot.is_near(&universe.path));
            assert!(universe.open(MarkerIndex(marker)), "the way opens");
        }
        assert!(spot.is_near(&universe.path), "in the body's cell");
        assert_eq!(
            destination.on_the_way(&universe.path),
            crate::destination::Way::Arrived
        );
    }

    #[test]
    fn the_arrival_pose_is_behind_and_above_the_body() {
        let spot = PlayerSpot::new(&[1, 2], [0.1, 0.0, 0.2], 0.0).expect("spot");
        let (camera, look) = spot.arrival_pose();
        let height = crate::body::body_height(spot.level());
        let (ahead, _) = crate::body::heading_basis([0.0, 1.0, 0.0], 0.0);
        let back = [camera[0] - look[0], camera[2] - look[2]];
        let along = back[0] * ahead[0] + back[1] * ahead[2];
        assert!((along + THIRD_BACK * height).abs() < 1e-12, "behind");
        assert!(
            (camera[1] - look[1] - THIRD_UP * height).abs() < 1e-12,
            "above"
        );
        assert!(look[1] > spot.position[1], "looks at the eye");
    }

    #[test]
    fn third_person_offset_is_defined_once() {
        // #754 AC1: one definition for the third-person distance and rise.
        assert_eq!(THIRD_BACK, 3.0, "three body heights behind");
        assert_eq!(THIRD_UP, 0.6, "0.6 above the eye");
        // The arrival pose is that helper with a level look.
        for yaw in [0.0, 0.7, -2.0] {
            let spot = PlayerSpot::new(&[1, 2], [0.1, 0.0, 0.2], yaw).expect("spot");
            let (camera, eye) = spot.arrival_pose();
            let height = crate::body::body_height(spot.level());
            let (ahead, _) = crate::body::heading_basis([0.0, 1.0, 0.0], yaw);
            assert_eq!(
                camera,
                third_person_camera(eye, ahead, [0.0, 1.0, 0.0], height),
                "arrival shares the helper (yaw {yaw})"
            );
        }
    }
}
