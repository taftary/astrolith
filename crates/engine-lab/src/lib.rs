//! Engine lab: a PC-only 3D editor view of the game's universe (#640,
//! renamed in #654).
//!
//! A development program, never part of the game and never shipped to
//! players (ADR 0029). It shows any level of the game's universe from L1 to
//! L10, a planet with its moons and its L11-L14 shells included, through
//! the game's own drawing code. This library holds the lab's pure parts;
//! `main.rs` is the window.

pub mod clipboard;
pub mod code;
pub mod keys;
pub mod layout;
pub mod nodes;
pub mod orbit;
pub mod scene;
pub mod select;
pub mod shaders;
pub mod shots;
pub mod stats;
pub mod units;

/// Window title of the lab (Q2).
pub const TITLE: &str = "Engine lab";

/// Whether this build target is a desktop the lab supports (Q3): Windows,
/// Linux or macOS. The binary refuses to build anywhere else.
#[must_use]
pub const fn is_desktop_target() -> bool {
    cfg!(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "macos"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Package names a workspace manifest at `path` depends on (normal,
    /// dev and build sections alike), read as plain text.
    fn depends_on(path: &str, name: &str) -> bool {
        let manifest = std::fs::read_to_string(path).expect("manifest");
        manifest.lines().map(str::trim).any(|line| {
            line.starts_with(&format!("{name}.")) || line.starts_with(&format!("{name} "))
        })
    }

    #[test]
    fn the_game_crates_never_depend_on_the_lab() {
        // C2 / AC1: the game never links the lab (Q1).
        let root = env!("CARGO_MANIFEST_DIR");
        for game in [
            "universe-core",
            "universe-render",
            "universe-app",
            "universe-verify",
        ] {
            let path = format!("{root}/../{game}/Cargo.toml");
            assert!(
                !depends_on(&path, "engine-lab"),
                "{game} depends on the lab"
            );
        }
        assert!(depends_on(&format!("{root}/Cargo.toml"), "universe-render"));
    }

    /// Text files under `dir`, skipping build output, git and agent notes.
    fn text_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if !matches!(
                    name.as_str(),
                    "target" | ".git" | ".agent" | "screenshots" | "guides"
                ) {
                    text_files(&path, out);
                }
            } else if [".rs", ".md", ".toml", ".yml", ".lock", ".wgsl"]
                .iter()
                .any(|ext| name.ends_with(ext))
            {
                out.push(path);
            }
        }
    }

    #[test]
    fn the_lab_is_named_engine_lab() {
        // C1 / AC1 of #654: the program, title and screenshots read
        // "engine-lab"; only ADR 0029 (its rename note) and links to its
        // file keep the old name.
        assert_eq!(TITLE, "Engine lab");
        assert!(shots::USAGE.starts_with("usage: engine-lab"));
        assert!(
            shots::screenshot_path(0).contains("/engine-lab-"),
            "screenshots are engine-lab-..."
        );
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut files = Vec::new();
        for top in ["crates", "docs", ".github", "scripts"] {
            text_files(&root.join(top), &mut files);
        }
        for top in [
            "Cargo.toml",
            "Cargo.lock",
            "deny.toml",
            "AGENTS.md",
            "CLAUDE.md",
        ] {
            files.push(root.join(top));
        }
        // Built from parts so this test never finds itself.
        let old = ["-lab", "_lab", " lab"].map(|tail| format!("planet{tail}"));
        let mut left = Vec::new();
        for path in files {
            if path.ends_with("0029-planet-lab.md") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let lower = text.to_lowercase().replace(&format!("0029-{}", old[0]), "");
            if old.iter().any(|name| lower.contains(name)) {
                left.push(path.display().to_string());
            }
        }
        assert!(left.is_empty(), "old name left in {left:?}");
    }

    #[test]
    fn the_lab_is_desktop_only() {
        // C15 / AC14: the lab builds for Windows, Linux and macOS only.
        assert!(is_desktop_target(), "tests run on a desktop");
        let main = include_str!("main.rs");
        assert!(
            main.contains("compile_error!"),
            "the binary refuses non-desktop targets"
        );
        assert_eq!(TITLE, "Engine lab");
    }
}
