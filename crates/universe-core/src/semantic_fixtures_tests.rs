//! Deterministic far-to-near fixtures per catalog family (#1128 T4-T6).
//!
//! For every catalog family registered in the demo journey's cells, one
//! real part is approached slowly along a fixed line, face-on when it is
//! flat, from far outside to inside its extent; the stages met, their
//! order and weights are checked against the approved ladder.

use super::tests::{host_of, journey_cells};
use super::*;
use crate::semantic_display::{Stage, apparent_angle, is_inside, select_stage, view_weight};
use std::collections::BTreeMap;

/// One registered part per family from the journey cells, first found.
fn one_per_family() -> BTreeMap<SemanticType, PartRecord> {
    let mut out = BTreeMap::new();
    for (level, cell) in journey_cells() {
        let mut records = Vec::new();
        classify_cell(level, &cell, host_of(level), &mut records);
        for record in records {
            if record.extent > 0.0 {
                out.entry(record.id.kind).or_insert(record);
            }
        }
    }
    out
}

/// Stages and weights met approaching `record` from 10^6 extents to a
/// tenth of its extent, along its normal when it has one.
fn approach(record: &PartRecord) -> Vec<(Stage, f64)> {
    let dir = record.orientation.unwrap_or([0.36, 0.48, 0.8]);
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    let dir = [dir[0] / len, dir[1] / len, dir[2] / len];
    let spec = record.spec();
    let mut stage = Stage::Hidden;
    let mut out = Vec::new();
    let steps = 3_000;
    for step in 0..=steps {
        let t = f64::from(step) / f64::from(steps);
        let distance = record.extent * 1.0e6_f64.powf(1.0 - t) * 10.0_f64.powf(-t);
        let eye = [
            record.anchor[0] + dir[0] * distance,
            record.anchor[1] + dir[1] * distance,
            record.anchor[2] + dir[2] * distance,
        ];
        let angle = apparent_angle(record.extent, distance);
        stage = select_stage(&spec, angle, stage, is_inside(record, eye));
        out.push((stage, view_weight(record, eye, 1.0)));
    }
    out
}

/// Checks the ladder for every family admitted at one of `levels`.
fn check_levels(levels: std::ops::RangeInclusive<u8>) {
    let parts = one_per_family();
    let mut checked = 0;
    for kind in SemanticType::ALL {
        let spec = kind.spec();
        if !spec.levels.iter().any(|level| levels.contains(level)) {
            continue;
        }
        let record = parts
            .get(&kind)
            .unwrap_or_else(|| panic!("{kind:?} has a registered part"));
        let path = approach(record);
        let stages: Vec<Stage> = path.iter().map(|(stage, _)| *stage).collect();
        // Far: a landmark or nothing; never more.
        assert!(
            stages[0] <= Stage::Landmark,
            "{kind:?} far: {:?}",
            stages[0]
        );
        // The ladder climbs in order, then the local view takes over
        // inside (parts with a local rule) or the top stage holds.
        let outside: Vec<Stage> = stages
            .iter()
            .copied()
            .take_while(|stage| *stage != Stage::Local)
            .collect();
        assert!(outside.windows(2).all(|w| w[0] <= w[1]), "{kind:?} climbs");
        // One stage at a time: never everything at once.
        assert!(
            outside
                .windows(2)
                .all(|w| (w[1] as u8).saturating_sub(w[0] as u8) <= 1),
            "{kind:?} climbs one stage at a time"
        );
        for wanted in [
            Stage::Landmark,
            Stage::Glyph,
            Stage::Outline,
            Stage::Composite,
        ] {
            assert!(outside.contains(&wanted), "{kind:?} meets {wanted:?}");
        }
        let near = *stages.last().unwrap_or(&Stage::Hidden);
        if spec.local == LocalRule::Keep {
            assert!(near >= Stage::Composite, "{kind:?} near: {near:?}");
        } else {
            assert_eq!(near, Stage::Local, "{kind:?} near is local");
        }
        // Previews only where the dimension has them (L1-L7 portals).
        let resolved = stages.contains(&Stage::Resolved);
        if resolved {
            assert!(
                spec.levels.iter().all(|level| *level <= 7),
                "{kind:?} preview"
            );
        }
        // Seen face-on from outside, the part draws.
        let composite_weight = path
            .iter()
            .find(|(stage, _)| *stage == Stage::Composite)
            .map_or(0.0, |(_, weight)| *weight);
        assert!(composite_weight > 0.5, "{kind:?} visible at composite");
        checked += 1;
    }
    assert!(checked > 0);
}

