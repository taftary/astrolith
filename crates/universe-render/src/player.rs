//! The player in the window: the dive to the player and the arrival (#607).
//!
//! [`Player`] holds where the body stands (`universe_core::player`): the
//! spot left behind in the settings when it belongs to this universe, else
//! the seeded spawn (Q1 B, Q2 A). The first frame's destination is the
//! player (Q4 A). The Go to player key and the Views button make it the
//! destination again (Q6 A), and pressing 1 away from the body flies there
//! first (Q7 A). When the dive arrives in the body's cell the camera glides
//! to just behind and above the body, looking at it (Q3 A): an exponential
//! approach that crosses the scale from a region down to a person in a few
//! seconds, then stays in dive mode, or hands the body over when 1 asked
//! for it. Everything here is `pub(crate)`.

use crate::Universe;
use crate::bindings::{Actions, Source, source};
use crate::input::{Autopilot, Flight, FlightMode, Navigation};
use crate::settings_ui::SettingsStore;
use bevy::prelude::*;
use universe_core::keymap::Action;
use universe_core::player::PlayerSpot;

/// Glide rate toward the arrival pose, per second: the gap shrinks by
/// `e^-rate` each second, so a region cell to a person takes about five.
pub(crate) const GLIDE_RATE: f64 = 3.0;

/// The glide ends within this share of a body height of the pose.
const GLIDE_DONE: f64 = 1e-3;

/// Where the player's body stands, and the dive to it (#607).
#[derive(Resource, Debug, Clone, PartialEq)]
pub(crate) struct Player {
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
}

impl Player {
    /// The player for universe `root`: the saved spot when it belongs to
    /// this universe, else the seeded spawn (or the root cell's middle if
    /// no branch reaches a region).
    #[must_use]
    pub(crate) fn for_root(root: u64, store: &SettingsStore) -> Player {
        let seeded = PlayerSpot::seeded(root).unwrap_or_else(PlayerSpot::at_root);
        let mut player = Player {
            root,
            spot: seeded,
            seeded,
            gliding: false,
            takeover: false,
        };
        player.follow(store);
        player
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

/// Marks the Views page's Go to player button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingGoPlayer;

/// Marks the Views page's Reset player button (S2).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingResetPlayer;

/// The player's systems and state (#607).
pub(crate) struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                player_input.before(crate::input::handle_input),
                player_glide
                    .after(crate::input::handle_input)
                    .before(crate::normal_mode::normal_mode_step),
                go_player_click,
            ),
        );
    }
}

/// Keeps the player for this universe, sets the first destination, and
/// reads the Go to player key and 1 pressed away from the body.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn player_input(
    mut commands: Commands,
    player: Option<ResMut<Player>>,
    universe: Res<Universe>,
    store: Res<SettingsStore>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut nav: ResMut<Navigation>,
    mut autopilot: ResMut<Autopilot>,
    mut flight: ResMut<Flight>,
) {
    let root = universe.root.0;
    let Some(mut player) = player.filter(|player| player.root == root) else {
        // First frame (or a new universe): the player arrives this frame,
        // and it is the first destination (Q4 A).
        let fresh = Player::for_root(root, &store);
        if nav.destination.is_none() {
            nav.destination = Some(fresh.spot.destination());
        }
        commands.insert_resource(fresh);
        return;
    };
    player.follow(&store);
    if nav.destination.is_none() {
        nav.destination = Some(player.spot.destination());
    }
    let actions = Actions {
        map: store.0.keymap,
        keys: &keys,
        buttons: &buttons,
    };
    if actions.just_pressed(Action::GoToPlayer) {
        nav.destination = Some(player.spot.destination());
    }
    // Mode keys count only with no modifier held, like `handle_input`
    // (Run, Save view and Bring a view back turn digits into speeds and
    // views).
    let modifier = actions.pressed(Action::Run)
        || actions.pressed(Action::SaveView)
        || actions.pressed(Action::RecallView);
    let take = actions.just_pressed(Action::NormalMode)
        && !modifier
        && flight.mode != FlightMode::Normal
        && !player.spot.is_near(&universe.path);
    if take {
        // Away from the body, 1 flies there first (Q7 A): the mode key is
        // used up here so it does not place a new body under the camera.
        if let Some(Source::Keys(first, second)) =
            source(store.0.keymap.binding(Action::NormalMode))
        {
            keys.clear_just_pressed(first);
            if let Some(second) = second {
                keys.clear_just_pressed(second);
            }
        }
        nav.destination = Some(player.spot.destination());
        flight.mode = FlightMode::Dive;
        *autopilot = Autopilot::Flying;
        player.takeover = true;
    }
}

