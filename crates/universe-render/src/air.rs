//! The planet's atmosphere in the window (#780 Q3 A).
//!
//! At the planet (L10), when its recipe has air, one Bevy `Atmosphere` sits
//! at the planet centre with the medium built from
//! [`AirModel`](universe_core::atmosphere::AirModel), the same numbers the
//! headless `LIGHT` line reads. The entity is [`Anchored`](crate::style::Anchored)
//! at the centre, so it moves with the render origin, and its scale maps
//! world units (open cells) to metres. The camera carries
//! `AtmosphereSettings` only while the atmosphere exists: one model from
//! orbit to the ground, a black sky on airless worlds and above the planet.

use crate::Universe;
use bevy::light::atmosphere::{
    Atmosphere, Falloff, PhaseFunction, ScatteringMedium, ScatteringTerm,
};
use bevy::math::curve::{FunctionCurve, Interval};
use bevy::pbr::AtmosphereSettings;
use bevy::prelude::*;
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

/// The sky mode for a camera `camera_r_m` from the planet centre, given the
/// current one (`space`), with the switch band of [`SPACE_MODE_ABOVE_M`].
#[must_use]
pub(crate) fn space_mode(air: &AirModel, camera_r_m: f64, space: bool) -> bool {
    if space {
        camera_r_m > air.outer_radius_m
    } else {
        camera_r_m > air.outer_radius_m + SPACE_MODE_ABOVE_M
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
        // World units are open cells; the atmosphere works in metres.
        Transform::from_scale(Vec3::splat(narrow(1.0 / metres_per_cell))),
        crate::style::Anchored([0.0; 3]),
    ));
    for (camera, has) in &cameras {
        if !has {
            commands.entity(camera).insert(AtmosphereSettings {
                // Far enough for the horizon from the top of the air, with
                // enough slices to keep the haze free of bands.
                aerial_view_lut_max_distance: 1.5e6,
                aerial_view_lut_size: UVec3::new(32, 32, 64),
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
    if space == state.space {
        return;
    }
    state.space = space;
    let method = if space {
        bevy::pbr::AtmosphereMode::Raymarched
    } else {
        bevy::pbr::AtmosphereMode::LookupTexture
    };
    for mut settings in &mut cameras {
        settings.rendering_method = method;
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
        let want = narrow(1.0 / metres_per_cell);
        assert!(
            (transform.scale.x - want).abs() <= want * 1e-6,
            "world units to metres"
        );
        assert!(
            app.world().get::<AtmosphereSettings>(camera).is_some(),
            "the camera draws the sky"
        );
        // Inside the air the lookup tables, from space the raymarched sky,
        // with a band so the mode never flickers.
        let top = air.outer_radius_m;
        assert!(!space_mode(&air, top - 1.0, false));
        assert!(!space_mode(&air, top + 1.0, false), "the band holds");
        assert!(space_mode(&air, top + SPACE_MODE_ABOVE_M + 1.0, false));
        assert!(space_mode(&air, top + 1.0, true), "the band holds");
        assert!(!space_mode(&air, top - 1.0, true));
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
}
