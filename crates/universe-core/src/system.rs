//! Stellar and planetary sampling for L5-L8 (#155).
//!
//! The dive needs real densities, types, companions, and planets where the
//! old scatter put uniform dots: O-M star types from the observed mix with
//! conditioned Salpeter masses, companion rolls from the observed
//! fractions, planet radii with the Earth-radii valley, Solar-template
//! spacing with Kepler periods, log-uniform cloud masses, and the two home
//! fixtures (Alpha Centauri triple, Solar eight). Every sampler draws only
//! from its own stream, so connecting this module never moves an existing
//! position, size, or portal mapping: the data rides along, navigation
//! never shifts.

use crate::r#gen::{PlanetInfo, StarInfo, StarKind};
use crate::seed::Rng;

/// Lowest stellar mass sampled, in solar masses (hydrogen-burning limit).
///
/// Shared with the galaxy star layouts in [`crate::astro`].
pub const MIN_STAR_MASS: f64 = 0.08;

/// Highest stellar mass sampled, in solar masses (Salpeter-tail truncation).
///
/// Shared with the galaxy star layouts in [`crate::astro`].
pub const MAX_STAR_MASS: f64 = 120.0;

/// Domain-separation tag folded into per-star sampling streams.
///
/// Star data draws from `hash_triple(cell_seed, STAR_STREAM_TAG, index)`,
/// never from the position/size stream, so existing markers never move.
pub const STAR_STREAM_TAG: u64 = 0x57A2_155A_57A2_155A;

/// Domain-separation tag folded into per-planet sampling streams.
///
/// Companion and planet draws use their own tags for the same reason.
pub const PLANET_STREAM_TAG: u64 = 0x9E37_155A_51A7_079B;

/// Domain-separation tag folded into per-cloud sampling streams.
pub const CLOUD_STREAM_TAG: u64 = 0xC10D_155A_C10D_155A;

/// Domain-separation tag folded into Oort-shell layout streams.
pub const OORT_STREAM_TAG: u64 = 0x0027_155A_0027_155A;

/// Observed main-sequence mix as (class, share) pairs.
///
/// M 76.5 percent, K 12.1, G 7.6, F 3, A 0.6, B 0.13, O 0.00003 percent:
/// three in four stars are M dwarfs. Shares normalize to one, so the table
/// survives a later recalibration without touching the sampler.
pub const STAR_MIX: [(StarKind, f64); 7] = [
    (StarKind::M, 0.765),
    (StarKind::K, 0.121),
    (StarKind::G, 0.076),
    (StarKind::F, 0.030),
    (StarKind::A, 0.006),
    (StarKind::B, 0.0013),
    (StarKind::O, 0.00003),
];

/// Main-sequence mass band per class, in solar masses (lower, upper).
///
/// Standard bands: O from 16 up, B 2.1-16, A 1.4-2.1, F 1.04-1.4,
/// G 0.8-1.04, K 0.45-0.8, M 0.08-0.45. Salpeter draws condition on these,
/// so the mix matches the observed table exactly while masses keep the
/// Salpeter shape inside each band.
pub const MASS_BANDS: [(StarKind, f64, f64); 7] = [
    (StarKind::O, 16.0, MAX_STAR_MASS),
    (StarKind::B, 2.1, 16.0),
    (StarKind::A, 1.4, 2.1),
    (StarKind::F, 1.04, 1.4),
    (StarKind::G, 0.8, 1.04),
    (StarKind::K, 0.45, 0.8),
    (StarKind::M, MIN_STAR_MASS, 0.45),
];

/// Companion rate per class (fraction of systems with a companion).
///
/// Anchored on the observed three points (O near 100 percent, solar types
/// 54 percent, M dwarfs 27 percent) with monotone interpolation between;
/// triples roll a second companion at 30 percent of the first rate, so
/// systems hold 1 to 3 stars with triples rarer than binaries.
pub const COMPANION_RATES: [(StarKind, f64); 7] = [
    (StarKind::O, 1.0),
    (StarKind::B, 1.0),
    (StarKind::A, 0.8),
    (StarKind::F, 0.54),
    (StarKind::G, 0.54),
    (StarKind::K, 0.35),
    (StarKind::M, 0.27),
];

