//! Binary entry point: windowed nested dive plus headless `--verify` and `--capture`.
//!
//! Without flags this opens the `Universe MVP` window: hover a marker, click
//! to target it, wheel or arrows to dive in and out, Spacebar for the
//! autopilot journey L1 -> L11, `Esc` quits. With `--verify` it runs headless
//! instead: no window is created; the autopilot journey is replayed from the
//! fixed root seed, every opened cell is regenerated twice and byte-compared,
//! the open/close inverse and the pre-entry preview are checked, canonical
//! snapshots print, and the process exits 0 on `VERIFY-OK` or 1 on
//! `VERIFY-FAIL`. With `--capture <dir>` it replays the same scripted journey
//! headless and writes one `level-L<N>.txt` snapshot plus one `frame-L<N>.ppm`
//! plot per level into `<dir>` (visual state the window would draw, as
//! assertable files), exiting 0 on `CAPTURE-OK` or 1 on `CAPTURE-FAIL`.
//! Any other command line prints one usage line to stderr and exits 2.
//! Flag parsing uses `std::env` only.

#[cfg(not(debug_assertions))]
use bevy::ecs::error::error;
#[cfg(debug_assertions)]
use bevy::ecs::error::panic;
use bevy::prelude::*;
use universe_core::coords::Level;
use universe_core::labels::window_title_for_level;
use universe_render::{DivePlugin, UniverseRenderPlugin};

use crate::capture::run_capture;
use crate::protocol::{EXIT_USAGE, usage};
use crate::verify::run_verify;

mod capture;
mod protocol;
mod verify;

/// What the command line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    /// No flags: open the window.
    Window,
    /// Exactly `--verify`: run the headless gate.
    Verify,
    /// Exactly `--capture <dir>`: write capture files into `<dir>`.
    Capture(String),
    /// Anything else: print usage to stderr, exit 2.
    Usage,
}

/// Parses the arguments after the program name (pure: unit-tested below).
fn parse_args(args: &[String]) -> Action {
    match args {
        [] => Action::Window,
        [verify] if verify == "--verify" => Action::Verify,
        [capture, dir] if capture == "--capture" => Action::Capture(dir.clone()),
        _ => Action::Usage,
    }
}

/// Starts the windowed app, `--verify` headless verification, or
/// `--capture <dir>` headless capture (no window in either headless mode).
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args) {
        Action::Window => {
            let mut app = App::new();
            #[cfg(debug_assertions)]
            app.set_error_handler(panic);
            #[cfg(not(debug_assertions))]
            app.set_error_handler(error);
            app.add_plugins(DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: window_title_for_level(Level::MIN),
                    ..default()
                }),
                ..default()
            }))
            .add_plugins((UniverseRenderPlugin, DivePlugin))
            .run();
        }
        Action::Verify => std::process::exit(run_verify()),
        Action::Capture(dir) => std::process::exit(run_capture(&dir)),
        Action::Usage => {
            eprintln!("{}", usage());
            std::process::exit(EXIT_USAGE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::EXIT_FAIL;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_string()).collect()
    }

    #[test]
    fn no_args_opens_the_window() {
        assert_eq!(parse_args(&[]), Action::Window);
    }

    #[test]
    fn verify_flag_verifies() {
        assert_eq!(parse_args(&args(&["--verify"])), Action::Verify);
    }

    #[test]
    fn capture_with_dir_captures() {
        assert_eq!(
            parse_args(&args(&["--capture", "out"])),
            Action::Capture("out".to_string())
        );
    }

    #[test]
    fn unknown_flag_is_usage() {
        assert_eq!(parse_args(&args(&["--frobnicate"])), Action::Usage);
    }

    #[test]
    fn capture_without_dir_is_usage() {
        assert_eq!(parse_args(&args(&["--capture"])), Action::Usage);
    }

    #[test]
    fn extra_args_are_usage() {
        assert_eq!(
            parse_args(&args(&["--verify", "--capture", "out"])),
            Action::Usage
        );
        assert_eq!(
            parse_args(&args(&["--capture", "out", "extra"])),
            Action::Usage
        );
    }

    #[test]
    fn capture_to_a_bad_dir_fails() {
        assert_eq!(run_capture("\0"), EXIT_FAIL);
    }

    #[test]
    fn usage_has_no_reserved_words() {
        for word in ["error", "panic", "warn", "deprecat", "FAILED"] {
            assert!(
                !usage().contains(word),
                "usage line contains reserved word {word}"
            );
        }
    }
}
