//! The effective graphics every control draws from (#1022 T8, Spec v4
//! points 1, 4 and 10, R3).
//!
//! [`EffectiveGraphics`] is what is drawn now: terrain, markers,
//! atmosphere, shadows and bloom. Manual draws the saved choices; in Auto
//! the controller writes it ([`crate::auto_quality`], window runs only;
//! without it Auto also draws the saved choices). It is never saved.
//!
//! The atmosphere's samples, bloom, shadows (`sun`), the marker budget
//! (`continuity`) and the terrain detail (`patches`) read it. Bloom fades
//! over half a second by its intensity, which also skips its passes at
//! zero. Shadows switch at once: the renderer has no shadow strength to
//! fade.

use crate::SettingsStore;
use bevy::pbr::AtmosphereSettings;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use universe_core::quality::{EffectiveQuality, atmosphere_samples, marker_tier, planet_tier};
use universe_core::settings::{Settings, Tier};

/// Seconds a bloom switch takes (R3: 0.5 s where supported).
pub(crate) const BLOOM_FADE_S: f32 = 0.5;

/// What is drawn now (see the module notes).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct EffectiveGraphics(pub EffectiveQuality);

impl Default for EffectiveGraphics {
    fn default() -> Self {
        EffectiveGraphics(manual_quality(&Settings::defaults()))
    }
}

/// The quality the saved choices draw (Manual; pure). Resolution is the
/// separate [`crate::RenderScale`]; the loading work stays at its middle
/// tier.
#[must_use]
pub fn manual_quality(s: &Settings) -> EffectiveQuality {
    EffectiveQuality {
        res_steps: 0,
        terrain_work: Tier::Normal,
        terrain: planet_tier(s.planet_detail),
        markers: marker_tier(s.marker_detail),
        atmosphere: s.atmosphere,
        shadows: s.shadows,
        bloom: s.bloom,
    }
}

/// Follows the saved choices when they change (see the module notes); in
/// Auto with its controller running, the controller drives instead.
fn follow_saved(
    store: Option<Res<SettingsStore>>,
    auto: Option<Res<crate::auto_quality::AutoQuality>>,
    mut effective: ResMut<EffectiveGraphics>,
) {
    let Some(store) = store.filter(|s| s.is_changed()) else {
        return;
    };
    if auto.is_some() && store.0.quality_mode == universe_core::settings::QualityMode::Auto {
        return;
    }
    let want = manual_quality(&store.0);
    if effective.0 != want {
        effective.0 = want;
    }
}

/// Moves the bloom toward on or off over [`BLOOM_FADE_S`]; the first
/// frame takes the effective state at once.
fn fade_bloom(
    time: Res<Time<Real>>,
    effective: Res<EffectiveGraphics>,
    mut level: Local<Option<f32>>,
    mut cameras: Query<&mut Bloom, With<Camera3d>>,
) {
    let target = if effective.0.bloom { 1.0 } else { 0.0 };
    let step = time.delta_secs() / BLOOM_FADE_S;
    let next = level.map_or(target, |l: f32| {
        if l < target {
            (l + step).min(target)
        } else {
            (l - step).max(target)
        }
    });
    *level = Some(next);
    let want = Bloom::NATURAL.intensity * next;
    for mut bloom in &mut cameras {
        if bloom.intensity != want {
            bloom.intensity = want;
        }
    }
}

/// Sets the atmosphere's samples for the effective tier, keeping the table
/// sizes (R3). Writes only on a change.
pub(crate) fn sync_atmosphere_tier(
    effective: Res<EffectiveGraphics>,
    mut cameras: Query<&mut AtmosphereSettings, With<Camera3d>>,
) {
    let (lut, raymarch) = atmosphere_samples(effective.0.atmosphere);
    for mut settings in &mut cameras {
        if settings.sky_view_lut_samples != lut || settings.sky_max_samples != raymarch {
            settings.sky_view_lut_samples = lut;
            settings.sky_max_samples = raymarch;
        }
    }
}

/// The effective graphics and the controls that only need it.
pub(crate) struct EffectivePlugin;

