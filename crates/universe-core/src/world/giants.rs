//! Giant band fields, storms and rings (#685, T9).
//!
//! Guides: `guides/universe/L10/generation/documents/planet-types.md`
//! (gas giants: bands from the spin, turbulence at band edges, storms with
//! a seeded great spot; ice giants faintly banded; hot Jupiters dark with
//! night-side glow; sub-Neptunes hazy) and `moons.md` (rings: dense and
//! faint bands, gaps at small moons, denser edge-on). Everything is pure
//! `f64` and deterministic (`E-DET-TIERS`); bands and storms are analytic
//! (sines plus seeded ellipses, no noise octaves), so the ground colour
//! budget stays at 39 of 40. Rocky kinds read a flat grey here; the biome
//! pass only calls this for giants.

use super::classify::WorldKind;
use super::noise3::smoothstep;

/// Stream tag of the giant draws (bands, storms, rings).
const GIANT_TAG: u64 = 0x61A7_E685_0000_0000;

/// Most storms evaluated per sample (fixed array, no allocation).
const MAX_STORMS: usize = 6;

/// Grey for non-giant kinds (never addressed by the biome pass).
const FLAT_GREY: [f64; 3] = [0.45, 0.45, 0.45];

/// A share in 0..1 from the hash stream (in-band draws).
fn share01(seed: u64, lane: u64) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: 53-bit hash share as a unit float, intended"
    )]
    let share =
        (crate::seed::hash_triple(seed, GIANT_TAG, lane) >> 11) as f64 / 9_007_199_254_740_992.0;
    share.clamp(0.0, 1.0 - f64::EPSILON)
}

/// Mix of two linear RGB colours by `t` (0 all `a`, 1 all `b`).
fn mix3(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Band count for a day `spin_hours` long: faster spins wear narrower,
/// more numerous bands (Rhines scaling, planet-types guide). Two to eight;
/// degenerate spins read three (an Earth day).
#[must_use]
pub fn band_count(spin_hours: f64) -> u8 {
    if !spin_hours.is_finite() || spin_hours <= 0.0 {
        return 3;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: band count 2..8 always fits u8"
    )]
    #[expect(
        clippy::cast_sign_loss,
        reason = "E-CAST: band count clamped non-negative, saturates like `as`"
    )]
    return (3.0 * (24.0 / spin_hours).sqrt().round()).clamp(2.0, 8.0) as u8;
}

/// One storm oval (fixed array element; only the first `count` apply).
#[derive(Debug, Clone, Copy)]
struct Storm {
    /// Centre latitude in -1..1 (pole-normalized).
    lat_norm: f64,
    /// Centre longitude in radians.
    lon: f64,
    /// Half-size in latitude units.
    size_lat: f64,
    /// Half-size in longitude units.
    size_lon: f64,
    /// Oval colour in linear RGB.
    colour: [f64; 3],
}

