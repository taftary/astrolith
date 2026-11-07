//! Explicit selected-body preparation, never a measured action (#1238).

use crate::Universe;
use crate::input::{Autopilot, Flight, FlightMode, Navigation};
use crate::player_state::PlayerState;
use crate::scenario_fixture::{Fixture, SelectedStart, StartMode};
use crate::scenario_setup::ScenarioContext;
use bevy::prelude::*;
use universe_core::body::Body;
use universe_core::destination::Destination;
use universe_core::moon_approach::{moon_reference_position, planet_relief_position};
use universe_core::player::PlayerSpot;

pub(crate) fn prepare(world: &mut World, start: SelectedStart) -> Result<(), String> {
    let seed = start.host.seed().0;
    let mut universe = Universe(universe_core::nav::Universe::new(seed));
    for &id in start.host.chain() {
        if !universe.0.open(universe_core::nav::MarkerIndex(id)) {
            return Err(format!("selected fixture cannot open marker {id}"));
        }
    }
    universe.0.advance(start.time.get());
    let position = match start.moon {
        Some(moon) => moon_reference_position(&universe, moon, start.direction, start.gap),
        None => planet_relief_position(&universe, start.host, start.direction, start.gap),
    }
    .map_err(|e| e.to_string())?
    .0;
    let fixture = Fixture::Selected(start);
    let yaw = start.look.yaw_rad();
    let spot = PlayerSpot::new(start.host.chain(), position, yaw)
        .ok_or("invalid selected fixture spot")?;
    let mut body = Body::standing(position, yaw);
    let solid_planet = start.moon.is_none()
        && universe_core::world_type::planet_type(seed, start.host.chain()).is_some();
    body.flying = !solid_planet;
    let up = universe_core::body::level_up(&universe, position);
    let forward = start.look.forward(up);
    let eye = universe_core::body::eye_of(&universe, &body).0;
    universe.path.set_offset(eye);
    let store = world
        .get_resource::<crate::SettingsStore>()
        .copied()
        .unwrap_or_default();
    let seeded = PlayerState::for_root(seed, &store).seeded;
    // Preparation can replace an entire context before normal input runs. Its
    // old collision view must not settle the new body onto the previous planet
    // during that first Update (patch synchronization runs later in Update).
    // Keep worker/cache/render resources intact: the production transition still
    // owns their retirement, and later measurement includes this preparation.
    if world
        .get_resource::<Universe>()
        .is_none_or(|old| old.open_seed() != universe.open_seed())
        && let Some(mut surface) = world.get_resource_mut::<crate::patches::GroundSurface>()
    {
        surface.0 = None;
        surface.1 = false;
        surface.2 = false;
        surface.3 = None;
    }
    world.insert_resource(PlayerState {
        root: seed,
        spot,
        seeded,
        body,
        // Free/dive leave this prepared body through the normal production leave
        // path on the next Update, not through a test-only persistence shortcut.
        active: true,
        selected: false,
        ground_pending: solid_planet,
        third_person: store.0.start_third_person,
        ..default()
    });
    world.insert_resource(Flight {
        mode: match start.mode {
            StartMode::Normal => FlightMode::Normal,
            StartMode::Free => FlightMode::Free,
            StartMode::Dive => FlightMode::Dive,
        },
        yaw,
        pitch: start.look.pitch_rad(),
        target_yaw: yaw,
        target_pitch: start.look.pitch_rad(),
        free_forward: forward,
        free_up: universe_core::flight::orthogonal_up(forward, up),
        ..default()
    });
    world.insert_resource(Navigation {
        destination: Some(
            Destination::anchored(spot.chain()).ok_or("invalid selected destination")?,
        ),
        forward,
        up,
        ..default()
    });
    world.insert_resource(Autopilot::default());
    world.insert_resource(universe);
    let mut clock = world.resource_mut::<crate::sky::SkyClock>();
    clock.0.game_s = start.time.get();
    clock.0.playing = false;
    world.insert_resource(ScenarioContext {
        kind: fixture,
        ground_start: fixture.needs_ground(),
        selected_moon: start
            .moon
            .map(universe_core::moon_catalog::moon_record)
            .transpose()
            .map_err(|e| e.to_string())?,
        home: universe_core::nest::autopilot_path(seed),
        other: Vec::new(),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_moon_is_repeatable_at_shared_time_and_never_fabricates_contact() {
        let host = universe_core::moon_catalog::PlanetIdentity::new(
            universe_core::nav::Seed(42),
            &universe_core::nest::autopilot_path(42),
        )
        .unwrap();
        let id = universe_core::moon_catalog::moons_for_planet(host).unwrap()[0]
            .identity
            .marker()
            .0;
        let chain = crate::scenario_setup::chain_text(host.chain());
        let fixture = crate::scenario_fixture::parse(
            42,
            &format!("moon {chain} {id} 0 1 0 10 86400 normal 45 -20")
                .split_whitespace()
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let Fixture::Selected(start) = fixture else {
            panic!("selected");
        };
        let mut world = World::new();
        world.init_resource::<crate::sky::SkyClock>();
        prepare(&mut world, start).unwrap();
        let first = world.resource::<PlayerState>().clone();
        assert!(first.body.flying && !first.body.on_ground && !first.ground_pending);
        assert_ne!(
            first.seeded, first.spot,
            "Reset keeps the production seeded spot"
        );
        assert_eq!(world.resource::<Universe>().time_s(), 86400.0);
        assert_eq!(world.resource::<crate::sky::SkyClock>().0.game_s, 86400.0);
        assert!(!world.resource::<crate::sky::SkyClock>().0.playing);
        prepare(&mut world, start).unwrap();
        assert_eq!(&first, world.resource::<PlayerState>());
        assert_eq!(world.resource::<Flight>().yaw, start.look.yaw_rad());
    }

    #[test]
    fn loaded_start_does_not_modify_gameplay_preferences_or_clock() {
        let mut app = crate::window_tests::headless_app();
        app.update();
        let world = app.world_mut();
        let before_player = world.resource::<PlayerState>().clone();
        let before_universe = world.resource::<Universe>().clone();
        let before_clock = *world.resource::<crate::sky::SkyClock>();
        let before_settings = *world.resource::<crate::SettingsStore>();
        let before_flight = *world.resource::<Flight>();
        crate::scenario_setup::prepare_fixture(world, 99, Fixture::Loaded).unwrap();
        assert_eq!(&before_player, world.resource::<PlayerState>());
        assert_eq!(&before_universe, world.resource::<Universe>());
        assert_eq!(&before_clock, world.resource::<crate::sky::SkyClock>());
        assert_eq!(&before_settings, world.resource::<crate::SettingsStore>());
        assert_eq!(&before_flight, world.resource::<Flight>());
    }
}
