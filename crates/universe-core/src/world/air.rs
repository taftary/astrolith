//! Air: the cosmic shoreline over a world's life, then its gas mix,
//! pressure and greenhouse warming (#685, T7).
//!
//! Guides: `guides/universe/L10/generation/documents/air-model.md`
//! (shoreline `I ∝ v_esc^4`, age integration, dynamo loss, the type
//! bands) and the measured Solar-System airs. Giants always keep their
//! air; rocky worlds and moons must hold theirs against their star's
//! light over their whole life, bright young red dwarfs included
//! ([`crate::world::star::luminosity`]). Everything is `f64` and
//! deterministic (`E-DET-TIERS`).

use super::classify::WorldKind;

/// Gravitational constant in m³/kg/s².
pub const GRAVITY_G: f64 = 6.674e-11;

/// Earth mass in kg.
pub const EARTH_MASS_KG: f64 = 5.972e24;

/// Earth mean radius in metres.
pub const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// Shoreline separation: `log10(I) - 4 log10(v)` below this keeps air
/// (fitted once: Mercury −1.69 and the Moon −1.51 fall airless above it;
/// Mars −3.17, Titan −3.65, Venus −3.78 and Earth −4.20 keep air below).
pub const SHORELINE_SEP: f64 = -2.3;

/// Small old worlds (dead dynamos) keep air only this far below the line.
pub const DYNAMO_MARGIN: f64 = 0.3;

/// Small means under half an Earth mass for the dynamo rule.
pub const DYNAMO_MASS_EARTH: f64 = 0.5;

/// Old means past three billion years for the dynamo rule.
pub const DYNAMO_AGE_GYR: f64 = 3.0;

/// Shoreline age samples across a world's life.
pub const SHORELINE_SAMPLES: u32 = 8;

/// Mass past which a world always keeps its air (gas giants and up).
pub const GIANT_MASS_EARTH: f64 = 10.0;

/// What a kept atmosphere is made of (air-model guide, step 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AirMix {
    /// No air kept.
    Vacuum,
    /// Thin or thick carbon dioxide (deserts, Venus-like, traces).
    CarbonDioxide,
    /// Nitrogen with carbon dioxide (Earth-like without a life flag;
    /// life itself is a spec non-goal, so oxygen never appears).
    NitrogenCarbonDioxide,
    /// Nitrogen with methane (Titan-like cold worlds).
    NitrogenMethane,
    /// Hydrogen and helium (giants, rogues, sub-Neptunes).
    HydrogenHelium,
    /// Rock vapour, sodium and silicon oxides (lava worlds).
    RockVapour,
    /// Water vapour with nitrogen and carbon dioxide (ocean worlds).
    WaterMix,
}

/// One world's air: whether it kept any, how much, and how much it warms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Air {
    /// Whether the world kept an atmosphere.
    pub present: bool,
    /// Surface pressure in bar (`None` with no surface to stand on).
    pub pressure_bar: Option<f64>,
    /// Greenhouse warming over the equilibrium temperature, in kelvin.
    pub greenhouse_k: f64,
    /// Gas mix.
    pub mix: AirMix,
}

/// Inputs to the air model: the body, its orbit, its star and its age.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AirInputs {
    /// Mass in Earth masses.
    pub mass_earth: f64,
    /// Radius in Earth radii.
    pub radius_earth: f64,
    /// Orbit in AU (0 for a rogue).
    pub orbit_au: f64,
    /// Star mass in Sun masses (0 for a rogue).
    pub star_mass_solar: f64,
    /// Equilibrium temperature in kelvin, at the system's age.
    pub t_eq_k: f64,
    /// Water share, 0..1.
    pub water_01: f64,
    /// System age in billions of years.
    pub age_gyr: f64,
}

