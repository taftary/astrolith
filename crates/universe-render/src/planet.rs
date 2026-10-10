//! Bare planet meshes for the open L10 cell (#375, ADR 0016).
//!
//! Gizmo indicators cannot show a world, so the open L10 cell builds exactly
//! two mesh kinds from the deterministic cell data, nothing else: one
//! flat-tone body (displaced by the same core relief field as the indicator
//! points, so bodies and points always agree) and one small body per moon.
//! Meshes rebuild when the open cell changes and despawn when the dive
//! leaves L10. Everything outside L10 stays gizmo indicators under ADR 0002.

use crate::Universe;
use crate::forms::unit;
use crate::input::Navigation;
use crate::style::{point_color_for_level, scaled, to_vec3};
use bevy::math::Isometry3d;
use bevy::mesh::{
    Mesh, Mesh3d, MeshBuilder, primitives::SphereKind, primitives::SphereMeshBuilder,
};
use bevy::prelude::*;
use universe_core::frame::{angular_radius, children_brightness};
use universe_core::r#gen::{MarkerKind, Point};
use universe_core::terrain::{
    BodyRecipe, EARTH_RADIUS_KM, PLANET_RADIUS_CELL, RELIEF_RANGE_CELL, face_uv_for, surface_info,
};

/// Icosphere subdivisions for planet bodies (252 vertices, even coverage).
pub(crate) const BODY_SUBDIVISIONS: u32 = 4;

/// Icosphere subdivisions for moons (42 vertices; moons are dots with shading).
pub(crate) const MOON_SUBDIVISIONS: u32 = 1;

/// Moon body radius in cell units (indicator moons draw at 0.005).
pub(crate) const MOON_RADIUS_CELL: f32 = 0.008;

/// Flat tone of the bare planet body and its moons (unlit base color, #375).
pub(crate) const PLANET_TONE: Color = Color::srgb(0.45, 0.42, 0.38);

/// Brightness steps of the three air-rim circles, innermost first (#394).
const AIR_RIM_BRIGHTNESS: [f32; 3] = [0.6, 0.4, 0.2];

/// Marks entities owned by [`draw_planets`].
#[derive(Component)]
pub(crate) struct PlanetMesh;

/// Tracks which open cell the spawned planet meshes were built for.
#[derive(Resource, Default)]
pub(crate) struct PlanetMeshState {
    /// Open-cell seed the current meshes belong to, if any.
    seed: Option<u64>,
}

/// Cell recipe rebuilt from one surface sample (cell-uniform fields repeat
/// on every L10 surface point, so the first one names the body).
#[must_use]
pub(crate) fn body_for(points: &[Point]) -> Option<BodyRecipe> {
    let surface = points.iter().find_map(|point| point.surface)?;
    Some(BodyRecipe {
        flattening: surface.flattening,
        tilt_deg: surface.tilt_deg,
        spin_hours: surface.spin_hours,
        radius_earth: surface.radius_earth,
        air: surface.air,
    })
}

/// Builds the displaced bare-planet body mesh for L10 surface points.
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
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, displaced);
    Some(mesh)
}

/// Spawns planet meshes for the open L10 cell, despawning them elsewhere.
///
/// Rebuilds only when the open cell changes (seed-keyed); per-frame cost is
/// one level check otherwise. Body and moons all carry [`PlanetMesh`]
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
            base_color: PLANET_TONE,
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

