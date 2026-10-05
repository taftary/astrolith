//! Planet meshes for the open L10 terminal cell (#156, ADR 0014).
//!
//! Gizmo indicators cannot show a world: shores, climate zones, moons, and
//! air need shaded bodies. This module builds exactly three mesh kinds from
//! the deterministic L10 cell data, nothing else: one vertex-colored body
//! per open L10 cell (biome colors sampled from the same core field as the
//! indicator points, so bodies and points always agree), one transparent
//! atmosphere rim shell, and one small body per moon. Meshes rebuild when
//! the open cell changes and despawn when the dive leaves L10. Everything
//! outside L10 stays gizmo indicators under ADR 0002.

use crate::Universe;
use crate::style::to_vec3;
use bevy::mesh::{
    Mesh, Mesh3d, MeshBuilder, primitives::SphereKind, primitives::SphereMeshBuilder,
};
use bevy::prelude::*;
use universe_core::r#gen::{AirKind, Point};
use universe_core::terrain::{
    Biome, BodyRecipe, PLANET_RADIUS_CELL, RELIEF_RANGE_CELL, face_uv_for, surface_info,
};

/// Icosphere subdivisions for planet bodies (252 vertices, even coverage).
pub(crate) const BODY_SUBDIVISIONS: u32 = 4;

/// Icosphere subdivisions for moons (42 vertices; moons are dots with shading).
pub(crate) const MOON_SUBDIVISIONS: u32 = 1;

/// Rim shell scale over the body radius (a thin glowing edge, not a second world).
pub(crate) const RIM_SCALE: f32 = 1.06;

/// Moon body radius in cell units (indicator moons draw at 0.005).
pub(crate) const MOON_RADIUS_CELL: f32 = 0.008;

/// Marks entities owned by [`draw_planets`].
#[derive(Component)]
pub(crate) struct PlanetMesh;

/// Tracks which open cell the spawned planet meshes were built for.
#[derive(Resource, Default)]
pub(crate) struct PlanetMeshState {
    /// Open-cell seed the current meshes belong to, if any.
    seed: Option<u64>,
}

/// Surface color for one climate zone (flat unlit PBR base colors).
#[must_use]
pub(crate) fn biome_color(biome: Biome) -> Color {
    match biome {
        Biome::Ocean => Color::srgb(0.05, 0.20, 0.55),
        Biome::Coast => Color::srgb(0.15, 0.45, 0.60),
        Biome::Desert => Color::srgb(0.76, 0.62, 0.35),
        Biome::Grassland => Color::srgb(0.35, 0.55, 0.25),
        Biome::Forest => Color::srgb(0.12, 0.35, 0.15),
        Biome::Tundra => Color::srgb(0.55, 0.55, 0.50),
        Biome::IceCap => Color::srgb(0.90, 0.93, 0.96),
        Biome::Mountain => Color::srgb(0.45, 0.40, 0.38),
    }
}

/// Atmosphere rim treatment for one air profile: shell opacity plus tint.
///
/// `None` means no appreciable atmosphere, so no rim mesh at all.
#[must_use]
pub(crate) fn rim_params(air: AirKind) -> Option<(f32, Color)> {
    match air {
        AirKind::None => None,
        AirKind::Thin => Some((0.15, Color::srgba(0.55, 0.70, 0.95, 0.15))),
        AirKind::Earth => Some((0.30, Color::srgba(0.30, 0.55, 1.00, 0.30))),
    }
}

/// Cell recipe rebuilt from one surface sample (cell-uniform fields repeat
/// on every L10 surface point, so the first one names the body).
#[must_use]
pub(crate) fn body_for(points: &[Point]) -> Option<BodyRecipe> {
    let surface = points.iter().find_map(|point| point.surface)?;
    Some(BodyRecipe {
        earth_like: surface.air == AirKind::Earth,
        flattening: surface.flattening,
        tilt_deg: surface.tilt_deg,
        spin_hours: surface.spin_hours,
        air: surface.air,
    })
}

