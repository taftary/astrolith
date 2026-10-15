//! Moon meshes and air gizmos for the open L10 cell (#375, ADR 0016, ADR 0024).
//!
//! The planet body itself is the one icosphere surface mesh
//! (`surface_mesh`, ADR 0024). This module adds one small body per moon,
//! rebuilt when the open cell changes and despawned when the dive leaves
//! L10, plus the air rim and limb preview gizmos. Everything outside L10
//! stays gizmo indicators under ADR 0002.

use crate::SettingsStore;
use crate::Universe;
use crate::input::Navigation;
use crate::planet_approach::surface_host;
use crate::planet_squares::{SurfaceMaterial, surface_material};
use crate::style::{scaled, to_vec3};
use bevy::asset::RenderAssetUsages;
use bevy::math::Isometry3d;
use bevy::mesh::{
    Indices, Mesh, Mesh3d, MeshBuilder, PrimitiveTopology, primitives::SphereKind,
    primitives::SphereMeshBuilder,
};
use bevy::prelude::*;
use universe_core::frame::{angular_radius, children_brightness};
use universe_core::r#gen::{MarkerKind, Point};
use universe_core::look::{PlanetLook, WorldMode};
use universe_core::surface::SurfaceFrame;
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
    /// Open-cell seed, moon count and whether the moons wear their
    /// Generated types the current meshes belong to, if any (the count
    /// lets a tool that clears the moons rebuild, #654; the look flag
    /// rebuilds on the Normal/Generated switch).
    seed: Option<(u64, usize, bool)>,
}

/// Rim tint for a Generated frame from the air model (#685, T9):
/// [`universe_core::world::clouds::sky_tint`] of the frame's own recipe.
/// `None` for tool-built frames without an orbit context, or for airless
/// worlds (which draw no rim, as before).
#[must_use]
pub(crate) fn generated_rim_tint(frame: &SurfaceFrame) -> Option<[f64; 3]> {
    let orbit = frame.orbit()?;
    let recipe = universe_core::world::biome::recipe_for_frame(frame)?;
    let air = universe_core::world::air::air_for(
        frame.planet_seed(),
        recipe.kind,
        &universe_core::world::air::AirInputs {
            mass_earth: recipe.mass_earth,
            radius_earth: recipe.radius_earth,
            orbit_au: recipe.orbit_au,
            star_mass_solar: recipe.star_mass,
            t_eq_k: recipe.t_eq_k,
            water_01: recipe.water,
            age_gyr: recipe.age_gyr,
        },
    );
    let star = universe_core::system::system_star(orbit.system_seed, orbit.home);
    universe_core::world::clouds::sky_tint(&air, star.kind)
}

/// Narrows a cell-space point for the GPU.
fn narrow(point: [f64; 3]) -> [f32; 3] {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cell-unit ring point narrowed for the GPU, intended"
    )]
    let narrowed = [point[0] as f32, point[1] as f32, point[2] as f32];
    narrowed
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

/// A displayed moon's Generated look: its type tint and potato shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MoonDraw {
    /// Type tint in linear RGB.
    tint: [f64; 3],
    /// Irregular shape in 0..1 (sphere at 0).
    irregularity: f64,
}

/// Type tint in linear RGB (moons guide, kind table; Dead is the grey the
/// moons always wore).
#[must_use]
pub(crate) fn moon_tint(kind: universe_core::world::moons::MoonKind) -> [f64; 3] {
    use universe_core::world::moons::MoonKind;
    match kind {
        MoonKind::Dead => [0.55, 0.55, 0.58],
        MoonKind::Volcanic => [0.85, 0.62, 0.20],
        MoonKind::Ocean => [0.72, 0.84, 0.90],
        MoonKind::Hazy => [0.90, 0.60, 0.30],
        MoonKind::Captured => [0.36, 0.31, 0.28],
        MoonKind::Shepherd => [0.70, 0.70, 0.72],
        MoonKind::Ringed => [0.78, 0.78, 0.80],
    }
}

/// Stream tag of the potato draws.
const POTATO_TAG: u64 = 0x907A_0685_0000_0000;

/// A share in 0..1 from the hash stream.
fn potato_share(seed: u64, lane: u64) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: 53-bit hash share as a unit float, intended"
    )]
    let share = (universe_core::seed::hash_triple(seed, POTATO_TAG, lane) >> 11) as f64
        / 9_007_199_254_740_992.0;
    share.clamp(0.0, 1.0 - f64::EPSILON)
}