/// Draws the planet's air as three concentric camera-facing circles (#394).
///
/// Indicators only: no mesh, no rim entity (the `planet-rim` count stays
/// zero). The circles sit just outside the body at `R + t·k/3` with the air
/// tint fading outward; airless worlds draw nothing. Near the targeted
/// region's open threshold the rim fades out while the limb preview fades
/// in (#430), so the entry swaps no full-weight element. Allocates nothing
/// per frame (`E-HOT-NOALLOC`): three gizmo circles, no buffers.
pub(crate) fn draw_air_rim(mut gizmos: Gizmos, universe: Res<Universe>, nav: Res<Navigation>) {
    if universe.level().get() != 10 {
        return;
    }
    let Some(body) = body_for(&universe.open.points) else {
        return;
    };
    let Some(air) = body.air else {
        return;
    };
    if body.radius_earth <= 0.0 || !body.radius_earth.is_finite() {
        return;
    }
    // Air thickness in cell units: the thickness share of the planet radius
    // times the body radius on screen.
    let planet_radius_km = body.radius_earth * EARTH_RADIUS_KM;
    if !planet_radius_km.is_finite() || planet_radius_km <= 0.0 {
        return;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cell-unit air thickness narrowed for the GPU, intended"
    )]
    let thickness = (air.thickness_km / planet_radius_km * PLANET_RADIUS_CELL) as f32;
    if !thickness.is_finite() || thickness <= 0.0 {
        return;
    }
    let camera = universe.path.offset();
    let camera_length =
        (camera[0] * camera[0] + camera[1] * camera[1] + camera[2] * camera[2]).sqrt();
    if !camera_length.is_finite() || camera_length <= 0.0 {
        return;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: view direction narrowed for the GPU, intended"
    )]
    let view_x = (camera[0] / camera_length) as f32;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: view direction narrowed for the GPU, intended"
    )]
    let view_y = (camera[1] / camera_length) as f32;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: view direction narrowed for the GPU, intended"
    )]
    let view_z = (camera[2] / camera_length) as f32;
    let view = Vec3::new(view_x, view_y, view_z);
    let rotation = Quat::from_rotation_arc(Vec3::Z, view);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: air tint narrowed for the GPU, intended"
    )]
    let tint_r = air.tint[0] as f32;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: air tint narrowed for the GPU, intended"
    )]
    let tint_g = air.tint[1] as f32;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: air tint narrowed for the GPU, intended"
    )]
    let tint_b = air.tint[2] as f32;
    let tint = Color::srgb(tint_r, tint_g, tint_b);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cell-unit body radius narrowed for the GPU, intended"
    )]
    let base = PLANET_RADIUS_CELL as f32;
    // Fade the rim out as the targeted region opens (#430): the kept limb
    // and rim below take over at the same brightness, so nothing pops.
    let mut fade = 1.0f32;
    if let Some(target) = nav.target
        && let Some(point) = universe.open.points.get(target as usize)
        && point.kind == MarkerKind::Portal
        && let universe_core::r#gen::Form::Patch { .. } = point.form
    {
        let camera = universe.path.offset();
        let at = point.portal_position();
        let distance = ((camera[0] - at[0]).powi(2)
            + (camera[1] - at[1]).powi(2)
            + (camera[2] - at[2]).powi(2))
        .sqrt();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a brightness, intended"
        )]
        let weight = children_brightness(angular_radius(universe.marker_radius(), distance)) as f32;
        fade = 1.0 - weight;
    }
    if fade <= 0.0 {
        return;
    }
    for (ring, brightness) in [1.0f32, 2.0, 3.0].iter().zip(AIR_RIM_BRIGHTNESS.iter()) {
        let radius = base + thickness * ring / 3.0;
        gizmos.circle(
            Isometry3d::new(Vec3::ZERO, rotation),
            radius,
            scaled(tint, *brightness * fade),
        );
    }
}

