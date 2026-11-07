//! Selected actual observation fields; fixture intent never becomes support.

use crate::scenario_measure::MeasureDiagnostics;
use crate::scenario_run::{PreparationStage, ScenarioRun};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::fmt::Write;

// Every stored graphics preference, plus the actual effective quality. This
// common evidence is identical with collection on and off; it sets no choices.
const COMPARISON_FIELDS: &[&str] = &[
    "settings.saved.quality_mode",
    "settings.saved.target_fps",
    "settings.saved.auto_ceiling",
    "settings.saved.vsync",
    "settings.saved.resolution_3d",
    "settings.saved.planet_detail",
    "settings.saved.marker_detail",
    "settings.saved.atmosphere",
    "settings.saved.shadows",
    "settings.saved.bloom",
    "settings.saved.ground_memory_limit",
    "settings.saved.fullscreen",
    "settings.saved.exposure",
    "settings.saved.labels",
    "settings.saved.label_limit",
    "settings.saved.dimming",
    "settings.saved.night_light",
    "settings.saved.mode_reminder",
    "settings.saved.show_pull",
    "settings.saved.scale_hud",
    "settings.saved.eye_adaptation",
    "settings.saved.body_colour",
    "settings.effective.terrain",
    "settings.effective.markers",
    "settings.effective.atmosphere",
    "settings.effective.shadows",
    "settings.effective.bloom",
    "settings.effective.render_scale",
    "settings.effective.terrain_work",
];

#[derive(Default)]
pub(crate) struct ComparisonScratch {
    previous: String,
    current: String,
}

pub(crate) fn compare(
    diagnostics: Res<MeasureDiagnostics>,
    run: Res<ScenarioRun>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<&Camera, With<Camera3d>>,
    mut scratch: Local<ComparisonScratch>,
) {
    if run.current.is_empty() {
        return;
    }
    scratch.current.clear();
    for key in COMPARISON_FIELDS {
        let value = run
            .current
            .get(*key)
            .map_or("unavailable-not-observed", String::as_str);
        let _ = write!(scratch.current, "\t{key}={value}");
    }
    let output = windows
        .single()
        .ok()
        .map(|w| (w.physical_width(), w.physical_height()));
    let target = cameras.single().ok().and_then(Camera::physical_target_size);
    match output {
        Some((w, h)) => {
            let _ = write!(scratch.current, "\toutput_pixels={w}x{h}");
        }
        None => scratch.current.push_str("\toutput_pixels=unavailable"),
    }
    match target {
        Some(s) => {
            let _ = write!(scratch.current, "\ttarget_pixels={}x{}", s.x, s.y);
        }
        None => scratch.current.push_str("\ttarget_pixels=unavailable"),
    }
    if scratch.current != scratch.previous {
        diagnostics.comparison_state(format!(
            "snapshot_frame={}{}",
            run.snapshot_frame
                .map_or_else(|| "unavailable".into(), |n| n.to_string()),
            scratch.current
        ));
        let ComparisonScratch { previous, current } = &mut *scratch;
        previous.clone_from(current);
    }
}

const FIELDS: &[&str] = &[
    "seed",
    "camera.path",
    "camera.position",
    "camera.position.frame",
    "camera.forward",
    "camera.up",
    "player.path",
    "player.mode",
    "player.active",
    "player.flying",
    "player.on_ground",
    "player.ground_pending",
    "player.position",
    "player.position.frame",
    "player.velocity_m_per_s",
    "player.heading_rad",
    "support.identity",
    "support.identity.reason",
    "support.kind",
    "support.contact",
    "ground.identity",
    "ground.clearance_m",
    "player.surface_clearance_m",
    "player.surface_clearance_m.reason",
    "fixture.kind",
    "fixture.selected.planet",
    "fixture.selected.system",
    "fixture.selected.moon",
    "fixture.moon.type",
    "fixture.moon.radius_km",
    "fixture.direction_basis",
    "clock.game_s",
    "clock.playing",
    "ready",
    "screen",
    "settings.saved.quality_mode",
    "settings.saved.planet_detail",
    "settings.saved.ground_memory_limit",
    "settings.saved.target_fps",
    "settings.saved.vsync",
    "settings.effective.terrain",
    "settings.effective.markers",
    "settings.effective.atmosphere",
    "settings.effective.shadows",
    "settings.effective.bloom",
    "settings.effective.render_scale",
    "settings.effective.terrain_work",
];

