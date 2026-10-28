//! The one sunlight in the window (#780 Q1 A, Q2 A).
//!
//! One directional light shines along the planet's sun (moving with the
//! sky's clock, #981)
//! ([`universe_core::light::sun_direction`]) at the planet, and along the
//! star direction's fallback above it, where only the figure is lit (stars,
//! galaxies and markers stay unlit glowing points). The global ambient
//! light at the planet is the moonlight model's even fill (#981 round 2):
//! the day fill while the sun is up at the camera, fading to the stars'
//! faint glow at night, slightly cool; the moons light the night from their
//! side ([`crate::moon_lights`]). Off the planet a brighter floor keeps the
//! figure readable.
//!
//! Near shadows (#780 Q4 A, S1): at the planet, when Settings > Display >
//! Scene > Shadows is on, the sun casts cascaded shadows around the camera
//! ([`SHADOW_CASCADES_M`]), from the ground (with its morphed shape) and
//! the figure. Distances and the depth bias are in metres, the planet's
//! world units (#780 F2 A). Far away the night side is the shadow.

use crate::Universe;
use bevy::prelude::*;

/// Near-shadow cascades in metres from the camera: nearest, first far
/// bound, farthest.
pub(crate) const SHADOW_CASCADES_M: (f64, f64, f64) = (0.05, 30.0, 3_000.0);

/// Shadow depth bias in metres.
pub(crate) const SHADOW_DEPTH_BIAS_M: f64 = 0.05;

/// The near-shadow cascade layout (world units are metres at the planet,
/// #780 F2 A).
#[must_use]
pub(crate) fn planet_cascades() -> bevy::light::CascadeShadowConfig {
    let (near, first, far) = SHADOW_CASCADES_M;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: cascade bounds narrowed for the GPU, intended"
    )]
    let unit = |m: f64| m as f32;
    bevy::light::CascadeShadowConfigBuilder {
        // Budget (Q7 A, ADR 0040): two cascades cover 3 km at a lower cost.
        num_cascades: 2,
        minimum_distance: unit(near),
        first_cascade_far_bound: unit(first),
        maximum_distance: unit(far),
        overlap_proportion: 0.2,
    }
    .build()
}

/// Marks the sun entity.
#[derive(Component)]
pub(crate) struct Sun;

/// Ambient brightness for a floor that is `share` of the sunlight on a
/// surface facing the sun (`E / pi` for a diffuse surface).
#[must_use]
pub(crate) fn floor_brightness(share: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: light level narrowed for the GPU, intended"
    )]
    let level = (share * universe_core::light::SUN_ILLUMINANCE / std::f64::consts::PI) as f32;
    level
}

/// The sun's direction this frame (toward the star, unit) and the even
/// fill's share: at L10 the planet's sun and the moonlight model's fill at
/// the `camera` with Night light `night` (#981 round 2), else the fallback
/// and the off-planet floor.
#[must_use]
pub(crate) fn sun_now(
    universe: &universe_core::nav::Universe,
    camera: [f64; 3],
    night: universe_core::moonlight::NightLight,
) -> ([f64; 3], f64) {
    match universe_core::light::sun_direction(universe) {
        Some(dir) => (
            dir,
            crate::moon_lights::sky_now(universe, camera, night)
                .map_or(universe_core::moonlight::DAY_FILL_SHARE, |sky| {
                    sky.ambient_share
                }),
        ),
        None => (
            universe_core::light::star_direction(universe),
            universe_core::light::OFF_PLANET_FLOOR,
        ),
    }
}

/// The even fill's colour: white by day, the moon's cool tint as the fill
/// falls to the stars' glow (`share` under the day fill).
#[must_use]
fn fill_colour(share: f64, planet: bool) -> Color {
    if !planet {
        return Color::WHITE;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: a blend weight narrowed for the colour, intended"
    )]
    let day = (share / universe_core::moonlight::DAY_FILL_SHARE).clamp(0.0, 1.0) as f32;
    Color::WHITE.mix(&crate::moon_lights::MOON_COLOUR, 1.0 - day)
}

