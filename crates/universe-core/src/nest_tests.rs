//! `nest` tests, split from `nest` under `E-FILE-SIZE` (#995).
use super::*;
use crate::density::densest_portal_index;
use crate::r#gen::MarkerKind;
use crate::nav::{DEMO_SEED, DiveEvent, DiveMode, MarkerIndex, Universe, WHEEL_FACTOR};
use crate::snapshot::snapshot_generated;

fn level(n: u8) -> Level {
    Level::new(n).expect("ladder level")
}

#[test]
fn ratios_follow_ladder_anchors_and_shrink() {
    for n in 1..MAX_LEVEL {
        let ratio = child_ratio(level(n)).expect("has child");
        let expected =
            10f64.powf(LADDER_EXPONENTS[usize::from(n)] - LADDER_EXPONENTS[usize::from(n - 1)]);
        assert!((ratio - expected).abs() < 1e-12, "L{n} ratio");
        assert!(
            ratio > 0.0 && ratio < 1.0,
            "L{n} ratio out of (0,1): {ratio}"
        );
    }
    assert!(child_ratio(Level::MAX).is_none());
    // Heliopause -> Sun is the widest gap: four decades.
    assert!(child_ratio(level(8)).expect("L8") < 1e-4);
    // Stellar neighborhood -> outer system is the narrowest: under half a decade.
    assert!(child_ratio(level(6)).expect("L6") > 0.3);
}

#[test]
fn budgets_cap_by_packing_and_stay_valid() {
    for n in 1..=MAX_LEVEL {
        let budget = level_budget(level(n));
        assert!(budget.is_valid(), "L{n} budget invalid");
        // L4 admits rich cells up to the fixture cap; every other level
        // stays within its base count (or the tighter packing cap).
        let cap = if n == 4 {
            RICH_CLUSTER_TOTAL
        } else {
            base_count(level(n)).max(MIN_MARKERS)
        };
        assert!(budget.max_count <= cap, "L{n} budget over cap");
    }
    assert!(level_budget(level(6)).max_count < 10, "L6 should be sparse");
    assert_eq!(level_budget(level(1)).max_count, 48);
    assert_eq!(
        level_budget(level(4)).max_count,
        RICH_CLUSTER_TOTAL,
        "L4 admits rich cells"
    );
}

#[test]
fn same_path_regenerates_identical_cells() {
    let chain = [3u32, 1, 7, 0, 2];
    let first = snapshot_generated(&generate_cell(42, &chain));
    let second = snapshot_generated(&generate_cell(42, &chain));
    assert_eq!(first, second);
    assert_ne!(
        first,
        snapshot_generated(&generate_cell(42, &[3, 1, 7, 0, 3]))
    );
    assert_ne!(first, snapshot_generated(&generate_cell(43, &chain)));
    assert_eq!(path_seed(42, &chain), path_seed(42, &chain));
    assert_ne!(path_seed(42, &chain), path_seed(42, &chain[..4]));
}

#[test]
fn every_level_down_to_l14_has_markers() {
    let chain = autopilot_path(42);
    assert_eq!(chain.len(), usize::from(MAX_OPEN_LEVEL - 1));
    for depth in 0..=chain.len() {
        let cell = generate_cell(42, &chain[..depth]);
        let budget = level_budget(path_level(&chain[..depth]));
        assert!(
            !cell.points.is_empty() && cell.points.len() <= budget.max_count as usize,
            "depth {depth} count {}",
            cell.points.len()
        );
        // #775 Q7 A: L10 moons live in the planet's own space (reach),
        // not the cell; every other point stays in the cell.
        // #995 T7: the star close-up holds its planets at their true
        // places, thousands of cells out.
        if path_level(&chain[..depth]).get() == 9 {
            continue;
        }
        let reach = if path_level(&chain[..depth]).get() == 10 {
            crate::nav::planet_space_for_chain(42, &chain[..depth])
                .map(|space| space.reach)
                .unwrap_or(HALF_BOUND)
        } else {
            HALF_BOUND
        };
        for point in &cell.points {
            let d = (point.position[0] * point.position[0]
                + point.position[1] * point.position[1]
                + point.position[2] * point.position[2])
                .sqrt();
            assert!(
                d <= reach + 1e-9,
                "depth {depth} point {d} beyond {reach}: {:?}",
                point.position
            );
        }
    }
    assert_eq!(chain, autopilot_path(42), "autopilot path must be fixed");
}