/// Moon body mesh of `radius`: a sphere displaced by `irregularity`
/// (moons guide, potato recipe): low-frequency radial noise at 10-40% of
/// the radius plus two crater bites in the outline. `0.0` reads the plain
/// sphere. Deterministic in `(seed, rank)`.
#[must_use]
pub(crate) fn potato_mesh(seed: u64, rank: u64, radius: f32, irregularity: f64) -> Mesh {
    let mut mesh = SphereMeshBuilder::new(
        radius,
        SphereKind::Ico {
            subdivisions: MOON_SUBDIVISIONS,
        },
    )
    .build();
    if !irregularity.is_finite() || irregularity <= 0.0 {
        return mesh;
    }
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return mesh;
    };
    let lane = universe_core::seed::hash_triple(seed, POTATO_TAG, rank);
    // Two bite centres as unit directions from the stream.
    let bite = |salt: u64| {
        let lon = potato_share(seed, rank.wrapping_add(salt)) * std::f64::consts::TAU;
        let lat = (potato_share(seed, rank.wrapping_add(salt + 1)) * 2.0 - 1.0)
            .clamp(-1.0, 1.0)
            .asin();
        [lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin()]
    };
    let bites = [
        (
            bite(11),
            0.35 + 0.15 * potato_share(seed, rank.wrapping_add(13)),
        ),
        (
            bite(21),
            0.35 + 0.15 * potato_share(seed, rank.wrapping_add(23)),
        ),
    ];
    let depths = [
        0.12 + 0.08 * potato_share(seed, rank.wrapping_add(14)),
        0.12 + 0.08 * potato_share(seed, rank.wrapping_add(24)),
    ];
    let mut out = Vec::with_capacity(positions.len() * 3);
    for vertex in positions {
        let point = [
            f64::from(vertex[0]),
            f64::from(vertex[1]),
            f64::from(vertex[2]),
        ];
        let reach = (point[0] * point[0] + point[1] * point[1] + point[2] * point[2]).sqrt();
        if reach <= 0.0 {
            out.push(*vertex);
            continue;
        }
        let dir = [point[0] / reach, point[1] / reach, point[2] / reach];
        let (noise, _) = universe_core::world::noise3::fbm_3d(
            lane,
            [dir[0] * 2.5, dir[1] * 2.5, dir[2] * 2.5],
            3,
            2.0,
            0.5,
        );
        let mut scale = 1.0 + irregularity * 0.30 * noise.clamp(-1.0, 1.0);
        for ((centre, width), depth) in bites.iter().zip(depths.iter()) {
            let along =
                (dir[0] * centre[0] + dir[1] * centre[1] + dir[2] * centre[2]).clamp(-1.0, 1.0);
            let angle = along.acos() / width;
            scale -= irregularity * depth * (-angle * angle).exp();
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: displaced moon vertex narrowed for the GPU, intended"
        )]
        let displaced = [
            (point[0] * scale) as f32,
            (point[1] * scale) as f32,
            (point[2] * scale) as f32,
        ];
        out.push(displaced);
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, out);
    mesh
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
    let moons = universe
        .open
        .points
        .iter()
        .filter(|point| point.moon.is_some())
        .count();
    let generated = universe.world.mode == WorldMode::Generated;
    let seed = (universe.open_seed(), moons, generated);
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
    // Typed moons in Generated frames with an orbit context: display rank
    // i wears system moon i (display orbits stay compressed, so only the
    // type and shape cross the seam). Classic keeps the grey spheres.
    let typed: Option<Vec<MoonDraw>> = if generated {
        universe.frame.as_ref().and_then(|frame| {
            let recipe = universe_core::world::biome::recipe_for_frame(frame)?;
            // The display record's day drives the synchronous radius (the
            // generator's own spin arrives with T12's sampling rewire).
            let spin = universe
                .open
                .points
                .iter()
                .find_map(|point| point.surface)
                .map_or(universe_core::terrain::EARTH_SPIN_HOURS, |surface| {
                    surface.spin_hours
                });
            let spec = universe_core::world::moons::PlanetSpec::from_recipe(&recipe, spin);
            Some(
                universe_core::world::moons::system_for(frame.planet_seed(), &spec)
                    .moons
                    .into_iter()
                    .map(|moon| MoonDraw {
                        tint: moon_tint(moon.kind),
                        irregularity: moon.irregularity_01,
                    })
                    .collect(),
            )
        })
    } else {
        None
    };
    let planet_seed = universe.open_seed();
    let mut rank = 0usize;
    for point in &universe.open.points {
        if point.moon.is_none() {
            continue;
        }
        let draw = typed.as_ref().and_then(|typed| typed.get(rank));
        rank += 1;
        let (material, moon) = match draw {
            Some(draw) => {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "E-CAST: 0..1 moon tint narrowed for the GPU, intended"
                )]
                let tint = Color::srgb(
                    draw.tint[0] as f32,
                    draw.tint[1] as f32,
                    draw.tint[2] as f32,
                );
                let rank = u64::try_from(rank).unwrap_or(u64::MAX);
                (
                    materials.add(surface_material(StandardMaterial {
                        base_color: tint,
                        unlit: true,
                        ..default()
                    })),
                    potato_mesh(planet_seed, rank, MOON_RADIUS_CELL, draw.irregularity),
                )
            }
            None => {
                let moon = SphereMeshBuilder::new(
                    MOON_RADIUS_CELL,
                    SphereKind::Ico {
                        subdivisions: MOON_SUBDIVISIONS,
                    },
                )
                .build();
                (gray.clone(), moon)
            }
        };
        commands.spawn((
            PlanetMesh,
            Name::new("planet-moon"),
            Mesh3d(meshes.add(moon)),
            MeshMaterial3d(material),
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
    // Generated orbit-aware frames answer from the air model (T9):
    // `Some(tint)` re-tints the rim, `Some(None)` is airless and draws
    // nothing even when size alone would hold air, `None` keeps the
    // size-based tint (Classic, or frames without an orbit context).
    let model = if universe.world.mode == WorldMode::Generated
        && let Some(frame) = universe.frame.as_ref()
        && frame.orbit().is_some()
    {
        Some(generated_rim_tint(frame))
    } else {
        None
    };
    if matches!(model, Some(None)) {
        return;
    }
    // Generated frames tint the rim from the air model (T9); Classic
    // keeps the size-based tint exactly as before.
    let tint = if let Some(Some(model)) = model {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: air tint narrowed for the GPU, intended"
        )]
        let tint_r = model[0] as f32;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: air tint narrowed for the GPU, intended"
        )]
        let tint_g = model[1] as f32;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: air tint narrowed for the GPU, intended"
        )]
        let tint_b = model[2] as f32;
        Color::srgb(tint_r, tint_g, tint_b)
    } else {
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
        Color::srgb(tint_r, tint_g, tint_b)
    };
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

