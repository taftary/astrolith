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

/// Distance from `record` at which a fresh climb first selects `stage`
/// and holds it, the middle of that range on a log scale; `None` when
/// the part never shows that stage from outside.
fn distance_for(record: &PartRecord, stage: Stage) -> Option<f64> {
    let spec = record.spec();
    let extent = record.extent.max(1e-12);
    let mut hits = Vec::new();
    for step in 0..=2_000 {
        let t = f64::from(step) / 2_000.0;
        let distance = extent * 1.0e5_f64.powf(1.0 - t) * 1.02_f64.powf(t);
        let got = select_stage(
            &spec,
            apparent_angle(extent, distance),
            Stage::Hidden,
            false,
        );
        if got == stage {
            hits.push(distance);
        }
    }
    let (first, last) = (hits.first()?, hits.last()?);
    Some((first * last).sqrt())
}

/// Writes the per-family real-window evidence shot file (#1128 AC15):
/// for every family registered on the demo journey, one shot per stage
/// it meets from outside (far landmark, glyph, outline, composite or
/// preview), one from inside or near, and face-on, oblique and edge-on
/// shots at its composite distance when it is flat. Run with
/// `SHOTS_OUT=<file> cargo test -p universe-core --lib write_family_evidence -- --ignored`.
#[test]
#[ignore = "writes the evidence shot file; run by hand for #1128 evidence"]
fn write_family_evidence_shots() {
    use crate::home::autopilot_path;
    use crate::nav::{DEMO_SEED, MarkerIndex, Universe};
    use crate::nest::generate_cell;
    let Ok(out_path) = std::env::var("SHOTS_OUT") else {
        return;
    };
    let path = autopilot_path(DEMO_SEED);
    let mut universe = Universe::new(DEMO_SEED);
    let mut text = String::new();
    for line in [
        "# #1128 AC15 per-family evidence, generated by write_family_evidence_shots.",
        "# name level where look developer; one part per family, on the demo",
        "# journey or (via=) in the first sibling cell that has it.",
    ] {
        text.push_str(line);
        text.push('\n');
    }
    let mut seen = std::collections::BTreeSet::new();
    for depth in 0..=path.len().min(9) {
        if depth > 0 {
            let marker = path[depth - 1];
            assert!(universe.open(MarkerIndex(marker)), "journey opens");
        }
        let level = universe.level();
        let frame = (level.get() == 10)
            .then(|| crate::nav::planet_frame(&universe))
            .flatten();
        emit_family_shots(
            &mut text,
            &mut seen,
            level,
            &universe.open,
            frame.as_ref(),
            None,
        );
    }
    // Families the journey's cells lack: the first sibling cell (one level
    // up, another portal) that has them.
    for depth in 2..=9_usize {
        let Some(parent) = path.get(..depth - 2) else {
            break;
        };
        let above = generate_cell(DEMO_SEED, parent);
        for (marker, point) in above.with_ids().take(64) {
            if point.kind != MarkerKind::Portal {
                continue;
            }
            let mut chain = parent.to_vec();
            chain.push(marker);
            let level = crate::nest::path_level(&chain);
            let cell = generate_cell(DEMO_SEED, &chain);
            emit_family_shots(&mut text, &mut seen, level, &cell, None, Some(marker));
        }
    }
    std::fs::write(out_path, text).expect("write the shot file");
}

