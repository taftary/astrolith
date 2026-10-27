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
use universe_core::settings::{GroundMemory, Settings};
use universe_render::{
    CapturedShot, DivePlugin, PatchMaterialPlugin, ScreensPlugin, ScreensQueue, SettingsStore,
    ShotFile, ShotSelection, TimingRun, UniverseRenderPlugin, parse_shot_file, shot_count_for,
};

use crate::gpu::{FramePacingPlugin, gpu_timers, render_plugin, task_pool_plugin};
use crate::protocol::{EXIT_FAIL, EXIT_PASS, screens_fail, screens_ok};

/// Name of the list file written next to the pictures.
pub(crate) const LIST_FILE: &str = "screens.txt";

/// Name of the timing file of a `--timing` run (#819).
pub(crate) const TIMING_FILE: &str = "timing.txt";

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
pub(crate) fn run_screens(dir: &str, selection: ShotSelection, workers: Option<usize>) -> i32 {
    run_with(dir, selection, None, false, workers)
}

/// Runs the screens mode on a shot file (#863): the file is read and
/// checked before the window opens, and a wrong line ends the run with the
/// line and the problem.
pub(crate) fn run_shot_file(dir: &str, file: &str, timing: bool, workers: Option<usize>) -> i32 {
    let shots = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read shot file {file}: {e}"))
        .and_then(|text| parse_shot_file(&text).map_err(|e| format!("shot file {file}: {e}")));
    match shots {
        Ok(shots) => run_with(
            dir,
            ShotSelection::default(),
            Some(ShotFile(shots)),
            timing,
            workers,
        ),
        Err(reason) => {
            println!("{}", screens_fail(&reason));
            EXIT_FAIL
        }
    }
}

/// Runs the window on the built-in list (with `selection`) or `shot_file`,
/// with `workers` ground build threads when set (#994).
fn run_with(
    dir: &str,
    selection: ShotSelection,
    shot_file: Option<ShotFile>,
    timing: bool,
    workers: Option<usize>,
) -> i32 {
    let root = Path::new(dir);
    if let Err(reason) = prepare_dir(root) {
        println!("{}", screens_fail(&reason));
        return EXIT_FAIL;
    }
    let mut app = App::new();
    // A timing run may measure with vsync off (#920 T4, measuring aid):
    // below the vsync floor the cost differences show (ADR 0044).
    let no_vsync = timing && std::env::var("ASTROLITH_TIMING_NO_VSYNC").is_ok_and(|v| v == "1");
    // Shots may take another Ground memory limit (#920 T7, a developer
    // aid: `ASTROLITH_GROUND_MEMORY=8` shoots at 8 MB); default otherwise.
    let ground_memory = std::env::var("ASTROLITH_GROUND_MEMORY")
        .ok()
        .and_then(|id| GroundMemory::from_id(&id))
        .unwrap_or_default();
    app.insert_resource(SettingsStore(Settings {
        vsync: !no_vsync,
        ground_memory,
        ..Settings::default()
    }))
    .insert_resource(ScreensOut {
        dir: root.to_path_buf(),
        shown: dir.to_string(),
        lines: Vec::new(),
        expected: shot_file
            .as_ref()
            .map_or_else(|| shot_count_for(&selection), |file| file.0.len()),
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
            .set(render_plugin())
            .set(task_pool_plugin(workers)),
    )
    .add_plugins((
        UniverseRenderPlugin,
        PatchMaterialPlugin,
        DivePlugin,
        ScreensPlugin,
        FramePacingPlugin,
    ))
    .add_systems(Update, drain_queue);
    if let Some(file) = shot_file {
        app.insert_resource(file);
    }
    if timing {
        // Times each view before its picture (#819), at the pace of a
        // focused window: the run's window rarely has focus, and Bevy slows
        // an unfocused one to a low-power pace.
        app.insert_resource(TimingRun::default())
            .insert_resource(bevy::winit::WinitSettings::continuous());
        if gpu_timers() {
            app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin)
                .add_systems(Update, print_gpu_timers);
        }
    }
    match app.run() {
        AppExit::Success => EXIT_PASS,
        AppExit::Error(_) => EXIT_FAIL,
    }
}

/// After each timed shot, its GPU pass times on stderr (#920 T4, measuring
/// aid, not the stdout protocol): one `GPU <shot> <pass> <ms>` line per
/// timed pass (times in ms, plus shader invocation and primitive counts),
/// averaged over the diagnostics history.
fn print_gpu_timers(
    store: Res<bevy::diagnostic::DiagnosticsStore>,
    timing: Option<Res<TimingRun>>,
    mut seen: Local<usize>,
) {
    let Some(timing) = timing else {
        return;
    };
    while *seen < timing.lines.len() {
        let shot = timing
            .lines
            .get(*seen)
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("?");
        for diagnostic in store.iter() {
            let path = diagnostic.path().as_str();
            let wanted = path.ends_with("elapsed_gpu")
                || path.ends_with("_invocations")
                || path.ends_with("clipper_primitives_out");
            if wanted && let Some(value) = diagnostic.average() {
                eprintln!("GPU {shot} {path} {value:.2}");
            }
        }
        *seen += 1;
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
    timing: Option<Res<TimingRun>>,
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
    let result = failure.map_or_else(|| finish(&out), Err).and_then(|files| {
        // A `--timing` run (#819): one TIMING line per shot, printed and
        // written to `timing.txt` beside the pictures.
        let Some(timing) = timing.as_ref() else {
            return Ok(files);
        };
        for line in &timing.lines {
            println!("{line}");
        }
        let path = out.dir.join(TIMING_FILE);
        std::fs::write(&path, list_text(&timing.lines))
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        Ok(files + 1)
    });
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
    let path = dir.join(&shot.file);
    rgba.save(&path)
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(list_line(&shot.file, size))
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
            run_screens(&target.to_string_lossy(), ShotSelection::default(), None),
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
