use super::*;
use crate::home::autopilot_path;
use crate::names::home_name;
use crate::nav::DEMO_SEED;

fn home_chain() -> Vec<u32> {
    autopilot_path(DEMO_SEED).get(..2).unwrap_or(&[]).to_vec()
}

#[test]
fn the_home_cell_is_the_local_group_with_the_milky_way_and_andromeda() {
    // #996 Goal 2: the home L3 cell holds about 150 galaxies, the Milky Way
    // is the journey's pick (marker 7, as before #996), Andromeda sits
    // 0.78 Mpc from it.
    let chain = home_chain();
    assert_eq!(members(DEMO_SEED, &chain), HOME_MEMBERS);
    let cell = generate_cell(DEMO_SEED, &chain);
    assert_eq!(cell.points.len(), HOME_MEMBERS as usize);
    let journey = autopilot_path(DEMO_SEED);
    assert_eq!(journey.get(2), Some(&7), "the journey keeps its L3 pick");
    assert_eq!(home_name(DEMO_SEED, &chain, 7), Some("Milky Way"));
    let andromeda = andromeda_marker(&cell.points, 7).expect("Andromeda");
    assert_eq!(home_name(DEMO_SEED, &chain, andromeda), Some("Andromeda"));
    let (mw, m31) = (
        cell.points[7].position,
        cell.points[andromeda as usize].position,
    );
    let gap =
        ((mw[0] - m31[0]).powi(2) + (mw[1] - m31[1]).powi(2) + (mw[2] - m31[2]).powi(2)).sqrt();
    assert!(
        (gap - ANDROMEDA_DISTANCE_CELLS).abs() < 1e-9,
        "{gap} against {ANDROMEDA_DISTANCE_CELLS}"
    );
}

#[test]
fn a_rich_cluster_holds_two_thousand_galaxies() {
    // #996 Goal 2: the cell entered through its parent's densest portal is
    // the rich one (Virgo), with 160 of its galaxies to enter.
    let parent = autopilot_path(DEMO_SEED).get(..1).unwrap_or(&[]).to_vec();
    let cell = generate_cell(DEMO_SEED, &parent);
    let seed = path_seed(DEMO_SEED, &parent);
    let anchor = parent.last().copied().map_or(0, i64::from);
    let densest = densest_portal_index(seed, anchor, &cell.points).expect("a portal");
    let mut rich = parent.clone();
    rich.push(densest);
    if rich == home_chain() {
        return;
    }
    assert_eq!(members(DEMO_SEED, &rich), RICH_CLUSTER_TOTAL);
    let galaxies = generate_cell(DEMO_SEED, &rich);
    assert_eq!(galaxies.points.len(), RICH_CLUSTER_TOTAL as usize);
    let open = galaxies
        .points
        .iter()
        .filter(|p| p.kind == MarkerKind::Portal)
        .count();
    assert_eq!(open, RICH_CLUSTER_PORTALS as usize);
}

#[test]
fn poorer_clusters_hold_fewer_galaxies() {
    let parent = autopilot_path(DEMO_SEED).get(..1).unwrap_or(&[]).to_vec();
    let cell = generate_cell(DEMO_SEED, &parent);
    for (index, point) in cell.points.iter().enumerate() {
        if point.kind != MarkerKind::Portal {
            continue;
        }
        let mut chain = parent.clone();
        chain.push(u32::try_from(index).unwrap_or(0));
        let n = members(DEMO_SEED, &chain);
        assert!(
            (MIN_MEMBERS..=RICH_CLUSTER_TOTAL).contains(&n),
            "{chain:?} {n}"
        );
    }
}

#[test]
fn a_typical_galaxy_is_an_l4_cell_across() {
    let level = Level::new(GALAXY_LEVEL).unwrap_or(Level::MIN);
    let expected = marker_radius(level).unwrap_or(0.0);
    assert!((radius(TYPICAL_SAMPLED_SIZE) - expected).abs() < 1e-15);
    assert_eq!(portals(2000), 160);
    assert_eq!(portals(100), 8);
    assert_eq!(portals(1), 1);
}
