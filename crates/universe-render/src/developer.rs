//! The developer marker layer: semantic parts on top of today's view (#1128).
//!
//! Spec v3 (option B): today's dots and shapes, navigation and physical
//! content stay as they are, drawn by `draw`, `stars` and `planet`; this
//! layer only adds developer parts on top. [`DeveloperLayer`] holds the
//! effective choices for this build (release always off, AC2), the open
//! cell's semantic [`Registry`], the parts the filters let through, and
//! each part's stage and weight this frame (decided in core,
//! `semantic_display`). `developer_draw` draws them and `developer_labels`
//! names them. Navigation never reads this resource (AC8): hover, target,
//! portal rings and navigation labels are drawn and picked without it.
//! While the layer is off, nothing is classified at all.

use bevy::prelude::*;
use universe_core::clouds_l4::host_of;
use universe_core::semantic::{HostShape, PartRecord, Registry};
use universe_core::semantic_display::{
    Candidate, Stage, apparent_angle, apply_budgets, is_inside, select_stage, stage_weight,
    view_weight,
};
use universe_core::settings::DeveloperMarkers;

use crate::{SettingsStore, Universe};

/// One part planned for drawing this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Planned {
    /// Place in the registry.
    pub slot: usize,
    /// Its stage.
    pub stage: Stage,
    /// Crossfade times view weight, `0..=1`.
    pub weight: f32,
    /// Its apparent angular radius.
    pub angle: f64,
}

/// Effective developer choices, the open cell's parts and those let through.
#[derive(Resource, Default)]
pub(crate) struct DeveloperLayer {
    effective: DeveloperMarkers,
    registry: Registry,
    /// Places in the registry of the parts the filters let through, reused
    /// every frame (`E-HOT-NOALLOC`).
    shown: Vec<usize>,
    /// Last frame's stage per registry slot (hysteresis).
    stages: Vec<Stage>,
    /// Budget scratch, reused.
    candidates: Vec<Candidate>,
    kept: Vec<usize>,
    /// Parts to draw this frame, after stages, weights and budgets.
    planned: Vec<Planned>,
}

impl DeveloperLayer {
    /// Effective choices this frame (release forces the master off).
    pub(crate) const fn effective(&self) -> DeveloperMarkers {
        self.effective
    }

    /// Parts the master switch and filters let through this frame.
    #[cfg_attr(not(test), expect(dead_code, reason = "read by tests"))]
    pub(crate) fn shown(&self) -> impl Iterator<Item = &PartRecord> + '_ {
        let records = self.registry.records();
        self.shown.iter().filter_map(move |&slot| records.get(slot))
    }

    /// Number of parts let through this frame.
    #[cfg_attr(not(test), expect(dead_code, reason = "read by tests"))]
    pub(crate) fn shown_count(&self) -> usize {
        self.shown.len()
    }

    /// Parts to draw this frame with their records.
    pub(crate) fn planned(&self) -> impl Iterator<Item = (&PartRecord, Planned)> + '_ {
        let records = self.registry.records();
        self.planned
            .iter()
            .filter_map(move |plan| records.get(plan.slot).map(|record| (record, *plan)))
    }

    /// Decides each shown part's stage and weight from the eye (open-cell
    /// units) and the near-ground weight, then applies the budgets.
    pub(crate) fn plan(&mut self, eye: [f64; 3], ground: f64, population_cap: usize) {
        self.planned.clear();
        self.candidates.clear();
        let records = self.registry.records();
        if self.stages.len() != records.len() {
            self.stages.clear();
            self.stages.resize(records.len(), Stage::Hidden);
        }
        for &slot in &self.shown {
            let (Some(record), Some(previous)) = (records.get(slot), self.stages.get_mut(slot))
            else {
                continue;
            };
            let spec = record.spec();
            let offset = [
                eye[0] - record.anchor[0],
                eye[1] - record.anchor[1],
                eye[2] - record.anchor[2],
            ];
            let distance =
                (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
            let angle = apparent_angle(record.extent, distance);
            let stage = select_stage(&spec, angle, *previous, is_inside(record, eye));
            *previous = stage;
            // Today's view already draws the dot and glyph of baseline
            // parts (Spec v3 B): the layer adds from the outline up.
            if stage == Stage::Hidden || (spec.baseline && stage < Stage::Outline) {
                continue;
            }
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: render-domain narrowing of a weight, intended"
            )]
            let weight =
                (stage_weight(&spec, stage, angle) * view_weight(record, eye, ground)) as f32;
            if weight <= 0.0 {
                continue;
            }
            self.candidates.push(Candidate {
                slot,
                group: spec.budget,
                importance: spec.importance,
                angle,
                distance,
                pinned: false,
            });
            self.planned.push(Planned {
                slot,
                stage,
                weight,
                angle,
            });
        }
        apply_budgets(&mut self.candidates, population_cap, &mut self.kept);
        self.kept.sort_unstable();
        let kept = &self.kept;
        self.planned
            .retain(|plan| kept.binary_search(&plan.slot).is_ok());
    }
}

/// Host galaxy shape for an L4 cell, read once per rebuild.
fn host_shape(universe: &universe_core::nav::Universe) -> Option<HostShape> {
    if universe.level().get() != 4 {
        return None;
    }
    host_of(universe.root.0, &universe.path.indices()).map(|host| HostShape {
        normal: host.galaxy.orientation,
        barred: host.galaxy.barred,
    })
}

/// Applies the effective choices and refreshes the parts let through.
///
/// The registry rebuilds only when the open cell or its content changes.
pub(crate) fn sync_developer_layer(
    store: Res<SettingsStore>,
    universe: Res<Universe>,
    origin: Res<crate::style::RenderOrigin>,
    mut layer: ResMut<DeveloperLayer>,
) {
    let effective = store.0.developer_markers.effective();
    let layer = &mut *layer;
    layer.effective = effective;
    layer.shown.clear();
    layer.planned.clear();
    if !effective.enabled {
        return;
    }
    let universe = &universe.0;
    let revision = (universe.open_seed(), universe.content_revision());
    let level = universe.level();
    if layer.registry.revision() != Some(revision) {
        let host = host_shape(universe);
        layer
            .registry
            .rebuild(level, &universe.open, host, revision);
        layer.stages.clear();
    }
    for (slot, record) in layer.registry.records().iter().enumerate() {
        if effective.allows(record.spec().class) {
            layer.shown.push(slot);
        }
    }
    // #1056: paths and references recede near the ground.
    let ground = universe_core::labels::orbit_path_weight(crate::hud::camera_height_m(
        universe,
        origin.camera,
    ));
    layer.plan(
        universe.path.offset(),
        ground,
        store.0.marker_detail.budget(),
    );
}

/// Registers the developer layer, its drawing and its labels.
pub(crate) struct DeveloperPlugin;

impl Plugin for DeveloperPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DeveloperLayer>()
            .add_systems(Startup, crate::developer_labels::spawn_developer_labels)
            .add_systems(
                Update,
                (
                    sync_developer_layer,
                    crate::developer_draw::draw_developer_parts,
                    // After the navigation labels move, so their boxes
                    // block this frame's developer labels.
                    crate::developer_labels::sync_developer_labels
                        .after(crate::labels::sync_labels),
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
#[path = "developer_tests.rs"]
mod tests;
