//! Actual timed sky positions, light and production occlusion inputs (#1238).
//! Visibility flags below are explicitly not an occlusion/readback verdict.

use crate::scenario_run::Snapshot;
use crate::scenario_values::{unavailable, value, vector};
use bevy::prelude::*;
use universe_core::coords::{OpenUnits, ParentUnits};
use universe_core::sky_observation::SkyGeometry;

fn body(
    snapshot: &mut Snapshot,
    key: &str,
    identity: &str,
    geometry: Option<SkyGeometry>,
    frame: &str,
    metres: f64,
) {
    value(snapshot, &format!("{key}.identity"), identity);
    let Some(g) = geometry else {
        unavailable(
            snapshot,
            &format!("{key}.position"),
            "no-finite-production-body-geometry",
        );
        return;
    };
    vector(
        snapshot,
        &format!("{key}.position"),
        g.centre.0.map(|v| v * metres),
        frame,
    );
    value(snapshot, &format!("{key}.radius_m"), g.radius_m);
    value(snapshot, &format!("{key}.distance_m"), g.distance_m);
    value(
        snapshot,
        &format!("{key}.angular_diameter_deg"),
        g.angular_diameter_deg,
    );
    match g.toward {
        Some(dir) => vector(
            snapshot,
            &format!("{key}.toward"),
            dir.0,
            &frame.replace("metres", "direction"),
        ),
        None => unavailable(
            snapshot,
            &format!("{key}.toward"),
            "observer-at-body-centre",
        ),
    }
    unavailable(
        snapshot,
        &format!("{key}.visible"),
        "depth-occlusion-not-read-back-use-production-inputs",
    );
}