impl Plugin for EffectivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EffectiveGraphics>()
            .add_systems(PreUpdate, follow_saved)
            .add_systems(PostUpdate, (fade_bloom, sync_atmosphere_tier));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use universe_core::drawn_set::MarkerDetail;
    use universe_core::settings::PlanetDetail;

    #[test]
    fn manual_draws_the_saved_choices() {
        let saved = Settings {
            planet_detail: PlanetDetail::High,
            marker_detail: MarkerDetail::Low,
            atmosphere: Tier::Low,
            shadows: true,
            bloom: false,
            ..Settings::defaults()
        };
        let q = manual_quality(&saved);
        assert_eq!(
            (q.terrain, q.markers, q.atmosphere),
            (Tier::High, Tier::Low, Tier::Low)
        );
        assert!(q.shadows && !q.bloom);
        assert_eq!(q.res_steps, 0);
    }

    #[test]
    fn terrain_combines_the_tier_and_the_memory_bias() {
        // R2: one effective terrain setting from the tier and the memory
        // bias; the bias only coarsens, never refines past the tier.
        use crate::patches::terrain_detail_px;
        for tier in Tier::ALL {
            let base = universe_core::quality::planet_detail(tier).quad_px();
            assert_eq!(terrain_detail_px(tier, 1.0), base);
            assert_eq!(terrain_detail_px(tier, 0.5), base, "never finer");
            assert_eq!(terrain_detail_px(tier, 2.0), 2.0 * base);
            assert!(terrain_detail_px(tier, 4.0) > terrain_detail_px(tier, 2.0));
        }
        // A lower tier under the same bias is coarser still.
        assert!(terrain_detail_px(Tier::Low, 2.0) > terrain_detail_px(Tier::High, 2.0));
    }

    #[test]
    fn each_marker_tier_keeps_its_budget_once_eased() {
        // R3: the effective tier's budget (500 / 2,000 / 5,000) bounds the
        // drawn markers once the cutoff has eased; portals, the target
        // among them, keep their actionable floor whatever the cutoff
        // (`draw`, #998).
        use universe_core::drawn_set::{Drawn, Layer, budget_cutoff, ease_cutoff};
        use universe_core::frame::angular_radius;
        let set: Vec<Drawn> = (0..6_000_i32)
            .map(|i| Drawn {
                layer: Layer::Open,
                at: [0.0, 0.0, -1.0],
                radius: 0.002 + 1e-6 * f64::from(i),
                portal: false,
                weight: 1.0,
                inside: None,
            })
            .collect();
        let mut angles = Vec::new();
        for tier in Tier::ALL {
            let budget = universe_core::quality::marker_detail(tier).budget();
            let target = budget_cutoff(&set, [0.0; 3], budget, &mut angles);
            let mut cutoff = 0.0;
            for _ in 0..60 {
                cutoff = ease_cutoff(cutoff, target, 1.0 / 60.0);
            }
            assert_eq!(cutoff, target, "{tier:?} lands within a second");
            let drawn = set
                .iter()
                .filter(|d| angular_radius(d.radius, 1.0) >= cutoff)
                .count();
            assert!(drawn <= budget, "{tier:?}: {drawn} over {budget}");
            assert!(drawn + 1 >= budget, "{tier:?}: {drawn} for {budget}");
        }
    }

    fn bloom_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Ten milliseconds a frame, whatever the wall clock (E-TEST-HERMETIC).
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(10),
        ));
        app.add_plugins(EffectivePlugin);
        app.world_mut().spawn((Camera3d::default(), Bloom::NATURAL));
        app
    }

    fn bloom_intensity(app: &mut App) -> f32 {
        app.world_mut()
            .query::<&Bloom>()
            .single(app.world())
            .map(|b| b.intensity)
            .expect("one bloom")
    }

    #[test]
    fn bloom_fades_over_half_a_second() {
        let mut app = bloom_app();
        app.insert_resource(SettingsStore(Settings {
            bloom: true,
            ..Settings::defaults()
        }));
        app.update();
        assert_eq!(
            bloom_intensity(&mut app),
            Bloom::NATURAL.intensity,
            "on at once"
        );
        app.world_mut().resource_mut::<SettingsStore>().0.bloom = false;
        let mut seen_between = false;
        for _ in 0..200 {
            app.update();
            let i = bloom_intensity(&mut app);
            seen_between |= i > 0.0 && i < Bloom::NATURAL.intensity;
            if i == 0.0 {
                break;
            }
        }
        assert!(seen_between, "it fades, not jumps");
        assert_eq!(
            bloom_intensity(&mut app),
            0.0,
            "off: its passes are skipped"
        );
    }
}
