//! L10 moons as meshes (rebuild step 3, #776).
//!
//! The sphere itself is LOD patches (`crate::patches`): six fixed faces from
//! #775 are gone. Moons stay here, built once per open cell (seed-keyed) and
//! despawned when the dive leaves L10. Every mesh carries [`PlanetMesh`] plus
//! a `Name`.

use crate::Universe;
use crate::style::RenderOrigin;
use bevy::mesh::primitives::{SphereKind, SphereMeshBuilder};
use bevy::mesh::{Mesh, Mesh3d};
use bevy::prelude::*;

/// Icosphere subdivisions for moons (42 vertices; moons are dots with shading).
pub(crate) const MOON_SUBDIVISIONS: u32 = 1;

/// Moon body radius in cell units (indicator moons draw at 0.005).
pub(crate) const MOON_RADIUS_CELL: f32 = 0.008;

/// Marks entities owned by [`draw_planets`].
#[derive(Component)]
pub(crate) struct PlanetMesh;

/// Tracks which open cell the spawned moon meshes were built for.
#[derive(Resource, Default)]
pub(crate) struct PlanetMeshState {
    /// Open-cell seed the current meshes belong to, if any.
    seed: Option<u64>,
}

/// Spawns moon meshes for the open L10 cell.
///
/// Rebuilds only when the open cell changes (seed-keyed); per-frame cost is
/// one level check otherwise. Every mesh carries [`PlanetMesh`] plus a
/// `Name`, so inspectors and logs can say which entity.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn draw_planets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    universe: Res<Universe>,
    origin: Res<RenderOrigin>,
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
    let gray = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.55, 0.58),
        // Lit by the planet's sun like the ground (#780 Q1 A).
        perceptual_roughness: 1.0,
        reflectance: 0.0,
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
            Transform::from_translation(origin.at(point.position)),
            crate::style::Anchored(point.position),
        ));
    }
    state.seed = Some(seed);
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use universe_core::r#gen::{Generator, Point};
    use universe_core::terrain::{BodyRecipe, TerrainSampler};

    /// Cell recipe from the first surface sample (test helper).
    fn body_for(points: &[Point]) -> Option<BodyRecipe> {
        let surface = points.iter().find_map(|point| point.surface)?;
        Some(BodyRecipe {
            tilt_deg: surface.tilt_deg,
            spin_hours: surface.spin_hours,
            radius_earth: surface.radius_earth,
            air: surface.air,
        })
    }

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

    #[test]
    fn moons_are_drawn_at_l10() {
        // #776 T3: the six fixed faces are gone (LOD patches draw the sphere
        // in `crate::patches`); the moons stay; leaving L10 despawns with no
        // error. Draw-list test with no window (E-TEST-NO-WINDOW).
        use crate::window_tests::headless_app;
        use universe_core::nav::MarkerIndex;
        let mut app = headless_app();
        {
            let mut universe = app.world_mut().resource_mut::<crate::Universe>();
            for _ in 0..9 {
                let marker = universe.autopilot_target().expect("dive continues");
                assert!(universe.open(MarkerIndex(marker)), "marker opens");
            }
            assert_eq!(universe.level().get(), 10, "the dive ends at L10");
        }
        app.update();
        let mut moons = 0usize;
        {
            let mut names = app
                .world_mut()
                .query_filtered::<&Name, With<super::PlanetMesh>>();
            for name in names.iter(app.world()) {
                if name.as_str() == "planet-moon" {
                    moons += 1;
                } else {
                    panic!("unexpected planet mesh {}", name.as_str());
                }
            }
        }
        assert!(moons >= 1, "the moons stay");
        // A second frame rebuilds nothing.
        app.update();
        {
            let mut count = app
                .world_mut()
                .query_filtered::<Entity, With<super::PlanetMesh>>();
            assert_eq!(count.iter(app.world()).count(), moons);
        }
        // Leaving L10 despawns everything with no error.
        {
            let mut universe = app.world_mut().resource_mut::<crate::Universe>();
            assert!(universe.close().is_some(), "the dive backs out");
        }
        app.update();
        {
            let mut count = app
                .world_mut()
                .query_filtered::<Entity, With<super::PlanetMesh>>();
            assert_eq!(count.iter(app.world()).count(), 0, "no mesh outside L10");
        }
    }
}