/// Appends the evidence shots of every family in `cell` not yet `seen`.
fn emit_family_shots(
    text: &mut String,
    seen: &mut std::collections::BTreeSet<SemanticType>,
    level: Level,
    cell: &Generated,
    frame: Option<&crate::planet_frame::PlanetFrame>,
    via: Option<u32>,
) {
    use std::fmt::Write as _;
    let mut records = Vec::new();
    classify_cell(level, cell, host_of(level), &mut records);
    let fixture = via.map_or_else(String::new, |marker| format!(" via={marker}"));
    // Largest part of each family, so it is readable on screen.
    let mut best: BTreeMap<SemanticType, PartRecord> = BTreeMap::new();
    for record in records {
        let slot = best.entry(record.id.kind).or_insert(record);
        if record.extent > slot.extent {
            *slot = record;
        }
    }
    for (kind, record) in best {
        if record.extent <= 0.0 || !seen.insert(kind) {
            continue;
        }
        let place = |dir: [f64; 3], distance: f64| -> String {
            let eye = [
                record.anchor[0] + dir[0] * distance,
                record.anchor[1] + dir[1] * distance,
                record.anchor[2] + dir[2] * distance,
            ];
            let look = [-dir[0], -dir[1], -dir[2]];
            match frame {
                Some(frame) => {
                    let at = frame.from_cell(eye).0;
                    let ahead = frame
                        .from_cell([eye[0] + look[0], eye[1] + look[1], eye[2] + look[2]])
                        .0;
                    format!(
                        "x={:.6e} y={:.6e} z={:.6e} dir={:.6},{:.6},{:.6}",
                        at[0],
                        at[1],
                        at[2],
                        ahead[0] - at[0],
                        ahead[1] - at[1],
                        ahead[2] - at[2]
                    )
                }
                None => format!(
                    "x={:.6e} y={:.6e} z={:.6e} dir={:.6},{:.6},{:.6}",
                    eye[0], eye[1], eye[2], look[0], look[1], look[2]
                ),
            }
        };
        let normal = record.orientation.map(|n| {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            [n[0] / len, n[1] / len, n[2] / len]
        });
        // A view tilted 40° off the normal (or a fixed diagonal).
        let view = normal.map_or([0.36, 0.48, 0.8], |n| {
            let side = if n[0].abs() < 0.9 {
                [0.0, -n[2], n[1]]
            } else {
                [-n[2], 0.0, n[0]]
            };
            let s = (side[0] * side[0] + side[1] * side[1] + side[2] * side[2]).sqrt();
            [
                n[0] * 0.77 + side[0] / s * 0.64,
                n[1] * 0.77 + side[1] / s * 0.64,
                n[2] * 0.77 + side[2] / s * 0.64,
            ]
        });
        let token = kind.token();
        let l = level.get();
        for (stage, tag) in [
            (Stage::Landmark, "1-landmark"),
            (Stage::Glyph, "2-glyph"),
            (Stage::Outline, "3-outline"),
            (Stage::Composite, "4-composite"),
            (Stage::Resolved, "5-preview"),
        ] {
            if let Some(distance) = distance_for(&record, stage) {
                let _ = writeln!(
                    text,
                    "{token}-{tag} L{l} {} developer{fixture}",
                    place(view, distance)
                );
            }
        }
        // Near or inside: half the extent from the anchor.
        let _ = writeln!(
            text,
            "{token}-6-near L{l} {} developer{fixture}",
            place(view, record.extent * 0.5)
        );
        if let (Some(n), Some(distance)) = (normal, distance_for(&record, Stage::Composite))
            && kind.spec().view == ViewRule::FaceOn
        {
            let side = if n[0].abs() < 0.9 {
                [0.0, -n[2], n[1]]
            } else {
                [-n[2], 0.0, n[0]]
            };
            let s = (side[0] * side[0] + side[1] * side[1] + side[2] * side[2]).sqrt();
            let side = [side[0] / s, side[1] / s, side[2] / s];
            let edge = [
                side[0] + n[0] * 0.01,
                side[1] + n[1] * 0.01,
                side[2] + n[2] * 0.01,
            ];
            let _ = writeln!(
                text,
                "{token}-dir-face L{l} {} developer{fixture}",
                place(n, distance)
            );
            let _ = writeln!(
                text,
                "{token}-dir-oblique L{l} {} developer{fixture}",
                place(view, distance)
            );
            let _ = writeln!(
                text,
                "{token}-dir-edge L{l} {} developer{fixture}",
                place(edge, distance)
            );
        }
    }
}
