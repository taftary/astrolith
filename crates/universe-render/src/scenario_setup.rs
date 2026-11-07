//! Explicit preparation of actual player state, never a measured action (#1227/#1238).

use crate::Universe;
use crate::input::{Autopilot, Flight, FlightMode, Navigation};
use crate::player_state::PlayerState;
use crate::scenario_file::{Fixture, Scenario};
use bevy::prelude::*;
use universe_core::body::Body;
use universe_core::destination::Destination;
use universe_core::nav::MarkerIndex;
use universe_core::player::PlayerSpot;

/// Fixture identities retained for read-only observations, not asserted outcomes.
#[derive(Resource, Debug, Clone)]
pub(crate) struct ScenarioContext {
    pub kind: Fixture,
    pub home: Vec<u32>,
    pub other: Vec<u32>,
    pub ground_start: bool,
    pub selected_moon: Option<universe_core::moon_catalog::ListedMoon>,
}

fn other_world(root: u64, home: &[u32]) -> Result<Vec<u32>, String> {
    let parent = home.get(..2).ok_or("home chain has no galaxy parent")?;
    let cell = universe_core::nest::generate_cell(root, parent);
    for (index, point) in (0u32..).zip(&cell.points) {
        if point.kind != universe_core::r#gen::MarkerKind::Portal || Some(&index) == home.get(2) {
            continue;
        }
        let mut prefix = parent.to_vec();
        prefix.push(index);
        if let Some(world) = universe_core::world_type::list_worlds(root, &prefix, 32)
            .into_iter()
            .find(|w| w.kind.is_some())
        {
            return Ok(world.chain);
        }
    }
    Err("no solid fixture planet in another galaxy".into())
}

/// Opens a fresh fixed-seed context and places the *body*, not just a camera.
pub(crate) fn prepare(world: &mut World, scenario: &Scenario) -> Result<(), String> {
    prepare_fixture(world, scenario.seed, scenario.fixture)
}

pub(crate) fn prepare_fixture(
    world: &mut World,
    seed: u64,
    fixture: Fixture,
) -> Result<(), String> {
    match fixture {
        Fixture::Selected(start) => return crate::scenario_selected::prepare(world, start),
        Fixture::Loaded => {
            // Deliberately no placement, universe opening, clock or preference write.
            world.insert_resource(ScenarioContext {
                kind: fixture,
                home: Vec::new(),
                other: Vec::new(),
                ground_start: false,
                selected_moon: None,
            });
            return Ok(());
        }
        _ => (),
    }
    prepare_legacy(world, seed, fixture)
}

