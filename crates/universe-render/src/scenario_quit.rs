//! Bounded public Menu/Quit input and a read-only normal exit witness (#1238).
//! This module never writes AppExit, Interaction, popup state or settings.

use crate::menu::Screen;
use crate::quit_popup::{QuitChoice, QuitPopup};
use crate::scenario_run::ScenarioRun;
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuitStage {
    Menu,
    Confirm,
    Pressed,
}

pub(crate) fn drive(world: &mut World, run: &mut ScenarioRun) -> Result<bool, String> {
    if run.quit_stage.is_none() {
        if *world.resource::<State<Screen>>().get() != Screen::Menu
            || world.resource::<QuitPopup>().open
        {
            return Err("quit requires the Menu screen with its confirmation closed".into());
        }
        run.quit_stage = Some(QuitStage::Menu);
    }
    if run.entered.elapsed().as_secs_f64() >= 10.0 {
        return Err("public normal Quit did not complete within 10 seconds".into());
    }
    let id = match run.quit_stage {
        Some(QuitStage::Menu) => "menu-Quit",
        Some(QuitStage::Confirm) => "quit-Quit",
        _ => return Err("confirmation input did not produce a production Quit exit".into()),
    };
    run.trace(format!(
        "QUIT-CONTROL\t{id}\tinput_phase={}\tstep={}",
        run.click_phase, run.step
    ));
    let confirm_press = run.quit_stage == Some(QuitStage::Confirm) && run.click_phase == 1;
    if crate::scenario::click(world, run, id)? {
        run.quit_stage = Some(QuitStage::Confirm);
        run.click_phase = 0;
    } else if confirm_press {
        // The next production Update must hit the real confirmation and issue
        // the exit. Last observes it; no extra driver frame can manufacture one.
        run.quit_stage = Some(QuitStage::Pressed);
    }
    Ok(false)
}

/// Reads the actual production exit and hit-tested confirmation after Update.
pub(crate) fn observe_exit(
    mut exits: MessageReader<AppExit>,
    mut run: ResMut<ScenarioRun>,
    screen: Res<State<Screen>>,
    popup: Res<QuitPopup>,
    choices: Query<(&Interaction, &QuitChoice)>,
) {
    let mut seen = false;
    let mut success = true;
    for exit in exits.read() {
        seen = true;
        success &= matches!(exit, AppExit::Success);
    }
    if !seen || run.outcome.is_some() {
        return;
    }
    let normal = success
        && run.quit_stage == Some(QuitStage::Pressed)
        && *screen.get() == Screen::Menu
        && popup.open
        && choices.iter().any(|(interaction, choice)| {
            *choice == QuitChoice::Quit && *interaction == Interaction::Pressed
        });
    run.record("terminal");
    if normal {
        run.normal_quit_witnessed = true;
        run.trace(
            "NORMAL-QUIT\twitnessed=true\tsource=production-confirmation-input-and-exit".into(),
        );
        run.terminal(Ok(()));
    } else {
        run.terminal(Err(
            "application exited without the expected public Quit confirmation".into(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn world(interaction: Interaction) -> World {
        let mut world = World::new();
        let scenario =
            crate::parse_scenario("scenario-v2\nseed 42\nsetup loaded\ntimeout 60\nquit").unwrap();
        let mut run = ScenarioRun::new(scenario)
            .for_profile(crate::ScenarioProfileStart::Restart)
            .unwrap();
        run.quit_stage = Some(QuitStage::Pressed);
        world.insert_resource(run);
        world.insert_resource(State::new(Screen::Menu));
        world.insert_resource(QuitPopup { open: true });
        world.init_resource::<Messages<AppExit>>();
        world.spawn((interaction, QuitChoice::Quit));
        world
    }

    #[test]
    fn ordinary_exit_is_not_public_quit_but_the_production_confirmation_is() {
        let mut incomplete = world(Interaction::None);
        incomplete.write_message(AppExit::Success);
        incomplete.run_system_once(observe_exit).unwrap();
        let run = incomplete.resource::<ScenarioRun>();
        assert!(!run.normal_quit_witnessed());
        assert!(run.outcome.as_ref().unwrap().is_err());
        let mut complete = world(Interaction::Pressed);
        complete
            .run_system_once(crate::quit_popup::quit_choice_click)
            .unwrap();
        complete.run_system_once(observe_exit).unwrap();
        let run = complete.resource::<ScenarioRun>();
        assert!(run.normal_quit_witnessed());
        assert_eq!(run.outcome, Some(Ok(())));
        assert!(
            run.pending
                .iter()
                .any(|line| line.starts_with("NORMAL-QUIT\twitnessed=true"))
        );
    }
}
