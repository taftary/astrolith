// Shared-grid patch rebuild (#920, ADR 0050).
//
// Every patch draws the same 33 x 33 grid plus skirts; its shape comes from
// one tile of the shared heights texture, picked by the entity's MeshTag.
// This is `universe_core::grid_rebuild` line for line: offsets from the
// patch centre built only from small differences, with the tangent steps
// worked out in f64 on the CPU, so no trigonometry runs here (GPU trig is
// only bounded to about 2^-11 on Vulkan). Own normals come from the
// rebuilt neighbours, the parent shape from the parent grid, as
// `lod_patch_morph` builds them on the CPU today; the parent level's
// normals are stored packed at the even points (#920 T4).

#define_import_path universe_render::patch_grid_common

#import bevy_pbr::{mesh_bindings::mesh, mesh_functions}

// `patch_material::GroundScale` (each tile holds its patch's mask).
struct PatchGrid {
    // x: metres per world unit, y: focal_px / detail_px.
    scale: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> patch_grid: PatchGrid;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var grid_heights: texture_2d_array<u32>;

// The tile layout of `universe_core::grid_tile`.
const TILE_W: i32 = 37;
const TILE_H: i32 = 63;
const ERR_ROW: i32 = 37;
const PARAM_ROW: i32 = 52;
// `grid_tile::MASK_TEXEL`: the morph mask, in the per-patch row.
const MASK_COL: i32 = 9;
const QUAD_COL: i32 = 10;
const STEP_U_ROW: i32 = 53;
const STEP_V_ROW: i32 = 54;
const PARENT_NORMAL_ROW: i32 = 55;
const SIDE: u32 = 33u;
// `patch_grid` pages: 27 x 16 tiles on each 1,024-texel layer.
const PAGE_COLS: u32 = 27u;
const PAGE_TILES: u32 = 432u;

struct Tile {
    origin: vec2<i32>,
    page: i32,
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
    // Quad length (metres), the unit of the stored morph errors.
    quad: f32,
    // Edges touching a coarser / finer neighbour as 0/1 lanes (East, West,
    // North, South), from the tile's mask texel.
    coarser: vec4<f32>,
    finer: vec4<f32>,
    // Worked out once per vertex from the values above.
    len_c2: f32,
    len_c: f32,
    cc: vec3<f32>,
    dir_c: vec3<f32>,
};

fn raw(t: Tile, x: i32, y: i32) -> u32 {
    return textureLoad(grid_heights, t.origin + vec2<i32>(x, y), t.page, 0).r;
}

fn texel(t: Tile, x: i32, y: i32) -> f32 {
    return bitcast<f32>(raw(t, x, y));
}

fn tile_of(instance_index: u32) -> Tile {
    let tag = mesh_functions::get_tag(instance_index);
    var t: Tile;
    let slot = tag % PAGE_TILES;
    t.page = i32(tag / PAGE_TILES);
    t.origin = vec2<i32>(i32(slot % PAGE_COLS) * TILE_W, i32(slot / PAGE_COLS) * TILE_H);
    let face = i32(raw(t, 0, PARAM_ROW));
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
    t.has_parent = f32(raw(t, 6, PARAM_ROW));
    t.tu_c = texel(t, 7, PARAM_ROW);
    t.tv_c = texel(t, 8, PARAM_ROW);
    t.quad = texel(t, QUAD_COL, PARAM_ROW);
    let mask = raw(t, MASK_COL, PARAM_ROW);
    t.coarser = vec4<f32>(vec4<u32>(mask, mask >> 1u, mask >> 2u, mask >> 3u) & vec4<u32>(1u));
    t.finer = vec4<f32>(vec4<u32>(mask >> 4u, mask >> 5u, mask >> 6u, mask >> 7u) & vec4<u32>(1u));
    t.len_c2 = 1.0 + t.tu_c * t.tu_c + t.tv_c * t.tv_c;
    t.len_c = sqrt(t.len_c2);
    t.cc = place(t, t.sign, t.tu_c, t.tv_c);
    t.dir_c = t.cc / t.len_c;
    return t;
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

// The tangent step of column `i` and row `j` (ring included, -2..34).
fn step_u(t: Tile, i: i32) -> f32 {
    return texel(t, i + 2, STEP_U_ROW);
}

fn step_v(t: Tile, j: i32) -> f32 {
    return texel(t, j + 2, STEP_V_ROW);
}

fn height(t: Tile, i: i32, j: i32) -> f32 {
    return texel(t, i + 2, j + 2);
}

// The grid point with height `dh` over the centre and tangent steps
// `dtu`, `dtv`, as `grid_rebuild::offset_from_steps`: the steps come from
// the tile (worked out in f64 on the CPU), so no trigonometry runs here.
fn point_from(t: Tile, dh: f32, dtu: f32, dtv: f32) -> GridPoint {
    let tu = t.tu_c + dtu;
    let tv = t.tv_c + dtv;
    let dlen2 = dtu * (tu + t.tu_c) + dtv * (tv + t.tv_c);
    let len = sqrt(t.len_c2 + dlen2);
    let dc = place(t, 0.0, dtu, dtv);
    let shrink = dlen2 / (len * t.len_c * (len + t.len_c));
    let dd = dc / len - t.cc * shrink;
    var p: GridPoint;
    p.offset = (t.radius_c + dh) * dd + dh * t.dir_c;
    p.dir = t.dir_c + dd;
    return p;
}

// Grid point (di, dj) of the patch (ring included, -2..34).
fn point_at(t: Tile, di: i32, dj: i32) -> GridPoint {
    return point_from(t, height(t, di, dj), step_u(t, di), step_v(t, dj));
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

// The grid point (di, dj) of vertex `v`: the 33 x 33 points row by row,
// then the skirts South, North, West, East (33 each), as `lod_patch_morph`
// orders them; z is 1 for a skirt vertex.
fn grid_at(v: u32) -> vec3<u32> {
    if (v < SIDE * SIDE) {
        return vec3<u32>(v % SIDE, v / SIDE, 0u);
    }
    let s = v - SIDE * SIDE;
    let k = s % SIDE;
    let e = s / SIDE;
    if (e == 0u) {
        return vec3<u32>(k, 0u, 1u);
    } else if (e == 1u) {
        return vec3<u32>(k, SIDE - 1u, 1u);
    } else if (e == 2u) {
        return vec3<u32>(0u, k, 1u);
    }
    return vec3<u32>(SIDE - 1u, k, 1u);
}

// Edge flags of grid point (di, dj): East, West, North, South as 0/1 for
// the odd points on an edge, which a coarser neighbour pins.
fn edge_flags(di: u32, dj: u32) -> vec4<f32> {
    let odd_i = (di % 2u) == 1u;
    let odd_j = (dj % 2u) == 1u;
    return vec4<f32>(
        select(0.0, 1.0, di == SIDE - 1u && odd_j),
        select(0.0, 1.0, di == 0u && odd_j),
        select(0.0, 1.0, dj == SIDE - 1u && odd_i),
        select(0.0, 1.0, dj == 0u && odd_i),
    );
}

// The stored morph error of grid point (di, dj): a half share of the quad
// length, two a texel (`grid_tile::tile_morph_err`).
fn morph_err_at(t: Tile, di: u32, dj: u32) -> f32 {
    let idx = i32(dj * SIDE + di);
    let k = idx / 2;
    let pair = unpack2x16float(raw(t, k % TILE_W, ERR_ROW + k / TILE_W));
    return t.quad * select(pair.y, pair.x, idx % 2 == 0);
}

// The CDLOD factor at `distance_m` from the camera for a vertex with
// `flags` and morph error `err`, with the edge pins of the tile's mask
// (core `lod_morph::vertex_factor`).
fn grid_factor(t: Tile, flags: vec4<f32>, err: f32, distance_m: f32) -> f32 {
    var factor = morph_factor(distance_m, err * patch_grid.scale.y);
    if (dot(flags, t.coarser) > 0.5) {
        factor = 1.0;
    } else if (dot(flags, t.finer) > 0.5) {
        factor = 0.0;
    }
    return factor;
}

struct GridVertex {
    tile: Tile,
    position: vec3<f32>,
    parent_position: vec3<f32>,
    morph_err: f32,
    flags: vec4<f32>,
};

// One vertex, positions only (the shadow pass): three grid points at most.
fn grid_vertex(instance_index: u32, vertex_index: u32) -> GridVertex {
    let t = tile_of(instance_index);
    let at = grid_at(vertex_index - mesh[instance_index].first_vertex_index);
    let i = i32(at.x);
    let j = i32(at.y);
    let own = point_at(t, i, j);
    var out: GridVertex;
    out.tile = t;
    out.position = own.offset;
    out.parent_position = own.offset;
    let odd_i = (at.x % 2u) == 1u;
    let odd_j = (at.y % 2u) == 1u;
    if (t.has_parent > 0.5 && (odd_i || odd_j)) {
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
        out.parent_position = (point_at(t, a.x, a.y).offset + point_at(t, b.x, b.y).offset) * 0.5;
    }
    if (at.z == 1u) {
        out.position = out.position - own.dir * t.depth;
        out.parent_position = out.parent_position - own.dir * t.depth;
    }
    out.morph_err = morph_err_at(t, at.x, at.y);
    out.flags = edge_flags(at.x, at.y);
    return out;
}

// The parent level's normal at even point (2 ei, 2 ej), unpacked from
// `grid_tile::pack_normal` (two 16-bit octahedral components).
fn parent_normal_even(t: Tile, ei: i32, ej: i32) -> vec3<f32> {
    let k = ej * 17 + ei;
    let bits = raw(t, k % TILE_W, PARENT_NORMAL_ROW + k / TILE_W);
    let p = vec2<f32>(f32(bits & 65535u), f32(bits >> 16u)) / 65535.0 * 2.0 - 1.0;
    var n = vec3<f32>(p.x, p.y, 1.0 - abs(p.x) - abs(p.y));
    let fold = max(-n.z, 0.0);
    n.x += select(fold, -fold, n.x >= 0.0);
    n.y += select(fold, -fold, n.y >= 0.0);
    return normalize(n);
}

// One lit vertex (the main pass): positions and both normals. The four
// neighbours of the own normal are shared with the parent pair where they
// coincide (an odd point on a row or column has its parent pair as two
// neighbours); the parent normals are the stored ones of the pair.
struct GridLit {
    tile: Tile,
    position: vec3<f32>,
    parent_position: vec3<f32>,
    normal: vec3<f32>,
    parent_normal: vec3<f32>,
    morph_err: f32,
    flags: vec4<f32>,
};

fn grid_lit(instance_index: u32, vertex_index: u32) -> GridLit {
    let t = tile_of(instance_index);
    let at = grid_at(vertex_index - mesh[instance_index].first_vertex_index);
    let di = at.x;
    let dj = at.y;
    let i = i32(di);
    let j = i32(dj);
    let su = step_u(t, i);
    let sv = step_v(t, j);
    let own = point_from(t, height(t, i, j), su, sv);
    let east = point_from(t, height(t, i + 1, j), step_u(t, i + 1), sv).offset;
    let west = point_from(t, height(t, i - 1, j), step_u(t, i - 1), sv).offset;
    let north = point_from(t, height(t, i, j + 1), su, step_v(t, j + 1)).offset;
    let south = point_from(t, height(t, i, j - 1), su, step_v(t, j - 1)).offset;
    var c = cross(east - west, north - south);
    if (dot(c, own.dir) < 0.0) {
        c = -c;
    }
    var out: GridLit;
    out.tile = t;
    out.normal = select(own.dir, normalize(c), dot(c, c) > 1e-30);
    out.position = own.offset;
    out.parent_position = own.offset;
    out.parent_normal = out.normal;
    let odd_i = (di % 2u) == 1u;
    let odd_j = (dj % 2u) == 1u;
    if (t.has_parent > 0.5) {
        if (odd_i && odd_j) {
            // The parent's diagonal b-c.
            let pa = point_at(t, i + 1, j - 1).offset;
            let pb = point_at(t, i - 1, j + 1).offset;
            out.parent_position = (pa + pb) * 0.5;
        } else if (odd_i) {
            out.parent_position = (west + east) * 0.5;
        } else if (odd_j) {
            out.parent_position = (south + north) * 0.5;
        }
        // Even ends of the pair, in parent-grid units.
        var a = vec2<i32>(i, j) / 2;
        var b = a;
        if (odd_i && odd_j) {
            a = vec2<i32>(i + 1, j - 1) / 2;
            b = vec2<i32>(i - 1, j + 1) / 2;
        } else if (odd_i) {
            a = vec2<i32>(i - 1, j) / 2;
            b = vec2<i32>(i + 1, j) / 2;
        } else if (odd_j) {
            a = vec2<i32>(i, j - 1) / 2;
            b = vec2<i32>(i, j + 1) / 2;
        }
        let na = parent_normal_even(t, a.x, a.y);
        let nb = parent_normal_even(t, b.x, b.y);
        let mid = (na + nb) * 0.5;
        out.parent_normal = select(na, normalize(mid), dot(mid, mid) > 1e-12);
    }
    if (at.z == 1u) {
        out.position = out.position - own.dir * t.depth;
        out.parent_position = out.parent_position - own.dir * t.depth;
    }
    out.morph_err = morph_err_at(t, di, dj);
    out.flags = edge_flags(di, dj);
    return out;
}

// The morphed normal at blend `factor`. A blend of opposed normals is a
// zero vector, and normalising it gives NaN (#780): keep the own normal.
fn lit_normal(l: GridLit, factor: f32) -> vec3<f32> {
    let blended = mix(l.normal, l.parent_normal, factor);
    return select(l.normal, normalize(blended), dot(blended, blended) > 1e-12);
}
