//! Patch-only planet ground (#729, ADR 0028).
//!
//! The open L10 planet draws as one dome mesh ([`PatchMesh`]): a fixed
//! grid bent onto the sphere around the follow focus (the walking body,
//! else the camera's ground point), sampling the planet-fixed surface so
//! terrain never slides. No ball, no markers, no edge lines, no skirts:
//! the grid is continuous, so neighbours never crack. During the L9
//! approach the dome of the nearest fading planet draws first, scaled by
//! the cell ratio like the old surface host. Everything here is computed
//! in `f64` and narrowed to `f32` only for the GPU.

use crate::Universe;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;
use universe_core::coords::Level;
use universe_core::frame::angular_radius;
use universe_core::nest::{approach_fade, child_ratio};
use universe_core::surface::SurfaceFrame;

/// The focus moves this share of its height before the mesh rebuilds.
const REBUILD_SHARE: f64 = 0.2;

/// Marks the patch dome entity owned by [`draw_patch`].
#[derive(Component)]
pub(crate) struct PatchMesh;

/// Tracks which surface and focus the spawned dome was built for.
#[derive(Resource, Default)]
pub(crate) struct PatchState {
    /// Open-cell seed, level, and approached planet marker the current dome
    /// belongs to, if any.
    key: Option<(u64, u8, Option<u32>)>,
    /// Strength the material was last set to.
    fade: f64,
    /// Follow focus (surface-cell units) the dome was built for.
    focus: Option<[f64; 3]>,
    /// Mapped camera (surface-cell units) the dome was built for.
    camera: Option<[f64; 3]>,
    /// Rim ring in open-cell units, drawn as the shown circle edge.
    rim: Vec<[f64; 3]>,
}

/// The surface to draw for `universe` this frame, if any.
///
/// Inside L10 it is the open cell's frame at full strength. At L9 it is
/// the nearest planet portal whose fade is above zero, mapped into its
/// own units. `None` anywhere else: below L9 there is no planet near, and
/// below L10 nothing opens.
struct PatchHost {
    /// The surface cell (the open L10 cell, or the approached planet).
    frame: SurfaceFrame,
    /// The approached planet's marker in the open cell (`None` inside).
    marker: Option<u32>,
    /// Where the surface cell's centre sits in open-cell units.
    offset: [f64; 3],
    /// Surface-cell units per open-cell unit (`1.0` inside).
    ratio: f64,
    /// Strength the dome draws at, `0.0..=1.0`.
    fade: f64,
}

/// Strength of the planet surface seen from `camera` in its own cell's
/// units: the fade of the cell shell (radius `0.5`) at that distance.
#[must_use]
fn shell_fade(camera: [f64; 3]) -> f64 {
    let distance = (camera[0] * camera[0] + camera[1] * camera[1] + camera[2] * camera[2]).sqrt();
    approach_fade(angular_radius(0.5, distance))
}

