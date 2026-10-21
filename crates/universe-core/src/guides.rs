//! Player guides text: About and Help pages (#473, T6).
//!
//! Pure text (`E-CORE-NO-BEVY`): the approved Spec v3 words (#742), kept in
//! the game's code as `&'static str`. The scale-ladder page is built from
//! [`crate::labels`] (`level_label`, `scale_label`, `scale_anchor`), so it
//! cannot drift from the game. `universe-render` shows these pages; the
//! `guides/` research documents are sources only and never shown.

use crate::coords::Level;
use crate::labels::{level_label, scale_anchor, scale_label};
use crate::nav::{MAX_NAV_LEVEL, MIN_NAV_LEVEL};

/// About > What is Astrolith (approved Spec v3 text, #742 G1).
pub const WHAT: &str = "Astrolith is a dive through the scales of the universe, from the cosmic web down to a planet. Every bright point on the way down is a place you can travel to. Pick one to go there; on the planet itself you walk and fly at true size. The same universe is built the same way every time."; // #779 G2

/// Help > Getting started (approved Spec v3 text, #742 G2).
pub const GETTING_STARTED: &str = "Press Space to let the autopilot fly to the chosen target.";

/// Help > Navigation (approved Spec v3 text, #742 G3, plus #873 G1, #884 G2).
pub const NAVIGATION: &str = "At the planet (L10) the dive ends: on the ground you walk and fly at true size. Its star lights the side facing it; the far side is a dim night, and the time of day stays fixed. In dive mode, W A S D or dragging with the left mouse button move you around the target at the same distance; the arrows dive in and out. Drag with the right mouse button to turn the view around the target. From the planet (L10) down, the target is the planet's center: diving in stops 3 m above the ground. Your body is selected at the start: Space flies to the planet, then 10 m above it. P selects or deselects your body; clicking any other point deselects it."; // #779 G1, #780 G1, #873 G1, #884 G2

/// Help > Normal mode (approved Spec v2 text, #756 G1).
pub const NORMAL_MODE: &str = "There is nothing to open: walk freely. Your body speeds up and slows down gradually, and keeps its momentum in turns. In third person the mouse turns the camera around you, W A S D move you where the camera looks, and your body turns to face the way it walks. Double-tap Space to take off or land.";

/// Help > Free flight (approved #884 G1 text).
pub const FREE_FLIGHT: &str = "Press F or 2 to fly freely, like a spaceship. W A S D move you; hold the right mouse button and drag to look around, all the way over the top if you like. Q and E roll the view to level it; planets never turn it for you. Shift+1 to Shift+9 set your speed, from slowest to fastest. Press F again or 3 to go back to diving. In free mode you pass through everything, even the ground.";

/// Help > Saved views (approved Spec v1 text).
pub const SAVED_VIEWS: &str = "Ctrl+1 to Ctrl+8 save where you are and where you look. Alt+1 to Alt+8 bring you back there. Saved views are kept when you close the game; clear them in Settings > Views.";

/// Help > Touch (#561, approved Spec v2 text, G1, plus #873 G2).
pub const TOUCH: &str = "On a touch screen: tap a point to choose it, tap again to cycle through the places under your finger. Pinch apart to dive in, pinch together to rise out. The round button at the bottom right flies the autopilot. Tap the mode reminder at the top left to switch between normal, free and dive. In normal and free mode the stick moves you (push it to the edge to run), and dragging anywhere else looks around. In normal mode, Jump jumps (double-tap to fly, hold to climb), Down descends while flying, and Camera switches the view. Settings > Controls > Touch chooses when the controls show and which hand holds the stick. In dive mode, drag with one finger to move around the target.";

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
}

impl GuidePage {
    /// Every guides page, in panel-list order.
    pub const ALL: [GuidePage; 8] = [
        GuidePage::What,
        GuidePage::Ladder,
        GuidePage::GettingStarted,
        GuidePage::Navigation,
        GuidePage::NormalMode,
        GuidePage::FreeFlight,
        GuidePage::SavedViews,
        GuidePage::Touch,
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
    }
}