/// Escape speed in km/s for `mass_earth` Earth masses and `radius_earth`
/// Earth radii (`sqrt(2 G M / R)`).
#[must_use]
pub fn escape_velocity_kms(mass_earth: f64, radius_earth: f64) -> f64 {
    if !mass_earth.is_finite()
        || !radius_earth.is_finite()
        || mass_earth <= 0.0
        || radius_earth <= 0.0
    {
        return 0.0;
    }
    (2.0 * GRAVITY_G * mass_earth * EARTH_MASS_KG / (radius_earth * EARTH_RADIUS_M)).sqrt()
        / 1_000.0
}

/// Starlight received in Earth units for luminosity `lum` (Sun units) at
/// `orbit_au` AU.
#[must_use]
pub fn insolation_earth(lum: f64, orbit_au: f64) -> f64 {
    if !lum.is_finite() || !orbit_au.is_finite() || orbit_au <= 0.0 {
        return 0.0;
    }
    lum.max(0.0) / (orbit_au * orbit_au)
}

/// Shoreline value `log10(I) - 4 log10(v)`: below
/// [`SHORELINE_SEP`] the world holds its air at that moment.
#[must_use]
pub fn shoreline_value(insolation: f64, escape_kms: f64) -> f64 {
    if !insolation.is_finite() || !escape_kms.is_finite() || insolation <= 0.0 || escape_kms <= 0.0
    {
        return f64::INFINITY;
    }
    insolation.log10() - 4.0 * escape_kms.log10()
}

/// Whether the world keeps air over its life (air-model guide, step 1):
/// giants always do; rocky worlds must spend most of their sampled life
/// on the air side of the shoreline (the samples cross the star's
/// brightening, including young red dwarfs); small old worlds with dead
/// dynamos must additionally sit comfortably below the line today.
#[must_use]
pub fn keeps_air(
    mass_earth: f64,
    radius_earth: f64,
    orbit_au: f64,
    star_mass_solar: f64,
    age_gyr: f64,
) -> bool {
    if !mass_earth.is_finite() || !radius_earth.is_finite() {
        return false;
    }
    if mass_earth >= GIANT_MASS_EARTH {
        return true;
    }
    if orbit_au <= 0.0 {
        return false;
    }
    if !star_mass_solar.is_finite()
        || star_mass_solar <= 0.0
        || !age_gyr.is_finite()
        || age_gyr <= 0.0
    {
        return false;
    }
    let escape = escape_velocity_kms(mass_earth, radius_earth);
    let mut air_side = 0u32;
    for i in 0..SHORELINE_SAMPLES {
        let share = (f64::from(i) + 0.5) / f64::from(SHORELINE_SAMPLES);
        let at = (age_gyr * share).max(0.01);
        let lum = super::star::luminosity(star_mass_solar, at);
        if shoreline_value(insolation_earth(lum, orbit_au), escape) < SHORELINE_SEP {
            air_side += 1;
        }
    }
    if air_side * 2 < SHORELINE_SAMPLES {
        return false;
    }
    if mass_earth < DYNAMO_MASS_EARTH && age_gyr > DYNAMO_AGE_GYR {
        let now = super::star::luminosity(star_mass_solar, age_gyr);
        let margin =
            insolation_earth(now, orbit_au) / (10f64.powf(SHORELINE_SEP) * escape.powf(4.0));
        return margin < DYNAMO_MARGIN;
    }
    true
}

/// A share in 0..1 from the hash stream (in-band draws).
fn unit01(seed: u64, tag: u64, lane: u64) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: 53-bit hash share as a unit float, intended"
    )]
    let share = (crate::seed::hash_triple(seed, tag, lane) >> 11) as f64 / 9_007_199_254_740_992.0;
    share.clamp(0.0, 1.0 - f64::EPSILON)
}

/// Stream tag of the air draws.
const AIR_TAG: u64 = 0xA192_E685_0000_0000;

