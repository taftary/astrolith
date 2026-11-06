// The ground's fragment shader (#1161 T4): Bevy's standard lit ground with
// its colour from the world type's palette (`universe_core::world_type`),
// varied by height, slope and latitude, with a mid-scale tone variation and
// a fine close detail that fades with distance (S1: Terrain quality sets
// its reach). Everything reads the point's place on the body, so the
// material turns with the planet, holds across detail levels and meets
// across patch edges. Surface colour stays the only input this changes; the
// sun, the moons and the fill still light it as before (#1160 C5).

#import universe_render::patch_grid_common::{value_noise, ground_look}
#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
    mesh_view_bindings::{view, globals},
}

// `patch_grid.wgsl`'s output.
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
    @location(8) detail: vec4<f32>,
    @location(9) body: vec3<f32>,
    // rgb: the base ground colour (per vertex); w: the mid tone.
    @location(10) tint: vec4<f32>,
    // #1162: x: coast field (negative on water), y: water depth (m), z: dry
    // lake ring share.
    @location(11) water: vec3<f32>,
};

// `grid_tile::NOISE_PERIOD_M`, and the close detail's coarsest wavelength.
const NOISE_PERIOD_M: f32 = 4096.0;
const DETAIL_COARSE_M: f32 = 64.0;
const DETAIL_OCTAVES: u32 = 8u;

// The close detail in [-1, 1] at `place` (m, periodic): only the two
// octaves that fit a pixel of `footprint_m` (the finer one about 4 pixels
// across), cross-faded by the distance, so each pixel reads two noise
// values at any distance (#1161 T4 speed).
fn close_detail(place: vec3<f32>, footprint_m: f32) -> f32 {
    let finest = DETAIL_COARSE_M / f32(1u << (DETAIL_OCTAVES - 1u));
    let wanted = max(4.0 * footprint_m, finest);
    let level = clamp(log2(DETAIL_COARSE_M / wanted), 0.0, f32(DETAIL_OCTAVES - 1u));
    let k = floor(level);
    let fade = smoothstep(0.0, 1.0, level - k);
    let coarse_m = DETAIL_COARSE_M / exp2(k);
    let fine_m = coarse_m * 0.5;
    let a = value_noise(place / coarse_m, i32(NOISE_PERIOD_M / coarse_m)) * 2.0 - 1.0;
    let b = value_noise(place / fine_m, i32(NOISE_PERIOD_M / fine_m)) * 2.0 - 1.0;
    return a * 0.65 + b * 0.35 * fade;
}

// How much of this point glows on a lava world (#1161 T5, Q6 A), 0 to 1:
// cracks at two scales (the ridges of a noise field from the body
// direction, so they turn with the planet and meet across patches), and the
// low lava fields under the plains at a softer glow. A crack network finer
// than a few pixels fades to its average glow, so the far view stays calm
// (no speckle). Steady: it reads no clock.
const CRACK_WIDE_M: f32 = 1500.0;
const CRACK_NARROW_M: f32 = 250.0;
const CRACK_MEAN: f32 = 0.06;
const FIELD_GLOW: f32 = 0.4;

fn cracks_at(body: vec3<f32>, radius_m: f32, cell_m: f32, low: f32, high: f32, footprint_m: f32) -> f32 {
    let keep = smoothstep(8.0 * footprint_m, 24.0 * footprint_m, cell_m);
    if (keep <= 0.0) {
        return CRACK_MEAN;
    }
    let ridge = 1.0 - abs(2.0 * value_noise(body * (radius_m / cell_m), 0) - 1.0);
    return mix(CRACK_MEAN, smoothstep(low, high, ridge), keep);
}

