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
