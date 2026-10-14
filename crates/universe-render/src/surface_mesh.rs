//! One icosphere surface mesh for the open planet (#458, ADR 0024, #632).
//!
//! Gizmo indicators cannot show a world, so the open planet builds exactly
//! one mesh: the core icosphere (`universe_core::icosphere`), split once
//! everywhere by default (Q2) and further where the camera looks, by size
//! on screen ([`crate::surface_split`]), down to the ground of every shell
//! level (#632). Vertices sit on the planet's flat faces (#593: finer
//! splits lie flat inside the 80 faces), computed in `f64` and narrowed to
//! `f32` only here, with flat normals, in one base tone. Every drawn
//! triangle edge shows as a thin marker line (a retained gizmo built with
//! the mesh, #593). Short skirts hang under every triangle edge so
//! neighbours split to different depths never show a gap. The mesh rebuilds
//! when the planet changes or the camera moves a fraction of its height;
//! it despawns off the planet.

use crate::Universe;
use crate::planet_squares::{SurfaceMaterial, surface_material};
use crate::surface_split::{DrawnTriangle, SplitView, centre_reach, gap, surface_triangles};
use crate::tokens::{
    SURFACE_EDGE_SQUARES_TINT, SURFACE_EDGE_TINT, SURFACE_EDGE_WIDTH_PX, SURFACE_TONE,
};
use bevy::asset::RenderAssetUsages;
use bevy::camera::Projection;
use bevy::gizmos::{GizmoAsset, config::GizmoLineConfig, retained::Gizmo};
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;
use std::collections::HashSet;
use universe_core::surface::SurfaceFrame;

/// Skirt length as a share of a triangle's reach.
const SKIRT_SHARE: f64 = 0.15;

/// The camera moves this share of its height before the mesh rebuilds.
const REBUILD_SHARE: f64 = 0.2;

/// Edge lines draw this much closer than the surface they lie on, so they
/// never flicker against it (retained gizmo depth bias, -1..1).
const EDGE_DEPTH_BIAS: f32 = -0.002;

/// Marks the surface mesh entity owned by [`draw_surface`].
#[derive(Component)]
pub(crate) struct SurfaceMesh;

/// Marks the surface edge-line entity owned by [`draw_surface`] (#593).
#[derive(Component)]
pub(crate) struct SurfaceEdges;

/// Tracks which surface and camera the spawned surface mesh was built for.
#[derive(Resource, Default)]
pub(crate) struct SurfaceMeshState {
    /// Open-cell seed, level, approached planet marker (#603), the
    /// squares look (#626) and the debug views (#640) the current mesh
    /// belongs to, if any.
    key: Option<(u64, u8, Option<u32>, bool, SurfaceDebug)>,
    /// Strength the material was last set to (#603).
    fade: f64,
    /// Strength the edge lines were last built at (#603).
    edges_fade: f64,
    /// The current mesh's edges and anchor, kept to rebuild the edge lines
    /// at a new strength without splitting again (#603).
    edges: Vec<([f64; 3], [f64; 3])>,
    /// Anchor (surface-cell units) the current edges are relative to.
    edges_anchor: [f64; 3],
    /// Camera offset the current mesh was split for.
    camera: Option<[f64; 3]>,
    /// View direction the current mesh was split for (#598).
    forward: Option<[f64; 3]>,
    /// Face centres and normals (half a triangle's reach long) of the
    /// current mesh, kept for the normals debug view (#640; only with it
    /// on, and only on an open planet, where surface units are open units).
    pub(crate) normals: Vec<([f64; 3], [f64; 3])>,
}

/// Debug views of the surface for development tools (#640, engine lab).
///
/// The game never inserts it, so its default (all off) leaves the game's
/// surface exactly as before.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SurfaceDebug {
    /// Hide the filled surface, keep the triangle edge lines.
    pub wireframe: bool,
    /// Draw each triangle's face normal as a short line.
    pub normals: bool,
    /// Colour each triangle by its icosphere depth (LOD depth).
    pub lod_colors: bool,
}

/// Figures of the last surface build, for development tools (#640).
#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub struct SurfaceStats {
    /// Drawn triangles.
    pub faces: usize,
    /// Distinct triangle edges.
    pub edges: usize,
    /// Distinct triangle corners.
    pub vertices: usize,
    /// Drawn triangles per icosphere depth (index = depth).
    pub per_depth: Vec<usize>,
}