// The ground colour of this point: the base colour from the palette (per
// vertex, `patch_grid_common::ground_base`), then the ice cracks and the
// close detail near the camera, per pixel.
fn ground_colour(in: GroundVertexOutput) -> vec3<f32> {
    let look = ground_look;
    let radius_m = look.bands.x;
    let reach_m = look.bands.z;
    var c = in.tint.rgb;
    // Ice cracks (criterion 1): long dark blue-grey lines from the same
    // ridge field as the lava cracks, colour only, fading like them.
    if (look.steep.w > 0.5) {
        let body = normalize(in.body);
        let distance_m = length(in.world_position.xyz - view.world_position);
        let footprint_m = max(distance_m * 0.0015, 0.01);
        let wide = cracks_at(body, radius_m, CRACK_WIDE_M, 0.95, 0.99, footprint_m);
        let narrow = cracks_at(body, radius_m, CRACK_NARROW_M, 0.96, 0.995, footprint_m);
        let crack = clamp(max(wide, 0.6 * narrow), 0.0, 1.0);
        c = mix(c, look.steep.rgb * 0.7, crack * 0.8);
    }
    // The close detail near the camera.
    let distance_m = length(in.world_position.xyz - view.world_position);
    let near = 1.0 - smoothstep(0.5 * reach_m, reach_m, distance_m);
    if (near > 0.0) {
        let footprint_m = max(distance_m * 0.0015, 0.01);
        let d = close_detail(in.detail.xyz, footprint_m);
        c = c * (1.0 + 0.22 * d * near);
    }
    return max(c, vec3<f32>(0.0));
}

// Seas and lakes (#1162 T5). The water is the ground's own surface, flat
// at its level (core `water`), so it is opaque and lit like the ground by
// the sun, the moons and the fill; only its colour, its shine and, near
// the camera, its normal differ.

// The pale dry ring of a partly filled lake (sRGB 0.78, 0.74, 0.62).
const RING_COLOUR: vec3<f32> = vec3<f32>(0.571, 0.509, 0.342);
// Fresh sea ice, a little bluer than snow.
const SEA_ICE: vec3<f32> = vec3<f32>(0.70, 0.80, 0.90);
// `universe_core::water::WAVE_PERIOD_S`: every wave repeats within it.
const WAVE_PERIOD_S: f32 = 960.0;
// `grid_tile::NOISE_PERIOD_M`, the period of the place the waves read.
const PLACE_PERIOD_M: f32 = 4096.0;

// One wave train: whole cycles per place period along x, y and z (so it
// meets itself across the wrap), whole cycles per wave period in time,
// and its slope amplitude.
fn wave_slope(place: vec3<f32>, cycles: vec3<f32>, turns: f32, amp: f32, t: f32) -> vec3<f32> {
    let k = cycles * (6.2831853 / PLACE_PERIOD_M);
    let w = turns * (6.2831853 / WAVE_PERIOD_S);
    return k * (amp * cos(dot(place, k) - w * t));
}

// The waves' slope at `place` (m, periodic) and wave time `t`: four trains
// of about 6 to 25 m crossing at different angles, each fading once its
// wavelength gets below a few pixels of `footprint_m`.
fn waves_at(place: vec3<f32>, t: f32, footprint_m: f32) -> vec3<f32> {
    var g = vec3<f32>(0.0);
    let f = 3.0 * footprint_m;
    g += wave_slope(place, vec3<f32>(131.0, 37.0, 97.0), 214.0, 0.22, t) * smoothstep(f, 2.0 * f, 24.5);
    g += wave_slope(place, vec3<f32>(-83.0, 151.0, 41.0), 239.0, 0.16, t) * smoothstep(f, 2.0 * f, 23.1);
    g += wave_slope(place, vec3<f32>(197.0, -113.0, 163.0), 293.0, 0.09, t) * smoothstep(f, 2.0 * f, 14.7);
    g += wave_slope(place, vec3<f32>(-311.0, -227.0, 389.0), 389.0, 0.05, t) * smoothstep(f, 2.0 * f, 7.5);
    return g;
}

// The water's wave time now: the synced time plus the shader time since,
// at the clock's rate (0 while the sky is paused).
fn wave_time() -> f32 {
    let w = ground_look.waves;
    return w.x + (globals.time - w.y) * w.z;
}

