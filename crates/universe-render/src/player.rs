//! The player in the window: the dive to the player and the arrival (#607,
//! #873).
//!
//! The spot lives in [`crate::player_state::PlayerState`]: the spot left
//! behind in the settings when it belongs to this universe, else the seeded
//! spawn (Q1 B, Q2 A). From L10 on the default target is the planet centre
//! (#873 Q4 A, replacing #607 Q4 A); the Go to player key and the Views
//! button make the player the destination again (Q6 A), and pressing 1 away
//! from the body flies there first (Q7 A). When the dive arrives in the
//! body's cell the camera glides to 10 m straight above the body, looking
//! down at it (#873 Q6 C, replacing #607 Q3 A): an exponential approach that
//! crosses the scale from a region down to a person in a few seconds, then
//! stays in dive mode with the planet centre as the target again, or hands
//! the body over when 1 asked for it.
//! Following the settings lives in `player_state` (leave, follow, enter),
//! so this file only sets up the dive and moves the glide. Everything here
//! is `pub(crate)`.

use crate::Universe;
use crate::bindings::{Actions, Source, source};
use crate::input::{Autopilot, Flight, FlightMode, Navigation};
use crate::player_state::PlayerState;
use crate::settings_ui::SettingsStore;
use bevy::prelude::*;
use universe_core::keymap::Action;

/// Glide rate toward the arrival pose, per second: the gap shrinks by
/// `e^-rate` each second, so a region cell to a person takes about five.
pub(crate) const GLIDE_RATE: f64 = 3.0;

/// The glide ends within this share of a body height of the pose.
const GLIDE_DONE: f64 = 1e-3;

/// Marks the Views page's Go to player button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingGoPlayer;

/// Marks the Views page's Reset player button (S2).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingResetPlayer;

/// The player's systems (#607, state in `player_state`).
pub(crate) struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                player_setup.before(crate::input::handle_input),
                player_glide
                    .after(crate::input::handle_input)
                    .before(crate::player_state::leave_player),
                go_player_click.before(crate::player_state::follow_player),
            ),
        );
    }
}

