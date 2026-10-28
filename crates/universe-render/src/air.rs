//! The planet's atmosphere in the window (#780 Q3 A).
//!
//! At the planet (L10), when its recipe has air, one Bevy `Atmosphere` sits
//! at the planet centre with the medium built from
//! [`AirModel`](universe_core::atmosphere::AirModel), the same numbers the
//! headless `LIGHT` line reads. The entity is [`Anchored`](crate::style::Anchored)
//! at the centre, so it moves with the render origin; world units are
//! metres at the planet (#780 F2 A), so it needs no scale. The camera carries
//! `AtmosphereSettings` only while the atmosphere exists: one model from
//! orbit to the ground, a black sky on airless worlds and above the planet.
//! After L10 closes, `clear_stale_sky` drops the engine's leftover sky from
//! the render camera (#880, ADR 0046).

use crate::Universe;
use bevy::light::atmosphere::{
    Atmosphere, Falloff, PhaseFunction, ScatteringMedium, ScatteringTerm,
};
use bevy::math::curve::{FunctionCurve, Interval};
use bevy::pbr::AtmosphereSettings;
use bevy::pbr::resources::{AtmosphereTextures, AtmosphereTransformsOffset, GpuAtmosphere};
use bevy::pbr::{ExtractedAtmosphere, GpuAtmosphereSettings};
use bevy::prelude::*;
use bevy::render::extract_component::DynamicUniformIndex;
use universe_core::atmosphere::{AirModel, MIE_ASYMMETRY};

/// Marks the atmosphere entity.
#[derive(Component)]
pub(crate) struct PlanetAir;

/// Frames the medium waits before the atmosphere uses it: the GPU tables
/// are prepared after the asset arrives, and the atmosphere pass refuses a
/// medium that is not prepared yet.
pub(crate) const MEDIUM_WAIT_FRAMES: u8 = 2;

/// Metres above the top of the air where the camera switches to the
/// raymarched sky (seen from space), and back below the top: lookup
/// tables inside the air, where they are exact near the ground, raymarching
/// from space, where the tables lose the limb (Bevy's own guidance).
pub(crate) const SPACE_MODE_ABOVE_M: f64 = 10_000.0;

/// Height over sea level from which the haze is raymarched per pixel even
/// inside the air (#1070, owner Q1 A): higher up the haze table reaches the
/// far horizon, and its 64 slices show as stripes on the ground (about
/// 17 km each at 100 km).
pub(crate) const RAYMARCH_FROM_M: f64 = 50_000.0;

/// Height over sea level under which the haze table comes back: the band
/// below [`RAYMARCH_FROM_M`] keeps a camera hovering there from flickering.
pub(crate) const TABLE_BELOW_M: f64 = 45_000.0;

/// Aerial-haze reach bounds in metres: near the ground the haze table spans
/// a few tens of kilometres, so its first slice stays shallow and nearby
/// ground keeps its own colour and shadows; high up it reaches the horizon.
pub(crate) const AERIAL_REACH_M: (f64, f64) = (32_000.0, 1_500_000.0);

/// How far the aerial-haze table reaches for a camera `camera_r_m` from the
/// centre: the distance to the sea-level horizon, within
/// [`AERIAL_REACH_M`]. One fixed long reach made the first slice 23 km deep,
/// so nearby ground took on the haze of far ground (#780 T4).
#[must_use]
pub(crate) fn aerial_reach_m(air: &AirModel, camera_r_m: f64) -> f64 {
    let sea = air.sea_radius_m;
    let horizon = (camera_r_m * camera_r_m - sea * sea).max(0.0).sqrt();
    let (near, far) = AERIAL_REACH_M;
    if horizon.is_finite() {
        horizon.clamp(near, far)
    } else {
        far
    }
}