#[must_use]
fn patch_host(universe: &Universe) -> Option<PatchHost> {
    let camera = universe.path.offset();
    if universe.level().get() == 10 {
        let frame = universe.frame.clone()?;
        return Some(PatchHost {
            frame,
            marker: None,
            offset: [0.0; 3],
            ratio: 1.0,
            fade: shell_fade(camera),
        });
    }
    if universe.level().get() != 9 {
        return None;
    }
    let ratio = child_ratio(Level::new(9)?)?;
    let (marker, offset, fade) = universe
        .open
        .points
        .iter()
        .enumerate()
        .filter(|(_, point)| point.kind == universe_core::r#gen::MarkerKind::Portal)
        .filter_map(|(index, point)| {
            let at = point.portal_position();
            let inside = [
                (camera[0] - at[0]) / ratio,
                (camera[1] - at[1]) / ratio,
                (camera[2] - at[2]) / ratio,
            ];
            let fade = shell_fade(inside);
            (fade > 0.0).then(|| (u32::try_from(index).ok(), at, fade))
        })
        .filter_map(|(index, at, fade)| Some((index?, at, fade)))
        .max_by(|a, b| a.2.total_cmp(&b.2))?;
    let root = universe.root.0;
    let mut chain = universe.path.indices();
    chain.push(marker);
    let frame = SurfaceFrame::for_path(root, &chain)?;
    Some(PatchHost {
        frame,
        marker: Some(marker),
        offset,
        ratio,
        fade,
    })
}

/// Focus point (open-cell units) the dome centres on.
///
/// The placed walking body when normal mode runs, else the camera: the
/// dome stays under the feet on the ground and under the camera's ground
/// point in flight. Allocation-free (`E-HOT-NOALLOC`).
#[must_use]
pub(crate) fn patch_focus(body: Option<[f64; 3]>, camera: [f64; 3]) -> [f64; 3] {
    body.unwrap_or(camera)
}

/// Rings of the hemisphere grid (pole to rim).
const HEMI_RINGS: usize = 24;

/// Segments around each hemisphere ring.
const HEMI_SEGMENTS: usize = 96;

/// Orthonormal tangent basis at unit `centre`, deterministic.
fn tangent_basis(centre: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let reference = if centre[1].abs() > 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let mut u = [
        centre[1] * reference[2] - centre[2] * reference[1],
        centre[2] * reference[0] - centre[0] * reference[2],
        centre[0] * reference[1] - centre[1] * reference[0],
    ];
    let length = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
    if length.is_finite() && length > 0.0 {
        u = [u[0] / length, u[1] / length, u[2] / length];
    }
    let v = [
        centre[1] * u[2] - centre[2] * u[1],
        centre[2] * u[0] - centre[0] * u[2],
        centre[0] * u[1] - centre[1] * u[0],
    ];
    (u, v)
}

/// Dome triangles around `centre` (planet axes), in planet units.
///
/// A polar grid over the complete hemisphere facing `centre`: the pole
/// vertex, then rings out to exactly PI/2, each node re-projected onto
/// the flat-faced surface, so every sample is planet-fixed. Fixed
/// resolution (`HEMI_RINGS` by `HEMI_SEGMENTS`), pure and deterministic.
#[expect(
    clippy::cast_precision_loss,
    reason = "E-CAST: ring and segment counters below 2^7, exactly representable"
)]
#[must_use]
pub(crate) fn patch_dome(frame: &SurfaceFrame, centre: [f64; 3]) -> Vec<[[f64; 3]; 3]> {
    let length = (centre[0] * centre[0] + centre[1] * centre[1] + centre[2] * centre[2]).sqrt();
    let centre = if length.is_finite() && length > 0.0 {
        [centre[0] / length, centre[1] / length, centre[2] / length]
    } else {
        [1.0, 0.0, 0.0]
    };
    let (u, v) = tangent_basis(centre);
    let half_pi = std::f64::consts::FRAC_PI_2;
    let node = |ring: usize, segment: usize| {
        let theta = half_pi * (ring as f64) / (HEMI_RINGS as f64);
        let phi = 2.0 * std::f64::consts::PI * (segment as f64) / (HEMI_SEGMENTS as f64);
        let (sin_theta, cos_theta) = theta.sin_cos();
        let dir = [
            centre[0] * cos_theta + (u[0] * phi.cos() + v[0] * phi.sin()) * sin_theta,
            centre[1] * cos_theta + (u[1] * phi.cos() + v[1] * phi.sin()) * sin_theta,
            centre[2] * cos_theta + (u[2] * phi.cos() + v[2] * phi.sin()) * sin_theta,
        ];
        frame.surface_point(dir)
    };
    let pole = frame.surface_point(centre);
    let mut triangles = Vec::with_capacity(HEMI_SEGMENTS * (2 * HEMI_RINGS - 1));
    for k in 0..HEMI_SEGMENTS {
        let next = (k + 1) % HEMI_SEGMENTS;
        triangles.push([pole, node(1, k), node(1, next)]);
    }
    for ring in 1..HEMI_RINGS {
        for k in 0..HEMI_SEGMENTS {
            let next = (k + 1) % HEMI_SEGMENTS;
            let (a, b, c, d) = (
                node(ring, k),
                node(ring, next),
                node(ring + 1, k),
                node(ring + 1, next),
            );
            triangles.push([a, c, d]);
            triangles.push([a, d, b]);
        }
    }
    triangles
}