/// The air of one world of `kind` (air-model guide, steps 2-3): gas mix,
/// pressure band and greenhouse offset from the guides' reference rows,
/// drawn within the band from `seed`. Estimates throughout, documented
/// per row.
#[must_use]
pub fn air_for(seed: u64, kind: WorldKind, inputs: &AirInputs) -> Air {
    let absent = Air {
        present: false,
        pressure_bar: None,
        greenhouse_k: 0.0,
        mix: AirMix::Vacuum,
    };
    if matches!(
        kind,
        WorldKind::SubNeptune | WorldKind::IceGiant | WorldKind::GasGiant | WorldKind::HotJupiter
    ) {
        return Air {
            present: true,
            pressure_bar: None,
            greenhouse_k: 0.0,
            mix: AirMix::HydrogenHelium,
        };
    }
    if kind == WorldKind::Rogue {
        if inputs.mass_earth > 0.3 {
            return Air {
                present: true,
                pressure_bar: Some(0.5 + unit01(seed, AIR_TAG, 0) * 1.5),
                greenhouse_k: 10.0 + unit01(seed, AIR_TAG, 1) * 20.0,
                mix: AirMix::HydrogenHelium,
            };
        }
        return absent;
    }
    if !keeps_air(
        inputs.mass_earth,
        inputs.radius_earth,
        inputs.orbit_au,
        inputs.star_mass_solar,
        inputs.age_gyr,
    ) {
        return absent;
    }
    match kind {
        WorldKind::EarthLike => Air {
            present: true,
            pressure_bar: Some(0.5 + unit01(seed, AIR_TAG, 0) * 2.5),
            greenhouse_k: 20.0 + unit01(seed, AIR_TAG, 1) * 30.0,
            mix: AirMix::NitrogenCarbonDioxide,
        },
        WorldKind::Desert | WorldKind::SuperEarth if inputs.t_eq_k > 300.0 => {
            let thick = if kind == WorldKind::SuperEarth {
                2.0
            } else {
                1.0
            };
            Air {
                present: true,
                pressure_bar: Some((10.0 + unit01(seed, AIR_TAG, 0) * 90.0) * thick),
                greenhouse_k: 200.0 + unit01(seed, AIR_TAG, 1) * 300.0,
                mix: AirMix::CarbonDioxide,
            }
        }
        WorldKind::Desert => Air {
            present: true,
            pressure_bar: Some(0.001 + unit01(seed, AIR_TAG, 0) * 0.099),
            greenhouse_k: 2.0 + unit01(seed, AIR_TAG, 1) * 8.0,
            mix: AirMix::CarbonDioxide,
        },
        WorldKind::Lava => Air {
            present: true,
            pressure_bar: Some(0.0005),
            greenhouse_k: 0.0,
            mix: AirMix::RockVapour,
        },
        WorldKind::Ice => Air {
            present: true,
            pressure_bar: Some(0.1 + unit01(seed, AIR_TAG, 0) * 1.4),
            greenhouse_k: 5.0 + unit01(seed, AIR_TAG, 1) * 10.0,
            mix: AirMix::NitrogenMethane,
        },
        WorldKind::Ocean => Air {
            present: true,
            pressure_bar: Some(1.0 + unit01(seed, AIR_TAG, 0) * 9.0),
            greenhouse_k: 25.0 + unit01(seed, AIR_TAG, 1) * 35.0,
            mix: AirMix::WaterMix,
        },
        WorldKind::SuperEarth => Air {
            present: true,
            pressure_bar: Some(1.0 + unit01(seed, AIR_TAG, 0) * 5.0),
            greenhouse_k: 20.0 + unit01(seed, AIR_TAG, 1) * 30.0,
            mix: AirMix::NitrogenCarbonDioxide,
        },
        WorldKind::Iron | WorldKind::Dwarf | WorldKind::Carbon => Air {
            present: true,
            pressure_bar: Some(0.001 + unit01(seed, AIR_TAG, 0) * 0.009),
            greenhouse_k: 0.0,
            mix: AirMix::CarbonDioxide,
        },
        WorldKind::SubNeptune
        | WorldKind::IceGiant
        | WorldKind::GasGiant
        | WorldKind::HotJupiter
        | WorldKind::Rogue => absent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Inputs for a reference world (mass, radius, orbit, star, t_eq).
    fn world(
        mass_earth: f64,
        radius_earth: f64,
        orbit_au: f64,
        star_mass_solar: f64,
        t_eq_k: f64,
    ) -> AirInputs {
        AirInputs {
            mass_earth,
            radius_earth,
            orbit_au,
            star_mass_solar,
            t_eq_k,
            water_01: 0.5,
            age_gyr: 4.5,
        }
    }

    #[test]
    fn air_follows_the_shoreline() {
        // C11 / AC11: Mercury-like and Moon-like worlds come out airless;
        // Mars-, Venus- and Titan-like worlds keep air in their band.
        assert!(
            !keeps_air(0.055, 0.383, 0.39, 1.0, 4.5),
            "mercury keeps air"
        );
        assert!(!keeps_air(0.0123, 0.2727, 1.0, 1.0, 4.5), "moon keeps air");
        assert!(keeps_air(0.107, 0.532, 1.52, 1.0, 4.5), "mars loses air");
        assert!(keeps_air(0.815, 0.949, 0.72, 1.0, 4.5), "venus loses air");
        assert!(keeps_air(0.0225, 0.404, 9.58, 1.0, 4.5), "titan loses air");
        assert!(keeps_air(1.0, 1.0, 1.0, 1.0, 4.57), "earth loses air");
        assert!(keeps_air(318.0, 11.2, 5.2, 1.0, 4.5), "jupiter loses air");
        let escape = escape_velocity_kms(1.0, 1.0);
        assert!((escape - 11.19).abs() < 0.05, "earth escape {escape} km/s");
        // Young red dwarfs push close-in worlds across the line early:
        // a world air-side on the main sequence still loses its air when
        // young, and keeps it when old.
        assert!(
            !keeps_air(0.6, 0.8, 0.011, 0.2, 0.15),
            "close dwarf world keeps air while its star burns bright"
        );
        assert!(
            keeps_air(0.6, 0.8, 0.011, 0.2, 5.0),
            "close dwarf world loses air after settling down"
        );
        assert_eq!(
            keeps_air(0.3, 0.7, 0.05, 0.2, 0.3),
            keeps_air(0.3, 0.7, 0.05, 0.2, 0.3)
        );
    }

    #[test]
    fn air_bands_follow_the_reference_rows() {
        let earth = air_for(7, WorldKind::EarthLike, &world(1.0, 1.0, 1.0, 1.0, 255.0));
        assert!(earth.present && earth.mix == AirMix::NitrogenCarbonDioxide);
        assert!((0.5..=3.0).contains(&earth.pressure_bar.unwrap_or(-1.0)));
        assert!((20.0..=50.0).contains(&earth.greenhouse_k));
        let venus = air_for(7, WorldKind::Desert, &world(0.815, 0.949, 0.72, 1.0, 330.0));
        assert!(venus.present && venus.mix == AirMix::CarbonDioxide);
        assert!((10.0..=100.0).contains(&venus.pressure_bar.unwrap_or(-1.0)));
        assert!((200.0..=500.0).contains(&venus.greenhouse_k));
        let mars = air_for(7, WorldKind::Desert, &world(0.107, 0.532, 1.52, 1.0, 210.0));
        assert!((0.001..=0.1).contains(&mars.pressure_bar.unwrap_or(-1.0)));
        let mercury = air_for(7, WorldKind::Iron, &world(0.055, 0.383, 0.39, 1.0, 440.0));
        assert!(!mercury.present && mercury.mix == AirMix::Vacuum);
        let titan = air_for(7, WorldKind::Ice, &world(0.0225, 0.404, 9.58, 1.0, 82.0));
        assert!(titan.present && titan.mix == AirMix::NitrogenMethane);
        // Seeded within the band, replayed exactly.
        let again = air_for(7, WorldKind::EarthLike, &world(1.0, 1.0, 1.0, 1.0, 255.0));
        assert_eq!(earth, again);
        assert_ne!(
            earth.pressure_bar,
            air_for(8, WorldKind::EarthLike, &world(1.0, 1.0, 1.0, 1.0, 255.0)).pressure_bar
        );
    }
}
