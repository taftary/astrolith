//! Read-only snapshots after production systems. No expected scenario values
//! are used to report player, camera, terrain, or UI state (#1227).

use crate::Universe;
use crate::input::Flight;
use crate::player_state::PlayerState;
use crate::scenario_file::Fixture;
use crate::scenario_run::{ScenarioRun, Snapshot};
use crate::scenario_setup::{ScenarioContext, chain_text};
use crate::scenario_values::{value, vector};
use bevy::math::DVec3;
use bevy::prelude::*;

fn prefix(chain: &[u32], depth: usize) -> String {
    chain
        .get(..depth)
        .map_or_else(|| "unavailable".into(), chain_text)
}

fn snapshot(world: &mut World) -> Result<Snapshot, String> {
    let mut s = Snapshot::new();
    let camera_pose = world
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(world)
        .ok()
        .copied();
    let has_moon_mesh = world
        .query_filtered::<Entity, With<crate::planet::PlanetMesh>>()
        .iter(world)
        .next()
        .is_some();
    let universe = world.resource::<Universe>();
    let player = world.resource::<PlayerState>();
    let flight = world.resource::<Flight>();
    let chain = if player.active {
        universe.path.indices()
    } else {
        player.spot.chain().to_vec()
    };
    let position = if player.active {
        player.body.position
    } else {
        player.spot.position
    };
    let level = universe_core::nest::path_level(&chain);
    let metres = 1.0 / universe_core::body::metre(level);
    let path = chain_text(&chain);
    let frame = format!("seed={}/path={path}/metres", player.root);
    value(&mut s, "seed", universe.root.0);
    value(&mut s, "player.id", format!("player-{}", player.root));
    value(&mut s, "player.active", player.active);
    value(&mut s, "player.mode", format!("{:?}", flight.mode));
    value(&mut s, "player.flying", player.active && player.body.flying);
    value(
        &mut s,
        "player.on_ground",
        player.active && player.body.on_ground,
    );
    value(&mut s, "player.third_person", player.third_person);
    value(&mut s, "player.path", &path);
    value(&mut s, "player.galaxy", prefix(&chain, 3));
    value(&mut s, "player.system", prefix(&chain, 7));
    value(&mut s, "player.planet", prefix(&chain, 9));
    value(&mut s, "player.level", level.get());
    value(&mut s, "player.position.units", "metres");
    vector(
        &mut s,
        "player.position",
        position.map(|v| v * metres),
        &frame,
    );
    value(&mut s, "player.surface_clearance_m", "unavailable");
    let surface = world.get_resource::<crate::patches::GroundSurface>();
    let same_cell = chain == universe.path.indices();
    let moon = same_cell
        .then(|| {
            universe
                .open
                .with_ids()
                .filter(|(_, p)| p.moon.is_some())
                .find_map(|(id, p)| {
                    let d = DVec3::from_array(position) - DVec3::from_array(p.position);
                    let gap = d.length() - p.radius;
                    (gap.abs() < p.radius * 0.01).then_some((id, gap))
                })
        })
        .flatten();
    let region = if let Some((id, gap)) = moon {
        value(&mut s, "player.moon", id);
        value(
            &mut s,
            "player.moon_state",
            if player.active && player.body.flying && !player.body.on_ground {
                "hovering"
            } else {
                "not-hovering"
            },
        );
        value(&mut s, "moon.reference_sphere_gap_m", gap * metres);
        value(
            &mut s,
            "moon.gap_kind",
            "geometric-reference-sphere-not-collision",
        );
        format!("moon/{path}/{id}")
    } else if same_cell && level.get() == 10 {
        if let Some(g) = crate::patches::ground_under(surface, universe, position) {
            let gap = (DVec3::from_array(position) - DVec3::from_array(g.point))
                .dot(DVec3::from_array(g.up))
                * metres;
            value(&mut s, "player.surface_clearance_m", gap);
        }
        let at = DVec3::from_array(position).length() * metres;
        let radius = universe_core::nav::planet_frame(universe).map_or(0.0, |f| f.radius_m());
        if (at - radius).abs() <= 100_000.0 {
            format!("planet/{path}")
        } else {
            format!("cell/{path}")
        }
    } else {
        format!("cell/{path}")
    };
    value(&mut s, "player.region", region);
    value(&mut s, "camera.path", chain_text(&universe.path.indices()));
    let camera_frame = format!(
        "seed={}/path={}/metres",
        universe.root.0,
        chain_text(&universe.path.indices())
    );
    let cm = 1.0 / universe_core::body::metre(universe.level());
    let origin = world.resource::<crate::style::RenderOrigin>();
    vector(
        &mut s,
        "camera.position",
        origin.camera.map(|v| v * cm),
        &camera_frame,
    );
    value(&mut s, "camera.position.units", "metres");
    let camera_ready = if let Some(camera) = camera_pose {
        let forward = camera.forward().as_vec3();
        vector(
            &mut s,
            "camera.up",
            camera.up().as_vec3().to_array().map(f64::from),
            &camera_frame.replace("metres", "direction"),
        );
        vector(
            &mut s,
            "camera.forward",
            [
                f64::from(forward.x),
                f64::from(forward.y),
                f64::from(forward.z),
            ],
            &camera_frame.replace("metres", "direction"),
        );
        true
    } else {
        crate::scenario_values::unavailable(
            &mut s,
            "camera.forward",
            "no-unique-production-camera-pose",
        );
        crate::scenario_values::unavailable(
            &mut s,
            "camera.up",
            "no-unique-production-camera-pose",
        );
        false
    };
    let context = world.resource::<ScenarioContext>();
    value(&mut s, "fixture.kind", context.kind.name());
    value(
        &mut s,
        "fixture.home.path",
        if context.home.is_empty() {
            "unavailable".into()
        } else {
            chain_text(&context.home)
        },
    );
    value(
        &mut s,
        "fixture.other.path",
        if context.other.is_empty() {
            "unavailable".into()
        } else {
            chain_text(&context.other)
        },
    );
    value(
        &mut s,
        "clock.game_s",
        world.resource::<crate::sky::SkyClock>().0.game_s,
    );
    value(
        &mut s,
        "clock.playing",
        world.resource::<crate::sky::SkyClock>().0.playing,
    );
    if let Fixture::Selected(start) = context.kind {
        value(
            &mut s,
            "fixture.selected.planet",
            chain_text(start.host.chain()),
        );
        value(
            &mut s,
            "fixture.selected.system",
            chain_text(start.host.system()),
        );
        value(
            &mut s,
            "fixture.placement_origin",
            "illustrative-preparation-not-travel",
        );
        value(
            &mut s,
            "fixture.direction_basis",
            if start.moon.is_some() {
                "moon-proxy-local-untranslated-host-cell-axes"
            } else {
                "planet-fixed-axes"
            },
        );
        if let Some(moon) = start.moon {
            value(&mut s, "fixture.selected.moon", moon.marker().0);
        }
        value(&mut s, "fixture.request.gap_m", start.gap.get());
        value(&mut s, "fixture.request.time_s", start.time.get());
        vector(
            &mut s,
            "fixture.request.direction",
            start.direction.components(),
            "declared-fixture-direction-not-observed-player-up",
        );
        if let Some(moon) = context.selected_moon {
            value(&mut s, "fixture.moon.type", moon.kind.name());
            value(&mut s, "fixture.moon.radius_km", moon.info.radius_km);
            value(&mut s, "fixture.moon.orbit_km", moon.info.orbit_km);
            value(&mut s, "fixture.moon.period_days", moon.info.period_days);
            value(&mut s, "fixture.moon.paint_seed", moon.paint_seed);
            value(&mut s, "fixture.moon.evidence", "observed-kind");
            value(&mut s, "fixture.moon.instance_origin", moon.origin.name());
            value(&mut s, "fixture.moon.terrain_origin", moon.terrain_origin());
            value(
                &mut s,
                "fixture.moon.property_origin",
                moon.property_origin(),
            );
            value(
                &mut s,
                "fixture.moon.property_source",
                moon.property_source(),
            );
        }
    }
    let player = world.resource::<PlayerState>();
    let universe = world.resource::<Universe>();
    let surface = world.get_resource::<crate::patches::GroundSurface>();
    let ground_ready = !context.ground_start
        || (!player.ground_pending
            && player.body.on_ground
            && crate::patches::ground_ready(surface, universe));
    let moon_ready = context.kind != Fixture::NearMoon
        || (moon.is_some() && player.body.flying && !player.body.on_ground && has_moon_mesh);
    let selected_ready = context.selected_moon.is_none_or(|selected| {
        universe.root == selected.identity.planet().seed()
            && universe.path.indices() == selected.identity.planet().chain()
            && universe
                .open
                .point_by_id(selected.identity.marker().0)
                .is_some_and(|p| p.moon.is_some())
            && has_moon_mesh
    });
    value(
        &mut s,
        "ready",
        camera_ready
            && ground_ready
            && moon_ready
            && selected_ready
            && (context.kind == Fixture::Loaded
                || world.resource::<Flight>().mode != crate::input::FlightMode::Normal
                || player.active),
    );
    value(
        &mut s,
        "screen",
        format!("{:?}", world.resource::<State<crate::menu::Screen>>().get()),
    );
    let panel = world.resource::<crate::panel::PanelState>();
    value(
        &mut s,
        "panel.selection",
        if panel.selected.is_empty() {
            "none"
        } else {
            &panel.selected
        },
    );
    for id in &panel.expanded {
        value(&mut s, &format!("panel.expanded.{id}"), true);
    }
    value(
        &mut s,
        "panel.scroll",
        world.resource::<crate::panel::PanelScrolled>().0,
    );
    value(
        &mut s,
        "panel.list_scroll",
        world.resource::<crate::panel::PanelListScrolled>().0,
    );
    let store = world.resource::<crate::SettingsStore>();
    value(&mut s, "settings.labels", store.0.labels);
    value(&mut s, "settings.debug_axes", store.0.debug_axes);
    let pending = world.resource::<crate::settings_ui::PendingWindow>();
    value(&mut s, "settings.fullscreen.enabled", !pending.fullscreen);
    value(&mut s, "settings.vsync.enabled", !pending.vsync);
    crate::scenario_ui::observe(world, &mut s);
    crate::production_diagnostic::observe(world, &mut s);
    crate::scenario_ground::observe(world, &mut s);
    crate::scenario_settings::observe(world, &mut s);
    crate::scenario_sky::observe(world, &mut s)?;
    crate::scenario_layout::observe(world, &mut s)?;
    if s.len() > 32_768 || s.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>() > 2_097_152 {
        return Err("snapshot limit reached (32768 values or 2 MiB text)".into());
    }
    Ok(s)
}

