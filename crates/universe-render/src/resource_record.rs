//! Explicit resource sidecar v1 payloads; physical GPU allocation is unavailable.

use crate::resource_diagnostic::{AssetKey, Descriptor, ResourceEvent, Role};

fn asset(key: AssetKey) -> String {
    // AssetId contains its generation/type. It is supplemental, process-local
    // identity; numeric `resource` is the sidecar's bounded lifetime identity.
    match key {
        AssetKey::Image(id) => format!("image:{id:?}"),
        AssetKey::Mesh(id) => format!("mesh:{id:?}"),
        AssetKey::Standard(id) => format!("standard-material:{id:?}"),
        AssetKey::Ground(id) => format!("ground-material:{id:?}"),
    }
}

fn descriptor(value: Descriptor) -> String {
    match value {
        Descriptor::Mesh { vertices, vertex_bytes, index_bytes } => format!("vertices={vertices}\tlogical_vertex_bytes={vertex_bytes}\tlogical_index_bytes={index_bytes}\tmeasurement=populated-source-arrays-at-birth\tallocation_padding_included=false"),
        Descriptor::Atlas { width, height, layers } => format!("width={width}\theight={height}\tlayers={layers}\tmip_levels=1\tsamples=1\tformat=R32Uint\tlogical_texture_bytes={}\ttotal_gpu_allocation=false", u64::from(width) * u64::from(height) * u64::from(layers) * 4),
        Descriptor::Material => "logical_material_bytes=unavailable-engine-bind-group-layout\ttotal_gpu_allocation=false".into(),
        Descriptor::UnavailableMeshData => "vertices=unavailable-extracted-mesh-data\tlogical_vertex_bytes=unavailable-extracted-mesh-data\tlogical_index_bytes=unavailable-extracted-mesh-data".into(),
    }
}

