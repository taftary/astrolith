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

/// About › What is Astrolith (approved Spec v1 text).
pub const WHAT: &str = "Astrolith is a dive through the scales of the universe, from the cosmic web down to a single room. Every bright point is a place you can enter. Pick one, dive in, and the world around you becomes the next, smaller level. The same universe is built the same way every time.";

/// Help › Getting started (approved Spec v1 text).
pub const GETTING_STARTED: &str = "Move the mouse over a bright point to highlight it; its name appears. Click to make it your target. Hold the Up arrow (or scroll) to dive toward it; hold Down to rise back out. Press Space to let the autopilot fly the whole journey from L1 to the room.";

/// Help › Navigation (approved Spec v1 text).
pub const NAVIGATION: &str = "Click: choose a target. Click again on the same spot: cycle between the places under the pointer. Up arrow, = or mouse wheel: dive in. Down arrow or -: rise out. Space: autopilot on or off. E / Shift+E: brighter / darker. Esc: open the menu.";

/// Help › Free flight (approved Spec v1 text).
pub const FREE_FLIGHT: &str = "Press F to fly freely. W A S D move you, hold the right mouse button and drag to look around. Shift+1 to Shift+9 set your speed, from slowest to fastest. Press F again to go back to diving.";

/// Help › Saved views (approved Spec v1 text).
pub const SAVED_VIEWS: &str = "Ctrl+1 to Ctrl+8 save where you are and where you look. 1 to 8 bring you back there. Saved views are kept when you close the game; clear them in Settings › Views.";

/// Guides page id (matches the panel list leaf ids).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuidePage {
    /// About › What is Astrolith.
    What,
    /// About › The scale ladder (built from [`crate::labels`]).
    Ladder,
    /// Help › Getting started.
    GettingStarted,
    /// Help › Navigation.
    Navigation,
    /// Help › Free flight.
    FreeFlight,
    /// Help › Saved views.
    SavedViews,
}

impl GuidePage {
    /// Every guides page, in panel-list order.
    pub const ALL: [GuidePage; 6] = [
        GuidePage::What,
        GuidePage::Ladder,
        GuidePage::GettingStarted,
        GuidePage::Navigation,
        GuidePage::FreeFlight,
        GuidePage::SavedViews,
    ];

    /// Panel list leaf id for this page.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            GuidePage::What => "what",
            GuidePage::Ladder => "ladder",
            GuidePage::GettingStarted => "getting-started",
            GuidePage::Navigation => "navigation",
            GuidePage::FreeFlight => "free-flight",
            GuidePage::SavedViews => "saved-views",
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
            GuidePage::FreeFlight => "Free flight",
            GuidePage::SavedViews => "Saved views",
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
        GuidePage::FreeFlight => Some(FREE_FLIGHT),
        GuidePage::SavedViews => Some(SAVED_VIEWS),
    }
}

/// About › The scale ladder: one row per level from the game's own labels.
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
            "{} · {} ({})\n",
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
        assert_eq!(GuidePage::ALL.len(), 6);
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
                "free-flight",
                "saved-views"
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
}