fn lava_glow(in: GroundVertexOutput) -> f32 {
    let look = ground_look;
    let radius_m = look.bands.x;
    let scale = max(look.bands.y, 1e-3);
    let hn = (in.detail.w - radius_m) / (4000.0 * scale);
    let body = normalize(in.body);
    let distance_m = length(in.world_position.xyz - view.world_position);
    let footprint_m = max(distance_m * 0.0015, 0.01);
    let wide = cracks_at(body, radius_m, CRACK_WIDE_M, 0.93, 0.985, footprint_m);
    let narrow = cracks_at(body, radius_m, CRACK_NARROW_M, 0.95, 0.99, footprint_m);
    let fields = FIELD_GLOW * smoothstep(0.05, -0.05, hn);
    return clamp(max(max(wide, 0.7 * narrow), fields), 0.0, 1.0);
}

@fragment
fn fragment(in: GroundVertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var v: VertexOutput;
    v.position = in.position;
    v.world_position = in.world_position;
    v.world_normal = in.world_normal;
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    v.instance_index = in.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    v.visibility_range_dither = in.visibility_range_dither;
#endif
    var pbr_input = pbr_input_from_standard_material(v, is_front);
    if (ground_look.low.w > 0.5) {
        var colour = ground_colour(in);
        // Seas and lakes (#1162 T5).
        if (ground_look.water.x > 0.5) {
            // The coast: where the coast field crosses zero, a pixel wide.
            let wet = clamp(0.5 - in.water.x / max(fwidth(in.water.x), 1e-4), 0.0, 1.0);
            let ring = smoothstep(0.35, 0.65, in.water.z) * (1.0 - wet);
            colour = mix(colour, RING_COLOUR * (0.9 + 0.2 * in.tint.w), ring);
            if (wet > 0.0) {
                let body = normalize(in.body);
                let edge = ground_look.water.y;
                let frozen = select(smoothstep(edge - 0.004, edge + 0.004, abs(body.y)), 1.0, ground_look.water.x > 1.5);
                let depth = max(in.water.y, 0.0);
                let sea = mix(ground_look.sea_shallow.rgb, ground_look.sea_deep.rgb, smoothstep(0.0, 1.0, sqrt(depth / 60.0)));
                let ice = mix(SEA_ICE, ground_look.cap.rgb, 0.5) * (0.95 + 0.1 * in.tint.w);
                colour = mix(colour, mix(sea, ice, frozen), wet);
                let liquid = wet * (1.0 - frozen);
                // A wet, smooth surface: the sun's and the moons' glint.
                pbr_input.material.perceptual_roughness = mix(1.0, 0.12, liquid);
                pbr_input.material.reflectance = vec3<f32>(mix(0.0, 0.35, liquid));
                // Small moving waves near the camera (S1: Terrain quality
                // sets the reach); only the normal bends, so the surface
                // stays flat to stand on.
                let distance_m = length(in.world_position.xyz - view.world_position);
                let near = liquid * (1.0 - smoothstep(0.5 * ground_look.water.z, ground_look.water.z, distance_m));
                if (near > 0.0) {
                    let footprint_m = max(distance_m * 0.0015, 0.01);
                    let g = waves_at(in.detail.xyz, wave_time(), footprint_m) * near;
                    let n = pbr_input.N;
                    let bent = normalize(n - (g - n * dot(g, n)));
                    pbr_input.N = bent;
                    pbr_input.world_normal = bent;
                }
            }
        }
        // Lava (T5): the own light, separate from the surface colour and the
        // sun and moons (#1160 C5); molten ground is darker where it glows.
        if (ground_look.glow.w > 0.0) {
            let glow = lava_glow(in);
            colour = colour * (1.0 - 0.6 * glow);
            pbr_input.material.emissive = vec4<f32>(ground_look.glow.rgb * ground_look.glow.w * glow, 1.0);
        }
        pbr_input.material.base_color = vec4<f32>(colour, 1.0);
    }
    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
