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
        3 => "L3 - Superclusters",
        4 => "L4 - Galaxy clusters and groups",
        5 => "L5 - Galaxies",
        6 => "L6 - Galactic structures",
        7 => "L7 - Stellar neighborhood",
        8 => "L8 - Outer solar system",
        9 => "L9 - Planetary system",
        10 => "L10 - Stars",
        11 => "L11 - Planets and moons",
        _ => "Beyond MVP",
    }
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
        3 => "Laniakea, 520 Mly across",
        4 => "Virgo Cluster, 15 Mly across",
        5 => "Milky Way, 100 kly across",
        6 => "molecular-cloud complex, 100 pc",
        7 => "Alpha Centauri, 4.37 ly away",
        8 => "Oort cloud edge, 100,000 AU",
        9 => "heliopause, 120 AU",
        10 => "Sun, 1.39e9 m across (IAU)",
        11 => "Earth, 1.28e7 m across",
        _ => "beyond MVP",
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
    use crate::nav::{MAX_NAV_LEVEL, MIN_NAV_LEVEL};

    #[test]
    fn titles_carry_scale_readout() {
        for n in MIN_NAV_LEVEL..=MAX_NAV_LEVEL {
            let level = Level::new(n).expect("level");
            assert!(window_title_for_level(level).contains(&scale_label(level)));
        }
        assert_eq!(scale_label(Level::MIN), "10^27 m");
    }
}