/// The sky mode for a camera `camera_r_m` from the planet centre, given the
/// current one (`space`): raymarched from [`RAYMARCH_FROM_M`] over sea
/// level, or from [`SPACE_MODE_ABOVE_M`] over the top of a thinner air,
/// whichever is lower; the table back below [`TABLE_BELOW_M`] or the top
/// of the air, whichever is lower (#1070). The band between keeps the mode
/// from flickering.
#[must_use]
pub(crate) fn space_mode(air: &AirModel, camera_r_m: f64, space: bool) -> bool {
    let sea = air.sea_radius_m;
    if space {
        camera_r_m > (sea + TABLE_BELOW_M).min(air.outer_radius_m)
    } else {
        camera_r_m > (sea + RAYMARCH_FROM_M).min(air.outer_radius_m + SPACE_MODE_ABOVE_M)
    }
}

/// Which planet's air is built, to rebuild only when it changes.
#[derive(Resource, Debug, Default)]
pub(crate) struct AirState {
    /// Seed of the planet whose air is built or being built (`None`: none).
    pub seed: Option<u64>,
    /// The medium of the current planet.
    pub medium: Option<Handle<ScatteringMedium>>,
    /// Every medium made so far, by planet seed. Media are never dropped:
    /// the render world may still hold a removed atmosphere for a frame,
    /// and a dropped medium would then be missing on the GPU.
    pub made: std::collections::HashMap<u64, Handle<ScatteringMedium>>,
    /// Frames left before the atmosphere is spawned (0: spawned).
    pub wait: u8,
    /// The open planet's air and metres per world unit, once spawned.
    pub air: Option<(AirModel, f64)>,
    /// Whether the camera uses the raymarched sky (seen from space).
    pub space: bool,
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "E-CAST: medium coefficients narrowed for the GPU, intended"
)]
fn vec3_of(a: [f64; 3]) -> Vec3 {
    Vec3::new(a[0] as f32, a[1] as f32, a[2] as f32)
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "E-CAST: medium proportions narrowed for the GPU, intended"
)]
fn narrow(v: f64) -> f32 {
    v as f32
}

/// Bevy falloff reading one of core's density profiles (`pick` chooses
/// Rayleigh, Mie or ozone) over the falloff parameter.
fn falloff_of(air: AirModel, pick: fn((f64, f64, f64)) -> f64) -> Falloff {
    Falloff::from_curve(FunctionCurve::new(Interval::UNIT, move |p: f32| {
        narrow(pick(air.densities_at_falloff(f64::from(p))))
    }))
}

/// The window's medium for `air`: Rayleigh, Mie and ozone terms with the
/// same coefficients and density profiles as the core model (sea-level
/// density below sea level, down to the deepest floor).
#[must_use]
pub(crate) fn medium_for(air: &AirModel) -> ScatteringMedium {
    ScatteringMedium::new(
        256,
        256,
        [
            ScatteringTerm {
                absorption: Vec3::ZERO,
                scattering: vec3_of(air.rayleigh),
                falloff: falloff_of(*air, |d| d.0),
                phase: PhaseFunction::Rayleigh,
            },
            ScatteringTerm {
                absorption: vec3_of(air.mie_absorption),
                scattering: Vec3::splat(narrow(air.mie_scattering)),
                falloff: falloff_of(*air, |d| d.1),
                phase: PhaseFunction::Mie {
                    asymmetry: narrow(MIE_ASYMMETRY),
                },
            },
            ScatteringTerm {
                absorption: vec3_of(air.ozone),
                scattering: Vec3::ZERO,
                falloff: falloff_of(*air, |d| d.2),
                phase: PhaseFunction::Isotropic,
            },
        ],
    )
    .with_label("planet_air")
}

/// The open planet's air, its seed and its metres per world unit, if it
/// has air and is open.
#[must_use]
pub(crate) fn open_air(universe: &universe_core::nav::Universe) -> Option<(u64, AirModel, f64)> {
    if universe.level().get() != 10 {
        return None;
    }
    let root = universe.root.0;
    let chain = universe.path.indices();
    let air = AirModel::for_planet(root, &chain)?;
    let frame = universe_core::planet_frame::PlanetFrame::for_planet(root, &chain);
    Some((
        universe_core::nest::path_seed(root, &chain),
        air,
        frame.metres_per_cell(),
    ))
}