impl SurfaceStats {
    /// The figures of `triangles`.
    #[must_use]
    pub(crate) fn of(triangles: &[DrawnTriangle], edges: usize) -> SurfaceStats {
        let bits = |p: [f64; 3]| [p[0].to_bits(), p[1].to_bits(), p[2].to_bits()];
        let vertices: HashSet<[u64; 3]> = triangles
            .iter()
            .flat_map(|triangle| triangle.corners.map(bits))
            .collect();
        let mut per_depth = Vec::new();
        for triangle in triangles {
            let depth = usize::from(triangle.addr.depth());
            if per_depth.len() <= depth {
                per_depth.resize(depth + 1, 0);
            }
            if let Some(count) = per_depth.get_mut(depth) {
                *count += 1;
            }
        }
        SurfaceStats {
            faces: triangles.len(),
            edges,
            vertices: vertices.len(),
            per_depth,
        }
    }

    /// Deepest split drawn (0 when nothing is drawn).
    #[must_use]
    pub fn max_depth(&self) -> u8 {
        u8::try_from(self.per_depth.len().saturating_sub(1)).unwrap_or(u8::MAX)
    }
}

/// Tone of LOD depth `depth` in the debug view: a ramp from deep blue
/// (the default split) through green to warm white (the deepest), so
/// neighbouring depths read apart.
#[must_use]
pub(crate) fn lod_tone(depth: u8) -> LinearRgba {
    let t = f32::from(depth.min(24)) / 24.0;
    let hue = 240.0 * (1.0 - t);
    Color::hsl(hue, 0.7, 0.35 + 0.3 * t).to_linear()
}

/// The camera turns this far (radians, about 10 degrees) before the mesh
/// rebuilds for the new view (#598, Q6 A).
pub(crate) const REBUILD_TURN: f64 = 10.0 * std::f64::consts::PI / 180.0;

/// Whether the mesh built for `last_forward` needs a rebuild for
/// `forward`: a turn past [`REBUILD_TURN`] (#598). The target no longer
/// matters to the split (Spec v2).
#[must_use]
pub(crate) fn view_changed(last_forward: Option<[f64; 3]>, forward: [f64; 3]) -> bool {
    let Some(last) = last_forward else {
        return true;
    };
    let along = (last[0] * forward[0] + last[1] * forward[1] + last[2] * forward[2])
        / (gap(last, [0.0; 3]) * gap(forward, [0.0; 3])).max(1e-300);
    along.clamp(-1.0, 1.0).acos() > REBUILD_TURN
}

/// Narrows a cell-space point for the GPU.
fn narrow(point: [f64; 3]) -> [f32; 3] {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cell-unit surface point narrowed for the GPU, intended"
    )]
    let narrowed = [point[0] as f32, point[1] as f32, point[2] as f32];
    narrowed
}

/// Every edge of `triangles` once, as cell-space point pairs (#593).
///
/// Neighbours share an edge's corners exactly (memoised corners), so a
/// shared edge is kept once whichever way round it is listed.
#[must_use]
pub(crate) fn surface_edges(triangles: &[DrawnTriangle]) -> Vec<([f64; 3], [f64; 3])> {
    let bits = |point: [f64; 3]| [point[0].to_bits(), point[1].to_bits(), point[2].to_bits()];
    let mut seen: HashSet<([u64; 3], [u64; 3])> = HashSet::new();
    let mut out = Vec::with_capacity(triangles.len() * 2);
    for triangle in triangles {
        let [a, b, c] = triangle.corners;
        for (from, to) in [(a, b), (b, c), (c, a)] {
            let key = if bits(from) <= bits(to) {
                (bits(from), bits(to))
            } else {
                (bits(to), bits(from))
            };
            if seen.insert(key) {
                out.push((from, to));
            }
        }
    }
    out
}

/// The edge lines of `triangles` as one retained gizmo, relative to
/// `anchor` like the mesh (#593, Q3 A, Q4 A).
#[cfg(test)]
#[must_use]
pub(crate) fn edge_gizmo(triangles: &[DrawnTriangle], anchor: [f64; 3]) -> GizmoAsset {
    edge_lines(&surface_edges(triangles), anchor, 1.0, false)
}