/// Builds the vertex-colored planet body mesh for L10 surface points.
///
/// Vertices sample the same core field as the indicator points through
/// [`face_uv_for`], displaced by relief over [`PLANET_RADIUS_CELL`] with the
/// polar squash applied. Returns `None` when no surface point exists.
#[must_use]
pub(crate) fn planet_mesh_for(seed: u64, points: &[Point], body: &BodyRecipe) -> Option<Mesh> {
    if points.iter().all(|point| point.surface.is_none()) {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cell-unit radius narrowed for the GPU, intended"
    )]
    let base = PLANET_RADIUS_CELL as f32;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cell-unit relief narrowed for the GPU, intended"
    )]
    let relief = RELIEF_RANGE_CELL as f32;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: flattening narrowed for the GPU, intended"
    )]
    let squash = 1.0 - body.flattening as f32;
    let mut mesh = SphereMeshBuilder::new(
        base,
        SphereKind::Ico {
            subdivisions: BODY_SUBDIVISIONS,
        },
    )
    .build();
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION).cloned()
    else {
        return None;
    };
    let mut displaced = Vec::with_capacity(positions.len());
    let mut colors = Vec::with_capacity(positions.len());
    for vertex in &positions {
        let direction = [
            f64::from(vertex[0]),
            f64::from(vertex[1]),
            f64::from(vertex[2]),
        ];
        let length = (direction[0] * direction[0]
            + direction[1] * direction[1]
            + direction[2] * direction[2])
            .sqrt();
        if length <= 0.0 {
            displaced.push([0.0, 0.0, 0.0]);
            colors.push([0.0, 0.0, 0.0, 1.0]);
            continue;
        }
        let unit = [
            direction[0] / length,
            direction[1] / length,
            direction[2] / length,
        ];
        let (face, u, v) = face_uv_for(unit);
        let info = surface_info(seed, face, u, v, body);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: relief height narrowed for the GPU, intended"
        )]
        let height = info.height as f32;
        let radius = base + (height - 0.5) * 2.0 * relief;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: unit direction narrowed for the GPU, intended"
        )]
        let unit_f = [unit[0] as f32, unit[1] as f32, unit[2] as f32];
        displaced.push([
            unit_f[0] * radius,
            unit_f[1] * radius * squash,
            unit_f[2] * radius,
        ]);
        let linear = biome_color(info.biome).to_linear();
        colors.push([linear.red, linear.green, linear.blue, 1.0]);
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, displaced);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    Some(mesh)
}

