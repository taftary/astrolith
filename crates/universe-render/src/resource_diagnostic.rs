//! Bounded scalar asset identities; never an extra owner of production assets.
//!
//! Creation/explicit removal are recorded at production sites. Separate main
//! ownership and render-store observations expose the retirement stages without
//! claiming that wgpu/driver allocations have physically been freed.

use crate::loading_diagnostic::{LoadingDiagnostics, LoadingEvent, Producer, Source};
use crate::patch_grid::PatchGridMaterial;
use bevy::mesh::Indices;
use bevy::prelude::*;

/// Maximum distinct asset lifetimes retained by one opt-in process.
pub(crate) const RESOURCE_LIMIT: usize = 8_192;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum AssetKey {
    Image(AssetId<Image>),
    Mesh(AssetId<Mesh>),
    Standard(AssetId<StandardMaterial>),
    Ground(AssetId<PatchGridMaterial>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    SharedAtlas,
    SharedGrid,
    SharedGroundMaterial,
    MoonProxy(universe_core::nav::MarkerIndex),
    MoonMaterial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Descriptor {
    Mesh {
        vertices: usize,
        vertex_bytes: usize,
        index_bytes: usize,
    },
    Atlas {
        width: u32,
        height: u32,
        layers: u32,
    },
    Material,
    UnavailableMeshData,
}

impl Descriptor {
    /// Populated data only, at birth before Bevy may move it to RenderWorld.
    pub(crate) fn mesh(mesh: &Mesh) -> Self {
        let (Ok(attributes), Ok(indices)) = (mesh.try_attributes(), mesh.try_indices_option())
        else {
            return Self::UnavailableMeshData;
        };
        let mut vertices = None;
        let mut vertex_bytes = 0;
        for (_, values) in attributes {
            vertices = Some(vertices.map_or(values.len(), |n: usize| n.min(values.len())));
            vertex_bytes += values.get_bytes().len();
        }
        let index_bytes = match indices {
            Some(Indices::U16(values)) => values.len() * size_of::<u16>(),
            Some(Indices::U32(values)) => values.len() * size_of::<u32>(),
            None => 0,
        };
        Self::Mesh {
            vertices: vertices.unwrap_or(0),
            vertex_bytes,
            index_bytes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainOwnership {
    pub in_asset_store: bool,
    pub entity_refs: usize,
    pub known_owner_refs: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResourceEvent {
    Created {
        id: usize,
        key: AssetKey,
        role: Role,
        descriptor: Descriptor,
    },
    MainRemoval {
        id: usize,
        existed: bool,
    },
    MainOwnership {
        id: usize,
        state: MainOwnership,
    },
    RenderState {
        id: usize,
        extracted_frame: u64,
        present: bool,
        previously_resident: bool,
    },
    RenderMesh {
        id: usize,
        extracted_frame: u64,
        vertices: u32,
        stride: u64,
        index_bytes: u64,
    },
    RenderAtlas {
        id: usize,
        extracted_frame: u64,
        width: u32,
        height: u32,
        layers: u32,
        mips: u32,
        samples: u32,
    },
    RenderFrame {
        extracted_frame: u64,
        tracked_resident: usize,
        meshes: usize,
        images: usize,
        materials: usize,
        active: usize,
        outgoing: usize,
        shared: usize,
        current_atlas: Option<AssetId<Image>>,
    },
    Scene {
        meshes: usize,
        images: usize,
        standard: usize,
        ground: usize,
        entities: usize,
        groups: usize,
        active_proxies: usize,
        outgoing_proxies: usize,
        proxy_vertices: usize,
        proxy_vertex_bytes: usize,
        proxy_index_bytes: usize,
    },
    Atlas {
        image: AssetId<Image>,
        pages: u32,
        tiles: usize,
        free: usize,
        next: u32,
        writes: usize,
        write_bytes: usize,
    },
    ExtractWrites {
        extracted_frame: u64,
        image: AssetId<Image>,
        previous_image: Option<AssetId<Image>>,
        before: usize,
        added: usize,
        pending_bytes: usize,
    },
    SubmitWrites {
        extracted_frame: u64,
        image: AssetId<Image>,
        writes: usize,
        bytes: usize,
        elapsed_ns: u64,
    },
}

#[derive(Debug)]
pub(super) struct Entry {
    pub id: usize,
    pub key: AssetKey,
    pub role: Role,
    pub source: Source,
    pub descriptor: Descriptor,
    pub main: Option<MainOwnership>,
    pub render: Option<bool>,
    pub was_resident: bool,
}

#[derive(Debug)]
pub(super) struct Registry {
    pub entries: Vec<Entry>,
    pub problem: Option<String>,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            entries: Vec::with_capacity(RESOURCE_LIMIT),
            problem: None,
        }
    }
}

impl Registry {
    fn insert(
        &mut self,
        key: AssetKey,
        role: Role,
        source: Source,
        descriptor: Descriptor,
    ) -> Option<usize> {
        if let Some(entry) = self.entries.iter().find(|e| e.key == key) {
            return Some(entry.id);
        }
        if self.entries.len() >= RESOURCE_LIMIT {
            self.problem =
                Some("loading resource lifetime limit exceeded (8192 distinct assets)".into());
            return None;
        }
        let id = self.entries.len();
        self.entries.push(Entry {
            id,
            key,
            role,
            source,
            descriptor,
            main: None,
            render: None,
            was_resident: false,
        });
        Some(id)
    }
}

impl Producer {
    pub(crate) fn resource(&self, event: ResourceEvent) {
        self.emit(LoadingEvent::Resource(event));
    }

    pub(crate) fn created(&self, key: AssetKey, role: Role, descriptor: Descriptor) {
        if let Ok(mut registry) = self.diagnostics.0.resources.lock() {
            let previous = registry.entries.len();
            if let Some(id) = registry.insert(key, role, self.source, descriptor)
                && id == previous
            {
                self.resource(ResourceEvent::Created {
                    id,
                    key,
                    role,
                    descriptor,
                });
                if descriptor == Descriptor::UnavailableMeshData {
                    registry.problem =
                        Some("required mesh descriptor unavailable at creation".into());
                }
            }
        }
    }

    pub(crate) fn removed(&self, key: AssetKey, existed: bool) {
        if let Ok(registry) = self.diagnostics.0.resources.lock()
            && let Some(entry) = registry.entries.iter().find(|e| e.key == key)
        {
            self.diagnostics.emit(
                entry.source,
                LoadingEvent::Resource(ResourceEvent::MainRemoval {
                    id: entry.id,
                    existed,
                }),
            );
        }
    }
}

pub(super) fn install(app: &mut App) {
    app.add_systems(
        Last,
        crate::resource_scene::observe.before(crate::ScenarioSystems::Observe),
    );
    crate::resource_render::install(app);
}

impl LoadingDiagnostics {
    pub(crate) fn resource_problem(&self, reason: &str) {
        if let Ok(mut registry) = self.0.resources.lock() {
            registry.problem.get_or_insert_with(|| reason.to_string());
        }
    }
}

#[cfg(test)]
#[path = "resource_diagnostic_tests.rs"]
mod tests;
