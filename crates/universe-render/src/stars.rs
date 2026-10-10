//! Emissive billboards for bright stars and galaxies (#157, ADR 0015).
//!
//! Gizmo spheres carry every marker's hue, but they are LDR dots: they
//! cannot bloom. This module adds one camera-facing emissive quad per
//! bright portal tint, so strong lights glow through [`Bloom`](bevy::post_process::bloom::Bloom)
//! and the filmic tonemap. Dim populations and faint dwarfs stay gizmos
//! only (the brightness gate below); planets keep their meshes; previews
//! keep their gizmos, so entering changes nothing on screen (R7).

use crate::Universe;
use crate::style::to_vec3;
use crate::tokens::dot_floor_world;
use bevy::color::LinearRgba;
use bevy::math::{DVec3, Vec3};
use bevy::mesh::{Mesh, Mesh3d, MeshBuilder, primitives::RectangleMeshBuilder};
use bevy::prelude::*;
use std::f32::consts::PI;
use universe_core::r#gen::{ColorInfo, MarkerKind, Point};
use universe_core::nav::drawn_radius;

/// Minimum tint brightness carrying an emissive billboard.
///
/// The Sun reads `1.0`; O/B/A/F stars read brighter, K/M dwarfs dimmer.
/// Galaxies read `1.0`/`0.9`: spirals and ellipticals glow, irregulars do
/// not. Populations never glow whatever their tint.
pub(crate) const BILLBOARD_BRIGHTNESS_GATE: f64 = 1.0;

/// Glow quad alpha for near bright portals (emissive core over the gizmo).
pub(crate) const BILLBOARD_ALPHA: f32 = 0.8;

/// Glow quad alpha past the far-view gate (impostor record, dimmer).
pub(crate) const IMPOSTOR_ALPHA: f32 = 0.45;

/// Marks entities owned by [`draw_star_billboards`].
#[derive(Component)]
pub(crate) struct StarBillboard {
    /// Cell-local marker position the quad was spawned at.
    position: [f64; 3],
}

/// Tracks which open cell the spawned billboards were built for.
#[derive(Resource, Default)]
pub(crate) struct BillboardState {
    /// Open-cell seed the current billboards belong to, if any.
    seed: Option<u64>,
}

/// Display tint for one marker when it earns a billboard, else `None`.
///
/// Portals whose core tint reaches [`BILLBOARD_BRIGHTNESS_GATE`] glow;
/// everything else (populations, dim dwarfs, untinted points) stays gizmos.
#[must_use]
pub(crate) fn billboard_for(point: &Point) -> Option<ColorInfo> {
    if point.kind != MarkerKind::Portal {
        return None;
    }
    let tint = point.tint?;
    if tint.brightness >= BILLBOARD_BRIGHTNESS_GATE {
        Some(tint)
    } else {
        None
    }
}

/// Emissive HDR color for one billboard tint (linear, brightness applied).
///
/// Values over `1.0` are the point: they push through the filmic tonemap
/// and trip the bloom threshold while gizmo hues stay LDR.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "E-CAST: tint channels narrowed for the GPU, intended"
)]
pub(crate) fn emissive_for(tint: ColorInfo) -> [f32; 3] {
    [
        tint.red as f32 * tint.brightness as f32,
        tint.green as f32 * tint.brightness as f32,
        tint.blue as f32 * tint.brightness as f32,
    ]
}

/// Spawns emissive billboards for the open cell, facing them every frame.
///
/// Rebuilds only when the open cell changes (seed-keyed); per-frame cost is
/// one quaternion plus one scale per bright portal, with no allocation.
/// Quads face the camera (`looking_at` aims `-Z`, so a half turn about `Y`
/// shows the `+Z` quad face); size tracks the drawn marker radius so dives
/// grow the glow with the gizmo underneath.
pub(crate) fn draw_star_billboards(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    universe: Res<Universe>,
    mut state: ResMut<BillboardState>,
    mut billboards: Query<(
        Entity,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &StarBillboard,
        &mut Transform,
    )>,
) {
    let seed = universe.open_seed();
    if state.seed != Some(seed) {
        for (entity, mesh, material, _, _) in &billboards {
            commands.entity(entity).despawn();
            meshes.remove(&mesh.0);
            materials.remove(&material.0);
        }
        state.seed = None;
        let quad = meshes.add(RectangleMeshBuilder::default().build());
        for point in &universe.open.points {
            let Some(tint) = billboard_for(point) else {
                continue;
            };
            let lit = emissive_for(tint);
            let alpha = if point.galaxy.is_some_and(|info| info.far_view) {
                IMPOSTOR_ALPHA
            } else {
                BILLBOARD_ALPHA
            };
            let material = materials.add(StandardMaterial {
                base_color: Color::srgba(0.0, 0.0, 0.0, alpha),
                emissive: LinearRgba::rgb(lit[0], lit[1], lit[2]),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                cull_mode: None,
                ..default()
            });
            commands.spawn((
                StarBillboard {
                    position: point.position,
                },
                Name::new("star-billboard"),
                Mesh3d(quad.clone()),
                MeshMaterial3d(material),
                Transform::from_translation(to_vec3(point.position)),
            ));
        }
        state.seed = Some(seed);
    }
    let camera = DVec3::from_array(universe.path.offset());
    let camera_at = to_vec3(camera.to_array());
    let radius = universe.marker_radius();
    for (_, _, _, marker, mut transform) in &mut billboards {
        let at = DVec3::from_array(marker.position);
        let distance = (camera - at).length();
        if distance < 1e-6 {
            continue;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a radius, intended"
        )]
        let drawn = (drawn_radius(radius, distance) as f32).max(dot_floor_world(distance));
        let facing = to_vec3(marker.position);
        let aimed = Transform::from_translation(facing).looking_at(camera_at, Vec3::Y);
        transform.translation = facing;
        transform.rotation = aimed.rotation * Quat::from_rotation_y(PI);
        transform.scale = Vec3::splat(drawn.max(1e-6) * 2.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a portal point with the given tint (mirrors core samplers).
    fn portal(tint: Option<ColorInfo>) -> Point {
        Point {
            tint,
            ..Point::bare([0.0, 0.0, 0.0], 0.05, MarkerKind::Portal)
        }
    }

    #[test]
    fn gate_glows_bright_portals_only() {
        let sun = portal(Some(ColorInfo {
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            brightness: 1.0,
        }));
        assert!(billboard_for(&sun).is_some(), "the Sun glows");
        let dwarf = portal(Some(ColorInfo {
            red: 1.0,
            green: 0.55,
            blue: 0.35,
            brightness: 0.45,
        }));
        assert!(billboard_for(&dwarf).is_none(), "dim dwarfs stay gizmos");
        let mut population = sun;
        population.kind = MarkerKind::Population;
        assert!(
            billboard_for(&population).is_none(),
            "populations never glow"
        );
        assert!(
            billboard_for(&portal(None)).is_none(),
            "untinted stays gizmo"
        );
    }

    #[test]
    fn emissive_pushes_bright_stars_over_one() {
        let giant = emissive_for(ColorInfo {
            red: 0.55,
            green: 0.68,
            blue: 1.0,
            brightness: 3.0,
        });
        assert!(
            giant[2] > 1.0 && giant[0] < giant[2],
            "O giant reads blue HDR: {giant:?}"
        );
        let sun = emissive_for(ColorInfo {
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            brightness: 1.0,
        });
        assert_eq!(sun, [1.0, 1.0, 1.0]);
    }
}
