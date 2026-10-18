//! One player state: the spot, the glide, and the body (#754).
//!
//! [`PlayerState`] merges the dive-side spot (`Player`, `spot`, `seeded`,
//! `gliding`, `takeover`) with the body-side view (`NormalState`, `body`,
//! third or first person, `active`, the pull readout). One resource, one
//! order: leaving saves the spot first, following the settings reads it
//! back, entering places the body. `Flight` (mode, yaw, pitch) and
//! `Navigation` (destination, focus, target) stay where they are: they
//! belong to every mode, not to the player.
//!
//! Order, every frame while in the universe view:
//! 1. `leave_player`: leaving normal mode parks the body and saves the spot.
//! 2. `follow_player`: the settings are read back (saved spot or seeded one).
//! 3. `enter_player`: entering normal mode places the body under the camera.
//!
//! Go to player, Reset player, and the Views reset read the same resource,
//! so they behave as today. Everything here is `pub(crate)`.

use crate::Universe;
use crate::input::{Flight, FlightMode, Navigation};
use crate::settings_ui::SettingsStore;
use bevy::prelude::*;
use universe_core::body::{Body, level_ground};
use universe_core::player::PlayerSpot;

/// Where the player's body stands, glides, and walks (#754, Q4 A).
///
/// The spot side (`root`, `spot`, `seeded`, `gliding`, `takeover`) is the
/// old `Player`; the body side (`body`, `third_person`, `active`, the pull,
/// the handover ease) is the old `NormalState`. One resource so saving on
/// leave and reading on enter have one clear order.
#[derive(Resource, Debug, Clone, PartialEq)]
pub(crate) struct PlayerState {
    /// Universe seed the spot belongs to.
    pub root: u64,
    /// Where the body stands.
    pub spot: PlayerSpot,
    /// The seeded spawn, kept for Reset player and the Views reset.
    pub seeded: PlayerSpot,
    /// The camera is on its way to the player and glides in on arrival.
    pub gliding: bool,
    /// Hand the body over when the glide ends (1 pressed far away, Q7 A).
    pub takeover: bool,
    /// The body, in the open cell's units.
    pub body: Body,
    /// Third person (true) or first person (false).
    pub third_person: bool,
    /// Whether the body was placed since normal mode was entered.
    pub active: bool,
    /// Camera point at takeover (the arrival point after a glide), while the
    /// handover eases to the body's own target (#754 AC2).
    pub handover_from: Option<[f64; 3]>,
    /// Seconds since takeover, `0.0` on the first body frame.
    pub handover_t: f64,
    /// Strongest pull on the body this frame, m/s2 (#559).
    pub pull_strength: f64,
    /// Unit direction toward the strongest pull, open-cell units.
    pub pull_toward: [f64; 3],
    /// Name of the body pulling hardest.
    pub pull_name: String,
}

impl Default for PlayerState {
    fn default() -> PlayerState {
        PlayerState {
            root: 0,
            spot: PlayerSpot::at_root(),
            seeded: PlayerSpot::at_root(),
            gliding: false,
            takeover: false,
            body: Body::standing([0.0; 3], 0.0),
            third_person: true,
            active: false,
            handover_from: None,
            handover_t: 0.0,
            pull_strength: 0.0,
            pull_toward: [0.0; 3],
            pull_name: String::new(),
        }
    }
}

impl PlayerState {
    /// The player for universe `root`: the saved spot when it belongs to
    /// this universe, else the seeded spawn (or the root cell's middle if
    /// no branch reaches a region).
    #[must_use]
    pub(crate) fn for_root(root: u64, store: &SettingsStore) -> PlayerState {
        let seeded = PlayerSpot::seeded(root).unwrap_or_else(PlayerSpot::at_root);
        let mut state = PlayerState {
            root,
            spot: seeded,
            seeded,
            gliding: false,
            takeover: false,
            body: Body::standing([0.0; 3], 0.0),
            third_person: true,
            active: false,
            handover_from: None,
            handover_t: 0.0,
            pull_strength: 0.0,
            pull_toward: [0.0; 3],
            pull_name: String::new(),
        };
        state.follow(store);
        state
    }

    /// Follows the settings: the saved spot when it belongs to this
    /// universe, else the seeded one (after a reset, S2).
    pub(crate) fn follow(&mut self, store: &SettingsStore) {
        let want = match store.0.player {
            Some((seed, spot)) if seed == self.root => spot,
            _ => self.seeded,
        };
        if self.spot != want {
            self.spot = want;
        }
    }

