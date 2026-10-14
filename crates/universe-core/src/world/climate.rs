//! Climate: energy-balance temperature, wind and rain belts, rain shadow
//! (#685, T7).
//!
//! Guides: `guides/universe/L10/generation/documents/climate-and-look.md`
//! (Budyko-Sellers energy balance solved as `T(φ) = T0 + T2 P2(sin φ)`
//! with an ice-albedo second pass, Hadley belts widening as spin slows,
//! rain shadows on lee slopes, seasons as annual means). Tidally locked
//! worlds use the star direction instead of latitude; locking itself is
//! wired by T11, so the branch takes an explicit dayness here. Everything
//! is `f64` and deterministic (`E-DET-TIERS`).

use super::noise3::smoothstep;
use super::plates::{BoundaryKind, Plates, boundary_at};
use crate::icosphere::normalize;

/// Reference equilibrium temperature in kelvin (Earth, for scaling).
pub const EARTH_T_EQ_K: f64 = 255.0;

/// Equator-to-pole contrast in °C at Earth transport (about 45 K over the
/// globe, `1.5 × |T2|`).
pub const EARTH_CONTRAST_C: f64 = 30.0;

/// Subtropical dry belt latitude in degrees for a 24-hour spin.
pub const EARTH_BELT_LAT_DEG: f64 = 30.0;

/// Ice-albedo feedback: latitudes first solving below this (°C) cool by
/// [`ICE_FEEDBACK_C`] on the second pass.
pub const ICE_THRESHOLD_C: f64 = -10.0;
/// Ice-albedo cooling in °C on the second pass.
pub const ICE_FEEDBACK_C: f64 = 8.0;

/// Lapse rate in K per km at Earth gravity (scaled by gravity and air).
pub const LAPSE_K_PER_KM: f64 = 6.5;

/// Locked day-side excess over the mean in °C (cloud-covered hot disc,
/// cooled per the guides).
pub const LOCKED_DAY_EXCESS_C: f64 = 30.0;

/// Locked night-side deficit scale (the night radiator; thick air spreads
/// heat and softens it).
pub const LOCKED_NIGHT_DEFICIT_C: f64 = 60.0;

/// Rain-shadow reach past a mountain boundary, in radians.
pub const SHADOW_REACH_RAD: f64 = 0.06;

/// Inputs to the climate model: the air's warming, its thickness, the
/// planet's water and spin, and whether it is tidally locked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClimateInputs {
    /// Equilibrium temperature in kelvin, at the system's age.
    pub t_eq_k: f64,
    /// Greenhouse warming in kelvin.
    pub greenhouse_k: f64,
    /// Surface pressure in bar (`None` with no surface to stand on).
    pub pressure_bar: Option<f64>,
    /// Water share, 0..1.
    pub water_01: f64,
    /// Day length in hours.
    pub spin_hours: f64,
    /// Axial tilt in degrees.
    pub tilt_deg: f64,
    /// Tidally locked to its star (wired by T11; false until then).
    pub locked: bool,
}

/// One planet's climate: two temperature numbers, its belts and its heat
/// transport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Climate {
    /// Global mean surface temperature in °C.
    pub t0_c: f64,
    /// Equator-to-pole contrast in °C (negative: poles colder).
    pub t2_c: f64,
    /// Subtropical dry-belt latitude in degrees.
    pub belt_lat_deg: f64,
    /// Extra dry-band pairs past the primary (fast rotators).
    pub extra_bands: u8,
    /// Scaled heat transport (1 near Earth's; 0.2 airless).
    pub d_scaled: f64,
    /// Surface pressure in bar (0 without air).
    pub pressure_bar: f64,
    /// Water share, 0..1.
    pub water_01: f64,
    /// Tidally locked to its star.
    pub locked: bool,
}

/// Subtropical dry-belt latitude in degrees for a day `spin_hours` long:
/// about 30° for an Earth day, poleward as spin slows (Hadley widening
/// as `Ω^-1/2`, GEN-S047), clamped off the poles. Degenerate spins read
/// Earth's belt.
#[must_use]
pub fn dry_belt_lat_deg(spin_hours: f64) -> f64 {
    if !spin_hours.is_finite() || spin_hours <= 0.0 {
        return EARTH_BELT_LAT_DEG;
    }
    (EARTH_BELT_LAT_DEG * (spin_hours / 24.0).sqrt()).clamp(12.0, 80.0)
}

