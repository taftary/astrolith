//! Bounded main-world observations after production commands have applied.
//! Known ownership is deliberately not a claim to count every strong handle.

use crate::loading_diagnostic::{LoadingDiagnostics, LoadingEvent, Producer};
use crate::patch_grid::{GridAtlas, PatchGridMaterial};
use crate::resource_diagnostic::{AssetKey, Descriptor, MainOwnership, ResourceEvent, Role};
use bevy::ecs::query::QueryState;
use bevy::prelude::*;

const REFERENCE_LIMIT: usize = 32_768;
type MeshQuery = (
    &'static Mesh3d,
    Option<&'static MeshMaterial3d<StandardMaterial>>,
    Option<&'static MeshMaterial3d<PatchGridMaterial>>,
    Option<&'static crate::planet::PlanetMesh>,
    Option<&'static crate::parent_proxy::ParentProxy>,
);

#[derive(Default)]
pub(super) struct Scratch {
    context: Option<((u64, u64, u8), Producer)>,
    query: Option<QueryState<MeshQuery>>,
    references: Vec<AssetKey>,
    owners: Vec<AssetKey>,
    groups: Vec<(AssetKey, AssetKey)>,
    proxies: Vec<AssetKey>,
}

impl Scratch {
    fn clear(&mut self) {
        self.references.clear();
        self.owners.clear();
        self.groups.clear();
        self.proxies.clear();
    }
}

fn count<A: Asset>(world: &World) -> Option<usize> {
    world.get_resource::<Assets<A>>().map(Assets::len)
}

fn present<A: Asset>(world: &World, id: AssetId<A>) -> bool {
    world
        .get_resource::<Assets<A>>()
        .is_some_and(|a| a.contains(id))
}

fn store_present(world: &World, key: AssetKey) -> bool {
    match key {
        AssetKey::Image(id) => present(world, id),
        AssetKey::Mesh(id) => present(world, id),
        AssetKey::Standard(id) => present(world, id),
        AssetKey::Ground(id) => present(world, id),
    }
}

fn references(keys: &[AssetKey], key: AssetKey) -> usize {
    keys.partition_point(|k| *k <= key) - keys.partition_point(|k| *k < key)
}

