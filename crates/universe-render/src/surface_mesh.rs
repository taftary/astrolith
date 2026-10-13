//! One icosphere surface mesh for the open L10-L13 cell (#458, ADR 0024).
//!
//! Gizmo indicators cannot show a world, so the open surface cell builds
//! exactly one mesh: the core icosphere (`universe_core::icosphere`), split
//! once everywhere by default (Q2) and further near the camera, up to the
//! next level's depth, so the triangles you approach are already the ones
//! you enter (C3). Vertices sit on the planet's flat faces through the open
//! cell's `SurfaceFrame` (#593: finer splits lie flat inside the 80 faces),
//! computed in `f64` and narrowed to `f32` only here, with flat normals.
//! Enterable clusters of the open cell draw brighter, the entered parent
//! cluster a step dimmer, everything else in the base tone (Q4). Every drawn
//! triangle edge shows as a thin marker line (a retained gizmo built with
//! the mesh, #593). Short skirts hang under every triangle edge so
//! neighbours split to different depths never show a gap. The mesh rebuilds
//! when the open cell changes or the camera moves a fraction of its height;
//! it despawns off L10-L13.

use crate::Universe;
use crate::surface_split::{
    DrawnTriangle, SplitTarget, SplitView, centre_reach, gap, surface_triangles,
};
use crate::tokens::{
    SURFACE_EDGE_TINT, SURFACE_EDGE_WIDTH_PX, SURFACE_OPEN_TONE, SURFACE_PARENT_TONE, SURFACE_TONE,
};
use bevy::asset::RenderAssetUsages;
use bevy::camera::Projection;
use bevy::gizmos::{GizmoAsset, config::GizmoLineConfig, retained::Gizmo};
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use universe_core::icosphere::TriAddr;
use universe_core::surface::SurfaceFrame;

/// Skirt length as a share of a triangle's reach.
const SKIRT_SHARE: f64 = 0.15;

/// The camera moves this share of its height before the mesh rebuilds.
const REBUILD_SHARE: f64 = 0.2;

/// Brightness of the second tone of alternating enterable clusters.
const ALTERNATE_DIM: f32 = 0.88;

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
    /// Open-cell seed, level, and approached planet marker (#603) the
    /// current mesh belongs to, if any.
    key: Option<(u64, u8, Option<u32>)>,
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
    /// Target marker the current mesh was split for (#598).
    target: Option<u32>,
}

/// The camera turns this far (radians, about 10 degrees) before the mesh
/// rebuilds for the new view (#598, Q6 A).
pub(crate) const REBUILD_TURN: f64 = 10.0 * std::f64::consts::PI / 180.0;

/// Whether the mesh built for (`last_forward`, `last_target`) needs a
/// rebuild for (`forward`, `target`): a turn past [`REBUILD_TURN`] or a
/// different target (#598, Q6 A).
#[must_use]
pub(crate) fn view_changed(
    last_forward: Option<[f64; 3]>,
    last_target: Option<u32>,
    forward: [f64; 3],
    target: Option<u32>,
) -> bool {
    let Some(last) = last_forward else {
        return true;
    };
    let along = (last[0] * forward[0] + last[1] * forward[1] + last[2] * forward[2])
        / (gap(last, [0.0; 3]) * gap(forward, [0.0; 3])).max(1e-300);
    along.clamp(-1.0, 1.0).acos() > REBUILD_TURN || last_target != target
}

/// Tone of a drawn triangle under the open cell's clusters (Q4).
fn tone_for(
    addr: TriAddr,
    frame: &SurfaceFrame,
    open: &HashMap<TriAddr, bool>,
    parent: &HashSet<TriAddr>,
) -> LinearRgba {
    if let Some(alternate) = addr
        .ancestor_at(frame.depth())
        .and_then(|tile| open.get(&tile))
    {
        let tone = SURFACE_OPEN_TONE.to_linear();
        return if *alternate {
            tone * ALTERNATE_DIM
        } else {
            tone
        };
    }
    if let Some(depth) = frame.parent_depth()
        && addr
            .ancestor_at(depth)
            .is_some_and(|tile| parent.contains(&tile))
    {
        return SURFACE_PARENT_TONE.to_linear();
    }
    SURFACE_TONE.to_linear()
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
    edge_lines(&surface_edges(triangles), anchor, 1.0)
}