/// Rim ring of the dome around `centre` (planet axes), in planet units.
///
/// The outermost grid ring in order, sitting at exactly PI/2: the shown
/// circle edge of the hemisphere. Pure and deterministic.
#[expect(
    clippy::cast_precision_loss,
    reason = "E-CAST: segment counters below 2^7, exactly representable"
)]
#[must_use]
pub(crate) fn dome_rim(frame: &SurfaceFrame, centre: [f64; 3]) -> Vec<[f64; 3]> {
    let length = (centre[0] * centre[0] + centre[1] * centre[1] + centre[2] * centre[2]).sqrt();
    let centre = if length.is_finite() && length > 0.0 {
        [centre[0] / length, centre[1] / length, centre[2] / length]
    } else {
        [1.0, 0.0, 0.0]
    };
    let (u, v) = tangent_basis(centre);
    let mut rim = Vec::with_capacity(HEMI_SEGMENTS);
    for k in 0..HEMI_SEGMENTS {
        let phi = 2.0 * std::f64::consts::PI * (k as f64) / (HEMI_SEGMENTS as f64);
        let dir = [
            u[0] * phi.cos() + v[0] * phi.sin(),
            u[1] * phi.cos() + v[1] * phi.sin(),
            u[2] * phi.cos() + v[2] * phi.sin(),
        ];
        rim.push(frame.surface_point(dir));
    }
    rim
}

/// Narrows a point for the GPU.
fn narrow(point: [f64; 3]) -> [f32; 3] {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: surface point narrowed for the GPU, intended"
    )]
    let narrowed = [point[0] as f32, point[1] as f32, point[2] as f32];
    narrowed
}

/// Builds the dome mesh of `triangles` in planet units, with vertices
/// relative to `anchor` (an [`crate::style::Anchored`] entity places the
/// mesh back at the open-cell anchor each frame, scaled by the cell
/// ratio).
///
/// One flat-coloured triangle list in the base surface tone with flat
/// face normals. Returns `None` when nothing is drawn.
#[must_use]
fn patch_mesh_of(triangles: &[[[f64; 3]; 3]], anchor: [f64; 3]) -> Option<Mesh> {
    use crate::tokens::SURFACE_TONE;
    if triangles.is_empty() {
        return None;
    }
    let tone = SURFACE_TONE.to_linear().to_f32_array();
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(triangles.len() * 3);
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(triangles.len() * 3);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(triangles.len() * 3);
    for [a, b, c] in triangles {
        let e0 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let e1 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let mut normal = [
            e0[1] * e1[2] - e0[2] * e1[1],
            e0[2] * e1[0] - e0[0] * e1[2],
            e0[0] * e1[1] - e0[1] * e1[0],
        ];
        let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if length.is_finite() && length > 0.0 {
            normal = [normal[0] / length, normal[1] / length, normal[2] / length];
        }
        // Outward: away from the planet centre (frames are planet-centred).
        let mid = [
            (a[0] + b[0] + c[0]) / 3.0,
            (a[1] + b[1] + c[1]) / 3.0,
            (a[2] + b[2] + c[2]) / 3.0,
        ];
        let facing = normal[0] * mid[0] + normal[1] * mid[1] + normal[2] * mid[2];
        let sign = if facing < 0.0 { -1.0 } else { 1.0 };
        let normal = [normal[0] * sign, normal[1] * sign, normal[2] * sign];
        for point in [*a, *b, *c] {
            positions.push(narrow([
                point[0] - anchor[0],
                point[1] - anchor[1],
                point[2] - anchor[2],
            ]));
            colors.push(tone);
            normals.push(narrow(normal));
        }
    }
    let count = u32::try_from(positions.len()).ok()?;
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32((0..count).collect()));
    Some(mesh)
}

/// Material for the dome at `fade` strength: opaque at full strength,
/// alpha-blended while it fades.
fn faded(material: &mut StandardMaterial, fade: f64) {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: a 0..1 strength narrowed for a colour, intended"
    )]
    let alpha = fade.clamp(0.0, 1.0) as f32;
    material.base_color = Color::WHITE.with_alpha(alpha);
    material.alpha_mode = if alpha >= 1.0 {
        AlphaMode::Opaque
    } else {
        AlphaMode::Blend
    };
}

