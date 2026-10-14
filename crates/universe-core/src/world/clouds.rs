//! Clouds, drift and sky tint (#685, T9).
//!
//! Guides: `guides/universe/L10/generation/documents/climate-and-look.md`
//! (steps 5-6: cover from the rain belts, two shells, drift with the belt
//! winds, sky tint from thickness, dust, haze and starlight). Cover and
//! drift are pure `f64` and deterministic (`E-DET-TIERS`): the same seed
//! and clock give the same clouds. The shell geometry lives in render
//! (visual only, so GPU float noise is acceptable there); what this module
//! owns is the climate-derived cover share, the belt-wind drift and the
//! air-derived tint, all headless-tested.

use super::air::Air;
use crate::r#gen::StarKind;

/// Stream tag of the belt-wind draws.
const CLOUD_TAG: u64 = 0xC10D_E685_0000_0000;

/// A share in 0..1 from the hash stream (in-band draws).
fn share01(seed: u64, lane: u64) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: 53-bit hash share as a unit float, intended"
    )]
    let share =
        (crate::seed::hash_triple(seed, CLOUD_TAG, lane) >> 11) as f64 / 9_007_199_254_740_992.0;
    share.clamp(0.0, 1.0 - f64::EPSILON)
}

/// Cloud-cover share in 0..1 at latitude `lat_rad`: heavy over the
/// equatorial rain belt and the mid-latitude storm tracks, thin over the
/// subtropical dry belts and the poles (climate guide, step 5). The belt
/// geometry is the rain-belt geometry
/// ([`super::climate::rain_01`]): `belt_lat_deg` and `extra_bands` come
/// from the spin ([`super::climate::dry_belt_lat_deg`],
/// [`super::climate::extra_belt_pairs`]). A non-finite latitude reads the
/// neutral mid cover.
#[must_use]
pub fn cloud_cover_01(belt_lat_deg: f64, extra_bands: u8, lat_rad: f64) -> f64 {
    if !lat_rad.is_finite() {
        return 0.5;
    }
    let belt = if belt_lat_deg.is_finite() {
        belt_lat_deg.clamp(12.0, 80.0)
    } else {
        super::climate::EARTH_BELT_LAT_DEG
    };
    let lat = lat_rad.abs().to_degrees();
    let gauss = |centre: f64, width: f64| {
        let t = (lat - centre) / width;
        (-t * t).exp()
    };
    let mut dry = gauss(belt, 12.0);
    for k in 1..=extra_bands {
        let band = belt + f64::from(k) * (90.0 - belt) / f64::from(extra_bands + 1);
        dry = dry.max(gauss(band, 8.0));
    }
    (0.30 + 0.55 * gauss(0.0, 12.0) - 0.70 * dry + 0.40 * gauss(55.0, 18.0)
        - 0.20 * gauss(85.0, 10.0))
    .clamp(0.0, 1.0)
}

/// Zonal wind in radians per second at latitude `lat_rad`: westward near
/// the equator (trades), eastward in mid-latitudes (westerlies), weak
/// polar easterlies (climate guide, step 2; the rain-shadow zoning reads
/// the same belts). Magnitude is a small seeded share of the spin rate
/// per belt, so fast spinners drift faster; degenerate spins read an
/// Earth day. A non-finite latitude reads still air.
#[must_use]
pub fn belt_wind_rad_s(lat_rad: f64, spin_hours: f64, seed: u64) -> f64 {
    if !lat_rad.is_finite() {
        return 0.0;
    }
    let spin = if spin_hours.is_finite() && spin_hours > 0.0 {
        spin_hours
    } else {
        24.0
    };
    let lat_deg = lat_rad.abs().to_degrees();
    let belt = super::climate::dry_belt_lat_deg(spin);
    let (lane, sign) = if lat_deg < belt {
        (0, -1.0)
    } else if lat_deg < 65.0 {
        (1, 1.0)
    } else {
        (2, -0.5)
    };
    let rate = std::f64::consts::TAU / (spin * 3_600.0);
    sign * rate * 0.08 * (0.7 + 0.6 * share01(seed, lane))
}

/// `direction` carried by the belt wind over `elapsed_s` seconds: rotated
/// about the spin axis by `ω(lat) × t` (climate guide, step 5). The render
/// samples its cloud noise here, so shells drift with the winds from the
/// seed and the clock alone. Degenerate input reads back unchanged.
#[must_use]
pub fn drifted_dir(direction: [f64; 3], elapsed_s: f64, spin_hours: f64, seed: u64) -> [f64; 3] {
    if !direction.iter().all(|c| c.is_finite()) || !elapsed_s.is_finite() || elapsed_s == 0.0 {
        return direction;
    }
    let unit = crate::icosphere::normalize(direction);
    let lat = unit[1].clamp(-1.0, 1.0).asin();
    let angle = belt_wind_rad_s(lat, spin_hours, seed) * elapsed_s;
    if angle == 0.0 {
        return unit;
    }
    let (sin, cos) = angle.sin_cos();
    [
        unit[0] * cos - unit[2] * sin,
        unit[1],
        unit[0] * sin + unit[2] * cos,
    ]
}

