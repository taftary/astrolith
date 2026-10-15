//! The game's planet view for development tools (#640, ADR 0029).
//!
//! [`PlanetViewPlugin`] runs the planet drawing the game runs at L10, with
//! the same systems in the same order: the open cell's markers (moon
//! orbits, scenery), the moons, the one icosphere surface with its LOD
//! (#593, #598), the squares look (#626) and the air rim. A tool (the
//! engine lab) inserts the [`crate::Universe`] opened down to a planet and
//! sets the camera with [`PlanetViewPose`]; it can switch the surface's
//! [`SurfaceDebug`] views and read [`SurfaceStats`]. The game never adds
//! this plugin, so nothing here changes the game.

use crate::Universe;
use crate::input::Navigation;
use crate::planet::PlanetMeshState;
use crate::style::{RenderOrigin, to_vec3};
use crate::surface_mesh::SurfaceMeshState;
use bevy::prelude::*;

pub use crate::surface_mesh::{SurfaceDebug, SurfaceStats};

/// Where the view's camera is and looks, in the open cell's units.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct PlanetViewPose {
    /// Camera position.
    pub offset: [f64; 3],
    /// Unit view direction.
    pub forward: [f64; 3],
    /// Unit camera up.
    pub up: [f64; 3],
}

impl Default for PlanetViewPose {
    fn default() -> PlanetViewPose {
        PlanetViewPose {
            offset: [0.0, 0.0, 3.0],
            forward: [0.0, 0.0, -1.0],
            up: [0.0, 1.0, 0.0],
        }
    }
}

/// Normal lines brightness in the debug view.
const NORMAL_TINT: Color = Color::srgb(0.35, 0.85, 1.0);

/// Draws the game's planet view for a development tool (#640).
pub struct PlanetViewPlugin;

/// The view's system order: pose, anchors, then the game's draw chain.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanetViewSystems {
    /// Camera pose and render origin.
    Pose,
    /// The planet drawing.
    Draw,
}

impl Plugin for PlanetViewPlugin {
    fn build(&self, app: &mut App) {
        if !app
            .world()
            .contains_resource::<Assets<crate::planet_squares::SurfaceMaterial>>()
        {
            app.init_asset::<crate::planet_squares::SurfaceMaterial>();
        }
        if !app.world().contains_resource::<Universe>() {
            app.insert_resource(Universe(universe_core::nav::Universe::new(
                universe_core::nav::DEMO_SEED,
            )));
        }
        if !app.world().contains_resource::<crate::SettingsStore>() {
            app.insert_resource(crate::SettingsStore(
                universe_core::settings::Settings::default(),
            ));
        }
        app.init_resource::<Navigation>()
            .init_resource::<RenderOrigin>()
            .init_resource::<PlanetMeshState>()
            .init_resource::<SurfaceMeshState>()
            .init_resource::<SurfaceDebug>()
            .init_resource::<SurfaceStats>()
            .init_resource::<PlanetViewPose>()
            .configure_sets(
                Update,
                (PlanetViewSystems::Pose, PlanetViewSystems::Draw).chain(),
            )
            .add_systems(
                Update,
                (
                    (
                        crate::settings_ui::sync_world,
                        apply_pose,
                        crate::style::place_anchored,
                    )
                        .chain()
                        .in_set(PlanetViewSystems::Pose),
                    (
                        crate::draw::draw_open_cell,
                        crate::planet::draw_planets,
                        crate::surface_mesh::draw_surface,
                        crate::planet_squares::update_squares,
                        crate::planet::draw_air_rim,
                        draw_surface_normals,
                    )
                        .chain()
                        .in_set(PlanetViewSystems::Draw),
                ),
            );
    }
}

/// Puts the camera at the pose: the universe's camera offset (the shell
/// level follows it, #632), the navigation heading the surface split
/// reads, the render origin (ADR 0026) and the camera transform, with a
/// near plane at a share of the height above the ground.
fn apply_pose(
    pose: Res<PlanetViewPose>,
    mut universe: ResMut<Universe>,
    mut nav: ResMut<Navigation>,
    mut origin: ResMut<RenderOrigin>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Camera3d>>,
) {
    if universe.path.offset() != pose.offset {
        universe.path.set_offset(pose.offset);
    }
    nav.forward = pose.forward;
    nav.up = pose.up;
    origin.0 = pose.offset;
    let height = universe
        .path
        .height_above_ground_m()
        .zip(universe.path.planet_metres_per_unit())
        .map_or(1.0, |(metres, metres_per_unit)| metres / metres_per_unit);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: render-domain narrowing of a clamped gap, intended"
    )]
    let near = (height.abs() * 0.05).clamp(1e-12, 0.1) as f32;
    for (mut transform, mut projection) in &mut cameras {
        *transform = Transform::from_translation(Vec3::ZERO)
            .looking_to(to_vec3(pose.forward), to_vec3(pose.up));
        if let Projection::Perspective(perspective) = &mut *projection {
            perspective.near = near;
        }
    }
}