#[test]
fn l1_l3_far_glyph_structural_composite_near_fixtures() {
    // T4: regions, boundary, graticule, cosmic web, clusters/groups,
    // galaxies and their contexts.
    check_levels(1..=3);
    // Shared L1-L3 structures carry one identity per cell.
    for (level, cell) in journey_cells() {
        if level.get() > 3 {
            continue;
        }
        let mut records = Vec::new();
        classify_cell(level, &cell, host_of(level), &mut records);
        for kind in [
            SemanticType::UniverseBoundary,
            SemanticType::Graticule,
            SemanticType::RegionContext,
            SemanticType::ClusterContext,
        ] {
            assert!(records.iter().filter(|r| r.id.kind == kind).count() <= 1);
        }
        // Walls keep their true sheet orientation from the density field.
        for record in records.iter().filter(|r| r.id.kind == SemanticType::Wall) {
            let Parent::Marker(id) = record.parent else {
                continue;
            };
            if let Some(Form::Sheet { normal }) = cell.point_by_id(id).map(|point| point.form) {
                assert_eq!(record.orientation, Some(normal));
            }
        }
    }
}

/// Face-on, oblique and edge-on weights and the inside weight of `record`.
fn direction_and_inside(record: &PartRecord) -> (f64, f64, f64, f64) {
    let n = record.orientation.unwrap_or([0.0, 0.0, 1.0]);
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    let n = [n[0] / len, n[1] / len, n[2] / len];
    // Any direction in the plane.
    let side = if n[0].abs() < 0.9 {
        [0.0, -n[2], n[1]]
    } else {
        [-n[2], 0.0, n[0]]
    };
    let side_len = (side[0] * side[0] + side[1] * side[1] + side[2] * side[2]).sqrt();
    let side = [side[0] / side_len, side[1] / side_len, side[2] / side_len];
    let far = record.extent * 20.0;
    let at = |dir: [f64; 3], distance: f64| {
        [
            record.anchor[0] + dir[0] * distance,
            record.anchor[1] + dir[1] * distance,
            record.anchor[2] + dir[2] * distance,
        ]
    };
    let oblique = [
        (n[0] + side[0] * 3.0) / 10f64.sqrt(),
        (n[1] + side[1] * 3.0) / 10f64.sqrt(),
        (n[2] + side[2] * 3.0) / 10f64.sqrt(),
    ];
    (
        view_weight(record, at(n, far), 1.0),
        view_weight(record, at(oblique, far), 1.0),
        view_weight(record, at(side, far), 1.0),
        view_weight(record, record.anchor, 1.0),
    )
}

