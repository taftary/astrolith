//! Journey matching, home fixtures, and the autopilot (#151, #154-#156).
//!
//! Split from [`crate::nest`] when that file crossed the drift-guard fail
//! threshold (#156 T4): the dive path answers "is this cell the home one"
//! here, generation keeps living in `nest`. Public autopilot picks stay
//! re-exported through `nest`, so the core API surface does not move.

use crate::coords::Level;
use crate::r#gen::{Generated, MarkerKind, Point};
use crate::nest::{MAX_OPEN_LEVEL, generate_cell, path_level, path_seed};
use crate::seed::hash_cell;

/// Whether `chain` follows the autopilot journey marker by marker.
///
/// Every marker must equal the autopilot pick in its prefix cell. Prefix
/// cells regenerate from strictly shorter chains, so the walk always
/// terminates. Every other chain is a procedural neighbour by definition.
#[must_use]
pub(crate) fn chain_matches_journey(root: u64, chain: &[u32]) -> bool {
    let mut prefix: Vec<u32> = Vec::with_capacity(chain.len());
    for &marker in chain {
        let cell = generate_cell(root, &prefix);
        let level = path_level(&prefix);
        let seed = path_seed(root, &prefix);
        let candidates = cell_candidates(level, seed, &cell);
        if autopilot_marker(seed, &candidates) != Some(marker) {
            return false;
        }
        prefix.push(marker);
    }
    true
}

/// Whether the L3 cell at `chain` is the home cell: the Local Group with
/// the Milky Way and Andromeda (#154 Q5, moved to L3 by #996).
///
/// True exactly when `chain` (two markers: L1, L2) matches the fixed
/// journey prefix under `root`: every marker equals the autopilot pick in
/// its cell. Prefix cells regenerate from strictly shorter chains, so the
/// walk always terminates. Every other chain is a procedural neighbour by
/// definition.
#[must_use]
pub(crate) fn is_home_l3_cell(root: u64, chain: &[u32]) -> bool {
    if path_level(chain).get() != 3 || chain.len() != 2 {
        return false;
    }
    chain_matches_journey(root, chain)
}

/// Whether the L6-L9 cell at `chain` sits on the journey path (#155 Q4).
///
/// True exactly when the cell level is 6, 7, 8, or 9 and the whole chain
/// matches the autopilot picks, so the Alpha Centauri triple (L6), the Sun
/// (L7), and the Solar catalog planets (L8/L9) land on the home path and
/// nowhere else. Same termination shape as [`is_home_l3_cell`].
#[must_use]
pub(crate) fn is_home_system_cell(root: u64, chain: &[u32]) -> bool {
    if !matches!(path_level(chain).get(), 6..=9) {
        return false;
    }
    chain_matches_journey(root, chain)
}

/// Whether the L10 cell at `chain` sits on the journey path (#156 Q5).
///
/// True exactly when the cell level is 10 and the whole chain matches the
/// autopilot picks, so the Earth-Moon home cell lands on the home path
/// and nowhere else. Same termination shape as `is_home_system_cell`.
/// Public for the planet frame (#774): the frame radius comes from the cell
/// recipe, which needs the home fixture.
#[must_use]
pub fn is_home_planet_cell(root: u64, chain: &[u32]) -> bool {
    if path_level(chain).get() != 10 {
        return false;
    }
    chain_matches_journey(root, chain)
}

/// Deterministic autopilot marker for a cell: a seeded pick among `portals`.
///
/// Drawn from the cell seed, so the Spacebar journey is the same every run
/// (owner decision: fixed journey). Populations never open, so only portal
/// indices are candidates. Returns `None` for a cell with no portals.
#[must_use]
pub fn autopilot_marker(cell_seed: u64, portals: &[u32]) -> Option<u32> {
    if portals.is_empty() {
        return None;
    }
    let roll = hash_cell(cell_seed, 0, 0x4155_544f, 0, 0);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: remainder below portal count, a budgeted cell size"
    )]
    let pick = (roll % portals.len() as u64) as u32;
    // `pick` is below the portal count by construction; `u32 as usize`
    // widens on every supported target, so no truncation lint applies.
    portals.get(pick as usize).copied()
}

/// The journey's marker in its home cells, by cell level, as recorded
/// before #996 for the fixed root: the L3 cell opens marker 7, L4 opens 1,
/// L5 opens 2 and L6 opens 0. #996 changes what those cells hold; pinning
/// the picks keeps every seed below them, so the Sun, Earth and the ground
/// stay exactly as they were (Spec v5 Goal 6).
#[must_use]
pub const fn home_pick(level: Level) -> Option<u32> {
    match level.get() {
        3 => Some(7),
        4 => Some(1),
        5 => Some(2),
        6 => Some(0),
        _ => None,
    }
}

/// Makes the autopilot pick `want` in a home cell holding `points`,
/// through which markers open, not through the pick rule.
///
/// The autopilot picks the `roll % k`-th of the cell's `k` portals. This
/// takes the `k` closest to `at_least` whose position `p = roll % k` fits,
/// then opens `p` markers before `want`, `want`, and `k - p - 1` after it,
/// the biggest first (rank order); every other marker is a population.
/// Ranks and positions do not move. Leaves the cell unchanged when `want`
/// is out of range.
pub fn pin_journey_pick(cell_seed: u64, points: &mut [Point], want: u32, at_least: usize) {
    let rank: Vec<usize> = (0..points.len()).collect();
    pin_journey_pick_ranked(cell_seed, points, want, at_least, &rank);
}

