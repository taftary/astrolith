//! One icosphere surface mesh for the open L10-L13 cell (#458, ADR 0024).
//!
//! Gizmo indicators cannot show a world, so the open surface cell builds
//! exactly one mesh: the core icosphere (`universe_core::icosphere`), split
//! once everywhere by default (Q2) and further near the camera, up to the
//! next level's depth, so the triangles you approach are already the ones
//! you enter (C3). Vertices sit on the relief surface through the open
//! cell's `SurfaceFrame`, computed in `f64` and narrowed to `f32` only here.
//! Enterable clusters of the open cell draw brighter, the entered parent
//! cluster a step dimmer, everything else in the base tone (Q4). Short
//! skirts hang under every triangle edge so neighbours split to different
//! depths never show a gap. The mesh rebuilds when the open cell changes
//! or the camera moves a fraction of its height; it despawns off L10-L13.

use crate::Universe;
use crate::tokens::{SURFACE_OPEN_TONE, SURFACE_PARENT_TONE, SURFACE_TONE};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use universe_core::icosphere::{TriAddr, base_corners, split_corners};
use universe_core::surface::{SurfaceFrame, WINDOW_RADIUS, surface_depth};

/// A triangle splits while its reach over its camera distance exceeds this.
pub(crate) const SPLIT_ANGLE: f64 = 0.5;

/// Most triangles one mesh holds (the breadth-first split stops here).
pub(crate) const TRIANGLE_CAP: usize = 12_000;

/// A window stays resolved while the camera is within this many of its
/// cell's units (past the close distance of about ten, so entries and
/// exits never cross it).
const ZONE_TRIGGER: f64 = 12.0;

/// Skirt length as a share of a triangle's reach.
const SKIRT_SHARE: f64 = 0.15;

/// The camera moves this share of its height before the mesh rebuilds.
const REBUILD_SHARE: f64 = 0.2;

/// Brightness of the second tone of alternating enterable clusters.
const ALTERNATE_DIM: f32 = 0.88;

/// Marks entities owned by [`draw_surface`].
#[derive(Component)]
pub(crate) struct SurfaceMesh;

/// Tracks which open cell and camera the spawned surface mesh was built for.
#[derive(Resource, Default)]
pub(crate) struct SurfaceMeshState {
    /// Open-cell seed and level the current mesh belongs to, if any.
    key: Option<(u64, u8)>,
    /// Camera offset the current mesh was split for.
    camera: Option<[f64; 3]>,
}

/// One drawn triangle: its address and its corners in cell units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DrawnTriangle {
    /// Icosphere address.
    pub(crate) addr: TriAddr,
    /// Corners on the relief surface, in open-cell units.
    pub(crate) corners: [[f64; 3]; 3],
}

/// Deepest split near the camera: the next level's depth (L13: own + 2).
#[must_use]
pub(crate) fn max_depth(frame: &SurfaceFrame) -> u8 {
    frame
        .level()
        .deeper()
        .and_then(surface_depth)
        .unwrap_or(frame.depth().saturating_add(2))
}

/// Centre and reach of three cell-space corners.
fn centre_reach(corners: [[f64; 3]; 3]) -> ([f64; 3], f64) {
    let [a, b, c] = corners;
    let centre = [
        (a[0] + b[0] + c[0]) / 3.0,
        (a[1] + b[1] + c[1]) / 3.0,
        (a[2] + b[2] + c[2]) / 3.0,
    ];
    let reach = [a, b, c]
        .iter()
        .map(|corner| gap(*corner, centre))
        .fold(0.0, f64::max);
    (centre, reach)
}

/// Distance between two points.
fn gap(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// A window kept resolved to its cell's depth while the camera is near it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Zone {
    /// Window centre in open-cell units.
    centre: [f64; 3],
    /// Window radius in open-cell units.
    radius: f64,
    /// Icosphere depth the window resolves to.
    depth: u8,
    /// Camera distance (open-cell units) within which the zone applies.
    trigger: f64,
}