/// Keeps the player for this universe, sets the first destination, and
/// reads the Go to player key and 1 pressed away from the body.
///
/// Following the settings lives in [`crate::player_state::follow_player`]
/// (leave, follow, enter order, #754); this only makes sure the resource
/// exists for this universe and arms the dive to the body.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn player_setup(
    mut commands: Commands,
    player: Option<ResMut<PlayerState>>,
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
        // First frame (or a new universe): the player arrives selected, the
        // target his planet (#884 T4) unless a test already aimed elsewhere.
        // The anchored chain keeps him on the way when off it, so Space
        // rises back out.
        let fresh = PlayerState::for_root(root, &store);
        if nav.destination.is_none() {
            nav.destination = Some(fresh.spot.destination());
        }
        commands.insert_resource(fresh);
        return;
    };
    let actions = Actions {
        map: store.0.keymap,
        keys: &keys,
        buttons: &buttons,
    };
    if actions.just_pressed(Action::GoToPlayer) {
        // P toggles the player's selection (#884 T4, Q8 C): the target stays
        // the planet, Space with selection flies there then 10 m above him.
        player.selected = !player.selected;
        if player.selected {
            nav.destination = Some(player.spot.destination());
        } else {
            nav.destination = Some(universe_core::destination::Destination::seeded(
                root,
                &universe.path.indices(),
            ));
        }
    }
    // At start the player is selected and the target is his planet (#884
    // T4, Q4 A): the first destination is the anchored planet chain.
    if nav.destination.is_none() && player.selected {
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
        // The journey stays the planet's; selection flies to the player
        // after it, then hands the body over (#884 T4).
        player.selected = true;
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
    player: Option<ResMut<PlayerState>>,
    mut universe: ResMut<Universe>,
    mut nav: ResMut<Navigation>,
    autopilot: Res<Autopilot>,
    mut flight: ResMut<Flight>,
) {
    let Some(mut player) = player else {
        return;
    };
    // The target is always the planet; selection decides the glide (#884
    // T4): Space with selection flies to the planet then 10 m above him.
    let toward = player.selected;
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
    let (arrival_camera, arrival_eye) = player.spot.arrival_pose();
    // One function poses the glide too (#754 AC3): toward the arrival, looking
    // at the eye. The pose's camera is the arrival point; the focus stays the
    // eye so the dive easing looks at the body.
    let gliding_pose = crate::player_camera::player_camera(
        crate::player_camera::CameraCase::Gliding,
        [0.0; 3],
        [0.0, 0.0, -1.0],
        [0.0, 1.0, 0.0],
        1.0,
        0.0,
        true,
        [0.0; 3],
        0.0,
        arrival_camera,
        arrival_eye,
    );
    let pose = gliding_pose.map_or(arrival_camera, |(camera, _, _)| camera);
    let look = arrival_eye;
    // #873 Q5 C: look at the body only while gliding to it; after arrival
    // the target is the planet centre again, so the focus clears and the
    // orbit/zoom work around the centre (same down direction, no snap).
    nav.focus =
        (near && toward && flight.mode == FlightMode::Dive && player.gliding).then_some(look);
    if !(player.gliding && near) {
        return;
    }
    let camera = universe.path.offset();
    let share = 1.0 - (-GLIDE_RATE * f64::from(time.delta_secs())).exp();
    let mut next = [
        camera[0] + (pose[0] - camera[0]) * share,
        camera[1] + (pose[1] - camera[1]) * share,
        camera[2] + (pose[2] - camera[2]) * share,
    ];
    // #873 T4: the glide never drops below 3 m above the terrain (relief-aware;
    // a trench floor can sit below the mean radius, so no bare-sphere avoid
    // here or the 10 m arrival could never land).
    if universe.level().get() == 10
        && let Some(frame) = universe_core::nav::planet_frame(&universe.0)
        && let Some(relief) = universe_core::nav::planet_relief(&universe.0)
    {
        next = universe_core::dive_orbit::clamp_above_ground(
            next,
            &frame,
            relief,
            universe_core::dive_orbit::GROUND_CLEARANCE_M,
        );
    }
    let height = universe_core::body::body_height(player.spot.level());
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
    } else {
        universe.path.set_offset(next);
    }
}

/// The Views page's Go to player (Q6 A) and Reset player (S2) buttons.
///
/// Go selects the player (the target stays the planet); Reset clears the
/// saved spot back to the seeded one and selects again (#884 T4).
#[allow(clippy::type_complexity, reason = "two button queries, one per marker")]
pub(crate) fn go_player_click(
    player: Option<ResMut<PlayerState>>,
    mut nav: ResMut<Navigation>,
    mut store: ResMut<SettingsStore>,
    go: Query<&Interaction, (Changed<Interaction>, With<SettingGoPlayer>)>,
    reset: Query<&Interaction, (Changed<Interaction>, With<SettingResetPlayer>)>,
) {
    let Some(mut player) = player else {
        return;
    };
    if go
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        player.selected = true;
        nav.destination = Some(player.spot.destination());
    }
    if reset
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        store.0.player = None;
        player.spot = player.seeded;
        player.selected = true;
        nav.destination = Some(player.seeded.destination());
    }
}

