//! Player guides text: About and Help pages (#473, T6).
//!
//! Pure text (`E-CORE-NO-BEVY`): the approved Spec v1 words, kept in the
//! game's code as `&'static str`. The scale-ladder page is built from
//! [`crate::labels`] (`level_label`, `scale_label`, `scale_anchor`), so it
//! cannot drift from the game. `universe-render` shows these pages; the
//! `guides/` research documents are sources only and never shown.

use crate::coords::Level;
use crate::labels::{level_label, scale_anchor, scale_label};
use crate::nav::{MAX_NAV_LEVEL, MIN_NAV_LEVEL};

/// About > What is Astrolith (approved Spec v1 text).
pub const WHAT: &str = "Astrolith is a dive through the scales of the universe, from the cosmic web down to a single room. Every bright point is a place you can enter. Pick one, dive in, and the world around you becomes the next, smaller level. The same universe is built the same way every time.";

/// Help > Getting started (approved Spec v1 text; #607 G1).
pub const GETTING_STARTED: &str = "Move the mouse over a bright point to highlight it; its name appears. Click to make it your target. Hold the Up arrow (or scroll) to dive toward it; hold Down to rise back out. Press Space to let the autopilot fly the whole journey from L1 down to your player, standing on a planet.";

/// Help > Navigation (approved Spec v1 text; #607 G2).
pub const NAVIGATION: &str = "Click: choose a target. Click again on the same spot: cycle between the places under the pointer. Up arrow or mouse wheel: dive in. Down arrow: rise out. Space: autopilot on or off. P: go back to your player. E / Shift+E: darker / brighter. Esc: open the menu. 1: normal mode. 2: free mode. 3: dive mode. On the planet and below, the readout at the bottom left also names the next level you can dive into. You can change any key in Settings > Controls > Keys. On a planet, your level follows your height above the ground: L11 below 1,000 km, L12 below 100 km, L13 below 1 km, L14 below 10 m (Earth heights; on other planets L11 to L13 scale with the planet's size, and L14 is always 10 m). Climb back up to return to the level above.";

/// Help > Normal mode (#558, approved Spec v2 text, G1; #607 G3).
pub const NORMAL_MODE: &str = "Press 1 to walk in your own body. Your body waits where you leave it. Press 1 near it to take it back, or from far away to fly there first. W A S D walk, the mouse looks around, Shift runs, Space jumps. Double-tap Space to take off and fly; while flying, Space goes up and C goes down; double-tap Space again to land. V switches between first and third person. Walk toward a place and it opens around you. Gravity comes from every body around you, as in real space: near a planet you fall and walk, far from everything you drift. A faint arrow and the readout show the strongest pull.";

/// Help > Free flight (approved Spec v1 text).
pub const FREE_FLIGHT: &str = "Press F or 2 to fly freely. W A S D move you, hold the right mouse button and drag to look around. Shift+1 to Shift+9 set your speed, from slowest to fastest. Press F again or 3 to go back to diving. In free mode you pass through everything, even the ground.";

/// Help > Saved views (approved Spec v1 text).
pub const SAVED_VIEWS: &str = "Ctrl+1 to Ctrl+8 save where you are and where you look. Alt+1 to Alt+8 bring you back there. Saved views are kept when you close the game; clear them in Settings > Views.";

/// Help > Touch (#561, approved Spec v2 text, G1).
pub const TOUCH: &str = "On a touch screen: tap a point to choose it, tap again to cycle through the places under your finger. Pinch apart to dive in, pinch together to rise out. The round button at the bottom right flies the autopilot. Tap the mode reminder at the top left to switch between normal, free and dive. In normal and free mode the stick moves you (push it to the edge to run), and dragging anywhere else looks around. In normal mode, Jump jumps (double-tap to fly, hold to climb), Down descends while flying, and Camera switches the view. Settings > Controls > Touch chooses when the controls show and which hand holds the stick.";

/// Help > Planets (#685, approved Spec v2 text, G1).
pub const PLANETS: &str = "Every planet and moon is made from its seed and the age of its star system. Its size, its air and its distance from the star decide its type: rocky, desert, ocean, ice, lava, or a giant made of gas. Mountains, rifts, volcanoes, craters, rivers, lakes and clouds follow from that type and from its age: old surfaces are covered in craters, young ones are smooth. Moons are made the same way. Choose Settings > Display > Scene > Planet look > Generated to see them.";

/// Guides page id (matches the panel list leaf ids).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuidePage {
    /// About > What is Astrolith.
    What,
    /// About > The scale ladder (built from [`crate::labels`]).
    Ladder,
    /// Help > Getting started.
    GettingStarted,
    /// Help > Navigation.
    Navigation,
    /// Help > Normal mode (#558).
    NormalMode,
    /// Help > Free flight.
    FreeFlight,
    /// Help > Saved views.
    SavedViews,
    /// Help > Touch (#561).
    Touch,
    /// Help > Planets (#685).
    Planets,
}

impl GuidePage {
    /// Every guides page, in panel-list order.
    pub const ALL: [GuidePage; 9] = [
        GuidePage::What,
        GuidePage::Ladder,
        GuidePage::GettingStarted,
        GuidePage::Navigation,
        GuidePage::NormalMode,
        GuidePage::FreeFlight,
        GuidePage::SavedViews,
        GuidePage::Touch,
        GuidePage::Planets,
    ];