/// Solar-template span ends in AU (inner Mercury edge, outer Neptune edge).
///
/// Named ends keep the interpolation free of indexing; the catalog arrays
/// below stay the single source for per-planet values.
pub const SOLAR_AXIS_LO_AU: f64 = 0.39;

/// Outer end of the Solar-template span in AU (Neptune).
pub const SOLAR_AXIS_HI_AU: f64 = 30.1;

/// Solar-template orbit axes in AU (Mercury to Neptune, catalog order).
///
/// Home systems (the Solar fixture) use these exactly; procedural systems
/// interpolate across the same span for their own planet count.
pub const SOLAR_AXES_AU: [f64; 8] = [
    SOLAR_AXIS_LO_AU,
    0.72,
    1.0,
    1.52,
    5.20,
    9.58,
    19.2,
    SOLAR_AXIS_HI_AU,
];

/// Solar-template planet radii in Earth radii (catalog order).
pub const SOLAR_RADII_EARTH: [f64; 8] = [0.383, 0.949, 1.0, 0.532, 11.21, 9.45, 4.01, 3.88];

/// Alpha Centauri triple as (class, mass) pairs: A, B, Proxima.
///
/// G2V 1.1, K1V 0.9, M5.5V 0.12 solar masses at 4.37/4.37/4.24 light-years.
/// The L6 home portal carries exactly this triple; positions stay sampled
/// so rank order and the downstream journey never move.
pub const ALPHA_CEN_TRIPLE: [(StarKind, f64); 3] =
    [(StarKind::G, 1.1), (StarKind::K, 0.9), (StarKind::M, 0.12)];

/// Days per Earth year (period scale for Kepler's law).
pub const DAYS_PER_YEAR: f64 = 365.25;

/// Draws a truncated Salpeter mass in `[MIN_STAR_MASS, MAX_STAR_MASS]`.
///
/// `dN/dM ~ M^-2.35` via the inverse-CDF transform; the single uniform draw
/// keeps the stream aligned with the galaxy layouts that share it.
#[must_use]
pub fn salpeter_mass(rng: &mut Rng) -> f64 {
    let remaining = 1.0 - rng.next_f64();
    (MIN_STAR_MASS / remaining.powf(1.0 / 1.35)).min(MAX_STAR_MASS)
}

/// Samples a spectral class from the observed mix.
///
/// One uniform draw against the normalized cumulative table; a fresh stream
/// replays the same class every time.
#[must_use]
pub fn sample_star_kind(rng: &mut Rng) -> StarKind {
    let total: f64 = STAR_MIX.iter().map(|(_, share)| share).sum();
    let mut roll = rng.next_f64() * total;
    for (kind, share) in STAR_MIX {
        roll -= share;
        if roll <= 0.0 {
            return kind;
        }
    }
    StarKind::M
}

/// Samples a mass inside the class band, Salpeter-shaped.
///
/// Rejection-samples up to sixteen Salpeter draws for one inside the band,
/// falling back to the band midpoint; every path is stream-determined, so
/// replays agree bit for bit.
#[must_use]
pub fn star_mass(rng: &mut Rng, kind: StarKind) -> f64 {
    let (_, low, high) = MASS_BANDS
        .iter()
        .find(|(band, _, _)| *band == kind)
        .copied()
        .unwrap_or((StarKind::M, MIN_STAR_MASS, 0.45));
    for _ in 0..16 {
        let mass = salpeter_mass(rng);
        if (low..=high).contains(&mass) {
            return mass;
        }
    }
    (low + high) / 2.0
}

/// Samples one star (class plus banded mass) from `rng`.
#[must_use]
pub fn sample_star(rng: &mut Rng) -> StarInfo {
    let kind = sample_star_kind(rng);
    StarInfo {
        kind,
        mass_solar: star_mass(rng, kind),
    }
}

/// Companion rate for the class (fraction of systems with a companion).
#[must_use]
pub fn companion_rate(kind: StarKind) -> f64 {
    COMPANION_RATES
        .iter()
        .find(|(band, _)| *band == kind)
        .map(|(_, rate)| *rate)
        .unwrap_or(0.27)
}

