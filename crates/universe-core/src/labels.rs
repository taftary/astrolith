//! Level labels, scale readouts, and window titles.
//!
//! Moved from `universe-render` in M3 (#85). Pure text derived from the
//! frozen ladder; no window needed.

use crate::coords::Level;
use crate::nest::scale_exponent;

/// Short display label for a level, from the frozen ladder.
#[must_use]
pub fn level_label(level: Level) -> &'static str {
    match level.get() {
        1 => "L1 - Observable universe",
        2 => "L2 - Cosmic web",
        3 => "L3 - Galaxy clusters and groups",
        4 => "L4 - Galaxies",
        5 => "L5 - Galactic structures",
        6 => "L6 - Stellar neighborhood",
        7 => "L7 - Outer solar system",
        8 => "L8 - Planetary system",
        9 => "L9 - Stars",
        10 => "L10 - Planets and moons",
        _ => "Beyond MVP",
    }
}

/// Lowest height above the ground, in metres, of each ground level (#892 T3,
/// Q4 B): L10 from 1,000 km, L11 from 100 km, L12 from 1 km, L13 from 10 m,
/// L14 below 10 m.
pub const GROUND_LEVEL_FLOORS_M: [(u8, f64); 4] = [
    (10, 1_000_000.0),
    (11, 100_000.0),
    (12, 1_000.0),
    (13, 10.0),
];

/// The level shown at `height_m` above the real ground (#892 T3, Q4 B):
/// `10..=14`. Names for heights only; nothing opens below L10. Non-finite
/// heights read as on the ground (L14). Pure.
#[must_use]
pub fn ground_level(height_m: f64) -> u8 {
    if !height_m.is_finite() {
        return 14;
    }
    GROUND_LEVEL_FLOORS_M
        .iter()
        .find(|(_, floor)| height_m >= *floor)
        .map_or(14, |(level, _)| *level)
}

/// Panel name of a ground level (#892 T3, Q5 A): L10 keeps its ladder
/// label; L11-L14 take the neutral height names. Pure.
#[must_use]
pub fn ground_label(level: u8) -> &'static str {
    match level {
        11 => "L11 - High above the planet",
        12 => "L12 - Above the land",
        13 => "L13 - Close to the ground",
        14 => "L14 - On the ground",
        _ => "L10 - Planets and moons",
    }
}

/// Panel height line for `height_m` above the ground (#892 T3, Q5 A):
/// metres below 1 km, kilometres with one decimal below 100 km, whole
/// kilometres above (e.g. `"height 240 m above ground"`,
/// `"height 4.2 km above ground"`). Negative or non-finite read as 0. Pure.
#[must_use]
pub fn height_line(height_m: f64) -> String {
    let h = if height_m.is_finite() {
        height_m.max(0.0)
    } else {
        0.0
    };
    if h < 1_000.0 {
        format!("height {h:.0} m above ground")
    } else if h < 100_000.0 {
        format!("height {:.1} km above ground", h / 1_000.0)
    } else {
        format!("height {:.0} km above ground", h / 1_000.0)
    }
}

/// Star radii within which the planetary system is named after its star
/// (#995 T10, Q2 A): "L9 - Stars" near the star, "L8 - Planetary system"
/// elsewhere, the way L11-L14 name heights. Ten radii is about 0.05 AU for
/// the Sun, well inside Mercury's orbit.
pub const STAR_CLOSE_UP_RADII: f64 = 10.0;

/// The level the planetary system's view is named by (#995 T10): the star
/// close-up (L9) within [`STAR_CLOSE_UP_RADII`] of the star, else `open`.
#[must_use]
pub fn system_level(open: Level, star_distance_m: f64, star_radius_m: f64) -> Level {
    if open.get() == 8 && star_distance_m < STAR_CLOSE_UP_RADII * star_radius_m {
        Level::new(9).unwrap_or(open)
    } else {
        open
    }
}