#[test]
fn open_then_close_is_exact_inverse() {
    let start = [1.1, 0.7, -0.9];
    let mut path = MarkerPath::root(start);
    let chain = autopilot_path(42);
    let mut positions = Vec::new();
    for &marker in &chain {
        let cell = generate_cell(42, &path.indices());
        let position = marker_position(&cell, marker).expect("marker exists");
        positions.push(position);
        assert!(path.open(marker, position));
        assert_eq!(path.indices().last(), Some(&marker));
    }
    assert_eq!(path.level().get(), MAX_OPEN_LEVEL);
    assert!(!path.can_open());
    assert!(!path.open(0, [0.0; 3]), "L14 markers must not open");
    for expected in positions.iter().rev() {
        let closed = path.close().expect("something to close");
        assert_eq!(closed.position, *expected);
    }
    assert!(path.close().is_none());
    assert_eq!(path.level(), Level::MIN);
    for (got, want) in path.offset().iter().zip(start) {
        assert!((got - want).abs() < 1e-9, "offset drifted: {got} vs {want}");
    }
}

#[test]
fn opening_keeps_the_camera_where_it_was() {
    let mut path = MarkerPath::root([0.0, 0.0, 2.0]);
    let marker = [0.1, 0.0, 0.0];
    let ratio = child_ratio(Level::MIN).expect("L1 ratio");
    assert!(path.open(0, marker));
    let offset = path.offset();
    let expected = [-0.1 / ratio, 0.0, 2.0 / ratio];
    for (got, want) in offset.iter().zip(expected) {
        assert!((got - want).abs() < 1e-9);
    }
    assert!(
        path.distance_to_center() > 1.0,
        "camera must still be outside"
    );
}

#[test]
fn opened_frame_maps_points_with_the_path_and_directions_by_rotation_only() {
    let up = [0.6, 0.8, 0.0];
    let opened = Opened {
        marker: 3,
        position: [0.2, -0.1, 0.3],
        ratio: child_ratio(Level::MIN).expect("L1 ratio"),
        anonymous: false,
        up,
    };
    // Points cross the frame exactly like the camera offset does.
    let mut path = MarkerPath::root([0.9, 0.4, -0.7]);
    assert!(path.open_oriented(3, opened.position, up));
    assert_eq!(path.offset(), opened.to_child_point([0.9, 0.4, -0.7]));
    let back = opened.to_parent_point(path.offset());
    for (got, want) in back.iter().zip([0.9, 0.4, -0.7]) {
        assert!((got - want).abs() < 1e-9, "point round trip drifted");
    }
    // Directions only turn: unit length is kept and the round trip is exact.
    let direction = [0.0, 0.0, -1.0];
    let turned = opened.to_child_direction(direction);
    let length = (turned[0] * turned[0] + turned[1] * turned[1] + turned[2] * turned[2]).sqrt();
    assert!(
        (length - 1.0).abs() < 1e-12,
        "rotation must keep unit length"
    );
    assert!(
        (turned[2] + 1.0).abs() > 1e-6,
        "a tilted frame must turn the direction"
    );
    let returned = opened.to_parent_direction(turned);
    for (got, want) in returned.iter().zip(direction) {
        assert!((got - want).abs() < 1e-12, "direction round trip drifted");
    }
    // Identity frames leave directions untouched bit for bit.
    let flat = Opened {
        up: IDENTITY_UP,
        ..opened
    };
    assert_eq!(flat.to_child_direction(direction), direction);
}

