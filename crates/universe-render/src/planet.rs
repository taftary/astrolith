//! Moon meshes and air gizmos for the open L10 cell (#375, ADR 0016, ADR 0024).
//!
//! The planet body itself is the one icosphere surface mesh
//! (`surface_mesh`, ADR 0024). This module adds one small body per moon,
//! rebuilt when the open cell changes and despawned when the dive leaves
//! L10, plus the air rim and limb preview gizmos. Everything outside L10
//! stays gizmo indicators under ADR 0002.

use crate::Universe;
use crate::input::Navigation;
use crate::planet_squares::{SurfaceMaterial, surface_material};
use crate::style::{scaled, to_vec3};
use bevy::math::Isometry3d;
use bevy::mesh::{
    Mesh, Mesh3d, MeshBuilder, primitives::SphereKind, primitives::SphereMeshBuilder,
};
use bevy::prelude::*;
use universe_core::frame::{angular_radius, children_brightness};
use universe_core::r#gen::{MarkerKind, Point};
use universe_core::terrain::{BodyRecipe, EARTH_RADIUS_KM, PLANET_RADIUS_CELL};

/// Icosphere subdivisions for moons (42 vertices; moons are dots with shading).
pub(crate) const MOON_SUBDIVISIONS: u32 = 1;

/// Moon body radius in cell units (indicator moons draw at 0.005).
pub(crate) const MOON_RADIUS_CELL: f32 = 0.008;

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
        tilt_deg: surface.tilt_deg,
        spin_hours: surface.spin_hours,
        radius_earth: surface.radius_earth,
        air: surface.air,
    })
}

/// Spawns moon meshes for the open L10 cell, despawning them elsewhere.
///
/// Rebuilds only when the open cell changes (seed-keyed); per-frame cost is
/// one level check otherwise. Moons carry [`PlanetMesh`] plus a `Name`, so
/// inspectors and logs can say which entity.
pub(crate) fn draw_planets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SurfaceMaterial>>,
    universe: Res<Universe>,
    mut state: ResMut<PlanetMeshState>,
    owned: Query<(Entity, &Mesh3d, &MeshMaterial3d<SurfaceMaterial>), With<PlanetMesh>>,
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
    let gray = materials.add(surface_material(StandardMaterial {
        base_color: Color::srgb(0.55, 0.55, 0.58),
        unlit: true,
        ..default()
    }));
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
            crate::style::Anchored(point.position),
        ));
    }
    state.seed = Some(seed);
}

/// Draws the planet's air as three concentric camera-facing circles (#394).
///
/// Indicators only: no mesh, no rim entity (the `planet-rim` count stays
/// zero). The circles sit just outside the body at `R + t*k/3` with the air
/// tint fading outward; airless worlds draw nothing. Near the targeted
/// region's open threshold the rim fades out while the limb preview fades
/// in (#430), so the entry swaps no full-weight element. Allocates nothing
/// per frame (`E-HOT-NOALLOC`): three gizmo circles, no buffers.
pub(crate) fn draw_air_rim(
    mut gizmos: Gizmos,
    universe: Res<Universe>,
    nav: Res<Navigation>,
    origin: Res<crate::style::RenderOrigin>,
) {
    let cell = origin.cell();
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
            Isometry3d::new(cell, rotation),
            radius,
            scaled(tint, *brightness * fade),
        );
    }
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
    fn home_cell_names_its_body() {
        let cell = TerrainSampler::home().generate(7, &demo_budget());
        let body = body_for(&cell.points).expect("home cell names its body");
        assert!(body.radius_earth > 0.0);
        assert!(body_for(&[]).is_none());
    }
}
