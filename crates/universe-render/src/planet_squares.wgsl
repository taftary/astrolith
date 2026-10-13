// Black-and-white 100 m squares on planets (#626, Spec v2).
//
// A fragment-only extension of StandardMaterial. With the Planet look set
// to Squares, the base colour becomes a 3D chessboard of SQUARE_M cubes in
// planet space (black 0, white 1), faded to an even gray once a square is
// only a few pixels wide; the place tones are not drawn. With Normal it is
// the standard unlit path, unchanged. The pure reference is
// `universe_core::squares`.

#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, main_pass_post_lighting_processing},
    forward_io::{VertexOutput, FragmentOutput},
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
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> squares: PlanetSquares;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    if squares.look.x > 0.5 {
        let local = in.world_position.xyz * squares.origin.w;
        let planet = vec3<f32>(
            dot(squares.row_x.xyz, local),
            dot(squares.row_y.xyz, local),
            dot(squares.row_z.xyz, local),
        ) + squares.origin.xyz;
        let cell = planet / squares.look.y;
        let index = floor(cell);
        let sum = index.x + index.y + index.z;
        let parity = select(1.0, 0.0, sum - 2.0 * floor(sum * 0.5) < 0.5);
        // Squares per pixel; the fade runs from the sharp to the gray size
        // like `universe_core::squares::gray_share`.
        let fw = fwidth(cell);
        let per_px = max(fw.x, max(fw.y, fw.z));
        let share = smoothstep(1.0 / squares.look.z, 1.0 / squares.look.w, per_px);
        let tone = mix(parity, 0.5, share);
        pbr_input.material.base_color = vec4<f32>(vec3<f32>(tone), pbr_input.material.base_color.a);
    }
    var out: FragmentOutput;
    out.color = pbr_input.material.base_color;
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
