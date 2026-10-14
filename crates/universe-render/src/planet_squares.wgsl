// Planet squares (#626, Spec v2), tunable by the engine lab (#665).
//
// A fragment-only extension of StandardMaterial. With the Planet look set
// to Squares, the base colour becomes a 3D chessboard of cubes in planet
// space (colour A for parity 0, colour B for parity 1), faded to their even
// mean once a square is only a few pixels wide; the place tones are not
// drawn. With Normal it is the standard unlit path, unchanged. The game
// always sends 100 m, black, white, 8 px, 2 px and no debug view; the pure
// reference is `universe_core::squares`. The debug views (position, normal,
// distance) are selected only by the engine lab.

#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, main_pass_post_lighting_processing},
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::view,
}

struct PlanetSquares {
    // Rows of the render-to-planet rotation (xyz; w unused).
    row_x: vec4<f32>,
    row_y: vec4<f32>,
    row_z: vec4<f32>,
    // xyz: the render origin's planet position in metres, reduced into
    // one period; w: metres per render unit.
    origin: vec4<f32>,
    // x: 1 with Squares on; y: square edge in metres; z: sharp size in
    // pixels; w: gray size in pixels.
    look: vec4<f32>,
    // rgb: colour of the parity-0 squares.
    colour_a: vec4<f32>,
    // rgb: colour of the parity-1 squares.
    colour_b: vec4<f32>,
    // x: debug view (0 off, 1 position, 2 normal, 3 distance); y, z: the
    // distance view's near and far metres.
    debug: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> squares: PlanetSquares;

// Render-space vector `v` in the planet frame.
fn to_planet(v: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        dot(squares.row_x.xyz, v),
        dot(squares.row_y.xyz, v),
        dot(squares.row_z.xyz, v),
    );
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    let alpha = pbr_input.material.base_color.a;
    let planet = to_planet(in.world_position.xyz * squares.origin.w) + squares.origin.xyz;
    let cell = planet / squares.look.y;
    if squares.look.x > 0.5 {
        let index = floor(cell);
        let sum = index.x + index.y + index.z;
        let parity = select(1.0, 0.0, sum - 2.0 * floor(sum * 0.5) < 0.5);
        // Squares per pixel; the fade runs from the sharp to the gray size
        // like `universe_core::squares::gray_share`.
        let fw = fwidth(cell);
        let per_px = max(fw.x, max(fw.y, fw.z));
        let share = smoothstep(1.0 / squares.look.z, 1.0 / squares.look.w, per_px);
        let pure = mix(squares.colour_a.rgb, squares.colour_b.rgb, parity);
        let even = (squares.colour_a.rgb + squares.colour_b.rgb) * 0.5;
        pbr_input.material.base_color = vec4<f32>(mix(pure, even, share), alpha);
    }
    let debug = squares.debug.x;
    if debug > 0.5 && debug < 1.5 {
        pbr_input.material.base_color = vec4<f32>(fract(cell), alpha);
    } else if debug > 1.5 && debug < 2.5 {
        let normal = normalize(to_planet(in.world_normal));
        pbr_input.material.base_color = vec4<f32>(normal * 0.5 + 0.5, alpha);
    } else if debug > 2.5 {
        let metres = length(in.world_position.xyz - view.world_position) * squares.origin.w;
        let span = max(squares.debug.z - squares.debug.y, 1e-3);
        let near = 1.0 - clamp((metres - squares.debug.y) / span, 0.0, 1.0);
        pbr_input.material.base_color = vec4<f32>(vec3<f32>(near), alpha);
    }
    var out: FragmentOutput;
    out.color = pbr_input.material.base_color;
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