/// `edges` as one retained gizmo relative to `anchor`, at `fade` strength
/// (#603: the lines fade with the surface), orange over the squares look
/// (#626).
#[must_use]
pub(crate) fn edge_lines(
    edges: &[([f64; 3], [f64; 3])],
    anchor: [f64; 3],
    fade: f64,
    squares: bool,
) -> GizmoAsset {
    let mut gizmo = GizmoAsset::new();
    let local = |point: [f64; 3]| {
        Vec3::from(narrow([
            point[0] - anchor[0],
            point[1] - anchor[1],
            point[2] - anchor[2],
        ]))
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: a 0..1 strength narrowed for a colour, intended"
    )]
    let tint = if squares {
        SURFACE_EDGE_SQUARES_TINT
    } else {
        SURFACE_EDGE_TINT
    }
    .with_alpha(fade.clamp(0.0, 1.0) as f32);
    for (from, to) in edges {
        gizmo.line(local(*from), local(*to), tint);
    }
    gizmo
}

/// The edge lines rebuild when the strength has moved this much (#603).
const EDGE_FADE_STEP: f64 = 0.05;

/// Material for a surface at `fade` strength: opaque at full strength,
/// alpha-blended while it fades (#603).
fn faded(material: &mut StandardMaterial, fade: f64) {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: a 0..1 strength narrowed for a colour, intended"
    )]
    let alpha = fade.clamp(0.0, 1.0) as f32;
    material.base_color = Color::WHITE.with_alpha(alpha);
    material.alpha_mode = if alpha >= 1.0 {
        AlphaMode::Opaque
    } else {
        AlphaMode::Blend
    };
}

/// Outward unit normal of a flat triangle (away from `centre`).
fn face_normal(corners: [[f64; 3]; 3], centre: [f64; 3]) -> [f64; 3] {
    let [a, b, c] = corners;
    let e0 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let e1 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let cross = [
        e0[1] * e1[2] - e0[2] * e1[1],
        e0[2] * e1[0] - e0[0] * e1[2],
        e0[0] * e1[1] - e0[1] * e1[0],
    ];
    let length = gap(cross, [0.0; 3]).max(1e-300);
    let outward = (a[0] - centre[0]) * cross[0]
        + (a[1] - centre[1]) * cross[1]
        + (a[2] - centre[2]) * cross[2];
    let sign = if outward < 0.0 { -1.0 } else { 1.0 };
    [
        cross[0] * sign / length,
        cross[1] * sign / length,
        cross[2] * sign / length,
    ]
}

/// Builds the surface mesh of `triangles`, with its vertices relative to
/// `anchor` (an [`crate::style::Anchored`] entity places the mesh back at
/// `anchor` each frame).
///
/// One flat-coloured triangle list: three vertices per drawn triangle with
/// its tone as vertex colour and its flat face normal, plus a skirt quad
/// under each edge, pulled toward the planet centre by [`SKIRT_SHARE`] of
/// the triangle's reach. Returns `None` when nothing is drawn.
#[must_use]
pub(crate) fn surface_mesh_of(triangles: &[DrawnTriangle], anchor: [f64; 3]) -> Option<Mesh> {
    surface_mesh_toned(triangles, anchor, |_| SURFACE_TONE.to_linear())
}

