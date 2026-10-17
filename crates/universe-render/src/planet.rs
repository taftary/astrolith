//! Moon meshes, the far ball, and air gizmos for the open L10 cell (#742).
//!
//! The planet ground itself is the detail window
//! ([`crate::patch::draw_patch`]); this module adds the coarse
//! whole-planet ball beside it ([`draw_far_ball`], the far field of the
//! #742 crossfade) plus one small body per moon, rebuilt when the open
//! cell changes and despawned when the dive leaves L10, plus the air rim
//! and limb preview gizmos. Everything outside L10 stays gizmo indicators
//! under ADR 0002.

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
use universe_core::terrain::{BodyRecipe, EARTH_RADIUS_KM, PLANET_RADIUS_CELL};

/// Icosphere subdivisions for moons (42 vertices; moons are dots with shading).
pub(crate) const MOON_SUBDIVISIONS: u32 = 1;

/// Latitude bands of the coarse far ball (pole to pole).
const BALL_LATS: usize = 12;

/// Longitude segments around the far ball.
const BALL_LONGS: usize = 24;

/// Marks entities owned by [`draw_far_ball`].
#[derive(Component)]
pub(crate) struct FarBall;

/// Tracks which surface the spawned far ball was built for.
#[derive(Resource, Default)]
pub(crate) struct FarBallState {
    /// Open-cell seed, level, and approached planet marker the current ball
    /// belongs to, if any.
    key: Option<(u64, u8, Option<u32>)>,
    /// Strength the material was last set to.
    fade: f64,
}

/// Radius of the coarse far ball in planet units: the lowest face plane of
/// the flat-faced surface, tucked a hair under it, so the ball never pokes
/// through the detail window where the two overlap. Pure.
#[must_use]
pub(crate) fn ball_radius() -> f64 {
    universe_core::terrain::surface_floor_radius() * 0.99
}

/// Far-ball triangles: a coarse UV sphere of [`ball_radius`] around the
/// planet centre, in planet units. Fixed budget, pure and deterministic.
#[expect(
    clippy::cast_precision_loss,
    reason = "E-CAST: band counters below 2^5, exactly representable"
)]
#[must_use]
pub(crate) fn ball_triangles() -> Vec<[[f64; 3]; 3]> {
    let radius = ball_radius();
    let node = |lat: usize, long: usize| {
        let theta = std::f64::consts::PI * (lat as f64) / (BALL_LATS as f64);
        let phi = 2.0 * std::f64::consts::PI * (long as f64) / (BALL_LONGS as f64);
        let (sin_theta, cos_theta) = theta.sin_cos();
        [
            radius * sin_theta * phi.cos(),
            radius * cos_theta,
            radius * sin_theta * phi.sin(),
        ]
    };
    let mut triangles = Vec::with_capacity(2 * BALL_LATS * BALL_LONGS);
    for long in 0..BALL_LONGS {
        let next = (long + 1) % BALL_LONGS;
        triangles.push([node(0, long), node(1, long), node(1, next)]);
    }
    for lat in 1..BALL_LATS - 1 {
        for long in 0..BALL_LONGS {
            let next = (long + 1) % BALL_LONGS;
            triangles.push([node(lat, long), node(lat + 1, long), node(lat + 1, next)]);
            triangles.push([node(lat, long), node(lat + 1, next), node(lat, next)]);
        }
    }
    for long in 0..BALL_LONGS {
        let next = (long + 1) % BALL_LONGS;
        triangles.push([
            node(BALL_LATS - 1, long),
            node(BALL_LATS, long),
            node(BALL_LATS, next),
        ]);
    }
    triangles
}

