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
    // Every marker back near the camera is the one that left, bit for
    // bit; the hysteresis may keep a few edge blocks either way.
    let mut back = 0usize;
    for (id, point) in &after {
        if let Some((_, was)) = before.iter().find(|(held, _)| held == id) {
            assert_eq!(point, was, "marker {id} changed");
            back += 1;
        }
    }
    assert!(
        back * 10 >= before.len() * 9,
        "{back} of {} came back",
        before.len()
    );
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

#[test]
fn marker_detail_limits_the_l1_regions_drawn() {
    // Spec v2 AC7: at L1 every region opens, and Low, Normal and High still
    // draw different numbers, never more than the budget.
    use crate::drawn_set::{LayerFades, MarkerDetail, budget_cutoff, drawn_set};
    use crate::frame::angular_radius;
    use crate::preview::PreviewCache;
    let mut universe = Universe::new(DEMO_SEED);
    universe.path.set_offset([0.05, -0.1, 0.2]);
    universe.stream_blocks();
    let previews = PreviewCache::default();
    let mut set = Vec::new();
    drawn_set(&universe, &previews, &LayerFades::settled(), &mut set);
    let camera = universe.path.offset();
    let mut angles = Vec::new();
    let mut counts = Vec::new();
    for detail in MarkerDetail::ALL {
        let cutoff = budget_cutoff(&set, camera, detail.budget(), &mut angles);
        let drawn = set
            .iter()
            .filter(|d| {
                let v = [
                    d.at[0] - camera[0],
                    d.at[1] - camera[1],
                    d.at[2] - camera[2],
                ];
                let distance = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                d.weight > 0.0 && angular_radius(d.radius, distance) >= cutoff
            })
            .count();
        assert!(drawn <= detail.budget() + 1, "{detail:?}: {drawn}");
        counts.push(drawn);
    }
    assert!(counts.windows(2).all(|w| w[0] < w[1]), "{counts:?}");
}

#[test]
fn streamed_markers_fade_at_the_edge_and_the_fixture_never_does() {
    let universe = Universe::new(DEMO_SEED);
    let camera = universe.path.offset();
    let far = [camera[0] + 100.0, camera[1], camera[2]];
    assert_eq!(universe.stream_fade(0, 7, far), 1.0, "fixture region");
    assert_eq!(universe.stream_fade(0, 9_000, far), 0.0, "far block marker");
    assert_eq!(
        universe.stream_fade(0, 9_000, camera),
        1.0,
        "near block marker"
    );
}

#[test]
#[ignore = "residency probe (#1104 T8): cargo test --release -p universe-core --lib resident_max_prints -- --ignored --nocapture"]
fn resident_max_prints() {
    // Largest resident set per streamed cell: across L1 on a straight
    // flight, and on the journey down to L7 (open, parent, grandparent).
    let mut universe = Universe::new(DEMO_SEED);
    let mut l1_max = 0usize;
    for step in 0..=400 {
        let t = f64::from(step) / 400.0;
        universe
            .path
            .set_offset([1.4 - 2.8 * t, 1.0 - 2.0 * t, 1.4 - 2.8 * t]);
        universe.stream_blocks();
        l1_max = l1_max.max(universe.open.points.len());
    }
    let mut universe = Universe::new(DEMO_SEED);
    let factor = (-crate::nav::AUTOPILOT_RATE / 60.0).exp();
    let mut target = None;
    let mut worst = [0usize; 8];
    for _ in 0..40_000 {
        let level = usize::from(universe.level().get());
        if level >= 8 {
            break;
        }
        let live = universe.open.points.len()
            + universe.parent.as_ref().map_or(0, |c| c.points.len())
            + universe.grandparent.as_ref().map_or(0, |c| c.points.len());
        if let Some(slot) = worst.get_mut(level) {
            *slot = (*slot).max(live);
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
    println!("RESIDENT l1_flight_max={l1_max} journey_live_by_level={worst:?}");
    let mut universe = Universe::new(DEMO_SEED);
    let mut target = None;
    for _ in 0..40_000 {
        if universe.level().get() >= 5 {
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
    let mut l5_max = 0usize;
    for step in 0..=400 {
        let t = f64::from(step) / 400.0;
        universe
            .path
            .set_offset([-0.49 + 0.98 * t, 0.3 - 0.6 * t, 0.45 - 0.9 * t]);
        universe.stream_blocks();
        l5_max = l5_max.max(universe.open.points.len());
    }
    println!("RESIDENT l5_flight_max={l5_max}");
}

#[test]
fn a_flight_across_l1_stays_within_the_resident_bound() {
    // ADR 0012 #1104 amendment: fixture plus at most RESIDENT_BOUND.
    let mut universe = Universe::new(DEMO_SEED);
    let bound = crate::regions_l1::FIXTURE + RESIDENT_BOUND;
    for step in 0..=60 {
        let t = f64::from(step) / 60.0;
        universe
            .path
            .set_offset([1.4 - 2.8 * t, 1.0 - 2.0 * t, 1.4 - 2.8 * t]);
        universe.stream_blocks();
        let held = u32::try_from(universe.open.points.len()).unwrap_or(u32::MAX);
        assert!(held <= bound, "step {step}: {held} > {bound}");
    }
}

#[test]
fn the_l5_shot_places_hold_their_own_stars_at_the_same_density() {
    // #1104 R1 (C19): at the shots' three places (the centre, the old 20-pc
    // edge, the outer 10 pc) the stars near the camera are as many and are
    // other stars. Prints the counts for the shot evidence.
    let mut universe = Universe::new(DEMO_SEED);
    let journey = crate::home::autopilot_path(DEMO_SEED);
    let factor = (-crate::nav::AUTOPILOT_RATE / 60.0).exp();
    let mut target = None;
    for _ in 0..40_000 {
        if universe.path.indices() == journey[..4] {
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
    // An L5 cell is 100 pc across.
    let pc = 0.01;
    let mut nearest_sets = Vec::new();
    for (name, at) in [
        ("centre", [0.0, 0.0, 0.02]),
        ("old-edge", [0.2, -0.18, 0.05]),
        ("outer", [0.45, -0.4, 0.1]),
    ] {
        universe.path.set_offset(at);
        universe.stream_blocks();
        let mut near: Vec<(f64, u32)> = universe
            .open
            .with_ids()
            .filter(|(_, p)| p.star.is_some())
            .map(|(id, p)| {
                let d: f64 = (0..3).map(|a| (p.position[a] - at[a]).powi(2)).sum();
                (d.sqrt() / pc, id)
            })
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        let within = |r: f64| near.iter().filter(|(d, _)| *d <= r).count();
        let ids: Vec<u32> = near.iter().take(3).map(|(_, id)| *id).collect();
        println!(
            "L5-STARS place={name} within_2pc={} within_5pc={} within_10pc={} nearest_ids={ids:?} nearest_pc={:.2}",
            within(2.0),
            within(5.0),
            within(10.0),
            near.first().map_or(0.0, |n| n.0)
        );
        // 0.14 per cubic parsec: about 73 stars within 5 pc.
        let five = within(5.0);
        assert!(
            (40..=110).contains(&five),
            "{name}: {five} stars within 5 pc"
        );
        nearest_sets.push(ids);
    }
    assert!(
        nearest_sets.windows(2).all(|w| w[0] != w[1]),
        "each place has its own nearest stars"
    );
}