/// The nearest body line (#995 T10, Q4 A): `"nearest Earth 2.7 million km"`.
/// Metres below a kilometre, kilometres below a million, millions of
/// kilometres below 0.1 AU, then astronomical units. Pure.
#[must_use]
pub fn nearest_line(name: &str, distance_m: f64) -> String {
    let d = if distance_m.is_finite() {
        distance_m.max(0.0)
    } else {
        0.0
    };
    let au = 149_597_870_700.0;
    let shown = if d < 1_000.0 {
        format!("{d:.0} m")
    } else if d < 1.0e9 {
        format!("{:.0} km", d / 1_000.0)
    } else if d < 0.1 * au {
        format!("{:.1} million km", d / 1.0e9)
    } else {
        format!("{:.2} AU", d / au)
    };
    format!("nearest {name} {shown}")
}

/// Panel scale for `height_m` above the ground (#892 T3): the height's
/// power of ten, rounded (`240 m` -> `"10^2 m"`, `4.2 km` -> `"10^4 m"`),
/// never below `10^0 m`. Pure.
#[must_use]
pub fn ground_scale_label(height_m: f64) -> String {
    let h = if height_m.is_finite() {
        height_m.max(1.0)
    } else {
        1.0
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: a rounded decimal exponent of a planet height is small"
    )]
    let exponent = h.log10().round() as i32;
    format!("10^{exponent} m")
}

/// Display order label for `level` (e.g. `"10^21 m"`), rounded from the anchor.
#[must_use]
pub fn scale_label(level: Level) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: ladder exponents are small whole numbers"
    )]
    let exponent = scale_exponent(level).round() as i32;
    format!("10^{exponent} m")
}

/// Anchor object behind the level's exponent, for `--verify` output.
#[must_use]
pub fn scale_anchor(level: Level) -> &'static str {
    match level.get() {
        1 => "observable universe, 93 Gly across",
        2 => "Sloan Great Wall, 1.37 Gly",
        3 => "Virgo Cluster, 15 Mly across",
        4 => "Milky Way, 100 kly across",
        5 => "molecular-cloud complex, 100 pc",
        6 => "Alpha Centauri, 4.37 ly away",
        7 => "Oort cloud edge, 100,000 AU",
        8 => "heliopause, 120 AU",
        9 => "Sun, 1.39e9 m across (IAU)",
        10 => "Earth, 1.28e7 m across",
        _ => "beyond MVP",
    }
}

/// The three modes the mode reminder names (#557, #558).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeName {
    /// Normal mode: the body (key 1).
    Normal,
    /// Free mode: the floating camera (key 2).
    Free,
    /// Dive mode (key 3).
    Dive,
}

/// The on-screen mode reminder (#557): the current mode, then its keys.
#[must_use]
pub fn mode_line(mode: ModeName) -> &'static str {
    match mode {
        ModeName::Normal => "Normal - 1 normal - 2 free - 3 dive",
        ModeName::Free => "Free - 1 normal - 2 free - 3 dive",
        ModeName::Dive => "Dive - 1 normal - 2 free - 3 dive",
    }
}

/// Readout line for the strongest pull in normal mode (#559): its body and
/// strength, e.g. `"pull: Earth 9.8 m/s2"` (tiny pulls in scientific form,
/// ASCII only for the bundled font).
#[must_use]
pub fn pull_line(name: &str, strength: f64) -> String {
    if strength >= 0.1 {
        format!("pull: {name} {strength:.1} m/s2")
    } else {
        format!("pull: {name} {strength:.1e} m/s2")
    }
}