/// Windows to keep resolved for the open cell: every window of every
/// opened level (this cell, its siblings, and theirs up the chain) at its
/// level's depth, and each enterable child at the child's depth. Each applies while the camera is within
/// [`ZONE_TRIGGER`] of its own cell units, so a parent and its child agree
/// on the zones at an entry (scale-invariant, C3).
fn zones(frame: &SurfaceFrame) -> Vec<Zone> {
    use universe_core::surface::cell_span_l10;
    let level = frame.level();
    let Some(span) = cell_span_l10(level) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (opened, centres) in frame.windows() {
        let (Some(depth), Some(opened_span)) = (surface_depth(*opened), cell_span_l10(*opened))
        else {
            continue;
        };
        let scale = opened_span / span;
        for centre in centres {
            out.push(Zone {
                centre: frame.to_cell(*centre),
                radius: WINDOW_RADIUS * scale,
                depth,
                trigger: ZONE_TRIGGER * scale,
            });
        }
    }
    if let (Some(child), Some(ratio)) = (
        level.deeper().and_then(surface_depth),
        universe_core::nest::child_ratio(level),
    ) {
        for portal in frame.portal_positions() {
            out.push(Zone {
                centre: portal,
                radius: WINDOW_RADIUS * ratio,
                depth: child,
                trigger: ZONE_TRIGGER * ratio,
            });
        }
    }
    out
}

