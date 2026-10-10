//! Filled ground meshes for the open tail cells (#458, ADR 0020).
//!
//! Gizmo indicators cannot show a world, so each open L11-L13 cell builds
//! exactly two mesh kinds from the deterministic cell data, nothing else:
//! one filled ground disc following the true-curvature bowl the tail points
//! settle on (so bodies and points always agree), plus one highlight disc
//! per enterable portal in a brighter tone (the highlighted tiles, Q4).
//! Meshes rebuild when the open cell changes and despawn when the dive
//! leaves L11-L13. Everything outside L11-L13 stays gizmo indicators plus
//! the L10 planet meshes under ADR 0016/0020.

use crate::Universe;
use crate::style::to_vec3;
use crate::tokens::{TAIL_GROUND_TONE, TAIL_TILE_TONE};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;
use universe_core::coords::Level;
use universe_core::frame::planet_radius_cells;
use universe_core::r#gen::{MarkerKind, Point};
use universe_core::ground::ground_height;

/// Rings of the ground disc (centre plus this many rings of sectors).
pub(crate) const TAIL_GROUND_RINGS: usize = 8;

/// Sectors per ring of the ground disc (fixed loop, no allocation).
pub(crate) const TAIL_GROUND_SECTORS: usize = 24;

/// Ground disc radius in cell units (the open cell spans `±0.5`).
pub(crate) const TAIL_GROUND_RADIUS: f32 = 0.5;

/// Lift of highlight discs above the ground to avoid z-fighting.
pub(crate) const TAIL_HIGHLIGHT_LIFT: f32 = 0.001;

/// Marks entities owned by [`draw_tail_ground`].
#[derive(Component)]
pub(crate) struct TailGroundMesh;

/// Tracks which open cell the spawned tail meshes were built for.
#[derive(Resource, Default)]
pub(crate) struct TailGroundState {
    /// Open-cell seed and level the current meshes belong to, if any.
    key: Option<(u64, u8)>,
}