/// [`pin_journey_pick`] with the other portals chosen in `rank` order
/// (marker indices, most wanted first) instead of index order.
pub fn pin_journey_pick_ranked(
    cell_seed: u64,
    points: &mut [Point],
    want: u32,
    at_least: usize,
    rank: &[usize],
) {
    let want = want as usize;
    let total = points.len();
    if want >= total {
        return;
    }
    let roll = hash_cell(cell_seed, 0, 0x4155_544f, 0, 0);
    let fits = |k: usize| {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: remainder below k, a budgeted cell size"
        )]
        let p = (roll % k as u64) as usize;
        (p <= want && k - p - 1 < total - want).then_some(p)
    };
    // The count closest to `at_least` that fits, looking both ways.
    let Some((k, p)) = (1..=total)
        .filter_map(|k| fits(k).map(|p| (k, p)))
        .min_by_key(|(k, _)| k.abs_diff(at_least.max(1)))
    else {
        return;
    };
    for point in points.iter_mut() {
        point.kind = MarkerKind::Population;
    }
    let before = rank.iter().filter(|&&i| i < want).take(p);
    let after = rank
        .iter()
        .filter(|&&i| i > want && i < total)
        .take(k - p - 1);
    for &index in before.chain(after).chain(std::iter::once(&want)) {
        if let Some(point) = points.get_mut(index) {
            point.kind = MarkerKind::Portal;
        }
    }
}

/// [`autopilot_candidates`] of a cell, as stable marker IDs (#1104 T2).
#[must_use]
pub fn cell_candidates(level: Level, seed: u64, cell: &Generated) -> Vec<u32> {
    autopilot_candidates(level, seed, &cell.points)
        .into_iter()
        .map(|index| cell.id_at(index as usize))
        // #1104: in a streamed cell the journey picks among the fixture
        // alone (L1 regions 0 to 7, the L5 ball), so it never depends on
        // which blocks are resident.
        .filter(|&id| match level.get() {
            1 => id < crate::regions_l1::FIXTURE,
            5 => id < crate::stars_l5::CORE_STARS,
            _ => true,
        })
        .collect()
}

/// Portal indices the autopilot may pick at `level` (#151 journey rules).
///
/// L8 picks the star (`portals[0]` by construction); every other level
/// picks among all portals (#996: L3 holds galaxies, so its old
/// group-tier rule is gone). Empty exactly when the cell holds no portals.
#[must_use]
pub fn autopilot_candidates(level: Level, _seed: u64, points: &[Point]) -> Vec<u32> {
    let mut portals: Vec<u32> = Vec::new();
    for (index, point) in points.iter().enumerate() {
        if point.kind == MarkerKind::Portal {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: marker index into a budgeted cell, always fits u32"
            )]
            let marker = index as u32;
            portals.push(marker);
        }
    }
    if level.get() == 8 {
        return portals.first().copied().into_iter().collect();
    }
    portals
}

/// Marker indices the autopilot opens from L1 down to [`MAX_OPEN_LEVEL`].
///
/// Pure in `root`, and walked once per root per thread (#996): with real
/// counts the walk generates thousands of markers, and names, labels and
/// the journey checks ask for it every frame.
#[must_use]
pub fn autopilot_path(root: u64) -> Vec<u32> {
    let known = JOURNEYS.with(|journeys| {
        journeys
            .borrow()
            .iter()
            .find(|(r, _)| *r == root)
            .map(|(_, path)| path.clone())
    });
    if let Some(path) = known {
        return path;
    }
    let path = walk_autopilot_path(root);
    JOURNEYS.with(|journeys| {
        let mut journeys = journeys.borrow_mut();
        if journeys.len() >= JOURNEY_CACHE_CAP {
            journeys.remove(0);
        }
        journeys.push((root, path.clone()));
    });
    path
}

/// Journeys kept per thread by [`autopilot_path`] (#996).
const JOURNEY_CACHE_CAP: usize = 4;

thread_local! {
    static JOURNEYS: std::cell::RefCell<Vec<(u64, Vec<u32>)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

fn walk_autopilot_path(root: u64) -> Vec<u32> {
    let mut chain = Vec::with_capacity(usize::from(MAX_OPEN_LEVEL - 1));
    while path_level(&chain).get() < MAX_OPEN_LEVEL {
        let cell = generate_cell(root, &chain);
        let level = path_level(&chain);
        let seed = path_seed(root, &chain);
        let candidates = cell_candidates(level, seed, &cell);
        let Some(marker) = autopilot_marker(seed, &candidates) else {
            break;
        };
        chain.push(marker);
    }
    chain
}

/// Returns the portal position of `marker` in `cell`, if it exists.
///
/// The portal is the child cell: dives aim here, openings store this, and
/// previews map children from it. The body's drawn position stays on the
/// point itself (#384).
#[must_use]
pub fn marker_position(cell: &Generated, marker: u32) -> Option<[f64; 3]> {
    cell.point_by_id(marker)
        .map(|point| point.portal_position())
}