#[test]
fn l4_l7_far_middle_near_direction_inside_fixtures() {
    // T5: clouds and the shared host disk, arms and bar; stars and their
    // host context; primaries, companions and arcs; the central star,
    // belts, Oort reservoir and system boundary.
    check_levels(4..=7);
    let parts = one_per_family();
    for (kind, record) in &parts {
        let spec = kind.spec();
        if !spec.levels.iter().any(|level| (4..=7).contains(level)) {
            continue;
        }
        let (face, oblique, edge, inside) = direction_and_inside(record);
        if spec.view == ViewRule::FaceOn && record.orientation.is_some() {
            assert!(face > 0.99, "{kind:?} face-on");
            assert!(oblique > edge && oblique < face + 1e-9, "{kind:?} oblique");
            assert!(edge < 0.01, "{kind:?} edge-on");
        }
        if matches!(spec.local, LocalRule::FadeInside | LocalRule::FadeNearPlane) {
            assert!(inside < 0.01, "{kind:?} clears from inside");
        }
        // Stars keep their classification colours.
        if matches!(
            kind,
            SemanticType::BrightStar
                | SemanticType::Star
                | SemanticType::PrimaryStar
                | SemanticType::CompanionStar
                | SemanticType::CentralStar
        ) {
            assert_eq!(spec.colour, ColourRole::Classification);
        }
    }
    // The host disk, arms and bar share the host galaxy's plane, and its
    // cloud boundaries lie in that same plane (the cell's own host).
    for (level, cell) in journey_cells() {
        if level.get() != 4 {
            continue;
        }
        let mut records = Vec::new();
        classify_cell(level, &cell, None, &mut records);
        let disk = records
            .iter()
            .find(|r| r.id.kind == SemanticType::HostDisk)
            .and_then(|r| r.orientation)
            .expect("host disk");
        for record in records.iter().filter(|r| {
            matches!(
                r.id.kind,
                SemanticType::HostArms | SemanticType::HostBar | SemanticType::CloudBoundary
            )
        }) {
            let n = record.orientation.expect("oriented");
            let d = n[0] * disk[0] + n[1] * disk[1] + n[2] * disk[2];
            assert!(d.abs() > 0.999, "{:?} in the host plane", record.id.kind);
        }
    }
}

#[test]
fn l8_l10_far_to_ground_fixtures() {
    // T6: the star, planets, belts, heliopause and true orbits; moons,
    // their paths and the equator and spin references. L9 adds no family
    // and L11-L14 (height labels over L10) register nothing of their own.
    check_levels(8..=10);
    for kind in SemanticType::ALL {
        assert!(kind.spec().levels.iter().all(|level| *level <= 10));
    }
    // Belts at L8 are one shared band each, however many members.
    for (level, cell) in journey_cells() {
        if level.get() != 8 {
            continue;
        }
        let mut records = Vec::new();
        classify_cell(level, &cell, None, &mut records);
        for kind in [SemanticType::AsteroidBelt, SemanticType::KuiperBelt] {
            assert!(records.iter().filter(|r| r.id.kind == kind).count() <= 1);
        }
        // Orbit paths sit about the star and pass through their planet.
        for path in records
            .iter()
            .filter(|r| r.id.kind == SemanticType::OrbitPath)
        {
            let Parent::Marker(id) = path.parent else {
                continue;
            };
            let planet = cell.point_by_id(id).expect("owner");
            let r = (planet.position[0].powi(2)
                + planet.position[1].powi(2)
                + planet.position[2].powi(2))
            .sqrt();
            assert!((r - path.extent).abs() <= 1e-9 * r.max(1.0), "true orbit");
        }
    }
}

#[test]
fn near_field_recedes_scaffolding_and_never_previews_planets() {
    // C14: on the ground, every path and reference has receded (physical
    // terrain and navigation stay readable), and no planet, moon or system
    // body ever reaches the preview stage however close.
    let parts = one_per_family();
    for (kind, record) in &parts {
        let spec = kind.spec();
        if spec.local == LocalRule::FadeNearGround {
            let high = view_weight(record, [0.0, 0.0, record.extent * 50.0], 1.0);
            let ground = view_weight(record, [0.0, 0.0, record.extent * 50.0], 0.0);
            assert!(ground <= 0.0, "{kind:?} recedes near the ground");
            assert!(high >= ground);
        }
        if spec.levels.iter().any(|level| *level >= 8) {
            for angle in [0.1, 0.5, 1.0, std::f64::consts::FRAC_PI_2] {
                let stage = select_stage(&spec, angle, Stage::Composite, false);
                assert_ne!(stage, Stage::Resolved, "{kind:?} never previews");
            }
        }
    }
    for kind in [
        SemanticType::OrbitPath,
        SemanticType::MoonPath,
        SemanticType::Equator,
        SemanticType::SpinAxis,
    ] {
        assert_eq!(kind.spec().local, LocalRule::FadeNearGround, "{kind:?}");
    }
}