/// Spawns the coarse whole-planet ball for the open L10 cell, or for the
/// planet being approached from L9, despawning it elsewhere (#742).
///
/// The ball is static (rebuilt only when the surface changes); per-frame
/// cost is the crossfade follow otherwise. It draws at the host fade times
/// the ball weight, complementing the detail window with no holes. The
/// mesh carries [`FarBall`] and a `Name`, so inspectors and logs can say
/// which entity.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn draw_far_ball(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    universe: Res<Universe>,
    mut state: ResMut<FarBallState>,
    mut owned: Query<
        (
            Entity,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &mut Visibility,
        ),
        With<FarBall>,
    >,
) {
    use crate::patch::{ball_weight, faded, height_above, patch_host, patch_mesh_of};
    let host = patch_host(&universe);
    let key = host
        .as_ref()
        .map(|host| (universe.open_seed(), host.frame.level().get(), host.marker));
    if key != state.key {
        for (entity, mesh, material, _) in &owned {
            commands.entity(entity).despawn();
            meshes.remove(&mesh.0);
            materials.remove(&material.0);
        }
        state.key = None;
    }
    let Some(host) = host else {
        return;
    };
    let camera = universe.path.offset();
    let anchor = [
        (camera[0] - host.offset[0]) / host.ratio,
        (camera[1] - host.offset[1]) / host.ratio,
        (camera[2] - host.offset[2]) / host.ratio,
    ];
    let fade = host.fade * ball_weight(height_above(&host.frame, anchor));
    if state.key == key {
        // Same ball: follow the crossfade without rebuilding.
        if (fade - state.fade).abs() > 1e-6 {
            for (_, _, material, mut visibility) in &mut owned {
                if let Some(mut material) = materials.get_mut(&material.0) {
                    faded(&mut material, fade);
                }
                *visibility = if fade > 0.0 {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
            state.fade = fade;
        }
        return;
    }
    let Some(mesh) = patch_mesh_of(&ball_triangles(), anchor) else {
        return;
    };
    let mut material = StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        cull_mode: None,
        ..default()
    };
    faded(&mut material, fade);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: a cell ratio narrowed for the GPU transform, intended"
    )]
    let scale = Transform::from_scale(Vec3::splat(host.ratio as f32));
    let visibility = if fade > 0.0 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    commands.spawn((
        FarBall,
        Name::new("far-ball"),
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(material)),
        scale,
        visibility,
        crate::style::Anchored(camera),
    ));
    state.key = key;
    state.fade = fade;
}

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

/// Draws the planet limb and air preview around the targeted region (#430).
///
/// Gizmo circles only, never a mesh (`E-RENDER-NO-MESH`): the limb circle
/// about the planet centre plus the air rim, both turned into the target
/// patch's tangent plane, fading in by the target's angular size. At the
/// open instant the limb matches the kept parent context the region draws
/// after entry, so despawning the L10 body mesh swaps no full-weight
/// element. Allocates nothing per frame (`E-HOT-NOALLOC`).
pub(crate) fn draw_limb_preview(
    mut gizmos: Gizmos,
    universe: Res<Universe>,
    nav: Res<Navigation>,
    origin: Res<crate::style::RenderOrigin>,
) {
    let cell = origin.cell();
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
    gizmos.circle(Isometry3d::new(cell, rotation), limb, limb_color);
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
        Isometry3d::new(cell, rotation),
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
    fn home_cell_names_its_body() {
        let cell = TerrainSampler::home().generate(7, &demo_budget());
        let body = body_for(&cell.points).expect("home cell names its body");
        assert!(body.radius_earth > 0.0);
        assert!(body_for(&[]).is_none());
    }

    #[test]
    fn far_ball_tucks_under_the_detail_ground() {
        use universe_core::surface::SurfaceFrame;
        let chain = universe_core::nest::autopilot_path(universe_core::nav::DEMO_SEED);
        let frame =
            SurfaceFrame::for_path(universe_core::nav::DEMO_SEED, &chain).expect("planet frame");
        let radius = ball_radius();
        assert!(radius > 0.0, "the ball exists");
        // Every sampled face outranks the ball: no poke-through where the
        // window and the ball overlap.
        for lat in 0..=6 {
            for long in 0..12 {
                let theta = std::f64::consts::PI * f64::from(lat) / 6.0;
                let phi = 2.0 * std::f64::consts::PI * f64::from(long) / 12.0;
                let (sin_theta, cos_theta) = theta.sin_cos();
                let dir = [sin_theta * phi.cos(), cos_theta, sin_theta * phi.sin()];
                let ground = frame.surface_point(dir);
                let ground_radius =
                    (ground[0] * ground[0] + ground[1] * ground[1] + ground[2] * ground[2]).sqrt();
                assert!(
                    radius < ground_radius,
                    "ball under the ground: {radius} vs {ground_radius}"
                );
            }
        }
        let triangles = ball_triangles();
        assert_eq!(
            triangles.len(),
            2 * BALL_LATS * BALL_LONGS - 2 * BALL_LONGS,
            "fixed coarse budget"
        );
        assert_eq!(triangles, ball_triangles(), "the ball is deterministic");
    }
}
