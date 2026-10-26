//! The sky's clock in the window (#981 T2): game time moves the star
//! system every frame.
//!
//! [`SkyClock`] holds the core [`Clock`]; [`advance_sky`] ticks it with the
//! frame's real time, moves the bodies to it, and opens a planet whose
//! moving gravity zone reaches the camera, carrying the heading into the
//! planet exactly like a dive open. Screenshot runs keep the clock still,
//! so every picture shows the seeded sky (#981 T6 adds a `time=` word).

use bevy::prelude::*;
use universe_core::clock::Clock;
use universe_core::nav::DiveEvent;

use crate::Universe;
use crate::input::{Navigation, carry_into_child, target_on_the_way};

/// The sky's clock (#981): game time, speed step and play/pause.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SkyClock(pub Clock);

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