pub(crate) fn observe(world: &mut World, snapshot: &mut Snapshot) -> Result<(), String> {
    let universe = world.resource::<crate::Universe>();
    let clock = world.resource::<crate::sky::SkyClock>().0;
    value(snapshot, "clock.speed", clock.speed());
    value(snapshot, "sky.game_s", universe.time_s());
    value(
        snapshot,
        "sky.source",
        "current-production-open-and-parent-cells-shared-clock",
    );
    let chain = universe.path.indices();
    let frame = format!(
        "seed={}/path={}/metres",
        universe.root.0,
        crate::scenario_setup::chain_text(&chain)
    );
    let metres = 1.0 / universe_core::body::metre(universe.level());
    let camera = world.resource::<crate::style::RenderOrigin>().camera;
    vector(
        snapshot,
        "sky.observer.position",
        camera.map(|v| v * metres),
        &frame,
    );
    // Unit-tagged core inputs stay in open units. Text position output uses metres.
    let geometry = |at: [f64; 3], radius: f64| {
        SkyGeometry::new(OpenUnits(at), OpenUnits(camera), radius, metres)
    };
    let mut count = 0usize;
    unavailable(
        snapshot,
        "sky.sun.position",
        "no-central-production-star-in-parent-cell",
    );
    if let Some(pf) = universe_core::nav::planet_frame(universe) {
        count += 1;
        let host = format!(
            "seed={}/planet={}",
            universe.root.0,
            crate::scenario_setup::chain_text(&chain)
        );
        body(
            snapshot,
            "sky.host",
            &host,
            geometry([0.0; 3], pf.radius_cells()),
            &frame,
            metres,
        );
        value(
            snapshot,
            "sky.host.source",
            "actual-surface-reference-radius-not-exact-relief-occluder",
        );
        for (id, point) in universe.open.with_ids().filter(|(_, p)| p.moon.is_some()) {
            count += 1;
            if count > 512 {
                return Err("sky body observation limit 512 reached".into());
            }
            let key = format!("sky.moon.{id}");
            body(
                snapshot,
                &key,
                &format!("{host}/moon={id}"),
                geometry(point.position, point.radius),
                &frame,
                metres,
            );
            value(
                snapshot,
                &format!("{key}.source"),
                "current-production-open-cell-moon-reference-sphere-not-proxy-pose",
            );
            value(
                snapshot,
                &format!("{key}.radius_source"),
                "generated-physical-moon-radius",
            );
        }
        if let Some(relief) = universe_core::nav::planet_relief(universe) {
            value(
                snapshot,
                "sky.occlusion.terrain_cull_radius_m",
                pf.radius_m() + relief.height_bounds_m().0,
            );
            value(
                snapshot,
                "sky.occlusion.terrain_cull_source",
                "production-lowest-relief-horizon-selection-input-not-visible-result",
            );
        }
        if let (Some(parent), Some(entered)) = (&universe.parent, universe.path.entered()) {
            for (id, point) in parent
                .with_ids()
                .filter(|(_, p)| p.star.is_some() || p.planet.is_some())
            {
                if id == entered.marker {
                    continue;
                }
                count += 1;
                if count > 512 {
                    return Err("sky body observation limit 512 reached".into());
                }
                let at =
                    universe_core::nav::sibling_in_open_units(entered, ParentUnits(point.position))
                        .0
                        .0;
                let kind = if point.star.is_some() {
                    "star"
                } else {
                    "planet"
                };
                let key = format!("sky.parent.{kind}.{id}");
                let identity = format!(
                    "seed={}/path={}/marker={id}",
                    universe.root.0,
                    crate::scenario_setup::chain_text(chain.get(..8).unwrap_or(&[]))
                );
                body(
                    snapshot,
                    &key,
                    &identity,
                    geometry(at, point.radius / entered.ratio),
                    &frame,
                    metres,
                );
                value(
                    snapshot,
                    &format!("{key}.source"),
                    "production-parent-sibling-mapping",
                );
                value(
                    snapshot,
                    &format!("{key}.radius_source"),
                    if point.star.is_some() {
                        "production-star-body-display-proxy-not-catalog-photosphere"
                    } else {
                        "generated-physical-planet-radius"
                    },
                );
                if point.star.is_some() && point.position == [0.0; 3] {
                    body(
                        snapshot,
                        "sky.sun",
                        &identity,
                        geometry(at, point.radius / entered.ratio),
                        &frame,
                        metres,
                    );
                    value(
                        snapshot,
                        "sky.sun.radius_source",
                        "production-star-body-display-proxy-not-catalog-photosphere",
                    );
                }
            }
        }
    } else {
        unavailable(
            snapshot,
            "sky.host.position",
            "not-in-a-production-surface-cell",
        );
    }
    value(snapshot, "sky.body_count", count);
    value(
        snapshot,
        "sky.occlusion.source",
        "production-body-spheres-ground-triangle-camera-projection-and-horizon-inputs",
    );
    unavailable(
        snapshot,
        "sky.occlusion.pixel_visibility",
        "no-production-depth-readback-or-visibility-query",
    );
    match universe_core::light::sun_direction(universe) {
        Some(dir) => vector(
            snapshot,
            "sky.sun_direction",
            dir,
            &frame.replace("metres", "direction"),
        ),
        None => unavailable(
            snapshot,
            "sky.sun_direction",
            "no-production-sun-direction-in-this-cell",
        ),
    }
    if let Some(state) = world.get_resource::<crate::air::AirState>() {
        value(snapshot, "scene.atmosphere.active", state.air.is_some());
        value(snapshot, "scene.atmosphere.wait_frames", state.wait);
        value(snapshot, "scene.atmosphere.space", state.space);
        if let Some(seed) = state.seed {
            value(snapshot, "scene.atmosphere.recipe_seed", seed);
        } else {
            unavailable(
                snapshot,
                "scene.atmosphere.recipe_seed",
                "no-active-production-air-recipe",
            );
        }
        if let Some(handle) = &state.medium {
            value(
                snapshot,
                "scene.atmosphere.medium_asset",
                format!("{:?}", handle.id()),
            );
        }
        if let Some((air, _)) = state.air {
            value(
                snapshot,
                "scene.atmosphere.inner_radius_m",
                air.inner_radius_m,
            );
            value(
                snapshot,
                "scene.atmosphere.outer_radius_m",
                air.outer_radius_m,
            );
        }
    } else {
        unavailable(
            snapshot,
            "scene.atmosphere.active",
            "production-atmosphere-resource-absent",
        );
    }
    proxies(world, snapshot)?;
    // Query actual transforms after sync_sun, not just the desired core direction.
    let mut suns = world.query_filtered::<(&Transform, &DirectionalLight), With<crate::sun::Sun>>();
    if let Ok((pose, lamp)) = suns.single(world) {
        vector(
            snapshot,
            "light.from_sun",
            pose.forward().as_vec3().to_array().map(f64::from),
            &frame.replace("metres", "direction"),
        );
        value(snapshot, "light.illuminance_lux", lamp.illuminance);
        value(snapshot, "light.shadows", lamp.shadow_maps_enabled);
    } else {
        unavailable(snapshot, "light.from_sun", "no-unique-production-sun-lamp");
    }
    let mut cameras = world.query_filtered::<&Projection, With<Camera3d>>();
    if let Ok(Projection::Perspective(p)) = cameras.single(world) {
        value(snapshot, "sky.camera.fov_rad", p.fov);
        value(snapshot, "sky.camera.aspect_ratio", p.aspect_ratio);
        let render_metres = metres / world.resource::<crate::style::RenderOrigin>().scale();
        value(
            snapshot,
            "sky.camera.near_m",
            f64::from(p.near) * render_metres,
        );
        value(
            snapshot,
            "sky.camera.far_m",
            f64::from(p.far) * render_metres,
        );
    } else {
        unavailable(
            snapshot,
            "sky.camera.fov_rad",
            "no-unique-perspective-production-camera",
        );
    }
    Ok(())
}