/// Glides the camera to the arrival pose once the dive reaches the body's
/// cell, looking at the body; hands the body over when 1 asked for it.
pub(crate) fn player_glide(
    time: Res<Time>,
    player: Option<ResMut<Player>>,
    mut universe: ResMut<Universe>,
    mut nav: ResMut<Navigation>,
    autopilot: Res<Autopilot>,
    mut flight: ResMut<Flight>,
) {
    let Some(mut player) = player else {
        return;
    };
    let toward = player.is_destination(nav.destination);
    if *autopilot == Autopilot::Flying && toward {
        player.gliding = true;
    }
    let near = player.spot.is_near(&universe.path);
    if flight.mode != FlightMode::Dive || !toward {
        player.gliding = false;
        if flight.mode != FlightMode::Dive {
            player.takeover = false;
        }
    }
    if player.gliding && !near && *autopilot == Autopilot::Idle {
        // A dive input cancelled the journey before it arrived.
        player.gliding = false;
        player.takeover = false;
    }
    let (pose, look) = player.spot.arrival_pose(universe.root.0);
    nav.focus = (near && toward && flight.mode == FlightMode::Dive).then_some(look);
    if !(player.gliding && near) {
        return;
    }
    // On the planet the glide is the dive's own approach (#632): the gap
    // shrinks at the glide rate, held above the ground, down through the
    // shell levels to the pose in L14.
    let factor = (-GLIDE_RATE * f64::from(time.delta_secs())).exp();
    universe.approach(pose, factor);
    let next = universe.path.offset();
    let height = universe_core::body::body_height(player.spot.metre(universe.root.0));
    let gap =
        ((next[0] - pose[0]).powi(2) + (next[1] - pose[1]).powi(2) + (next[2] - pose[2]).powi(2))
            .sqrt();
    if gap <= GLIDE_DONE * height {
        universe.path.set_offset(pose);
        player.gliding = false;
        if player.takeover {
            player.takeover = false;
            flight.mode = FlightMode::Normal;
        }
    }
}

/// The Views page's Go to player (Q6 A) and Reset player (S2) buttons.
#[allow(clippy::type_complexity, reason = "two button queries, one per marker")]
pub(crate) fn go_player_click(
    player: Option<Res<Player>>,
    mut nav: ResMut<Navigation>,
    mut store: ResMut<SettingsStore>,
    go: Query<&Interaction, (Changed<Interaction>, With<SettingGoPlayer>)>,
    reset: Query<&Interaction, (Changed<Interaction>, With<SettingResetPlayer>)>,
) {
    let Some(player) = player else {
        return;
    };
    if go
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        nav.destination = Some(player.spot.destination());
    }
    if reset
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        store.0.player = None;
        nav.destination = Some(player.seeded.destination());
    }
}

