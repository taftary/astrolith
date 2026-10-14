//! Cloud shells for Generated planets (#685, T9).
//!
//! Two translucent shells (low thick sheets, high thin wisps) above planets
//! that keep air: the climate cover share
//! ([`universe_core::world::clouds::cloud_cover_01`]) sampled through fBm
//! noise thresholded by the cover, drifting with the belt winds from the
//! seed and the game clock. Visual only: shells never feed the ground or
//! gameplay, so the pattern rebuilds in steps (about a kilometre of drift
//! per rebuild, invisible from orbit). The Clouds setting hides both
//! shells; Classic looks and airless worlds draw nothing. Shells follow
//! the surface host at its fade, like the mesh.

use crate::SettingsStore;
use crate::Universe;
use crate::planet_approach::surface_host;
use crate::planet_squares::{SurfaceMaterial, surface_material};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;
use universe_core::look::PlanetLook;
use universe_core::surface::SurfaceFrame;

/// Icosphere depth of the shells (1,280 triangles; clouds need no metre detail).
const CLOUD_DEPTH: u8 = 3;

/// Low shell altitude in metres (flat sheets and heaps).
const LOW_ALT_M: f64 = 2_000.0;

/// High shell altitude in metres (thin wisps).
const HIGH_ALT_M: f64 = 9_000.0;

/// Pattern rebuild period in seconds (drift steps with the clock).
const REBUILD_SECS: f64 = 30.0;

/// Noise frequencies of the low (blocky) and high (streaked) layers.
const LOW_FREQ: f64 = 3.0;

/// Noise frequency of the high layer.
const HIGH_FREQ: f64 = 7.0;

/// Peak alpha of the low layer.
const LOW_MAX: f64 = 0.90;

/// Peak alpha of the high layer.
const HIGH_MAX: f64 = 0.55;

/// Noise octaves per cloud vertex.
const CLOUD_OCTAVES: u32 = 4;

/// Stream tag of the cloud noise lanes (low 0, high 1).
const CLOUD_NOISE_TAG: u64 = 0xC10D_C10D_0000_0000;

/// Marks entities owned by [`draw_clouds`].
#[derive(Component)]
pub(crate) struct CloudShell;

/// Tracks which planet the spawned shells were built for.
#[derive(Resource, Default)]
pub(crate) struct CloudState {
    /// Planet seed the current shells belong to, if any.
    seed: Option<u64>,
    /// Clock bucket the pattern was sampled for.
    bucket: u64,
    /// Strength the material was last set to.
    fade: f64,
}

/// Shell altitude in cell units on `frame` for `layer` (0 low, else high).
#[must_use]
pub(crate) fn shell_alt_units(frame: &SurfaceFrame, layer: u8) -> f64 {
    let alt_m = if layer == 0 { LOW_ALT_M } else { HIGH_ALT_M };
    alt_m / frame.metres_per_unit()
}

/// Whether `frame` wears shells: orbit-aware frames read the air model
/// (kept air only), tool-built frames the size-based body air (the rim's
/// Classic rule).
#[must_use]
pub(crate) fn cloudy(frame: &SurfaceFrame) -> bool {
    if let Some(recipe) = universe_core::world::biome::recipe_for_frame(frame) {
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
        return air.present;
    }
    universe_core::terrain::air_for_radius_earth(frame.radius_earth()).is_some()
}

/// White vertex alpha over `direction` at `elapsed_s` seconds: the belt
/// cover share through fBm sampled at the drifted direction, thresholded
/// by the cover (climate guide, step 5). Degenerate input reads clear.
#[must_use]
pub(crate) fn cloud_alpha(
    seed: u64,
    spin_hours: f64,
    layer: u8,
    direction: [f64; 3],
    elapsed_s: f64,
) -> f64 {
    if !direction.iter().all(|c| c.is_finite()) || !elapsed_s.is_finite() {
        return 0.0;
    }
    let spin = if spin_hours.is_finite() && spin_hours > 0.0 {
        spin_hours
    } else {
        universe_core::terrain::EARTH_SPIN_HOURS
    };
    let unit = universe_core::icosphere::normalize(direction);
    let lat = unit[1].clamp(-1.0, 1.0).asin();
    let cover = universe_core::world::clouds::cloud_cover_01(
        universe_core::world::climate::dry_belt_lat_deg(spin),
        universe_core::world::climate::extra_belt_pairs(spin),
        lat,
    );
    if cover <= 0.0 {
        return 0.0;
    }
    let (freq, max) = if layer == 0 {
        (LOW_FREQ, LOW_MAX)
    } else {
        (HIGH_FREQ, HIGH_MAX)
    };
    let lane = universe_core::seed::hash_triple(seed, CLOUD_NOISE_TAG, u64::from(layer));
    let at = universe_core::world::clouds::drifted_dir(unit, elapsed_s, spin, seed);
    let (noise, _) = universe_core::world::noise3::fbm_3d(
        lane,
        [at[0] * freq, at[1] * freq, at[2] * freq],
        CLOUD_OCTAVES,
        2.0,
        0.5,
    );
    let share = (0.5 + 0.5 * noise).clamp(0.0, 1.0);
    ((share - (1.0 - cover)) / 0.25).clamp(0.0, 1.0) * max
}

