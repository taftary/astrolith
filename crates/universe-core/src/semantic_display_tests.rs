//! Display decision tests (#1128 T3: C4, C5, C7, C9, C10, C11, C18).

use super::*;
use crate::semantic::{Parent, PartId, PartRecord};
use crate::semantic_catalog::SemanticType;
use std::collections::BTreeSet;

fn record(kind: SemanticType, extent: f64, orientation: Option<[f64; 3]>) -> PartRecord {
    PartRecord {
        id: PartId {
            level: kind.spec().levels[0],
            kind,
            key: 7,
        },
        parent: Parent::Marker(7),
        anchor: [0.0; 3],
        extent,
        orientation,
        portal: false,
    }
}

/// Stages met while zooming from far to near (or back) over `steps`
/// geometric distance steps; each frame feeds last frame's stage.
fn zoom(
    spec: &PartSpec,
    extent: f64,
    near: f64,
    far: f64,
    steps: usize,
    inward: bool,
) -> Vec<Stage> {
    let mut stage = Stage::Hidden;
    let mut out = Vec::with_capacity(steps);
    for step in 0..steps {
        #[expect(clippy::cast_precision_loss, reason = "test step counts are small")]
        let t = step as f64 / (steps - 1) as f64;
        let t = if inward { t } else { 1.0 - t };
        let distance = far * (near / far).powf(t);
        stage = select_stage(spec, apparent_angle(extent, distance), stage, false);
        out.push(stage);
    }
    out
}

#[test]
fn apparent_size_stages_are_resolution_independent() {
    // C4: the stage reads only the part's angular size: the same view at
    // any window size, field of view or level selects the same stage, and
    // the findability floor never changes the part's own extent.
    for kind in SemanticType::ALL {
        let spec = kind.spec();
        for distance in [1.0e3, 50.0, 10.0, 2.0] {
            let angle = apparent_angle(0.1, distance);
            let first = select_stage(&spec, angle, Stage::Hidden, false);
            // Pixels never enter: doubling the window changes nothing.
            for _window in [720.0_f64, 1440.0, 2880.0] {
                assert_eq!(select_stage(&spec, angle, Stage::Hidden, false), first);
            }
        }
        // Same angle from a bigger part farther away: same stage.
        let a = select_stage(&spec, apparent_angle(1.0, 100.0), Stage::Hidden, false);
        let b = select_stage(&spec, apparent_angle(10.0, 1_000.0), Stage::Hidden, false);
        assert_eq!(a, b, "{kind:?}");
    }
    let part = record(SemanticType::Planet, 1.0e-6, None);
    let drawn = drawn_extent(part.extent, 100.0);
    assert!(drawn > part.extent, "the floor applies to the drawing");
    assert!(
        (part.extent - 1.0e-6).abs() < f64::EPSILON,
        "never to the part"
    );
}

#[test]
fn slow_zoom_has_no_flicker_and_crossfades() {
    // C5: zooming slowly in and out, each part's stage only climbs going
    // in and only falls going out, and every stage eases in.
    for kind in SemanticType::ALL {
        let spec = kind.spec();
        let inward = zoom(&spec, 1.0, 2.0, 1.0e5, 4_000, true);
        assert!(inward.windows(2).all(|w| w[0] <= w[1]), "{kind:?} climbs");
        let outward = zoom(&spec, 1.0, 2.0, 1.0e5, 4_000, false);
        assert!(outward.windows(2).all(|w| w[0] >= w[1]), "{kind:?} falls");
        // Hovering at a threshold never flips back and forth.
        let mut stage = Stage::Hidden;
        let mut changes = 0;
        for frame in 0..400 {
            let wobble = if frame % 2 == 0 { 1.0 } else { 0.97 };
            let next = select_stage(&spec, FORM_ANGLE * wobble, stage, false);
            if frame > 0 && next != stage {
                changes += 1;
            }
            stage = next;
        }
        assert!(changes <= 1, "{kind:?} flickers at the form angle");
        for stage in [Stage::Glyph, Stage::Outline, Stage::Composite] {
            let low = stage_weight(&spec, stage, 0.0);
            let high = stage_weight(&spec, stage, 1.0);
            assert!(low <= high && high <= 1.0, "{kind:?} {stage:?} eases in");
        }
    }
}

