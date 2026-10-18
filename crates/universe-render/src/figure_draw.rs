//! One lit low-poly figure for the waiting and the active body (#755).
//!
//! The waiting body and the active body share this one code path (Q3 A).
//! The mesh is canonical (identity pose) with the entity rotation carrying
//! the heading; per-frame cost is a transform update, colours refresh in
//! place only when the body colour, star direction or orientation moves.
//! Unlit with CPU vertex colours, so nothing else in the scene changes.

use crate::Universe;
use crate::input::{Flight, FlightMode};
use crate::player_state::PlayerState;
use crate::style::RenderOrigin;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;
use universe_core::body::{body_height, heading_basis};
use universe_core::figure::is_accent;
use universe_core::settings::BodyColour;

/// Marks the figure mesh entity.
#[derive(Component)]
pub(crate) struct FigureMesh;

/// Pose of the figure in open-cell units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FigurePose {
    /// Feet position.
    pub feet: [f64; 3],
    /// Ground up.
    pub up: [f64; 3],
    /// Body forward.
    pub forward: [f64; 3],
    /// Body right.
    pub right: [f64; 3],
    /// One body height in cell units.
    pub height: f64,
    /// First person (no head) or third person (full figure).
    pub first_person: bool,
}

/// Figure pose for this frame, if any.
///
/// Active body in normal mode, else the waiting body while its cell is open.
/// Both come from this one function (AC3). `None` when no body shows.
#[must_use]
pub(crate) fn figure_pose(
    universe: &Universe,
    state: &PlayerState,
    flight: &Flight,
) -> Option<FigurePose> {
    let level = universe.level();
    let height = body_height(level);
    if flight.mode == FlightMode::Normal && state.active {
        let up = universe_core::body::level_up(universe, state.body.position);
        let (forward, right) = heading_basis(up, state.body.yaw);
        return Some(FigurePose {
            feet: state.body.position,
            up,
            forward,
            right,
            height,
            first_person: !state.third_person,
        });
    }
    if state.spot.is_near(&universe.path) {
        let up = universe_core::body::level_up(universe, state.spot.position);
        let (forward, right) = heading_basis(up, state.spot.yaw);
        return Some(FigurePose {
            feet: state.spot.position,
            up,
            forward,
            right,
            height,
            first_person: false,
        });
    }
    None
}

/// Star direction in the figure's local frame.
fn to_star_local(to_star: [f64; 3], pose: &FigurePose) -> [f64; 3] {
    let minus_forward = [-pose.forward[0], -pose.forward[1], -pose.forward[2]];
    [
        to_star[0] * pose.right[0] + to_star[1] * pose.right[1] + to_star[2] * pose.right[2],
        to_star[0] * pose.up[0] + to_star[1] * pose.up[1] + to_star[2] * pose.up[2],
        to_star[0] * minus_forward[0]
            + to_star[1] * minus_forward[1]
            + to_star[2] * minus_forward[2],
    ]
}

/// Canonical triangles at the identity pose (feet at the origin).
fn canonical_tris(first_person: bool, height: f64) -> Vec<universe_core::figure::FigureTriangle> {
    if first_person {
        universe_core::figure::first_person_triangles(
            [0.0; 3],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, -1.0],
            [1.0, 0.0, 0.0],
            height,
        )
        .to_vec()
    } else {
        universe_core::figure::figure_triangles(
            [0.0; 3],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, -1.0],
            [1.0, 0.0, 0.0],
            height,
        )
        .to_vec()
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "E-CAST: figure offsets narrowed for the GPU, intended"
)]
fn narrow3(v: [f64; 3]) -> [f32; 3] {
    [v[0] as f32, v[1] as f32, v[2] as f32]
}

/// Builds the figure mesh for `pose` with `colour` lit by `to_star` (world).
#[must_use]
pub(crate) fn figure_mesh_of(pose: &FigurePose, colour: BodyColour, to_star: [f64; 3]) -> Mesh {
    let tris = canonical_tris(pose.first_person, pose.height);
    let local_star = to_star_local(to_star, pose);
    let (suit, accent) = colour.colours();
    let count = tris.len() * 3;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(count);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(count);
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(count);
    for tri in &tris {
        let base = if is_accent(tri.part) { accent } else { suit };
        let lit = universe_core::light::shade(base, tri.normal, local_star);
        for v in tri.vertices {
            positions.push(narrow3(v));
            normals.push(narrow3(tri.normal));
            colors.push([lit[0], lit[1], lit[2], 1.0]);
        }
    }
    let count_u32 = u32::try_from(positions.len()).unwrap_or(0);
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32((0..count_u32).collect()));
    mesh
}