/// Narrows a cell-space point for the GPU.
fn narrow(point: [f64; 3]) -> [f32; 3] {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cell-unit cloud point narrowed for the GPU, intended"
    )]
    let narrowed = [point[0] as f32, point[1] as f32, point[2] as f32];
    narrowed
}

/// Shell mesh for `frame`'s `layer` at `elapsed_s`: the coarse icosphere
/// lifted to the shell altitude over the flat faces, with per-vertex
/// cloud alpha. `None` when nothing addresses (never on a valid frame).
#[must_use]
pub(crate) fn cloud_mesh(
    frame: &SurfaceFrame,
    seed: u64,
    spin_hours: f64,
    layer: u8,
    elapsed_s: f64,
) -> Option<Mesh> {
    let alt = shell_alt_units(frame, layer);
    let triangles = universe_core::icosphere::all_at_depth(CLOUD_DEPTH);
    if triangles.is_empty() {
        return None;
    }
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(triangles.len() * 3);
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(triangles.len() * 3);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(triangles.len() * 3);
    for addr in triangles {
        let Some(corners) = universe_core::icosphere::tri_corners(addr) else {
            continue;
        };
        for corner in corners {
            let unit = universe_core::icosphere::normalize(corner);
            let flat = universe_core::terrain::surface_radius(unit);
            let point = [
                unit[0] * (flat + alt),
                unit[1] * (flat + alt),
                unit[2] * (flat + alt),
            ];
            let alpha = cloud_alpha(seed, spin_hours, layer, unit, elapsed_s);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: 0..1 cloud alpha narrowed for the GPU, intended"
            )]
            let alpha = alpha as f32;
            positions.push(narrow(point));
            colors.push([1.0, 1.0, 1.0, alpha]);
            normals.push(narrow(unit));
        }
    }
    if positions.is_empty() {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cloud vertex count below u32 range, intended"
    )]
    let count = positions.len() as u32;
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

/// Shell material at `fade` strength: translucent always (the vertex alpha
/// carries the pattern), fading with the host surface.
fn cloud_material(fade: f64) -> StandardMaterial {
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

/// Clock bucket of `elapsed_s`: the pattern rebuilds once per bucket, so
/// shells drift with the clock in quiet steps.
#[must_use]
pub(crate) fn clock_bucket(elapsed_s: f64) -> u64 {
    if !elapsed_s.is_finite() || elapsed_s < 0.0 {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: clock buckets saturate like `as` past u64 range"
    )]
    #[expect(
        clippy::cast_sign_loss,
        reason = "E-CAST: clock buckets non-negative, saturates like `as`"
    )]
    return (elapsed_s / REBUILD_SECS).floor() as u64;
}

