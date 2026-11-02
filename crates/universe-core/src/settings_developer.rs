//! Developer marker display choices (`E-FILE-SIZE` split from `settings`, #1128 S1/S2).
//! Re-exported from [`crate::settings`].

/// One independently filterable class of developer marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeveloperPart {
    /// Object-body indicators.
    Bodies,
    /// Members of generated populations.
    Populations,
    /// Internal and shared structures.
    Structures,
    /// Orbit and connection paths.
    Paths,
    /// Areas, volumes and boundaries.
    Boundaries,
    /// Axes, graticules and other references.
    References,
    /// Developer-only labels.
    Labels,
}

impl DeveloperPart {
    /// Every filter in Settings order.
    pub const ALL: [DeveloperPart; 7] = [
        DeveloperPart::Bodies,
        DeveloperPart::Populations,
        DeveloperPart::Structures,
        DeveloperPart::Paths,
        DeveloperPart::Boundaries,
        DeveloperPart::References,
        DeveloperPart::Labels,
    ];

    /// Display name in Settings.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            DeveloperPart::Bodies => "Bodies",
            DeveloperPart::Populations => "Populations",
            DeveloperPart::Structures => "Structures",
            DeveloperPart::Paths => "Paths",
            DeveloperPart::Boundaries => "Boundaries",
            DeveloperPart::References => "References",
            DeveloperPart::Labels => "Labels",
        }
    }

    /// Settings-file key for this filter.
    #[must_use]
    pub const fn file_key(self) -> &'static str {
        match self {
            DeveloperPart::Bodies => "developer_marker_bodies",
            DeveloperPart::Populations => "developer_marker_populations",
            DeveloperPart::Structures => "developer_marker_structures",
            DeveloperPart::Paths => "developer_marker_paths",
            DeveloperPart::Boundaries => "developer_marker_boundaries",
            DeveloperPart::References => "developer_marker_references",
            DeveloperPart::Labels => "developer_marker_labels",
        }
    }

    /// Filter from a settings-file key.
    #[must_use]
    pub fn from_file_key(key: &str) -> Option<DeveloperPart> {
        DeveloperPart::ALL
            .into_iter()
            .find(|part| part.file_key() == key)
    }

    const fn index(self) -> usize {
        match self {
            DeveloperPart::Bodies => 0,
            DeveloperPart::Populations => 1,
            DeveloperPart::Structures => 2,
            DeveloperPart::Paths => 3,
            DeveloperPart::Boundaries => 4,
            DeveloperPart::References => 5,
            DeveloperPart::Labels => 6,
        }
    }
}

/// Saved developer marker master switch and independent part filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeveloperMarkers {
    /// Master switch: off on a fresh profile and after Reset Developer.
    pub enabled: bool,
    filters: [bool; 7],
}

impl Default for DeveloperMarkers {
    fn default() -> DeveloperMarkers {
        DeveloperMarkers {
            enabled: false,
            filters: [true; 7],
        }
    }
}

impl DeveloperMarkers {
    /// Whether a part is enabled by the saved master and its own filter.
    #[must_use]
    pub fn allows(self, part: DeveloperPart) -> bool {
        self.enabled && self.filters.get(part.index()).copied().unwrap_or(false)
    }

    /// Saved state of one part filter, independent of the master.
    #[must_use]
    pub fn filter(self, part: DeveloperPart) -> bool {
        self.filters.get(part.index()).copied().unwrap_or(false)
    }

    /// Flips one saved part filter.
    pub fn toggle_filter(&mut self, part: DeveloperPart) {
        if let Some(slot) = self.filters.get_mut(part.index()) {
            *slot = !*slot;
        }
    }

    /// Sets one saved part filter.
    pub fn set_filter(&mut self, part: DeveloperPart, enabled: bool) {
        if let Some(slot) = self.filters.get_mut(part.index()) {
            *slot = enabled;
        }
    }

    /// Effective choices for this build: release always disables the layer,
    /// including when every saved option is on (#1128 AC2).
    #[must_use]
    pub fn effective(self) -> DeveloperMarkers {
        self.effective_for(cfg!(debug_assertions))
    }

    /// Effective choices for an explicit build kind, kept pure so tests cover
    /// both policies without changing the universe or saved settings.
    #[must_use]
    pub fn effective_for(self, debug_build: bool) -> DeveloperMarkers {
        DeveloperMarkers {
            enabled: self.enabled && debug_build,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_filters_match_the_approved_controls() {
        let mut markers = DeveloperMarkers::default();
        assert!(!markers.enabled);
        for part in DeveloperPart::ALL {
            assert!(markers.filter(part));
            assert!(!markers.allows(part), "master off wins for {part:?}");
            markers.toggle_filter(part);
            assert!(!markers.filter(part));
        }
        markers.enabled = true;
        assert!(
            DeveloperPart::ALL
                .into_iter()
                .all(|part| !markers.allows(part))
        );
    }

    #[test]
    fn debug_build_applies_master_and_part_filters() {
        let mut saved = DeveloperMarkers {
            enabled: true,
            ..DeveloperMarkers::default()
        };
        saved.set_filter(DeveloperPart::Paths, false);
        let effective = saved.effective_for(true);
        assert!(effective.allows(DeveloperPart::Bodies));
        assert!(!effective.allows(DeveloperPart::Paths));
        saved.enabled = false;
        assert!(
            DeveloperPart::ALL
                .into_iter()
                .all(|part| !saved.effective_for(true).allows(part))
        );
    }

    #[test]
    fn release_build_forcibly_excludes_developer_markers() {
        let saved = DeveloperMarkers {
            enabled: true,
            ..DeveloperMarkers::default()
        };
        assert_eq!(saved.effective_for(true), saved);
        assert_eq!(saved.effective().enabled, cfg!(debug_assertions));
        let effective = saved.effective_for(false);
        assert!(!effective.enabled);
        assert!(
            DeveloperPart::ALL
                .into_iter()
                .all(|part| !effective.allows(part))
        );
        assert!(saved.enabled, "the saved choices are not rewritten");
    }
}