/// [`surface_mesh_of`] with each triangle's tone from `tone_of` (the LOD
/// depth view, #640).
#[must_use]
pub(crate) fn surface_mesh_toned(
    triangles: &[DrawnTriangle],
    anchor: [f64; 3],
    tone_of: impl Fn(&DrawnTriangle) -> LinearRgba,
) -> Option<Mesh> {
    if triangles.is_empty() {
        return None;
    }
    let centre = [0.0; 3];
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(triangles.len() * 21);
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(triangles.len() * 21);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(triangles.len() * 21);
    let mut push = |point: [f64; 3], tone: LinearRgba, normal: [f64; 3]| {
        // Vertices sit relative to `anchor` (the camera at build time), so
        // ground near the camera keeps full precision (ADR 0026).
        positions.push(narrow([
            point[0] - anchor[0],
            point[1] - anchor[1],
            point[2] - anchor[2],
        ]));
        colors.push(tone.to_f32_array());
        // The flat face's normal: the standard pipeline wants one even
        // when unlit.
        normals.push(narrow(normal));
    };
    for triangle in triangles {
        let tone = tone_of(triangle);
        let [a, b, c] = triangle.corners;
        let normal = face_normal(triangle.corners, centre);
        push(a, tone, normal);
        push(b, tone, normal);
        push(c, tone, normal);
        let (_, reach) = centre_reach(triangle.corners);
        let skirt = |point: [f64; 3]| {
            let toward = [
                centre[0] - point[0],
                centre[1] - point[1],
                centre[2] - point[2],
            ];
            let length = gap(centre, point).max(1e-300);
            let step = reach * SKIRT_SHARE / length;
            [
                point[0] + toward[0] * step,
                point[1] + toward[1] * step,
                point[2] + toward[2] * step,
            ]
        };
        for (from, to) in [(a, b), (b, c), (c, a)] {
            let (low_from, low_to) = (skirt(from), skirt(to));
            push(from, tone, normal);
            push(to, tone, normal);
            push(low_to, tone, normal);
            push(from, tone, normal);
            push(low_to, tone, normal);
            push(low_from, tone, normal);
        }
    }
    let count = u32::try_from(positions.len()).ok()?;
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32((0..count).collect()));
    Some(mesh)
}

/// Height of `camera` above the planet surface (cell units, at least tiny).
fn height_above(frame: &SurfaceFrame, camera: [f64; 3]) -> f64 {
    let height = gap(camera, [0.0; 3]) - gap(frame.surface_point(camera), [0.0; 3]);
    if height.is_finite() {
        height.abs().max(1e-9)
    } else {
        1e-9
    }
}