/// Spawns the two cloud shells for a cloudy Generated planet, despawning
/// them elsewhere (#685, T9).
///
/// Rebuilds when the planet changes or the clock bucket turns over (the
/// drift); per-frame cost is the host lookup plus small comparisons.
/// Both shells share one material and follow the host fade without
/// rebuilding. The Clouds setting off, Classic looks, airless worlds and
/// views without a surface host draw nothing.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn draw_clouds(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SurfaceMaterial>>,
    universe: Res<Universe>,
    store: Res<SettingsStore>,
    time: Res<Time>,
    mut state: ResMut<CloudState>,
    owned: Query<(Entity, &Mesh3d, &MeshMaterial3d<SurfaceMaterial>), With<CloudShell>>,
) {
    let host = surface_host(&universe);
    let on = store.0.planet_look == PlanetLook::Generated
        && store.0.clouds
        && host.as_ref().is_some_and(|host| cloudy(&host.frame));
    let bucket = clock_bucket(time.elapsed_secs_f64());
    let key = if on {
        host.as_ref().map(|host| host.frame.planet_seed())
    } else {
        None
    };
    if key != state.seed || (key.is_some() && bucket != state.bucket) {
        for (entity, mesh, material) in &owned {
            commands.entity(entity).despawn();
            meshes.remove(&mesh.0);
            materials.remove(&material.0);
        }
        state.seed = None;
    }
    let (Some(host), Some(seed)) = (host, key) else {
        return;
    };
    if state.seed == key {
        // Same shells: follow the host fade without rebuilding.
        if (host.fade - state.fade).abs() > 1e-6 {
            let material = cloud_material(host.fade);
            for (_, _, owned) in &owned {
                if let Some(mut slot) = materials.get_mut(&owned.0) {
                    slot.base.base_color = material.base_color;
                }
            }
            state.fade = host.fade;
        }
        state.bucket = bucket;
        return;
    }
    let orbit_home = host.frame.orbit().is_some_and(|orbit| orbit.home);
    let body = universe_core::terrain::body_recipe(seed, orbit_home);
    let material = materials.add(surface_material(cloud_material(host.fade)));
    let elapsed = time.elapsed_secs_f64();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: a cell ratio narrowed for the GPU transform, intended"
    )]
    let ratio = host.ratio as f32;
    for layer in [0u8, 1u8] {
        let Some(mesh) = cloud_mesh(&host.frame, seed, body.spin_hours, layer, elapsed) else {
            continue;
        };
        commands.spawn((
            CloudShell,
            Name::new(if layer == 0 {
                "cloud-shell-low"
            } else {
                "cloud-shell-high"
            }),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material.clone()),
            Transform {
                scale: Vec3::splat(ratio),
                ..default()
            },
            crate::style::Anchored(host.offset),
        ));
    }
    state.seed = key;
    state.bucket = bucket;
    state.fade = host.fade;
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn shells_sit_above_the_faces_in_order() {
        let frame = generated_frame(7, 1.0, 365.25);
        let low = shell_alt_units(&frame, 0);
        let high = shell_alt_units(&frame, 1);
        assert!(low > 0.0 && high > low, "{low} {high}");
        assert_eq!(low, shell_alt_units(&frame, 0), "replay differs");
    }

    #[test]
    fn clouds_gate_on_air_and_setting_inputs() {
        // T9 wiring: an Earth-like context wears shells; a searched
        // airless context does not; tool frames fall back to the
        // size-based air.
        assert!(cloudy(&generated_frame(7, 1.0, 365.25)));
        assert!(!cloudy(&airless_frame()));
        assert!(cloudy(&SurfaceFrame::body(7, 1.0)));
        assert!(!cloudy(&SurfaceFrame::body(7, 0.3)));
    }

    /// First airless frame in range (close-in orbit; the recipe draws its
    /// own mass, so airlessness is searched, not assumed).
    fn airless_frame() -> SurfaceFrame {
        (0..500u64)
            .map(|seed| generated_frame(seed, 0.39, 88.0))
            .find(|frame| !cloudy(frame))
            .expect("no airless world in range")
    }

    #[test]
    fn cloud_alpha_thins_over_deserts_and_replays() {
        // Cover wiring: equatorial columns average wetter than subtropical
        // ones; alpha stays in its layer band and replays.
        let belt = |lat_deg: f64| {
            let mut sum = 0.0;
            for i in 0..24 {
                let lon = f64::from(i) * std::f64::consts::TAU / 24.0;
                let lat = lat_deg.to_radians();
                let dir = [lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin()];
                sum += cloud_alpha(7, 24.0, 0, dir, 1_000.0);
            }
            sum / 24.0
        };
        assert!(belt(0.0) > belt(30.0), "deserts cloudier than equator");
        for dir in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.3, -0.5, 0.8]] {
            for layer in [0u8, 1u8] {
                let alpha = cloud_alpha(7, 24.0, layer, dir, 1_000.0);
                let max = if layer == 0 { LOW_MAX } else { HIGH_MAX };
                assert!((0.0..=max + 1e-9).contains(&alpha), "{alpha}");
                assert_eq!(alpha, cloud_alpha(7, 24.0, layer, dir, 1_000.0));
            }
        }
        // The seed moves the pattern: somewhere along a meridian the two
        // seeds read different partial cover (never saturated both ends).
        let mut differs = false;
        let mut partial = false;
        for i in 0..48 {
            let lat = -1.2 + 2.4 * f64::from(i) / 47.0;
            let dir = universe_core::icosphere::normalize([lat.cos(), lat.sin(), 0.3]);
            let a = cloud_alpha(7, 24.0, 0, dir, 1_000.0);
            let b = cloud_alpha(8, 24.0, 0, dir, 1_000.0);
            differs = differs || a != b;
            partial = partial || (a > 0.0 && a < LOW_MAX);
        }
        assert!(partial, "no partial cover anywhere");
        assert!(differs, "seed does nothing");
        assert_eq!(cloud_alpha(7, 24.0, 0, [f64::NAN, 0.0, 0.0], 1_000.0), 0.0);
        assert_eq!(clock_bucket(0.0), 0);
        assert_eq!(clock_bucket(REBUILD_SECS), 1);
        assert_eq!(clock_bucket(f64::NAN), 0);
    }

    #[test]
    fn cloud_mesh_covers_the_sphere_and_replays() {
        let frame = generated_frame(7, 1.0, 365.25);
        let mesh = cloud_mesh(&frame, 7, 24.0, 0, 1_000.0).expect("shell mesh");
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("shell carries positions");
        };
        assert_eq!(positions.len(), 20 * 4_usize.pow(3) * 3);
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("shell carries colours");
        };
        assert_eq!(colors.len(), positions.len());
        let mut peak = 0.0f32;
        for colour in colors {
            assert!((0.0..=1.0).contains(&colour[3]), "alpha out of share");
            peak = peak.max(colour[3]);
        }
        assert!(peak > 0.2, "no clouds anywhere: {peak}");
        let again = cloud_mesh(&frame, 7, 24.0, 0, 1_000.0).expect("shell mesh");
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(again)) =
            again.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("shell carries colours");
        };
        assert_eq!(colors, again, "replay differs");
    }
}
