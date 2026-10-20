//! `--screens <dir>` driver: real window, one PNG per UI screen (#492, ADR 0021).
//!
//! The folder is created and probed before the window opens, so an
//! unwritable folder fails fast. The window runs with default settings only:
//! no settings file is read or written (AC6). The render-side
//! [`ScreensPlugin`] walks the screens and queues each captured picture; this
//! driver writes the PNG files, checks they are not blank and have the asked size,
//! writes `screens.txt`, prints the one terminal line, and exits.

use bevy::prelude::*;
use bevy::window::WindowResolution;
use std::path::{Path, PathBuf};
use universe_core::coords::Level;
use universe_core::labels::window_title_for_level;
use universe_core::settings::Settings;
use universe_render::{
    CapturedShot, DivePlugin, PatchMaterialPlugin, ScreensPlugin, ScreensQueue, SettingsStore,
    ShotSelection, UniverseRenderPlugin, shot_count_for,
};

use crate::gpu::{FramePacingPlugin, render_plugin};
use crate::protocol::{EXIT_FAIL, EXIT_PASS, screens_fail, screens_ok};

/// Name of the list file written next to the pictures.
pub(crate) const LIST_FILE: &str = "screens.txt";

/// Opening window size in physical pixels (the first shot is wide).
const START_SIZE: (u32, u32) = (1280, 720);

/// Output folder plus the list lines written so far.
#[derive(Resource, Debug)]
struct ScreensOut {
    /// Folder the pictures go to.
    dir: PathBuf,
    /// Folder as typed on the command line (for the terminal line).
    shown: String,
    /// One `screens.txt` line per stored picture.
    lines: Vec<String>,
    /// Pictures this run takes (#847: every shot or the chosen groups).
    expected: usize,
}

/// Runs the screens mode and returns the process exit code.
pub(crate) fn run_screens(dir: &str, selection: ShotSelection) -> i32 {
    let root = Path::new(dir);
    if let Err(reason) = prepare_dir(root) {
        println!("{}", screens_fail(&reason));
        return EXIT_FAIL;
    }
    let mut app = App::new();
    app.insert_resource(SettingsStore(Settings::default()))
        .insert_resource(ScreensOut {
            dir: root.to_path_buf(),
            shown: dir.to_string(),
            lines: Vec::new(),
            expected: shot_count_for(&selection),
        })
        .insert_resource(selection)
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: window_title_for_level(Level::MIN),
                        resolution: WindowResolution::new(START_SIZE.0, START_SIZE.1)
                            .with_scale_factor_override(1.0),
                        ..default()
                    }),
                    ..default()
                })
                .set(render_plugin()),
        )
        .add_plugins((
            UniverseRenderPlugin,
            PatchMaterialPlugin,
            DivePlugin,
            ScreensPlugin,
            FramePacingPlugin,
        ))
        .add_systems(Update, drain_queue);
    match app.run() {
        AppExit::Success => EXIT_PASS,
        AppExit::Error(_) => EXIT_FAIL,
    }
}

/// Creates `dir` and proves it is writable with a probe file.
fn prepare_dir(dir: &Path) -> Result<(), String> {
    let shown = dir.display();
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create folder {shown}: {e}"))?;
    let probe = dir.join(".screens-probe");
    std::fs::write(&probe, b"probe").map_err(|e| format!("cannot write folder {shown}: {e}"))?;
    std::fs::remove_file(&probe).map_err(|e| format!("cannot clean folder {shown}: {e}"))
}

/// Writes queued pictures; on the end state writes the list and exits.
fn drain_queue(
    mut queue: ResMut<ScreensQueue>,
    mut out: ResMut<ScreensOut>,
    mut exit: MessageWriter<AppExit>,
) {
    let mut failure = queue.failure.take();
    for shot in std::mem::take(&mut queue.captured) {
        if failure.is_some() {
            break;
        }
        match store(&out.dir, shot) {
            Ok(line) => out.lines.push(line),
            Err(reason) => failure = Some(reason),
        }
    }
    if failure.is_none() && !queue.done {
        return;
    }
    let result = failure.map_or_else(|| finish(&out), Err);
    match result {
        Ok(files) => {
            println!("{}", screens_ok(files, &out.shown));
            exit.write(AppExit::Success);
        }
        Err(reason) => {
            println!("{}", screens_fail(&reason));
            exit.write(AppExit::from_code(1));
        }
    }
    queue.done = false;
}