#[test]
fn anon_table_matches_spec_spans() {
    // #770: L11-L14 removed; spans stop at L10.
    let table = [
        (1, 2),
        (2, 2),
        (3, 2),
        (4, 2),
        (5, 2),
        (6, 0),
        (7, 2),
        (8, 3),
        (9, 2),
        (10, 0),
    ];
    for (n, k) in table {
        assert_eq!(anon_cells(level(n)), k, "L{n} span milestones");
    }
    assert_eq!(anon_cells(Level::MAX), 0, "the terminal rung takes none");
}

#[test]
fn oriented_open_and_close_are_exact_inverses() {
    let start = [0.5, 0.4, -0.3];
    let mut path = MarkerPath::root(start);
    let up = [1.0, 0.0, 0.0];
    assert!(path.open_oriented(0, [0.1, 0.0, 0.0], up));
    // The camera stood off along +X, which is the child up: it lands
    // above the child ground.
    assert!(path.offset()[1] > 1.0, "camera must sit above the ground");
    let closed = path.close().expect("something to close");
    assert_eq!(closed.up, up);
    for (got, want) in path.offset().iter().zip(start) {
        assert!((got - want).abs() < 1e-9, "oriented offset drifted");
    }
}

#[test]
fn milestones_stay_invisible_to_generation() {
    let mut path = MarkerPath::root([0.0; 3]);
    assert!(path.is_at_root());
    assert!(path.open(3, [0.1, 0.2, 0.3]));
    path.open_anonymous(3);
    path.open_anonymous(5);
    assert_eq!(path.indices(), vec![3]);
    assert_eq!(path.level().get(), 2);
    assert_eq!(path.entered().map(|o| o.marker), Some(3));
    assert_eq!(path.top_anon_run(), 2);
    assert_eq!(path.anonymous_depth(), 2);
    assert!(!path.is_at_root());
}

#[test]
fn milestones_unwind_exactly() {
    let start = [1.1, 0.7, -0.9];
    let mut path = MarkerPath::root(start);
    assert!(path.open(3, [0.1, 0.0, 0.0]));
    path.open_anonymous(3);
    path.open_anonymous(3);
    assert!(path.open(5, [-0.2, 0.1, 0.0]));
    path.open_anonymous(5);
    while path.close().is_some() {}
    assert!(path.is_at_root());
    assert_eq!(path.indices(), Vec::<u32>::new());
    for (got, want) in path.offset().iter().zip(start) {
        assert!((got - want).abs() < 1e-9, "offset drifted: {got} vs {want}");
    }
}

#[test]
fn magnification_milestones_mark_long_spans() {
    let mut universe = Universe::new(DEMO_SEED);
    let marker = universe.autopilot_target().expect("root has markers");
    let mut max_run = 0usize;
    let mut opened = false;
    for _ in 0..1000 {
        match universe.dive(Some(marker), WHEEL_FACTOR, DiveMode::Targeted) {
            DiveEvent::Opened(m) => {
                assert_eq!(m, marker);
                opened = true;
                break;
            }
            DiveEvent::Closed(_) => panic!("closed while approaching"),
            DiveEvent::Moved => {}
        }
        max_run = max_run.max(universe.path.top_anon_run());
    }
    assert!(opened, "never opened L2");
    assert_eq!(max_run, 2, "L1-L2 span crosses 2 milestones");
    assert_eq!(universe.level().get(), 2);
}