/// Spawns the sun once.
pub(crate) fn spawn_sun(mut commands: Commands) {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: illuminance narrowed for the GPU, intended"
    )]
    let illuminance = universe_core::light::SUN_ILLUMINANCE as f32;
    commands.spawn((
        Sun,
        Name::new("sun"),
        DirectionalLight {
            illuminance,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::default(),
    ));
}

/// Points the sun along this frame's direction, sets the night floor and
/// the near shadows. Writes only on change (`E-HOT-NOALLOC`).
#[allow(
    clippy::type_complexity,
    reason = "Bevy systems take one query per engine input"
)]
pub(crate) fn sync_sun(
    universe: Res<Universe>,
    origin: Option<Res<crate::style::RenderOrigin>>,
    store: Option<Res<crate::SettingsStore>>,
    ambient: Option<ResMut<GlobalAmbientLight>>,
    mut at_planet: Local<bool>,
    mut suns: Query<
        (
            &mut Transform,
            &mut DirectionalLight,
            &mut bevy::light::CascadeShadowConfig,
        ),
        With<Sun>,
    >,
) {
    let planet = universe.level().get() == 10;
    let shadows = planet && store.as_ref().is_none_or(|s| s.0.shadows);
    let arrived = planet && !*at_planet;
    *at_planet = planet;
    let camera = origin.map_or([0.0, 1.0, 0.0], |origin| origin.camera);
    let night = crate::moon_lights::night_light(store.as_deref());
    let (dir, share) = sun_now(&universe, camera, night);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: direction narrowed for the GPU, intended"
    )]
    let to_star = Vec3::new(dir[0] as f32, dir[1] as f32, dir[2] as f32);
    let up = if to_star.y.abs() < 0.9 {
        Vec3::Y
    } else {
        Vec3::X
    };
    // A directional light shines along its forward (-Z): away from the star.
    let rotation = Transform::default().looking_to(-to_star, up).rotation;
    for (mut transform, mut light, mut cascades) in &mut suns {
        if transform.rotation != rotation {
            transform.rotation = rotation;
        }
        if light.shadow_maps_enabled != shadows {
            light.shadow_maps_enabled = shadows;
        }
        if arrived {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: bias narrowed for the GPU, intended"
            )]
            let bias = SHADOW_DEPTH_BIAS_M as f32;
            light.shadow_depth_bias = bias;
            *cascades = planet_cascades();
        }
    }
    let brightness = floor_brightness(share);
    let colour = fill_colour(share, planet);
    let Some(mut ambient) = ambient else {
        return;
    };
    if ambient.brightness != brightness || ambient.color != colour {
        ambient.brightness = brightness;
        ambient.color = colour;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_tests::headless_app;
    use universe_core::nav::MarkerIndex;

    /// Whether every material on entities named `name` is lit.
    fn lit_named(app: &mut App, name: &str) -> Option<bool> {
        let handles: Vec<Handle<StandardMaterial>> = app
            .world_mut()
            .query::<(&Name, &MeshMaterial3d<StandardMaterial>)>()
            .iter(app.world())
            .filter(|(n, _)| n.as_str() == name)
            .map(|(_, m)| m.0.clone())
            .collect();
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        (!handles.is_empty()).then(|| {
            handles
                .iter()
                .all(|h| materials.get(h).is_some_and(|m| !m.unlit))
        })
    }

    #[test]
    fn one_sunlight_lights_ground_moons_and_figure() {
        // #780 Q1 A: one sun from the star direction lights the ground (by
        // its terrain normals), the moons and the figure; the even fill is
        // the moonlight model's at the camera (#981 round 2); above the
        // planet stars stay unlit points.
        let mut app = headless_app();
        app.init_resource::<GlobalAmbientLight>();
        app.update();
        {
            let mut universe = app.world_mut().resource_mut::<Universe>();
            for _ in 0..9 {
                let marker = universe.autopilot_target().expect("dive continues");
                assert!(universe.open(MarkerIndex(marker)), "marker opens");
            }
        }
        crate::player::player_here(&mut app);
        // #919 T1: the waiting figure stays hidden until the near ground
        // is ready; wait for that before checking it is lit.
        let mut ready = false;
        for _ in 0..600 {
            app.update();
            let ground = app.world().resource::<crate::patches::GroundSurface>();
            if ground.0.is_some() && ground.1 {
                ready = true;
                break;
            }
        }
        assert!(ready, "near ground never ready");
        let sun = universe_core::light::sun_direction(app.world().resource::<Universe>())
            .expect("a sun at the planet");
        let forward = app
            .world_mut()
            .query_filtered::<&Transform, With<Sun>>()
            .single(app.world())
            .expect("one sun")
            .forward();
        let along = -(f64::from(forward.x) * sun[0]
            + f64::from(forward.y) * sun[1]
            + f64::from(forward.z) * sun[2]);
        assert!(along > 0.999_999, "the sun shines from the star: {along}");
        let fill = app.world().resource::<GlobalAmbientLight>().brightness;
        let share = f64::from(fill / floor_brightness(1.0));
        let camera = app.world().resource::<crate::style::RenderOrigin>().camera;
        let want = crate::moon_lights::sky_now(
            app.world().resource::<Universe>(),
            camera,
            universe_core::moonlight::NightLight::default(),
        )
        .expect("on the planet")
        .ambient_share;
        assert!((share - want).abs() < 1e-6, "fill {share}, model {want}");
        // The ground: every patch material is lit (one rule, morph normals).
        assert!(!crate::patch_material::ground_base().unlit);
        assert_eq!(lit_named(&mut app, "planet-moon"), Some(true), "moons lit");
        assert_eq!(
            lit_named(&mut app, "player-figure"),
            Some(true),
            "figure lit"
        );
        // Off the planet the stars and markers stay unlit glowing points.
        let root = universe_core::nav::Universe::new(universe_core::nav::DEMO_SEED);
        assert_eq!(universe_core::light::sun_direction(&root), None);
        let (_, floor) = sun_now(
            &root,
            [0.0, 1.0, 0.0],
            universe_core::moonlight::NightLight::default(),
        );
        assert_eq!(floor, universe_core::light::OFF_PLANET_FLOOR);
        assert!(
            include_str!("stars.rs").contains("unlit: true"),
            "stars stay unlit"
        );
    }

    /// The sun's shadow switch and cascade far bound.
    fn sun_shadows(app: &mut App) -> (bool, f32) {
        app.world_mut()
            .query_filtered::<(&DirectionalLight, &bevy::light::CascadeShadowConfig), With<Sun>>()
            .single(app.world())
            .map(|(light, cascades)| {
                (
                    light.shadow_maps_enabled,
                    cascades.bounds.last().copied().unwrap_or(0.0),
                )
            })
            .expect("one sun")
    }

    #[test]
    fn near_shadows_follow_the_setting() {
        // C4 (#780 Q4 A, S1): at the planet the sun casts near shadows in
        // metre cascades while Settings > Display > Scene > Shadows is on;
        // turning it off removes them; off the planet there are none.
        let mut app = headless_app();
        app.insert_resource(crate::SettingsStore(universe_core::settings::Settings {
            shadows: true,
            ..Default::default()
        }));
        app.update();
        assert!(!sun_shadows(&mut app).0, "no shadows off the planet");
        {
            let mut universe = app.world_mut().resource_mut::<Universe>();
            for _ in 0..9 {
                let marker = universe.autopilot_target().expect("dive continues");
                assert!(universe.open(MarkerIndex(marker)), "marker opens");
            }
        }
        app.update();
        let (on, far) = sun_shadows(&mut app);
        assert!(on, "shadows on at the planet when chosen");
        let want = planet_cascades().bounds.last().copied().unwrap_or(0.0);
        assert!(
            (far - want).abs() < 1.0 && far > 1_000.0,
            "cascades in metres: {far}"
        );
        app.world_mut()
            .resource_mut::<crate::SettingsStore>()
            .0
            .shadows = false;
        app.update();
        assert!(!sun_shadows(&mut app).0, "the setting turns them off");
        app.world_mut()
            .resource_mut::<crate::SettingsStore>()
            .0
            .shadows = true;
        app.update();
        assert!(sun_shadows(&mut app).0, "and back on");
    }
}
