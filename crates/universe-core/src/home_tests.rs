//! Journey matching tests (#1153).

use super::*;
use crate::nav::DEMO_SEED;
use crate::nest::{autopilot_path, generations};

#[test]
fn asking_twice_whether_the_journey_planet_is_home_builds_no_cell_again() {
    // #1153: the planet overlay and the route ask this every frame; since
    // #1113 each answer replayed the whole journey, rebuilding the L7-L9
    // cells (about 80 ms a frame on the ground).
    let journey = autopilot_path(DEMO_SEED);
    assert!(
        is_home_planet_cell(DEMO_SEED, &journey),
        "journey planet is home"
    );
    let before = generations();
    assert!(
        is_home_planet_cell(DEMO_SEED, &journey),
        "same answer again"
    );
    assert_eq!(generations(), before, "the second answer builds no cell");
}

#[test]
fn a_neighbour_planet_stays_not_home_when_asked_again() {
    let mut neighbour = autopilot_path(DEMO_SEED);
    if let Some(last) = neighbour.last_mut() {
        *last += 1;
    }
    assert!(
        !is_home_planet_cell(DEMO_SEED, &neighbour),
        "neighbour is not home"
    );
    let before = generations();
    assert!(
        !is_home_planet_cell(DEMO_SEED, &neighbour),
        "same answer again"
    );
    assert_eq!(generations(), before, "the second answer builds no cell");
}
