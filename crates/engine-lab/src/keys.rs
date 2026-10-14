//! The lab's single keys and what they do (#640, #654 D3, D5).
//!
//! One table, so the window and the tests read the same keys: F shows the
//! selected object (the classic editor key), G switches Orbit and
//! Free-fly, Page Up and Page Down move to the parent and child level.

use bevy::input::keyboard::KeyCode;

/// What a single key does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabAction {
    /// Show the selected object (F, #654 D3-D4).
    ShowSelected,
    /// Switch Orbit and Free-fly (G, #654 D3).
    ToggleFreeFly,
    /// One level up (Page Up, #654 D5).
    Parent,
    /// Open the selected object (Page Down, #654 D5).
    Child,
    /// The Home view (Home).
    Home,
    /// A screenshot (F12).
    Screenshot,
}

/// The action of `key`, if it has one.
#[must_use]
pub fn action_for(key: KeyCode) -> Option<LabAction> {
    match key {
        KeyCode::KeyF => Some(LabAction::ShowSelected),
        KeyCode::KeyG => Some(LabAction::ToggleFreeFly),
        KeyCode::PageUp => Some(LabAction::Parent),
        KeyCode::PageDown => Some(LabAction::Child),
        KeyCode::Home => Some(LabAction::Home),
        KeyCode::F12 => Some(LabAction::Screenshot),
        _ => None,
    }
}

/// Every key with an action, for the input loop.
pub const KEYS: [KeyCode; 6] = [
    KeyCode::KeyF,
    KeyCode::KeyG,
    KeyCode::PageUp,
    KeyCode::PageDown,
    KeyCode::Home,
    KeyCode::F12,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g_switches_free_fly_and_f_shows_the_selected() {
        // C5 / AC4 of #654: G is Free-fly, F shows the selected; the
        // Camera menu names G.
        assert_eq!(action_for(KeyCode::KeyG), Some(LabAction::ToggleFreeFly));
        assert_eq!(action_for(KeyCode::KeyF), Some(LabAction::ShowSelected));
        assert_eq!(action_for(KeyCode::PageUp), Some(LabAction::Parent));
        assert_eq!(action_for(KeyCode::PageDown), Some(LabAction::Child));
        assert_eq!(action_for(KeyCode::KeyX), None);
        for key in KEYS {
            assert!(action_for(key).is_some(), "{key:?} has an action");
        }
        let camera = crate::layout::MENUS
            .iter()
            .find(|(menu, _)| *menu == "Camera")
            .map(|(_, items)| *items)
            .unwrap_or_default();
        assert!(camera.contains(&"Orbit / Free-fly (G)"), "{camera:?}");
        assert!(camera.iter().all(|item| !item.contains("(F)")));
    }
}
