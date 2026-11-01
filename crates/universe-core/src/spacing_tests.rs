use super::*;

#[test]
fn the_probe_measures_every_journey_level_from_l1_to_l7() {
    // #996 AC1: one row per level, L1 to L7, on the fixed journey.
    let rows = journey_spacing(DEMO_SEED);
    let levels: Vec<u8> = rows.iter().map(|s| s.level.get()).collect();
    assert_eq!(levels, vec![1, 2, 3, 4, 5, 6, 7]);
    assert!(rows.iter().all(|s| s.count > 0), "{rows:?}");
}

#[test]
fn l1_and_l7_keep_their_spacing() {
    // #996 AC11: the levels this item does not touch measure as before.
    for spacing in journey_spacing(DEMO_SEED) {
        let check = check_for(spacing.level);
        assert!(passes(&spacing, check), "{spacing:?} against {check:?}");
    }
}

#[test]
fn the_median_gap_is_the_study_method() {
    // Four markers on a line, gaps 0.1, 0.1, 0.2: nearest neighbours are
    // 0.1, 0.1, 0.1, 0.2, so the median is 0.1 cells.
    let point = |x: f64| Point::bare([x, 0.0, 0.0], 0.01, MarkerKind::Population);
    let level = Level::new(2).unwrap_or(Level::MIN);
    let spacing = measure(level, &[point(0.0), point(0.1), point(0.2), point(0.4)]);
    assert!((spacing.median_cells - 0.1).abs() < 1e-12, "{spacing:?}");
    let metres = 0.1 * 10f64.powf(scale_exponent(level));
    assert!((spacing.median_m - metres).abs() < metres * 1e-12);
}

#[test]
fn a_real_check_allows_a_factor_of_two_each_way() {
    let level = Level::new(4).unwrap_or(Level::MIN);
    let cell_m = 10f64.powf(scale_exponent(level));
    let spacing = Spacing {
        level,
        count: 100,
        portals: 1,
        median_cells: 1e18 / cell_m,
        median_m: 1e18,
    };
    let near = Check::Real {
        gap_m: 1.9e18,
        count: 190.0,
    };
    let far = Check::Real {
        gap_m: 2.1e18,
        count: 100.0,
    };
    assert!(passes(&spacing, near));
    assert!(!passes(&spacing, far));
}

#[test]
fn the_verify_lines_print_one_per_level() {
    let mut out = String::new();
    assert!(verify_spacing(&mut out), "{out}");
    assert_eq!(
        out.lines().filter(|l| l.starts_with("SPACING ")).count(),
        10
    );
}

#[test]
fn the_l5_core_and_edge_rows_print() {
    // #1104 T1: the central ball is pinned bit for bit (AC3); the edge zone
    // waits for the outer stars (T5).
    let mut out = String::new();
    assert!(verify_spacing(&mut out), "{out}");
    let core = out
        .lines()
        .find(|l| l.contains("cell=core"))
        .unwrap_or_default();
    assert!(core.contains("count=5000"), "{core}");
    assert!(core.contains("hash=cdb2dd0612eae0dc same PASS"), "{core}");
    let edge = out
        .lines()
        .find(|l| l.contains("cell=edge"))
        .unwrap_or_default();
    assert!(edge.ends_with(" PASS"), "{edge}");
}

#[test]
fn the_grid_search_equals_all_pairs() {
    // The bucket search is exact: same gaps as comparing every pair.
    use crate::seed::Rng;
    let mut rng = Rng::new(7);
    let points: Vec<Point> = (0..600)
        .map(|_| {
            let x = rng.next_f64() - 0.5;
            let y = (rng.next_f64() - 0.5) * 0.1;
            let z = rng.next_f64() * rng.next_f64() - 0.5;
            Point::bare([x, y, z], 0.001, MarkerKind::Population)
        })
        .collect();
    let mut brute: Vec<f64> = points
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            points
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, q)| distance(p.position, q.position))
                .min_by(f64::total_cmp)
        })
        .collect();
    brute.sort_by(f64::total_cmp);
    let level = Level::new(3).unwrap_or(Level::MIN);
    let spacing = measure(level, &points);
    assert_eq!(spacing.median_cells.to_bits(), median(&brute).to_bits());
}

