//! The developer marker layer: semantic parts on top of today's view (#1128).
//!
//! Spec v3 (option B): today's dots and shapes, navigation and physical
//! content stay as they are, drawn by `draw`, `stars` and `planet`; this
//! layer only adds developer parts on top. [`DeveloperLayer`] holds the
//! effective choices for this build (release always off, AC2), the open
//! cell's semantic [`Registry`], and the parts the filters let through.
//! Navigation never reads this resource (AC8): hover, target, portal rings
//! and navigation labels are drawn and picked without it. While the layer
//! is off, nothing is classified at all.

use bevy::prelude::*;
use universe_core::clouds_l4::host_of;
use universe_core::semantic::{HostShape, PartRecord, Registry};
use universe_core::settings::DeveloperMarkers;

use crate::{SettingsStore, Universe};

/// Effective developer choices, the open cell's parts and those let through.
#[derive(Resource, Default)]
pub(crate) struct DeveloperLayer {
    effective: DeveloperMarkers,
    registry: Registry,
    /// Places in the registry of the parts the filters let through, reused
    /// every frame (`E-HOT-NOALLOC`).
    shown: Vec<usize>,
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "read by the developer drawing from T3 (#1133)")
)]
impl DeveloperLayer {
    /// Effective choices this frame (release forces the master off).
    pub(crate) const fn effective(&self) -> DeveloperMarkers {
        self.effective
    }

    /// Parts the master switch and filters let through this frame.
    pub(crate) fn shown(&self) -> impl Iterator<Item = &PartRecord> + '_ {
        let records = self.registry.records();
        self.shown.iter().filter_map(move |&slot| records.get(slot))
    }

    /// Number of parts let through this frame.
    pub(crate) fn shown_count(&self) -> usize {
        self.shown.len()
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
    mut layer: ResMut<DeveloperLayer>,
) {
    let effective = store.0.developer_markers.effective();
    let layer = &mut *layer;
    layer.effective = effective;
    layer.shown.clear();
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
    }
    for (slot, record) in layer.registry.records().iter().enumerate() {
        if effective.allows(record.spec().class) {
            layer.shown.push(slot);
        }
    }
}

/// Registers the developer layer.
pub(crate) struct DeveloperPlugin;

impl Plugin for DeveloperPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DeveloperLayer>()
            .add_systems(Update, sync_developer_layer);
    }
}

#[cfg(test)]
#[path = "developer_tests.rs"]
mod tests;
