// Shared-grid patch rebuild (#920 T2 prototype, behind ASTROLITH_SHARED_GRID).
//
// Every patch draws the same 33 x 33 grid plus skirts; its shape comes from
// one tile of the shared heights texture, picked by the entity's MeshTag.
// This is `universe_core::grid_rebuild` line for line: offsets from the
// patch centre built only from small differences, with polynomial sin and
// cos (GPU trig is only bounded to about 2^-11 on Vulkan). Normals come
// from the rebuilt neighbours, the parent shape from the parent grid, as
// `lod_patch_morph` builds them on the CPU today.

#define_import_path universe_render::patch_grid_common

#import bevy_pbr::{mesh_bindings::mesh, mesh_functions}

struct PatchGrid {
    // Edges touching a coarser neighbour as 0/1: East, West, North, South.
    coarser: vec4<f32>,
    // Edges touching a finer neighbour as 0/1: East, West, North, South.
    finer: vec4<f32>,
    // x: metres per world unit, y: focal_px / detail_px.
    scale: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> patch_grid: PatchGrid;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var grid_heights: texture_2d<f32>;

const TILE_W: i32 = 37;
const TILE_H: i32 = 70;
const TILES_X: u32 = 55u;
const ERR_ROW: i32 = 37;
const PARAM_ROW: i32 = 67;
const STEP_U_ROW: i32 = 68;
const STEP_V_ROW: i32 = 69;
const SIDE: u32 = 33u;

struct Tile {
    origin: vec2<i32>,
    dom: i32,
    ua: i32,
    va: i32,
    sign: f32,
    au: f32,
    av: f32,
    step: f32,
    radius_c: f32,
    depth: f32,
    has_parent: f32,
    tu_c: f32,
    tv_c: f32,
};

fn texel(t: Tile, x: i32, y: i32) -> f32 {
    return textureLoad(grid_heights, t.origin + vec2<i32>(x, y), 0).r;
}

fn tile_of(instance_index: u32) -> Tile {
    let tag = mesh_functions::get_tag(instance_index);
    var t: Tile;
    t.origin = vec2<i32>(i32(tag % TILES_X) * TILE_W, i32(tag / TILES_X) * TILE_H);
    let face = i32(texel(t, 0, PARAM_ROW) + 0.5);
    // PosX, NegX, PosY, NegY, PosZ, NegZ as cube_sphere lays them out.
    t.dom = face / 2;
    t.sign = select(1.0, -1.0, face % 2 == 1);
    t.ua = select(0, 1, t.dom == 0);
    t.va = select(1, 2, t.dom != 2);
    t.au = texel(t, 1, PARAM_ROW);
    t.av = texel(t, 2, PARAM_ROW);
    t.step = texel(t, 3, PARAM_ROW);
    t.radius_c = texel(t, 4, PARAM_ROW);
    t.depth = texel(t, 5, PARAM_ROW);
    t.has_parent = texel(t, 6, PARAM_ROW);
    t.tu_c = texel(t, 7, PARAM_ROW);
    t.tv_c = texel(t, 8, PARAM_ROW);
    return t;
}

fn poly_sin(x: f32) -> f32 {
    let x2 = x * x;
    return x * (1.0 + x2 * (-1.0 / 6.0 + x2 * (1.0 / 120.0 + x2 * (-1.0 / 5040.0 + x2 / 362880.0))));
}

fn poly_cos(x: f32) -> f32 {
    let x2 = x * x;
    return 1.0 + x2 * (-0.5 + x2 * (1.0 / 24.0 + x2 * (-1.0 / 720.0 + x2 * (1.0 / 40320.0 - x2 / 3628800.0))));
}

fn place(t: Tile, d: f32, u: f32, v: f32) -> vec3<f32> {
    var out = vec3<f32>(0.0);
    out[t.dom] = d;
    out[t.ua] = u;
    out[t.va] = v;
    return out;
}

struct GridPoint {
    offset: vec3<f32>,
    dir: vec3<f32>,
};

// Grid point (di, dj) of the patch (ring included, -2..34), as
// `grid_rebuild::offset_from_steps`: the tangent steps come from the tile
// (worked out in f64 on the CPU), so no trigonometry runs here.
fn point_at(t: Tile, di: i32, dj: i32) -> GridPoint {
    let dh = texel(t, di + 2, dj + 2);
    let tu_c = t.tu_c;
    let tv_c = t.tv_c;
    let dtu = texel(t, di + 2, STEP_U_ROW);
    let dtv = texel(t, dj + 2, STEP_V_ROW);
    let tu = tu_c + dtu;
    let tv = tv_c + dtv;
    let len_c2 = 1.0 + tu_c * tu_c + tv_c * tv_c;
    let dlen2 = dtu * (tu + tu_c) + dtv * (tv + tv_c);
    let len_c = sqrt(len_c2);
    let len = sqrt(len_c2 + dlen2);
    let cc = place(t, t.sign, tu_c, tv_c);
    let dc = place(t, 0.0, dtu, dtv);
    let shrink = dlen2 / (len * len_c * (len + len_c));
    let dd = dc / len - cc * shrink;
    let dir_c = cc / len_c;
    var p: GridPoint;
    p.offset = (t.radius_c + dh) * dd + dh * dir_c;
    p.dir = dir_c + dd;
    return p;
}

// Outward unit normal at (di, dj) from the points k apart.
fn normal_at(t: Tile, di: i32, dj: i32, k: i32, dir: vec3<f32>) -> vec3<f32> {
    let e = point_at(t, di + k, dj).offset;
    let w = point_at(t, di - k, dj).offset;
    let n = point_at(t, di, dj + k).offset;
    let s = point_at(t, di, dj - k).offset;
    var c = cross(e - w, n - s);
    if (dot(c, dir) < 0.0) {
        c = -c;
    }
    if (dot(c, c) <= 1e-30) {
        return dir;
    }
    return normalize(c);
}

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

struct GridVertex {
    position: vec3<f32>,
    parent_position: vec3<f32>,
    morph_err: f32,
    // Edge flags (East, West, North, South as 0/1).
    flags: vec4<f32>,
    // What the normals need later (only where they are used).
    tile: Tile,
    at: vec2<i32>,
    dir: vec3<f32>,
    // The parent pair of an odd point (equal to `at` for an even one).
    a: vec2<i32>,
    b: vec2<i32>,
    a_dir: vec3<f32>,
    b_dir: vec3<f32>,
    even: bool,
};

// One vertex of the shared grid: the 33 x 33 points row by row, then the
// skirts South, North, West, East (33 each), as `lod_patch_morph` orders
// them. Positions only: three grid points at most. Normals come from
// `grid_normal`, which only the passes that light the ground call.
fn grid_vertex(instance_index: u32, vertex_index: u32) -> GridVertex {
    let t = tile_of(instance_index);
    let v = vertex_index - mesh[instance_index].first_vertex_index;
    var di = v % SIDE;
    var dj = v / SIDE;
    let skirt = v >= SIDE * SIDE;
    if (skirt) {
        let s = v - SIDE * SIDE;
        let k = s % SIDE;
        let e = s / SIDE;
        if (e == 0u) {
            di = k;
            dj = 0u;
        } else if (e == 1u) {
            di = k;
            dj = SIDE - 1u;
        } else if (e == 2u) {
            di = 0u;
            dj = k;
        } else {
            di = SIDE - 1u;
            dj = k;
        }
    }
    let i = i32(di);
    let j = i32(dj);
    let own = point_at(t, i, j);
    var out: GridVertex;
    out.tile = t;
    out.at = vec2<i32>(i, j);
    out.dir = own.dir;
    out.position = own.offset;
    out.parent_position = own.offset;
    out.a = out.at;
    out.b = out.at;
    out.a_dir = own.dir;
    out.b_dir = own.dir;
    let odd_i = (di % 2u) == 1u;
    let odd_j = (dj % 2u) == 1u;
    out.even = !odd_i && !odd_j;
    if (t.has_parent > 0.5 && !out.even) {
        var a = vec2<i32>(i, j - 1);
        var b = vec2<i32>(i, j + 1);
        if (odd_i && !odd_j) {
            a = vec2<i32>(i - 1, j);
            b = vec2<i32>(i + 1, j);
        } else if (odd_i && odd_j) {
            // The parent's diagonal b-c.
            a = vec2<i32>(i + 1, j - 1);
            b = vec2<i32>(i - 1, j + 1);
        }
        let pa = point_at(t, a.x, a.y);
        let pb = point_at(t, b.x, b.y);
        out.parent_position = (pa.offset + pb.offset) * 0.5;
        out.a = a;
        out.b = b;
        out.a_dir = pa.dir;
        out.b_dir = pb.dir;
    }
    if (skirt) {
        out.position = out.position - own.dir * t.depth;
        out.parent_position = out.parent_position - own.dir * t.depth;
    }
    let idx = i32(dj * SIDE + di);
    out.morph_err = texel(t, idx % TILE_W, ERR_ROW + idx / TILE_W);
    out.flags = vec4<f32>(
        select(0.0, 1.0, di == SIDE - 1u && odd_j),
        select(0.0, 1.0, di == 0u && odd_j),
        select(0.0, 1.0, dj == SIDE - 1u && odd_i),
        select(0.0, 1.0, dj == 0u && odd_i),
    );
    return out;
}

// The morphed normal of `g` at blend `factor`: its own normal, blended
// with the parent-level normal only when the vertex is morphing (most are
// not, so most skip the parent's twelve grid points).
fn grid_normal(g: GridVertex, factor: f32) -> vec3<f32> {
    let t = g.tile;
    let own = normal_at(t, g.at.x, g.at.y, 1, g.dir);
    if (factor <= 0.0 || t.has_parent < 0.5) {
        return own;
    }
    var parent = own;
    if (g.even) {
        parent = normal_at(t, g.at.x, g.at.y, 2, g.dir);
    } else {
        let na = normal_at(t, g.a.x, g.a.y, 2, g.a_dir);
        let nb = normal_at(t, g.b.x, g.b.y, 2, g.b_dir);
        let mid = (na + nb) * 0.5;
        parent = select(na, normalize(mid), dot(mid, mid) > 1e-12);
    }
    // A blend of opposed normals is a zero vector, and normalising it gives
    // NaN (#780): keep the own normal then.
    let blended = mix(own, parent, factor);
    if (dot(blended, blended) > 1e-12) {
        return normalize(blended);
    }
    return own;
}

// The CDLOD factor for `g` at `distance_m` from the camera, with the edge
// pins of this material's mask (as `patch_morph.wgsl`).
fn grid_factor(g: GridVertex, distance_m: f32) -> f32 {
    var factor = morph_factor(distance_m, g.morph_err * patch_grid.scale.y);
    if (dot(g.flags, patch_grid.coarser) > 0.5) {
        factor = 1.0;
    } else if (dot(g.flags, patch_grid.finer) > 0.5) {
        factor = 0.0;
    }
    return factor;
}