#[test]
fn a_large_cell_is_sampled_at_a_fixed_stride() {
    // #1104 T1: hundreds of thousands of markers stay cheap to measure; a
    // regular lattice measures its own pitch whatever the stride.
    let n = 60i32;
    let pitch = 1.0 / f64::from(n);
    let points: Vec<Point> = (0..n * n * n)
        .map(|i| {
            let at = |k: i32| (f64::from(k) + 0.5) * pitch - 0.5;
            Point::bare(
                [at(i % n), at((i / n) % n), at(i / (n * n))],
                0.001,
                MarkerKind::Portal,
            )
        })
        .collect();
    assert!(points.len() > SAMPLE_CAP);
    let spacing = measure(Level::MIN, &points);
    assert_eq!(spacing.count, points.len());
    assert!((spacing.median_cells - pitch).abs() < 1e-9, "{spacing:?}");
}

#[test]
fn an_l6_cell_holds_a_neighbour_about_one_time_in_three() {
    // #996 Goal 5: its own system, and a neighbour as often as 0.35 more
    // stars in a 1.35 pc cell make one: 1 - e^-0.35, about 30 percent.
    use crate::r#gen::Generator;
    use crate::nest::level_budget;
    use crate::sysgen::GalaxyGenerator;
    let level = Level::new(6).unwrap_or(Level::MIN);
    let budget = level_budget(level);
    let mut with_neighbour = 0u32;
    for seed in 0..1000u64 {
        let cell = GalaxyGenerator::new(level, false, false, false).generate(seed, &budget);
        let systems = cell
            .points
            .iter()
            .filter(|p| p.kind == MarkerKind::Portal)
            .count();
        assert!((1..=2).contains(&systems), "seed {seed}: {systems}");
        assert_eq!(
            cell.points[0].position, [0.0; 3],
            "its own system is central"
        );
        with_neighbour += u32::from(systems == 2);
    }
    assert!((250..=350).contains(&with_neighbour), "{with_neighbour}");
}

#[test]
#[ignore = "timing probe (#996): cargo test --release -p universe-core --lib generation_cost_prints -- --ignored --nocapture"]
fn generation_cost_prints() {
    use std::time::Instant;
    let path = autopilot_path(DEMO_SEED);
    for depth in 0..=6usize {
        let chain = &path[..depth];
        let start = Instant::now();
        let cell = generate_cell(DEMO_SEED + 1000 + depth as u64, chain);
        let cold = start.elapsed().as_secs_f64() * 1e3;
        let start = Instant::now();
        let _ = autopilot_path(DEMO_SEED + 2000 + depth as u64);
        let path_ms = start.elapsed().as_secs_f64() * 1e3;
        let start = Instant::now();
        let _ = crate::names::home_name(DEMO_SEED, chain, 0);
        let name_ms = start.elapsed().as_secs_f64() * 1e3;
        println!(
            "GEN L{} points={} cold_ms={cold:.1} autopilot_path_ms={path_ms:.1} home_name_ms={name_ms:.1}",
            depth + 1,
            cell.points.len()
        );
    }
}

#[test]
#[ignore = "timing probe (#996): cargo test --release -p universe-core --lib drawn_cost_prints -- --ignored --nocapture"]
fn drawn_cost_prints() {
    use crate::drawn_set::{LayerFades, MarkerDetail, budget_cutoff, drawn_set};
    use crate::nav::{AUTOPILOT_RATE, DiveEvent, DiveMode, Universe};
    use crate::preview::PreviewCache;
    use std::time::Instant;
    let mut universe = Universe::new(DEMO_SEED);
    let mut previews = PreviewCache::default();
    let fades = LayerFades::settled();
    let mut set = Vec::new();
    let mut angles = Vec::new();
    let factor = (-AUTOPILOT_RATE / 60.0).exp();
    let mut target = None;
    let mut worst = [(0usize, 0.0f64); 11];
    for _ in 0..20_000 {
        if universe.level().get() >= 7 {
            break;
        }
        if target.is_none() {
            target = universe.journey_target();
        }
        if let DiveEvent::Opened(_) = universe.dive(target, factor, DiveMode::Journey) {
            target = None;
        }
        let start = Instant::now();
        previews.sync(&universe);
        drawn_set(&universe, &previews, &fades, &mut set);
        let _ = budget_cutoff(
            &set,
            universe.path.offset(),
            MarkerDetail::default().budget(),
            &mut angles,
        );
        let ms = start.elapsed().as_secs_f64() * 1e3;
        let slot = &mut worst[usize::from(universe.level().get())];
        if ms > slot.1 {
            *slot = (set.len(), ms);
        }
    }
    for (level, (len, ms)) in worst.iter().enumerate().skip(1).take(6) {
        println!("DRAWN L{level} set={len} worst_ms={ms:.2}");
    }
}
