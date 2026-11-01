use super::*;
use crate::nav::DEMO_SEED;
use crate::nest::{generate_cell, path_seed};

fn seed() -> u64 {
    path_seed(DEMO_SEED, &[])
}

#[test]
fn the_fixture_keeps_the_eight_old_region_numbers() {
    // IDs 0 to 7 sit in eight distinct lattice cells, one per old octant.
    let mut cells: Vec<[u32; 3]> = (0..FIXTURE).map(fixture_lattice).collect();
    for (id, cell) in cells.iter().enumerate() {
        let want = |bit: usize| if (id >> bit) & 1 == 0 { -0.25 } else { 0.25 };
        let centre = |c: u32| (f64::from(c) + 0.5) * pitch() - 0.5;
        assert!((centre(cell[0]) - want(2)).abs() <= pitch(), "{id}");
        assert!((centre(cell[1]) - want(1)).abs() <= pitch(), "{id}");
        assert!((centre(cell[2]) - want(0)).abs() <= pitch(), "{id}");
    }
    cells.sort_unstable();
    cells.dedup();
    assert_eq!(cells.len(), 8);
    let cell = generate_cell(DEMO_SEED, &[]);
    assert_eq!(cell.ids, (0..8).collect::<Vec<u32>>());
    assert!(cell.points.iter().all(|p| p.kind == MarkerKind::Portal));
    assert_eq!(cell.child_constraints.len(), 8, "octant budgets kept");
}

#[test]
fn the_whole_cell_holds_every_lattice_region_once() {
    // Spec v3 AC1 (D1 A): the lattice cells inside the universe sphere.
    let all = all(seed());
    assert_eq!(all.len(), 164_968);
    assert!(
        all.iter()
            .all(|p| p.position.iter().map(|c| c * c).sum::<f64>().sqrt()
                < 0.5 + 0.15 * 3f64.sqrt() * pitch())
    );
    assert!(all.iter().all(|p| p.kind == MarkerKind::Portal));
    assert!(all.iter().all(|p| p.position.iter().all(|c| c.abs() < 0.5)));
}

#[test]
fn a_region_is_rebuilt_from_its_id() {
    // Spec v2 AC5: the same ID gives the same region, resident or not.
    let s = seed();
    for key in [
        BlockKey([0, 0, 0]),
        BlockKey([16, 3, 9]),
        BlockKey([12, 4, 4]),
    ] {
        for (id, point) in crate::blocks::block(&Regions, &GRID, s, key) {
            assert!(id >= FIXTURE);
            assert_eq!(marker(s, id), Some(point));
        }
    }
    assert_eq!(
        marker(s, 4),
        generate_cell(DEMO_SEED, &[]).points.get(4).cloned()
    );
}

#[test]
fn a_fixture_cell_holds_no_second_region() {
    // The lattice cell of a fixture region yields nothing in its block.
    let s = seed();
    for id in 0..FIXTURE {
        let lattice = fixture_lattice(id);
        let key = BlockKey([
            lattice[0] / BLOCK_SIDE,
            lattice[1] / BLOCK_SIDE,
            lattice[2] / BLOCK_SIDE,
        ]);
        let held = crate::blocks::block(&Regions, &GRID, s, key);
        let inside = (0..GRID.slots)
            .filter(|&slot| in_universe(lattice_of(key, slot)))
            .count();
        assert_eq!(held.len(), inside - 1, "block of fixture {id}");
    }
}

#[test]
fn every_region_id_fits_and_decodes() {
    let last = GRID
        .encode(BlockKey([16, 16, 16]), GRID.slots - 1)
        .unwrap_or(0);
    assert_eq!(last, FIXTURE + 314_432 - 1);
    assert_eq!(
        GRID.decode(last).map(|(k, s)| lattice_of(k, s)),
        Some([67, 67, 67])
    );
}

#[test]
fn regions_far_from_the_start_open_into_the_cosmic_web() {
    // Spec v2 AC4: any region opens into an L2 cell, resident or not.
    let mut rng = Rng::new(1104);
    for _ in 0..12 {
        let id = FIXTURE + rng.next_u32() % 314_424;
        if marker(seed(), id).is_none() {
            continue; // outside the universe sphere: no region there
        }
        assert!(crate::destination::chain_opens(DEMO_SEED, &[id]), "{id}");
        let l2 = generate_cell(DEMO_SEED, &[id]);
        assert!(!l2.points.is_empty(), "{id} opens onto an empty cell");
    }
}

#[test]
fn the_journey_picks_among_the_fixture_alone() {
    let cell = generate_cell(DEMO_SEED, &[]);
    let level = Level::MIN;
    let picks = crate::home::cell_candidates(level, seed(), &cell);
    assert_eq!(picks, (0..FIXTURE).collect::<Vec<u32>>());
    // The journey's region on main (verify golden: JOURNEY opened=L2 marker=7).
    assert_eq!(crate::home::autopilot_path(DEMO_SEED).first(), Some(&7));
}