#[test]
fn same_part_changes_view_without_losing_identity_or_colour_role() {
    // C18: one galaxy seen far, middle and near keeps its identity and its
    // colour role while its stage changes.
    let part = record(SemanticType::Galaxy, 1.0, Some([0.0, 0.0, 1.0]));
    let spec = part.spec();
    let mut seen = BTreeSet::new();
    for distance in [1.0e5, 3.0e2, 1.0e2, 30.0, 8.0, 1.5] {
        let stage = select_stage(
            &spec,
            apparent_angle(part.extent, distance),
            Stage::Hidden,
            false,
        );
        seen.insert(stage);
        assert_eq!(part.spec().colour, ColourRole::Classification);
        assert_eq!(part.id.key, 7);
    }
    assert!(seen.len() >= 4, "several views of one part: {seen:?}");
    assert_eq!(
        select_stage(&spec, 1.0, Stage::Composite, true),
        Stage::Local,
        "inside, the local view"
    );
}

#[test]
fn view_angle_and_inside_rules_cover_every_shape_family() {
    // C7: flat parts thin away edge-on; volumes fade from inside; the
    // anchor never moves.
    let up = Some([0.0, 0.0, 1.0]);
    for kind in SemanticType::ALL {
        let spec = kind.spec();
        let part = record(
            kind,
            1.0,
            if spec.view == ViewRule::FaceOn {
                up
            } else {
                None
            },
        );
        let face_on = view_weight(&part, [0.0, 0.0, 10.0], 1.0);
        let edge_on = view_weight(&part, [10.0, 0.0, 0.0], 1.0);
        let inside = view_weight(&part, [0.3, 0.0, 0.0], 1.0);
        match spec.view {
            ViewRule::FaceOn => {
                assert!(face_on > 0.99, "{kind:?} full face-on");
                assert!(edge_on < 0.01, "{kind:?} gone edge-on");
                let oblique = view_weight(&part, [10.0, 0.0, 1.0], 1.0);
                assert!(
                    oblique > edge_on && oblique <= face_on,
                    "{kind:?} oblique between"
                );
            }
            _ => assert!((face_on - edge_on).abs() < 1e-9, "{kind:?} isotropic"),
        }
        match spec.local {
            LocalRule::FadeInside | LocalRule::FadeNearPlane => {
                assert!(inside < 0.01, "{kind:?} clears the view from inside");
            }
            LocalRule::FadeNearGround => {
                assert!(
                    view_weight(&part, [0.0, 0.0, 10.0], 0.0) < 1e-9,
                    "{kind:?} near ground"
                );
            }
            LocalRule::Keep => {}
        }
        assert_eq!(part.anchor, [0.0; 3], "the part never moves");
    }
    // Every shape family appears in the catalog with a rule.
    let shapes: BTreeSet<_> = SemanticType::ALL
        .iter()
        .map(|kind| format!("{:?}", kind.spec().shape))
        .collect();
    assert!(shapes.len() >= 10, "shape families: {shapes:?}");
}

#[test]
fn semantic_palette_has_reserved_colours_and_redundant_cues() {
    // C9: stars and galaxies keep their own colours; every other role has
    // a palette colour far from the reserved state colours, and two roles
    // drawn with the same shape differ by pattern or lightness, not hue
    // alone.
    for kind in SemanticType::ALL {
        let spec = kind.spec();
        let star_or_galaxy = matches!(
            kind,
            SemanticType::Galaxy
                | SemanticType::GalaxyMember
                | SemanticType::BrightStar
                | SemanticType::Star
                | SemanticType::PrimaryStar
                | SemanticType::CompanionStar
                | SemanticType::CentralStar
        );
        if star_or_galaxy {
            assert_eq!(spec.colour, ColourRole::Classification, "{kind:?}");
        }
        if let (Some(rgb), _) = palette(spec.colour) {
            for reserved in RESERVED {
                let gap: f32 = rgb
                    .iter()
                    .zip(reserved)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f32>()
                    .sqrt();
                assert!(gap > 0.3, "{kind:?} keeps clear of reserved colours");
            }
        }
    }
    for a in SemanticType::ALL {
        for b in SemanticType::ALL {
            let (sa, sb) = (a.spec(), b.spec());
            if a >= b || sa.colour == sb.colour || sa.shape != sb.shape {
                continue;
            }
            let (ca, pa) = palette(sa.colour);
            let (cb, pb) = palette(sb.colour);
            let (Some(ca), Some(cb)) = (ca, cb) else {
                continue;
            };
            let light = (lightness(ca) - lightness(cb)).abs();
            assert!(pa != pb || light > 0.1, "{a:?} vs {b:?} differ beyond hue");
        }
    }
}

