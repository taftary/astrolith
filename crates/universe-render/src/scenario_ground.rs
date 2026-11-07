//! Observations of actual production ground, never the selected fixture (#1238).

use crate::scenario_run::Snapshot;
use crate::scenario_values::{text, unavailable, value, vector};
use bevy::prelude::*;
use universe_core::coords::OpenUnits;
use universe_core::surface_observation::{GroundObservation, SurfaceIdentity};

fn identity(snapshot: &mut Snapshot, prefix: &str, id: SurfaceIdentity) {
    let host = id.host();
    let planet = crate::scenario_setup::chain_text(host.chain());
    let (kind, suffix) = match id {
        SurfaceIdentity::Planet(_) => ("planet", String::new()),
        SurfaceIdentity::Moon(moon) => ("moon", format!("/moon={}", moon.marker().0)),
    };
    value(
        snapshot,
        &format!("{prefix}.identity"),
        format!("seed={}/planet={planet}{suffix}", host.seed().0),
    );
    value(snapshot, &format!("{prefix}.kind"), kind);
    value(snapshot, &format!("{prefix}.planet"), planet);
    value(
        snapshot,
        &format!("{prefix}.system"),
        crate::scenario_setup::chain_text(host.system()),
    );
}

pub(crate) fn observe(world: &World, snapshot: &mut Snapshot) {
    let universe = world.resource::<crate::Universe>();
    let player = world.resource::<crate::player_state::PlayerState>();
    let same_cell = player.root == universe.root.0
        && (player.active || player.spot.chain() == universe.path.indices());
    let feet = if player.active {
        player.body.position
    } else {
        player.spot.position
    };
    let metres = 1.0 / universe_core::body::metre(universe.level());
    let frame = format!(
        "seed={}/path={}/metres",
        universe.root.0,
        crate::scenario_setup::chain_text(&universe.path.indices())
    );
    let direction = frame.replace("metres", "direction");
    let surface = world.get_resource::<crate::patches::GroundSurface>();
    let drawn = surface.and_then(|s| s.0.as_ref());
    let owner = surface.and_then(|s| s.3).filter(|id| {
        id.host().seed() == universe.root && id.host().chain() == universe.path.indices().as_slice()
    });
    let ground = same_cell
        .then(|| crate::patches::ground_under(surface, universe, feet))
        .flatten();
    let observed = owner
        .zip(ground)
        .and_then(|(id, g)| GroundObservation::from_ground(id, g, OpenUnits(feet), metres));
    // Preserve the legacy field's near-moon meaning. The owner-labeled host
    // query remains separately available as ground.clearance_m, not contact.
    match observed {
        Some(g)
            if !snapshot.contains_key("player.moon")
                || matches!(g.identity, SurfaceIdentity::Moon(_)) =>
        {
            value(snapshot, "player.surface_clearance_m", g.clearance.0)
        }
        _ => unavailable(
            snapshot,
            "player.surface_clearance_m",
            "no-drawn-ground-for-this-body-context",
        ),
    }
    value(
        snapshot,
        "ground.source",
        "production-drawn-surface-query-not-contact",
    );
    for key in [
        "ground.identity",
        "ground.kind",
        "ground.planet",
        "ground.system",
        "ground.point",
        "ground.up",
        "ground.clearance_m",
        "ground.slope_deg",
        "ground.normal",
        "ground.patch",
        "ground.local_spot",
        "ground.local_origin",
        "ground.local_up",
        "ground.triangle.0",
        "ground.triangle.1",
        "ground.triangle.2",
        "support.identity",
        "support.kind",
        "support.planet",
        "support.system",
    ] {
        unavailable(snapshot, key, "no-matching-production-drawn-ground");
    }
    if let Some(g) = observed {
        identity(snapshot, "ground", g.identity);
        vector(
            snapshot,
            "ground.point",
            g.point.0.map(|v| v * metres),
            &frame,
        );
        vector(snapshot, "ground.up", g.up.0, &direction);
        value(snapshot, "ground.clearance_m", g.clearance.0);
        if player.active && player.body.on_ground {
            identity(snapshot, "support", g.identity);
        } else {
            unavailable(
                snapshot,
                "support.identity",
                "production-body-not-in-ground-contact",
            );
            unavailable(
                snapshot,
                "support.kind",
                "production-body-not-in-ground-contact",
            );
        }
        if let Some(drawn) = drawn {
            let local_frame = drawn.frame();
            if let Some(triangle) = drawn.drawn_triangle_at(local_frame.from_cell(feet)) {
                value(snapshot, "ground.patch", format!("{:?}", triangle.patch));
                vector(snapshot, "ground.normal", triangle.normal.0, &direction);
                value(snapshot, "ground.slope_deg", triangle.slope.0);
                for (i, vertex) in triangle.vertices.iter().enumerate() {
                    vector(
                        snapshot,
                        &format!("ground.triangle.{i}"),
                        vertex.0,
                        &format!("{frame}/planet-fixed"),
                    );
                }
            }
        }
    }
    value(
        snapshot,
        "support.contact",
        player.active && player.body.on_ground,
    );
    value(
        snapshot,
        "support.flying",
        player.active && player.body.flying,
    );
    if player.active && same_cell {
        let up = ground.map_or_else(|| universe_core::body::level_up(universe, feet), |g| g.up);
        vector(snapshot, "player.up", up, &direction);
        vector(
            snapshot,
            "player.velocity_m_per_s",
            player.body.velocity.map(|v| v * metres),
            &format!("{frame}/per-second"),
        );
        value(snapshot, "player.heading_rad", player.body.yaw);
        value(snapshot, "player.roll_rad", player.body.roll);
        vector(
            snapshot,
            "player.forward",
            universe_core::body::heading_basis(up, player.body.yaw).0,
            &direction,
        );
        let pull = universe_core::gravity::pull(universe, feet);
        vector(
            snapshot,
            "gravity.acceleration_m_per_s2",
            pull.accel.map(|v| v * metres),
            &format!("{frame}/per-second-squared"),
        );
        value(snapshot, "gravity.strongest_m_per_s2", pull.strongest);
        text(snapshot, "gravity.strongest_name", &pull.name);
        match pull.source {
            Some(universe_core::gravity::PullSource::OpenPlanet) => value(
                snapshot,
                "gravity.strongest_identity",
                format!(
                    "seed={}/planet={}",
                    universe.root.0,
                    crate::scenario_setup::chain_text(&universe.path.indices())
                ),
            ),
            Some(universe_core::gravity::PullSource::Marker(marker)) => value(
                snapshot,
                "gravity.strongest_identity",
                format!(
                    "seed={}/path={}/marker={}",
                    universe.root.0,
                    crate::scenario_setup::chain_text(&universe.path.indices()),
                    marker.0
                ),
            ),
            None => unavailable(
                snapshot,
                "gravity.strongest_identity",
                "no-massive-production-pull-source",
            ),
        }
        vector(snapshot, "gravity.toward", pull.toward, &direction);
        value(
            snapshot,
            "gravity.source",
            "production-pull-at-observed-feet-after-update",
        );
        if let Some(anchor) = player.anchor.0 {
            vector(
                snapshot,
                "ground.local_spot",
                anchor
                    .to_local(universe_core::planet_frame::PlanetPos(
                        feet.map(|v| v * metres),
                    ))
                    .0,
                &format!("{frame}/production-body-anchor={anchor:?}/local-metres"),
            );
            vector(snapshot, "ground.local_origin", anchor.origin.0, &frame);
            vector(snapshot, "ground.local_up", anchor.up, &direction);
        }
    } else {
        for key in [
            "player.up",
            "player.forward",
            "player.velocity_m_per_s",
            "player.heading_rad",
            "player.roll_rad",
            "gravity.acceleration_m_per_s2",
            "gravity.strongest_m_per_s2",
            "gravity.strongest_name",
            "gravity.strongest_identity",
            "gravity.toward",
        ] {
            unavailable(snapshot, key, "no-active-body-in-this-production-cell");
        }
    }
    // The chosen/nearby moon is never promoted to supporting ground. Future
    // production moon surfaces use the same owner/geometry record above.
    let moon_ground = observed.filter(|g| matches!(g.identity, SurfaceIdentity::Moon(_)));
    match moon_ground {
        Some(g) => value(snapshot, "moon.ground.clearance_m", g.clearance.0),
        None => unavailable(
            snapshot,
            "moon.ground.clearance_m",
            "production-moon-ground-not-present",
        ),
    }
    if let Some(relief) = universe_core::nav::planet_relief(universe) {
        value(snapshot, "scene.surface.recipe_seed", relief.seed());
        value(snapshot, "scene.surface.radius_m", relief.radius_m());
        value(snapshot, "scene.surface.cratered", relief.is_cratered());
        if let Some(up) = ground.map(|g| g.up) {
            value(
                snapshot,
                "scene.surface.terrain_height_m",
                relief.terrain_m(up),
            );
            match relief.crater_height_m(up) {
                Some(height) => value(snapshot, "scene.surface.crater_delta_m", height),
                None => unavailable(
                    snapshot,
                    "scene.surface.crater_delta_m",
                    "not-applicable-no-crater-field",
                ),
            }
        }
    } else {
        unavailable(
            snapshot,
            "scene.surface.recipe_seed",
            "no-production-surface-recipe",
        );
    }
    if let Some(recipe) = universe.open.points.iter().find_map(|p| p.surface) {
        value(snapshot, "scene.surface.spin_hours", recipe.spin_hours);
        value(snapshot, "scene.surface.tilt_deg", recipe.tilt_deg);
        value(
            snapshot,
            "scene.surface.has_air_recipe",
            recipe.air.is_some(),
        );
        match recipe.air {
            Some(air) => value(snapshot, "scene.surface.air_thickness_km", air.thickness_km),
            None => unavailable(
                snapshot,
                "scene.surface.air_thickness_km",
                "not-applicable-airless-production-recipe",
            ),
        }
    }
    if let Some(tasks) = world.get_resource::<crate::patches::PatchTasks>() {
        match tasks.world {
            Some(kind) => value(snapshot, "scene.surface.world_type", kind.name()),
            None => unavailable(
                snapshot,
                "scene.surface.world_type",
                "no-built-ground-world-type",
            ),
        }
    }
    if let Some(atlas) = world.get_resource::<crate::patch_grid::GridAtlas>() {
        let look = atlas.diagnostic_look();
        value(
            snapshot,
            "scene.material.source",
            "cached-shared-grid-uniforms-see-actual-material-table-main-world",
        );
        value(snapshot, "scene.material.ice_cracks", look.steep.w);
        value(snapshot, "scene.material.crater_shading", look.bands.w);
        value(snapshot, "scene.material.detail_reach_m", look.bands.z);
        value(snapshot, "scene.material.water_kind", look.water.x);
        value(snapshot, "scene.material.glow", look.glow.w);
        value(
            snapshot,
            "scene.material.asset_count",
            atlas.diagnostic_materials().len(),
        );
        for (i, handle) in atlas.diagnostic_materials().iter().enumerate() {
            let key = format!("scene.material.asset.{i}");
            value(snapshot, &format!("{key}.id"), format!("{:?}", handle.id()));
            let asset = world
                .get_resource::<Assets<crate::patch_grid::PatchGridMaterial>>()
                .and_then(|assets| assets.get(handle));
            if let Some(material) = asset {
                value(
                    snapshot,
                    &format!("{key}.look"),
                    format!("{:?}", material.extension.look),
                );
                value(
                    snapshot,
                    &format!("{key}.heights"),
                    format!("{:?}", material.extension.heights.id()),
                );
                value(
                    snapshot,
                    &format!("{key}.cull_mode"),
                    format!("{:?}", material.base.cull_mode),
                );
            } else {
                unavailable(
                    snapshot,
                    &format!("{key}.look"),
                    "material-handle-not-in-main-world-asset-store",
                );
            }
        }
    } else {
        unavailable(
            snapshot,
            "scene.material.source",
            "no-production-grid-materials",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actual_surface_identity_is_separate_from_contact_and_mismatched_ground_is_absent() {
        let mut world = World::new();
        let mut universe = universe_core::nav::Universe::new(42);
        for &marker in &universe_core::nest::autopilot_path(42) {
            assert!(universe.open(universe_core::nav::MarkerIndex(marker)));
        }
        let identity =
            universe_core::moon_catalog::PlanetIdentity::from_open_cell(&universe).unwrap();
        let frame = universe_core::nav::planet_frame(&universe).unwrap();
        let relief = universe_core::nav::planet_relief(&universe).unwrap();
        let key =
            universe_core::lod::PatchKey::new(universe_core::cube_sphere::Face::PosX, 0, 0, 0)
                .unwrap();
        let mesh = std::sync::Arc::new(
            universe_core::lod_mesh::build_patch(key, frame.radius_m(), relief).unwrap(),
        );
        let feet = frame.to_cell(universe_core::planet_frame::PlanetPos([
            frame.radius_m() + 5.0,
            0.0,
            0.0,
        ]));
        let drawn = universe_core::drawn_surface::DrawnSurface::new(
            frame,
            frame.from_cell(feet),
            2.0,
            vec![(key, mesh, 0)],
        );
        let player = crate::player_state::PlayerState {
            root: 42,
            active: true,
            body: universe_core::body::Body::standing(feet, 0.0),
            ..default()
        };
        world.insert_resource(crate::Universe(universe));
        world.insert_resource(player.clone());
        world.insert_resource(crate::patches::GroundSurface(
            Some(drawn),
            true,
            true,
            Some(SurfaceIdentity::Planet(identity)),
        ));
        let mut snapshot = Snapshot::new();
        observe(&world, &mut snapshot);
        assert_eq!(snapshot["ground.kind"], "planet");
        assert_eq!(
            snapshot["support.identity"], "unavailable",
            "a query is not contact"
        );
        assert!(
            snapshot["ground.clearance_m"]
                .parse::<f64>()
                .unwrap()
                .is_finite()
        );
        assert!(!snapshot.contains_key("ground.identity.reason"));
        assert_eq!(snapshot["moon.ground.clearance_m"], "unavailable");
        assert_eq!(
            world.resource::<crate::player_state::PlayerState>(),
            &player
        );
        world.resource_mut::<crate::Universe>().root = universe_core::nav::Seed(7);
        snapshot.clear();
        observe(&world, &mut snapshot);
        assert_eq!(snapshot["ground.identity"], "unavailable");
        assert_eq!(snapshot["player.surface_clearance_m"], "unavailable");
    }
}