/// Gap between two points.
fn gap(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Height of the focus above the surface (planet units, at least tiny).
fn height_above(frame: &SurfaceFrame, focus: [f64; 3]) -> f64 {
    let reach = gap(focus, [0.0; 3]);
    let ground = gap(frame.surface_point(focus), [0.0; 3]);
    let height = reach - ground;
    if height.is_finite() {
        height.abs().max(1e-9)
    } else {
        1e-9
    }
}

/// Spawns the patch dome for the open L10 cell, or for the planet being
/// approached from L9, despawning it elsewhere. The dome draws at the
/// host's fade.
///
/// Rebuilds when the surface changes, the camera or the follow focus has
/// moved more than [`REBUILD_SHARE`] of its height; per-frame cost is a
/// few probes otherwise. The mesh carries [`PatchMesh`] and a `Name`, so
/// inspectors and logs can say which entity.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn draw_patch(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut gizmos: Gizmos,
    universe: Res<Universe>,
    normal: Res<crate::normal_mode::NormalState>,
    origin: Res<crate::style::RenderOrigin>,
    mut state: ResMut<PatchState>,
    mut owned: Query<
        (
            Entity,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &mut Visibility,
        ),
        With<PatchMesh>,
    >,
) {
    let host = patch_host(&universe);
    let key = host
        .as_ref()
        .map(|host| (universe.open_seed(), host.frame.level().get(), host.marker));
    let camera = universe.path.offset();
    // The follow focus in open-cell units: the placed body on foot, else
    // the camera. Mapped into the surface cell like the camera was.
    let focus_open = patch_focus(normal.active.then_some(normal.body.position), camera);
    let focus = host.as_ref().map(|host| {
        [
            (focus_open[0] - host.offset[0]) / host.ratio,
            (focus_open[1] - host.offset[1]) / host.ratio,
            (focus_open[2] - host.offset[2]) / host.ratio,
        ]
    });
    let anchor = host.as_ref().map(|host| {
        [
            (camera[0] - host.offset[0]) / host.ratio,
            (camera[1] - host.offset[1]) / host.ratio,
            (camera[2] - host.offset[2]) / host.ratio,
        ]
    });
    let moved = match (&host, anchor, state.camera) {
        // Both sides in surface-cell units: the open-cell camera would
        // dwarf the share through the cell ratio.
        (Some(host), Some(now), Some(last)) => {
            gap(now, last) > REBUILD_SHARE * height_above(&host.frame, last)
        }
        _ => true,
    };
    let refocused = match (&host, focus, state.focus) {
        (Some(host), Some(now), Some(last)) => {
            gap(now, last) > REBUILD_SHARE * height_above(&host.frame, last)
        }
        (Some(_), Some(_), None) => true,
        _ => false,
    };
    if key != state.key || moved || refocused {
        for (entity, mesh, material, _) in &owned {
            commands.entity(entity).despawn();
            meshes.remove(&mesh.0);
            materials.remove(&material.0);
        }
        state.key = None;
        state.camera = None;
        state.focus = None;
        state.rim.clear();
    }
    let (Some(host), Some(focus), Some(anchor)) = (host, focus, anchor) else {
        return;
    };
    // The shown rim draws every frame from the stored ring, closed into
    // a loop.
    if host.fade > 0.0
        && let Some(first) = state.rim.first()
    {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: a 0..1 strength narrowed for a colour, intended"
        )]
        let tint = crate::tokens::RIM_TINT.with_alpha(host.fade.clamp(0.0, 1.0) as f32);
        let mut previous = origin.at(*first);
        for point in state.rim.iter().skip(1).chain(std::iter::once(first)) {
            let at = origin.at(*point);
            gizmos.line(previous, at, tint);
            previous = at;
        }
    }
    if state.key == key {
        // Same dome: follow the fade without rebuilding.
        if (host.fade - state.fade).abs() > 1e-6 {
            for (_, _, material, mut visibility) in &mut owned {
                if let Some(mut material) = materials.get_mut(&material.0) {
                    faded(&mut material, host.fade);
                }
                *visibility = if host.fade > 0.0 {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
            state.fade = host.fade;
        }
        return;
    }
    let centre = host.frame.patch_centre(focus);
    let triangles = patch_dome(&host.frame, centre);
    let Some(mesh) = patch_mesh_of(&triangles, anchor) else {
        return;
    };
    // Rim ring mapped into open-cell units like the mesh vertices,
    // lifted a hair off the surface so the lines never z-fight it.
    let rim = dome_rim(&host.frame, centre)
        .into_iter()
        .map(|point| {
            let length = (point[0] * point[0] + point[1] * point[1] + point[2] * point[2]).sqrt();
            let lift = if length.is_finite() && length > 0.0 {
                crate::tokens::RIM_LIFT / length
            } else {
                0.0
            };
            let raised = [
                point[0] * (1.0 + lift),
                point[1] * (1.0 + lift),
                point[2] * (1.0 + lift),
            ];
            [
                camera[0] + host.ratio * (raised[0] - anchor[0]),
                camera[1] + host.ratio * (raised[1] - anchor[1]),
                camera[2] + host.ratio * (raised[2] - anchor[2]),
            ]
        })
        .collect();
    let mut material = StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        cull_mode: None,
        ..default()
    };
    faded(&mut material, host.fade);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: a cell ratio narrowed for the GPU transform, intended"
    )]
    let scale = Transform::from_scale(Vec3::splat(host.ratio as f32));
    let visibility = if host.fade > 0.0 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    commands.spawn((
        PatchMesh,
        Name::new("patch"),
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(material)),
        scale,
        visibility,
        crate::style::Anchored(camera),
    ));
    state.key = key;
    state.camera = Some(anchor);
    state.focus = Some(focus);
    state.fade = host.fade;
    state.rim = rim;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The demo journey's L10 surface frame.
    fn frame() -> SurfaceFrame {
        let chain = universe_core::nest::autopilot_path(universe_core::nav::DEMO_SEED);
        SurfaceFrame::for_path(universe_core::nav::DEMO_SEED, &chain).expect("planet frame")
    }

    /// Angle between two directions.
    fn angle(a: [f64; 3], b: [f64; 3]) -> f64 {
        let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let la = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
        let lb = (b[0] * b[0] + b[1] * b[1] + b[2] * b[2]).sqrt();
        (dot / la / lb).clamp(-1.0, 1.0).acos()
    }

    #[test]
    fn l10_draws_only_the_follow_patch() {
        let surface = frame();
        let portal = surface.portal_positions()[0];
        let centre = surface.patch_centre(portal);
        let unit = {
            let l = (centre[0] * centre[0] + centre[1] * centre[1] + centre[2] * centre[2]).sqrt();
            [centre[0] / l, centre[1] / l, centre[2] / l]
        };
        let triangles = patch_dome(&surface, centre);
        assert_eq!(
            triangles.len(),
            HEMI_SEGMENTS * (2 * HEMI_RINGS - 1),
            "fixed hemisphere budget"
        );
        // Every corner lies inside the hemisphere; nothing beyond PI/2.
        let half_pi = std::f64::consts::FRAC_PI_2;
        let mut nearest = f64::INFINITY;
        let mut farthest = 0.0f64;
        for triangle in &triangles {
            for corner_point in triangle {
                let off = angle(*corner_point, unit);
                assert!(off <= half_pi + 1e-9, "dome stays in its half: {off}");
                nearest = nearest.min(off);
                farthest = farthest.max(off);
            }
        }
        assert!(nearest < 1e-9, "the dome covers its focus");
        assert!(
            (farthest - half_pi).abs() < 1e-9,
            "the dome reaches the full half: {farthest}"
        );
        assert_eq!(
            triangles,
            patch_dome(&surface, centre),
            "the dome is deterministic"
        );
    }

    #[test]
    fn dome_rim_forms_a_shown_circle() {
        let surface = frame();
        let portal = surface.portal_positions()[0];
        let centre = surface.patch_centre(portal);
        let unit = {
            let l = (centre[0] * centre[0] + centre[1] * centre[1] + centre[2] * centre[2]).sqrt();
            [centre[0] / l, centre[1] / l, centre[2] / l]
        };
        let rim = dome_rim(&surface, centre);
        assert_eq!(rim.len(), HEMI_SEGMENTS, "one loop, no gaps");
        // Every rim vertex sits at exactly PI/2: the shown circle edge.
        let half_pi = std::f64::consts::FRAC_PI_2;
        for point in &rim {
            let off = angle(*point, unit);
            assert!(
                (off - half_pi).abs() < 1e-9,
                "rim vertex on the circle: {off}"
            );
        }
        // Consecutive vertices step evenly: the loop cannot bunch up.
        let mut first = f64::INFINITY;
        let mut last = 0.0f64;
        for pair in rim.windows(2) {
            let step = angle(pair[0], pair[1]);
            first = first.min(step);
            last = last.max(step);
        }
        assert!(
            (last - first).abs() < 1e-9,
            "even steps around the rim: {first} to {last}"
        );
        assert_eq!(rim, dome_rim(&surface, centre), "the rim is deterministic");
    }

    #[test]
    fn patch_focus_follows_the_placed_body_else_the_camera() {
        let body = [0.1, 0.34, -0.05];
        let camera = [0.0, 0.5, 0.0];
        assert_eq!(patch_focus(Some(body), camera), body, "on foot: the body");
        assert_eq!(patch_focus(None, camera), camera, "flying: the camera");
    }
}