pub(crate) fn observe(world: &mut World) {
    world.resource_scope(|world, mut run: Mut<ScenarioRun>| {
        if !run.prepared || run.outcome.is_some() {
            return;
        }
        let mut current = match snapshot(world) {
            Ok(current) => current,
            Err(reason) => {
                run.terminal(Err(reason));
                return;
            }
        };
        if current.get("ready").is_some_and(|v| v == "true")
            && let Some(region) = current.get("player.region")
            && !run.visited.contains(region)
        {
            if run.visited.len() >= 4096 {
                run.terminal(Err("visitation observation limit reached".into()));
                return;
            }
            run.visited.push(region.clone());
        }
        let context = world.resource::<ScenarioContext>();
        for (label, path) in [("home", &context.home), ("other", &context.other)] {
            value(
                &mut current,
                &format!("fixture.{label}.visited"),
                if path.is_empty() {
                    "unavailable".to_owned()
                } else {
                    run.visited
                        .contains(&format!("planet/{}", chain_text(path)))
                        .to_string()
                },
            );
        }
        run.current = current;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_read_player_and_camera_separately_without_moving_either() {
        let mut world = World::new();
        world.init_resource::<crate::sky::SkyClock>();
        world.init_resource::<crate::panel::PanelState>();
        world.init_resource::<crate::panel::PanelScrolled>();
        world.init_resource::<crate::panel::PanelListScrolled>();
        world.init_resource::<crate::SettingsStore>();
        world.init_resource::<crate::settings_ui::PendingWindow>();
        world.insert_resource(State::new(crate::menu::Screen::Universe));
        world.init_resource::<crate::style::RenderOrigin>();
        let scenario = crate::parse_scenario(
            "scenario-v1\nseed 42\nsetup near-moon\ntimeout 30\nobserve start",
        )
        .unwrap();
        crate::scenario_setup::prepare(&mut world, &scenario).unwrap();
        world.spawn((Transform::default(), Camera3d::default()));
        let before = world.resource::<PlayerState>().clone();
        let first = snapshot(&mut world).unwrap();
        world.resource_mut::<crate::style::RenderOrigin>().camera = [1.0, 2.0, 3.0];
        world
            .resource_mut::<crate::panel::PanelState>()
            .select("developer");
        world.resource_mut::<crate::SettingsStore>().0.labels = false;
        let second = snapshot(&mut world).unwrap();
        assert_eq!(first.get("player.position"), second.get("player.position"));
        assert_ne!(first.get("camera.position"), second.get("camera.position"));
        assert_eq!(world.resource::<PlayerState>(), &before);
        assert_eq!(
            second.get("player.flying").map(String::as_str),
            Some("true")
        );
        assert!(second.contains_key("player.moon"));
        assert_eq!(
            second.get("player.surface_clearance_m").map(String::as_str),
            Some("unavailable")
        );
        assert_eq!(
            second.get("moon.gap_kind").map(String::as_str),
            Some("geometric-reference-sphere-not-collision")
        );
        assert_eq!(
            second.get("panel.selection").map(String::as_str),
            Some("developer")
        );
        assert_eq!(
            second.get("settings.labels").map(String::as_str),
            Some("false")
        );
    }
}