/// Builds the atmosphere when a planet with air opens, removes it when it
/// closes, and gives the camera its settings only while it exists.
#[allow(
    clippy::type_complexity,
    reason = "Bevy systems take one query per engine input"
)]
pub(crate) fn sync_air(
    mut commands: Commands,
    universe: Res<Universe>,
    media: Option<ResMut<Assets<ScatteringMedium>>>,
    mut state: ResMut<AirState>,
    owned: Query<Entity, With<PlanetAir>>,
    cameras: Query<(Entity, Has<AtmosphereSettings>), With<Camera3d>>,
    effective: Option<Res<crate::effective::EffectiveGraphics>>,
) {
    let open = open_air(&universe);
    let seed = open.map(|(seed, _, _)| seed);
    if state.seed != seed {
        for entity in &owned {
            commands.entity(entity).despawn();
        }
        for (camera, has) in &cameras {
            if has {
                commands.entity(camera).remove::<AtmosphereSettings>();
            }
        }
        state.seed = None;
        state.medium = None;
        state.wait = 0;
        state.air = None;
        state.space = false;
        let (Some((seed, air, _)), Some(mut media)) = (open, media) else {
            return;
        };
        // First the medium; the atmosphere follows once its tables exist.
        let medium = state
            .made
            .entry(seed)
            .or_insert_with(|| media.add(medium_for(&air)))
            .clone();
        state.medium = Some(medium);
        state.seed = Some(seed);
        state.wait = MEDIUM_WAIT_FRAMES;
        return;
    }
    if state.wait == 0 {
        return;
    }
    state.wait -= 1;
    if state.wait > 0 {
        return;
    }
    let (Some((_, air, metres_per_cell)), Some(medium)) = (open, state.medium.clone()) else {
        return;
    };
    commands.spawn((
        PlanetAir,
        Name::new("planet-air"),
        Atmosphere {
            inner_radius: narrow(air.inner_radius_m),
            outer_radius: narrow(air.outer_radius_m),
            ground_albedo: vec3_of(air.ground_albedo),
            medium,
        },
        // World units are metres at the planet (#780 F2 A), as the
        // atmosphere's own.
        Transform::default(),
        crate::style::Anchored([0.0; 3]),
    ));
    let samples = universe_core::quality::atmosphere_samples(
        effective.map_or(universe_core::settings::Tier::Normal, |e| e.0.atmosphere),
    );
    for (camera, has) in &cameras {
        if !has {
            commands.entity(camera).insert(AtmosphereSettings {
                // Enough slices to keep the haze free of bands; the reach
                // follows the height (`aerial_reach_m`).
                aerial_view_lut_max_distance: narrow(AERIAL_REACH_M.1),
                aerial_view_lut_size: UVec3::new(32, 32, 64),
                // Budget (Q7 A, ADR 0040): half Bevy's sky table and fewer
                // samples, the same picture at a lower cost on the dev PC.
                // The samples follow the effective tier, Normal being
                // today's 10 / 8; the table sizes never change (#1022 R3).
                sky_view_lut_size: UVec2::new(200, 100),
                sky_view_lut_samples: samples.0,
                sky_max_samples: samples.1,
                ..default()
            });
        }
    }
    state.air = Some((air, metres_per_cell));
    state.space = false;
}