pub(crate) fn payload(event: ResourceEvent) -> String {
    match event {
        ResourceEvent::Created {
            id,
            key,
            role,
            descriptor: value,
        } => {
            let marker = match role {
                Role::MoonProxy(marker) => marker.0.to_string(),
                _ => "not-a-moon-proxy".into(),
            };
            let role = match role {
                Role::SharedAtlas => "shared-atlas",
                Role::SharedGrid => "shared-grid",
                Role::SharedGroundMaterial => "shared-ground-material",
                Role::MoonProxy(_) => "moon-proxy",
                Role::MoonMaterial => "moon-material",
            };
            format!(
                "kind=resource-created\tresource={id}\tasset={}\trole={role}\tmoon_marker={marker}\t{}",
                asset(key),
                descriptor(value)
            )
        }
        ResourceEvent::MainRemoval { id, existed } => format!(
            "kind=main-remove-request\tresource={id}\tremoved_from_store={existed}\trender_retired=false\tphysical_gpu_release=unavailable-driver-lifetime"
        ),
        ResourceEvent::MainOwnership { id, state } => format!(
            "kind=main-ownership\tresource={id}\tin_asset_store={}\tentity_refs={}\tknown_owner_refs={}\towner_scope=atlas-cache-and-render-entities\tall_strong_handles_counted=false",
            state.in_asset_store, state.entity_refs, state.known_owner_refs
        ),
        ResourceEvent::RenderState {
            id,
            extracted_frame,
            present,
            previously_resident,
        } => format!(
            "kind=render-resource\tresource={id}\textracted_main_frame={extracted_frame}\tin_render_store={present}\tpreviously_resident={previously_resident}\tstage={}\tphysical_gpu_release=unavailable-driver-lifetime",
            if present {
                "resident"
            } else if previously_resident {
                "absent-after-residence"
            } else {
                "not-yet-observed-resident"
            }
        ),
        ResourceEvent::RenderFrame {
            extracted_frame,
            tracked_resident,
            meshes,
            images,
            materials,
            active,
            outgoing,
            shared,
            current_atlas,
        } => format!(
            "kind=render-frame\textracted_main_frame={extracted_frame}\ttracked_resident={tracked_resident}\tactive_private_resident={active}\toutgoing_resident={outgoing}\tshared_resident={shared}\tcurrent_atlas={}\trender_meshes={meshes}\trender_images={images}\trender_materials={materials}\tcheckpoint=after-prepare-assets\ttotal_gpu_allocation=unavailable\tactual_draw_calls=unavailable\tgpu_execution_ms=unavailable-not-enabled",
            current_atlas.map_or_else(|| "none".into(), |id| asset(AssetKey::Image(id)))
        ),
        ResourceEvent::RenderMesh {
            id,
            extracted_frame,
            vertices,
            stride,
            index_bytes,
        } => format!(
            "kind=render-mesh-descriptor\tresource={id}\textracted_main_frame={extracted_frame}\tvertices={vertices}\tvertex_stride_bytes={stride}\tlogical_gpu_vertex_bytes={}\tlogical_gpu_index_bytes={index_bytes}\tmeasurement=prepared-render-layout\tslab_padding_included=false\ttotal_gpu_allocation=false",
            u64::from(vertices) * stride
        ),
        ResourceEvent::RenderAtlas {
            id,
            extracted_frame,
            width,
            height,
            layers,
            mips,
            samples,
        } => format!(
            "kind=render-atlas-descriptor\tresource={id}\textracted_main_frame={extracted_frame}\twidth={width}\theight={height}\tlayers={layers}\tmip_levels={mips}\tsamples={samples}\tformat=R32Uint\tbase_level_logical_bytes={}\ttotal_gpu_allocation=false",
            u64::from(width) * u64::from(height) * u64::from(layers) * 4
        ),
        ResourceEvent::Scene {
            meshes,
            images,
            standard,
            ground,
            entities,
            groups,
            active_proxies,
            outgoing_proxies,
            proxy_vertices,
            proxy_vertex_bytes,
            proxy_index_bytes,
        } => format!(
            "kind=resource-scene\tmain_meshes={meshes}\tmain_images={images}\tmain_standard_materials={standard}\tmain_ground_materials={ground}\tmesh_entities={entities}\tlogical_submission_groups={groups}\tgroup_key=mesh-material-pair-not-actual-draw-call\tactive_proxy_entities={active_proxies}\toutgoing_proxy_entities={outgoing_proxies}\tproxy_vertices={proxy_vertices}\tproxy_logical_vertex_bytes={proxy_vertex_bytes}\tproxy_logical_index_bytes={proxy_index_bytes}\tproxy_bytes_scope=unique-referenced-mesh-source-data"
        ),
        ResourceEvent::Atlas {
            image,
            pages,
            tiles,
            free,
            next,
            writes,
            write_bytes,
        } => format!(
            "kind=atlas-state\tasset={}\tpages={pages}\ttile_entries={tiles}\tfree_tiles={free}\tnext_tile={next}\tmain_pending_writes={writes}\tmain_pending_logical_bytes={write_bytes}",
            asset(AssetKey::Image(image))
        ),
        ResourceEvent::ExtractWrites {
            extracted_frame,
            image,
            previous_image,
            before,
            added,
            pending_bytes,
        } => format!(
            "kind=render-writes-extract\textracted_main_frame={extracted_frame}\tasset={}\tprevious_asset={}\tpending_before={before}\tadded={added}\tpending_logical_bytes={pending_bytes}\timage_changed={}",
            asset(AssetKey::Image(image)),
            previous_image.map_or_else(|| "none".into(), |id| asset(AssetKey::Image(id))),
            previous_image.is_some_and(|id| id != image)
        ),
        ResourceEvent::SubmitWrites {
            extracted_frame,
            image,
            writes,
            bytes,
            elapsed_ns,
        } => format!(
            "kind=render-writes-submit\textracted_main_frame={extracted_frame}\tasset={}\twrites={writes}\tlogical_bytes={bytes}\tqueue_write_cpu_ns={elapsed_ns}\tgpu_completion=unavailable-not-fenced",
            asset(AssetKey::Image(image))
        ),
    }
}
