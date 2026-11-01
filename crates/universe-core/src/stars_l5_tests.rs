use super::*;
use crate::home::autopilot_path;
use crate::nav::DEMO_SEED;
use crate::nest::generate_cell;

#[test]
fn the_home_cell_has_the_sun_at_its_centre_and_alpha_centauri_as_the_pick() {
    // #996 Goal 4: the journey keeps its recorded L5 pick (marker 2), now
    // Alpha Centauri 1.34 pc from the Sun at the centre.
    let journey = autopilot_path(DEMO_SEED);
    assert_eq!(journey.get(4), Some(&2), "the journey keeps its L5 pick");
    let cell = generate_cell(DEMO_SEED, &journey[..4]);
    assert_eq!(cell.points.len(), CORE_STARS as usize);
    assert_eq!(
        cell.points[0].position, [0.0; 3],
        "the Sun is at the centre"
    );
    let alpha = cell.points[2];
    assert_eq!(alpha.kind, MarkerKind::Portal);
    let pc = norm(alpha.position) * cell_pc();
    assert!((pc - ALPHA_CEN_PC).abs() < 1e-9, "{pc}");
}

#[test]
fn stars_fill_a_ball_at_the_real_density_and_bright_ones_open() {
    let budget = crate::nest::level_budget(Level::new(STAR_LEVEL).unwrap_or(Level::MIN));
    let cell = generate(99, &budget, None);
    let radius = core_radius_cells();
    assert!(
        cell.points
            .iter()
            .all(|p| norm(p.position) <= radius + 1e-12)
    );
    let open: Vec<&Point> = cell
        .points
        .iter()
        .filter(|p| p.kind == MarkerKind::Portal)
        .collect();
    assert!((15..=80).contains(&open.len()), "{}", open.len());
    assert!(open.iter().all(|p| is_bright(p)));
    // Ball of 5,000 at 0.14 per cubic parsec: about 40 pc across.
    let across_pc = 2.0 * radius * cell_pc();
    assert!((35.0..=45.0).contains(&across_pc), "{across_pc}");
}

fn home_l5() -> (Vec<u32>, u64) {
    let chain = autopilot_path(DEMO_SEED)[..4].to_vec();
    let seed = crate::nest::path_seed(DEMO_SEED, &chain);
    (chain, seed)
}

#[test]
fn outer_stars_fill_the_cell_beyond_the_ball() {
    // #1104 Spec v2 Goal 2: none inside the ball, all inside the cell,
    // about 135,000 at the real density.
    let (_, seed) = home_l5();
    let outer = all_outer(seed);
    let radius = core_radius_cells();
    assert!(outer.iter().all(|p| norm(p.position) >= radius));
    assert!(
        outer
            .iter()
            .all(|p| p.position.iter().all(|c| c.abs() <= 0.5))
    );
    let expected = 140_000.0 - f64::from(CORE_STARS);
    let count = f64::from(u32::try_from(outer.len()).unwrap_or(0));
    assert!((count / expected - 1.0).abs() < 0.02, "{count}");
}

#[test]
fn an_outer_star_is_rebuilt_from_its_id() {
    let (_, seed) = home_l5();
    let key = BlockKey([0, 9, 4]);
    let held = crate::blocks::block(&OuterStars, &OUTER_GRID, seed, key);
    assert!(!held.is_empty());
    for (id, point) in held {
        assert!(id >= CORE_STARS);
        assert_eq!(outer_marker(seed, id), Some(point));
    }
}

#[test]
fn bright_outer_stars_open_into_their_neighbourhood() {
    // As in the ball: A, B and O stars open; a far one leads to L6.
    let (chain, seed) = home_l5();
    let n = OUTER_GRID.per_axis;
    let bright = (0..n)
        .flat_map(|x| (0..OUTER_SLOTS).map(move |slot| (x, slot)))
        .find_map(|(x, slot)| {
            let id = OUTER_GRID.encode(BlockKey([x, 0, 0]), slot)?;
            let point = outer_marker(seed, id)?;
            (point.kind == MarkerKind::Portal).then_some((id, point))
        });
    let Some((id, point)) = bright else {
        panic!("a corner row of blocks holds a bright star");
    };
    assert!(is_bright(&point));
    let mut deeper = chain;
    deeper.push(id);
    assert!(crate::destination::chain_opens(DEMO_SEED, &deeper));
    assert!(!generate_cell(DEMO_SEED, &deeper).points.is_empty());
}
