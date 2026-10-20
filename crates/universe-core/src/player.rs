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

/// The start spot's angle from the sub-solar point in radians (#780 F1 A):
/// the morning side, with the sun about 40 degrees from overhead.
pub const MORNING_RAD: f64 = 40.0 * std::f64::consts::PI / 180.0;

/// The hashed spread of the start spot around the morning point, per axis,
/// in radians: at most 12 degrees, so the sun stays over 30 degrees up.
pub const MORNING_SPREAD_RAD: f64 = 12.0 * std::f64::consts::PI / 180.0;

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
    /// [`THIRD_UP`] above it, looking at the eye. Cell units; up is away
    /// from the planet centre on a planet (the L10 cell is planet-centred),
    /// `+Y` elsewhere. Shares [`third_person_camera`] with the third-person
    /// view (#754).
    #[must_use]
    pub fn arrival_pose(&self) -> ([f64; 3], [f64; 3]) {
        let height = crate::body::body_height(self.level());
        let up = if usize::from(self.len) == SPAWN_CHAIN {
            unit_or_y(self.position)
        } else {
            [0.0, 1.0, 0.0]
        };
        let (ahead, _) = crate::body::heading_basis(up, self.yaw);
        let lift = crate::body::eye_height(self.level());
        let eye = [
            self.position[0] + up[0] * lift,
            self.position[1] + up[1] * lift,
            self.position[2] + up[2] * lift,
        ];
        (third_person_camera(eye, ahead, up, height), eye)
    }

    /// The start spot for `root` (Q1 B, Q2 A re-anchored #729; #774 Q7 A: true
    /// surface; #799 Q5 A: terrain height plus 2 m; #770: no ground until
    /// step 6).
    ///
    /// From L1, each level picks a portal by hash among the autopilot's
    /// candidates, falling back to the next candidates when a branch never
    /// reaches the planet. The spot is on the morning side under the planet's
    /// fixed sun ([`MORNING_RAD`] from the sub-solar point, #780 F1 A),
    /// spread by a hashed [`MORNING_SPREAD_RAD`] per axis, at the relief surface
    /// (the planet's own radius plus [`crate::relief::Relief`] height) plus
    /// 2 m, through the one L10 metre, with a hashed heading. Your body still
    /// floats; walking is step 6 (#779). `None` only if no branch reaches a
    /// region.
    #[must_use]
    pub fn seeded(root: u64) -> Option<PlayerSpot> {
        let mut chain = Vec::with_capacity(SPAWN_CHAIN);
        if !spawn_walk(root, &mut chain) {
            return None;
        }
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
        let a = (unit(1) * 2.0 - 1.0) * MORNING_SPREAD_RAD;
        let b = (unit(2) * 2.0 - 1.0) * MORNING_SPREAD_RAD;
        let yaw = unit(3) * std::f64::consts::TAU;
        // Relief surface (#799 Q5 A): the planet's own radius plus the relief
        // height at the spot direction, plus 2 m, so the spot is never inside
        // a mountain. Direction from the sphere point, then out along it.
        let home = crate::home::is_home_planet_cell(root, &chain);
        let recipe = crate::terrain::body_recipe(seed, home);
        let r = crate::frame::planet_radius_cells_for(recipe.radius_earth);
        let mpc = crate::planet_frame::metres_per_cell_l10();
        let relief = crate::relief::Relief::for_planet(root, &chain);
        // The morning point, tipped by the hashed spread along two tangents
        // (the L10 cell is planet-centred, so a cell direction is a planet
        // direction).
        let morning = morning_point(root, &chain);
        let (e1, e2) = tangents(morning);
        let (ta, tb) = (a.tan(), b.tan());
        let dir0 = unit_or_y([
            morning[0] + e1[0] * ta + e2[0] * tb,
            morning[1] + e1[1] * ta + e2[1] * tb,
            morning[2] + e1[2] * ta + e2[2] * tb,
        ]);
        let h0 = relief.height_m(dir0);
        let h0 = if h0.is_finite() { h0 } else { 0.0 };
        let delta_cells = (h0 + 2.0) / mpc;
        let r_terrain = r + delta_cells;
        // Out along the direction, so the spot direction stays dir0 and its
        // height above the relief is exactly 2 m.
        let pos = [
            dir0[0] * r_terrain,
            dir0[1] * r_terrain,
            dir0[2] * r_terrain,
        ];
        PlayerSpot::new(&chain, pos, yaw)
    }
}