fn proxies(world: &mut World, snapshot: &mut Snapshot) -> Result<(), String> {
    let mut query = world.query::<(
        Entity,
        &crate::planet::PlanetMesh,
        &crate::style::Anchored,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
    )>();
    let proxies: Vec<_> = query
        .iter(world)
        .take(513)
        .map(|(entity, owner, at, mesh, material)| {
            (entity, *owner, at.0, mesh.0.clone(), material.0.clone())
        })
        .collect();
    if proxies.len() > 512 {
        return Err("moon proxy observation limit 512 reached".into());
    }
    value(snapshot, "sky.proxy_count", proxies.len());
    for (entity, owner, at, mesh, material) in proxies {
        let key = format!("sky.proxy.{}", entity.to_bits());
        value(
            snapshot,
            &format!("{key}.source"),
            "actual-production-proxy-entity-not-predicted-orbit-or-ground",
        );
        value(snapshot, &format!("{key}.marker"), owner.marker.0);
        value(snapshot, &format!("{key}.world_type"), owner.kind.name());
        value(snapshot, &format!("{key}.paint_seed"), owner.paint_seed);
        value(snapshot, &format!("{key}.mesh"), format!("{:?}", mesh.id()));
        value(
            snapshot,
            &format!("{key}.material"),
            format!("{:?}", material.id()),
        );
        if let Some(host) = owner.host {
            let path = crate::scenario_setup::chain_text(host.chain());
            value(
                snapshot,
                &format!("{key}.identity"),
                format!(
                    "seed={}/planet={path}/moon={}",
                    host.seed().0,
                    owner.marker.0
                ),
            );
            let metres =
                1.0 / universe_core::body::metre(universe_core::nest::path_level(host.chain()));
            vector(
                snapshot,
                &format!("{key}.position"),
                at.map(|v| v * metres),
                &format!("seed={}/path={path}/metres", host.seed().0),
            );
        } else {
            unavailable(
                snapshot,
                &format!("{key}.identity"),
                "proxy-has-no-validated-surface-owner",
            );
        }
        if let Some(asset) = world
            .get_resource::<Assets<StandardMaterial>>()
            .and_then(|a| a.get(&material))
        {
            value(
                snapshot,
                &format!("{key}.material_state"),
                format!("{:?}", asset),
            );
        } else {
            unavailable(
                snapshot,
                &format!("{key}.material_state"),
                "proxy-material-not-in-main-world-store",
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sky_reads_current_timed_cells_and_never_turns_inputs_into_pixel_visibility() {
        let mut world = World::new();
        let mut universe = universe_core::nav::Universe::new(42);
        for &marker in &universe_core::nest::autopilot_path(42) {
            assert!(universe.open(universe_core::nav::MarkerIndex(marker)));
        }
        universe.advance(3600.0);
        let moon_before = universe.open.point_by_id(1).unwrap().position;
        let path_before = universe.path.offset();
        world.insert_resource(crate::Universe(universe));
        world.insert_resource(crate::sky::SkyClock(universe_core::clock::Clock {
            game_s: 3600.0,
            playing: false,
            ..default()
        }));
        world.insert_resource(crate::style::RenderOrigin {
            camera: [0.5, 0.0, 0.0],
            ..default()
        });
        let mut snapshot = Snapshot::new();
        observe(&mut world, &mut snapshot).unwrap();
        assert_eq!(snapshot["sky.game_s"], "3600");
        assert_ne!(snapshot["sky.sun.position"], "unavailable");
        assert!(!snapshot.contains_key("sky.sun.position.reason"));
        assert_eq!(snapshot["sky.occlusion.pixel_visibility"], "unavailable");
        assert!(
            crate::scenario_run::evaluate(
                &crate::scenario_file::Check::Present("sky.occlusion.pixel_visibility".into()),
                &snapshot,
                &Default::default()
            )
            .is_err()
        );
        let universe = world.resource::<crate::Universe>();
        assert_eq!(universe.open.point_by_id(1).unwrap().position, moon_before);
        assert_eq!(universe.path.offset(), path_before);
    }
}