/// Refreshes the mesh colours in place (no new allocation beyond the writes).
pub(crate) fn refresh_colours(
    mesh: &mut Mesh,
    pose: &FigurePose,
    colour: BodyColour,
    to_star: [f64; 3],
) {
    let tris = canonical_tris(pose.first_person, pose.height);
    let local_star = to_star_local(to_star, pose);
    let (suit, accent) = colour.colours();
    let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR)
    else {
        return;
    };
    let mut cursor = 0usize;
    for tri in &tris {
        let base = if is_accent(tri.part) { accent } else { suit };
        let lit = universe_core::light::shade(base, tri.normal, local_star);
        for _ in 0..3 {
            if let Some(slot) = colors.get_mut(cursor) {
                *slot = [lit[0], lit[1], lit[2], 1.0];
            }
            cursor += 1;
        }
    }
}

/// Rotation carrying the canonical figure to `pose`.
#[must_use]
pub(crate) fn figure_rotation(pose: &FigurePose) -> Quat {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: figure basis narrowed for the GPU transform, intended"
    )]
    let cols = [
        Vec3::new(
            pose.right[0] as f32,
            pose.right[1] as f32,
            pose.right[2] as f32,
        ),
        Vec3::new(pose.up[0] as f32, pose.up[1] as f32, pose.up[2] as f32),
        Vec3::new(
            -pose.forward[0] as f32,
            -pose.forward[1] as f32,
            -pose.forward[2] as f32,
        ),
    ];
    Quat::from_mat3(&Mat3::from_cols(cols[0], cols[1], cols[2]))
}

/// Tracks the spawned figure to rebuild only on geometry change.
#[derive(Resource, Debug, Default)]
pub(crate) struct FigureState {
    /// `(level, first_person)` the current mesh was built for.
    pub key: Option<(u8, bool)>,
    /// Last colour inputs the mesh colours were built for.
    pub last_colour: Option<BodyColour>,
    /// Last local star the colours were built for.
    pub last_star: Option<[f64; 3]>,
}