/// Triangles of the open surface split for `camera` (cell units).
///
/// Breadth first from the 20 base faces: every triangle splits down to the
/// default depth (1); triangles over a nearby window (the open cell, its
/// opened ancestors, its enterable children) down to that window's depth,
/// so clusters and their tones show; and further while a triangle looks
/// large from the camera ([`SPLIT_ANGLE`]) and stays above [`max_depth`]. The split stops once
/// [`TRIANGLE_CAP`] triangles are queued, so the coarsest rings survive a
/// tight budget. Corners are memoised per unit-sphere direction, so shared
/// corners read the same relief once and land on identical positions.
#[must_use]
pub(crate) fn surface_triangles(frame: &SurfaceFrame, camera: [f64; 3]) -> Vec<DrawnTriangle> {
    let default_depth = universe_core::surface::SURFACE_DEPTH
        .first()
        .copied()
        .unwrap_or(1);
    let deepest = max_depth(frame);
    let active: Vec<Zone> = zones(frame)
        .into_iter()
        .filter(|zone| gap(camera, zone.centre) <= zone.trigger)
        .collect();
    let mut cache: HashMap<[u64; 3], [f64; 3]> = HashMap::new();
    let mut corner = |unit: [f64; 3]| -> [f64; 3] {
        let key = [unit[0].to_bits(), unit[1].to_bits(), unit[2].to_bits()];
        *cache.entry(key).or_insert_with(|| frame.surface_cell(unit))
    };
    let mut queue: VecDeque<(TriAddr, [[f64; 3]; 3])> = VecDeque::new();
    for face in 0..20u8 {
        if let (Some(addr), Some(units)) = (TriAddr::root(face), base_corners(face)) {
            queue.push_back((addr, units));
        }
    }
    let mut out = Vec::new();
    while let Some((addr, units)) = queue.pop_front() {
        let corners = [corner(units[0]), corner(units[1]), corner(units[2])];
        let (centre, reach) = centre_reach(corners);
        let distance = gap(camera, centre).max(1e-12);
        let zoned = active.iter().any(|zone| {
            addr.depth() < zone.depth && gap(centre, zone.centre) <= zone.radius * 1.25 + reach
        });
        let wants = addr.depth() < default_depth
            || zoned
            || (addr.depth() < deepest && reach / distance > SPLIT_ANGLE);
        let room = out.len() + queue.len() + 4 <= TRIANGLE_CAP;
        match addr.children() {
            Some(kids) if wants && room => {
                for (kid, kid_units) in kids.into_iter().zip(split_corners(units)) {
                    queue.push_back((kid, kid_units));
                }
            }
            _ => out.push(DrawnTriangle { addr, corners }),
        }
    }
    out
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

/// Builds the open cell's surface mesh for `camera` (cell units), with its
/// vertices relative to `anchor` (an [`crate::style::Anchored`] entity
/// places the mesh back at `anchor` each frame).
///
/// One flat-coloured triangle list: three vertices per drawn triangle with
/// its tone as vertex colour, plus a skirt quad under each edge, pulled
/// toward the planet centre by [`SKIRT_SHARE`] of the triangle's reach.
/// Returns `None` when nothing is drawn.
#[must_use]
pub(crate) fn surface_mesh(
    frame: &SurfaceFrame,
    camera: [f64; 3],
    anchor: [f64; 3],
) -> Option<Mesh> {
    let triangles = surface_triangles(frame, camera);
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
    let mut push = |point: [f64; 3], tone: LinearRgba| {
        // Vertices sit relative to `anchor` (the camera at build time), so
        // ground near the camera keeps full precision (ADR 0026).
        positions.push(narrow([
            point[0] - anchor[0],
            point[1] - anchor[1],
            point[2] - anchor[2],
        ]));
        colors.push(tone.to_f32_array());
        // Outward from the planet centre: the standard pipeline wants a
        // normal even when unlit.
        let length = gap(centre, point).max(1e-300);
        normals.push(narrow([
            (point[0] - centre[0]) / length,
            (point[1] - centre[1]) / length,
            (point[2] - centre[2]) / length,
        ]));
    };
    for triangle in &triangles {
        let tone = tone_for(triangle.addr, frame, &open, &parent);
        let [a, b, c] = triangle.corners;
        push(a, tone);
        push(b, tone);
        push(c, tone);
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
            push(from, tone);
            push(to, tone);
            push(low_to, tone);
            push(from, tone);
            push(low_to, tone);
            push(low_from, tone);
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

/// Spawns the open L10-L13 surface mesh, despawning it elsewhere.
///
/// Rebuilds when the open cell changes or the camera has moved more than
/// [`REBUILD_SHARE`] of its height since the last split; per-frame cost is
/// one height probe otherwise. The mesh carries [`SurfaceMesh`] plus a
/// `Name`, so inspectors and logs can say which entity.
pub(crate) fn draw_surface(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    universe: Res<Universe>,
    mut state: ResMut<SurfaceMeshState>,
    owned: Query<(Entity, &Mesh3d, &MeshMaterial3d<StandardMaterial>), With<SurfaceMesh>>,
) {
    let frame = universe.frame.as_ref();
    let key = frame.map(|frame| (universe.open_seed(), frame.level().get()));
    let camera = universe.path.offset();
    let moved = match (frame, state.camera) {
        (Some(frame), Some(last)) => gap(camera, last) > REBUILD_SHARE * height_above(frame, last),
        _ => true,
    };
    if key != state.key || moved {
        for (entity, mesh, material) in &owned {
            commands.entity(entity).despawn();
            meshes.remove(&mesh.0);
            materials.remove(&material.0);
        }
        state.key = None;
        state.camera = None;
    }
    let Some(frame) = frame else {
        return;
    };
    if state.key == key {
        return;
    }
    let Some(mesh) = surface_mesh(frame, camera, camera) else {
        return;
    };
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        SurfaceMesh,
        Name::new("surface"),
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material),
        Transform::IDENTITY,
        crate::style::Anchored(camera),
    ));
    state.key = key;
    state.camera = Some(camera);
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
    fn far_planet_is_the_icosphere_split_once() {
        let triangles = surface_triangles(&frame(10), [0.0, 0.0, 50.0]);
        assert_eq!(triangles.len(), 80, "Q2: 80 triangles by default");
        assert!(triangles.iter().all(|t| t.addr.depth() == 1));
    }

    #[test]
    fn vertices_sit_on_the_relief_surface() {
        for n in 10..=13u8 {
            let surface = frame(n);
            let camera = [0.0, 0.2, 0.0];
            for triangle in surface_triangles(&surface, camera).iter().take(400) {
                let units = universe_core::icosphere::tri_corners(triangle.addr).expect("corners");
                for (unit, corner) in units.iter().zip(triangle.corners) {
                    assert_eq!(
                        surface.surface_cell(*unit),
                        corner,
                        "L{n} vertex off the relief"
                    );
                }
            }
        }
    }

    #[test]
    fn entry_keeps_the_same_triangles_on_both_sides() {
        // C3: the split is angular, so the parent and child frames (one
        // similarity apart) draw the same triangles at the entry rest.
        for n in 10..=12u8 {
            let parent = frame(n);
            let child = frame(n + 1);
            let ground = child.ground_height(0.1, -0.05);
            let camera_child = [0.1, ground + 3.0, -0.05];
            let camera_parent = parent.to_cell(child.to_planet(camera_child));
            let before: HashSet<TriAddr> = surface_triangles(&parent, camera_parent)
                .iter()
                .map(|t| t.addr)
                .collect();
            let after: HashSet<TriAddr> = surface_triangles(&child, camera_child)
                .iter()
                .map(|t| t.addr)
                .collect();
            assert!(before.len() <= TRIANGLE_CAP && after.len() <= TRIANGLE_CAP);
            let differ = before.symmetric_difference(&after).count();
            assert!(
                differ * 100 <= before.len(),
                "L{n} -> L{}: {differ} of {} triangles change at entry",
                n + 1,
                before.len()
            );
        }
    }

    #[test]
    fn enterable_clusters_draw_brighter() {
        let surface = frame(11);
        let mesh = surface_mesh(&surface, [0.0, 0.3, 0.0], [0.0; 3]).expect("surface mesh");
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
}
