//! Isolated real-window scenarios (#1227), with explicit disposable production
//! persistence only for --scenario-profile (#1238). Memory-only stays unchanged.

use bevy::prelude::*;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use universe_render::{
    DivePlugin, PatchMaterialPlugin, SCENARIO_BYTES, ScenarioPlugin, ScenarioRun, ScenarioSystems,
    SettingsStore, UniverseRenderPlugin, parse_scenario,
};

use crate::gpu::{render_plugin, task_pool_plugin};
use crate::options::WindowOptions;
use crate::protocol::{EXIT_FAIL, EXIT_PASS, scenario_fail, scenario_ok};

#[derive(Resource)]
struct ScenarioOut {
    writer: BufWriter<std::fs::File>,
    problem: Option<String>,
}

// App::run moves the app into the window runner and leaves an empty App.
// Keep only the flushed outcome outside it; never read the consumed world.
#[derive(Resource, Debug, Clone, Default)]
struct Receipt(Arc<Mutex<Option<Result<(), String>>>>);

impl Receipt {
    fn set(&self, result: Result<(), String>) {
        if let Ok(mut value) = self.0.lock() {
            *value = Some(result);
        }
    }
}

/// Drains every frame so failures/outer timeouts retain the last issued action.
fn stream(
    mut run: ResMut<ScenarioRun>,
    mut out: ResMut<ScenarioOut>,
    receipt: Res<Receipt>,
    mut exit: MessageWriter<AppExit>,
) {
    if out.problem.is_some() {
        return;
    }
    for line in run.pending.drain(..) {
        if let Err(reason) = writeln!(out.writer, "{line}") {
            let problem = format!("cannot write scenario evidence: {reason}");
            receipt.set(Err(problem.clone()));
            out.problem = Some(problem);
            exit.write(AppExit::from_code(1));
            return;
        }
    }
    if let Err(reason) = out.writer.flush() {
        let problem = format!("cannot flush scenario evidence: {reason}");
        receipt.set(Err(problem.clone()));
        out.problem = Some(problem);
        exit.write(AppExit::from_code(1));
        return;
    }
    if let Some(outcome) = &run.outcome {
        receipt.set(outcome.clone());
        if !run.normal_quit_witnessed() {
            exit.write(if outcome.is_ok() {
                AppExit::Success
            } else {
                AppExit::from_code(1)
            });
        }
    }
}

fn read_scenario(file: &str) -> Result<String, String> {
    let file = std::fs::File::open(file).map_err(|e| format!("cannot open scenario: {e}"))?;
    let mut text = String::new();
    file.take(u64::try_from(SCENARIO_BYTES).unwrap_or(131_072) + 1)
        .read_to_string(&mut text)
        .map_err(|e| format!("cannot read scenario: {e}"))?;
    if text.len() > SCENARIO_BYTES {
        return Err("scenario exceeds 128 KiB".into());
    }
    Ok(text)
}

fn prepare(
    file: &str,
    root: &Path,
    profile: Option<&crate::scenario_profile::Profile>,
) -> Result<(ScenarioRun, ScenarioOut), String> {
    // Refuse even an empty existing directory. Never truncate prior evidence.
    std::fs::create_dir(root).map_err(|e| format!("need a new output directory: {e}"))?;
    let mut writer = BufWriter::new(
        std::fs::File::create_new(root.join("scenario.tsv")).map_err(|e| e.to_string())?,
    );
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    writeln!(
        writer,
        "SCENARIO-RUN\tversion=1\texecutable={}\tscenario={file}\tsettings={}",
        exe.display(),
        if profile.is_some() {
            "production-disposable-profile"
        } else {
            "defaults-no-file-io"
        }
    )
    .map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())?;
    if let Some(profile) = profile {
        writeln!(
            writer,
            "PERSISTENCE\tstart={:?}\tsettings={}\tcompletion=public-normal-quit",
            profile.start,
            profile.settings.display()
        )
        .map_err(|e| e.to_string())?;
    }
    let parsed = read_scenario(file).and_then(|text| {
        std::fs::write(root.join("fixture.scenario"), &text).map_err(|e| e.to_string())?;
        let run = ScenarioRun::new(parse_scenario(&text)?);
        match profile {
            Some(profile) => {
                let run = run.for_profile(profile.start)?;
                profile.claim()?;
                Ok(run)
            }
            None => Ok(run),
        }
    });
    match parsed {
        Ok(run) => Ok((
            run,
            ScenarioOut {
                writer,
                problem: None,
            },
        )),
        Err(reason) => {
            let _ = writeln!(writer, "{}", scenario_fail(&reason));
            let _ = writer.flush();
            Err(reason)
        }
    }
}

