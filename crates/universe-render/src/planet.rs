//! L10 moons as meshes (rebuild step 3, #776).
//!
//! The sphere itself is LOD patches (`crate::patches`): six fixed faces from
//! #775 are gone. Moons stay here, built once per open cell (seed-keyed) and
//! despawned when the dive leaves L10. Every mesh carries [`PlanetMesh`] plus
//! a `Name`.

use crate::Universe;
use crate::resource_diagnostic::{AssetKey, Descriptor, Role};
use crate::style::RenderOrigin;
use bevy::mesh::primitives::{SphereKind, SphereMeshBuilder};
use bevy::mesh::{Mesh, Mesh3d};
use bevy::prelude::*;

/// Icosphere subdivisions for moons. Bevy's icosphere has `10 (s + 1)^2 + 2`
/// points, so 31 gives 10,242, about 2 degrees apart: the smaller painted
/// craters (about 3 degrees across) keep their round shape (#1161 T6 and
/// R1, Q8 A). Built once per open cell.
pub(crate) const MOON_SUBDIVISIONS: u32 = 31;

/// Each vertex of `mesh` (a unit-radius-centred sphere) coloured with the
/// moon tone of `kind` (sRGB to linear), as a vertex colour attribute.
fn paint_moon(mesh: &mut Mesh, kind: universe_core::world_type::WorldType, seed: u64) {
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return;
    };
    let paint = universe_core::world_type::MoonPaint::new(kind, seed);
    let colours: Vec<[f32; 4]> = positions
        .iter()
        .map(|p| {
            let dir = [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])];
            let [r, g, b] = paint.tone(dir);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: colour channels narrowed for the GPU, intended"
            )]
            let c = Color::srgb(r as f32, g as f32, b as f32).to_linear();
            [c.red, c.green, c.blue, 1.0]
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
}

/// Marks entities owned by [`draw_planets`].
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct PlanetMesh {
    /// Owner captured where the production proxy was actually built.
    pub(crate) host: Option<universe_core::moon_catalog::PlanetIdentity>,
    pub(crate) marker: universe_core::nav::MarkerIndex,
    pub(crate) kind: universe_core::world_type::WorldType,
    pub(crate) paint_seed: u64,
}

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
    diagnostics: Option<Res<crate::LoadingDiagnostics>>,
) {
    let seed = universe.open_seed();
    let active = universe.level().get() == 10;
    let witness = if (active && state.seed != Some(seed)) || (!active && state.seed.is_some()) {
        diagnostics.as_ref().map(|d| d.context(&universe))
    } else {
        None
    };
    if !active || state.seed != Some(seed) {
        for (entity, mesh, material) in &owned {
            commands.entity(entity).despawn();
            let removed_mesh = meshes.remove(&mesh.0).is_some();
            let removed_material = materials.remove(&material.0).is_some();
            if let Some(witness) = &witness {
                witness.removed(AssetKey::Mesh(mesh.id()), removed_mesh);
                witness.removed(AssetKey::Standard(material.id()), removed_material);
            }
        }
        state.seed = None;
    }
    if !active || state.seed == Some(seed) {
        return;
    }
    // White base: each moon's colour is in its vertices (#1161 T6).
    let gray = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        // Lit by the planet's sun like the ground (#780 Q1 A).
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..default()
    });
    if let Some(witness) = &witness {
        witness.created(
            AssetKey::Standard(gray.id()),
            Role::MoonMaterial,
            Descriptor::Material,
        );
    }
    // The planet's physics, for the moons' types (Q7 A): read once per cell.
    let chain = universe.path.indices();
    let planet = chain
        .get(..9)
        .and_then(|l10| universe_core::world_type::planet_inputs(universe.root.0, l10));
    let host = universe_core::moon_catalog::PlanetIdentity::from_open_cell(&universe).ok();
    for (index, (id, point)) in (0u64..).zip(universe.open.with_ids()) {
        let Some(info) = point.moon else {
            continue;
        };
        // True size (#995 T9): the moon's own radius in cell units; its
        // gizmo marker keeps it findable while it is smaller than a pixel.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: a moon radius in cell units, small and finite"
        )]
        let radius = point.radius as f32;
        let mut moon = SphereMeshBuilder::new(
            radius,
            SphereKind::Ico {
                subdivisions: MOON_SUBDIVISIONS,
            },
        )
        .build();
        let moon_seed = universe_core::seed::hash_triple(seed, index, 0x1161);
        let kind = planet
            .as_ref()
            .map_or(universe_core::world_type::WorldType::Barren, |p| {
                universe_core::world_type::classify_moon(p, moon_seed, info.radius_km / 6_371.0)
            });
        paint_moon(&mut moon, kind, moon_seed);
        let descriptor = witness.as_ref().map(|_| Descriptor::mesh(&moon));
        let handle = meshes.add(moon);
        if let (Some(witness), Some(descriptor)) = (&witness, descriptor) {
            witness.created(
                AssetKey::Mesh(handle.id()),
                Role::MoonProxy(universe_core::nav::MarkerIndex(id)),
                descriptor,
            );
        }
        commands.spawn((
            PlanetMesh {
                host,
                marker: universe_core::nav::MarkerIndex(id),
                kind,
                paint_seed: moon_seed,
            },
            Name::new("planet-moon"),
            Mesh3d(handle),
            MeshMaterial3d(gray.clone()),
            Transform::from_translation(origin.at(point.position))
                .with_scale(Vec3::splat(origin.len(1.0))),
            crate::style::Anchored(point.position),
            crate::style::CellSized,
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
    fn a_moon_ball_is_fine_enough_for_its_craters() {
        // #1161 R1 (criterion 3): about 10,000 points, so a painted crater
        // of 0.05 radians keeps several points across.
        use bevy::mesh::primitives::{SphereKind, SphereMeshBuilder};
        let mesh = SphereMeshBuilder::new(
            1.0,
            SphereKind::Ico {
                subdivisions: super::MOON_SUBDIVISIONS,
            },
        )
        .build();
        let points = mesh.count_vertices();
        assert!((9_000..=20_000).contains(&points), "{points} points");
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
