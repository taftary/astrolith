//! Level labels, scale readouts, and window titles.
//!
//! Moved from `universe-render` in M3 (#85). Pure text derived from the
//! frozen ladder; no window needed.

use crate::coords::Level;
use crate::nest::scale_exponent;
use crate::shell::shell_exponent;

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
        11 => "L11 - Regions",
        12 => "L12 - Cities and landscapes",
        13 => "L13 - Buildings",
        14 => "L14 - Room",
        _ => "Beyond MVP",
    }
}

/// Display order label for `level` (e.g. `"10^21 m"`), rounded from the anchor.
///
/// The planet shells L11-L14 read their Earth enter height (#632).
#[must_use]
pub fn scale_label(level: Level) -> String {
    let shown = shell_exponent(level).unwrap_or_else(|| scale_exponent(level));
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: ladder exponents are small whole numbers"
    )]
    let exponent = shown.round() as i32;
    format!("10^{exponent} m")
}

/// Anchor object behind the level's exponent, for `--verify` output; the
/// planet shells name their enter height (#632, G1).
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
        11 => "below 1,000 km above the ground on Earth",
        12 => "below 100 km above the ground on Earth",
        13 => "below 1 km above the ground on Earth",
        14 => "below 10 m above the ground, on every planet",
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

/// Readout line naming the next level you can dive into (#543).
///
/// `Some("next: L11 - Regions")` from the planet down to the buildings
/// shell (L10-L13, #632); `None` above the planet and in the room shell,
/// the deepest.
#[must_use]
pub fn next_level_line(level: Level) -> Option<String> {
    if !(10..=13).contains(&level.get()) {
        return None;
    }
    level
        .deeper()
        .map(|next| format!("next: {}", level_label(next)))
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

    #[test]
    fn next_line_names_the_next_level_from_planet_to_plot() {
        assert_eq!(
            next_level_line(Level::new(10).expect("L10")).as_deref(),
            Some("next: L11 - Regions")
        );
        assert_eq!(
            next_level_line(Level::new(13).expect("L13")).as_deref(),
            Some("next: L14 - Room")
        );
        for n in [1u8, 9, 14] {
            assert!(next_level_line(Level::new(n).expect("level")).is_none());
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
    fn the_tail_readout_matches_the_shell_heights() {
        let readout: Vec<String> = (11..=14).filter_map(Level::new).map(scale_label).collect();
        assert_eq!(readout, vec!["10^6 m", "10^5 m", "10^3 m", "10^1 m"]);
        for n in 10..=13 {
            let level = Level::new(n).expect("ladder level");
            assert!(next_level_line(level).is_some_and(|line| line.starts_with("next: ")));
        }
        assert_eq!(next_level_line(Level::MAX), None);
        assert_eq!(scale_label(Level::new(10).expect("planet")), "10^7 m");
    }
}