/// Rolls the companion count (0 to 2) for a star of the class.
///
/// The first companion lands at the class rate, a second at 30 percent of
/// it, so systems hold 1 to 3 stars with triples rarer than binaries.
#[must_use]
pub fn roll_companions(rng: &mut Rng, kind: StarKind) -> u32 {
    let rate = companion_rate(kind);
    if rng.next_f64() < rate {
        if rng.next_f64() < rate * 0.3 { 2 } else { 1 }
    } else {
        0
    }
}

/// Samples a planet radius in Earth radii with the observed valley.
///
/// Rocky worlds cluster near 1.3 and sub-Neptunes near 2.4 Earth radii;
/// the 1.5-2 valley between them stays empty by construction (rocky draws
/// clamp below 1.5, enveloped draws clamp above 2.0). Giants arrive only
/// through the Solar fixture.
#[must_use]
pub fn sample_planet_radius(rng: &mut Rng) -> f64 {
    let bell = rng.next_f64() + rng.next_f64() + rng.next_f64() - 1.5;
    if rng.next_f64() < 0.5 {
        (1.3 * 2.0f64.powf(bell * 0.3)).clamp(0.5, 1.5 - 1e-9)
    } else {
        (2.4 * 2.0f64.powf(bell * 0.25)).clamp(2.0 + 1e-9, 4.0)
    }
}

/// Orbital period in days from Kepler's law for the host mass.
///
/// `P = 365.25 * sqrt(a^3 / M)` with `a` in AU and `M` in solar masses, so
/// period-squared stays proportional to distance-cubed by construction.
/// Degenerate inputs clamp to the hydrogen-burning limit and a dust-grain
/// orbit instead of producing `NaN`.
#[must_use]
pub fn kepler_period_days(orbit_au: f64, star_mass_solar: f64) -> f64 {
    let mass = star_mass_solar.max(MIN_STAR_MASS);
    let axis = orbit_au.max(1e-6);
    DAYS_PER_YEAR * ((axis * axis * axis) / mass).sqrt()
}

/// Orbit axis in AU for slot `index` of `count` planets.
///
/// Log-interpolated across the Solar-template span (0.39 to 30.1 AU), so a
/// 3-planet system reads as an evenly spaced excerpt of the same template;
/// a lone planet sits at 1 AU. The home eight use the catalog axes instead.
#[must_use]
pub fn template_axis(index: usize, count: usize) -> f64 {
    if count <= 1 {
        return 1.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: slot interpolation over a dozen planets, exactness irrelevant"
    )]
    let span = (SOLAR_AXIS_HI_AU / SOLAR_AXIS_LO_AU).powf(index as f64 / (count - 1) as f64);
    SOLAR_AXIS_LO_AU * span
}

/// Samples a planet (valley radius, template orbit, Kepler period).
///
/// Non-home systems draw every field; the home eight take catalog radii
/// and axes with periods from the same Kepler law.
#[must_use]
pub fn sample_planet(
    rng: &mut Rng,
    index: usize,
    count: usize,
    star_mass_solar: f64,
) -> PlanetInfo {
    let orbit_au = template_axis(index, count);
    PlanetInfo {
        radius_earth: sample_planet_radius(rng),
        orbit_au,
        period_days: kepler_period_days(orbit_au, star_mass_solar),
    }
}

/// Samples a cloud mass in solar masses (10 to 10 million, log-uniform).
///
/// The giant-molecular-cloud spectrum strung along the arms; one uniform
/// draw across six decades.
#[must_use]
pub fn sample_cloud_mass(rng: &mut Rng) -> f64 {
    10.0 * 1_000_000.0f64.powf(rng.next_f64())
}

/// Host star of a planetary system from the shared system seed.
///
/// L8 and L9 derive the same host from the seed both cells agree on, so the
/// close-up and the system view share one mass for Kepler's law. Home
/// systems read the Sun instead of a draw.
#[must_use]
pub fn system_star(system_seed: u64, home: bool) -> StarInfo {
    if home {
        return StarInfo {
            kind: StarKind::G,
            mass_solar: 1.0,
        };
    }
    let mut rng = Rng::new(crate::seed::hash_triple(system_seed, STAR_STREAM_TAG, 0));
    sample_star(&mut rng)
}

