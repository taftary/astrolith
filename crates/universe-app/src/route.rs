//! `--route <file> <dir>` driver (#1022 T2, Spec v3 R4): a real window
//! drives the camera along a route file and records every frame.
//!
//! The file is read and checked before the window opens; a wrong line ends
//! the run with the line and the problem. The window runs with default
//! settings plus the file's `set` lines (no settings file is read or
//! written), at a focused window's pace, 1600 x 900 unless `--size` says
//! otherwise. At the end it writes into `<dir>`:
//!
//! - `frames.txt`: one line per frame (seconds, scenario, interval, work,
//!   main, render, wait, in milliseconds);
//! - `route.txt`: the `ROUTE-RUN` line read from the window and the
//!   adapter, the launch time, the summary lines and one `ROUTE-LEG` line
//!   per planet leg.
//!
//! It prints `route.txt` and `ROUTE-OK`, or `ROUTE-FAIL <reason>`.

use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::window::WindowResolution;
use std::path::PathBuf;
use universe_core::coords::Level;
use universe_core::labels::window_title_for_level;
use universe_core::settings::Settings;
use universe_render::{
    DivePlugin, PatchMaterialPlugin, RoutePlugin, RouteRun, SettingsStore, TIMING_SIZE,
    UniverseRenderPlugin, frame_line, parse_route, summary, timing_header,
};

use crate::gpu::{FramePacingPlugin, render_plugin, task_pool_plugin};
use crate::options::WindowOptions;
use crate::protocol::{EXIT_FAIL, EXIT_PASS, route_fail, route_ok};
use crate::screens::run_facts;

/// Folder the run writes into.
#[derive(Resource, Debug)]
struct RouteOut(PathBuf);

/// Runs a route file into `dir` and returns the process exit code.
pub(crate) fn run_route(file: &str, dir: &str, options: WindowOptions) -> i32 {
    let route = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read route file {file}: {e}"))
        .and_then(|text| parse_route(&text).map_err(|e| format!("route file {file}: {e}")));
    let route = match route {
        Ok(route) => route,
        Err(reason) => {
            println!("{}", route_fail(&reason));
            return EXIT_FAIL;
        }
    };
    let root = PathBuf::from(dir);
    if let Err(e) = std::fs::create_dir_all(&root) {
        println!(
            "{}",
            route_fail(&format!("cannot create folder {dir}: {e}"))
        );
        return EXIT_FAIL;
    }
    let launched = std::time::Instant::now();
    let settings = Settings::parse(&route.settings);
    let size = options.size.unwrap_or(TIMING_SIZE);
    let mut app = App::new();
    app.insert_resource(SettingsStore(settings))
        .insert_resource(RouteRun::new(route).launched_at(launched))
        .insert_resource(RouteOut(root))
        .insert_resource(bevy::winit::WinitSettings::continuous())
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: window_title_for_level(Level::MIN),
                        resolution: WindowResolution::new(size.0, size.1)
                            .with_scale_factor_override(options.scale.unwrap_or(1.0)),
                        ..default()
                    }),
                    ..default()
                })
                .set(render_plugin())
                .set(task_pool_plugin(options.workers)),
        )
        .add_plugins((
            UniverseRenderPlugin,
            PatchMaterialPlugin,
            DivePlugin,
            RoutePlugin,
            FramePacingPlugin,
            universe_render::FrameCapPlugin,
        ))
        .add_systems(Last, finish_route);
    match app.run() {
        AppExit::Success => EXIT_PASS,
        AppExit::Error(_) => EXIT_FAIL,
    }
}

/// The `route.txt` lines of a finished run (pure).
fn report_lines(header: String, run: &RouteRun) -> Vec<String> {
    let mut lines = vec![
        header,
        format!("ROUTE-LAUNCH launch_ms={:.0}", run.launch_ms),
    ];
    lines.extend(summary(&run.route, &run.frames));
    lines.extend(run.legs.iter().cloned());
    lines
}

/// Writes the files and exits once the route has run.
fn finish_route(
    run: Res<RouteRun>,
    out: Res<RouteOut>,
    windows: Query<&Window>,
    adapter: Option<Res<RenderAdapterInfo>>,
    mut exit: MessageWriter<AppExit>,
    mut finished: Local<bool>,
) {
    if !run.done || *finished {
        return;
    }
    *finished = true;
    let header = timing_header(&run_facts(windows.iter().next(), adapter.as_deref())).replacen(
        "TIMING-RUN",
        "ROUTE-RUN",
        1,
    );
    let lines = report_lines(header, &run);
    let scenario = |leg: usize| run.route.legs.get(leg).map_or("?", |l| l.scenario.as_str());
    let mut frames = String::with_capacity(run.frames.len() * 48);
    for frame in &run.frames {
        frames.push_str(&frame_line(frame, scenario(frame.leg)));
        frames.push('\n');
    }
    let report = lines.join("\n") + "\n";
    let written = std::fs::write(out.0.join("frames.txt"), frames)
        .and_then(|()| std::fs::write(out.0.join("route.txt"), &report));
    match written {
        Ok(()) => {
            for line in &lines {
                println!("{line}");
            }
            println!(
                "{}",
                route_ok(run.frames.len(), &out.0.display().to_string())
            );
            exit.write(AppExit::Success);
        }
        Err(e) => {
            println!(
                "{}",
                route_fail(&format!("cannot write the route files: {e}"))
            );
            exit.write(AppExit::from_code(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_names_the_run_launch_summary_and_legs() {
        let route =
            parse_route("a L10 lat=0 lon=0 h=2 heading=0 for=1 scenario=ground\n").expect("reads");
        let run = RouteRun::new(route);
        let lines = report_lines("ROUTE-RUN size=1600x900".to_string(), &run);
        assert_eq!(lines[0], "ROUTE-RUN size=1600x900");
        assert_eq!(lines[1], "ROUTE-LAUNCH launch_ms=0");
        assert!(
            lines.iter().any(|l| l.starts_with("ROUTE-ALL frames=0")),
            "{lines:?}"
        );
    }
}
