//! Planet sphere meshes for the open L10 cell (rebuild step 2, #775).
//!
//! The planet is back as six cube-sphere faces (Q2 A): one Bevy `Mesh` per
//! face at a fixed [`SPHERE_GRID`] (no LOD yet, step 3 #776), one unlit
//! flat-colour material, built once per open cell (seed-keyed, like the
//! moons) and despawned when the dive leaves L10. Vertices run through the
//! one planet-frame conversion (#774), relative to an [`Anchored`] patch
//! origin. At orbital and flying distances the sphere stays still; within a
//! few hundred metres of the surface it can move in steps of about 0.4 m
//! until the LOD patches of #776 (Spec v3). Face meshes carry [`PlanetMesh`]
//! plus a `Name`.

use crate::Universe;
use crate::style::{RenderOrigin, to_vec3};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::primitives::{SphereKind, SphereMeshBuilder};
use bevy::mesh::{Indices, Mesh, Mesh3d, MeshBuilder, PrimitiveTopology};
use bevy::prelude::*;
use universe_core::cube_sphere::{Face, grid_point};
use universe_core::planet_frame::{PlanetFrame, PlanetPos};

/// Quads per cube-face edge for the fixed-resolution sphere (Q2 A).
///
/// An engine constant replaced by LOD in step 3 (#776). `65 x 65` vertices
/// per face.
pub(crate) const SPHERE_GRID: u32 = 64;

/// Icosphere subdivisions for moons (42 vertices; moons are dots with shading).
pub(crate) const MOON_SUBDIVISIONS: u32 = 1;

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

/// One cube-sphere face as a Bevy mesh, planet-relative (Q2 A, Q4 A).
///
/// Vertices are surface points through the one conversion
/// (`PlanetFrame::to_cell` of `radius x dir`), so the mesh is anchored at
/// the planet centre and placed camera-relative by [`place_anchored`].
#[must_use]
pub(crate) fn sphere_face_mesh(face: Face, frame: &PlanetFrame) -> Mesh {
    let n = SPHERE_GRID;
    let side = usize::try_from(n + 1).unwrap_or(0);
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(side * side);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(side * side);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(side * side);
    for j in 0..=n {
        for i in 0..=n {
            let dir = grid_point(face, i, j, n).unwrap_or([0.0, 1.0, 0.0]);
            let planet = universe_core::cube_sphere::to_planet(dir, frame.radius_m())
                .unwrap_or(PlanetPos([0.0, 0.0, 0.0]));
            let cell = frame.to_cell(planet);
            positions.push(to_vec3(cell).to_array());
            normals.push(to_vec3(dir).to_array());
            let iu = u16::try_from(i).unwrap_or(0);
            let ju = u16::try_from(j).unwrap_or(0);
            let nu = u16::try_from(n).unwrap_or(1);
            uvs.push([f32::from(iu) / f32::from(nu), f32::from(ju) / f32::from(nu)]);
        }
    }
    let mut indices: Vec<u32> = Vec::with_capacity(usize::try_from(n * n * 6).unwrap_or(0));
    for j in 0..n {
        for i in 0..n {
            let stride = n + 1;
            let a = j * stride + i;
            let b = a + 1;
            let c = a + stride;
            let d = c + 1;
            for v in [a, c, b, b, c, d] {
                indices.push(v);
            }
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
    mesh
}

/// Face name for inspectors and logs (`E-ENTITY-NAME`).
#[must_use]
pub(crate) const fn face_name(face: Face) -> &'static str {
    match face {
        Face::PosX => "planet-face-+X",
        Face::NegX => "planet-face--X",
        Face::PosY => "planet-face-+Y",
        Face::NegY => "planet-face--Y",
        Face::PosZ => "planet-face-+Z",
        Face::NegZ => "planet-face--Z",
    }
}

/// Spawns the planet sphere plus moon meshes for the open L10 cell.
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
    // One flat-colour unlit material for the sphere (Q2 A; light in #780).
    let planet_colour = crate::style::point_color_for_level(
        universe_core::coords::Level::new(10).unwrap_or(universe_core::coords::Level::MIN),
    );
    let planet = materials.add(StandardMaterial {
        base_color: planet_colour,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let gray = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.55, 0.58),
        unlit: true,
        ..default()
    });
    // Planet frame for the open cell (one source, #774 review item 4).
    let chain = universe.path.indices();
    let frame = chain
        .get(..9)
        .map(|l10| PlanetFrame::for_planet(universe.root.0, l10));
    if let Some(frame) = frame {
        let at_centre = origin.cell();
        for face in Face::all() {
            let mesh = sphere_face_mesh(face, &frame);
            commands.spawn((
                PlanetMesh,
                Name::new(face_name(face)),
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(planet.clone()),
                Transform::from_translation(at_centre),
                crate::style::Anchored([0.0, 0.0, 0.0]),
            ));
        }
    }
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
    fn the_planet_sphere_is_drawn_at_l10() {
        // AC4 / C4 (T4, Q2 A): at L10 the planet is six face meshes in one
        // flat colour, unlit, at fixed resolution, through the one conversion;
        // the moons stay; leaving L10 despawns with no error. Draw-list test
        // with no window (E-TEST-NO-WINDOW).
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
        // Six faces plus the moons.
        let mut faces = 0usize;
        let mut moons = 0usize;
        {
            let mut names = app
                .world_mut()
                .query_filtered::<&Name, With<super::PlanetMesh>>();
            for name in names.iter(app.world()) {
                if name.as_str().starts_with("planet-face-") {
                    faces += 1;
                } else if name.as_str() == "planet-moon" {
                    moons += 1;
                }
            }
        }
        assert_eq!(faces, 6, "six cube faces");
        assert!(moons >= 1, "the moons stay");
        // Vertex count: `6 x 65 x 65` at the fixed grid.
        let mut verts = 0usize;
        {
            let mut query = app
                .world_mut()
                .query_filtered::<(&Name, &Mesh3d), With<super::PlanetMesh>>();
            let handles: Vec<(String, Mesh3d)> = query
                .iter(app.world())
                .filter(|(name, _)| name.as_str().starts_with("planet-face-"))
                .map(|(name, handle)| (name.as_str().to_string(), handle.clone()))
                .collect();
            let meshes = app.world().resource::<Assets<Mesh>>();
            for (_, handle) in handles {
                let mesh = meshes.get(&handle.0).expect("face mesh");
                let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                else {
                    panic!("face positions");
                };
                verts += pos.len();
            }
        }
        assert_eq!(verts, 6 * 65 * 65, "fixed resolution, no LOD yet");
        // A second frame rebuilds nothing.
        app.update();
        {
            let mut count = app
                .world_mut()
                .query_filtered::<Entity, With<super::PlanetMesh>>();
            assert_eq!(count.iter(app.world()).count(), faces + moons);
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