/// Spawns the surface mesh and its edge lines for the open planet,
/// or for the planet being approached from the level above (#603),
/// despawning them elsewhere. The surface draws at the host's fade.
///
/// Rebuilds when the open cell changes, the camera has moved more than
/// [`REBUILD_SHARE`] of its height, or turned past [`REBUILD_TURN`]
/// (#598); per-frame cost is one height probe otherwise. The split follows
/// size on screen in the camera's view ([`crate::surface_split`]). The mesh carries [`SurfaceMesh`] and the
/// lines [`SurfaceEdges`], each with a `Name`, so inspectors and logs can
/// say which entity.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn draw_surface(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SurfaceMaterial>>,
    mut gizmos: ResMut<Assets<GizmoAsset>>,
    universe: Res<Universe>,
    nav: Res<crate::input::Navigation>,
    mut state: ResMut<SurfaceMeshState>,
    owned: Query<
        (
            Entity,
            &Mesh3d,
            &MeshMaterial3d<SurfaceMaterial>,
            &mut Visibility,
        ),
        With<SurfaceMesh>,
    >,
    lines: Query<(Entity, &Gizmo), With<SurfaceEdges>>,
    cameras: Query<&Projection, With<Camera3d>>,
    windows: Query<&Window>,
    store: Res<crate::SettingsStore>,
    debug: Option<Res<SurfaceDebug>>,
    stats: Option<ResMut<SurfaceStats>>,
) {
    let host = crate::planet_approach::surface_host(&universe);
    // A new Planet look rebuilds the mesh and its edge colour (#626); a
    // new debug view rebuilds it too (#640, never in the game).
    let squares = store.0.planet_look.squares();
    let debug = debug.map(|debug| *debug).unwrap_or_default();
    let key = host.as_ref().map(|host| {
        (
            // The surface's own seed: in the game the open planet's (or
            // the approached one's, with its marker); a tool showing
            // another body on its own rebuilds too (#654).
            host.frame.planet_seed(),
            host.frame.level().get(),
            host.marker,
            squares,
            debug,
        )
    });
    let camera = universe.path.offset();
    let moved = match (host.as_ref(), state.camera) {
        (Some(host), Some(last)) => {
            gap(host.to_surface(camera), host.to_surface(last))
                > REBUILD_SHARE * height_above(&host.frame, host.to_surface(last))
        }
        _ => true,
    };
    let turned = view_changed(state.forward, nav.forward);
    let mut owned = owned;
    if key != state.key || moved || turned {
        for (entity, mesh, material, _) in &owned {
            commands.entity(entity).despawn();
            meshes.remove(&mesh.0);
            materials.remove(&material.0);
        }
        for (entity, edges) in &lines {
            commands.entity(entity).despawn();
            gizmos.remove(&edges.handle);
        }
        state.key = None;
        state.camera = None;
        state.forward = None;
    }
    let Some(host) = host else {
        return;
    };
    if state.key == key {
        // Same mesh: follow the fade without splitting again (#603).
        if (host.fade - state.fade).abs() > 1e-6 {
            for (_, _, material, mut visibility) in &mut owned {
                if let Some(mut material) = materials.get_mut(&material.0) {
                    faded(&mut material.base, host.fade);
                }
                *visibility = if host.fade > 0.0 && !debug.wireframe {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
            state.fade = host.fade;
        }
        let edge_step = (host.fade - state.edges_fade).abs();
        let edge_end = (host.fade >= 1.0 || host.fade <= 0.0) && edge_step > 0.0;
        if edge_step >= EDGE_FADE_STEP || edge_end {
            for (_, edges) in &lines {
                if let Some(mut asset) = gizmos.get_mut(&edges.handle) {
                    *asset = edge_lines(&state.edges, state.edges_anchor, host.fade, squares);
                }
            }
            state.edges_fade = host.fade;
        }
        return;
    }
    let frame = &host.frame;
    let inside = host.to_surface(camera);
    // The real camera's view cone (Q1 A); all around without a camera.
    let fov = cameras.iter().find_map(|projection| match projection {
        Projection::Perspective(perspective) => Some(f64::from(perspective.fov)),
        _ => None,
    });
    let (aspect, height) = windows
        .iter()
        .next()
        .map_or((16.0 / 9.0, 1080.0), |window| {
            (
                f64::from(window.width() / window.height().max(1.0)),
                f64::from(window.height()),
            )
        });
    let view = fov.map_or(SplitView::all_around(inside), |fov| {
        SplitView::camera(inside, nav.forward, nav.up, fov, aspect, height)
    });
    let triangles = surface_triangles(frame, &view);
    let mesh = if debug.lod_colors {
        surface_mesh_toned(&triangles, inside, |triangle| {
            lod_tone(triangle.addr.depth())
        })
    } else {
        surface_mesh_of(&triangles, inside)
    };
    let Some(mesh) = mesh else {
        return;
    };
    let mut material = StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        cull_mode: None,
        ..default()
    };
    faded(&mut material, host.fade);
    // Built in the surface cell's units around the mapped camera, scaled
    // back by the cell ratio: an approached planet keeps full precision
    // near the camera (ADR 0026) at its true size and place (#603).
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: a cell ratio narrowed for the GPU transform, intended"
    )]
    let scale = Transform::from_scale(Vec3::splat(host.ratio as f32));
    // Wireframe (#640) keeps only the edge lines.
    let visibility = if host.fade > 0.0 && !debug.wireframe {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    commands.spawn((
        SurfaceMesh,
        Name::new("surface"),
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(surface_material(material))),
        scale,
        visibility,
        crate::style::Anchored(camera),
    ));
    let edges = surface_edges(&triangles);
    if let Some(mut stats) = stats {
        *stats = SurfaceStats::of(&triangles, edges.len());
    }
    state.normals = if debug.normals && host.marker.is_none() {
        triangles
            .iter()
            .map(|triangle| {
                // Half the triangle's reach long, so a line reads at any depth.
                let (centre, reach) = centre_reach(triangle.corners);
                let normal = face_normal(triangle.corners, [0.0; 3]);
                (centre, normal.map(|axis| axis * reach * 0.5))
            })
            .collect()
    } else {
        Vec::new()
    };
    commands.spawn((
        SurfaceEdges,
        Name::new("surface-edges"),
        Gizmo {
            handle: gizmos.add(edge_lines(&edges, inside, host.fade, squares)),
            line_config: GizmoLineConfig {
                width: SURFACE_EDGE_WIDTH_PX,
                ..default()
            },
            depth_bias: EDGE_DEPTH_BIAS,
        },
        scale,
        crate::style::Anchored(camera),
    ));
    state.key = key;
    state.camera = Some(camera);
    state.forward = Some(nav.forward);
    state.fade = host.fade;
    state.edges_fade = host.fade;
    state.edges = edges;
    state.edges_anchor = inside;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The demo journey's planet surface (L10).
    fn frame() -> SurfaceFrame {
        let chain = universe_core::nest::autopilot_path(universe_core::nav::DEMO_SEED);
        let planet = chain.get(..9).expect("journey reaches the planet");
        SurfaceFrame::for_path(universe_core::nav::DEMO_SEED, planet).expect("planet surface")
    }

    #[test]
    fn vertices_sit_on_the_flat_faces() {
        let surface = frame();
        let camera = [0.0, 0.36, 0.0];
        for triangle in surface_triangles(&surface, &SplitView::all_around(camera))
            .iter()
            .take(400)
        {
            let units = universe_core::icosphere::tri_corners(triangle.addr).expect("corners");
            for (unit, corner) in units.iter().zip(triangle.corners) {
                assert_eq!(
                    surface.surface_point(*unit),
                    corner,
                    "vertex off the surface"
                );
            }
        }
    }

    #[test]
    fn the_whole_planet_draws_in_one_tone() {
        // Nothing opens below the planet (#632): no brighter places.
        let surface = frame();
        let camera = surface.surface_point([0.3, 0.8, 0.5]).map(|c| c * 1.02);
        let triangles = surface_triangles(&surface, &SplitView::all_around(camera));
        let mesh = surface_mesh_of(&triangles, [0.0; 3]).expect("surface mesh");
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("surface carries vertex colours");
        };
        let base = SURFACE_TONE.to_linear().to_f32_array();
        assert!(colors.iter().all(|colour| colour == &base), "one tone");
    }

    #[test]
    fn every_drawn_edge_shows_once_and_follows_the_splits() {
        let surface = frame();
        let far = surface_triangles(&surface, &SplitView::all_around([0.0, 0.0, 50.0]));
        // 80 triangles of a closed surface: 120 shared edges.
        assert_eq!(surface_edges(&far).len(), 120, "each edge once");
        let camera = surface.surface_point([0.3, 0.8, 0.5]).map(|c| c * 1.2);
        let near = surface_triangles(&surface, &SplitView::all_around(camera));
        let edges = surface_edges(&near);
        assert!(edges.len() > 120, "finer splits near the camera add edges");
        // Every drawn triangle's three edges are among the lines.
        let bits = |p: [f64; 3]| [p[0].to_bits(), p[1].to_bits(), p[2].to_bits()];
        let lines: HashSet<([u64; 3], [u64; 3])> = edges
            .iter()
            .flat_map(|(a, b)| [(bits(*a), bits(*b)), (bits(*b), bits(*a))])
            .collect();
        for triangle in &near {
            let [a, b, c] = triangle.corners;
            for (from, to) in [(a, b), (b, c), (c, a)] {
                assert!(lines.contains(&(bits(from), bits(to))), "edge drawn");
            }
        }
        let gizmo = edge_gizmo(&near, camera);
        assert_eq!(
            gizmo.buffer().list_positions.len(),
            edges.len() * 2,
            "one marker line per edge"
        );
    }

    #[test]
    fn mesh_normals_are_flat_per_triangle() {
        let surface = frame();
        let triangles = surface_triangles(&surface, &SplitView::all_around([0.0, 0.0, 50.0]));
        let mesh = surface_mesh_of(&triangles, [0.0; 3]).expect("surface mesh");
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("surface carries normals");
        };
        // Each triangle: 3 face vertices then 18 skirt vertices, one normal.
        for chunk in normals.chunks(21) {
            assert!(chunk.iter().all(|n| n == &chunk[0]), "one flat normal");
        }
    }

    #[test]
    fn only_a_turn_past_ten_degrees_asks_for_a_rebuild() {
        let ahead = [0.0, 0.0, -1.0];
        let turned = |degrees: f64| {
            let (sin, cos) = degrees.to_radians().sin_cos();
            [-sin, 0.0, -cos]
        };
        assert!(view_changed(None, ahead), "nothing built yet");
        assert!(!view_changed(Some(ahead), turned(9.0)));
        assert!(view_changed(Some(ahead), turned(11.0)));
        assert!(
            !view_changed(Some(ahead), ahead),
            "a new target alone keeps the mesh"
        );
    }
}