/// Switches the camera between the lookup-table sky inside the air and the
/// raymarched sky from space ([`space_mode`]), from the rendered camera.
/// Writes only when the mode flips.
pub(crate) fn sync_sky_mode(
    origin: Res<crate::style::RenderOrigin>,
    mut state: ResMut<AirState>,
    mut cameras: Query<&mut AtmosphereSettings, With<Camera3d>>,
) {
    let Some((air, metres_per_cell)) = state.air else {
        return;
    };
    let c = origin.camera;
    let camera_r_m = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt() * metres_per_cell;
    let space = space_mode(&air, camera_r_m, state.space);
    let reach = narrow(aerial_reach_m(&air, camera_r_m));
    let switched = space != state.space;
    let method = if space {
        bevy::pbr::AtmosphereMode::Raymarched
    } else {
        bevy::pbr::AtmosphereMode::LookupTexture
    };
    state.space = space;
    for mut settings in &mut cameras {
        // Written only on a real change (`E-HOT-NOALLOC`): the reach moves
        // with the height, within 1%.
        let held = settings.aerial_view_lut_max_distance;
        if (held - reach).abs() > reach * 0.01 {
            settings.aerial_view_lut_max_distance = reach;
        }
        if switched {
            settings.rendering_method = method;
        }
    }
}

/// Whether a render camera without sky settings still holds sky data.
///
/// `has_settings` mirrors the game-side camera's `AtmosphereSettings` through
/// Bevy's `SyncComponent` (`AtmosphereSettings` -> `GpuAtmosphereSettings`,
/// removed from the render entity when the game side removes it). `has_sky_data`
/// is whether any of the prepared sky components is still present. True means
/// the frame would keep drawing the planet's sky after leaving (#880).
#[must_use]
pub(crate) fn stale_sky(has_settings: bool, has_sky_data: bool) -> bool {
    !has_settings && has_sky_data
}