fn payload(
    run: &ScenarioRun,
    output: Option<UVec2>,
    target: Option<UVec2>,
    ground: Option<(bool, bool)>,
) -> String {
    let phase = match (&run.outcome, run.prepared, run.started, run.preparation) {
        (Some(Ok(())), ..) => "completed",
        (Some(Err(_)), ..) => "failed",
        (_, false, ..) => "before-setup",
        (_, _, false, _) => "setup-waiting",
        (_, _, _, Some(PreparationStage::Released)) => "preparation-released",
        (_, _, _, Some(PreparationStage::Placed)) => "preparation-waiting",
        _ => "actions",
    };
    let number = |n: Option<u32>| n.map_or_else(|| "unavailable".into(), |n| n.to_string());
    let mut result = format!(
        "step={}\tphase={phase}\tsnapshot_frame={}\tsnapshot_available={}\toutput_width={}\toutput_height={}\ttarget_width={}\ttarget_height={}\tground_ring_ready={}\tground_detail_ready={}",
        run.step,
        run.snapshot_frame.map_or_else(
            || "unavailable-before-first-observation".into(),
            |n| n.to_string()
        ),
        !run.current.is_empty(),
        number(output.map(|s| s.x)),
        number(output.map(|s| s.y)),
        number(target.map(|s| s.x)),
        number(target.map(|s| s.y)),
        ground.map_or_else(
            || "unavailable-no-matching-production-surface".into(),
            |g| g.0.to_string()
        ),
        ground.map_or_else(
            || "unavailable-no-matching-production-surface".into(),
            |g| g.1.to_string()
        )
    );
    for field in FIELDS {
        let value = run
            .current
            .get(*field)
            .map_or("unavailable-not-observed", String::as_str);
        let _ = write!(result, "\t{field}={value}");
    }
    result
}

pub(crate) fn collect(
    diagnostics: Res<MeasureDiagnostics>,
    run: Res<ScenarioRun>,
    universe: Res<crate::Universe>,
    surface: Option<Res<crate::patches::GroundSurface>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<&Camera, With<Camera3d>>,
) {
    let output = windows
        .single()
        .ok()
        .map(|w| UVec2::new(w.physical_width(), w.physical_height()));
    let target = cameras.single().ok().and_then(Camera::physical_target_size);
    let ground = surface
        .as_ref()
        .filter(|s| {
            s.3.is_some_and(|id| {
                id.host().seed() == universe.root
                    && id.host().chain() == universe.path.indices().as_slice()
            })
        })
        .map(|s| (s.1, s.2));
    diagnostics.state(payload(&run, output, target, ground));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measurement_state_cannot_turn_requested_moon_or_readiness_into_support() {
        let scenario = crate::parse_scenario("scenario-v2\nseed 42\nsetup moon 7.2.7.1.2.0.0.0.5 1 0 1 0 10 0 normal 0 0\ntimeout 20\nobserve test").unwrap();
        let mut run = ScenarioRun::new(scenario);
        run.current
            .insert("fixture.selected.moon".into(), "1".into());
        run.current.insert("ready".into(), "true".into());
        let line = payload(&run, None, None, None);
        assert!(line.contains("\tsupport.identity=unavailable-not-observed\t"));
        assert!(line.contains("\tground_ring_ready=unavailable-no-matching-production-surface\t"));
        assert!(line.contains("\ttarget_width=unavailable\t"));
        assert!(!line.contains("support.contact=true"));
    }
}