    /// Leaves the body at `position` facing `yaw` in the open cell of
    /// `universe`: its new spot, saved with the settings (Q5 A, S2).
    pub(crate) fn leave(
        &mut self,
        universe: &Universe,
        position: [f64; 3],
        yaw: f64,
        store: &mut SettingsStore,
    ) {
        if let Some(spot) = PlayerSpot::new(&universe.path.indices(), position, yaw) {
            self.spot = spot;
            store.0.player = Some((self.root, spot));
        }
    }

    /// Whether `destination` is this player's.
    #[must_use]
    pub(crate) fn is_destination(
        &self,
        destination: Option<universe_core::destination::Destination>,
    ) -> bool {
        destination == Some(self.spot.destination())
    }
}

/// Leaving normal mode parks the body: its new spot is saved first, before
/// anything reads the settings (step 1 of the leave, follow, enter order).
pub(crate) fn leave_player(
    universe: Res<Universe>,
    flight: Res<Flight>,
    mut store: ResMut<SettingsStore>,
    mut state: ResMut<PlayerState>,
) {
    // Reads the body, writes the spot: no allocation per frame
    // (`E-HOT-NOALLOC`).
    if flight.mode != FlightMode::Normal && state.active {
        // The body stays where it is left: its new spot, saved (#607,
        // Q5 A, S2).
        let position = state.body.position;
        let yaw = state.body.yaw;
        state.leave(&universe, position, yaw, &mut store);
    }
    if flight.mode != FlightMode::Normal {
        state.active = false;
        state.handover_from = None;
        state.handover_t = 0.0;
    }
}

/// Follows the settings: the saved spot when it belongs to this universe,
/// else the seeded one (step 2: reads what leaving saved).
pub(crate) fn follow_player(state: Option<ResMut<PlayerState>>, store: Res<SettingsStore>) {
    let Some(mut state) = state else {
        return;
    };
    state.follow(&store);
}

/// Entering normal mode places the body under the camera (step 3: reads
/// what following holds). Runs after the glide, so an arrival hands over in
/// the same frame it lands. The look pitch starts at the arrival tilt, so the
/// first body frame looks the same way as the last arrival frame, and the
/// position eases from the arrival point over [`crate::player_camera::HANDOVER_S`]
/// (#754 AC2).
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn enter_player(
    universe: Res<Universe>,
    mut flight: ResMut<Flight>,
    nav: Res<Navigation>,
    store: Res<SettingsStore>,
    mut state: ResMut<PlayerState>,
) {
    if flight.mode != FlightMode::Normal || state.active {
        return;
    }
    let eye = universe.path.offset();
    state.body = if state.spot.is_near(&universe.path) {
        // Near the body, 1 takes it over where it stands (#607, Q7 A).
        let mut body = Body::standing(state.spot.position, state.spot.yaw);
        body.flying = level_ground(&universe, state.spot.position).is_none();
        body
    } else {
        crate::normal_mode::place_body(&universe, eye, nav.forward)
    };
    state.third_person = store.0.start_third_person;
    state.active = true;
    // Still handover: the direction already matches the arrival, the position
    // eases from the arrival point (the current offset after a glide).
    state.handover_from = Some(universe.path.offset());
    state.handover_t = 0.0;
    flight.yaw = state.body.yaw;
    flight.pitch = crate::player_camera::arrival_tilt();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_tests::headless_app;

    #[test]
    fn player_state_saves_then_reads() {
        // #754 AC4: leaving saves first, following reads back.
        let mut app = headless_app();
        app.update();
        let root = app.world().resource::<Universe>().root.0;
        let mut state = PlayerState::for_root(root, app.world().resource::<SettingsStore>());
        // Enter, walk the spot elsewhere, leave: the save holds the new spot.
        state.active = true;
        state.body = Body::standing([0.11, 0.0, -0.07], 1.2);
        let universe = app.world().resource::<Universe>().clone();
        let mut store = *app.world().resource::<SettingsStore>();
        let spot = PlayerSpot::new(
            &universe.path.indices(),
            state.body.position,
            state.body.yaw,
        )
        .expect("spot");
        state.leave(&universe, state.body.position, state.body.yaw, &mut store);
        assert_eq!(state.spot, spot, "leaving parks the spot");
        assert_eq!(store.0.player, Some((root, spot)), "and saves it");
        // Following reads the saved spot back, even into a fresh state.
        let mut again = PlayerState::for_root(root, &store);
        // Fresh starts at the saved spot because the store holds it.
        assert_eq!(again.spot, spot, "a new start finds the left body");
        again.spot = PlayerSpot::at_root();
        again.follow(&store);
        assert_eq!(again.spot, spot, "following reads the save back");
        // Reset player: back to the seeded spot.
        store.0.player = None;
        again.follow(&store);
        assert_eq!(again.spot, again.seeded, "reset returns the seeded spawn");
    }
}