/// Builds the filled tail ground disc for `level`.
///
/// A triangle fan over concentric rings in the ground plane, each vertex
/// settled onto the true-curvature bowl through [`ground_height`], with
/// bowl normals. Returns `None` outside L11-L13 or for a degenerate
/// planet radius. Pure in `level`: the same level always yields the same
/// mesh (`E-DET-TIERS` never sees it; meshes feed the GPU only).
#[must_use]
pub(crate) fn tail_ground_mesh(level: Level) -> Option<Mesh> {
    if !(11..=13).contains(&level.get()) {
        return None;
    }
    let radius = planet_radius_cells(level)?;
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cell-unit planet radius narrowed for the GPU, intended"
    )]
    let radius_f = radius as f32;
    let mut positions = Vec::with_capacity(1 + TAIL_GROUND_RINGS * TAIL_GROUND_SECTORS);
    let mut normals = Vec::with_capacity(1 + TAIL_GROUND_RINGS * TAIL_GROUND_SECTORS);
    let mut uvs = Vec::with_capacity(1 + TAIL_GROUND_RINGS * TAIL_GROUND_SECTORS);
    positions.push([0.0, 0.0, 0.0]);
    normals.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 0.5]);
    for ring in 1..=TAIL_GROUND_RINGS {
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: ring index below 8, exactly representable"
        )]
        let circle = TAIL_GROUND_RADIUS * ring as f32 / TAIL_GROUND_RINGS as f32;
        for sector in 0..TAIL_GROUND_SECTORS {
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: sector index below 24, exactly representable"
            )]
            let angle = 2.0 * std::f32::consts::PI * sector as f32 / TAIL_GROUND_SECTORS as f32;
            let (x, z) = (circle * angle.cos(), circle * angle.sin());
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: cell-unit ground narrowed for the GPU, intended"
            )]
            let y = ground_height(level, f64::from(x), f64::from(z)) as f32;
            positions.push([x, y, z]);
            let slope = Vec3::new(x / radius_f, 1.0, z / radius_f);
            let normal = slope.try_normalize().unwrap_or(Vec3::Y);
            normals.push(normal.to_array());
            uvs.push([0.5 + x, 0.5 + z]);
        }
    }
    let mut indices =
        Vec::with_capacity(TAIL_GROUND_SECTORS * (1 + (TAIL_GROUND_RINGS - 1) * 2) * 3);
    let at = |ring: usize, sector: usize| -> u32 {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: disc vertex below 200, always fits u32"
        )]
        let index = if ring == 0 {
            0
        } else {
            1 + (ring - 1) * TAIL_GROUND_SECTORS + sector % TAIL_GROUND_SECTORS
        } as u32;
        index
    };
    for sector in 0..TAIL_GROUND_SECTORS {
        indices.extend([at(0, 0), at(1, sector + 1), at(1, sector)]);
    }
    for ring in 1..TAIL_GROUND_RINGS {
        for sector in 0..TAIL_GROUND_SECTORS {
            indices.extend([
                at(ring, sector),
                at(ring + 1, sector + 1),
                at(ring + 1, sector),
            ]);
            indices.extend([
                at(ring, sector),
                at(ring, sector + 1),
                at(ring + 1, sector + 1),
            ]);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    Some(mesh)
}

/// Builds one highlight disc per enterable portal in `points`.
///
/// Small flat fans at the portal spots, lifted by [`TAIL_HIGHLIGHT_LIFT`]
/// so they read above the ground. Populations carry no highlight: only
/// today's enterable counts glow (Q9). Returns an empty vector when no
/// portal exists.
#[must_use]
pub(crate) fn tail_highlight_meshes(points: &[Point]) -> Vec<Mesh> {
    points
        .iter()
        .filter(|point| point.kind == MarkerKind::Portal)
        .map(|point| {
            let centre = to_vec3(point.position) + Vec3::Y * TAIL_HIGHLIGHT_LIFT;
            let size = 0.03f32;
            let mut positions = Vec::with_capacity(1 + TAIL_GROUND_SECTORS);
            let mut normals = Vec::with_capacity(1 + TAIL_GROUND_SECTORS);
            let mut uvs = Vec::with_capacity(1 + TAIL_GROUND_SECTORS);
            positions.push(centre.to_array());
            normals.push(Vec3::Y.to_array());
            uvs.push([0.5, 0.5]);
            for sector in 0..TAIL_GROUND_SECTORS {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "E-CAST: sector index below 24, exactly representable"
                )]
                let angle = 2.0 * std::f32::consts::PI * sector as f32 / TAIL_GROUND_SECTORS as f32;
                let at = centre + Vec3::new(size * angle.cos(), 0.0, size * angle.sin());
                positions.push(at.to_array());
                normals.push(Vec3::Y.to_array());
                uvs.push([0.5 + angle.cos() * 0.06, 0.5 + angle.sin() * 0.06]);
            }
            let mut indices = Vec::with_capacity(TAIL_GROUND_SECTORS * 3);
            for sector in 0..TAIL_GROUND_SECTORS {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "E-CAST: highlight vertex below 32, always fits u32"
                )]
                let (next, current) = (
                    (1 + (sector + 1) % TAIL_GROUND_SECTORS) as u32,
                    (1 + sector) as u32,
                );
                indices.extend([0, next, current]);
            }
            let mut mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
            mesh.insert_indices(Indices::U32(indices));
            mesh
        })
        .collect()
}

/// Spawns tail ground meshes for the open L11-L13 cell, despawning them
/// elsewhere.
///
/// Rebuilds only when the open cell changes (seed-and-level-keyed);
/// per-frame cost is one level check otherwise. Ground and highlights all
/// carry [`TailGroundMesh`] plus a `Name`, so inspectors and logs can say
/// which entity.
pub(crate) fn draw_tail_ground(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    universe: Res<Universe>,
    mut state: ResMut<TailGroundState>,
    owned: Query<(Entity, &Mesh3d, &MeshMaterial3d<StandardMaterial>), With<TailGroundMesh>>,
) {
    let seed = universe.open_seed();
    let level = universe.level();
    let active = (11..=13).contains(&level.get());
    let key = active.then_some((seed, level.get()));
    if key != state.key {
        for (entity, mesh, material) in &owned {
            commands.entity(entity).despawn();
            meshes.remove(&mesh.0);
            materials.remove(&material.0);
        }
        state.key = None;
    }
    if key.is_none() || state.key == key {
        return;
    }
    let Some(ground) = tail_ground_mesh(level) else {
        return;
    };
    let ground_material = materials.add(StandardMaterial {
        base_color: TAIL_GROUND_TONE,
        unlit: true,
        ..default()
    });
    commands.spawn((
        TailGroundMesh,
        Name::new("tail-ground"),
        Mesh3d(meshes.add(ground)),
        MeshMaterial3d(ground_material),
        Transform::IDENTITY,
    ));
    let tile_material = materials.add(StandardMaterial {
        base_color: TAIL_TILE_TONE,
        unlit: true,
        ..default()
    });
    for mesh in tail_highlight_meshes(&universe.open.points) {
        commands.spawn((
            TailGroundMesh,
            Name::new("tail-tile"),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(tile_material.clone()),
            Transform::IDENTITY,
        ));
    }
    state.key = key;
}

