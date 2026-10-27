//! Moonlight in the window (#981 correction round 2, Spec v4).
//!
//! The moons are the night lights ([`universe_core::moonlight`]): up to
//! [`MAX_MOON_LIGHTS`] directional lights, without shadows, each aimed at
//! its moon and as bright as its phase and height allow, scaled by the
//! Night light setting (Display, Scene). Off the planet, or with no moon
//! up, they are dark. The even fill (the day fill plus the stars' glow) is the global
//! ambient light, set by [`crate::sun::sync_sun`] from the same model.

use crate::Universe;
use bevy::prelude::*;
use universe_core::moonlight::{MAX_MOON_LIGHTS, NightLight, SkyLight};

/// Marks moon light `0..MAX_MOON_LIGHTS`, brightest first.
#[derive(Component)]
pub(crate) struct MoonLamp(pub(crate) usize);

/// Moonlight's colour: sunlight off grey rock, seen by night eyes, reads
/// slightly cool.
pub(crate) const MOON_COLOUR: Color = Color::srgb(0.78, 0.85, 1.0);

/// The night light setting, or its default without a settings store.
#[must_use]
pub(crate) fn night_light(store: Option<&crate::SettingsStore>) -> NightLight {
    store.map_or_else(NightLight::default, |store| store.0.night_light)
}

/// The sky light at the rendered camera, `None` off the planet.
#[must_use]
pub(crate) fn sky_now(
    universe: &universe_core::nav::Universe,
    camera: [f64; 3],
    night: NightLight,
) -> Option<SkyLight> {
    universe_core::moonlight::sky_light_at(universe, camera, night)
}

/// Spawns the moon lights once, dark.
pub(crate) fn spawn_moon_lights(mut commands: Commands) {
    for index in 0..MAX_MOON_LIGHTS {
        commands.spawn((
            MoonLamp(index),
            Name::new("moon light"),
            DirectionalLight {
                illuminance: 0.0,
                color: MOON_COLOUR,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::default(),
        ));
    }
}

/// Aims and dims the moon lights for this frame. Writes only on change
/// (`E-HOT-NOALLOC`).
pub(crate) fn sync_moon_lights(
    universe: Res<Universe>,
    origin: Res<crate::style::RenderOrigin>,
    store: Option<Res<crate::SettingsStore>>,
    mut lamps: Query<(&MoonLamp, &mut Transform, &mut DirectionalLight)>,
) {
    let sky = sky_now(&universe, origin.camera, night_light(store.as_deref()));
    for (lamp, mut transform, mut light) in &mut lamps {
        let moon = sky.and_then(|sky| sky.moons.get(lamp.0).copied().flatten());
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: light level and direction narrowed for the GPU, intended"
        )]
        let (illuminance, to_moon) = moon.map_or((0.0, Vec3::Y), |moon| {
            let level = (moon.share * universe_core::light::SUN_ILLUMINANCE) as f32;
            let [x, y, z] = moon.direction;
            (level, Vec3::new(x as f32, y as f32, z as f32))
        });
        if light.illuminance != illuminance {
            light.illuminance = illuminance;
        }
        if illuminance > 0.0 {
            let up = if to_moon.y.abs() < 0.9 {
                Vec3::Y
            } else {
                Vec3::X
            };
            // A directional light shines along its forward (-Z): away from
            // the moon.
            let rotation = Transform::default().looking_to(-to_moon, up).rotation;
            if transform.rotation != rotation {
                transform.rotation = rotation;
            }
        }
    }
}

#[cfg(test)]
#[path = "moon_lights_tests.rs"]
mod tests;
