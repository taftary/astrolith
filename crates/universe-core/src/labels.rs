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
        11 => "L11 - Regions",
        12 => "L12 - Cities and landscapes",
        13 => "L13 - Buildings",
        14 => "L14 - Room",
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
        3 => "Virgo Cluster, 15 Mly across",
        4 => "Milky Way, 100 kly across",
        5 => "molecular-cloud complex, 100 pc",
        6 => "Alpha Centauri, 4.37 ly away",
        7 => "Oort cloud edge, 100,000 AU",
        8 => "heliopause, 120 AU",
        9 => "Sun, 1.39e9 m across (IAU)",
        10 => "Earth, 1.28e7 m across",
        11 => "500 km range or basin, 5.01e5 m across",
        12 => "city, 1.0e4 m across",
        13 => "building, 3.16e1 m across",
        14 => "room, 5.01e0 m across",
        _ => "beyond MVP",
    }
}

/// Readout line naming the next level you can dive into (#543).
///
/// `Some("next: L11 - Regions")` from the planet down to the plot level
/// (L10-L13); `None` above the planet and in the room, which never opens.
#[must_use]
pub fn next_level_line(level: Level) -> Option<String> {
    if !(10..=13).contains(&level.get()) {
        return None;
    }
    level
        .deeper()
        .map(|next| format!("next: {}", level_label(next)))
}

/// Size note after an enterable place's label at L11-L13 (#543): the number
/// of surface tiles its cluster spans, e.g. `"14 tiles"`.
#[must_use]
pub fn tile_note(tiles: usize) -> String {
    if tiles == 1 {
        "1 tile".to_string()
    } else {
        format!("{tiles} tiles")
    }
}

/// Ground ruler for the region, city, and plot levels (#543).
///
/// A round real length a little under the cell size, `10^floor(e - 0.5)`
/// metres for the level exponent `e`: 100 km at L11, 1 km at L12, 10 m at
/// L13. Returns the length in cell units (one cell spans `10^e` m) and its
/// text; `None` outside L11-L13.
#[must_use]
pub fn ground_ruler(level: Level) -> Option<(f64, &'static str)> {
    let text = match level.get() {
        11 => "100 km",
        12 => "1 km",
        13 => "10 m",
        _ => return None,
    };
    let exponent = scale_exponent(level);
    let metres = 10f64.powf((exponent - 0.5).floor());
    Some((metres / 10f64.powf(exponent), text))
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
        assert_eq!(tile_note(14), "14 tiles");
        assert_eq!(tile_note(1), "1 tile");
    }

    #[test]
    fn ground_ruler_is_a_round_length_inside_the_cell() {
        let want = [
            (11u8, 1.0e5, "100 km"),
            (12, 1.0e3, "1 km"),
            (13, 10.0, "10 m"),
        ];
        for (n, metres, text) in want {
            let level = Level::new(n).expect("tail level");
            let (length, label) = ground_ruler(level).expect("tail levels carry a ruler");
            let expected = metres / 10f64.powf(scale_exponent(level));
            assert!((length - expected).abs() < 1e-12, "L{n} ruler {length}");
            assert!(
                (0.05..0.5).contains(&length),
                "L{n} ruler leaves the view: {length}"
            );
            assert_eq!(label, text);
        }
        for n in [1u8, 10, 14] {
            assert!(ground_ruler(Level::new(n).expect("level")).is_none());
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
}