/// `edges` as one retained gizmo relative to `anchor`, at `fade` strength
/// (#603: the lines fade with the surface).
#[must_use]
pub(crate) fn edge_lines(
    edges: &[([f64; 3], [f64; 3])],
    anchor: [f64; 3],
    fade: f64,
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
    let tint = SURFACE_EDGE_TINT.with_alpha(fade.clamp(0.0, 1.0) as f32);
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
pub(crate) fn surface_mesh_of(
    frame: &SurfaceFrame,
    triangles: &[DrawnTriangle],
    anchor: [f64; 3],
) -> Option<Mesh> {
    if triangles.is_empty() {
        return None;
    }
    let open: HashMap<TriAddr, bool> = frame
        .clusters()
        .into_iter()
        .filter(|cluster| cluster.enterable)
        .flat_map(|cluster| {
            let alternate = cluster.anchor.stream_seed(frame.planet_seed()) & 1 == 1;
            cluster.tiles.into_iter().map(move |tile| (tile, alternate))
        })
        .collect();
    let parent: HashSet<TriAddr> = frame.parent_tiles().iter().copied().collect();
    let centre = frame.to_cell([0.0; 3]);
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
        let tone = tone_for(triangle.addr, frame, &open, &parent);
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

/// Height of `camera` above the open surface (cell units, at least tiny).
fn height_above(frame: &SurfaceFrame, camera: [f64; 3]) -> f64 {
    let height = if frame.level().get() == 10 {
        let reach = gap(camera, [0.0; 3]);
        reach - gap(frame.surface_cell(camera), [0.0; 3])
    } else {
        camera[1] - frame.ground_height(camera[0], camera[2])
    };
    if height.is_finite() {
        height.abs().max(1e-9)
    } else {
        1e-9
    }
}

/// Spawns the surface mesh and its edge lines for the open L10-L13 cell,
/// or for the planet being approached from the level above (#603),
/// despawning them elsewhere. The surface draws at the host's fade.
///
/// Rebuilds when the open cell changes, the camera has moved more than
/// [`REBUILD_SHARE`] of its height, turned past [`REBUILD_TURN`], or the
/// target changed since the last split (#598); per-frame cost is one
/// height probe otherwise. The split follows the camera's view cone and
/// the target ([`crate::surface_split`]). The mesh carries [`SurfaceMesh`] and the
/// lines [`SurfaceEdges`], each with a `Name`, so inspectors and logs can
/// say which entity.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn draw_surface(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut gizmos: ResMut<Assets<GizmoAsset>>,
    universe: Res<Universe>,
    nav: Res<crate::input::Navigation>,
    mut state: ResMut<SurfaceMeshState>,
    owned: Query<
        (
            Entity,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &mut Visibility,
        ),
        With<SurfaceMesh>,
    >,
    lines: Query<(Entity, &Gizmo), With<SurfaceEdges>>,
    cameras: Query<&Projection, With<Camera3d>>,
    windows: Query<&Window>,
) {
    let host = crate::planet_approach::surface_host(&universe, &nav);
    let key = host
        .as_ref()
        .map(|host| (universe.open_seed(), host.frame.level().get(), host.marker));
    let camera = universe.path.offset();
    let moved = match (host.as_ref(), state.camera) {
        (Some(host), Some(last)) => {
            gap(host.to_surface(camera), host.to_surface(last))
                > REBUILD_SHARE * height_above(&host.frame, host.to_surface(last))
        }
        _ => true,
    };
    let turned = view_changed(state.forward, state.target, nav.forward, nav.target);
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
                    faded(&mut material, host.fade);
                }
                *visibility = if host.fade > 0.0 {
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
                    *asset = edge_lines(&state.edges, state.edges_anchor, host.fade);
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
    let aspect = windows.iter().next().map_or(16.0 / 9.0, |window| {
        f64::from(window.width() / window.height().max(1.0))
    });
    let view = fov.map_or(SplitView::all_around(inside), |fov| {
        SplitView::camera(inside, nav.forward, fov, aspect)
    });
    let target = host.target.map(|position| SplitTarget { position });
    let triangles = surface_triangles(frame, &view, target);
    let Some(mesh) = surface_mesh_of(frame, &triangles, inside) else {
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
    let visibility = if host.fade > 0.0 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    commands.spawn((
        SurfaceMesh,
        Name::new("surface"),
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(material)),
        scale,
        visibility,
        crate::style::Anchored(camera),
    ));
    let edges = surface_edges(&triangles);
    commands.spawn((
        SurfaceEdges,
        Name::new("surface-edges"),
        Gizmo {
            handle: gizmos.add(edge_lines(&edges, inside, host.fade)),
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
    state.target = nav.target;
    state.fade = host.fade;
    state.edges_fade = host.fade;
    state.edges = edges;
    state.edges_anchor = inside;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The demo journey's surface frame at level `n` (L10-L13).
    fn frame(n: u8) -> SurfaceFrame {
        let chain = universe_core::nest::autopilot_path(universe_core::nav::DEMO_SEED);
        let named = chain
            .get(..usize::from(n) - 1)
            .expect("journey reaches the surface");
        SurfaceFrame::for_path(universe_core::nav::DEMO_SEED, named).expect("surface frame")
    }

    #[test]
    fn vertices_sit_on_the_flat_faces() {
        for n in 10..=13u8 {
            let surface = frame(n);
            let camera = [0.0, 0.2, 0.0];
            for triangle in surface_triangles(&surface, &SplitView::all_around(camera), None)
                .iter()
                .take(400)
            {
                let units = universe_core::icosphere::tri_corners(triangle.addr).expect("corners");
                for (unit, corner) in units.iter().zip(triangle.corners) {
                    assert_eq!(
                        surface.surface_cell(*unit),
                        corner,
                        "L{n} vertex off the surface"
                    );
                }
            }
        }
    }

    #[test]
    fn enterable_clusters_draw_brighter() {
        let surface = frame(11);
        let triangles = surface_triangles(&surface, &SplitView::all_around([0.0, 0.3, 0.0]), None);
        let mesh = surface_mesh_of(&surface, &triangles, [0.0; 3]).expect("surface mesh");
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("surface carries vertex colours");
        };
        let open = SURFACE_OPEN_TONE.to_linear().to_f32_array();
        let base = SURFACE_TONE.to_linear().to_f32_array();
        assert!(colors.contains(&open), "enterable tiles glow");
        assert!(colors.contains(&base), "the rest stays in base tone");
    }

    #[test]
    fn every_drawn_edge_shows_once_and_follows_the_splits() {
        let surface = frame(10);
        let far = surface_triangles(&surface, &SplitView::all_around([0.0, 0.0, 50.0]), None);
        // 80 triangles of a closed surface: 120 shared edges.
        assert_eq!(surface_edges(&far).len(), 120, "each edge once");
        let portal = surface.portal_positions()[0];
        let camera = [portal[0] * 1.2, portal[1] * 1.2, portal[2] * 1.2];
        let near = surface_triangles(&surface, &SplitView::all_around(camera), None);
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
        let surface = frame(10);
        let triangles = surface_triangles(&surface, &SplitView::all_around([0.0, 0.0, 50.0]), None);
        let mesh = surface_mesh_of(&surface, &triangles, [0.0; 3]).expect("surface mesh");
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
    fn a_turn_past_ten_degrees_or_a_new_target_asks_for_a_rebuild() {
        let ahead = [0.0, 0.0, -1.0];
        let turned = |degrees: f64| {
            let (sin, cos) = degrees.to_radians().sin_cos();
            [-sin, 0.0, -cos]
        };
        assert!(view_changed(None, None, ahead, None), "nothing built yet");
        assert!(!view_changed(Some(ahead), Some(3), turned(9.0), Some(3)));
        assert!(view_changed(Some(ahead), Some(3), turned(11.0), Some(3)));
        assert!(view_changed(Some(ahead), Some(3), ahead, Some(4)));
        assert!(view_changed(Some(ahead), None, ahead, Some(4)));
    }
}