#[test]
fn milestones_need_target_portal_and_long_span() {
    // No target: dives to the center, pushes nothing.
    let mut universe = Universe::new(DEMO_SEED);
    for _ in 0..300 {
        universe.dive(None, WHEEL_FACTOR, DiveMode::Targeted);
    }
    assert_eq!(universe.path.top_anon_run(), 0);
    assert_eq!(universe.level(), Level::MIN);
    // Population target: never opens, never milestones.
    let population = universe
        .open
        .points
        .iter()
        .position(|point| point.kind == MarkerKind::Population);
    if let Some(index) = population {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: test index into a budgeted cell, always fits u32"
        )]
        let marker = index as u32;
        for _ in 0..300 {
            assert_eq!(
                universe.dive(Some(marker), WHEEL_FACTOR, DiveMode::Targeted),
                DiveEvent::Moved
            );
        }
        assert_eq!(universe.path.top_anon_run(), 0);
        assert_eq!(universe.level(), Level::MIN);
    }
    // Short span (L6-L7 takes none): approach opens with no milestones.
    // Manual opens leave a far offset, so the first dives may close back
    // out; retarget like the autopilot until L7 opens.
    let chain = autopilot_path(DEMO_SEED);
    let mut far = Universe::new(DEMO_SEED);
    for &m in &chain[..5] {
        assert!(far.open(MarkerIndex(m)), "open to L6");
    }
    assert_eq!(far.level().get(), 6);
    let mut target: Option<u32> = None;
    let mut opened = false;
    for _ in 0..10000 {
        if target.is_none() {
            target = far.autopilot_target();
        }
        match far.dive(target, WHEEL_FACTOR, DiveMode::Targeted) {
            DiveEvent::Opened(_) => {
                if far.level().get() == 7 {
                    opened = true;
                    break;
                }
                target = None;
            }
            DiveEvent::Closed(o) => {
                target = Some(o.marker);
            }
            DiveEvent::Moved => {
                if far.level().get() == 6 {
                    assert_eq!(far.path.top_anon_run(), 0, "short span milestones");
                }
            }
        }
    }
    assert!(opened, "never opened L7");
    assert_eq!(far.level().get(), 7);
}

#[test]
fn marker_seeds_depend_on_index_and_level() {
    let base = marker_seed(7, level(2), 0);
    assert_ne!(base, marker_seed(7, level(2), 1));
    assert_ne!(base, marker_seed(7, level(3), 0));
    assert_eq!(base, marker_seed(7, level(2), 0));
}

#[test]
fn home_journey_fixtures_hold() {
    let chain = autopilot_path(42);
    assert_eq!(chain.len(), 9, "home journey must open L2-L10");
    // Home L3 pick lands in the group tier (structural journey rule).
    let level3 = Level::new(3).expect("L3");
    let l3seed = path_seed(42, &chain[..2]);
    let l3 = generate_cell(42, &chain[..2]);
    let (clusters, _) = crate::density::portal_tiers(level3, l3seed, l3.points.len());
    let candidates = autopilot_candidates(level3, l3seed, &l3.points);
    let pick = autopilot_marker(l3seed, &candidates).expect("L3 pick");
    assert!(
        (pick as usize) >= clusters,
        "home L3 pick {pick} must be a group portal (clusters={clusters})"
    );
    // Home L4 reached through the group is poor, not rich.
    assert!(
        !is_rich_cluster_cell(42, &chain[..3]),
        "group-entered L4 must not be rich"
    );
    // The densest cluster portal of the home L3 cell opens rich content.
    let home_l3 = generate_cell(42, &chain[..2]);
    let anchor = chain[..2].last().copied().map_or(0, i64::from);
    let densest = densest_portal_index(l3seed, anchor, &home_l3.points)
        .expect("home L3 has a cluster portal");
    assert!(
        (densest as usize) < clusters,
        "densest portal must be a cluster portal"
    );
    let mut rich_chain = chain[..2].to_vec();
    rich_chain.push(densest);
    assert!(
        is_rich_cluster_cell(42, &rich_chain),
        "densest-cluster cell must be rich"
    );
    let rich = generate_cell(42, &rich_chain);
    assert_eq!(rich.points.len(), 2000, "rich L4 holds thousands");
    let rich_portals = rich
        .points
        .iter()
        .filter(|point| point.kind == MarkerKind::Portal)
        .count();
    assert_eq!(rich_portals, 160, "rich L4 portals for large members");
    // L6-L9 home content is pinned by the home-system test below.
    let l6 = generate_cell(42, &chain[..5]);
    assert!(
        l6.points
            .iter()
            .any(|point| point.kind == MarkerKind::Portal),
        "home L6 must hold a system portal"
    );
}