/// Extra dry-band pairs past the primary (fast rotators band like giants;
/// Earth spins too slowly for any).
#[must_use]
pub fn extra_belt_pairs(spin_hours: f64) -> u8 {
    if !spin_hours.is_finite() {
        return 0;
    }
    if spin_hours < 15.0 {
        2
    } else if spin_hours < 20.0 {
        1
    } else {
        0
    }
}

/// One planet's climate from its air and spin (climate guide, steps 1-2).
#[must_use]
pub fn climate_for(inputs: &ClimateInputs) -> Climate {
    let pressure = inputs.pressure_bar.unwrap_or(0.0);
    let pressure = if pressure.is_finite() && pressure > 0.0 {
        pressure
    } else {
        0.0
    };
    let water = if inputs.water_01.is_finite() {
        inputs.water_01.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let d_scaled = 0.2 + 0.55 * pressure.min(3.0) + 0.5 * water;
    let t_eq = if inputs.t_eq_k.is_finite() && inputs.t_eq_k > 0.0 {
        inputs.t_eq_k
    } else {
        1.0
    };
    let tilt = if inputs.tilt_deg.is_finite() {
        inputs.tilt_deg.clamp(0.0, 177.0)
    } else {
        0.0
    };
    Climate {
        t0_c: t_eq + inputs.greenhouse_k - 273.15,
        t2_c: -EARTH_CONTRAST_C * (t_eq / EARTH_T_EQ_K) / d_scaled.sqrt()
            * (1.0 - tilt / 180.0 * 0.5),
        belt_lat_deg: dry_belt_lat_deg(inputs.spin_hours),
        extra_bands: extra_belt_pairs(inputs.spin_hours),
        d_scaled,
        pressure_bar: pressure,
        water_01: water,
        locked: inputs.locked,
    }
}

/// Legendre `P2(sin φ)` shaping the meridional profile.
fn p2_sin_lat(lat_rad: f64) -> f64 {
    let s = lat_rad.sin();
    (3.0 * s * s - 1.0) / 2.0
}

/// Surface temperature in °C at latitude `lat_rad` and `height_m` metres
/// above the mean radius: the two-pass energy balance (ice latitudes
/// cool once more) minus the lapse rate. With `dayness` (cosine of the
/// angle to the point under the star) the locked branch applies instead:
/// a hot cloud-covered disc, a cold night radiator, and a mild
/// terminator ring whose width grows with transport.
#[must_use]
pub fn temperature_c(
    climate: &Climate,
    lat_rad: f64,
    height_m: f64,
    gravity: f64,
    dayness: Option<f64>,
) -> f64 {
    let lapse = if gravity.is_finite() && gravity > 0.0 {
        LAPSE_K_PER_KM * (gravity / 9.81) * climate.pressure_bar.min(1.0)
    } else {
        0.0
    };
    let surface = if let Some(cos_theta) = dayness {
        let day = climate.t0_c + LOCKED_DAY_EXCESS_C;
        let night = climate.t0_c - LOCKED_NIGHT_DEFICIT_C / (1.0 + climate.d_scaled);
        night + (day - night) * cos_theta.max(0.0).sqrt()
    } else {
        let first = climate.t0_c + climate.t2_c * p2_sin_lat(lat_rad);
        if first < ICE_THRESHOLD_C {
            first - ICE_FEEDBACK_C
        } else {
            first
        }
    };
    surface - lapse * height_m.max(0.0) / 1_000.0
}

/// Rain in 0..1 at latitude `lat_rad`: wet equator, dry subtropics,
/// wetter mid-latitudes, dry poles, extra bands on fast rotators;
/// `shadow_01` (lee slopes) and high continental interiors dry out.
#[must_use]
pub fn rain_01(climate: &Climate, lat_rad: f64, shadow_01: f64, elev_m: f64, is_land: bool) -> f64 {
    let lat = lat_rad.abs().to_degrees();
    let gauss = |centre: f64, width: f64| {
        let t = (lat - centre) / width;
        (-t * t).exp()
    };
    let mut dry = gauss(climate.belt_lat_deg, 12.0);
    for k in 1..=climate.extra_bands {
        let band = climate.belt_lat_deg
            + f64::from(k) * (90.0 - climate.belt_lat_deg) / f64::from(climate.extra_bands + 1);
        dry = dry.max(gauss(band, 8.0));
    }
    let mut rain = 0.15 + 0.75 * gauss(0.0, 10.0) - 0.85 * dry + 0.5 * gauss(55.0, 18.0)
        - 0.3 * gauss(85.0, 10.0);
    rain *= 1.0 - 0.7 * shadow_01.clamp(0.0, 1.0);
    if is_land {
        rain *= 1.0 - 0.25 * smoothstep(1_000.0, 2_500.0, elev_m);
    }
    rain.clamp(0.0, 1.0)
}

/// Rain shadow in 0..1 at `direction`: lee slopes past convergent
/// boundaries dry out (climate guide, step 2). The wind blows zonally
/// (trades, westerlies, polar easterlies by belt); past a mountain
/// boundary on the downwind side within reach the shadow holds at half
/// strength on both flanks (side-unresolved staging, documented).
#[must_use]
pub fn rain_shadow_01(
    field: &Plates,
    direction: [f64; 3],
    radius_m: f64,
    belt_lat_deg: f64,
) -> f64 {
    if !direction.iter().all(|c| c.is_finite()) || !radius_m.is_finite() || radius_m <= 0.0 {
        return 0.0;
    }
    let unit = normalize(direction);
    let lat_deg = unit[1].clamp(-1.0, 1.0).asin().to_degrees().abs();
    let zone: f64 = if lat_deg < belt_lat_deg {
        -1.0
    } else if lat_deg < 65.0 {
        1.0
    } else {
        -1.0
    };
    if unit[0] * unit[0] + unit[2] * unit[2] < 1e-12 {
        return 0.0;
    }
    let east = normalize([-unit[2], 0.0, unit[0]]);
    let wind = [east[0] * zone, 0.0, east[2] * zone];
    let Some(edge) = boundary_at(field, unit) else {
        return 0.0;
    };
    let mountain = match edge.kind {
        BoundaryKind::ConvergentOceanContinent | BoundaryKind::ConvergentContinentContinent => 1.0,
        BoundaryKind::ConvergentOceanOcean => 0.7,
        BoundaryKind::Divergent | BoundaryKind::Transform => 0.0,
    };
    if mountain <= 0.0 {
        return 0.0;
    }
    let along = (wind[0] * edge.normal[0] + wind[2] * edge.normal[2]).abs();
    if along < 0.3 {
        return 0.0;
    }
    mountain * 0.5 * (1.0 - (edge.edge_dist_rad / SHADOW_REACH_RAD).min(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::plates::{
        BoundaryKind, Plates, boundary_at, plate_field_for, tectonic_input_for,
    };

    /// Earth-like climate inputs (255 K, +33 K greenhouse, 1 bar).
    fn earth_inputs() -> ClimateInputs {
        ClimateInputs {
            t_eq_k: 255.0,
            greenhouse_k: 33.0,
            pressure_bar: Some(1.0),
            water_01: 0.5,
            spin_hours: 24.0,
            tilt_deg: 23.4,
            locked: false,
        }
    }

    #[test]
    fn belts_widen_as_spin_slows() {
        assert!((dry_belt_lat_deg(24.0) - 30.0).abs() < 1e-9);
        assert!(dry_belt_lat_deg(240.0) > dry_belt_lat_deg(24.0));
        assert!(dry_belt_lat_deg(6.0) < dry_belt_lat_deg(24.0));
        assert_eq!(dry_belt_lat_deg(f64::NAN), EARTH_BELT_LAT_DEG);
        assert_eq!(extra_belt_pairs(24.0), 0);
        assert!(extra_belt_pairs(10.0) > 0);
    }

    #[test]
    fn earth_temperature_reads_temperate_with_ice_caps() {
        let climate = climate_for(&earth_inputs());
        assert!((climate.t0_c - 15.0).abs() < 2.0, "mean {}", climate.t0_c);
        let equator = temperature_c(&climate, 0.0, 0.0, 9.81, None);
        assert!((20.0..35.0).contains(&equator), "equator {equator}");
        let pole = temperature_c(&climate, std::f64::consts::FRAC_PI_2, 0.0, 9.81, None);
        assert!(pole < -5.0, "no ice cap: {pole}");
        let mid = temperature_c(&climate, 45.0f64.to_radians(), 0.0, 9.81, None);
        assert!((0.0..15.0).contains(&mid), "mid-latitudes {mid}");
        // Lapse cools the heights; thin air barely lapses.
        let high = temperature_c(&climate, 0.0, 5_000.0, 9.81, None);
        assert!(
            (high - (equator - 32.5)).abs() < 1.0,
            "lapse {high} vs {equator}"
        );
        assert_eq!(temperature_c(&climate, 0.0, 0.0, 9.81, None), equator);
    }

    #[test]
    fn locked_worlds_contrast_day_and_night() {
        let climate = climate_for(&ClimateInputs {
            locked: true,
            ..earth_inputs()
        });
        let noon = temperature_c(&climate, 0.0, 0.0, 9.81, Some(1.0));
        let dusk = temperature_c(&climate, 0.0, 0.0, 9.81, Some(0.1));
        let night = temperature_c(&climate, 0.0, 0.0, 9.81, Some(-1.0));
        assert!(noon > dusk && dusk > night, "{noon} {dusk} {night}");
        assert!(dusk < climate.t0_c + 20.0, "no mild terminator: {dusk}");
    }

    #[test]
    fn rain_falls_in_belts_and_lee_slopes() {
        let climate = climate_for(&earth_inputs());
        let equator = rain_01(&climate, 0.0, 0.0, 0.0, true);
        let desert = rain_01(&climate, 30.0f64.to_radians(), 0.0, 500.0, true);
        let storm = rain_01(&climate, 55.0f64.to_radians(), 0.0, 0.0, true);
        assert!(equator > 0.6, "dry equator: {equator}");
        assert!(desert < 0.35, "wet subtropics: {desert}");
        assert!(storm > desert, "storm track drier than desert: {storm}");
        let plain = rain_01(&climate, 10.0f64.to_radians(), 0.0, 500.0, true);
        let shadowed = rain_01(&climate, 10.0f64.to_radians(), 1.0, 500.0, true);
        assert!(shadowed < plain, "shadow wets: {shadowed} vs {plain}");
        // Mountain boundaries cast shadows; rifts cast none.
        let convergent = find_kind(BoundaryKind::ConvergentOceanContinent);
        let conv_shadow = rain_shadow_01(
            &convergent.0,
            convergent.1,
            super::super::plates::EARTH_RADIUS_M,
            30.0,
        );
        assert!(conv_shadow > 0.0, "no shadow past a mountain boundary");
        let divergent = find_kind(BoundaryKind::Divergent);
        assert_eq!(
            rain_shadow_01(
                &divergent.0,
                divergent.1,
                super::super::plates::EARTH_RADIUS_M,
                30.0
            ),
            0.0,
            "a rift casts a shadow"
        );
    }

    /// First field with a boundary of `want`, and its edge point.
    fn find_kind(want: BoundaryKind) -> (Plates, [f64; 3]) {
        for seed in 0..200u64 {
            let input = tectonic_input_for(seed);
            let Some(field) = plate_field_for(seed, super::super::plates::EARTH_RADIUS_M, &input)
            else {
                continue;
            };
            for dir in [
                [0.3, 0.8, 0.5],
                [-0.4, 0.2, 0.7],
                [0.1, -0.6, 0.4],
                [0.7, 0.1, -0.5],
            ] {
                if boundary_at(&field, dir)
                    .is_some_and(|edge| edge.kind == want && edge.edge_dist_rad < 0.02)
                {
                    return (field, dir);
                }
            }
        }
        panic!("no {want:?} boundary in range");
    }
}