/// Unit `v`, or `+Y` when it has no direction.
fn unit_or_y(v: [f64; 3]) -> [f64; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len.is_finite() && len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// Two unit tangents at unit `n`, perpendicular to it and to each other.
fn tangents(n: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let helper = if n[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let e1 = unit_or_y([
        helper[1] * n[2] - helper[2] * n[1],
        helper[2] * n[0] - helper[0] * n[2],
        helper[0] * n[1] - helper[1] * n[0],
    ]);
    let e2 = [
        n[1] * e1[2] - n[2] * e1[1],
        n[2] * e1[0] - n[0] * e1[2],
        n[0] * e1[1] - n[1] * e1[0],
    ];
    (e1, e2)
}

/// The morning point of the planet reached by `chain` (#780 F1 A): where
/// its fixed sun stands [`MORNING_RAD`] from overhead
/// ([`crate::light::from_sun`]), or `+Y` when the planet has no sun.
#[must_use]
pub fn morning_point(root: u64, chain: &[u32]) -> [f64; 3] {
    let mut universe = crate::nav::Universe::new(root);
    for &marker in chain {
        if !universe.open(crate::nav::MarkerIndex(marker)) {
            return [0.0, 1.0, 0.0];
        }
    }
    crate::light::sun_direction(&universe).map_or([0.0, 1.0, 0.0], |sun| {
        crate::light::from_sun(sun, MORNING_RAD)
    })
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

    /// The sun's height above the horizon in degrees at unit-free `at`.
    fn sun_up_deg(sun: [f64; 3], at: [f64; 3]) -> f64 {
        let len = (at[0] * at[0] + at[1] * at[1] + at[2] * at[2]).sqrt();
        let cos = (sun[0] * at[0] + sun[1] * at[1] + sun[2] * at[2]) / len;
        90.0 - cos.clamp(-1.0, 1.0).acos().to_degrees()
    }

    #[test]
    fn arrival_is_in_daylight() {
        // C11 (#780 F1 A): the dive's arrival point and the seeded start spot
        // are on the morning side, the sun at least 30 degrees up; the night
        // side is still on the same planet, reachable by moving round it.
        for root in [DEMO_SEED, DEMO_SEED ^ 0x55, DEMO_SEED ^ 0xA5A5, 7, 42] {
            let Some(spot) = PlayerSpot::seeded(root) else {
                continue;
            };
            let mut universe = Universe::new(root);
            for &marker in spot.chain() {
                assert!(universe.open(MarkerIndex(marker)), "chain opens");
            }
            let sun = crate::light::sun_direction(&universe).expect("a sun");
            let arrival = universe.path.offset();
            let at_arrival = sun_up_deg(sun, arrival);
            assert!(at_arrival >= 30.0, "root {root}: arrival sun {at_arrival}");
            assert!((at_arrival - 50.0).abs() < 1e-6, "40 degrees from overhead");
            let at_spot = sun_up_deg(sun, spot.position);
            assert!(at_spot >= 30.0, "root {root}: spot sun {at_spot}");
            let night = crate::light::from_sun(sun, 150f64.to_radians());
            assert!(sun_up_deg(sun, night) < 0.0, "the night side is there");
        }
        // The dive's own journey arrives there too.
        let mut universe = Universe::new(DEMO_SEED);
        for _ in 0..9 {
            let marker = universe.autopilot_target().expect("dive continues");
            assert!(universe.open(MarkerIndex(marker)));
        }
        let sun = crate::light::sun_direction(&universe).expect("a sun");
        assert!(sun_up_deg(sun, universe.path.offset()) >= 30.0);
    }

    #[test]
    fn the_same_seed_always_spawns_at_the_same_spot_on_the_planet_ground() {
        // #774 Q7 A + #799 Q5 A: the spot sits at the relief surface plus 2 m
        // from its own record; #770: no ground until step 6.
        let first = PlayerSpot::seeded(DEMO_SEED).expect("a spot");
        let again = PlayerSpot::seeded(DEMO_SEED).expect("a spot");
        assert_eq!(first, again, "fixed per seed (Q1 B)");
        assert_eq!(first.chain().len(), SPAWN_CHAIN);
        assert_eq!(first.level().get(), 10, "the planet (Q2 A re-anchored)");
        let seed = path_seed(DEMO_SEED, first.chain());
        let home = crate::home::is_home_planet_cell(DEMO_SEED, first.chain());
        let recipe = crate::terrain::body_recipe(seed, home);
        let r = crate::frame::planet_radius_cells_for(recipe.radius_earth);
        let mpc = crate::planet_frame::metres_per_cell_l10();
        let relief = crate::relief::Relief::for_planet(DEMO_SEED, first.chain());
        let [x, y, z] = first.position;
        let dist = (x * x + y * y + z * z).sqrt();
        assert!(dist.is_finite() && dist > 0.0);
        let dir = [x / dist, y / dist, z / dist];
        let h = relief.height_m(dir);
        // Spot distance equals the relief radius plus 2 m, within 1 µm.
        let expect = r + (h + 2.0) / mpc;
        assert!(
            ((dist - expect).abs() * mpc) < 1e-6,
            "spot distance {dist} cells equals relief radius {expect} within 1 µm (h {h})"
        );
        let other = PlayerSpot::seeded(DEMO_SEED ^ 0x55).expect("a spot");
        assert_ne!(first, other, "another seed, another spot");
    }

    #[test]
    fn spot_floats_above_the_relief() {
        // AC7 / C7 (T4, Q5 A): the spot is at the terrain height plus 2 m,
        // never inside the ground, over several seeds.
        let mpc = crate::planet_frame::metres_per_cell_l10();
        for root in [DEMO_SEED, 1u64, 42, 999, 12345] {
            let Some(spot) = PlayerSpot::seeded(root) else {
                continue;
            };
            let chain = spot.chain().to_vec();
            let seed = path_seed(root, &chain);
            let home = crate::home::is_home_planet_cell(root, &chain);
            let recipe = crate::terrain::body_recipe(seed, home);
            let r = crate::frame::planet_radius_cells_for(recipe.radius_earth);
            let relief = crate::relief::Relief::for_planet(root, &chain);
            let [x, y, z] = spot.position;
            let dist = (x * x + y * y + z * z).sqrt();
            assert!(dist.is_finite() && dist > 0.0, "finite spot distance");
            let dir = [x / dist, y / dist, z / dist];
            let h = relief.height_m(dir);
            assert!(h.is_finite(), "finite terrain height");
            // 2 m above the relief, within a millimetre.
            let above_m = (dist - r) * mpc - h;
            assert!(
                (above_m - 2.0).abs() < 1e-3,
                "root {root}: spot floats {above_m} m above the relief, want 2 m"
            );
            assert!(
                above_m > 0.0,
                "root {root}: spot never inside the ground ({above_m} m)"
            );
        }
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