/// Ring mesh segments around the planet.
const RING_SEGMENTS: u32 = 96;

/// Ring mesh bands across the ring.
const RING_BANDS: u32 = 6;

/// Marks entities owned by [`draw_rings`].
#[derive(Component)]
pub(crate) struct RingMesh;

/// Tracks which planet the spawned ring was built for.
#[derive(Resource, Default)]
pub(crate) struct RingState {
    /// Planet seed the current ring belongs to, if any.
    seed: Option<u64>,
    /// Strength the ring material was last set to.
    fade: f64,
}

/// Ring inputs for a frame: planet seed and spin-axis direction, when the
/// frame wears rings (`None` without an orbit context or on ringless
/// seeds, whose rarity follows kind and youth in
/// [`universe_core::world::giants::has_rings`]).
#[must_use]
pub(crate) fn ring_for(frame: &SurfaceFrame) -> Option<(u64, [f64; 3])> {
    let orbit = frame.orbit()?;
    let recipe = universe_core::world::biome::recipe_for_frame(frame)?;
    if !universe_core::world::giants::has_rings(frame.planet_seed(), recipe.kind, recipe.age_gyr) {
        return None;
    }
    let body = universe_core::terrain::body_recipe(frame.planet_seed(), orbit.home);
    let tilt = body.tilt_deg.to_radians();
    Some((frame.planet_seed(), [tilt.sin(), tilt.cos(), 0.0]))
}

