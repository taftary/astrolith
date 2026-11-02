//! Semantic registry tests (#1128 T2, C3, C6, C19).

use super::*;
use crate::home::autopilot_path;
use crate::nav::DEMO_SEED;
use crate::nest::{generate_cell, path_level};
use std::collections::{BTreeMap, BTreeSet};

/// The home journey's cells L1 to L10 plus the first other portal's cell
/// at every level, so rare families (companions, voids) are met too.
fn journey_cells() -> Vec<(Level, Generated)> {
    let path = autopilot_path(DEMO_SEED);
    let mut cells = Vec::new();
    for depth in 0..=path.len().min(9) {
        let Some(chain) = path.get(..depth) else {
            break;
        };
        let cell = generate_cell(DEMO_SEED, chain);
        let level = path_level(chain);
        let sibling = cell
            .with_ids()
            .find(|(id, point)| point.kind == MarkerKind::Portal && path.get(depth) != Some(id))
            .map(|(id, _)| id);
        if let Some(other) = sibling
            && level.get() < 10
        {
            let mut chain = chain.to_vec();
            chain.push(other);
            cells.push((path_level(&chain), generate_cell(DEMO_SEED, &chain)));
        }
        cells.push((level, cell));
    }
    cells
}

fn host_of(level: Level) -> Option<HostShape> {
    (level.get() == 4).then(|| {
        let path = autopilot_path(DEMO_SEED);
        let chain = path.get(..3).unwrap_or(&[]);
        let host = crate::clouds_l4::host_of(DEMO_SEED, chain).expect("home host");
        HostShape {
            normal: host.galaxy.orientation,
            barred: host.galaxy.barred,
        }
    })
}

#[test]
fn semantic_registry_covers_catalog_and_shares_parts() {
    // C3/C19: every catalog entry carries complete metadata.
    for kind in SemanticType::ALL {
        let spec = kind.spec();
        assert!(!spec.levels.is_empty(), "{kind:?} has a context");
        assert!(
            spec.levels.iter().all(|level| (1..=10).contains(level)),
            "{kind:?} levels inside L1-L10"
        );
        assert!(
            spec.band.exit <= spec.band.enter && spec.band.exit >= 0.0,
            "{kind:?} exits at or below where it enters"
        );
        assert_eq!(
            spec.budget.cap().is_none(),
            spec.class == crate::settings_developer::DeveloperPart::Populations,
            "{kind:?}: only populations follow Marker detail"
        );
        assert!(!kind.token().is_empty());
    }
    let tokens: BTreeSet<_> = SemanticType::ALL.iter().map(|kind| kind.token()).collect();
    assert_eq!(tokens.len(), SemanticType::ALL.len(), "tokens are unique");

    let mut seen = BTreeSet::new();
    for (level, cell) in journey_cells() {
        let mut registry = Registry::default();
        assert!(registry.rebuild(level, &cell, host_of(level), (1, 1)));
        let first = registry.records().to_vec();
        assert!(
            !registry.rebuild(level, &cell, host_of(level), (1, 1)),
            "same content revision does not rebuild"
        );
        assert!(registry.rebuild(level, &cell, host_of(level), (1, 2)));
        assert_eq!(registry.records(), first.as_slice(), "identity is stable");
        // Each identity once (shared parts draw once).
        let ids: BTreeSet<_> = first.iter().map(|record| record.id).collect();
        assert_eq!(ids.len(), first.len(), "L{} identities unique", level.get());
        let markers: BTreeSet<u32> = cell.with_ids().map(|(id, _)| id).collect();
        for record in &first {
            assert!(
                record.spec().levels.contains(&level.get()),
                "{:?} admitted at L{}",
                record.id.kind,
                level.get()
            );
            assert!(record.extent.is_finite() && record.extent >= 0.0);
            if let Parent::Marker(id) = record.parent {
                assert!(markers.contains(&id), "parent marker exists");
            }
            if record.spec().shared {
                assert_eq!(
                    record.parent,
                    Parent::Cell,
                    "shared parts belong to the cell"
                );
            }
            seen.insert(record.id.kind);
        }
        // Every generated marker (ground samples aside) is described.
        for (_, point) in cell.with_ids() {
            assert!(
                point.surface.is_some() || is_registered(level, point),
                "L{} point {:?} is registered: {:?}",
                level.get(),
                point.form,
                point
            );
        }
        // Shared belts: many members, one band each.
        let mut counts: BTreeMap<SemanticType, usize> = BTreeMap::new();
        for record in &first {
            *counts.entry(record.id.kind).or_default() += 1;
        }
        let members = counts.get(&SemanticType::BeltMember).copied().unwrap_or(0);
        let bands: usize = [
            SemanticType::KuiperBelt,
            SemanticType::ScatteredDisc,
            SemanticType::AsteroidBelt,
        ]
        .iter()
        .map(|kind| counts.get(kind).copied().unwrap_or(0))
        .sum();
        assert!(bands <= 2, "L{}: at most two shared bands", level.get());
        if members > 2 {
            assert!(bands >= 1 && bands < members, "members share their band");
        }
    }
    let missing: Vec<_> = SemanticType::ALL
        .iter()
        .filter(|kind| !seen.contains(kind))
        .collect();
    assert!(
        missing.is_empty(),
        "catalog entries never registered: {missing:?}"
    );
}

#[test]
fn complete_l1_l10_catalog_is_registered() {
    // C6: each level registers its families and nothing from elsewhere;
    // L9 adds no family of its own.
    let mut by_level: BTreeMap<u8, BTreeSet<SemanticType>> = BTreeMap::new();
    for (level, cell) in journey_cells() {
        let mut out = Vec::new();
        classify_cell(level, &cell, host_of(level), &mut out);
        by_level
            .entry(level.get())
            .or_default()
            .extend(out.iter().map(|record| record.id.kind));
    }
    assert_eq!(
        by_level.keys().copied().collect::<Vec<_>>(),
        (1..=10).collect::<Vec<_>>()
    );
    for kind in SemanticType::ALL {
        let spec = kind.spec();
        let first = spec.levels.first().copied().unwrap_or(0);
        assert!(
            by_level
                .get(&first)
                .is_some_and(|kinds| kinds.contains(&kind)),
            "{kind:?} shows at L{first}"
        );
    }
    let l9 = by_level.get(&9).cloned().unwrap_or_default();
    for kind in l9 {
        assert!(
            kind.spec().levels.iter().any(|level| *level != 9),
            "{kind:?}: L9 adds no new family"
        );
    }
}
