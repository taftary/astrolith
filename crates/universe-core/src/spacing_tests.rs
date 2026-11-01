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
    assert_eq!(out.lines().filter(|l| l.starts_with("SPACING ")).count(), 8);
}