/// Draws the planet limb and air preview around the targeted region (#430).
///
/// Gizmo circles only, never a mesh (`E-RENDER-NO-MESH`): the limb circle
/// about the planet centre plus the air rim, both turned into the target
/// patch's tangent plane, fading in by the target's angular size. At the
/// open instant the limb matches the kept parent context the region draws
/// after entry, so despawning the L10 body mesh swaps no full-weight
/// element. Allocates nothing per frame (`E-HOT-NOALLOC`).
pub(crate) fn draw_limb_preview(mut gizmos: Gizmos, universe: Res<Universe>, nav: Res<Navigation>) {
    use crate::sky::limb_preview_brightness;
    if universe.level().get() != 10 {
        return;
    }
    let Some(target) = nav.target else {
        return;
    };
    let Some(point) = universe.open.points.get(target as usize) else {
        return;
    };
    if point.kind != MarkerKind::Portal {
        return;
    }
    let universe_core::r#gen::Form::Patch { normal } = point.form else {
        return;
    };
    let camera = universe.path.offset();
    if !camera.iter().all(|component| component.is_finite()) {
        return;
    }
    let at = point.portal_position();
    let distance =
        ((camera[0] - at[0]).powi(2) + (camera[1] - at[1]).powi(2) + (camera[2] - at[2]).powi(2))
            .sqrt();
    let angular = angular_radius(universe.marker_radius(), distance);
    let brightness = limb_preview_brightness(angular);
    if brightness <= 0.0 {
        return;
    }
    let rotation = Quat::from_rotation_arc(Vec3::Z, unit(normal));
    let limb_color = scaled(point_color_for_level(universe.level()), brightness);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cell-unit body radius narrowed for the GPU, intended"
    )]
    let limb = PLANET_RADIUS_CELL as f32;
    gizmos.circle(Isometry3d::new(Vec3::ZERO, rotation), limb, limb_color);
    let Some(body) = body_for(&universe.open.points) else {
        return;
    };
    let Some(air) = body.air else {
        return;
    };
    if !body.radius_earth.is_finite() || body.radius_earth <= 0.0 {
        return;
    }
    let planet_radius_km = body.radius_earth * EARTH_RADIUS_KM;
    if !planet_radius_km.is_finite() || planet_radius_km <= 0.0 {
        return;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cell-unit air thickness narrowed for the GPU, intended"
    )]
    let thickness = (air.thickness_km / planet_radius_km * PLANET_RADIUS_CELL) as f32;
    if !thickness.is_finite() || thickness <= 0.0 {
        return;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: air tint narrowed for the GPU, intended"
    )]
    let tint = Color::srgb(air.tint[0] as f32, air.tint[1] as f32, air.tint[2] as f32);
    gizmos.circle(
        Isometry3d::new(Vec3::ZERO, rotation),
        limb + thickness,
        scaled(tint, brightness),
    );
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
    fn body_mesh_carries_relief_without_paint() {
        let cell = TerrainSampler::home().generate(7, &demo_budget());
        let body = body_for(&cell.points).expect("home cell names its body");
        let mesh = planet_mesh_for(7, &cell.points, &body).expect("home cell meshes");
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("body has positions");
        assert!(
            mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_none(),
            "the bare body carries no paint"
        );
        let bevy::mesh::VertexAttributeValues::Float32x3(verts) = positions else {
            panic!("body positions have the displaced-mesh shape");
        };
        assert!(
            verts.len() > 200,
            "icosphere resolves relief: {}",
            verts.len()
        );
        let mut min_radius = f32::INFINITY;
        let mut max_radius = f32::NEG_INFINITY;
        for vertex in verts {
            let radius =
                (vertex[0] * vertex[0] + vertex[1] * vertex[1] + vertex[2] * vertex[2]).sqrt();
            min_radius = min_radius.min(radius);
            max_radius = max_radius.max(radius);
        }
        assert!(
            max_radius > min_radius,
            "relief displaces vertices: {min_radius}..{max_radius}"
        );
    }

    #[test]
    fn empty_cells_mesh_nothing() {
        assert!(body_for(&[]).is_none());
        let body = BodyRecipe {
            flattening: universe_core::terrain::EARTH_FLATTENING,
            tilt_deg: universe_core::terrain::EARTH_TILT_DEG,
            spin_hours: universe_core::terrain::EARTH_SPIN_HOURS,
            radius_earth: 1.0,
            air: universe_core::terrain::air_for_radius_earth(1.0),
        };
        assert!(planet_mesh_for(7, &[], &body).is_none());
    }
}
