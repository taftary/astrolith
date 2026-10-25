// Shared-grid patch vertex shader (#920 T2 prototype): the shape rebuilt in
// `patch_grid_common`, then the CDLOD blend as `patch_morph.wgsl`.

#import bevy_pbr::{
    mesh_functions,
    mesh_view_bindings::view,
    forward_io::VertexOutput,
    view_transformations::position_world_to_clip,
}
#import universe_render::patch_grid_common::{grid_vertex, grid_factor, patch_grid}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let g = grid_vertex(vertex.instance_index, vertex.vertex_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let own_world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(g.position, 1.0));
    let distance_m = length(own_world.xyz - view.world_position) * patch_grid.scale.x;
    let factor = grid_factor(g, distance_m);
    let morphed_pos = mix(g.position, g.parent_position, factor);
    let blended = mix(g.normal, g.parent_normal, factor);
    var morphed_nrm = g.normal;
    if (dot(blended, blended) > 1e-12) {
        morphed_nrm = normalize(blended);
    }
    out.world_normal = mesh_functions::mesh_normal_local_to_world(morphed_nrm, vertex.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(morphed_pos, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
    return out;
}