/// Draws the surface's face normals as short lines (the normals debug
/// view, #640): from each drawn triangle's centre along its normal.
fn draw_surface_normals(
    mut gizmos: Gizmos,
    state: Res<SurfaceMeshState>,
    origin: Res<RenderOrigin>,
) {
    for (centre, normal) in &state.normals {
        let tip = [
            centre[0] + normal[0],
            centre[1] + normal[1],
            centre[2] + normal[2],
        ];
        gizmos.line(origin.at(*centre), origin.at(tip), NORMAL_TINT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface_mesh::SurfaceMesh;
    use bevy::asset::AssetPlugin;
    use bevy::gizmos::{
        GizmoAsset, GizmoHandles,
        config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    };
    use universe_core::nav::{DEMO_SEED, MarkerIndex};

    /// A headless app with only the planet view, on the demo journey's
    /// planet, the camera `metres` over the north pole looking ahead.
    fn view_app(metres: f64) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_resource::<GizmoConfigStore>();
        app.init_gizmo_group::<DefaultGizmoConfigGroup>();
        app.init_asset::<GizmoAsset>();
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.init_resource::<GizmoHandles>();
        let mut universe = universe_core::nav::Universe::new(DEMO_SEED);
        for &marker in universe_core::nest::autopilot_path(DEMO_SEED)
            .iter()
            .take(9)
        {
            assert!(universe.open(MarkerIndex(marker)));
        }
        let metres_per_unit = universe.path.planet_metres_per_unit().expect("on a planet");
        app.insert_resource(Universe(universe));
        app.add_plugins(PlanetViewPlugin);
        let ground = universe_core::terrain::surface_radius([0.0, 1.0, 0.0]);
        let lift = metres / metres_per_unit;
        app.insert_resource(PlanetViewPose {
            offset: [0.0, ground + lift, 0.0],
            forward: [0.0, -0.6, -0.8],
            up: [0.0, 0.8, -0.6],
        });
        app.update();
        app
    }

    fn surface(app: &mut App) -> Vec<(Entity, Visibility)> {
        let mut query = app
            .world_mut()
            .query_filtered::<(Entity, &Visibility), With<SurfaceMesh>>();
        query.iter(app.world()).map(|(e, v)| (e, *v)).collect()
    }

    #[test]
    fn the_planet_view_draws_the_game_planet_for_a_tool() {
        // One surface mesh for the home planet, the camera at the pose, the
        // shell level following the height (#640, #632).
        let mut app = view_app(500.0);
        assert_eq!(surface(&mut app).len(), 1, "one icosphere surface");
        let universe = app.world().resource::<Universe>();
        assert_eq!(universe.shown_level().get(), 13, "500 m reads L13");
        let origin = app.world().resource::<RenderOrigin>().0;
        assert_eq!(origin, universe.path.offset(), "camera-relative drawing");
    }

    #[test]
    fn the_debug_views_toggle_and_count_lod_splits() {
        // C10 / AC9: wireframe hides the filled surface, normals draw one
        // line per triangle, the LOD view recolours by depth, and the
        // split counts per depth add up to the drawn triangles.
        let mut app = view_app(500.0);
        let stats = app.world().resource::<SurfaceStats>().clone();
        assert!(
            stats.faces > 80,
            "the LOD splits near the camera: {}",
            stats.faces
        );
        assert_eq!(stats.per_depth.iter().sum::<usize>(), stats.faces);
        assert!(
            stats.max_depth() > 10,
            "deep splits at 500 m: {}",
            stats.max_depth()
        );
        assert!(stats.edges > stats.faces && stats.vertices > 0);
        let plain = surface(&mut app);
        assert_eq!(plain[0].1, Visibility::Inherited);
        assert!(
            app.world()
                .resource::<SurfaceMeshState>()
                .normals
                .is_empty()
        );

        app.world_mut().resource_mut::<SurfaceDebug>().wireframe = true;
        app.update();
        let wire = surface(&mut app);
        assert_eq!(wire.len(), 1);
        assert_eq!(wire[0].1, Visibility::Hidden, "wireframe hides the fill");

        app.world_mut().resource_mut::<SurfaceDebug>().normals = true;
        app.update();
        let faces = app.world().resource::<SurfaceStats>().faces;
        assert_eq!(
            app.world().resource::<SurfaceMeshState>().normals.len(),
            faces
        );

        app.world_mut().resource_mut::<SurfaceDebug>().lod_colors = true;
        app.update();
        let entity = surface(&mut app)[0].0;
        let handle = app.world().get::<Mesh3d>(entity).expect("mesh").0.clone();
        let meshes = app.world().resource::<Assets<Mesh>>();
        let mesh = meshes.get(&handle).expect("mesh asset");
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("vertex colours");
        };
        let tones: std::collections::HashSet<[u32; 4]> =
            colors.iter().map(|c| c.map(f32::to_bits)).collect();
        assert!(tones.len() > 2, "several depths, several tones");

        *app.world_mut().resource_mut::<SurfaceDebug>() = SurfaceDebug::default();
        app.update();
        assert_eq!(
            surface(&mut app)[0].1,
            Visibility::Inherited,
            "all off again"
        );
        assert!(
            app.world()
                .resource::<SurfaceMeshState>()
                .normals
                .is_empty()
        );
    }
}