fn prepare_legacy(world: &mut World, seed: u64, fixture: Fixture) -> Result<(), String> {
    let home = universe_core::nest::autopilot_path(seed);
    if home.len() != 9 {
        return Err("fixture seed does not reach a planet".into());
    }
    let other = other_world(seed, &home)?;
    let chain = if fixture == Fixture::OtherPlanet {
        &other
    } else {
        &home
    };
    let depth = match fixture {
        Fixture::Planet | Fixture::OtherPlanet | Fixture::NearMoon => 9,
        Fixture::System => 7,
        Fixture::Interstellar => 5,
        Fixture::Intergalactic => 2,
        Fixture::Selected(_) | Fixture::Loaded => return Err("not a legacy fixture".into()),
    };
    let mut universe = Universe(universe_core::nav::Universe::new(seed));
    for &id in chain.iter().take(depth) {
        if !universe.0.open(MarkerIndex(id)) {
            return Err(format!("fixture cannot open marker {id}"));
        }
    }
    // Hold celestial time, not the player or input systems. This is preparation
    // only; a scenario may subsequently operate the production time controls.
    universe.0.advance(0.0);
    let surface = matches!(fixture, Fixture::Planet | Fixture::OtherPlanet);
    let position = if surface {
        let frame = universe_core::nav::planet_frame(&universe).ok_or("no planet frame")?;
        let relief = universe_core::nav::planet_relief(&universe).ok_or("no planet relief")?;
        frame.to_cell(universe_core::planet_frame::PlanetPos([
            frame.radius_m() + relief.height_m([1.0, 0.0, 0.0]) + 2.0,
            0.0,
            0.0,
        ]))
    } else if fixture == Fixture::NearMoon {
        let moon = universe
            .open
            .points
            .iter()
            .find(|p| p.moon.is_some())
            .ok_or("fixture planet has no moon")?;
        let gap = 10.0 * universe_core::body::metre(universe.level());
        [
            moon.position[0] + moon.radius + gap,
            moon.position[1],
            moon.position[2],
        ]
    } else {
        // A real open-cell location away from any selected body; no camera
        // playback runs after this one-time placement.
        [0.13, 0.21, 0.31]
    };
    let spot =
        PlayerSpot::new(&universe.path.indices(), position, 0.0).ok_or("invalid fixture spot")?;
    let mut body = Body::standing(position, 0.0);
    body.flying = !surface;
    // Opening the chain preserves the old root-space camera. Normal mode
    // renders from the body, but free/dive read the path offset on release.
    // Reconcile that context once during setup, never during measured input.
    let eye = universe_core::body::eye_of(&universe, &body).0;
    universe.path.set_offset(eye);
    let state = PlayerState {
        root: seed,
        spot,
        seeded: spot,
        body,
        active: true,
        selected: false,
        ground_pending: surface,
        ..default()
    };
    let up = universe_core::body::level_up(&universe, position);
    let forward = universe_core::body::heading_basis(up, 0.0).0;
    world.insert_resource(universe);
    world.insert_resource(state);
    world.insert_resource(Flight {
        mode: FlightMode::Normal,
        ..default()
    });
    world.insert_resource(Navigation {
        destination: Some(Destination::anchored(spot.chain()).ok_or("invalid destination")?),
        forward,
        up,
        ..default()
    });
    world.insert_resource(Autopilot::default());
    let mut clock = world.resource_mut::<crate::sky::SkyClock>();
    clock.0.game_s = 0.0;
    clock.0.playing = false;
    world.insert_resource(ScenarioContext {
        kind: fixture,
        home,
        other,
        ground_start: surface,
        selected_moon: None,
    });
    Ok(())
}

pub(crate) fn chain_text(chain: &[u32]) -> String {
    if chain.is_empty() {
        "root".into()
    } else {
        chain
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(".")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_scenario;

    #[test]
    fn all_starting_contexts_place_the_real_body_and_distinct_galaxies() {
        for fixture in [
            "planet",
            "planet-other",
            "near-moon",
            "system",
            "interstellar",
            "intergalactic",
        ] {
            let scenario = parse_scenario(&format!(
                "scenario-v1\nseed 42\nsetup {fixture}\ntimeout 30\nobserve start"
            ))
            .unwrap();
            let mut world = World::new();
            world.init_resource::<crate::sky::SkyClock>();
            prepare(&mut world, &scenario).unwrap();
            let state = world.resource::<PlayerState>();
            let universe = world.resource::<Universe>();
            assert!(state.active);
            assert_eq!(state.spot.chain(), universe.path.indices());
            assert_eq!(state.spot.position, state.body.position);
            let context = world.resource::<ScenarioContext>();
            assert_ne!(context.home[..3], context.other[..3]);
            let expected = match fixture {
                "system" => 8,
                "interstellar" => 6,
                "intergalactic" => 3,
                _ => 10,
            };
            assert_eq!(universe.level().get(), expected);
            if fixture == "near-moon" {
                assert!(state.body.flying && !state.ground_pending);
                let moon = universe
                    .open
                    .points
                    .iter()
                    .find(|p| p.moon.is_some())
                    .unwrap();
                let gap = (state.body.position[0] - moon.position[0]) - moon.radius;
                assert!((gap / universe_core::body::metre(universe.level()) - 10.0).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn releasing_a_fixture_camera_never_reuses_the_inherited_path_offset() {
        let scenario =
            parse_scenario("scenario-v1\nseed 42\nsetup planet\ntimeout 30\nobserve start")
                .unwrap();
        let mut world = World::new();
        world.init_resource::<crate::sky::SkyClock>();
        prepare(&mut world, &scenario).unwrap();
        let universe = world.resource::<Universe>();
        let state = world.resource::<PlayerState>();
        let eye = universe_core::body::eye_of(universe, &state.body).0;
        assert_eq!(universe.path.offset(), eye);
    }
}
