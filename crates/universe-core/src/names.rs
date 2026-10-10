//! Seeded marker names and label text (#446).
//!
//! Pure naming behind Spec v1 AC8: the home chain keeps its real names, tail
//! surface portals read `"<kind> <n>"`, and every other portal gets a short
//! made-up name that is identical every run for the same seed. Names are
//! computed when a label first shows and cached per open cell, never per
//! frame (`E-HOT-NOALLOC`).

use crate::coords::Level;
use crate::r#gen::Point;
use crate::nest::{autopilot_path, path_seed};
use crate::seed::{Rng, hash_triple};

/// Domain-separation tag for the name stream (never the position stream).
const NAME_STREAM_TAG: u64 = 0x006E_616D_655F_3134;

/// Syllables composed into cosmic names (lowercase; capitalised on build).
const SYLLABLES: [&str; 24] = [
    "ve", "lo", "ra", "ke", "sh", "ta", "run", "mi", "na", "dor", "al", "bel", "tra", "qui", "xel",
    "mor", "thi", "ol", "ma", "ri", "gel", "sol", "tau", "cen",
];

/// Display kind of one marker for its label (Spec v1 AC8).
///
/// Sampler data wins when present (galaxy, star, planet, moon, cloud);
/// otherwise the level names the rung. Pure.
#[must_use]
pub fn marker_kind_name(level: Level, point: &Point) -> &'static str {
    if point.moon.is_some() {
        return "moon";
    }
    if point.planet.is_some() {
        return "planet";
    }
    if point.star.is_some() {
        return "star";
    }
    if point.galaxy.is_some() {
        return "galaxy";
    }
    if point.cloud.is_some() {
        return "cloud";
    }
    match level.get() {
        1 => "octant",
        2 => "web",
        3 => "cluster",
        4 => "galaxy",
        5 => "cloud",
        6 => "star",
        7 => "system",
        8 => "planet",
        9 => "star",
        10 => "region",
        11 => "region",
        12 => "city",
        13 => "building",
        14 => "room",
        _ => "marker",
    }
}

/// Home-chain name for the journey pick in its cell, if any.
///
/// Compares `marker` against the seeded journey at this depth: L4 Milky Way,
/// L6 Alpha Centauri, L7-L8 Sun, L9 Earth. Returns `None` off the journey.
#[must_use]
pub fn home_name(root: u64, chain: &[u32], marker: u32) -> Option<&'static str> {
    let journey = autopilot_path(root);
    if chain.len() >= journey.len() {
        return None;
    }
    let mut prefix_ok = true;
    for (depth, &held) in chain.iter().enumerate() {
        if journey.get(depth) != Some(&held) {
            prefix_ok = false;
            break;
        }
    }
    if !prefix_ok {
        return None;
    }
    if journey.get(chain.len()) != Some(&marker) {
        return None;
    }
    match chain.len() {
        // Chain length is the open level minus one: len 3 is the L4 cell.
        3 => Some("Milky Way"),
        5 => Some("Alpha Centauri"),
        6 | 7 => Some("Sun"),
        8 => Some("Earth"),
        _ => None,
    }
}

/// Builds one made-up name from `stream` (two to three syllables, sometimes two words).
fn composed_name(stream: &mut Rng) -> String {
    let first_words = 2 + usize::try_from(stream.next_u32() % 2).unwrap_or(0);
    let mut first = String::new();
    for _ in 0..first_words {
        let slot = stream.next_u32() % u32::try_from(SYLLABLES.len()).unwrap_or(1);
        if let Some(syllable) = SYLLABLES.get(usize::try_from(slot).unwrap_or(0)) {
            first.push_str(syllable);
        }
    }
    let mut first = capitalised(&first);
    if stream.next_u32().is_multiple_of(2) {
        let mut second = String::new();
        for _ in 0..2 {
            let slot = stream.next_u32() % u32::try_from(SYLLABLES.len()).unwrap_or(1);
            if let Some(syllable) = SYLLABLES.get(usize::try_from(slot).unwrap_or(0)) {
                second.push_str(syllable);
            }
        }
        first.push(' ');
        first.push_str(&capitalised(&second));
    }
    first
}

/// Capitalises the first character, leaving the rest as composed.
fn capitalised(word: &str) -> String {
    let mut chars = word.chars();
    let Some(head) = chars.next() else {
        return String::new();
    };
    let mut out = String::with_capacity(word.len());
    for upper in head.to_uppercase() {
        out.push(upper);
    }
    out.push_str(chars.as_str());
    out
}