#[test]
fn autopilot_skips_populations_deterministically() {
    assert!(autopilot_marker(1, &[]).is_none());
    assert!(autopilot_marker(1, &[0, 1, 2, 3, 4]).expect("index") < 5);
    let portals = [2u32, 5, 9];
    let first = autopilot_marker(7, &portals).expect("portal");
    assert!(portals.contains(&first));
    assert_eq!(first, autopilot_marker(7, &portals).expect("portal"));
    let other = autopilot_marker(8, &portals).expect("portal");
    assert!(portals.contains(&other));
}

#[test]
fn home_system_cells_carry_alpha_cen_and_solar_fixtures() {
    use crate::r#gen::StarKind;
    use crate::system::{ALPHA_CEN_TRIPLE, SOLAR_AXES_AU};
    let chain = autopilot_path(DEMO_SEED);
    assert!(chain.len() >= 8, "journey must reach L9");
    // L6 home: the journey pick is Alpha Centauri A with B and Proxima
    // beside it, all fixed catalog data on sampled positions.
    let home6 = &chain[..5];
    assert!(is_home_system_cell(DEMO_SEED, home6));
    assert!(!is_home_system_cell(DEMO_SEED, &chain[..4]));
    let l6 = generate_cell(DEMO_SEED, home6);
    let seed6 = path_seed(DEMO_SEED, home6);
    let candidates = autopilot_candidates(level(6), seed6, &l6.points);
    let pick = autopilot_marker(seed6, &candidates).expect("home L6 pick");
    assert_eq!(pick, chain[5], "the journey opens Alpha Centauri");
    let primary = l6.points[pick as usize];
    assert_eq!(primary.kind, MarkerKind::Portal);
    let star = primary.star.expect("primary is a star");
    assert_eq!(star.kind, StarKind::G, "Alpha Centauri A is G2V-like");
    assert_eq!(star.mass_solar, ALPHA_CEN_TRIPLE[0].1);
    let mut seen_b = false;
    let mut seen_proxima = false;
    for point in &l6.points {
        if point.kind != MarkerKind::Population {
            continue;
        }
        if let Some(info) = point.star {
            seen_b |= info.kind == StarKind::K && info.mass_solar == ALPHA_CEN_TRIPLE[1].1;
            seen_proxima |= info.kind == StarKind::M && info.mass_solar == ALPHA_CEN_TRIPLE[2].1;
        }
    }
    assert!(seen_b, "Alpha Centauri B rides along");
    assert!(seen_proxima, "Proxima rides along");
    // A sibling off the path is procedural, never the triple.
    let mut sibling = home6.to_vec();
    let last = sibling.len() - 1;
    sibling[last] = sibling[last].wrapping_add(1);
    assert!(!is_home_system_cell(DEMO_SEED, &sibling));
    // L8 home: the Sun with the eight catalog planets in order.
    assert!(is_home_system_cell(DEMO_SEED, &chain[..7]));
    let l8 = generate_cell(DEMO_SEED, &chain[..7]);
    let sun = l8.points.first().expect("L8 star").star.expect("Sun");
    assert_eq!(sun.kind, StarKind::G);
    assert_eq!(sun.mass_solar, 1.0);
    let axes: Vec<f64> = l8
        .points
        .iter()
        .skip(1)
        .filter(|point| point.kind == MarkerKind::Portal)
        .filter_map(|point| point.planet.map(|data| data.orbit_au))
        .collect();
    assert_eq!(axes.len(), SOLAR_AXES_AU.len(), "Solar home holds eight");
    // The journey's planet shows the Earth's slot (#995 T6), so the
    // catalog is in the cell once each, with the Earth at the journey's
    // marker.
    let mut sorted = axes.clone();
    sorted.sort_by(f64::total_cmp);
    for (slot, axis) in sorted.iter().enumerate() {
        assert!(
            (axis - SOLAR_AXES_AU[slot]).abs() < 1e-12,
            "slot {slot} axis drifted: {axis}"
        );
    }
    let journey = chain[8] as usize;
    assert_eq!(axes[journey], 1.0, "the journey lands at 1 AU");
    // L9 home shows the same eight through the shared seed.
    assert!(is_home_system_cell(DEMO_SEED, &chain[..8]));
    let l9 = generate_cell(DEMO_SEED, &chain[..8]);
    let nine: Vec<_> = l9
        .points
        .iter()
        .filter(|point| point.kind == MarkerKind::Portal)
        .filter_map(|point| point.planet)
        .collect();
    let eight: Vec<_> = l8
        .points
        .iter()
        .skip(1)
        .filter(|point| point.kind == MarkerKind::Portal)
        .filter_map(|point| point.planet)
        .collect();
    assert_eq!(eight, nine, "L9 close-up matches the L8 system");
}

