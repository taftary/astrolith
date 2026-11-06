//! Opt-in scenario orchestration. Only setup mutates gameplay state (#1227).

use crate::bindings::Source;
use crate::scenario_file::Step;
use crate::scenario_input as input;
use crate::scenario_run::{ScenarioRun, evaluate};
use bevy::prelude::*;

/// Ordering boundary used by the app's streaming evidence writer.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScenarioSystems {
    /// Read-only observations after production update/layout/camera systems.
    Observe,
}

/// Installs the real-input driver, only for explicit scenario launches.
pub struct ScenarioPlugin;

impl Plugin for ScenarioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, drive.before(bevy::input::InputSystems));
    }
}

fn finish(world: &mut World, run: &mut ScenarioRun, result: Result<(), String>) {
    input::release(world);
    run.record("terminal");
    run.terminal(result);
}

fn execute(world: &mut World, run: &mut ScenarioRun, step: &Step) -> Result<bool, String> {
    match step {
        Step::Input(source, down) => input::input(world, *source, *down)?,
        Step::Pointer(x,y) => input::pointer(world, Vec2::new(*x,*y))?,
        Step::Motion(x,y) => input::motion(world, Vec2::new(*x,*y)),
        Step::Scroll(x,y) => input::scroll(world, *x,*y)?,
        Step::Click(id) => {
            match run.click_phase {
                0 => { let point = crate::scenario_ui::click_point(world, id)?; input::pointer(world, point)?; }
                1 => {
                    // Resolve again: a layout/visibility change must not click
                    // the old place. Production UI hit-testing owns activation.
                    let point = crate::scenario_ui::click_point(world, id)?;
                    input::pointer(world, point)?;
                    input::input(world, Source::Mouse(MouseButton::Left), true)?;
                }
                _ => {
                    let pressed = run.current.get(&format!("ui.{id}.interaction")).is_some_and(|s| s == "Pressed");
                    input::input(world, Source::Mouse(MouseButton::Left), false)?;
                    if !pressed { return Err(format!("button {id} did not receive the press (occluded or inactive)")); }
                    return Ok(true);
                }
            }
            run.click_phase += 1;
            return Ok(false);
        }
        Step::Observe(name) => {
            if run.baselines.len() >= 64 || run.baselines.contains_key(name) { return Err("baseline names must be unique, maximum 64".into()); }
            run.record(name);
            run.baselines.insert(name.clone(), run.current.clone());
        }
        Step::Expect(check) => evaluate(check, &run.current, &run.baselines)?,
        Step::Wait(check, timeout) => {
            if let Err(reason) = evaluate(check, &run.current, &run.baselines) {
                if run.entered.elapsed().as_secs_f64() >= *timeout { return Err(format!("wait timeout: {reason}")); }
                return Ok(false);
            }
        }
        Step::Hold(seconds) => return Ok(run.entered.elapsed().as_secs_f64() >= *seconds),
    }
    Ok(true)
}

fn drive(world: &mut World) {
    world.resource_scope(|world, mut run: Mut<ScenarioRun>| {
        if run.outcome.is_some() { return; }
        if run.timed_out(run.launched.elapsed().as_secs_f64()) {
            finish(world, &mut run, Err("total timeout".into()));
            return;
        }
        if !run.prepared {
            let header = format!("PHASE\tsetup\tseed={}\tfixture={:?}",run.scenario.seed,run.scenario.fixture);
            run.pending.push(header);
            match crate::scenario_setup::prepare(world, &run.scenario) {
                Ok(()) => run.prepared = true,
                Err(reason) => finish(world, &mut run, Err(format!("setup: {reason}"))),
            }
            return;
        }
        if !run.started {
            if run.current.get("ready").is_none_or(|v| v != "true") { return; }
            run.record("setup");
            run.pending.push("PHASE\tactions".into());
            run.started = true;
            run.entered = std::time::Instant::now();
        }
        let Some(step) = run.scenario.steps.get(run.step).cloned() else {
            finish(world, &mut run, Ok(()));
            return;
        };
        if !run.logged {
            let n = run.step;
            run.pending.push(format!("ACTION\t{n}\t{step:?}"));
            run.logged = true;
        }
        match execute(world, &mut run, &step) {
            Ok(true) => run.advance(),
            Ok(false) => (),
            Err(reason) => { let n = run.step; finish(world, &mut run, Err(format!("step {n}: {reason}"))); }
        }
    });
}
