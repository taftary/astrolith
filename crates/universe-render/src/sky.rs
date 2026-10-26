//! The sky's clock in the window (#981 T2): game time moves the star
//! system every frame.
//!
//! [`SkyClock`] holds the core [`Clock`]; [`advance_sky`] ticks it with the
//! frame's real time, moves the bodies to it, and opens a planet whose
//! moving gravity zone reaches the camera, carrying the heading into the
//! planet exactly like a dive open. Screenshot runs keep the clock still,
//! so every picture shows the seeded sky (#981 T6 adds a `time=` word).
//!
//! The time is kept between runs (#981 Q7 A, T5): [`restore_sky_time`]
//! starts the clock from the settings file, and [`keep_sky_time`] writes it
//! back on quit and every [`KEEP_EVERY_S`] seconds, so a closed window or a
//! crash loses little.

use bevy::prelude::*;
use universe_core::clock::Clock;
use universe_core::nav::DiveEvent;

use crate::Universe;
use crate::input::{Navigation, carry_into_child, target_on_the_way};

/// The sky's clock (#981): game time, speed step and play/pause.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SkyClock(pub Clock);

/// Real seconds between writes of the time to the settings file.
pub(crate) const KEEP_EVERY_S: f64 = 30.0;

/// Starts the clock where the last run left it (#981 T5).
pub(crate) fn restore_sky_time(store: Res<crate::SettingsStore>, mut clock: ResMut<SkyClock>) {
    clock.0.game_s = store.0.game_time_s;
}

/// Writes the clock's time into the settings on quit and every
/// [`KEEP_EVERY_S`] real seconds (#981 T5). Runs after the frame's quit
/// (`PostUpdate`), so the settings save that follows stores it.
pub(crate) fn keep_sky_time(
    time: Res<Time<Real>>,
    clock: Res<SkyClock>,
    mut exits: MessageReader<AppExit>,
    mut store: ResMut<crate::SettingsStore>,
    mut last: Local<f64>,
) {
    let now = time.elapsed_secs_f64();
    let quitting = exits.read().next().is_some();
    if !quitting && now - *last < KEEP_EVERY_S {
        return;
    }
    *last = now;
    if store.0.game_time_s != clock.0.game_s {
        store.0.game_time_s = clock.0.game_s;
    }
}

/// Ticks the clock and moves the star system to its time (#981 T2).
pub(crate) fn advance_sky(
    time: Res<Time>,
    mut clock: ResMut<SkyClock>,
    mut universe: ResMut<Universe>,
    mut nav: ResMut<Navigation>,
) {
    clock.0.tick(f64::from(time.delta_secs()));
    let game_s = clock.0.game_s;
    if game_s != universe.0.time_s() {
        universe.0.advance(game_s);
    }
    // Carried like a dive open (`input::handle_input`).
    if let Some(DiveEvent::Opened(_)) = universe.0.sync_zones() {
        if let Some(opened) = universe.0.path.entered() {
            carry_into_child(&mut nav, &opened);
        }
        nav.target = target_on_the_way(nav.destination, &universe.0.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_tests::headless_app;

    #[test]
    fn the_time_continues_from_the_last_run_and_is_kept_on_quit() {
        // #981 Q7 A, T5: start from the saved time; quitting writes it back.
        let mut app = headless_app();
        app.world_mut()
            .resource_mut::<crate::SettingsStore>()
            .0
            .game_time_s = 5_000.0;
        app.update();
        let started = app.world().resource::<SkyClock>().0.game_s;
        assert!(started >= 5_000.0, "restored: {started}");
        app.world_mut().resource_mut::<SkyClock>().0.game_s = 7_000.0;
        app.world_mut().write_message(AppExit::Success);
        app.update();
        let kept = app.world().resource::<crate::SettingsStore>().0.game_time_s;
        assert!(kept >= 7_000.0, "kept on quit: {kept}");
    }
}
