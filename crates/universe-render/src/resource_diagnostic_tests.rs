//! Hermetic descriptor/identity tests; real render retirement needs window proof.

use super::*;

#[test]
fn mesh_descriptor_is_actual_source_data_and_keeps_no_mesh_handle() {
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::PrimitiveTopology;
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32; 3]; 3]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32; 3]; 3]);
    mesh.insert_indices(Indices::U16(vec![0, 1, 2]));
    let descriptor = Descriptor::mesh(&mesh);
    assert_eq!(
        descriptor,
        Descriptor::Mesh {
            vertices: 3,
            vertex_bytes: 72,
            index_bytes: 6
        }
    );
    let diagnostics = LoadingDiagnostics::default();
    let producer = diagnostics.context(&universe_core::nav::Universe::new(42));
    let mut assets = Assets::<Mesh>::default();
    let handle = assets.add(mesh);
    let key = AssetKey::Mesh(handle.id());
    producer.created(
        key,
        Role::MoonProxy(universe_core::nav::MarkerIndex(1)),
        descriptor,
    );
    producer.created(
        key,
        Role::MoonProxy(universe_core::nav::MarkerIndex(1)),
        descriptor,
    );
    assert_eq!(diagnostics.0.resources.lock().unwrap().entries.len(), 1);
    assert!(assets.remove(handle.id()).is_some());
    producer.removed(key, true);
    let registry = diagnostics.0.resources.lock().unwrap();
    assert_eq!(registry.entries[0].descriptor, descriptor);
    assert_eq!(
        registry.entries[0].render, None,
        "main removal is not render retirement"
    );
    assert!(!registry.entries[0].was_resident);
}

#[test]
fn retained_lifetimes_are_bounded_and_exceeding_the_limit_is_sticky() {
    let diagnostics = LoadingDiagnostics::default();
    let producer = diagnostics.context(&universe_core::nav::Universe::new(42));
    let mut registry = Registry::default();
    let mut assets = Assets::<Image>::default();
    for _ in 0..RESOURCE_LIMIT {
        let handle = assets.add(Image::default());
        registry
            .insert(
                AssetKey::Image(handle.id()),
                Role::SharedAtlas,
                producer.source,
                Descriptor::Material,
            )
            .unwrap();
    }
    let capacity = registry.entries.capacity();
    let extra = assets.add(Image::default());
    assert!(
        registry
            .insert(
                AssetKey::Image(extra.id()),
                Role::SharedAtlas,
                producer.source,
                Descriptor::Material
            )
            .is_none()
    );
    assert_eq!(registry.entries.capacity(), capacity);
    assert!(registry.problem.unwrap().contains("8192 distinct assets"));
}