/// About > The scale ladder: one row per level from the game's own labels.
///
/// Each row names the level, its size as the scale readout writes it, and
/// its reference object, so the page matches the game by construction.
#[must_use]
pub fn scale_ladder() -> String {
    let count = MAX_NAV_LEVEL - MIN_NAV_LEVEL + 1;
    let mut out = format!("The {count} levels, from the observable universe down to a planet:\n");
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
        assert_eq!(GuidePage::ALL.len(), 8);
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
                "touch"
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
    fn guides_carry_the_true_size_words() {
        // #779 G1/G2: the walking sentences are back.
        assert_eq!(
            WHAT,
            "Astrolith is a dive through the scales of the universe, from the cosmic web down to a planet. Every bright point on the way down is a place you can travel to. Pick one to go there; on the planet itself you walk and fly at true size. The same universe is built the same way every time.",
            "G1 what-is words"
        );
        assert_eq!(
            GETTING_STARTED, "Press Space to let the autopilot fly to the chosen target.",
            "G2 autopilot sentence"
        );
        assert!(
            NAVIGATION.starts_with(
                "At the planet (L10) the dive ends: on the ground you walk and fly at true size."
            ),
            "G3 travel sentence"
        );
        assert_eq!(
            NORMAL_MODE,
            "There is nothing to open: walk freely. Your body speeds up and slows down gradually, and keeps its momentum in turns. In third person the mouse turns the camera around you, W A S D move you where the camera looks, and your body turns to face the way it walks. Double-tap Space to take off or land.",
            "G1 feel sentence (#756)"
        );
        assert!(
            scale_ladder().starts_with("The 10 levels"),
            "the ladder stops at L10"
        );
    }

    #[test]
    fn guides_restore_the_walking_sentences() {
        // #779 G1/G2: walking on the drawn ground returns.
        assert!(NAVIGATION.starts_with(
            "At the planet (L10) the dive ends: on the ground you walk and fly at true size."
        ));
        assert_eq!(
            WHAT,
            "Astrolith is a dive through the scales of the universe, from the cosmic web down to a planet. Every bright point on the way down is a place you can travel to. Pick one to go there; on the planet itself you walk and fly at true size. The same universe is built the same way every time."
        );
    }

    #[test]
    fn guides_carry_the_day_and_night_sentence() {
        // C9 (#780 G1) plus #873 G1 plus #884 G2: Help > Navigation tells of
        // the star's light, the dim night and the fixed time of day, then
        // ends with the dive-control plus selection text word for word.
        assert!(NAVIGATION.starts_with(
            "At the planet (L10) the dive ends: on the ground you walk and fly at true size. Its star lights the side facing it; the far side is a dim night, and the time of day stays fixed."
        ));
        assert!(
            NAVIGATION.ends_with(
                "In dive mode, W A S D or dragging with the left mouse button move you around the target at the same distance; the arrows dive in and out. Drag with the right mouse button to turn the view around the target. From the planet (L10) down, the target is the planet's center: diving in stops 3 m above the ground. Your body is selected at the start: Space flies to the planet, then 10 m above it. P selects or deselects your body; clicking any other point deselects it."
            ),
            "G1 dive-control plus selection words"
        );
    }

    #[test]
    fn guides_explain_dive_controls() {
        // AC9/C9 (#873 T5 G1, G2, superseded by #884 G1/G2): Help >
        // Navigation ends with the G2 text and Help > Touch with the G2 text,
        // word for word.
        assert!(
            NAVIGATION.ends_with(
                "In dive mode, W A S D or dragging with the left mouse button move you around the target at the same distance; the arrows dive in and out. Drag with the right mouse button to turn the view around the target. From the planet (L10) down, the target is the planet's center: diving in stops 3 m above the ground. Your body is selected at the start: Space flies to the planet, then 10 m above it. P selects or deselects your body; clicking any other point deselects it."
            ),
            "G1 word for word"
        );
        assert!(
            TOUCH.ends_with("In dive mode, drag with one finger to move around the target."),
            "G2 word for word"
        );
    }

    #[test]
    fn guides_explain_free_flight_and_selection() {
        // AC9/C9 (#884 T5 G1, G2, G3): Help > Free flight shows the G1 text
        // and Help > Navigation ends with the G2 text, word for word. No
        // Guides page names E for exposure (G3: E is roll, B is darker).
        assert_eq!(
            FREE_FLIGHT,
            "Press F or 2 to fly freely, like a spaceship. W A S D move you; hold the right mouse button and drag to look around, all the way over the top if you like. Q and E roll the view to level it; planets never turn it for you. Shift+1 to Shift+9 set your speed, from slowest to fastest. Press F again or 3 to go back to diving. In free mode you pass through everything, even the ground.",
            "G1 word for word"
        );
        assert!(
            NAVIGATION.ends_with(
                "From the planet (L10) down, the target is the planet's center: diving in stops 3 m above the ground. Your body is selected at the start: Space flies to the planet, then 10 m above it. P selects or deselects your body; clicking any other point deselects it."
            ),
            "G2 word for word"
        );
        for page in GuidePage::ALL {
            let text = static_text(page).unwrap_or("");
            // G3: no page names E for exposure (E rolls, B darkens). The
            // letter E appears in normal words; only the key naming counts.
            // Check the explicit key phrases the old text used.
            assert!(
                !text.contains("E darkens")
                    && !text.contains("(E)")
                    && !text.contains("press E")
                    && !text.contains("Press E"),
                "{page:?} names no E for exposure"
            );
        }
        assert_eq!(static_text(GuidePage::FreeFlight), Some(FREE_FLIGHT));
        assert_eq!(static_text(GuidePage::Navigation), Some(NAVIGATION));
    }

    #[test]
    fn guides_reference_no_removed_code() {
        // #770 Q7 A: L10/generation kept as knowledge; no guide references removed code.
        // Removed code symbols (core surface stack + harness).
        const REMOVED: &[&str] = &[
            "icosphere",
            "SurfaceFrame",
            "SURFACE_DEPTH",
            "surface_radius",
            "surface_floor_radius",
            "facet_plane",
            "PLANET_RADIUS_CELL",
            "FACET_DEPTH",
            "level_ground",
            "walk_levels",
            "ground_height",
            "clamp_above_ground",
            "landing_step",
            "landing_normal_for",
            "landing_floor_cap",
            "landing_distance",
            "patch.rs",
            "FarBall",
            "draw_far_ball",
            "draw_air_rim",
            "draw_limb_preview",
            "ground_guides",
        ];
        // The generation knowledge base is untouched: verified byte-identical
        // to main at 0d353be via `git diff 0d353be -- guides/universe/L10/generation/`
        // (empty); here we assert the research index still lists its pages.
        assert!(
            include_str!("../../../guides/universe/L10/generation/documents/README.md")
                .contains("terrain"),
            "generation knowledge kept"
        );
        // The cleaned navigation page no longer cites removed code (#770 review item 2).
        const PLANET_APPROACH: &str =
            include_str!("../../../guides/navigation/documents/planet-approach.md");
        // No removed symbol appears in the guide text constants (WHAT/NAVIGATION)
        // or in the cleaned navigation page.
        for symbol in REMOVED {
            assert!(!WHAT.contains(symbol), "WHAT references removed {symbol}");
            assert!(
                !NAVIGATION.contains(symbol),
                "NAVIGATION references removed {symbol}"
            );
            assert!(
                !PLANET_APPROACH.contains(symbol),
                "planet-approach.md references removed {symbol}"
            );
        }
    }

    #[test]
    fn the_normal_mode_guide_explains_the_feel() {
        // #756 C7 (G1): the Normal mode page names the gradual feel, the
        // camera-relative movement, and the double-tap.
        for phrase in [
            "speeds up and slows down gradually",
            "keeps its momentum in turns",
            "the mouse turns the camera around you",
            "your body turns to face the way it walks",
            "Double-tap Space to take off or land",
        ] {
            assert!(NORMAL_MODE.contains(phrase), "the guide says: {phrase}");
        }
        assert_eq!(static_text(GuidePage::NormalMode), Some(NORMAL_MODE));
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
}