/// Annulus mesh in the XZ plane (equatorial) centred on the origin, radii
/// in cell units, with per-vertex alpha from the face-on ring profile of
/// `seed` ([`universe_core::world::giants::ring_alpha`]: dense bands read
/// opaque, gaps clear).
#[must_use]
pub(crate) fn ring_mesh(seed: u64, inner: f64, outer: f64) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    for band in 0..=RING_BANDS {
        let share = f64::from(band) / f64::from(RING_BANDS);
        let radius = inner + (outer - inner) * share;
        let alpha = universe_core::world::giants::ring_alpha(seed, share, 1.0);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: 0..1 ring alpha narrowed for the GPU, intended"
        )]
        let alpha = alpha as f32;
        for segment in 0..=RING_SEGMENTS {
            let angle = f64::from(segment) / f64::from(RING_SEGMENTS) * std::f64::consts::TAU;
            let (sin, cos) = angle.sin_cos();
            positions.push(narrow([radius * cos, 0.0, radius * sin]));
            normals.push(narrow([0.0, 1.0, 0.0]));
            colors.push([1.0, 1.0, 1.0, alpha]);
        }
    }
    let mut indices = Vec::new();
    let row = RING_SEGMENTS + 1;
    for band in 0..RING_BANDS {
        for segment in 0..RING_SEGMENTS {
            let a = band * row + segment;
            let b = a + 1;
            let c = a + row;
            let d = c + 1;
            indices.extend([a, c, b, b, c, d]);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Ring material at `fade` strength: translucent always (the vertex alpha
/// carries the ring profile), fading with the host surface.
fn ring_material(fade: f64) -> StandardMaterial {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: a 0..1 strength narrowed for a colour, intended"
    )]
    let alpha = fade.clamp(0.0, 1.0) as f32;
    StandardMaterial {
        base_color: Color::WHITE.with_alpha(alpha),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    }
}

/// Spawns the ring annulus for a ringed Generated planet, despawning it
/// elsewhere (#685, T9).
///
/// Rebuilds only when the ringed planet changes (seed-keyed, following
/// the surface host like the mesh, so rings show on approach too);
/// per-frame cost is the host lookup plus a fade check. Ringless seeds,
/// Classic looks and views without a surface host draw nothing.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn draw_rings(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SurfaceMaterial>>,
    universe: Res<Universe>,
    store: Res<SettingsStore>,
    mut state: ResMut<RingState>,
    owned: Query<(Entity, &Mesh3d, &MeshMaterial3d<SurfaceMaterial>), With<RingMesh>>,
) {
    let host = surface_host(&universe);
    let ring = if store.0.planet_look == PlanetLook::Generated {
        host.as_ref().and_then(|host| ring_for(&host.frame))
    } else {
        None
    };
    let key = ring.map(|(seed, _)| seed);
    if key != state.seed {
        for (entity, mesh, material) in &owned {
            commands.entity(entity).despawn();
            meshes.remove(&mesh.0);
            materials.remove(&material.0);
        }
        state.seed = None;
    }
    let (Some(host), Some((seed, axis))) = (host, ring) else {
        return;
    };
    if state.seed == key {
        // Same ring: follow the host fade without rebuilding.
        if (host.fade - state.fade).abs() > 1e-6 {
            let material = ring_material(host.fade);
            for (_, _, owned) in &owned {
                if let Some(mut slot) = materials.get_mut(&owned.0) {
                    slot.base.base_color = material.base_color;
                }
            }
            state.fade = host.fade;
        }
        return;
    }
    let (inner_share, outer_share) = universe_core::world::giants::ring_edges(seed);
    let mesh = ring_mesh(
        seed,
        inner_share * PLANET_RADIUS_CELL,
        outer_share * PLANET_RADIUS_CELL,
    );
    let rotation = Quat::from_rotation_arc(Vec3::Y, to_vec3(axis));
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: a cell ratio narrowed for the GPU transform, intended"
    )]
    let ratio = host.ratio as f32;
    commands.spawn((
        RingMesh,
        Name::new("planet-ring"),
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(surface_material(ring_material(host.fade)))),
        Transform {
            rotation,
            scale: Vec3::splat(ratio),
            ..default()
        },
        crate::style::Anchored(host.offset),
    ));
    state.seed = key;
    state.fade = host.fade;
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

    /// A Generated frame with an orbit context.
    fn generated_frame(seed: u64, orbit_au: f64, period_days: f64) -> SurfaceFrame {
        use universe_core::look::{ReliefFactor, WorldMode, WorldOptions};
        SurfaceFrame::planet(seed)
            .with_world(WorldOptions {
                mode: WorldMode::Generated,
                relief: ReliefFactor::default(),
            })
            .with_orbit(universe_core::surface::OrbitContext {
                orbit_au,
                period_days,
                system_seed: 9,
                home: false,
            })
    }

    #[test]
    fn generated_rim_follows_the_air_model() {
        // T9: an Earth-like context tints blue, an airless Mercury-like
        // context draws nothing, tool frames without an orbit keep the
        // size-based path (`None` here; the caller falls back).
        let tint = generated_rim_tint(&generated_frame(7, 1.0, 365.25)).expect("earth holds a rim");
        assert!(
            tint[2] > tint[0] + 0.1 && tint[1] > tint[0],
            "earth rim not blue: {tint:?}"
        );
        assert_eq!(
            tint,
            generated_rim_tint(&generated_frame(7, 1.0, 365.25)).expect("replay"),
            "replay differs"
        );
        assert_eq!(
            generated_rim_tint(&airless_frame()),
            None,
            "airless world tinted"
        );
        assert_eq!(generated_rim_tint(&SurfaceFrame::planet(7)), None);
    }

    /// First airless frame in range (close-in orbit; the recipe draws its
    /// own mass, so airlessness is searched, not assumed).
    fn airless_frame() -> SurfaceFrame {
        (0..500u64)
            .map(|seed| generated_frame(seed, 0.39, 88.0))
            .find(|frame| generated_rim_tint(frame).is_none())
            .expect("no airless world in range")
    }

    #[test]
    fn ring_mesh_wears_the_profile() {
        // T9: one quad grid with the face-on profile as vertex alpha,
        // replayed exactly; dense bands read opaque, gaps clear.
        let (inner, outer) = universe_core::world::giants::ring_edges(7);
        let mesh = ring_mesh(7, inner * PLANET_RADIUS_CELL, outer * PLANET_RADIUS_CELL);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("ring carries positions");
        };
        assert_eq!(
            positions.len(),
            (RING_SEGMENTS + 1) as usize * (RING_BANDS + 1) as usize
        );
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("ring carries colours");
        };
        assert_eq!(colors.len(), positions.len());
        let mut densest = 0.0f32;
        let mut thinnest = f32::INFINITY;
        for colour in colors {
            assert!((0.0..=1.0).contains(&colour[3]), "alpha out of share");
            assert_eq!((colour[0], colour[1], colour[2]), (1.0, 1.0, 1.0));
            densest = densest.max(colour[3]);
            thinnest = thinnest.min(colour[3]);
        }
        assert!(densest > 0.5, "no dense band: {densest}");
        assert!(thinnest < 0.35, "no gaps: {thinnest}");
        let again = ring_mesh(7, inner * PLANET_RADIUS_CELL, outer * PLANET_RADIUS_CELL);
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(again)) =
            again.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("ring carries colours");
        };
        assert_eq!(colors, again, "replay differs");
        // The ring clears the body and spans outward.
        assert!(inner * PLANET_RADIUS_CELL > PLANET_RADIUS_CELL);
        assert!(outer > inner);
    }

    #[test]
    fn moon_tints_name_every_kind_with_dead_on_grey() {
        // T10: Dead keeps the grey the moons always wore (Classic reads
        // the same path); every kind replays its tint.
        use universe_core::world::moons::MoonKind;
        assert_eq!(moon_tint(MoonKind::Dead), [0.55, 0.55, 0.58]);
        for kind in MoonKind::ALL {
            let tint = moon_tint(kind);
            assert!(
                tint.iter()
                    .all(|c| c.is_finite() && (0.0..=1.0).contains(c))
            );
            assert_eq!(tint, moon_tint(kind), "replay differs");
        }
        assert_ne!(moon_tint(MoonKind::Volcanic), moon_tint(MoonKind::Captured));
    }

    #[test]
    fn potato_moons_leave_the_sphere_behind() {
        // T10: still a sphere at 0, lumpy within the potato amplitude at
        // 1, replayed exactly.
        let plain = potato_mesh(7, 0, MOON_RADIUS_CELL, 0.0);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(sphere)) =
            plain.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("moon carries positions");
        };
        let lumpy = potato_mesh(7, 0, MOON_RADIUS_CELL, 1.0);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(potato)) =
            lumpy.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("moon carries positions");
        };
        assert_eq!(sphere.len(), potato.len());
        let mut peak = 0.0f32;
        let mut moved = false;
        for (flat, bent) in sphere.iter().zip(potato.iter()) {
            let was = (flat[0] * flat[0] + flat[1] * flat[1] + flat[2] * flat[2]).sqrt();
            let is = (bent[0] * bent[0] + bent[1] * bent[1] + bent[2] * bent[2]).sqrt();
            peak = peak.max((is - was).abs() / was);
            moved = moved || (is - was).abs() / was > 0.01;
        }
        assert!(moved, "no lump anywhere");
        assert!(peak < 0.45, "potato overflows: {peak}");
        let again = potato_mesh(7, 0, MOON_RADIUS_CELL, 1.0);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(again)) =
            again.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("moon carries positions");
        };
        assert_eq!(potato, again, "replay differs");
        assert_ne!(potato, sphere);
    }
}