pub(super) fn observe(world: &mut World, mut scratch: Local<Scratch>) {
    let (Some(diagnostics), Some(universe)) = (
        world.get_resource::<LoadingDiagnostics>().cloned(),
        world.get_resource::<crate::Universe>(),
    ) else {
        return;
    };
    let key = (
        universe.root.0,
        universe.open_seed(),
        universe.level().get(),
    );
    if scratch
        .context
        .as_ref()
        .is_none_or(|(held, _)| *held != key)
    {
        scratch.context = Some((key, diagnostics.context(universe)));
    }
    let Some((_, producer)) = scratch.context.clone() else {
        return;
    };
    let (Some(meshes), Some(images), Some(standard), Some(ground)) = (
        count::<Mesh>(world),
        count::<Image>(world),
        count::<StandardMaterial>(world),
        count::<PatchGridMaterial>(world),
    ) else {
        diagnostics.resource_problem("required main-world asset store unavailable");
        return;
    };
    scratch.clear();
    let mut query = scratch
        .query
        .take()
        .unwrap_or_else(|| world.query::<MeshQuery>());
    let mut entities = 0;
    let (mut active_proxies, mut outgoing_proxies) = (0, 0);
    for (mesh, standard, ground, proxy, parent) in query.iter(world) {
        if scratch.references.len() + 2 > REFERENCE_LIMIT || scratch.groups.len() == REFERENCE_LIMIT
        {
            diagnostics.resource_problem("main-world resource reference limit exceeded (32768)");
            break;
        }
        entities += 1;
        let mesh = AssetKey::Mesh(mesh.id());
        scratch.references.push(mesh);
        let material = standard
            .map(|m| AssetKey::Standard(m.id()))
            .or_else(|| ground.map(|m| AssetKey::Ground(m.id())));
        if let Some(material) = material {
            scratch.references.push(material);
            scratch.groups.push((mesh, material));
        }
        if let Some(proxy) = proxy {
            if proxy.host.is_some() && proxy.host == producer.source.host {
                active_proxies += 1;
            } else {
                outgoing_proxies += 1;
            }
            scratch.proxies.push(mesh);
        }
        if let Some(parent) = parent {
            if Some(parent.0) == producer.source.host {
                active_proxies += 1;
            } else {
                outgoing_proxies += 1;
            }
            scratch.proxies.push(mesh);
        }
    }
    scratch.query = Some(query);
    if let Some(atlas) = world.get_resource::<GridAtlas>() {
        scratch.owners.extend(atlas.diagnostic_owners());
    }
    if let Some(proxies) = world.get_resource::<crate::planet::PlanetMeshState>() {
        scratch.owners.extend(proxies.diagnostic_owner());
    }
    if let Some(cache) = world.get_resource::<crate::patch_cache::PatchCache>() {
        if scratch.owners.len() + cache.map.len() > REFERENCE_LIMIT {
            diagnostics.resource_problem("main-world known-owner limit exceeded (32768)");
        } else {
            scratch.owners.extend(
                cache
                    .map
                    .values()
                    .filter_map(|entry| entry.gpu.as_ref())
                    .map(|(mesh, _)| AssetKey::Mesh(mesh.id())),
            );
        }
    }
    scratch.references.sort_unstable();
    scratch.owners.sort_unstable();
    scratch.proxies.sort_unstable();
    scratch.proxies.dedup();
    scratch.groups.sort_unstable();
    scratch.groups.dedup();
    let (mut proxy_vertices, mut proxy_vertex_bytes, mut proxy_index_bytes) = (0, 0, 0);
    if let Ok(mut registry) = diagnostics.0.resources.lock() {
        for entry in &mut registry.entries {
            let state = MainOwnership {
                in_asset_store: store_present(world, entry.key),
                entity_refs: references(&scratch.references, entry.key),
                known_owner_refs: references(&scratch.owners, entry.key),
            };
            if entry.main != Some(state) {
                diagnostics.emit(
                    entry.source,
                    LoadingEvent::Resource(ResourceEvent::MainOwnership {
                        id: entry.id,
                        state,
                    }),
                );
                entry.main = Some(state);
            }
            if scratch.proxies.binary_search(&entry.key).is_ok()
                && let Descriptor::Mesh {
                    vertices,
                    vertex_bytes,
                    index_bytes,
                } = entry.descriptor
            {
                proxy_vertices += vertices;
                proxy_vertex_bytes += vertex_bytes;
                proxy_index_bytes += index_bytes;
            }
        }
        if scratch.proxies.iter().any(|key| {
            !registry.entries.iter().any(|e| {
                e.key == *key
                    && matches!(e.role, Role::MoonProxy(_) | Role::ParentProxy)
                    && matches!(e.descriptor, Descriptor::Mesh { .. })
            })
        }) {
            registry
                .problem
                .get_or_insert_with(|| "required live proxy descriptor missing".into());
        }
    }
    producer.resource(ResourceEvent::Scene {
        meshes,
        images,
        standard,
        ground,
        entities,
        groups: scratch.groups.len(),
        active_proxies,
        outgoing_proxies,
        proxy_vertices,
        proxy_vertex_bytes,
        proxy_index_bytes,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counted_references_do_not_assume_one_owner_per_asset() {
        let a = AssetKey::Mesh(AssetId::default());
        let b = AssetKey::Image(AssetId::default());
        let mut keys = vec![a, b, a, a];
        keys.sort_unstable();
        assert_eq!(references(&keys, a), 3);
        assert_eq!(references(&keys, b), 1);
        assert_eq!(references(&keys, AssetKey::Ground(AssetId::default())), 0);
    }
}
