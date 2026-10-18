// LOD patch morph vertex shader (rebuild step 4, #777 T3).
//
// CDLOD geomorphing in the vertex shader (Q2 A, Q6 A, Q7 A): every vertex
// blends between its own position/normal and its parent-level position and
// normal over the last 30% of its band. The band and camera offset come
// from the per-patch uniform (updated per frame, no mesh rebuilds); the
// factor here mirrors `universe_core::lod_morph::morph_factor` exactly
// (the agreement test pins the same numbers). Edge pinning (Q3 A): on each
// edge touching a coarser neighbour the fine patch's odd vertices are
// pinned exactly onto the coarse edge (factor 1).

#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    forward_io::VertexOutput,
    view_transformations::position_world_to_clip,
}

struct PatchMorph {
    // Camera position relative to the patch centre, in metres (xyz).
    camera_rel: vec4<f32>,
    // Distance band in metres: x is start, y is end.
    band: vec4<f32>,
    // Edge mask as 0/1: East, West, North, South.
    edge_mask: vec4<f32>,
};

@group(2) @binding(100) var<uniform> patch_morph: PatchMorph;

fn morph_factor(distance_m: f32, band_start_m: f32, band_end_m: f32) -> f32 {
    let width = band_end_m - band_start_m;
    let zone = width * 0.3;
    let morph_start = band_end_m - zone;
    if (distance_m <= morph_start) {
        return 0.0;
    }
    if (distance_m >= band_end_m) {
        return 1.0;
    }
    return clamp((distance_m - morph_start) / zone, 0.0, 1.0);
}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(5) color: vec4<f32>,
    @location(17) parent_position: vec3<f32>,
    @location(18) parent_normal: vec3<f32>,
    @location(19) edge_flags: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let distance_m = length(patch_morph.camera_rel.xyz);
    let factor = morph_factor(distance_m, patch_morph.band.x, patch_morph.band.y);
    // Pin odd edge vertices on masked edges fully to the coarse edge.
    let pin = max(
        max(vertex.edge_flags.x * patch_morph.edge_mask.x, vertex.edge_flags.y * patch_morph.edge_mask.y),
        max(vertex.edge_flags.z * patch_morph.edge_mask.z, vertex.edge_flags.w * patch_morph.edge_mask.w),
    );
    let eff = max(factor, pin);
    let morphed_pos = mix(vertex.position, vertex.parent_position, eff);
    let morphed_nrm = normalize(mix(vertex.normal, vertex.parent_normal, eff));
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(morphed_nrm, vertex.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(morphed_pos, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
    out.color = vertex.color;
    return out;
}
