//! The editor layout: panels and menus (#640, AC1).
//!
//! One place names what the window shows, so the egui code and the tests
//! read the same list: a menu bar on top, the nodes tree on the left, the
//! tools and options panel on the right, the status bar at the bottom and
//! the 3D viewport in the middle.

/// The window's regions, in build order (egui lays panels out first).
pub const PANELS: [&str; 5] = [
    "menu bar",
    "nodes",
    "tools and options",
    "status bar",
    "viewport",
];

/// The menus and their items, in bar order.
pub const MENUS: [(&str, &[&str]); 5] = [
    ("File", &["Screenshot (F12)", "Quit"]),
    (
        "View",
        &[
            "Wireframe",
            "Normals",
            "LOD depth view",
            "Stats overlay",
            "Grid",
            "Axes",
        ],
    ),
    ("Camera", &["Orbit / Free-fly (G)", "Home"]),
    ("Shaders", &["Planet squares"]),
    ("Help", &["About"]),
];

/// The About text (Help menu).
pub const ABOUT: &str = "Engine lab: any level of the game's universe from L1 to L10, a planet with its moons and shells included, drawn with the game's own code. A development program, never part of the game.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lab_lays_out_the_editor_panels() {
        // C1 / AC1: menu bar, nodes tree, tools panel, viewport and status
        // bar, titled "Engine lab"; the menus carry the spec's items.
        for panel in [
            "menu bar",
            "nodes",
            "tools and options",
            "viewport",
            "status bar",
        ] {
            assert!(PANELS.contains(&panel), "{panel}");
        }
        let names: Vec<&str> = MENUS.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, ["File", "View", "Camera", "Shaders", "Help"]);
        let view = MENUS[1].1;
        for item in ["Wireframe", "Normals", "LOD depth view", "Stats overlay"] {
            assert!(view.contains(&item), "View > {item}");
        }
        assert!(MENUS[0].1.iter().any(|item| item.starts_with("Screenshot")));
        assert_eq!(crate::TITLE, "Engine lab");
    }
}