/// Clears Bevy's leftover sky after leaving the planet (#880).
///
/// When L10 closes, `sync_air` removes the game-side `AtmosphereSettings`.
/// Bevy 0.19's `extract_atmosphere` only clears its own `ExtractedAtmosphere`
/// copy on cameras that still have settings, so the render entity keeps the
/// stale sky and `render_sky` keeps drawing the planet as a gray disc fixed
/// on screen. This system runs in the extract step after Bevy's own
/// `extract_atmosphere` and removes the prepared sky components from render
/// entities that hold sky data but no longer have settings.
///
/// Only public types are removed (`E-PIN-BEVY`): `ExtractedAtmosphere`,
/// `GpuAtmosphere`, `AtmosphereTextures`, `AtmosphereTransformsOffset`, and
/// the two `DynamicUniformIndex` components. `GpuAtmosphereSettings` itself is
/// already removed by Bevy's sync when the game side drops the settings, so a
/// stale entity never has it; the passes that draw the sky (`render_sky`,
/// `atmosphere_luts`) already require the removed types, so they stop
/// matching once they are gone. The private bind groups
/// (`AtmosphereBindGroups`, `RenderSkyPipelineId`) stay but are never read
/// without the public ones.
///
/// Writes only when something is stale (`E-HOT-NOALLOC`): the query only
/// matches entities holding sky data, and cameras on the planet still have
/// settings so `stale_sky` is false and no command is queued.
#[allow(
    clippy::type_complexity,
    reason = "Bevy systems take one query per engine input"
)]
pub(crate) fn clear_stale_sky(
    mut commands: Commands,
    stale: Query<
        (
            Entity,
            Has<GpuAtmosphereSettings>,
            Has<ExtractedAtmosphere>,
            Has<GpuAtmosphere>,
            Has<AtmosphereTextures>,
            Has<AtmosphereTransformsOffset>,
            Has<DynamicUniformIndex<GpuAtmosphere>>,
            Has<DynamicUniformIndex<GpuAtmosphereSettings>>,
        ),
        Or<(
            With<ExtractedAtmosphere>,
            With<GpuAtmosphere>,
            With<AtmosphereTextures>,
            With<AtmosphereTransformsOffset>,
            With<DynamicUniformIndex<GpuAtmosphere>>,
            With<DynamicUniformIndex<GpuAtmosphereSettings>>,
        )>,
    >,
) {
    for (
        entity,
        has_settings,
        has_extracted,
        has_gpu,
        has_textures,
        has_offset,
        has_index_atmosphere,
        has_index_settings,
    ) in &stale
    {
        let has_data = has_extracted
            || has_gpu
            || has_textures
            || has_offset
            || has_index_atmosphere
            || has_index_settings;
        if !stale_sky(has_settings, has_data) {
            continue;
        }
        if has_extracted {
            commands.entity(entity).remove::<ExtractedAtmosphere>();
        }
        if has_gpu {
            commands.entity(entity).remove::<GpuAtmosphere>();
        }
        if has_textures {
            commands.entity(entity).remove::<AtmosphereTextures>();
        }
        if has_offset {
            commands
                .entity(entity)
                .remove::<AtmosphereTransformsOffset>();
        }
        if has_index_atmosphere {
            commands
                .entity(entity)
                .remove::<DynamicUniformIndex<GpuAtmosphere>>();
        }
        if has_index_settings {
            commands
                .entity(entity)
                .remove::<DynamicUniformIndex<GpuAtmosphereSettings>>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_tests::headless_app;
    use universe_core::nav::MarkerIndex;

    #[test]
    fn planet_air_comes_and_goes_with_the_planet() {
        // Q3 A: at the planet with air one atmosphere from core's numbers,
        // anchored at the centre; the camera gets the sky only while it
        // exists; leaving the planet removes both.
        let mut app = headless_app();
        app.init_asset::<ScatteringMedium>();
        let camera = app.world_mut().spawn(Camera3d::default()).id();
        app.update();
        {
            let mut universe = app.world_mut().resource_mut::<Universe>();
            for _ in 0..9 {
                let marker = universe.autopilot_target().expect("dive continues");
                assert!(universe.open(MarkerIndex(marker)), "marker opens");
            }
        }
        for _ in 0..=MEDIUM_WAIT_FRAMES {
            app.update();
        }
        let (_, air, metres_per_cell) =
            open_air(app.world().resource::<Universe>()).expect("the demo planet has air");
        let (atmosphere, transform, anchor) = app
            .world_mut()
            .query_filtered::<(&Atmosphere, &Transform, &crate::style::Anchored), With<PlanetAir>>()
            .single(app.world())
            .map(|(a, t, n)| (a.clone(), *t, n.0))
            .expect("one atmosphere at the planet");
        assert_eq!(atmosphere.inner_radius, narrow(air.inner_radius_m));
        assert_eq!(atmosphere.outer_radius, narrow(air.outer_radius_m));
        assert!(
            air.inner_radius_m < air.sea_radius_m,
            "air reaches the floors"
        );
        assert_eq!(anchor, [0.0; 3], "anchored at the planet centre");
        assert_eq!(transform.scale, Vec3::ONE, "world units are metres");
        let _ = metres_per_cell;
        assert!(
            app.world().get::<AtmosphereSettings>(camera).is_some(),
            "the camera draws the sky"
        );
        // Near the ground the lookup tables, from 50 km up the raymarched
        // sky (#1070), with a band so the mode never flickers.
        let sea = air.sea_radius_m;
        assert!(
            air.outer_radius_m > sea + RAYMARCH_FROM_M,
            "this air is thick"
        );
        assert!(
            !space_mode(&air, sea + 44_000.0, false),
            "the table at 44 km"
        );
        assert!(!space_mode(&air, sea + 46_000.0, false), "the band holds");
        assert!(
            space_mode(&air, sea + 51_000.0, false),
            "per pixel at 51 km"
        );
        assert!(space_mode(&air, sea + 46_000.0, true), "the band holds");
        assert!(!space_mode(&air, sea + 44_000.0, true), "the table again");
        // A thin air keeps the old rule: its top and 10 km over it.
        let thin = AirModel {
            outer_radius_m: sea + 20_000.0,
            ..air
        };
        assert!(!space_mode(&thin, sea + 25_000.0, false));
        assert!(space_mode(&thin, sea + 31_000.0, false));
        assert!(!space_mode(&thin, sea + 19_000.0, true));
        // Leaving the planet removes the air and the camera's sky.
        {
            let mut universe = app.world_mut().resource_mut::<Universe>();
            *universe = Universe(universe_core::nav::Universe::new(
                universe_core::nav::DEMO_SEED,
            ));
        }
        app.update();
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<(), With<PlanetAir>>()
                .iter(app.world())
                .count(),
            0,
            "no air off the planet"
        );
        assert!(app.world().get::<AtmosphereSettings>(camera).is_none());
    }

    #[test]
    fn aerial_haze_reach_follows_the_height() {
        // T4: on the ground the haze table spans tens of kilometres (a
        // shallow first slice), from the top of the air it reaches the
        // horizon, and it never leaves its bounds.
        let air = AirModel::new(
            6_371_000.0,
            Some(universe_core::r#gen::AirInfo {
                thickness_km: 100.0,
                tint: universe_core::terrain::THIN_AIR_TINT,
            }),
            -11_000.0,
        )
        .expect("earth air");
        let (near, far) = AERIAL_REACH_M;
        let sea = air.sea_radius_m;
        assert_eq!(aerial_reach_m(&air, sea + 2.0), near);
        assert_eq!(aerial_reach_m(&air, sea - 4_000.0), near, "below sea level");
        let at_100km = aerial_reach_m(&air, sea + 100_000.0);
        let horizon = ((sea + 100_000.0).powi(2) - sea * sea).sqrt();
        assert!((at_100km - horizon).abs() < 1.0, "{at_100km} vs {horizon}");
        assert_eq!(aerial_reach_m(&air, sea + 2_000_000.0), far);
        assert_eq!(
            aerial_reach_m(&air, f64::NAN),
            near,
            "a broken height stays near"
        );
    }

    /// Stale sky data is cleared once the settings are gone (#880 C1).
    ///
    /// The render-world `clear_stale_sky` system drops the prepared sky from
    /// cameras without settings (the leftover Bevy keeps after L10 closes)
    /// and keeps the sky on cameras that still have settings.
    #[test]
    fn stale_sky_is_cleared() {
        use bevy::asset::AssetId;

        // Pure rule first: only "no settings plus sky data" is stale.
        assert!(!stale_sky(true, true), "on the planet the sky stays");
        assert!(!stale_sky(true, false));
        assert!(!stale_sky(false, false), "nothing held, nothing to clear");
        assert!(
            stale_sky(false, true),
            "off the planet leftover sky is stale"
        );

        // Render-world system: one stale camera and one kept camera.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, clear_stale_sky);
        let extracted = ExtractedAtmosphere {
            inner_radius: 1.0,
            outer_radius: 2.0,
            ground_albedo: Vec3::ONE,
            medium: AssetId::default(),
            world_to_atmosphere: Mat4::IDENTITY,
        };
        let gpu = GpuAtmosphere {
            ground_albedo: Vec3::ONE,
            inner_radius: 1.0,
            outer_radius: 2.0,
            world_to_atmosphere: Mat4::IDENTITY,
        };
        // Stale: sky data without settings (what L10 leaves behind).
        let stale = app.world_mut().spawn((extracted.clone(), gpu.clone())).id();
        // Kept: the same data with settings (on the planet).
        let kept = app
            .world_mut()
            .spawn((
                extracted.clone(),
                gpu.clone(),
                GpuAtmosphereSettings::default(),
            ))
            .id();
        app.update();
        assert!(
            app.world().get::<ExtractedAtmosphere>(stale).is_none(),
            "stale extracted sky is removed"
        );
        assert!(
            app.world().get::<GpuAtmosphere>(stale).is_none(),
            "stale gpu sky is removed"
        );
        assert!(
            app.world().get::<ExtractedAtmosphere>(kept).is_some(),
            "the planet keeps its sky"
        );
        assert!(
            app.world().get::<GpuAtmosphere>(kept).is_some(),
            "the planet keeps its gpu sky"
        );
        // A second frame writes nothing: the stale entity no longer matches
        // the sky-data query, the kept one still has settings.
        app.update();
        assert!(
            app.world().get::<ExtractedAtmosphere>(kept).is_some(),
            "no second-frame clear"
        );
    }
}