/// Band-field colour in linear RGB at latitude `lat_rad`, longitude
/// `lon_rad` on a giant of `kind` spinning once per `spin_hours`: latitude
/// bands from the spin, turbulence warping strongest at band edges,
/// seeded storm ovals with a long-lived great spot on some gas giants,
/// faint bands and dark spots on ice giants, dark day sides with night
/// glow on hot Jupiters (`dayness`: cosine to the point under the star;
/// `None` reads the uniform day side, locking is wired by T11), soft haze
/// on sub-Neptunes. Rocky kinds read flat grey. Degenerate angles read
/// the unstormed mid-band mix.
#[must_use]
pub fn band_colour(
    seed: u64,
    kind: WorldKind,
    lat_rad: f64,
    lon_rad: f64,
    spin_hours: f64,
    dayness: Option<f64>,
) -> [f64; 3] {
    let gas = matches!(
        kind,
        WorldKind::GasGiant | WorldKind::IceGiant | WorldKind::HotJupiter | WorldKind::SubNeptune
    );
    if !gas {
        return FLAT_GREY;
    }
    // Palette and turbulence per kind.
    let (zone, belt, warp_amp) = match kind {
        WorldKind::GasGiant => {
            if share01(seed, 10) < 0.5 {
                ([0.87, 0.76, 0.60], [0.62, 0.46, 0.32], 0.35)
            } else {
                ([0.88, 0.80, 0.63], [0.72, 0.62, 0.46], 0.30)
            }
        }
        WorldKind::IceGiant => ([0.48, 0.72, 0.77], [0.40, 0.64, 0.71], 0.12),
        WorldKind::HotJupiter => ([0.27, 0.19, 0.17], [0.22, 0.15, 0.14], 0.05),
        _ => ([0.47, 0.61, 0.71], [0.42, 0.57, 0.67], 0.05),
    };
    if !lat_rad.is_finite() || !lon_rad.is_finite() {
        return mix3(belt, zone, 0.5);
    }
    let lat = lat_rad.clamp(-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2);
    let lat_norm = lat / std::f64::consts::FRAC_PI_2;
    let bands = f64::from(band_count(spin_hours));
    let phase0 = share01(seed, 11) * std::f64::consts::TAU;
    let phase1 = share01(seed, 12) * std::f64::consts::TAU;
    // Band profile with edge-turbulent warp (shear peaks at transitions).
    let arg0 = lat_norm * bands * std::f64::consts::PI + phase0;
    let profile0 = 0.5 + 0.5 * arg0.sin();
    let edge = 1.0 - (2.0 * profile0 - 1.0) * (2.0 * profile0 - 1.0);
    let arg = arg0 + warp_amp * (2.0 * lon_rad + phase1 + 2.0 * lat_norm).sin() * edge;
    let profile = 0.5 + 0.5 * arg.sin();
    let mut colour = mix3(belt, zone, profile);
    // Ice giants: faint bands plus bright high-cloud streaks.
    if kind == WorldKind::IceGiant {
        let streak = smoothstep(0.70, 0.95, (3.0 * lon_rad + phase1 + 6.0 * lat_norm).sin());
        colour = mix3(colour, [0.90, 0.95, 0.97], streak * 0.35);
    }
    // Hot Jupiters: dark days, ember glow on the night side.
    if kind == WorldKind::HotJupiter
        && let Some(d) = dayness
        && d.is_finite()
    {
        let night = (-d).clamp(0.0, 1.0);
        colour = mix3(colour, [0.90, 0.35, 0.10], night * 0.55);
    }
    // Storms: ovals at band latitudes, a great red spot on some.
    let great = kind == WorldKind::GasGiant && share01(seed, 13) < 0.35;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: storm count below MAX_STORMS always fits usize"
    )]
    #[expect(
        clippy::cast_sign_loss,
        reason = "E-CAST: storm count non-negative, saturates like `as`"
    )]
    let count = match kind {
        WorldKind::GasGiant => 2 + (share01(seed, 14) * 4.0) as usize,
        WorldKind::IceGiant => 1 + (share01(seed, 14) * 2.0) as usize,
        WorldKind::SubNeptune => (share01(seed, 14) * 2.0) as usize,
        _ => 0,
    }
    .min(MAX_STORMS);
    let lon_wrapped = lon_rad.rem_euclid(std::f64::consts::TAU);
    for (i, lane_base) in [20u64, 24, 28, 32, 36, 40].iter().enumerate() {
        if i >= count {
            continue;
        }
        let lane = *lane_base;
        let storm = if great && i == 0 {
            Storm {
                lat_norm: -(0.15 + 0.20 * share01(seed, lane)),
                lon: share01(seed, lane + 1) * std::f64::consts::TAU,
                size_lat: 0.10 + 0.06 * share01(seed, lane + 2),
                size_lon: 0.30 + 0.25 * share01(seed, lane + 3),
                colour: [0.85, 0.42, 0.25],
            }
        } else if kind == WorldKind::IceGiant {
            Storm {
                lat_norm: (share01(seed, lane) * 2.0 - 1.0) * 0.7,
                lon: share01(seed, lane + 1) * std::f64::consts::TAU,
                size_lat: 0.06 + 0.06 * share01(seed, lane + 2),
                size_lon: 0.15 + 0.20 * share01(seed, lane + 3),
                colour: [0.25, 0.40, 0.60],
            }
        } else {
            Storm {
                lat_norm: (share01(seed, lane) * 2.0 - 1.0) * 0.8,
                lon: share01(seed, lane + 1) * std::f64::consts::TAU,
                size_lat: 0.05 + 0.06 * share01(seed, lane + 2),
                size_lon: 0.12 + 0.18 * share01(seed, lane + 3),
                colour: [0.92, 0.88, 0.80],
            }
        };
        let dlon = (lon_wrapped - storm.lon.rem_euclid(std::f64::consts::TAU)
            + 3.0 * std::f64::consts::PI)
            .rem_euclid(std::f64::consts::TAU)
            - std::f64::consts::PI;
        let dlat = (lat_norm - storm.lat_norm) / storm.size_lat;
        let dlon = dlon * lat.cos() / storm.size_lon;
        let r2 = dlat * dlat + dlon * dlon;
        if r2 < 1.0 {
            let w = (1.0 - r2) * (1.0 - r2);
            colour = mix3(colour, storm.colour, w * 0.9);
        }
    }
    colour
}

