// Shared-grid patch vertex shader (#920 T2 prototype): the shape rebuilt in
// `patch_grid_common`, then the CDLOD blend as `patch_morph.wgsl`.

#import bevy_pbr::{
    mesh_functions,
    mesh_view_bindings::view,
    forward_io::VertexOutput,
    view_transformations::position_world_to_clip,
}
#import universe_render::patch_grid_common::{grid_lit, grid_factor, lit_normal, patch_grid}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let l = grid_lit(vertex.instance_index, vertex.vertex_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let own_world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(l.position, 1.0));
    let distance_m = length(own_world.xyz - view.world_position) * patch_grid.scale.x;
    let factor = grid_factor(l.tile, l.flags, l.morph_err, distance_m);
    let morphed_pos = mix(l.position, l.parent_position, factor);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(lit_normal(l, factor), vertex.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(morphed_pos, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
    return out;
}
