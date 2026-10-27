//! Binary entry point: windowed nested dive, headless `--verify` and `--capture`,
//! and the windowed `--screens` picture mode.
//!
//! Without flags this opens the `Universe MVP` window: hover a marker, click
//! to target it, wheel or arrows to dive in and out, Spacebar for the
//! autopilot journey L1 -> L10, `Esc` opens the menu (`#473`). With `--verify` it runs headless
//! instead: no window is created; the autopilot journey is replayed from the
//! fixed root seed, every opened cell is regenerated twice and byte-compared,
//! the open/close inverse and the pre-entry preview are checked, canonical
//! snapshots print, and the process exits 0 on `VERIFY-OK` or 1 on
//! `VERIFY-FAIL`. With `--capture <dir>` it replays the same scripted journey
//! headless and writes one `level-L<N>.txt` snapshot plus one `frame-L<N>.ppm`
//! plot per level into `<dir>` (visual state the window would draw, as
//! assertable files), exiting 0 on `CAPTURE-OK` or 1 on `CAPTURE-FAIL`.
//! With `--screens <dir>` it opens the real window with default settings,
//! shows each UI screen, saves one PNG per screen plus `screens.txt` into
//! `<dir>`, and exits 0 on `SCREENS-OK` or 1 on `SCREENS-FAIL` (ADR 0021).
//! With `--patch-bench` it times the ground patch builds of a fixed set on
//! one thread and prints one `PATCHBENCH` line (#994). A trailing
//! `--build-workers <n>` runs the window or `--screens` with exactly `n`
//! ground build threads (#994, the phone-like run); `--size <w>x<h>` and
//! `--scale <f>` set the window's physical size and scale factor (#1022
//! T1, measuring aids; see `options`). Any other command line prints one usage line to stderr and exits 2.
//! Flag parsing uses `std::env` only.

#[cfg(not(debug_assertions))]
use bevy::ecs::error::error;
#[cfg(debug_assertions)]
use bevy::ecs::error::panic;
use bevy::prelude::*;
use universe_core::coords::Level;
use universe_core::labels::window_title_for_level;
use universe_render::{DivePlugin, PatchMaterialPlugin, UniverseRenderPlugin};

use crate::bench::run_patch_bench;
use crate::capture::run_capture;
use crate::gpu::{FramePacingPlugin, render_plugin, task_pool_plugin};
use crate::options::{WindowOptions, split_options};
use crate::persist::{SettingsPath, load_or_default, settings_path};
use crate::protocol::{EXIT_USAGE, usage};
use crate::screens::{run_screens, run_shot_file};
use crate::verify::run_verify;

mod bench;
mod capture;
mod gpu;
mod options;
mod persist;
mod protocol;
mod screens;
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
    /// `--screens <dir> [groups]`: save window pictures into `<dir>`, every
    /// shot or only the chosen groups (#847).
    Screens(String, universe_render::ShotSelection),
    /// `--screens <dir> --shots <file> [--timing]`: the file's shots instead
    /// (#863), optionally timing each view (#819).
    ScreenShots(String, String, bool),
    /// Exactly `--patch-bench`: time the fixed set's builds (#994).
    PatchBench,
    /// Anything else: print usage to stderr, exit 2.
    Usage,
}

/// Parses the arguments after the program name (pure: unit-tested below).
fn parse_args(args: &[String]) -> Action {
    match args {
        [] => Action::Window,
        [verify] if verify == "--verify" => Action::Verify,
        [bench] if bench == "--patch-bench" => Action::PatchBench,
        [capture, dir] if capture == "--capture" => Action::Capture(dir.clone()),
        [screens, dir] if screens == "--screens" => {
            Action::Screens(dir.clone(), universe_render::ShotSelection::default())
        }
        [screens, dir, shots, file] if screens == "--screens" && shots == "--shots" => {
            Action::ScreenShots(dir.clone(), file.clone(), false)
        }
        [screens, dir, shots, file, timing]
            if screens == "--screens" && shots == "--shots" && timing == "--timing" =>
        {
            Action::ScreenShots(dir.clone(), file.clone(), true)
        }
        [screens, dir, groups] if screens == "--screens" => {
            universe_render::ShotSelection::parse(groups).map_or(Action::Usage, |selection| {
                Action::Screens(dir.clone(), selection)
            })
        }
        _ => Action::Usage,
    }
}

/// The action for `args` and its trailing window options (#994, #1022
/// T1): options only follow a window run, and `--size` only the plain window
/// or a `--timing` run (the built-in shots set their own sizes).
fn action_for(args: &[String]) -> (Action, WindowOptions) {
    let Some((rest, options)) = split_options(args) else {
        return (Action::Usage, WindowOptions::default());
    };
    let action = parse_args(rest);
    let sized = matches!(action, Action::Window | Action::ScreenShots(_, _, true));
    let window = sized || matches!(action, Action::Screens(..) | Action::ScreenShots(..));
    let given = options != WindowOptions::default();
    if (given && !window) || (options.size.is_some() && !sized) {
        return (Action::Usage, WindowOptions::default());
    }
    (action, options)
}