/// Sky and rim tint in linear RGB for `air` under a `star` (climate
/// guide, step 6): thin clean air reads blue, dusty air tan-orange, thick
/// haze orange, hydrogen envelopes pale amber, rock vapour dark rust,
/// humid envelopes pale cyan; starlight multiplies everything (redder
/// under cool stars). `None` with no air kept (airless worlds draw no rim,
/// as before).
#[must_use]
pub fn sky_tint(air: &Air, star: StarKind) -> Option<[f64; 3]> {
    if !air.present {
        return None;
    }
    let pressure = air.pressure_bar.unwrap_or(0.0);
    let pressure = if pressure.is_finite() && pressure > 0.0 {
        pressure
    } else {
        0.0
    };
    let base = match air.mix {
        super::air::AirMix::Vacuum => return None,
        super::air::AirMix::NitrogenCarbonDioxide => {
            if pressure >= 2.0 {
                [0.42, 0.62, 1.0]
            } else if pressure >= 0.3 {
                [0.55, 0.75, 1.0]
            } else {
                [0.70, 0.80, 0.95]
            }
        }
        super::air::AirMix::CarbonDioxide => {
            if pressure >= 5.0 {
                [0.92, 0.82, 0.66]
            } else {
                [0.85, 0.65, 0.45]
            }
        }
        super::air::AirMix::NitrogenMethane => {
            if pressure >= 1.0 {
                [0.95, 0.55, 0.25]
            } else {
                [0.95, 0.65, 0.35]
            }
        }
        super::air::AirMix::HydrogenHelium => [1.0, 0.85, 0.60],
        super::air::AirMix::RockVapour => [0.50, 0.25, 0.20],
        super::air::AirMix::WaterMix => [0.70, 0.85, 0.90],
    };
    let star = crate::system::star_tint(star);
    let tint = [
        base[0] * star.red,
        base[1] * star.green,
        base[2] * star.blue,
    ];
    if tint.iter().all(|c| c.is_finite()) {
        Some([
            tint[0].clamp(0.0, 1.0),
            tint[1].clamp(0.0, 1.0),
            tint[2].clamp(0.0, 1.0),
        ])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::climate::{climate_for, dry_belt_lat_deg, extra_belt_pairs};

    /// Earth-like belt geometry (24-hour spin).
    fn earth_belts() -> (f64, u8) {
        (dry_belt_lat_deg(24.0), extra_belt_pairs(24.0))
    }

    #[test]
    fn cloud_cover_follows_rain_belts() {
        // T9: heavy over the equatorial rain belt and the storm tracks,
        // thin over the subtropical deserts; slower spins push the dry
        // minimum poleward with the belts.
        let (belt, extra) = earth_belts();
        let equator = cloud_cover_01(belt, extra, 0.0);
        let desert = cloud_cover_01(belt, extra, 30.0f64.to_radians());
        let storm = cloud_cover_01(belt, extra, 55.0f64.to_radians());
        assert!(equator > 0.6, "dry equator: {equator}");
        assert!(desert < 0.35, "wet subtropics: {desert}");
        assert!(storm > desert, "storm track drier than desert: {storm}");
        assert!(
            equator > storm,
            "storm wetter than equator: {equator} vs {storm}"
        );
        for lat in [-80, -45, -15, 0, 15, 45, 80] {
            let cover = cloud_cover_01(belt, extra, f64::from(lat).to_radians());
            assert!((0.0..=1.0).contains(&cover), "cover out of share: {cover}");
        }
        assert_eq!(cloud_cover_01(belt, extra, f64::NAN), 0.5);
        // A slow spinner's belts sit poleward: 30° reads wetter than on a
        // fast Earth day.
        let (slow_belt, slow_extra) = (dry_belt_lat_deg(240.0), extra_belt_pairs(240.0));
        assert!(slow_belt > belt, "belts did not widen");
        assert!(
            cloud_cover_01(slow_belt, slow_extra, 30.0f64.to_radians()) > desert,
            "dry belt did not move poleward"
        );
        // Same belts, same cover.
        assert_eq!(
            cloud_cover_01(belt, extra, 0.3),
            cloud_cover_01(belt, extra, 0.3)
        );
        // The climate's own belts feed the cover (wiring check).
        let climate = climate_for(&crate::world::climate::ClimateInputs {
            t_eq_k: 255.0,
            greenhouse_k: 33.0,
            pressure_bar: Some(1.0),
            water_01: 0.5,
            spin_hours: 24.0,
            tilt_deg: 23.4,
            locked: false,
        });
        assert_eq!(climate.belt_lat_deg, belt);
        assert_eq!(climate.extra_bands, extra);
    }

    #[test]
    fn cloud_drift_moves_with_belt_winds() {
        // T9: still at rest, westward over the equator (trades), eastward
        // in mid-latitudes (westerlies); replayed exactly.
        let seed = 11u64;
        let spin = 24.0;
        let equator = [1.0, 0.0, 0.0];
        assert_eq!(drifted_dir(equator, 0.0, spin, seed), equator);
        let lon = |dir: [f64; 3]| dir[2].atan2(dir[0]);
        let after = drifted_dir(equator, 3_600.0, spin, seed);
        assert!(
            lon(after) < lon(equator),
            "equator drifts east: {} vs {}",
            lon(after),
            lon(equator)
        );
        let mid = crate::icosphere::normalize([1.0, 1.0, 0.0]);
        let mid_after = drifted_dir(mid, 3_600.0, spin, seed);
        assert!(
            lon(mid_after) > lon(mid),
            "mid-latitudes drift west: {} vs {}",
            lon(mid_after),
            lon(mid)
        );
        // Unit length kept, replayed, faster spins drift faster.
        for dir in [equator, mid, [0.0, 1.0, 0.0]] {
            let moved = drifted_dir(dir, 7_200.0, spin, seed);
            let reach = (moved[0] * moved[0] + moved[1] * moved[1] + moved[2] * moved[2]).sqrt();
            assert!((reach - 1.0).abs() < 1e-12, "drift leaves the sphere");
            assert_eq!(moved, drifted_dir(dir, 7_200.0, spin, seed));
        }
        let fast = drifted_dir(equator, 3_600.0, 6.0, seed);
        let slow = drifted_dir(equator, 3_600.0, 24.0, seed);
        assert!(
            (lon(equator) - lon(fast)).abs() > (lon(equator) - lon(slow)).abs(),
            "fast spinners drift faster"
        );
        // Degenerate input reads back unchanged.
        let nan = drifted_dir([f64::NAN, 0.0, 0.0], 100.0, spin, seed);
        assert!(
            nan[0].is_nan() && nan[1] == 0.0 && nan[2] == 0.0,
            "degenerate drift changed: {nan:?}"
        );
        assert_eq!(belt_wind_rad_s(f64::NAN, spin, seed), 0.0);
    }

    #[test]
    fn sky_tint_follows_air_and_star() {
        // T9: Earth blue, Titan orange, dusty thin air tan; airless draws
        // nothing; cool stars redden the tint.
        use super::super::air::{Air, AirMix};
        let earth = Air {
            present: true,
            pressure_bar: Some(1.0),
            greenhouse_k: 33.0,
            mix: AirMix::NitrogenCarbonDioxide,
        };
        let tint = sky_tint(&earth, StarKind::G).expect("earth holds a sky");
        for (got, want) in tint.iter().zip([0.55, 0.75, 1.0]) {
            assert!((got - want).abs() < 0.05, "earth sky {tint:?}");
        }
        let titan = Air {
            present: true,
            pressure_bar: Some(1.5),
            greenhouse_k: 10.0,
            mix: AirMix::NitrogenMethane,
        };
        let haze = sky_tint(&titan, StarKind::G).expect("titan holds haze");
        assert!(
            haze[0] > 0.9 && haze[1] < 0.75 && haze[2] < 0.45,
            "{haze:?}"
        );
        let dusty = Air {
            present: true,
            pressure_bar: Some(0.01),
            greenhouse_k: 5.0,
            mix: AirMix::CarbonDioxide,
        };
        let tan = sky_tint(&dusty, StarKind::G).expect("dust holds a sky");
        assert!(tan[0] > tan[2] + 0.2, "dust not tan: {tan:?}");
        let vacuum = Air {
            present: false,
            pressure_bar: None,
            greenhouse_k: 0.0,
            mix: AirMix::Vacuum,
        };
        assert_eq!(sky_tint(&vacuum, StarKind::G), None);
        assert_eq!(
            sky_tint(&earth, StarKind::M),
            sky_tint(&earth, StarKind::M),
            "replay differs"
        );
        let red = sky_tint(&earth, StarKind::M).expect("red sun still lights a sky");
        assert!(
            red[0] >= red[1] && red[1] > red[2],
            "cool star did not redden: {red:?}"
        );
    }
}