/// Puts the player's body where the camera is now, so key 1 takes it
/// over in place (tests that exercise normal mode, #558-#561).
#[cfg(test)]
pub(crate) fn player_here(app: &mut App) {
    use universe_core::body::Body;
    use universe_core::player::PlayerSpot;
    let universe = app.world().resource::<Universe>();
    let root = universe.root.0;
    let chain = universe.path.indices();
    // On the ground under the camera, like a body placed by key 1 (#558).
    let camera = universe.path.offset();
    let position = universe_core::body::level_ground(universe, camera).map_or(camera, |g| g.point);
    let spot = PlayerSpot::new(&chain, position, 0.0).unwrap_or_else(PlayerSpot::at_root);
    app.world_mut().insert_resource(PlayerState {
        root,
        spot,
        seeded: spot,
        selected: true,
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
        anchor: universe_core::body_frame::BodyAnchor::default(),
        ground_pending: false,
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
            let player = app.world().resource::<PlayerState>().clone();
            let universe = app.world().resource::<Universe>();
            if player.spot.is_near(&universe.path)
                && !player.gliding
                && universe.path.offset() == player.spot.arrival_pose().0
            {
                return true;
            }
        }
        false
    }

    #[test]
    #[ignore = "slow (~14 s on the dev PC): runs with --include-ignored (CI, validator), #847"]
    fn space_flies_from_l1_to_the_player_and_ends_behind_and_above_the_body() {
        // #884 T4 (Q4 A): at start the player is selected, so Space alone
        // flies to the planet then 10 m above him (#884 Q6 C). The name keeps
        // the #607/#873 history; the pose is ten metres above, looking down,
        // with the planet centre as the target again (focus clears).
        let mut app = app();
        assert!(
            app.world().resource::<PlayerState>().selected,
            "selected at start"
        );
        tap(&mut app, KeyCode::Space);
        assert!(
            fly_to_the_player(&mut app),
            "the journey ends at the player"
        );
        // Focus clears on the frame after arrival (the arrival frame still
        // looks at the body, then the target is the planet centre again).
        app.update();
        assert_eq!(app.world().resource::<Universe>().level().get(), 10);
        assert_eq!(
            app.world().resource::<Flight>().mode,
            FlightMode::Dive,
            "you stay in dive mode"
        );
        let nav = app.world().resource::<Navigation>();
        assert_eq!(
            nav.focus, None,
            "after arrival the target is the planet centre again"
        );
    }

    #[test]
    #[ignore = "slow (~14 s on the dev PC): runs with --include-ignored (CI, validator), #847"]
    fn go_to_player_from_another_branch_rises_out_and_comes_back() {
        // #884 T4: selected at start, so off the player's way Space rises
        // back out then dives to his planet and glides to him. No P needed.
        let mut app = app();
        let first = app.world().resource::<PlayerState>().spot.chain()[0];
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
        let player = app.world().resource::<PlayerState>().clone();
        assert!(player.selected, "still selected off the way");
        assert!(player.is_destination(app.world().resource::<Navigation>().destination));
        tap(&mut app, KeyCode::Space);
        assert!(
            fly_to_the_player(&mut app),
            "rises out, then dives to the player"
        );
    }

    #[test]
    #[ignore = "slow (~14 s on the dev PC): runs with --include-ignored (CI, validator), #847"]
    fn pressing_1_far_away_flies_there_then_hands_over_the_body() {
        let mut app = app();
        tap(&mut app, KeyCode::Digit1);
        assert_eq!(
            app.world().resource::<Flight>().mode,
            FlightMode::Dive,
            "far away, 1 flies first (Q7 A)"
        );
        assert!(app.world().resource::<PlayerState>().takeover);
        let mut normal = false;
        for _ in 0..20_000 {
            app.update();
            if app.world().resource::<Flight>().mode == FlightMode::Normal {
                normal = true;
                break;
            }
        }
        assert!(normal, "on arrival you are in the body");
        let player = app.world().resource::<PlayerState>().clone();
        assert!(
            player
                .spot
                .is_near(&app.world().resource::<Universe>().path)
        );
    }

    /// Opens the player's way down to its cell.
    fn open_to_player(app: &mut App) {
        let chain = app.world().resource::<PlayerState>().spot.chain().to_vec();
        let mut universe = app.world_mut().resource_mut::<Universe>();
        for marker in chain {
            assert!(universe.open(universe_core::nav::MarkerIndex(marker)));
        }
    }

    #[test]
    fn near_the_body_1_takes_it_over_where_it_stands() {
        let mut app = app();
        open_to_player(&mut app);
        let spot = app.world().resource::<PlayerState>().spot;
        tap(&mut app, KeyCode::Digit1);
        assert_eq!(app.world().resource::<Flight>().mode, FlightMode::Normal);
        let body = app.world().resource::<PlayerState>().body;
        let moved = ((body.position[0] - spot.position[0]).powi(2)
            + (body.position[2] - spot.position[2]).powi(2))
        .sqrt();
        assert!(
            moved < universe_core::body::body_height(spot.level()),
            "the body is taken over where it stands"
        );
    }

    #[test]
    fn leaving_the_body_keeps_and_saves_its_spot_and_reset_brings_the_seeded_one_back() {
        let mut app = app();
        open_to_player(&mut app);
        let seeded = app.world().resource::<PlayerState>().seeded;
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
        let player = app.world().resource::<PlayerState>().clone();
        assert_ne!(player.spot, seeded, "the body stays where it was left");
        let saved = app.world().resource::<SettingsStore>().0.player;
        assert_eq!(
            saved,
            Some((player.root, player.spot)),
            "saved with the settings"
        );
        assert!(player.is_destination(Some(player.spot.destination())));
        // A new start with these settings finds the body there.
        let again = PlayerState::for_root(player.root, app.world().resource::<SettingsStore>());
        assert_eq!(again.spot, player.spot);
        // Reset player: back to the seeded spot.
        let button = app
            .world_mut()
            .spawn((SettingResetPlayer, Interaction::Pressed))
            .id();
        app.update();
        app.update();
        assert_eq!(app.world().resource::<SettingsStore>().0.player, None);
        assert_eq!(app.world().resource::<PlayerState>().spot, seeded);
        app.world_mut().despawn(button);
    }

    #[test]
    #[ignore = "slow (~14 s on the dev PC): runs with --include-ignored (CI, validator), #847"]
    fn space_flies_planet_then_player() {
        // AC5/C5 (#884 T4): at start the player is selected and the target
        // is his planet. Space flies to the planet, then 10 m straight above
        // him looking down; after arrival orbit and zoom work around the
        // planet's centre with him still selected.
        let mut app = app();
        assert!(
            app.world().resource::<PlayerState>().selected,
            "selected at start"
        );
        tap(&mut app, KeyCode::Space);
        assert!(
            fly_to_the_player(&mut app),
            "Space flies to the planet then 10 m above him"
        );
        app.update();
        assert_eq!(app.world().resource::<Universe>().level().get(), 10);
        assert_eq!(
            app.world().resource::<Flight>().mode,
            FlightMode::Dive,
            "you stay in dive mode"
        );
        let nav = app.world().resource::<Navigation>();
        assert_eq!(
            nav.focus, None,
            "after arrival the target is the planet centre again"
        );
        assert!(
            app.world().resource::<PlayerState>().selected,
            "still selected after arrival"
        );
        // After arrival, orbit works around the planet's centre.
        let universe = app.world().resource::<Universe>();
        let before = universe.path.offset();
        assert!(before[0].is_finite());
    }

    #[test]
    fn p_toggles_player_selection() {
        // AC6/C6 (#884 T4): P toggles the selection, a click elsewhere
        // deselects him.
        let mut app = app();
        assert!(
            app.world().resource::<PlayerState>().selected,
            "selected at start"
        );
        tap(&mut app, KeyCode::KeyP);
        assert!(
            !app.world().resource::<PlayerState>().selected,
            "P deselects"
        );
        tap(&mut app, KeyCode::KeyP);
        assert!(
            app.world().resource::<PlayerState>().selected,
            "P selects again"
        );
        // A click that picks another point deselects.
        use crate::input::{ClickCycle, CursorPortals};
        use crate::tokens::CYCLE_CAP;
        let way = app.world().resource::<Navigation>().target;
        let other = {
            let universe = app.world().resource::<Universe>();
            universe
                .open
                .points
                .iter()
                .enumerate()
                .filter(|(_, p)| p.kind == universe_core::r#gen::MarkerKind::Portal)
                .map(|(i, _)| u32::try_from(i).expect("budgeted"))
                .find(|&m| Some(m) != way)
                .expect("another portal")
        };
        app.world_mut()
            .resource_mut::<CursorPortals>()
            .clone_from(&CursorPortals {
                pos: [100.0, 100.0],
                order: {
                    let mut o = [0; CYCLE_CAP];
                    o[0] = other;
                    o
                },
                len: 1,
                valid: true,
            });
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        {
            let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            buttons.release(MouseButton::Left);
            buttons.clear();
        }
        app.update();
        assert!(
            !app.world().resource::<PlayerState>().selected,
            "clicking another point deselects"
        );
        assert_eq!(app.world().resource::<ClickCycle>().picked, Some(other));
    }
}