/// Window title for the open `level`: app name, ladder label, true scale.
#[must_use]
pub fn window_title_for_level(level: Level) -> String {
    format!(
        "Universe MVP - {} | {}",
        level_label(level),
        scale_label(level)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_levels_follow_the_height() {
        // #892 T3 (Q4 B, Q5 A): the bands, the names, the height line and
        // the scale from the height above the ground.
        assert_eq!(ground_level(2_000_000.0), 10);
        assert_eq!(ground_level(1_000_000.0), 10);
        assert_eq!(ground_level(500_000.0), 11);
        assert_eq!(ground_level(50_000.0), 12);
        assert_eq!(ground_level(500.0), 13);
        assert_eq!(ground_level(10.0), 13);
        assert_eq!(ground_level(5.0), 14);
        assert_eq!(ground_level(-3.0), 14);
        assert_eq!(ground_level(f64::NAN), 14);
        assert_eq!(ground_label(11), "L11 - High above the planet");
        assert_eq!(ground_label(12), "L12 - Above the land");
        assert_eq!(ground_label(13), "L13 - Close to the ground");
        assert_eq!(ground_label(14), "L14 - On the ground");
        assert_eq!(ground_label(10), "L10 - Planets and moons");
        assert_eq!(height_line(240.0), "height 240 m above ground");
        assert_eq!(height_line(4_200.0), "height 4.2 km above ground");
        assert_eq!(height_line(500_000.0), "height 500 km above ground");
        assert_eq!(height_line(-1.0), "height 0 m above ground");
        assert_eq!(ground_scale_label(240.0), "10^2 m");
        assert_eq!(ground_scale_label(4_200.0), "10^4 m");
        assert_eq!(ground_scale_label(0.2), "10^0 m");
        for text in [
            height_line(3.0),
            ground_scale_label(3.0),
            ground_label(13).to_string(),
        ] {
            assert!(text.is_ascii(), "the bundled font is ASCII-only: {text}");
        }
    }

    #[test]
    fn pull_line_names_the_body_and_its_strength() {
        assert_eq!(pull_line("Earth", 9.806), "pull: Earth 9.8 m/s2");
        assert_eq!(pull_line("star 3", 0.00012), "pull: star 3 1.2e-4 m/s2");
        assert!(pull_line("planet", 0.5).is_ascii());
    }

    #[test]
    fn mode_line_names_the_mode_then_its_keys() {
        assert!(mode_line(ModeName::Dive).starts_with("Dive"));
        assert!(mode_line(ModeName::Free).starts_with("Free"));
        assert!(mode_line(ModeName::Normal).starts_with("Normal"));
        for mode in [ModeName::Normal, ModeName::Free, ModeName::Dive] {
            let line = mode_line(mode);
            assert!(
                line.contains("1 normal") && line.contains("2 free") && line.contains("3 dive")
            );
            assert!(line.is_ascii(), "the bundled font is ASCII-only");
        }
    }

    use crate::nav::{MAX_NAV_LEVEL, MIN_NAV_LEVEL};

    #[test]
    fn titles_carry_scale_readout() {
        for n in MIN_NAV_LEVEL..=MAX_NAV_LEVEL {
            let level = Level::new(n).expect("level");
            assert!(window_title_for_level(level).contains(&scale_label(level)));
        }
        assert_eq!(scale_label(Level::MIN), "10^27 m");
    }

    #[test]
    fn the_system_is_named_by_the_distance_to_its_star() {
        // #995 T10 (Q2 A): L9 within ten star radii, L8 beyond; other
        // levels keep their own name.
        let l8 = Level::new(8).expect("L8");
        let r = 6.957e8;
        assert_eq!(system_level(l8, 9.9 * r, r).get(), 9);
        assert_eq!(system_level(l8, 10.1 * r, r).get(), 8);
        let l10 = Level::new(10).expect("L10");
        assert_eq!(system_level(l10, 0.0, r), l10);
    }

    #[test]
    fn the_nearest_body_line_reads_in_human_units() {
        // #995 T10 (Q4 A).
        assert_eq!(nearest_line("Earth", 250.0), "nearest Earth 250 m");
        assert_eq!(
            nearest_line("Earth", 920_000_000.0),
            "nearest Earth 920000 km"
        );
        assert_eq!(nearest_line("Earth", 2.7e9), "nearest Earth 2.7 million km");
        assert_eq!(
            nearest_line("Sun", 149_597_870_700.0),
            "nearest Sun 1.00 AU"
        );
    }
}
