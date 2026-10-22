// LOD patch morph vertex shader (rebuild step 4, #777).
//
// CDLOD geomorphing per vertex (Q2 A, Q6 A, Q7 A, #777 review items 1, 3,
// 4): every vertex blends between its own position/normal and its parent
// position/normal by ITS OWN distance to the camera, over the last 30% of a
// band ending at `morph_err x focal / detail` (where the parent splits).
// This mirrors `universe_core::lod_morph::vertex_factor` line for line.
// Edges (Q3 A): an in-between point on an edge touching a coarser
// neighbour is pinned to the coarse edge (factor 1); on an edge touching a
// finer neighbour it holds its own place (factor 0). Nothing here changes
// per frame on the CPU: the mask uniform is one of 16 x 16 shared materials,
// the band scale changes only with Planet detail.

#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    mesh_view_bindings::view,
    forward_io::VertexOutput,
    view_transformations::position_world_to_clip,
}

struct PatchMorph {
    // Edges touching a coarser neighbour as 0/1: East, West, North, South.
    coarser: vec4<f32>,
    // Edges touching a finer neighbour as 0/1: East, West, North, South.
    finer: vec4<f32>,
    // x: metres per world unit, y: focal_px / detail_px.
    scale: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> patch_morph: PatchMorph;

fn morph_factor(distance_m: f32, end_m: f32) -> f32 {
    if (end_m <= 0.0) {
        return 0.0;
    }
    let zone = end_m * 0.3;
    let start = end_m - zone;
    if (distance_m <= start) {
        return 0.0;
    }
    if (distance_m >= end_m) {
        return 1.0;
    }
    return clamp((distance_m - start) / zone, 0.0, 1.0);
}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(17) parent_position: vec3<f32>,
    @location(18) parent_normal_packed: vec4<f32>,
    @location(20) morph_err: f32,
};

// Edge flags of this vertex (East, West, North, South as 0/1), from its
// place in the patch mesh (#918 T4), mirroring `patches::vertex_edge_bits`:
// the 33 x 33 grid row by row, then the skirts South, North, West, East,
// 33 each.
fn edge_flags(v: u32) -> vec4<f32> {
    let side = 33u;
    var di = v % side;
    var dj = v / side;
    if (v >= side * side) {
        let s = v - side * side;
        let k = s % side;
        let e = s / side;
        if (e == 0u) {
            di = k;
            dj = 0u;
        } else if (e == 1u) {
            di = k;
            dj = side - 1u;
        } else if (e == 2u) {
            di = 0u;
            dj = k;
        } else {
            di = side - 1u;
            dj = k;
        }
    }
    let odd_j = (dj % 2u) == 1u;
    let odd_i = (di % 2u) == 1u;
    return vec4<f32>(
        select(0.0, 1.0, di == side - 1u && odd_j),
        select(0.0, 1.0, di == 0u && odd_j),
        select(0.0, 1.0, dj == side - 1u && odd_i),
        select(0.0, 1.0, dj == 0u && odd_i),
    );
}


@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    // The vertex's own (unblended) place sets its distance, as in core.
    let own_world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    let distance_m = length(own_world.xyz - view.world_position) * patch_morph.scale.x;
    let end_m = vertex.morph_err * patch_morph.scale.y;
    var factor = morph_factor(distance_m, end_m);
    // The vertex's place in this mesh: the index less the mesh's first.
    let flags = edge_flags(vertex.vertex_index - mesh[vertex.instance_index].first_vertex_index);
    if (dot(flags, patch_morph.coarser) > 0.5) {
        factor = 1.0;
    } else if (dot(flags, patch_morph.finer) > 0.5) {
        factor = 0.0;
    }
    let morphed_pos = mix(vertex.position, vertex.parent_position, factor);
    // A blend of opposed normals is a zero vector, and normalising it gives
    // NaN, which lighting and bloom spread over the screen (#780): keep the
    // own normal then.
    let blended = mix(vertex.normal, vertex.parent_normal_packed.xyz, factor);
    var morphed_nrm = vertex.normal;
    if (dot(blended, blended) > 1e-12) {
        morphed_nrm = normalize(blended);
    }
    out.world_normal = mesh_functions::mesh_normal_local_to_world(morphed_nrm, vertex.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(morphed_pos, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
    return out;
}