/// Whether the planet wears rings (moons guide: rare and young): gas
/// giants often, ice giants sometimes, others almost never, young systems
/// likelier. T11 may promote decayed moons into rings; the seed decides
/// here.
#[must_use]
pub fn has_rings(seed: u64, kind: WorldKind, age_gyr: f64) -> bool {
    let base: f64 = match kind {
        WorldKind::GasGiant => 0.30,
        WorldKind::IceGiant => 0.20,
        WorldKind::SubNeptune => 0.08,
        WorldKind::HotJupiter => 0.05,
        _ => 0.015,
    };
    let youth = if !age_gyr.is_finite() {
        1.0
    } else if age_gyr < 2.0 {
        1.5
    } else if age_gyr > 4.0 {
        0.5
    } else {
        1.0
    };
    share01(seed, 40) < (base * youth).clamp(0.0, 0.6)
}

/// Ring inner and outer radii in planet radii (a Saturn-like span with a
/// seeded draw).
#[must_use]
pub fn ring_edges(seed: u64) -> (f64, f64) {
    let inner = 1.20 + 0.15 * share01(seed, 41);
    (inner, inner + 0.60 + 0.60 * share01(seed, 42))
}

/// Ring optical depth at `r_share` across the ring (0 inner edge, 1
/// outer): a dense middle band, faint inner and outer reaches, fine
/// banding, and narrow gaps at seeded radii (small moons). Outside
/// `0..=1` reads empty.
#[must_use]
pub fn ring_tau(seed: u64, r_share: f64) -> f64 {
    if !r_share.is_finite() || !(0.0..=1.0).contains(&r_share) {
        return 0.0;
    }
    for i in 0..3u64 {
        let centre = 0.20 + 0.60 * share01(seed, 50 + i);
        let width = 0.008 + 0.012 * share01(seed, 53 + i);
        if (r_share - centre).abs() < width {
            return 0.03;
        }
    }
    let t = (r_share - 0.55) / 0.20;
    let phase = share01(seed, 56) * std::f64::consts::TAU;
    let tau = 1.25 * (-t * t).exp()
        + 0.15
        + 0.15 * (36.0 * r_share + phase).sin() * r_share * (1.0 - r_share) * 4.0;
    tau * smoothstep(0.0, 0.05, r_share) * (1.0 - smoothstep(0.95, 1.0, r_share))
}