/// Starts only after strict parsing; malformed files do not open a window.
pub(crate) fn run_scenario(file: &str, directory: &str, options: WindowOptions) -> i32 {
    run(file, directory, options, None)
}

pub(crate) fn run_profile_scenario(
    file: &str,
    directory: &str,
    profile: &str,
    options: WindowOptions,
) -> i32 {
    let profile = match crate::scenario_profile::Profile::check(
        Path::new(profile),
        &crate::persist::settings_path(),
    ) {
        Ok(profile) => profile,
        Err(reason) => {
            println!("{}", scenario_fail(&reason));
            return EXIT_FAIL;
        }
    };
    run(file, directory, options, Some(profile))
}

fn run(
    file: &str,
    directory: &str,
    options: WindowOptions,
    profile: Option<crate::scenario_profile::Profile>,
) -> i32 {
    let (run, out) = match prepare(file, Path::new(directory), profile.as_ref()) {
        Ok(values) => values,
        Err(reason) => {
            println!("{}", scenario_fail(&reason));
            return EXIT_FAIL;
        }
    };
    let mut app = App::new();
    let receipt = Receipt::default();
    let settings = profile.as_ref().map_or_else(SettingsStore::default, |p| {
        SettingsStore(crate::persist::load_or_default(&p.settings))
    });
    app.insert_resource(settings)
        .insert_resource(receipt.clone())
        .insert_resource(run)
        .insert_resource(out)
        .insert_resource(bevy::winit::WinitSettings::continuous())
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Astrolith scenario".into(),
                        resolution: super::window_resolution(&options),
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
            ScenarioPlugin,
            universe_render::FrameCapPlugin,
            universe_render::AutoQualityPlugin,
        ))
        .add_systems(Last, stream.after(ScenarioSystems::Observe));
    if let Some(profile) = profile {
        app.insert_resource(crate::persist::SettingsPath(profile.settings))
            .add_systems(
                Last,
                crate::persist::save_on_change.before(ScenarioSystems::Observe),
            );
    }
    let app_exit = app.run();
    let outcome = receipt.0.lock().ok().and_then(|value| value.clone());
    let result = final_result(app_exit, outcome);
    match result {
        Ok(()) => {
            println!("{}", scenario_ok());
            EXIT_PASS
        }
        Err(reason) => {
            let line = scenario_fail(&reason);
            if let Ok(mut trace) = std::fs::OpenOptions::new()
                .append(true)
                .open(Path::new(directory).join("scenario.tsv"))
            {
                let _ = writeln!(trace, "{line}");
            }
            println!("{line}");
            EXIT_FAIL
        }
    }
}

fn final_result(exit: AppExit, outcome: Option<Result<(), String>>) -> Result<(), String> {
    match (exit, outcome) {
        (AppExit::Success, Some(Ok(()))) => Ok(()),
        (_, Some(Err(problem))) => Err(problem),
        _ => Err("application exited before scenario completion".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_files_retain_evidence_and_existing_output_is_not_overwritten() {
        let root =
            std::env::temp_dir().join(format!("astrolith-scenario-parser-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        let input = root.join("input.scenario");
        std::fs::write(&input, "scenario-v9\n").unwrap();
        let output = root.join("out");
        assert!(prepare(input.to_str().unwrap(), &output, None).is_err());
        let first = std::fs::read(output.join("scenario.tsv")).unwrap();
        assert!(String::from_utf8_lossy(&first).contains("SCENARIO-FAIL expected scenario-v1"));
        assert!(prepare(input.to_str().unwrap(), &output, None).is_err());
        assert_eq!(std::fs::read(output.join("scenario.tsv")).unwrap(), first);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn completion_survives_the_consumed_app_and_early_exit_never_succeeds() {
        let receipt = Receipt::default();
        let mut app = App::new();
        app.insert_resource(receipt.clone())
            .add_systems(Update, |receipt: Res<Receipt>| receipt.set(Ok(())));
        assert_eq!(app.run(), AppExit::Success);
        assert!(!app.world().contains_resource::<Receipt>());
        let observed = receipt.0.lock().unwrap().clone();
        assert_eq!(final_result(AppExit::Success, observed), Ok(()));
        assert!(final_result(AppExit::Success, None).is_err());
        assert!(final_result(AppExit::from_code(1), Some(Ok(()))).is_err());
        assert_eq!(
            final_result(AppExit::Success, Some(Err("write problem".into()))),
            Err("write problem".into())
        );
    }
}