/// The plain window's opening size: `--size` and `--scale` when given,
/// else Bevy's default following the desktop.
fn window_resolution(options: &WindowOptions) -> bevy::window::WindowResolution {
    let mut resolution = options
        .size
        .map_or_else(bevy::window::WindowResolution::default, |(w, h)| {
            bevy::window::WindowResolution::new(w, h)
        });
    if let Some(scale) = options.scale {
        resolution = resolution.with_scale_factor_override(scale);
    }
    resolution
}

/// Starts the windowed app, `--verify` headless verification, or
/// `--capture <dir>` headless capture (no window in either headless mode).
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (action, options) = action_for(&args);
    match action {
        Action::Window => {
            let path = settings_path();
            let settings = universe_render::SettingsStore(load_or_default(&path));
            let mut app = App::new();
            #[cfg(debug_assertions)]
            app.set_error_handler(panic);
            #[cfg(not(debug_assertions))]
            app.set_error_handler(error);
            app.insert_resource(SettingsPath(path))
                .insert_resource(settings)
                .add_plugins(
                    DefaultPlugins
                        .set(WindowPlugin {
                            primary_window: Some(Window {
                                title: window_title_for_level(Level::MIN),
                                resolution: window_resolution(&options),
                                ..default()
                            }),
                            ..default()
                        })
                        .set(render_plugin())
                        .set(task_pool_plugin(options.workers)),
                )
                .add_plugins((UniverseRenderPlugin, PatchMaterialPlugin, DivePlugin))
                .add_plugins(FramePacingPlugin)
                // `Last`: after the sky's time is kept on quit (#981 T5).
                .add_systems(Last, persist::save_on_change)
                .run();
        }
        Action::Verify => std::process::exit(run_verify()),
        Action::Capture(dir) => std::process::exit(run_capture(&dir)),
        Action::Screens(dir, selection) => {
            std::process::exit(run_screens(&dir, selection, options));
        }
        Action::ScreenShots(dir, file, timing) => {
            std::process::exit(run_shot_file(&dir, &file, timing, options));
        }
        Action::PatchBench => std::process::exit(run_patch_bench()),
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
    fn patch_bench_parses_alone() {
        assert_eq!(parse_args(&args(&["--patch-bench"])), Action::PatchBench);
        assert_eq!(
            parse_args(&args(&["--patch-bench", "extra"])),
            Action::Usage
        );
    }

    #[test]
    fn window_options_trail_a_window_run_only() {
        let timing = args(&[
            "--screens",
            "out",
            "--shots",
            "s.txt",
            "--timing",
            "--build-workers",
            "2",
            "--size",
            "1600x900",
        ]);
        let (action, options) = action_for(&timing);
        assert_eq!(
            action,
            Action::ScreenShots("out".into(), "s.txt".into(), true)
        );
        assert_eq!(
            (options.workers, options.size),
            (Some(2), Some((1600, 900)))
        );
        let window = args(&["--build-workers", "3", "--scale", "1.5"]);
        assert_eq!(action_for(&window).1.workers, Some(3));
        assert_eq!(action_for(&window).1.scale, Some(1.5));
        assert_eq!(action_for(&args(&["--verify"])).0, Action::Verify);
        for wrong in [
            &["--build-workers", "0"][..],
            &["--build-workers", "two"],
            &["--build-workers"],
            &["--verify", "--build-workers", "2"],
            &["--patch-bench", "--build-workers", "2"],
            // The built-in shots set their own sizes (#1022 T1).
            &["--screens", "out", "--size", "800x600"],
            &["--screens", "out", "--shots", "s.txt", "--size", "800x600"],
        ] {
            assert_eq!(action_for(&args(wrong)).0, Action::Usage, "{wrong:?}");
        }
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
    fn screens_with_dir_takes_pictures() {
        assert_eq!(
            parse_args(&args(&["--screens", "out"])),
            Action::Screens("out".to_string(), universe_render::ShotSelection::default())
        );
        // #847: a group list picks the shots.
        let Action::Screens(_, smoke) = parse_args(&args(&["--screens", "out", "smoke"])) else {
            panic!("smoke is a screens run");
        };
        assert!(smoke.smoke);
        let Action::Screens(_, two) = parse_args(&args(&["--screens", "out", "sky,walking"]))
        else {
            panic!("groups are a screens run");
        };
        assert_eq!(two.groups.len(), 2);
        // #863: a shot file replaces the list.
        assert_eq!(
            parse_args(&args(&["--screens", "out", "--shots", "shots.txt"])),
            Action::ScreenShots("out".to_string(), "shots.txt".to_string(), false)
        );
        assert_eq!(
            parse_args(&args(&[
                "--screens",
                "out",
                "--shots",
                "views.txt",
                "--timing"
            ])),
            Action::ScreenShots("out".to_string(), "views.txt".to_string(), true)
        );
        assert_eq!(
            parse_args(&args(&["--screens", "out", "--shots"])),
            Action::Usage
        );
    }

    #[test]
    fn screens_without_dir_or_with_extra_args_is_usage() {
        assert_eq!(parse_args(&args(&["--screens"])), Action::Usage);
        assert_eq!(
            parse_args(&args(&["--screens", "out", "extra"])),
            Action::Usage
        );
        assert_eq!(
            parse_args(&args(&["--screens", "out", "sky", "more"])),
            Action::Usage
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