/// Spawns planet meshes for the open L10 cell, despawning them elsewhere.
///
/// Rebuilds only when the open cell changes (seed-keyed); per-frame cost is
/// one level check otherwise. Body, rim, and moons all carry [`PlanetMesh`]
/// plus a `Name`, so inspectors and logs can say which entity.
pub(crate) fn draw_planets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    universe: Res<Universe>,
    mut state: ResMut<PlanetMeshState>,
    owned: Query<(Entity, &Mesh3d, &MeshMaterial3d<StandardMaterial>), With<PlanetMesh>>,
) {
    let seed = universe.open_seed();
    let active = universe.level().get() == 10;
    if !active || state.seed != Some(seed) {
        for (entity, mesh, material) in &owned {
            commands.entity(entity).despawn();
            meshes.remove(&mesh.0);
            materials.remove(&material.0);
        }
        state.seed = None;
    }
    if !active || state.seed == Some(seed) {
        return;
    }
    let Some(body) = body_for(&universe.open.points) else {
        return;
    };
    let body_handle =
        planet_mesh_for(seed, &universe.open.points, &body).map(|mesh| meshes.add(mesh));
    if let Some(handle) = body_handle {
        let material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            ..default()
        });
        commands.spawn((
            PlanetMesh,
            Name::new("planet-body"),
            Mesh3d(handle),
            MeshMaterial3d(material),
            Transform::IDENTITY,
        ));
    }
    if let Some((_, tint)) = rim_params(body.air) {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: cell-unit radius narrowed for the GPU, intended"
        )]
        let rim_radius = PLANET_RADIUS_CELL as f32 * RIM_SCALE;
        let rim = SphereMeshBuilder::new(
            rim_radius,
            SphereKind::Ico {
                subdivisions: MOON_SUBDIVISIONS,
            },
        )
        .build();
        let material = materials.add(StandardMaterial {
            base_color: tint,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });
        commands.spawn((
            PlanetMesh,
            Name::new("planet-rim"),
            Mesh3d(meshes.add(rim)),
            MeshMaterial3d(material),
            Transform::IDENTITY,
        ));
    }
    let gray = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.55, 0.58),
        unlit: true,
        ..default()
    });
    for point in &universe.open.points {
        if point.moon.is_none() {
            continue;
        }
        let moon = SphereMeshBuilder::new(
            MOON_RADIUS_CELL,
            SphereKind::Ico {
                subdivisions: MOON_SUBDIVISIONS,
            },
        )
        .build();
        commands.spawn((
            PlanetMesh,
            Name::new("planet-moon"),
            Mesh3d(meshes.add(moon)),
            MeshMaterial3d(gray.clone()),
            Transform::from_translation(to_vec3(point.position)),
        ));
    }
    state.seed = Some(seed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use universe_core::r#gen::Generator;
    use universe_core::terrain::TerrainSampler;

    /// L10 demo budget (mirrors the core sampler tests).
    fn demo_budget() -> universe_core::r#gen::Constraints {
        universe_core::r#gen::Constraints::new(1.0, 8, 24, [0.5, 0.5, 0.5]).expect("valid")
    }

    #[test]
    fn every_biome_has_a_distinct_color() {
        use universe_core::terrain::Biome;
        let biomes = [
            Biome::Ocean,
            Biome::Coast,
            Biome::Desert,
            Biome::Grassland,
            Biome::Forest,
            Biome::Tundra,
            Biome::IceCap,
            Biome::Mountain,
        ];
        let mut seen = std::collections::HashSet::new();
        for biome in biomes {
            let linear = biome_color(biome).to_linear();
            let key = (
                linear.red.to_bits(),
                linear.green.to_bits(),
                linear.blue.to_bits(),
            );
            assert!(seen.insert(key), "biome colors must differ: {biome:?}");
        }
    }

    #[test]
    fn rim_covers_earth_and_thin_only() {
        assert!(rim_params(AirKind::None).is_none());
        let (thin_scale, _) = rim_params(AirKind::Thin).expect("thin air rims");
        let (earth_scale, _) = rim_params(AirKind::Earth).expect("Earth rims");
        assert!(earth_scale > thin_scale, "Earth air reads denser");
    }

    #[test]
    fn body_mesh_carries_painted_vertices() {
        let cell = TerrainSampler::home().generate(7, &demo_budget());
        let body = body_for(&cell.points).expect("home cell names its body");
        let mesh = planet_mesh_for(7, &cell.points, &body).expect("home cell meshes");
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("body has positions");
        let colors = mesh
            .attribute(Mesh::ATTRIBUTE_COLOR)
            .expect("body is painted");
        let (
            bevy::mesh::VertexAttributeValues::Float32x3(verts),
            bevy::mesh::VertexAttributeValues::Float32x4(paint),
        ) = (positions, colors)
        else {
            panic!("body attributes have the painted-mesh shape");
        };
        assert_eq!(verts.len(), paint.len());
        assert!(
            verts.len() > 200,
            "icosphere resolves shores: {}",
            verts.len()
        );
        assert!(
            paint.iter().any(|pixel| pixel[2] > pixel[0]),
            "Earth reads blue somewhere"
        );
    }

    #[test]
    fn empty_cells_mesh_nothing() {
        assert!(body_for(&[]).is_none());
        let body = BodyRecipe {
            earth_like: true,
            flattening: universe_core::terrain::EARTH_FLATTENING,
            tilt_deg: universe_core::terrain::EARTH_TILT_DEG,
            spin_hours: universe_core::terrain::EARTH_SPIN_HOURS,
            air: AirKind::Earth,
        };
        assert!(planet_mesh_for(7, &[], &body).is_none());
    }
}