/// One mesh for the waiting and the active body (#755 AC3).
///
/// Geometry rebuilds only when the level or first/third-person flips;
/// colours refresh in place when the colour, star or orientation moves;
/// per-frame cost is the transform. Camera-relative via [`RenderOrigin`]
/// and [`crate::style::Anchored`] (ADR 0026).
#[allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn draw_figure(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    universe: Res<Universe>,
    flight: Res<Flight>,
    state: Res<PlayerState>,
    store: Res<crate::SettingsStore>,
    origin: Res<RenderOrigin>,
    mut figure: ResMut<FigureState>,
    mut owned: Query<
        (
            Entity,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
            &mut crate::style::Anchored,
        ),
        With<FigureMesh>,
    >,
) {
    let Some(pose) = figure_pose(&universe, &state, &flight) else {
        for (entity, mesh, material, _, _) in &owned {
            commands.entity(entity).despawn();
            meshes.remove(&mesh.0);
            materials.remove(&material.0);
        }
        figure.key = None;
        figure.last_colour = None;
        figure.last_star = None;
        return;
    };
    let colour = store.0.body_colour;
    let to_star = universe_core::light::star_direction(&universe);
    let key = (universe.level().get(), pose.first_person);
    if figure.key != Some(key) {
        for (entity, mesh, material, _, _) in &owned {
            commands.entity(entity).despawn();
            meshes.remove(&mesh.0);
            materials.remove(&material.0);
        }
        let mesh = figure_mesh_of(&pose, colour, to_star);
        let material = StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            cull_mode: None,
            ..default()
        };
        commands.spawn((
            FigureMesh,
            Name::new("player-figure"),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material)),
            Transform {
                translation: origin.at(pose.feet),
                rotation: figure_rotation(&pose),
                ..default()
            },
            crate::style::Anchored(pose.feet),
        ));
        figure.key = Some(key);
        figure.last_colour = Some(colour);
        figure.last_star = Some(to_star_local(to_star, &pose));
        return;
    }
    let local_star = to_star_local(to_star, &pose);
    let recolours = figure.last_colour != Some(colour)
        || figure.last_star.is_none_or(|last| {
            ((last[0] - local_star[0]).powi(2)
                + (last[1] - local_star[1]).powi(2)
                + (last[2] - local_star[2]).powi(2))
            .sqrt()
                > 1e-6
        });
    for (_, mesh_handle, _, mut transform, mut anchor) in &mut owned {
        anchor.0 = pose.feet;
        transform.translation = origin.at(pose.feet);
        transform.rotation = figure_rotation(&pose);
        if recolours && let Some(mut mesh) = meshes.get_mut(&mesh_handle.0) {
            refresh_colours(&mut mesh, &pose, colour, to_star);
        }
    }
    if recolours {
        figure.last_colour = Some(colour);
        figure.last_star = Some(local_star);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_tests::headless_app;
    use universe_core::figure::{FIGURE_TRIANGLES, FIRST_PERSON_TRIANGLES};
    use universe_core::nav::{DEMO_SEED, MarkerIndex};

    #[test]
    fn one_drawing_for_waiting_and_active() {
        // The waiting body and the active body come from the same code path
        // with the same geometry: one drawing (#755 AC3).
        let mut app = headless_app();
        app.update();
        let universe = app.world().resource::<Universe>().clone();
        let state = app.world().resource::<PlayerState>().clone();
        let flight = *app.world().resource::<Flight>();
        // Waiting pose when the spot is near (dives to the spot's cell below).
        let spot = universe_core::player::PlayerSpot::seeded(DEMO_SEED).expect("spot");
        let mut waiting_universe = universe_core::nav::Universe::new(DEMO_SEED);
        for &marker in spot.chain() {
            assert!(waiting_universe.open(MarkerIndex(marker)), "the way opens");
        }
        assert!(spot.is_near(&waiting_universe.path));
        let waiting_wrapped = Universe(waiting_universe.clone());
        let idle = Flight {
            mode: FlightMode::Dive,
            ..flight
        };
        let mut waiting_state = state.clone();
        waiting_state.spot = spot;
        waiting_state.active = false;
        let waiting = figure_pose(&waiting_wrapped, &waiting_state, &idle).expect("waiting shows");
        assert!(!waiting.first_person, "waiting shows the whole figure");
        // Active pose at the same feet and yaw uses the same triangles.
        let mut active_state = waiting_state.clone();
        active_state.active = true;
        active_state.third_person = true;
        active_state.body = universe_core::body::Body::standing(spot.position, spot.yaw);
        let diving = Flight {
            mode: FlightMode::Normal,
            ..flight
        };
        let active = figure_pose(&waiting_wrapped, &active_state, &diving).expect("active shows");
        assert!(!active.first_person);
        let _ = (universe, state);
        let waiting_tris = universe_core::figure::figure_triangles(
            waiting.feet,
            waiting.up,
            waiting.forward,
            waiting.right,
            waiting.height,
        );
        let active_tris = universe_core::figure::figure_triangles(
            active.feet,
            active.up,
            active.forward,
            active.right,
            active.height,
        );
        assert_eq!(waiting_tris.len(), FIGURE_TRIANGLES);
        assert_eq!(active_tris.len(), FIGURE_TRIANGLES);
        for (a, b) in waiting_tris.iter().zip(active_tris.iter()) {
            assert_eq!(a.vertices, b.vertices);
            assert_eq!(a.normal, b.normal);
            assert_eq!(a.part, b.part);
        }
        // The mesh carries the lit colours, not the outline tint.
        let mesh = figure_mesh_of(&waiting, BodyColour::GreyOrange, [0.0, 1.0, 0.0]);
        assert!(mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
    }

    #[test]
    fn first_person_shows_the_body_not_the_head() {
        // Third person shows the whole figure; first person looking down
        // shows the torso, arms and legs and never the head (#755 AC4, view).
        let height = body_height(universe_core::coords::Level::new(10).expect("L10"));
        let full = figure_mesh_of(
            &FigurePose {
                feet: [0.0; 3],
                up: [0.0, 1.0, 0.0],
                forward: [0.0, 0.0, -1.0],
                right: [1.0, 0.0, 0.0],
                height,
                first_person: false,
            },
            BodyColour::GreyOrange,
            [0.0, 1.0, 0.0],
        );
        let first = figure_mesh_of(
            &FigurePose {
                feet: [0.0; 3],
                up: [0.0, 1.0, 0.0],
                forward: [0.0, 0.0, -1.0],
                right: [1.0, 0.0, 0.0],
                height,
                first_person: true,
            },
            BodyColour::GreyOrange,
            [0.0, 1.0, 0.0],
        );
        let positions = |mesh: &Mesh| {
            let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("positions");
            };
            pos.len()
        };
        assert_eq!(positions(&full), FIGURE_TRIANGLES * 3);
        assert_eq!(positions(&first), FIRST_PERSON_TRIANGLES * 3);
        // The eye sits above the torso with the head gone, so looking down
        // shows the body and nothing blocks the forward view.
        let eye = [0.0, height * universe_core::body::EYE_SHARE, 0.0];
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(first_pos)) =
            first.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("first positions");
        };
        let near = height * universe_core::figure::FIRST_PERSON_NEAR_SHARE;
        for p in first_pos {
            let gap = ((f64::from(p[0]) - eye[0]).powi(2)
                + (f64::from(p[1]) - eye[1]).powi(2)
                + (f64::from(p[2]) - eye[2]).powi(2))
            .sqrt();
            assert!(gap > near, "first-person vertices outside the near plane");
        }
    }
}