#[test]
fn home_planet_cell_carries_earth_and_moon() {
    use crate::terrain::{
        EARTH_AIR_THICKNESS_KM, EARTH_SPIN_HOURS, EARTH_TILT_DEG, MOON_ORBIT_KM, MOON_PERIOD_DAYS,
        MOON_RADIUS_KM, THIN_AIR_TINT,
    };
    let chain = autopilot_path(DEMO_SEED);
    assert_eq!(chain.len(), 9, "home journey must open L2-L10");
    assert!(is_home_planet_cell(DEMO_SEED, &chain[..9]));
    assert!(!is_home_planet_cell(DEMO_SEED, &chain[..8]));
    let home = generate_cell(DEMO_SEED, &chain[..9]);
    let moons: Vec<_> = home.points.iter().filter_map(|point| point.moon).collect();
    assert_eq!(moons.len(), 1, "Earth keeps exactly one Moon");
    assert_eq!(moons[0].radius_km, MOON_RADIUS_KM);
    assert_eq!(
        home.points.len(),
        4,
        "one record plus Moon plus two scenery (#770: portals removed)"
    );
    assert_eq!(moons[0].orbit_km, MOON_ORBIT_KM);
    assert_eq!(moons[0].period_days, MOON_PERIOD_DAYS);
    let surfaces: Vec<_> = home
        .points
        .iter()
        .filter_map(|point| point.surface)
        .collect();
    assert_eq!(surfaces.len(), 1, "one surface record carries the recipe");
    for surface in &surfaces {
        assert_eq!(surface.tilt_deg, EARTH_TILT_DEG);
        assert_eq!(surface.spin_hours, EARTH_SPIN_HOURS);
        assert_eq!(surface.radius_earth, 1.0, "home planet is Earth-sized");
        let air = surface.air.expect("home planet holds air");
        assert_eq!(air.thickness_km, EARTH_AIR_THICKNESS_KM);
        assert_eq!(air.tint, THIN_AIR_TINT);
    }
    let portals = home
        .points
        .iter()
        .filter(|point| point.kind == MarkerKind::Portal)
        .count();
    assert_eq!(portals, 0, "#770: region portals removed");
    for point in &home.points {
        if point.kind == MarkerKind::Population {
            let scenery = point.surface.is_none() && point.moon.is_none();
            assert!(
                point.surface.is_some() != point.moon.is_some() || scenery,
                "a population point is surface, moon, or scenery"
            );
        }
    }
    // Region portals sit on the surface as patches; the Moon wears its
    // orbit; the equator ring and the spin axis draw as scenery (#384).
    use crate::r#gen::Form;
    for point in home.points.iter().filter(|point| point.surface.is_none()) {
        if point.kind == MarkerKind::Portal {
            assert!(
                matches!(point.form, Form::Patch { .. }),
                "region portals read as surface patches"
            );
        }
    }
    let moon_point = home
        .points
        .iter()
        .find(|point| point.moon.is_some())
        .expect("Moon point");
    assert!(matches!(moon_point.form, Form::Orbit { .. }));
    assert!(
        home.points
            .iter()
            .any(|point| matches!(point.form, Form::Ring { .. })),
        "the equator ring must draw"
    );
    assert!(
        home.points
            .iter()
            .any(|point| matches!(point.form, Form::Thread { .. })),
        "the spin axis must draw"
    );
    // A sibling off the path is procedural, never the Earth-Moon pair.
    let mut sibling = chain.clone();
    let last = sibling.len() - 1;
    sibling[last] = sibling[last].wrapping_add(1);
    assert!(!is_home_planet_cell(DEMO_SEED, &sibling));
    let other = generate_cell(DEMO_SEED, &sibling);
    assert!(
        other
            .points
            .iter()
            .filter_map(|point| point.moon)
            .all(|moon| { moon.radius_km != MOON_RADIUS_KM || moon.orbit_km != MOON_ORBIT_KM }),
        "procedural neighbours never copy the catalog Moon"
    );
}