#[cfg(test)]
mod tests {
    use super::*;
    use universe_core::r#gen::{Constraints, Form, Generator};
    use universe_core::tail::TailGenerator;

    /// Tail budget (mirrors the core tail tests: 8 points at L11-L13).
    fn tail_budget() -> Constraints {
        Constraints::new(1.0, 4, 8, [0.5, 0.5, 0.5]).expect("valid test budget")
    }

    fn level(n: u8) -> Level {
        Level::new(n).expect("ladder level")
    }

    #[test]
    fn tail_ground_follows_the_bowl() {
        use bevy::mesh::VertexAttributeValues;
        for n in [11u8, 12, 13] {
            let mesh = tail_ground_mesh(level(n)).expect("tail levels mesh");
            let Some(VertexAttributeValues::Float32x3(verts)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION).cloned()
            else {
                panic!("ground has positions");
            };
            assert_eq!(
                verts.len(),
                1 + TAIL_GROUND_RINGS * TAIL_GROUND_SECTORS,
                "disc resolves its rings"
            );
            for vertex in &verts {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "E-CAST: test ground narrowed for comparison, intended"
                )]
                let want =
                    ground_height(level(n), f64::from(vertex[0]), f64::from(vertex[2])) as f32;
                assert!(
                    (vertex[1] - want).abs() < 1e-6,
                    "L{n} ground leaves the bowl"
                );
                assert!(
                    (vertex[0] * vertex[0] + vertex[2] * vertex[2]).sqrt() <= TAIL_GROUND_RADIUS,
                    "L{n} ground leaves the cell"
                );
            }
        }
        assert!(
            tail_ground_mesh(level(10)).is_none(),
            "planet keeps its body"
        );
        assert!(tail_ground_mesh(level(14)).is_none(), "room keeps outlines");
    }

    #[test]
    fn tail_highlights_mark_enterable_portals_only() {
        let cell = TailGenerator::new(level(13), 8).generate(42, &tail_budget());
        let portals = cell
            .points
            .iter()
            .filter(|point| point.kind == MarkerKind::Portal)
            .count();
        assert_eq!(portals, 4, "half the scatter opens onward");
        let highlights = tail_highlight_meshes(&cell.points);
        assert_eq!(highlights.len(), portals, "one highlight per portal");
        for (mesh, point) in highlights.iter().zip(
            cell.points
                .iter()
                .filter(|point| point.kind == MarkerKind::Portal),
        ) {
            use bevy::mesh::VertexAttributeValues;
            let Some(VertexAttributeValues::Float32x3(verts)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION).cloned()
            else {
                panic!("highlight has positions");
            };
            let centre = to_vec3(point.position);
            let [cx, _, cz] = verts[0];
            assert!((cx - centre.x).abs() < 1e-6 && (cz - centre.z).abs() < 1e-6);
            assert!(
                matches!(point.form, Form::Box { .. }),
                "L13 highlights ride building boxes"
            );
        }
        let room = TailGenerator::new(level(11), 8).generate(7, &tail_budget());
        let room_portals = room
            .points
            .iter()
            .filter(|point| point.kind == MarkerKind::Portal)
            .count();
        assert_eq!(
            tail_highlight_meshes(&room.points).len(),
            room_portals,
            "L11 highlights ride dots alike"
        );
    }

    #[test]
    fn empty_cells_mesh_ground_but_no_highlights() {
        assert!(
            tail_ground_mesh(level(12)).is_some(),
            "surface exists anyway"
        );
        assert!(tail_highlight_meshes(&[]).is_empty());
    }
}
