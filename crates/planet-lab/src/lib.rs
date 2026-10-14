//! Planet lab: a PC-only 3D editor view of one planet and its moons (#640).
//!
//! A development program, never part of the game and never shipped to
//! players (ADR 0029). It shows one planet of the game's universe with its
//! moons (the L10-L14 world) through the game's own planet code. This
//! library holds the lab's pure parts; `main.rs` is the window.

/// Window title of the lab (Q2).
pub const TITLE: &str = "Planet lab";

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
                !depends_on(&path, "planet-lab"),
                "{game} depends on the lab"
            );
        }
        assert!(depends_on(&format!("{root}/Cargo.toml"), "universe-render"));
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
        assert_eq!(TITLE, "Planet lab");
    }
}