#[test]
fn home_l4_cell_holds_the_barred_milky_way_on_the_journey() {
    use crate::astro::MILKY_WAY_ORIENTATION;
    use crate::r#gen::GalaxyKind;
    let chain = autopilot_path(DEMO_SEED);
    assert!(chain.len() >= 4, "journey must reach L5");
    let home_chain = &chain[..3];
    assert!(is_home_l4_cell(DEMO_SEED, home_chain));
    assert!(!is_home_l4_cell(DEMO_SEED, &chain[..2]));
    let mut sibling = home_chain.to_vec();
    let last = sibling.len() - 1;
    sibling[last] = sibling[last].wrapping_add(1);
    assert!(!is_home_l4_cell(DEMO_SEED, &sibling));
    let home = generate_cell(DEMO_SEED, home_chain);
    let seed = path_seed(DEMO_SEED, home_chain);
    let candidates = autopilot_candidates(level(4), seed, &home.points);
    let pick = autopilot_marker(seed, &candidates).expect("home L4 pick");
    assert_eq!(pick, chain[3], "the journey opens the Milky Way");
    let milky_way = home.points[pick as usize];
    assert_eq!(milky_way.kind, MarkerKind::Portal);
    let shape = milky_way.galaxy.expect("L4 carries shape data");
    assert_eq!(shape.kind, GalaxyKind::Spiral);
    assert!(shape.barred, "the Milky Way is barred");
    assert_eq!(shape.orientation, MILKY_WAY_ORIENTATION);
    assert!(!shape.far_view, "the home galaxy is never an impostor");
}

#[test]
fn approach_fade_runs_from_half_the_open_angle_to_the_open() {
    assert_eq!(approach_fade(0.0), 0.0);
    assert_eq!(
        approach_fade(APPROACH_START),
        0.0,
        "starts at half the open angle"
    );
    assert_eq!(approach_fade(OPEN_ANGLE), 1.0, "full exactly at the open");
    assert_eq!(approach_fade(OPEN_ANGLE * 2.0), 1.0);
    assert_eq!(approach_fade(f64::NAN), 0.0);
    let mid = approach_fade((APPROACH_START + OPEN_ANGLE) * 0.5);
    assert!((mid - 0.5).abs() < 1e-12, "halfway is half: {mid}");
    // Both ways: bigger means stronger, smaller means weaker.
    let mut last = 0.0;
    for step in 0..=100 {
        let angular = APPROACH_START + (OPEN_ANGLE - APPROACH_START) * f64::from(step) / 100.0;
        let fade = approach_fade(angular);
        assert!(fade >= last, "never weaker as it grows");
        last = fade;
    }
}