/// Writes `screens.txt`; returns the file count (pictures plus the list).
fn finish(out: &ScreensOut) -> Result<usize, String> {
    if out.lines.len() != out.expected {
        return Err(format!(
            "stored {} of {} pictures",
            out.lines.len(),
            out.expected
        ));
    }
    let path = out.dir.join(LIST_FILE);
    std::fs::write(&path, list_text(&out.lines))
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(out.lines.len() + 1)
}

/// Checks one picture and writes it as PNG; returns its list line.
fn store(dir: &Path, shot: CapturedShot) -> Result<String, String> {
    let rgba = shot
        .image
        .try_into_dynamic()
        .map_err(|e| format!("cannot convert {}: {e}", shot.file))?
        .to_rgba8();
    let size = rgba.dimensions();
    if size != shot.size {
        return Err(format!(
            "{} is {}x{}, expected {}x{}",
            shot.file, size.0, size.1, shot.size.0, shot.size.1
        ));
    }
    if is_blank(rgba.as_raw()) {
        return Err(format!("{} is blank", shot.file));
    }
    let path = dir.join(shot.file);
    rgba.save(&path)
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(list_line(shot.file, size))
}

/// Whether an RGBA buffer holds a single colour (pure).
fn is_blank(rgba: &[u8]) -> bool {
    let mut pixels = rgba.chunks_exact(4);
    pixels
        .next()
        .is_none_or(|first| pixels.all(|pixel| pixel == first))
}

/// One `screens.txt` line: `<file> <width>x<height>` (pure).
fn list_line(file: &str, size: (u32, u32)) -> String {
    format!("{file} {}x{}", size.0, size.1)
}

/// Full `screens.txt` text, one line per picture (pure).
fn list_text(lines: &[String]) -> String {
    lines.iter().map(|line| format!("{line}\n")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_colour_is_blank_and_two_colours_are_not() {
        let one = [10, 20, 30, 255].repeat(16);
        assert!(is_blank(&one));
        let mut two = one.clone();
        two[4] = 11;
        assert!(!is_blank(&two));
        assert!(is_blank(&[]));
    }

    #[test]
    fn list_file_names_each_picture_with_its_size() {
        let lines = [
            list_line("universe.png", (1280, 720)),
            list_line("settings-narrow.png", (640, 720)),
        ];
        assert_eq!(
            list_text(&lines),
            "universe.png 1280x720\nsettings-narrow.png 640x720\n"
        );
    }

    #[test]
    fn unwritable_folder_fails_before_any_window() {
        // A path below a regular file cannot be created on any platform.
        let base = std::env::temp_dir().join(format!("screens-test-{}", std::process::id()));
        std::fs::create_dir_all(&base).expect("temp dir");
        let file = base.join("not-a-folder");
        std::fs::write(&file, b"x").expect("temp file");
        let target = file.join("screens");
        assert!(prepare_dir(&target).is_err());
        assert_eq!(
            run_screens(&target.to_string_lossy(), ShotSelection::default()),
            EXIT_FAIL
        );
        std::fs::remove_dir_all(&base).expect("clean temp dir");
    }

    #[test]
    fn writable_folder_is_prepared_and_left_empty() {
        let base = std::env::temp_dir().join(format!("screens-ok-{}", std::process::id()));
        prepare_dir(&base).expect("writable");
        assert_eq!(std::fs::read_dir(&base).expect("read").count(), 0);
        std::fs::remove_dir_all(&base).expect("clean temp dir");
    }

    #[test]
    fn driver_never_touches_the_settings_file() {
        let source = include_str!("screens.rs");
        let code = source.split("#[cfg(test)]").next().expect("code part");
        for word in [
            "persist",
            "settings_path",
            "load_or_default",
            "save_on_change",
        ] {
            assert!(!code.contains(word), "screens driver references {word}");
        }
    }
}
