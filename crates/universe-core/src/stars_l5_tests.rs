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