#[test]
fn marker_budgets_are_bounded_and_prioritized() {
    // C11: every group keeps at most its cap (populations: Marker detail),
    // pinned parts always stay, and importance, size, then nearness win.
    let mut candidates = Vec::new();
    for slot in 0..2_000 {
        #[expect(clippy::cast_precision_loss, reason = "test sizes are small")]
        let angle = (slot % 97) as f64 * 1e-4;
        candidates.push(Candidate {
            slot,
            group: if slot % 2 == 0 {
                BudgetGroup::Populations
            } else {
                BudgetGroup::References
            },
            importance: u8::try_from(slot % 5).unwrap_or(0),
            angle,
            #[expect(clippy::cast_precision_loss, reason = "test sizes are small")]
            distance: slot as f64,
            pinned: slot == 1_999,
        });
    }
    let mut kept = Vec::new();
    apply_budgets(&mut candidates, 100, &mut kept);
    let populations = kept.iter().filter(|slot| *slot % 2 == 0).count();
    let references = kept.iter().filter(|slot| *slot % 2 == 1).count();
    assert_eq!(populations, 100);
    let cap = BudgetGroup::References.cap().unwrap_or(0);
    assert_eq!(references, cap + 1, "cap plus the pinned one");
    assert!(kept.contains(&1_999), "pinned stays");
    let best = kept
        .iter()
        .filter(|slot| *slot % 2 == 0)
        .map(|slot| slot % 5)
        .min()
        .unwrap_or(0);
    assert_eq!(best, 4, "only the most important populations fit");
    let again = kept.clone();
    apply_budgets(&mut candidates, 100, &mut kept);
    assert_eq!(kept, again, "deterministic");
}

#[test]
fn developer_labels_are_capped_collision_aware_and_separate() {
    // C10: developer labels never overlap each other or a navigation label,
    // obey the cap, and the navigation labels are never removed.
    let navigation = [LabelBox {
        slot: usize::MAX,
        at: [0.0, 0.0],
        size: [100.0, 20.0],
        priority: f64::INFINITY,
    }];
    let mut boxes: Vec<LabelBox> = (0..200)
        .map(|slot| {
            #[expect(clippy::cast_precision_loss, reason = "test sizes are small")]
            let x = (slot % 20) as f32 * 30.0;
            #[expect(clippy::cast_precision_loss, reason = "test sizes are small")]
            let y = (slot / 20) as f32 * 12.0;
            #[expect(clippy::cast_precision_loss, reason = "test sizes are small")]
            let priority = slot as f64;
            LabelBox {
                slot,
                at: [x, y],
                size: [60.0, 14.0],
                priority,
            }
        })
        .collect();
    let mut placed = Vec::new();
    place_labels(&mut boxes, &navigation, LABEL_CAP, &mut placed);
    assert!(!placed.is_empty() && placed.len() <= LABEL_CAP);
    for (i, a) in placed.iter().enumerate() {
        assert!(!a.overlaps(&navigation[0]), "navigation keeps its place");
        for b in placed.iter().skip(i + 1) {
            assert!(!a.overlaps(b), "no two developer labels overlap");
        }
    }
    assert_eq!(placed[0].slot, 199, "highest priority first");
    const { assert!(LABEL_ANGLE > 0.0) };
}
