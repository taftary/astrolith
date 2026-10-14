//! Screenshots and the `--screens` picture list (#640, AC10, AC12).

use crate::nodes::Node;
use crate::orbit::Preset;

/// Folder the screenshot button saves into, under the working folder.
pub const SCREENSHOT_DIR: &str = "screenshots";

/// The screenshot path for a moment `unix_secs` seconds after 1970 (UTC):
/// `screenshots/planet-lab-YYYY-MM-DD-HHMMSS.png`.
#[must_use]
pub fn screenshot_path(unix_secs: u64) -> String {
    let days = unix_secs / 86_400;
    let rest = unix_secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{SCREENSHOT_DIR}/planet-lab-{year:04}-{month:02}-{day:02}-{:02}{:02}{:02}.png",
        rest / 3_600,
        rest % 3_600 / 60,
        rest % 60
    )
}

/// Calendar date of `days` after 1970-01-01 (Howard Hinnant's algorithm).
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

/// What the command line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// The editor window.
    Window,
    /// `--screens <dir>`: take every picture into `dir` and exit.
    Screens(String),
}

/// Reads the command line (without the program name); `None` for a wrong
/// one, which exits 2 with the usage line.
#[must_use]
pub fn parse_args(args: &[String]) -> Option<Launch> {
    match args {
        [] => Some(Launch::Window),
        [flag, dir] if flag == "--screens" => Some(Launch::Screens(dir.clone())),
        _ => None,
    }
}

/// The usage line.
pub const USAGE: &str = "usage: planet-lab [--screens <dir>]";

/// What one `--screens` picture shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotView {
    /// The default framing (Home).
    Home,
    /// One preset view.
    Preset(Preset),
    /// Free-fly a few metres over the ground.
    FreeFlyGround,
    /// Wireframe on.
    Wireframe,
    /// Normals on.
    Normals,
    /// LOD depth view on.
    LodDepth,
    /// A node selected, its panel open.
    Node(Node),
}

/// One picture: its file name and what it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shot {
    /// File name (`<name>.png`).
    pub file: &'static str,
    /// What it shows.
    pub view: ShotView,
}

/// Every `--screens` picture, in order (Spec v1: default, the seven
/// presets, free-fly near the ground, wireframe, normals, LOD depth, each
/// panel open).
pub const SHOTS: &[Shot] = &[
    Shot {
        file: "default.png",
        view: ShotView::Home,
    },
    Shot {
        file: "top.png",
        view: ShotView::Preset(Preset::Top),
    },
    Shot {
        file: "bottom.png",
        view: ShotView::Preset(Preset::Bottom),
    },
    Shot {
        file: "front.png",
        view: ShotView::Preset(Preset::Front),
    },
    Shot {
        file: "back.png",
        view: ShotView::Preset(Preset::Back),
    },
    Shot {
        file: "right.png",
        view: ShotView::Preset(Preset::Right),
    },
    Shot {
        file: "left.png",
        view: ShotView::Preset(Preset::Left),
    },
    Shot {
        file: "perspective.png",
        view: ShotView::Preset(Preset::Perspective),
    },
    Shot {
        file: "free-fly-ground.png",
        view: ShotView::FreeFlyGround,
    },
    Shot {
        file: "wireframe.png",
        view: ShotView::Wireframe,
    },
    Shot {
        file: "normals.png",
        view: ShotView::Normals,
    },
    Shot {
        file: "lod-depth.png",
        view: ShotView::LodDepth,
    },
    Shot {
        file: "node-planet.png",
        view: ShotView::Node(Node::Planet),
    },
    Shot {
        file: "node-moon.png",
        view: ShotView::Node(Node::Moon(0)),
    },
    Shot {
        file: "node-camera.png",
        view: ShotView::Node(Node::Camera),
    },
    Shot {
        file: "node-light.png",
        view: ShotView::Node(Node::Light),
    },
    Shot {
        file: "node-grid.png",
        view: ShotView::Node(Node::Grid),
    },
    Shot {
        file: "node-axes.png",
        view: ShotView::Node(Node::Axes),
    },
];

/// The `screens.txt` manifest: one line per picture, `file view`.
#[must_use]
pub fn manifest() -> String {
    SHOTS
        .iter()
        .map(|shot| format!("{} {:?}\n", shot.file, shot.view))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screenshots_are_named_by_date_and_time() {
        // C11 / AC10: 2026-10-14 00:15:22 UTC.
        assert_eq!(
            screenshot_path(1_791_936_922),
            "screenshots/planet-lab-2026-10-14-001522.png"
        );
        assert_eq!(
            screenshot_path(0),
            "screenshots/planet-lab-1970-01-01-000000.png"
        );
        assert_eq!(
            screenshot_path(951_782_400),
            "screenshots/planet-lab-2000-02-29-000000.png",
            "leap day"
        );
    }

    #[test]
    fn the_screens_list_covers_every_view_and_panel() {
        // C13 / AC12.
        for preset in Preset::ALL {
            assert!(
                SHOTS
                    .iter()
                    .any(|shot| shot.view == ShotView::Preset(preset)),
                "{preset:?}"
            );
        }
        for view in [
            ShotView::Home,
            ShotView::FreeFlyGround,
            ShotView::Wireframe,
            ShotView::Normals,
            ShotView::LodDepth,
        ] {
            assert!(SHOTS.iter().any(|shot| shot.view == view), "{view:?}");
        }
        for node in crate::nodes::tree(1) {
            assert!(
                SHOTS.iter().any(|shot| shot.view == ShotView::Node(node)),
                "{node:?} panel"
            );
        }
        let mut files: Vec<_> = SHOTS.iter().map(|shot| shot.file).collect();
        files.sort_unstable();
        files.dedup();
        assert_eq!(files.len(), SHOTS.len(), "unique names");
        assert_eq!(manifest().lines().count(), SHOTS.len());
    }

    #[test]
    fn the_command_line_opens_the_window_or_takes_the_screens() {
        let args = |list: &[&str]| list.iter().map(ToString::to_string).collect::<Vec<_>>();
        assert_eq!(parse_args(&args(&[])), Some(Launch::Window));
        assert_eq!(
            parse_args(&args(&["--screens", "out"])),
            Some(Launch::Screens("out".to_string()))
        );
        assert_eq!(
            parse_args(&args(&["--screens"])),
            None,
            "a folder is needed"
        );
        assert_eq!(parse_args(&args(&["--bogus"])), None);
    }
}
