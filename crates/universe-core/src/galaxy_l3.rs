//! L3 holds galaxies at their real numbers (#996, Spec v5 Goal 2).
//!
//! An L3 cell is the size of the Virgo cluster (ladder R5), so its markers
//! are galaxies, each the size of an L4 cell. How many: a rich cluster
//! like Virgo holds about 2,000 (study source R5), entered through its
//! parent's densest portal (the #151 prominence rule, now at L3); other
//! clusters hold fewer in proportion to the density where they sit; the
//! home cell is the Local Group and its neighbours. The biggest about one
//! in twelve can be entered (Virgo: about 160 of 2,000, R6). Pure
//! (`E-CORE-NO-BEVY`, `E-DET-TIERS`).

use crate::coords::Level;
use crate::density::{densest_portal_index, density_at};
use crate::r#gen::{GalaxyKind, MarkerKind, Point, clamp_to_sphere};
use crate::home::chain_matches_journey;
use crate::layouts::galaxy_form_and_portal;
use crate::nest::{generate_cell, marker_radius, path_level, path_seed};

/// Level whose markers are galaxies.
pub const GALAXY_LEVEL: u8 = 3;

/// Galaxies in a rich cluster cell such as Virgo (study R5).
pub const RICH_CLUSTER_TOTAL: u32 = 2000;

/// Galaxies that open in a rich cluster cell (the 160 largest, R6).
pub const RICH_CLUSTER_PORTALS: u32 = 160;

/// Galaxies in the home cell: the Local Group's 134 known members within
/// 1 Mpc (Wikipedia, Local Group) and the near side of its neighbouring
/// groups inside a 4.6 Mpc cell.
pub const HOME_MEMBERS: u32 = 150;

/// Fewest galaxies a cluster cell holds.
pub const MIN_MEMBERS: u32 = 30;

/// Typical sampled galaxy size (`astro::sample_galaxy`, the bell's middle).
const TYPICAL_SAMPLED_SIZE: f64 = 0.05;

/// Distance from the Milky Way to Andromeda, in L3 cells (0.78 Mpc over a
/// 1.41e23 m cell).
pub const ANDROMEDA_DISTANCE_CELLS: f64 = 0.78 * 3.0857e22 / 1.413e23;

/// Galaxies in the L3 cell at `chain` (one marker long) under `root`.
#[must_use]
pub fn members(root: u64, chain: &[u32]) -> u32 {
    if path_level(chain).get() != GALAXY_LEVEL {
        return MIN_MEMBERS;
    }
    if chain_matches_journey(root, chain) {
        return HOME_MEMBERS;
    }
    let Some((&opened, parent)) = chain.split_last() else {
        return MIN_MEMBERS;
    };
    let cell = generate_cell(root, parent);
    let seed = path_seed(root, parent);
    let anchor = parent.last().copied().map_or(0, i64::from);
    let Some(densest) = densest_portal_index(seed, anchor, &cell.points) else {
        return MIN_MEMBERS;
    };
    if densest == opened {
        return RICH_CLUSTER_TOTAL;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: lattice anchor is a small integer, exactly representable"
    )]
    let origin = anchor as f64;
    let density_of = |marker: u32| {
        cell.points
            .get(marker as usize)
            .filter(|p| p.kind == MarkerKind::Portal)
            .map_or(0.0, |p| {
                density_at(seed, origin + p.position[0], p.position[1], p.position[2])
            })
    };
    let top = density_of(densest);
    if top <= 0.0 {
        return MIN_MEMBERS;
    }
    let share = (density_of(opened) / top).clamp(0.0, 1.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "E-CAST: share in [0, 1] times 2,000 fits u32"
    )]
    let scaled = (f64::from(RICH_CLUSTER_TOTAL) * share).round() as u32;
    scaled.clamp(MIN_MEMBERS, RICH_CLUSTER_TOTAL)
}

/// Galaxies that open among `total`: the largest one in twelve and a half
/// (Virgo's 160 of 2,000), at least two (the Milky Way and Andromeda).
#[must_use]
pub fn portals(total: usize) -> usize {
    let share = total * RICH_CLUSTER_PORTALS as usize / RICH_CLUSTER_TOTAL as usize;
    share.max(2).min(total)
}

/// Drawn radius of a galaxy of sampled `size` in an L3 cell: a typical
/// galaxy is the size of an L4 cell, the Milky Way's.
#[must_use]
pub fn radius(size: f64) -> f64 {
    let level = Level::new(GALAXY_LEVEL).unwrap_or(Level::MIN);
    marker_radius(level).unwrap_or(0.0) * size / TYPICAL_SAMPLED_SIZE
}

/// Andromeda in the home cell: the largest portal other than the Milky
/// Way (`milky_way`).
#[must_use]
pub fn andromeda_marker(points: &[Point], milky_way: u32) -> Option<u32> {
    points
        .iter()
        .enumerate()
        .filter(|(_, p)| p.kind == MarkerKind::Portal)
        .filter_map(|(i, _)| u32::try_from(i).ok())
        .find(|&m| m != milky_way)
}

/// Places Andromeda in the home cell: a large spiral 0.78 Mpc from the
/// Milky Way, toward the cell centre. Only its position and shape change,
/// so ranks, counts and the journey hold.
pub fn place_andromeda(points: &mut [Point], milky_way: u32) {
    let Some(from) = points.get(milky_way as usize).map(|p| p.position) else {
        return;
    };
    let Some(marker) = andromeda_marker(points, milky_way) else {
        return;
    };
    let Some(point) = points.get_mut(marker as usize) else {
        return;
    };
    let length = (from[0] * from[0] + from[1] * from[1] + from[2] * from[2]).sqrt();
    let toward = if length > 1e-9 {
        [-from[0] / length, -from[1] / length, -from[2] / length]
    } else {
        [1.0, 0.0, 0.0]
    };
    point.position = clamp_to_sphere([
        from[0] + toward[0] * ANDROMEDA_DISTANCE_CELLS,
        from[1] + toward[1] * ANDROMEDA_DISTANCE_CELLS,
        from[2] + toward[2] * ANDROMEDA_DISTANCE_CELLS,
    ]);
    if let Some(galaxy) = point.galaxy.as_mut() {
        galaxy.kind = GalaxyKind::Spiral;
        galaxy.barred = false;
        galaxy.far_view = false;
        let (form, portal) = galaxy_form_and_portal(*galaxy, point.radius, false);
        point.form = form;
        point.portal = portal;
    }
}

#[cfg(test)]
#[path = "galaxy_l3_tests.rs"]
mod tests;