    /// Panel list leaf id for this page.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            GuidePage::What => "what",
            GuidePage::Ladder => "ladder",
            GuidePage::GettingStarted => "getting-started",
            GuidePage::Navigation => "navigation",
            GuidePage::NormalMode => "normal-mode",
            GuidePage::FreeFlight => "free-flight",
            GuidePage::SavedViews => "saved-views",
            GuidePage::Touch => "touch",
            GuidePage::Planets => "planets",
        }
    }

    /// Page title.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            GuidePage::What => "What is Astrolith",
            GuidePage::Ladder => "The scale ladder",
            GuidePage::GettingStarted => "Getting started",
            GuidePage::Navigation => "Navigation",
            GuidePage::NormalMode => "Normal mode",
            GuidePage::FreeFlight => "Free flight",
            GuidePage::SavedViews => "Saved views",
            GuidePage::Touch => "Touch",
            GuidePage::Planets => "Planets",
        }
    }
}

/// Body text for a static page, or `None` for the built ladder page.
#[must_use]
pub fn static_text(page: GuidePage) -> Option<&'static str> {
    match page {
        GuidePage::What => Some(WHAT),
        GuidePage::Ladder => None,
        GuidePage::GettingStarted => Some(GETTING_STARTED),
        GuidePage::Navigation => Some(NAVIGATION),
        GuidePage::NormalMode => Some(NORMAL_MODE),
        GuidePage::FreeFlight => Some(FREE_FLIGHT),
        GuidePage::SavedViews => Some(SAVED_VIEWS),
        GuidePage::Touch => Some(TOUCH),
        GuidePage::Planets => Some(PLANETS),
    }
}

/// About > The scale ladder: one row per level from the game's own labels.
///
/// Each row names the level, its size as the scale readout writes it, and
/// its reference object, so the page matches the game by construction.
#[must_use]
pub fn scale_ladder() -> String {
    let mut out =
        String::from("The 14 levels, from the observable universe down to a single room:\n");
    for n in MIN_NAV_LEVEL..=MAX_NAV_LEVEL {
        let level = Level::new(n).unwrap_or(Level::MIN);
        out.push_str(&format!(
            "{} - {} ({})\n",
            level_label(level),
            scale_label(level),
            scale_anchor(level)
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_has_a_title_id_and_text() {
        assert_eq!(GuidePage::ALL.len(), 9);
        for page in GuidePage::ALL {
            assert!(!page.title().is_empty(), "title for {page:?}");
            assert!(!page.id().is_empty(), "id for {page:?}");
            match static_text(page) {
                Some(text) => assert!(text.len() > 20, "{page:?} carries its approved text"),
                None => assert_eq!(page, GuidePage::Ladder, "only the ladder is built"),
            }
        }
    }

    #[test]
    fn page_ids_match_the_panel_list() {
        let ids: Vec<&str> = GuidePage::ALL.iter().map(|page| page.id()).collect();
        assert_eq!(
            ids,
            vec![
                "what",
                "ladder",
                "getting-started",
                "navigation",
                "normal-mode",
                "free-flight",
                "saved-views",
                "touch",
                "planets"
            ]
        );
    }

    #[test]
    fn ladder_covers_every_rung_with_its_readout() {
        let ladder = scale_ladder();
        for n in MIN_NAV_LEVEL..=MAX_NAV_LEVEL {
            let level = Level::new(n).expect("ladder level");
            assert!(
                ladder.contains(level_label(level)),
                "ladder names {n}: {}",
                level_label(level)
            );
            assert!(
                ladder.contains(&scale_label(level)),
                "ladder sizes {n}: {}",
                scale_label(level)
            );
            assert!(ladder.contains(scale_anchor(level)), "ladder anchors {n}");
        }
    }

    #[test]
    fn approved_text_names_no_research_paths() {
        for page in GuidePage::ALL {
            let text = static_text(page).unwrap_or("");
            assert!(
                !text.contains("guides/"),
                "{page:?} never points at research documents"
            );
        }
        assert!(
            !scale_ladder().contains("guides/"),
            "the ladder never points at research documents"
        );
    }

    #[test]
    fn the_scale_ladder_shows_the_shell_heights() {
        let ladder = scale_ladder();
        for row in [
            "L11 - Regions - 10^6 m (below 1,000 km above the ground on Earth)",
            "L12 - Cities and landscapes - 10^5 m (below 100 km above the ground on Earth)",
            "L13 - Buildings - 10^3 m (below 1 km above the ground on Earth)",
            "L14 - Room - 10^1 m (below 10 m above the ground, on every planet)",
        ] {
            assert!(ladder.contains(row), "ladder row {row}");
        }
    }

    #[test]
    fn the_planets_guide_page_exists() {
        // C17 / AC16 (T14, G1): Help > Planets carries the approved Spec
        // v2 text verbatim.
        assert!(GuidePage::ALL.contains(&GuidePage::Planets));
        assert_eq!(GuidePage::Planets.id(), "planets");
        assert_eq!(GuidePage::Planets.title(), "Planets");
        let text = static_text(GuidePage::Planets).expect("planets text");
        for sentence in [
            "Every planet and moon is made from its seed and the age of its star system.",
            "Mountains, rifts, volcanoes, craters, rivers, lakes and clouds follow from that type",
            "Moons are made the same way.",
            "Choose Settings > Display > Scene > Planet look > Generated to see them.",
        ] {
            assert!(text.contains(sentence), "G1 lost: {sentence}");
        }
    }

    #[test]
    fn navigation_explains_levels_by_height() {
        assert!(NAVIGATION.ends_with(
            "On a planet, your level follows your height above the ground: L11 below 1,000 km, \
             L12 below 100 km, L13 below 1 km, L14 below 10 m (Earth heights; on other planets \
             L11 to L13 scale with the planet's size, and L14 is always 10 m). Climb back up to \
             return to the level above."
        ));
    }
}
