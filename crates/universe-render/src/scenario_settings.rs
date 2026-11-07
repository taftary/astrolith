//! Read-only saved/live preferences, view slots and player selection (#1238).

use crate::scenario_run::Snapshot;
use crate::scenario_values::{text, unavailable, value, vector};
use bevy::prelude::*;
use universe_core::settings::SavedPose;

fn view(snapshot: &mut Snapshot, key: &str, pose: Option<SavedPose>) {
    value(snapshot, &format!("{key}.filled"), pose.is_some());
    let Some(p) = pose else {
        for field in [
            "mode",
            "yaw_rad",
            "pitch_rad",
            "speed_step",
            "offset",
            "forward",
            "up",
            "destination",
            "anchored",
        ] {
            unavailable(snapshot, &format!("{key}.{field}"), "empty-view-slot");
        }
        return;
    };
    value(snapshot, &format!("{key}.mode"), format!("{:?}", p.mode));
    value(snapshot, &format!("{key}.yaw_rad"), p.yaw);
    value(snapshot, &format!("{key}.pitch_rad"), p.pitch);
    value(snapshot, &format!("{key}.speed_step"), p.speed.get());
    vector(
        snapshot,
        &format!("{key}.offset"),
        p.offset,
        "saved-view-open-cell-units-see-destination-not-observed-player-pose",
    );
    vector(
        snapshot,
        &format!("{key}.forward"),
        p.free_forward,
        "saved-view-direction",
    );
    vector(
        snapshot,
        &format!("{key}.up"),
        p.free_up,
        "saved-view-direction",
    );
    match p.destination {
        Some(d) => {
            value(
                snapshot,
                &format!("{key}.destination"),
                crate::scenario_setup::chain_text(d.markers()),
            );
            value(snapshot, &format!("{key}.anchored"), d.is_anchored());
        }
        None => unavailable(
            snapshot,
            &format!("{key}.destination"),
            "view-has-no-destination",
        ),
    }
}

pub(crate) fn observe(world: &World, snapshot: &mut Snapshot) {
    let store = world.resource::<crate::SettingsStore>();
    value(
        snapshot,
        "settings.source",
        "production-settings-store-not-file-read-or-requested-values",
    );
    // Existing serializer is the single source of all saved preferences. This
    // does not read the real profile or perform file I/O in the observation path.
    for line in store.0.format().lines() {
        if let Some((key, val)) = line.split_once(" = ") {
            text(snapshot, &format!("settings.saved.{key}"), val);
        }
    }
    for (i, pose) in store.0.views.iter().enumerate() {
        view(snapshot, &format!("views.saved.{}", i + 1), *pose);
    }
    if let Some(slots) = world.get_resource::<crate::input::SavedSlots>() {
        for (i, pose) in slots.slots.iter().enumerate() {
            view(
                snapshot,
                &format!("views.live.{}", i + 1),
                pose.as_ref().map(crate::SettingsStore::view_pose),
            );
        }
    } else {
        unavailable(snapshot, "views.live", "live-view-slot-resource-absent");
        for slot in 1..=8 {
            unavailable(
                snapshot,
                &format!("views.live.{slot}.filled"),
                "live-view-slot-resource-absent",
            );
        }
    }
    let player = world.resource::<crate::player_state::PlayerState>();
    value(snapshot, "player.selected", player.selected);
    value(snapshot, "player.ground_pending", player.ground_pending);
    value(snapshot, "player.airborne", player.airborne);
    value(
        snapshot,
        "player.remembered.path",
        crate::scenario_setup::chain_text(player.spot.chain()),
    );
    let metres =
        1.0 / universe_core::body::metre(universe_core::nest::path_level(player.spot.chain()));
    vector(
        snapshot,
        "player.remembered.position",
        player.spot.position.map(|v| v * metres),
        &format!(
            "seed={}/path={}/metres",
            player.root,
            crate::scenario_setup::chain_text(player.spot.chain())
        ),
    );
    value(snapshot, "player.remembered.yaw_rad", player.spot.yaw);
    if let Some(flight) = world.get_resource::<crate::input::Flight>() {
        value(snapshot, "camera.look_yaw_rad", flight.yaw);
        value(snapshot, "camera.look_pitch_rad", flight.pitch);
    }
    if let Some(effective) = world.get_resource::<crate::EffectiveGraphics>() {
        let q = effective.0;
        for (key, tier) in [
            ("terrain", q.terrain),
            ("markers", q.markers),
            ("atmosphere", q.atmosphere),
            ("terrain_work", q.terrain_work),
        ] {
            value(snapshot, &format!("settings.effective.{key}"), tier.id());
        }
        value(snapshot, "settings.effective.shadows", q.shadows);
        value(snapshot, "settings.effective.bloom", q.bloom);
        value(snapshot, "settings.effective.res_steps", q.res_steps);
    } else {
        unavailable(
            snapshot,
            "settings.effective.terrain",
            "effective-quality-resource-absent",
        );
    }
    match world.get_resource::<crate::RenderScale>() {
        Some(scale) => value(snapshot, "settings.effective.render_scale", scale.0),
        None => unavailable(
            snapshot,
            "settings.effective.render_scale",
            "render-scale-resource-absent",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_slots_and_quality_are_read_without_recall_or_mutation() {
        let mut world = World::new();
        world.init_resource::<crate::SettingsStore>();
        world.init_resource::<crate::player_state::PlayerState>();
        world.init_resource::<crate::input::SavedSlots>();
        world.init_resource::<crate::EffectiveGraphics>();
        let mut saved = world.resource::<crate::SettingsStore>().0;
        saved.quality_mode = universe_core::settings::QualityMode::Manual;
        saved.labels = false;
        saved.views[7] = Some(SavedPose {
            offset: [1.0, 2.0, 3.0],
            destination: None,
            mode: universe_core::settings::ViewMode::Free,
            yaw: 0.25,
            pitch: -0.1,
            free_forward: [0.0, 0.0, -1.0],
            free_up: [0.0, 1.0, 0.0],
            speed: Default::default(),
        });
        world.resource_mut::<crate::SettingsStore>().0 = saved;
        let mut snapshot = Snapshot::new();
        observe(&world, &mut snapshot);
        assert_eq!(world.resource::<crate::SettingsStore>().0, saved);
        assert_eq!(snapshot["settings.saved.labels"], "off");
        assert_eq!(snapshot["views.saved.8.mode"], "Free");
        assert_eq!(
            snapshot["views.live.8.filled"], "false",
            "live and stored slots stay separate"
        );
        for slot in 1..=8 {
            assert!(snapshot.contains_key(&format!("views.saved.{slot}.filled")));
        }
    }
}