/// Puts the player's body where the camera is now, so key 1 takes it
/// over in place (tests that exercise normal mode, #558-#561).
#[cfg(test)]
pub(crate) fn player_here(app: &mut App) {
    let universe = app.world().resource::<Universe>();
    let root = universe.root.0;
    let chain = universe.path.indices();
    // On the ground under the camera, like a body placed by key 1 (#558).
    let camera = universe.path.offset();
    let position = universe_core::body::level_ground(universe, camera).map_or(camera, |g| g.point);
    let spot = PlayerSpot::new(&chain, position, 0.0).unwrap_or_else(PlayerSpot::at_root);
    app.world_mut().insert_resource(Player {
        root,
        spot,
        seeded: spot,
        gliding: false,
        takeover: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_tests::headless_app;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    /// A headless app on fixed 50 ms frames, one frame in.
    fn app() -> App {
        let mut app = headless_app();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )));
        app.update();
        app
    }

    fn tap(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
    }

    /// Runs frames until the camera rests at the player's arrival pose.
    fn fly_to_the_player(app: &mut App) -> bool {
        for _ in 0..20_000 {
            app.update();
            let player = app.world().resource::<Player>().clone();
            let universe = app.world().resource::<Universe>();
            if player.spot.is_near(&universe.path)
                && !player.gliding
                && universe.path.offset() == player.spot.arrival_pose(universe.root.0).0
            {
                return true;
            }
        }
        false
    }

    #[test]
    fn space_flies_from_l1_to_the_player_and_ends_behind_and_above_the_body() {
        let mut app = app();
        tap(&mut app, KeyCode::Space);
        assert!(
            fly_to_the_player(&mut app),
            "the journey ends at the player"
        );
        let player = app.world().resource::<Player>().clone();
        let universe = app.world().resource::<Universe>();
        assert_eq!(universe.level().get(), 10, "on the player's planet");
        assert_eq!(universe.shown_level().get(), 14, "in the room shell (#632)");
        assert_eq!(
            app.world().resource::<Flight>().mode,
            FlightMode::Dive,
            "you stay in dive mode (Q3 A)"
        );
        let root = universe.root.0;
        let nav = app.world().resource::<Navigation>();
        assert_eq!(
            nav.focus,
            Some(player.spot.arrival_pose(root).1),
            "looking at the body"
        );
    }

    #[test]
    fn go_to_player_from_another_branch_rises_out_and_comes_back() {
        let mut app = app();
        let first = app.world().resource::<Player>().spot.chain()[0];
        // Into another root portal: off the player's way.
        let other = {
            let universe = app.world().resource::<Universe>();
            universe
                .open
                .points
                .iter()
                .enumerate()
                .filter(|(_, p)| p.kind == universe_core::r#gen::MarkerKind::Portal)
                .map(|(i, _)| u32::try_from(i).expect("budgeted"))
                .find(|&m| m != first)
                .expect("another portal")
        };
        assert!(
            app.world_mut()
                .resource_mut::<Universe>()
                .open(universe_core::nav::MarkerIndex(other))
        );
        tap(&mut app, KeyCode::KeyP);
        let player = app.world().resource::<Player>().clone();
        assert!(player.is_destination(app.world().resource::<Navigation>().destination));
        tap(&mut app, KeyCode::Space);
        assert!(
            fly_to_the_player(&mut app),
            "rises out, then dives to the player"
        );
    }

    #[test]
    fn pressing_1_far_away_flies_there_then_hands_over_the_body() {
        let mut app = app();
        tap(&mut app, KeyCode::Digit1);
        assert_eq!(
            app.world().resource::<Flight>().mode,
            FlightMode::Dive,
            "far away, 1 flies first (Q7 A)"
        );
        assert!(app.world().resource::<Player>().takeover);
        let mut normal = false;
        for _ in 0..20_000 {
            app.update();
            if app.world().resource::<Flight>().mode == FlightMode::Normal {
                normal = true;
                break;
            }
        }
        assert!(normal, "on arrival you are in the body");
        let player = app.world().resource::<Player>().clone();
        assert!(
            player
                .spot
                .is_near(&app.world().resource::<Universe>().path)
        );
    }

    /// Opens the player's way down to its cell.
    fn open_to_player(app: &mut App) {
        let chain = app.world().resource::<Player>().spot.chain().to_vec();
        let mut universe = app.world_mut().resource_mut::<Universe>();
        for marker in chain {
            assert!(universe.open(universe_core::nav::MarkerIndex(marker)));
        }
    }

    #[test]
    fn near_the_body_1_takes_it_over_where_it_stands() {
        let mut app = app();
        open_to_player(&mut app);
        let spot = app.world().resource::<Player>().spot;
        tap(&mut app, KeyCode::Digit1);
        assert_eq!(app.world().resource::<Flight>().mode, FlightMode::Normal);
        let body = app
            .world()
            .resource::<crate::normal_mode::NormalState>()
            .body;
        let moved = ((body.position[0] - spot.position[0]).powi(2)
            + (body.position[2] - spot.position[2]).powi(2))
        .sqrt();
        assert!(
            moved < universe_core::body::body_height(spot.metre(universe_core::nav::DEMO_SEED)),
            "the body is taken over where it stands"
        );
    }

    #[test]
    fn leaving_the_body_keeps_and_saves_its_spot_and_reset_brings_the_seeded_one_back() {
        let mut app = app();
        open_to_player(&mut app);
        let seeded = app.world().resource::<Player>().seeded;
        tap(&mut app, KeyCode::Digit1);
        // Walk a while, then leave the body.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        for _ in 0..40 {
            app.update();
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyW);
        tap(&mut app, KeyCode::Digit3);
        let player = app.world().resource::<Player>().clone();
        assert_ne!(player.spot, seeded, "the body stays where it was left");
        let saved = app.world().resource::<SettingsStore>().0.player;
        assert_eq!(
            saved,
            Some((player.root, player.spot)),
            "saved with the settings"
        );
        assert!(player.is_destination(Some(player.spot.destination())));
        // A new start with these settings finds the body there.
        let again = Player::for_root(player.root, app.world().resource::<SettingsStore>());
        assert_eq!(again.spot, player.spot);
        // Reset player: back to the seeded spot.
        let button = app
            .world_mut()
            .spawn((SettingResetPlayer, Interaction::Pressed))
            .id();
        app.update();
        app.update();
        assert_eq!(app.world().resource::<SettingsStore>().0.player, None);
        assert_eq!(app.world().resource::<Player>().spot, seeded);
        app.world_mut().despawn(button);
    }
}