/// Planet `slot` of `count` from the shared system seed.
///
/// Slot streams are keyed by slot, so L8 and L9 attach identical data to
/// their own markers: catalog radii and axes for the home eight, the valley
/// distribution over template spacing elsewhere, periods from Kepler's law
/// for the host mass in every case.
#[must_use]
pub fn system_planet(
    system_seed: u64,
    slot: usize,
    count: usize,
    star_mass_solar: f64,
    home: bool,
) -> PlanetInfo {
    if home
        && count == SOLAR_AXES_AU.len()
        && let Some((orbit_au, radius_earth)) = SOLAR_AXES_AU
            .get(slot)
            .copied()
            .zip(SOLAR_RADII_EARTH.get(slot).copied())
    {
        return PlanetInfo {
            radius_earth,
            orbit_au,
            period_days: kepler_period_days(orbit_au, star_mass_solar),
        };
    }
    let mut rng = Rng::new(crate::seed::hash_triple(
        system_seed,
        PLANET_STREAM_TAG,
        slot as u64,
    ));
    sample_planet(&mut rng, slot, count, star_mass_solar)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::hash_triple;

    /// Samples one value from a fresh per-purpose stream (keeps tests short).
    fn stream(seed: u64, tag: u64, index: u64) -> Rng {
        Rng::new(hash_triple(seed, tag, index))
    }

    #[test]
    fn star_mix_is_m_dwarf_dominated_and_deterministic() {
        let mut counts = [0u32; 7];
        for seed in 0..20_000u64 {
            let mut rng = stream(seed, STAR_STREAM_TAG, 0);
            let kind = sample_star_kind(&mut rng);
            counts[kind as usize] += 1;
            let replay = sample_star_kind(&mut stream(seed, STAR_STREAM_TAG, 0));
            assert_eq!(kind, replay, "seed {seed} replayed a new class");
        }
        let total = f64::from(counts.iter().sum::<u32>());
        let share = |kind: StarKind| f64::from(counts[kind as usize]) / total;
        assert!(
            (0.72..0.81).contains(&share(StarKind::M)),
            "M share out of band: {}",
            share(StarKind::M)
        );
        assert!(
            (0.09..0.15).contains(&share(StarKind::K)),
            "K share out of band: {}",
            share(StarKind::K)
        );
        assert!(
            (0.05..0.10).contains(&share(StarKind::G)),
            "G share out of band: {}",
            share(StarKind::G)
        );
        assert!(
            share(StarKind::O) < 0.002,
            "O stars too common: {}",
            share(StarKind::O)
        );
        assert!(
            share(StarKind::M) > share(StarKind::K)
                && share(StarKind::K) > share(StarKind::G)
                && share(StarKind::G) > share(StarKind::F),
            "mix order broke: M={} K={} G={} F={}",
            share(StarKind::M),
            share(StarKind::K),
            share(StarKind::G),
            share(StarKind::F)
        );
    }

    #[test]
    fn star_masses_stay_inside_their_class_bands() {
        for kind in [
            StarKind::O,
            StarKind::B,
            StarKind::A,
            StarKind::F,
            StarKind::G,
            StarKind::K,
            StarKind::M,
        ] {
            let (_, low, high) = MASS_BANDS
                .iter()
                .find(|(band, _, _)| *band == kind)
                .copied()
                .expect("every class has a band");
            for seed in 0..500u64 {
                let mass = star_mass(&mut stream(seed, STAR_STREAM_TAG, 1), kind);
                assert!(
                    (low..=high).contains(&mass),
                    "{kind:?} mass out of band: {mass}"
                );
            }
        }
    }

    #[test]
    fn planet_radii_keep_the_valley_empty() {
        let mut rocky = 0u32;
        let mut enveloped = 0u32;
        for seed in 0..2_000u64 {
            let radius = sample_planet_radius(&mut stream(seed, PLANET_STREAM_TAG, 0));
            assert!(
                !(1.5..=2.0).contains(&radius),
                "valley filled at seed {seed}: {radius}"
            );
            if radius < 1.5 {
                rocky += 1;
            } else {
                enveloped += 1;
            }
        }
        assert!(rocky > 500, "rocky peak missing: {rocky}");
        assert!(enveloped > 500, "sub-Neptune peak missing: {enveloped}");
    }

    #[test]
    fn kepler_periods_obey_period_squared_over_distance_cubed() {
        assert_eq!(kepler_period_days(1.0, 1.0), DAYS_PER_YEAR);
        let ratio = |axis: f64| {
            let period = kepler_period_days(axis, 1.0);
            period * period / (axis * axis * axis)
        };
        let baseline = ratio(1.0);
        for axis in [0.39, 0.72, 1.52, 5.20, 9.58, 19.2, 30.1] {
            let drift = (ratio(axis) / baseline - 1.0).abs();
            assert!(drift < 1e-12, "Kepler drift at {axis} AU: {drift}");
        }
        // Catalog spot checks: Mercury ~88 days, Neptune ~60,190 days.
        let mercury = kepler_period_days(SOLAR_AXES_AU[0], 1.0);
        let neptune = kepler_period_days(SOLAR_AXES_AU[7], 1.0);
        assert!(
            (80.0..100.0).contains(&mercury),
            "Mercury period off: {mercury}"
        );
        assert!(
            (59_000.0..61_500.0).contains(&neptune),
            "Neptune period off: {neptune}"
        );
        // Degenerate inputs stay finite.
        assert!(kepler_period_days(0.0, 0.0).is_finite());
        assert!(kepler_period_days(f64::NAN, f64::NAN).is_finite());
    }

    #[test]
    fn template_axes_span_the_solar_range_in_log_order() {
        assert_eq!(template_axis(0, 1), 1.0, "lone planet sits at 1 AU");
        let axes: Vec<f64> = (0..8).map(|i| template_axis(i, 8)).collect();
        assert!(
            (axes[0] - 0.39).abs() < 1e-12 && (axes[7] - 30.1).abs() < 1e-9,
            "template span broke: {axes:?}"
        );
        for pair in axes.windows(2) {
            assert!(pair[0] < pair[1], "template order broke: {axes:?}");
        }
    }

    #[test]
    fn companion_rates_fall_from_o_to_m() {
        let order = [
            StarKind::O,
            StarKind::B,
            StarKind::A,
            StarKind::F,
            StarKind::G,
            StarKind::K,
            StarKind::M,
        ];
        let mut previous = 2.0;
        for kind in order {
            let rate = companion_rate(kind);
            assert!(
                rate <= previous,
                "{kind:?} rate {rate} breaks the fall from {previous}"
            );
            previous = rate;
        }
        assert_eq!(companion_rate(StarKind::O), 1.0);
        assert_eq!(companion_rate(StarKind::G), 0.54);
        assert_eq!(companion_rate(StarKind::M), 0.27);
        // Rolls stay in range and replay.
        for seed in 0..500u64 {
            let rolled = roll_companions(&mut stream(seed, PLANET_STREAM_TAG, 7), StarKind::G);
            assert!(rolled <= 2, "too many companions: {rolled}");
            assert_eq!(
                rolled,
                roll_companions(&mut stream(seed, PLANET_STREAM_TAG, 7), StarKind::G)
            );
        }
    }

    #[test]
    fn cloud_masses_span_ten_to_ten_million_solar() {
        let mut smallest = f64::INFINITY;
        let mut largest: f64 = 0.0;
        for seed in 0..2_000u64 {
            let mass = sample_cloud_mass(&mut stream(seed, CLOUD_STREAM_TAG, 0));
            assert!(
                (10.0..=10_000_000.0).contains(&mass),
                "cloud mass out of spectrum: {mass}"
            );
            smallest = smallest.min(mass);
            largest = largest.max(mass);
        }
        assert!(smallest < 100.0, "low end missing: {smallest}");
        assert!(largest > 1_000_000.0, "high end missing: {largest}");
    }
}
