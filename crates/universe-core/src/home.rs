//! Journey matching, home fixtures, and the autopilot (#151, #154-#156).
//!
//! Split from [`crate::nest`] when that file crossed the drift-guard fail
//! threshold (#156 T4): the dive path answers "is this cell the home one"
//! here, generation keeps living in `nest`. Public autopilot picks stay
//! re-exported through `nest`, so the core API surface does not move.

use crate::coords::Level;
use crate::density::densest_portal_index;
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
        let candidates = autopilot_candidates(level, seed, &cell.points);
        if autopilot_marker(seed, &candidates) != Some(marker) {
            return false;
        }
        prefix.push(marker);
    }
    true
}

/// Whether the L4 cell at `chain` is the Milky Way home cell (#154 Q5).
///
/// True exactly when `chain` (three markers: L1, L2, L3) matches the fixed
/// journey prefix under `root`: every marker equals the autopilot pick in
/// its cell. Prefix cells regenerate from strictly shorter chains, so the
/// walk always terminates (the same shape as `is_rich_cluster_cell`).
/// Every other chain is a procedural neighbour by definition.
#[must_use]
pub(crate) fn is_home_l4_cell(root: u64, chain: &[u32]) -> bool {
    if path_level(chain).get() != 4 || chain.len() != 3 {
        return false;
    }
    chain_matches_journey(root, chain)
}

/// Whether the L6-L9 cell at `chain` sits on the journey path (#155 Q4).
///
/// True exactly when the cell level is 6, 7, 8, or 9 and the whole chain
/// matches the autopilot picks, so the Alpha Centauri triple (L6), the Sun
/// (L7), and the Solar catalog planets (L8/L9) land on the home path and
/// nowhere else. Same termination shape as [`is_home_l4_cell`].
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
/// and nowhere else. Same termination shape as [`is_home_system_cell`].
#[must_use]
pub(crate) fn is_home_planet_cell(root: u64, chain: &[u32]) -> bool {
    if path_level(chain).get() != 10 {
        return false;
    }
    chain_matches_journey(root, chain)
}

/// Whether the L4 cell at `chain` shows rich-cluster content (#151).
///
/// True exactly when the cell was entered through its parent's densest
/// cluster portal (the prominence rule: the anchor object of a rich family
/// is always rich, so Virgo is rich on every visit while other clusters
/// draw poor). Only L4 chains consult the parent; every other level is
/// poor by definition, so no parent is regenerated for them.
#[must_use]
pub(crate) fn is_rich_cluster_cell(root: u64, chain: &[u32]) -> bool {
    if path_level(chain).get() != 4 {
        return false;
    }
    let Some((&opened, parent)) = chain.split_last() else {
        return false;
    };
    let parent_cell = generate_cell(root, parent);
    let anchor = parent.last().copied().map_or(0, i64::from);
    densest_portal_index(path_seed(root, parent), anchor, &parent_cell.points) == Some(opened)
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

/// Portal indices the autopilot may pick at `level` (#151 journey rules).
///
/// L3 picks among group-tier portals only (the home path runs through the
/// Local Group with Virgo as the rich sibling; groups always exist by the
/// `1 +` term, with an all-portals fallback that never triggers); L8 picks
/// the star (`portals[0]` by construction); every other level picks among
/// all portals. Empty exactly when the cell holds no portals.
#[must_use]
pub fn autopilot_candidates(level: Level, seed: u64, points: &[Point]) -> Vec<u32> {
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
    if level.get() == 3 {
        use crate::density::portal_tiers;
        let (clusters, _) = portal_tiers(level, seed, points.len());
        let groups: Vec<u32> = portals
            .iter()
            .copied()
            .filter(|marker| (*marker as usize) >= clusters)
            .collect();
        if !groups.is_empty() {
            return groups;
        }
    }
    portals
}

/// Marker indices the autopilot opens from L1 down to [`MAX_OPEN_LEVEL`].
#[must_use]
pub fn autopilot_path(root: u64) -> Vec<u32> {
    let mut chain = Vec::with_capacity(usize::from(MAX_OPEN_LEVEL - 1));
    while path_level(&chain).get() < MAX_OPEN_LEVEL {
        let cell = generate_cell(root, &chain);
        let level = path_level(&chain);
        let seed = path_seed(root, &chain);
        let candidates = autopilot_candidates(level, seed, &cell.points);
        let Some(marker) = autopilot_marker(seed, &candidates) else {
            break;
        };
        chain.push(marker);
    }
    chain
}

/// Returns the position of `marker` in `cell`, if it exists.
#[must_use]
pub fn marker_position(cell: &Generated, marker: u32) -> Option<[f64; 3]> {
    cell.points.get(marker as usize).map(|point| point.position)
}
