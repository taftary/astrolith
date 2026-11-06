// The ground's vertex shader (#920, ADR 0050): the shape rebuilt in
// `patch_grid_common`, then the CDLOD blend (core `lod_morph`). It also
// hands the fragment stage the point's place on the body (#1161 T4): its
// direction, its height above the radius, and its place in the periodic
// close detail (the tile's wrapped centre plus the point's offset), so the
// ground's material stays fixed to the body and seamless across patches.

#import bevy_pbr::{
    mesh_functions,
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
}
#import universe_render::patch_grid_common::{grid_lit, grid_factor, lit_normal, patch_grid, ground_place, mid_tone, ground_base, ground_look}
#ifdef GROUND_WATER
#import universe_render::patch_grid_common::grid_water
#endif

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
};

// Bevy's `forward_io::VertexOutput` fields at their own locations, then the
// ground's (8 and up, clear of Bevy's 0 to 7).
struct GroundVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
#ifdef VISIBILITY_RANGE_DITHER
    @location(7) @interpolate(flat) visibility_range_dither: i32,
#endif
    // xyz: place in the close detail (m, periodic); w: height above the radius (m).
    @location(8) detail: vec4<f32>,
    // xyz: the body-fixed unit direction.
    @location(9) body: vec3<f32>,
    // rgb: the base ground colour (`ground_base`, per vertex); w: the mid tone.
    @location(10) tint: vec4<f32>,
#ifdef GROUND_WATER
    // #1162: x: coast field (negative on water), y: water depth (m), z: dry
    // lake ring share.
    @location(11) water: vec3<f32>,
#endif
};

@vertex
fn vertex(vertex: Vertex) -> GroundVertexOutput {
    var out: GroundVertexOutput;
    let l = grid_lit(vertex.instance_index, vertex.vertex_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let own_world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(l.position, 1.0));
    let distance_m = length(own_world.xyz - view.world_position) * patch_grid.scale.x;
    let factor = grid_factor(l.tile, l.flags, l.morph_err, distance_m);
    let morphed_pos = mix(l.position, l.parent_position, factor);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(lit_normal(l, factor), vertex.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(morphed_pos, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
    let place = ground_place(l, morphed_pos);
    out.detail = place.detail;
    out.body = place.body;
    let up = normalize(mesh_functions::mesh_normal_local_to_world(place.body, vertex.instance_index));
    let mid = mid_tone(place.body, ground_look.bands.x);
    let h_m = place.detail.w - ground_look.bands.x;
    var base = ground_base(h_m, normalize(out.world_normal), up, place.body, mid);
    // Ice worlds (#1162 Q1 A): the frozen sea is flat ground at sea level,
    // drawn per vertex in a smoother, bluer ice than the land (no water
    // shader on these worlds, T7 speed).
    if (ground_look.water.x > 1.5) {
        base = mix(base, vec3<f32>(0.62, 0.74, 0.86), 0.55 * (1.0 - smoothstep(0.5, 1.5, abs(h_m))));
    }
    out.tint = vec4<f32>(base, mid);
#ifdef GROUND_WATER
    out.water = vec3<f32>(1.0, 0.0, 0.0);
    if (ground_look.water.x > 0.5 && ground_look.water.x < 1.5) {
        out.water = grid_water(l.tile, vertex.instance_index, vertex.vertex_index, factor);
    }
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = 0;
#endif
    return out;
}