/// Seeded name of one portal marker (Spec v1 AC8).
///
/// Home picks keep their real names; L11-L14 surface portals read
/// `"<kind> <n>"` with a human one-based index; every other portal gets a
/// composed syllable name. Deterministic in `(root, chain, marker)`.
#[must_use]
pub fn marker_name(root: u64, chain: &[u32], level: Level, marker: u32, point: &Point) -> String {
    if let Some(home) = home_name(root, chain, marker) {
        return home.to_string();
    }
    if level.get() >= 11 {
        let kind = marker_kind_name(level, point);
        return format!("{kind} {}", marker.saturating_add(1));
    }
    let seed = hash_triple(path_seed(root, chain), NAME_STREAM_TAG, u64::from(marker));
    let mut stream = Rng::new(seed);
    composed_name(&mut stream)
}

/// Label text for a portal: `"kind · name"`, or the kind alone when nameless.
#[must_use]
pub fn label_text(kind: &str, name: &str) -> String {
    if name.is_empty() {
        kind.to_string()
    } else {
        format!("{kind} · {name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r#gen::{MarkerKind, Point};
    use crate::nav::{DEMO_SEED, MarkerIndex, Universe};

    /// Level helper for tests.
    fn level(n: u8) -> Level {
        Level::new(n).expect("ladder level")
    }

    /// Bare portal point with no sampler data.
    fn bare() -> Point {
        Point::bare([0.0; 3], 0.01, MarkerKind::Portal)
    }

    #[test]
    fn same_seed_gives_the_same_name() {
        let chain: Vec<u32> = Vec::new();
        let first = marker_name(DEMO_SEED, &chain, level(4), 3, &bare());
        assert_eq!(first, marker_name(DEMO_SEED, &chain, level(4), 3, &bare()));
        assert_ne!(first, marker_name(43, &chain, level(4), 3, &bare()));
    }

    #[test]
    fn home_chain_keeps_its_real_names() {
        let journey = autopilot_path(DEMO_SEED);
        let l4 = marker_name(DEMO_SEED, &journey[..3], level(4), journey[3], &bare());
        assert_eq!(l4, "Milky Way");
        let l6 = marker_name(DEMO_SEED, &journey[..5], level(6), journey[5], &bare());
        assert_eq!(l6, "Alpha Centauri");
        let l8 = marker_name(DEMO_SEED, &journey[..7], level(8), journey[7], &bare());
        assert_eq!(l8, "Sun");
        let l9 = marker_name(DEMO_SEED, &journey[..8], level(9), journey[8], &bare());
        assert_eq!(l9, "Earth");
    }

    #[test]
    fn names_differ_across_siblings() {
        let chain: Vec<u32> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for marker in 0..8u32 {
            assert!(seen.insert(marker_name(DEMO_SEED, &chain, level(4), marker, &bare())));
        }
    }

    #[test]
    fn surface_names_number_the_kind() {
        assert_eq!(
            marker_name(DEMO_SEED, &[], level(12), 11, &bare()),
            "city 12"
        );
        assert_eq!(
            marker_name(DEMO_SEED, &[], level(11), 3, &bare()),
            "region 4"
        );
        let text = label_text("city", "city 12");
        assert_eq!(text, "city · city 12");
    }

    #[test]
    fn kind_names_follow_sampler_data_then_level() {
        let universe = Universe::new(DEMO_SEED);
        let l6 = crate::nest::generate_cell(DEMO_SEED, &[]);
        let _ = l6;
        let point = universe
            .open
            .points
            .iter()
            .find(|point| point.galaxy.is_some());
        if let Some(galaxy) = point {
            assert_eq!(marker_kind_name(level(4), galaxy), "galaxy");
        }
        assert_eq!(marker_kind_name(level(11), &bare()), "region");
        assert_eq!(label_text("star", ""), "star");
        assert_eq!(
            label_text("star", "Alpha Centauri"),
            "star · Alpha Centauri"
        );
    }

    #[test]
    fn journey_names_ride_the_open_cell() {
        let universe = Universe::new(DEMO_SEED);
        let first = universe.autopilot_target().expect("journey");
        let name = marker_name(
            DEMO_SEED,
            &[],
            universe.level(),
            first,
            universe
                .open
                .points
                .get(usize::try_from(first).unwrap_or(0))
                .map_or(&bare(), |point| point),
        );
        assert!(!name.is_empty());
        let _ = MarkerIndex(first);
    }
}