/// Ring opacity at `r_share` seen through `view_mu = |cos view angle|`:
/// `1 − exp(−τ/μ)`, so rings read denser edge-on (moons guide). Outside
/// the ring reads clear; degenerate views read face-on.
#[must_use]
pub fn ring_alpha(seed: u64, r_share: f64, view_mu: f64) -> f64 {
    let tau = ring_tau(seed, r_share);
    if tau <= 0.0 {
        return 0.0;
    }
    let mu = if view_mu.is_finite() {
        view_mu.clamp(0.05, 1.0)
    } else {
        1.0
    };
    (1.0 - (-tau / mu).exp()).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn giant_bands_follow_spin() {
        // T9: faster spins wear more bands; gas giants contrast, ice
        // giants faint, hot Jupiters dark with night glow; rocky kinds
        // read flat grey.
        assert_eq!(band_count(24.0), 3);
        assert!(band_count(10.0) > band_count(24.0));
        assert!(band_count(24.0) >= band_count(100.0));
        assert!((2..=8).contains(&band_count(6.0)));
        assert_eq!(band_count(f64::NAN), 3);
        assert_eq!(band_count(0.0), 3);
        // Bands across latitude: many tints, replayed, seed-sensitive.
        let sweep = |seed: u64, kind: WorldKind, spin: f64| {
            let mut out = Vec::new();
            for i in 0..33 {
                let lat = -1.4 + 2.8 * f64::from(i) / 32.0;
                out.push(band_colour(seed, kind, lat, 0.7, spin, None));
            }
            out
        };
        let gas = sweep(7, WorldKind::GasGiant, 10.0);
        let distinct: std::collections::BTreeSet<String> =
            gas.iter().map(|c| format!("{c:.4?}")).collect();
        assert!(distinct.len() >= 8, "no bands: {}", distinct.len());
        assert_eq!(gas, sweep(7, WorldKind::GasGiant, 10.0));
        assert_ne!(gas, sweep(8, WorldKind::GasGiant, 10.0));
        for colour in &gas {
            assert!(
                colour
                    .iter()
                    .all(|c| c.is_finite() && (0.0..=1.0).contains(c))
            );
        }
        // Contrast follows kind: gas bold, ice faint (latitude means over
        // longitudes, so storms and streaks average out and the bands
        // themselves are compared).
        let spread = |colours: &[[f64; 3]]| {
            let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
            for colour in colours {
                for c in colour {
                    lo = lo.min(*c);
                    hi = hi.max(*c);
                }
            }
            hi - lo
        };
        let means = |seed: u64, kind: WorldKind, spin: f64| {
            let mut out = Vec::new();
            for i in 0..33 {
                let lat = -1.4 + 2.8 * f64::from(i) / 32.0;
                let mut mean = [0.0; 3];
                for j in 0..8 {
                    let lon = f64::from(j) * std::f64::consts::TAU / 8.0;
                    let colour = band_colour(seed, kind, lat, lon, spin, None);
                    mean = [
                        mean[0] + colour[0],
                        mean[1] + colour[1],
                        mean[2] + colour[2],
                    ];
                }
                out.push([mean[0] / 8.0, mean[1] / 8.0, mean[2] / 8.0]);
            }
            out
        };
        let gas_means = means(7, WorldKind::GasGiant, 10.0);
        let ice_means = means(7, WorldKind::IceGiant, 16.0);
        assert!(spread(&gas_means) > 0.15, "gas bands flat");
        assert!(
            spread(&ice_means) < spread(&gas_means),
            "ice bands bolder than gas"
        );
        // Hot Jupiters dark, glowing on the night side.
        let day = band_colour(7, WorldKind::HotJupiter, 0.2, 0.7, 72.0, Some(1.0));
        let night = band_colour(7, WorldKind::HotJupiter, 0.2, 0.7, 72.0, Some(-1.0));
        let mean = |c: [f64; 3]| (c[0] + c[1] + c[2]) / 3.0;
        assert!(mean(day) < 0.4, "hot day side bright: {day:?}");
        assert!(
            night[0] > day[0] + 0.15,
            "no night glow: {night:?} vs {day:?}"
        );
        // Storms break zonal symmetry on gas giants.
        let mut varied = false;
        let first = band_colour(7, WorldKind::GasGiant, 0.3, 0.0, 10.0, None);
        for i in 1..24 {
            let lon = f64::from(i) * std::f64::consts::TAU / 24.0;
            let colour = band_colour(7, WorldKind::GasGiant, 0.3, lon, 10.0, None);
            if (colour[0] - first[0]).abs() + (colour[1] - first[1]).abs() > 1e-6 {
                varied = true;
                break;
            }
        }
        assert!(varied, "longitude does nothing");
        // Rocky kinds flat grey; degenerate angles finite.
        assert_eq!(
            band_colour(7, WorldKind::EarthLike, 0.2, 0.7, 24.0, None),
            FLAT_GREY
        );
        let nan = band_colour(7, WorldKind::GasGiant, f64::NAN, 0.7, 10.0, None);
        assert!(nan.iter().all(|c| c.is_finite()));
    }

    #[test]
    fn rings_thicken_edge_on() {
        // T9: dense middles, narrow gaps, clear off-ring; opacity grows
        // toward edge-on views.
        let (inner, outer) = ring_edges(7);
        assert!((1.1..1.6).contains(&inner), "inner {inner}");
        assert!(
            (0.5..1.4).contains(&(outer - inner)),
            "span {inner}..{outer}"
        );
        assert_eq!(ring_edges(7), ring_edges(7));
        // Find a ringed young gas giant in range.
        let seed = (0..500u64)
            .find(|seed| has_rings(*seed, WorldKind::GasGiant, 1.0))
            .expect("no rings in range");
        assert!(has_rings(seed, WorldKind::GasGiant, 1.0));
        assert_eq!(
            has_rings(seed, WorldKind::GasGiant, 1.0),
            has_rings(seed, WorldKind::GasGiant, 1.0)
        );
        let mut densest = 0.0f64;
        let mut thinnest = f64::INFINITY;
        for i in 0..=500 {
            let tau = ring_tau(seed, f64::from(i) / 500.0);
            densest = densest.max(tau);
            thinnest = thinnest.min(tau);
        }
        assert!(densest > 0.5, "no dense band: {densest}");
        assert!(thinnest < 0.1, "no gaps: {thinnest}");
        let face = ring_alpha(seed, 0.55, 1.0);
        let edge = ring_alpha(seed, 0.55, 0.05);
        assert!(edge > face && face > 0.0, "edge {edge} vs face {face}");
        assert_eq!(ring_alpha(seed, -0.1, 1.0), 0.0);
        assert_eq!(ring_alpha(seed, 1.1, 0.2), 0.0);
        assert_eq!(
            ring_alpha(seed, 0.55, f64::NAN),
            ring_alpha(seed, 0.55, 1.0)
        );
        // Rarity follows kind and youth: gas over dwarf, young over old.
        let mut young_gas = 0u32;
        let mut old_gas = 0u32;
        let mut young_dwarf = 0u32;
        for seed in 0..2_000u64 {
            young_gas += u32::from(has_rings(seed, WorldKind::GasGiant, 1.0));
            old_gas += u32::from(has_rings(seed, WorldKind::GasGiant, 8.0));
            young_dwarf += u32::from(has_rings(seed, WorldKind::Dwarf, 1.0));
        }
        assert!(
            (500..1_300).contains(&young_gas),
            "young gas ring share off: {young_gas}"
        );
        assert!(
            young_gas > old_gas,
            "age does nothing: {young_gas} vs {old_gas}"
        );
        assert!(young_gas > young_dwarf, "kind does nothing");
        assert!(young_dwarf < 150, "dwarfs ringed: {young_dwarf}");
    }
}
