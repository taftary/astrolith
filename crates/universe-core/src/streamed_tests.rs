use super::*;
use crate::nav::{DEMO_SEED, Universe};

#[test]
fn the_universe_starts_with_blocks_near_the_camera() {
    // Spec v2 Goal 1: L1 shows its regions near the camera, not only 8.
    let universe = Universe::new(DEMO_SEED);
    assert!(
        universe.open.points.len() > 5_000,
        "{}",
        universe.open.points.len()
    );
    assert_eq!(universe.open.points.len(), universe.open.ids.len());
    // The fixture leads, IDs 0 to 7.
    assert_eq!(&universe.open.ids[..8], &[0, 1, 2, 3, 4, 5, 6, 7]);
    assert!(
        universe.open.ids.windows(2).all(|w| w[0] < w[1]),
        "IDs ascend, no marker twice"
    );
}

#[test]
fn every_resident_marker_is_its_id_rebuilt() {
    // Spec v2 AC5: a resident marker equals the marker its ID rebuilds.
    let universe = Universe::new(DEMO_SEED);
    let seed = universe.open_seed();
    for (id, point) in universe.open.with_ids().step_by(97) {
        assert_eq!(
            Field::Regions.marker(seed, id).as_ref(),
            Some(point),
            "{id}"
        );
    }
}

#[test]
fn a_still_camera_rebuilds_nothing() {
    // Spec v2 AC9.
    let mut universe = Universe::new(DEMO_SEED);
    let revision = universe.content_revision();
    for _ in 0..10 {
        assert!(!universe.stream_blocks());
    }
    assert_eq!(universe.content_revision(), revision);
}

#[test]
fn leaving_and_coming_back_gives_the_same_markers() {
    // Spec v2 AC5: same IDs and bits on return.
    let mut universe = Universe::new(DEMO_SEED);
    let start = universe.path.offset();
    let before: Vec<(u32, Point)> = universe.open.with_ids().map(|(i, p)| (i, *p)).collect();
    universe.path.set_offset([-0.3, 0.2, -0.1]);
    assert!(universe.stream_blocks(), "a far move changes the blocks");
    assert_ne!(
        universe.open.ids,
        before.iter().map(|(i, _)| *i).collect::<Vec<u32>>()
    );
    universe.path.set_offset(start);
    universe.stream_blocks();
    let after: Vec<(u32, Point)> = universe.open.with_ids().map(|(i, p)| (i, *p)).collect();
    let mut a = before;
    let mut b = after;
    a.sort_by_key(|(i, _)| *i);
    b.sort_by_key(|(i, _)| *i);
    assert_eq!(a, b);
}

#[test]
fn a_small_step_across_a_block_edge_keeps_every_block() {
    // Spec v2 Goal 4: no churn at a block edge (hysteresis).
    let mut universe = Universe::new(DEMO_SEED);
    universe.path.set_offset([0.0, 0.0, 0.0]);
    universe.stream_blocks();
    let held = universe.open.ids.clone();
    let side = crate::regions_l1::GRID.side();
    universe.path.set_offset([side * 0.2, 0.0, 0.0]);
    universe.stream_blocks();
    universe.path.set_offset([0.0, 0.0, 0.0]);
    universe.stream_blocks();
    assert!(held.iter().all(|id| universe.open.ids.contains(id)));
}

#[test]
fn the_journey_l5_cell_streams_its_outer_stars() {
    // Spec v2 Goal 2: far out in the L5 cell there are stars near you.
    let mut universe = Universe::new(DEMO_SEED);
    let journey = crate::home::autopilot_path(DEMO_SEED);
    let mut target = None;
    let factor = (-crate::nav::AUTOPILOT_RATE / 60.0).exp();
    for _ in 0..40_000 {
        if universe.level().get() >= 5 && universe.path.indices() == journey[..4] {
            break;
        }
        if target.is_none() {
            target = universe.journey_target();
        }
        if let crate::nav::DiveEvent::Opened(_) =
            universe.dive(target, factor, crate::nav::DiveMode::Journey)
        {
            target = None;
        }
    }
    assert_eq!(universe.path.indices(), journey[..4].to_vec());
    let core = crate::stars_l5::CORE_STARS as usize;
    universe.path.set_offset([0.42, -0.4, 0.1]);
    universe.stream_blocks();
    assert!(universe.open.points.len() > core + 1_000);
    let near = universe
        .open
        .points
        .iter()
        .filter(|p| {
            let d: f64 = (0..3)
                .map(|a| (p.position[a] - [0.42, -0.4, 0.1][a]).powi(2))
                .sum();
            d.sqrt() < 0.05
        })
        .count();
    assert!(near > 50, "stars around the camera at the old edge: {near}");
}

#[test]
#[ignore = "timing probe (#1104 T6): cargo test --release -p universe-core --lib block_cost_prints -- --ignored --nocapture"]
fn block_cost_prints() {
    use std::time::Instant;
    let mut universe = Universe::new(DEMO_SEED);
    universe.path.set_offset([0.0, 0.0, 0.0]);
    universe.stream_blocks();
    let side = crate::regions_l1::GRID.side();
    let mut worst = 0.0f64;
    let mut total = 0.0f64;
    let mut changes = 0u32;
    for step in 1..=200 {
        let x = f64::from(step) * side * 0.05;
        universe.path.set_offset([x, 0.0, 0.0]);
        let start = Instant::now();
        let changed = universe.stream_blocks();
        let ms = start.elapsed().as_secs_f64() * 1e3;
        total += ms;
        if changed {
            changes += 1;
        }
        worst = worst.max(ms);
    }
    let level = universe.level();
    let start = Instant::now();
    let names = universe
        .open
        .with_ids()
        .map(|(id, p)| crate::names::marker_name(DEMO_SEED, &[], level, id, p).len())
        .sum::<usize>();
    let names_ms = start.elapsed().as_secs_f64() * 1e3;
    println!("BLOCKS L1 names_ms={names_ms:.2} chars={names}");
    println!(
        "BLOCKS L1 resident={} changes={changes} worst_ms={worst:.3} mean_ms={:.3}",
        universe.open.points.len(),
        total / 200.0
    );
}
