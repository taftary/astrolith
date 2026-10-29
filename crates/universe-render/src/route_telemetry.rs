//! Opt-in, once-per-second route observations (#1087), never normal play.
//!
//! Counts refer to main-world asset stores, not GPU allocations. Missing
//! resources are `none`; a genuine empty store is zero. The app drains one
//! pending CSV row into a streaming sidecar so interrupted runs keep evidence.

use crate::patch_grid::PatchGridMaterial;
use crate::{EffectiveGraphics, FrameWork, RenderScale, RouteRun, Universe};
use bevy::prelude::*;

/// Versioned sidecar columns; the existing route protocol is unchanged.
pub const TELEMETRY_HEADER: &str = "schema,elapsed_s,leg,level,meshes,images,standard_materials,ground_materials,terrain_bytes,terrain_cap_bytes,ring_ready,detail_ready,terrain,markers,atmosphere,shadows,bloom,render_scale,cap_sleep_last_ms,frame_work_available,root_seed,world_path,output_width,output_height,target_width,target_height";

/// A single pending row, drained by the app after the collector runs.
#[derive(Resource, Debug, Default)]
pub struct RouteTelemetry {
    /// Next row to write, including its trailing newline.
    pub pending: Option<String>,
    next_s: f64,
    last_leg: Option<usize>,
    finished: bool,
}

/// Installed only by the explicitly instrumented route driver.
#[derive(Debug, Clone, Copy, Default)]
pub struct RouteTelemetryPlugin;

impl Plugin for RouteTelemetryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RouteTelemetry>()
            .add_systems(PostUpdate, collect);
    }
}

fn count<A: Asset>(world: &World) -> String {
    world
        .get_resource::<Assets<A>>()
        .map_or_else(|| "none".to_string(), |assets| assets.len().to_string())
}

fn observed<T: std::fmt::Display>(value: Option<T>) -> String {
    value.map_or_else(|| "none".to_string(), |v| v.to_string())
}

/// Exclusive only once per observation; no renderer resources are mutated.
fn collect(world: &mut World) {
    let Some(run) = world.get_resource::<RouteRun>() else {
        return;
    };
    let (leg, elapsed, done) = (run.leg, run.t, run.done);
    let Some(state) = world.get_resource::<RouteTelemetry>() else {
        return;
    };
    if state.finished || (elapsed < state.next_s && state.last_leg == Some(leg) && !done) {
        return;
    }
    let level = world.get_resource::<Universe>().map(|u| u.0.level().get());
    let ground = world.get_resource::<crate::patches::GroundSurface>();
    // Surface flags at other levels are not applicable, even if resources remain.
    let ring = ground.filter(|_| level == Some(10)).map(|g| g.1);
    let ready = ground.filter(|_| level == Some(10)).map(|g| g.2);
    let bytes = world
        .get_resource::<crate::patch_cache::PatchCache>()
        .map(crate::patch_cache::PatchCache::bytes);
    let cap = world
        .get_resource::<crate::patches::PatchTasks>()
        .map(|t| t.cap_bytes);
    let quality = world.get_resource::<EffectiveGraphics>().map(|q| q.0);
    let scale = world.get_resource::<RenderScale>().map(|s| s.0);
    let work = world.get_resource::<FrameWork>().map(FrameWork::sample);
    let seed = world.get_resource::<Universe>().map(|u| u.0.root.0);
    let path = world.get_resource::<Universe>().map(|u| {
        u.0.path
            .indices()
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join("/")
    });
    let output = world
        .query::<&Window>()
        .iter(world)
        .next()
        .map(|w| (w.physical_width(), w.physical_height()));
    let target = world
        .query_filtered::<&Camera, With<Camera3d>>()
        .iter(world)
        .next()
        .and_then(Camera::physical_target_size);
    let row = format!(
        "1,{elapsed:.3},{leg},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
        observed(level),
        count::<Mesh>(world),
        count::<Image>(world),
        count::<StandardMaterial>(world),
        count::<PatchGridMaterial>(world),
        observed(bytes),
        observed(cap),
        observed(ring),
        observed(ready),
        observed(quality.map(|q| q.terrain.id())),
        observed(quality.map(|q| q.markers.id())),
        observed(quality.map(|q| q.atmosphere.id())),
        observed(quality.map(|q| q.shadows)),
        observed(quality.map(|q| q.bloom)),
        observed(scale),
        observed(work.map(|w| w.cap_ms)),
        work.is_some(),
        observed(seed),
        observed(path),
        observed(output.map(|s| s.0)),
        observed(output.map(|s| s.1)),
        observed(target.map(|s| s.x)),
        observed(target.map(|s| s.y)),
    );
    if let Some(mut state) = world.get_resource_mut::<RouteTelemetry>() {
        state.pending = Some(row);
        state.next_s = elapsed + 1.0;
        state.last_leg = Some(leg);
        state.finished = done;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_counts_real_stores_and_does_not_invent_missing_readings() {
        let mut world = World::new();
        world.insert_resource(RouteRun::new(
            crate::parse_route("a L10 lat=0 lon=0 h=2 heading=0 for=3 scenario=ground")
                .expect("route"),
        ));
        world.init_resource::<RouteTelemetry>();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<Assets<StandardMaterial>>();
        world.init_resource::<Assets<PatchGridMaterial>>();
        let image = world.resource_mut::<Assets<Image>>().add(Image::default());
        let material = world
            .resource_mut::<Assets<PatchGridMaterial>>()
            .add(PatchGridMaterial::default());
        collect(&mut world);
        let row = world
            .resource_mut::<RouteTelemetry>()
            .pending
            .take()
            .expect("row");
        assert_eq!(
            row.trim().split(',').count(),
            TELEMETRY_HEADER.split(',').count()
        );
        assert!(
            row.starts_with("1,0.000,0,none,0,1,0,1,none,none,none,none,"),
            "{row}"
        );
        assert!(
            row.ends_with(",none,false,none,none,none,none,none,none\n"),
            "{row}"
        );
        collect(&mut world);
        assert!(world.resource::<RouteTelemetry>().pending.is_none());
        world.resource_mut::<Assets<Image>>().remove(image.id());
        world
            .resource_mut::<Assets<PatchGridMaterial>>()
            .remove(material.id());
        world.resource_mut::<RouteRun>().t = 1.1;
        collect(&mut world);
        assert!(
            world
                .resource::<RouteTelemetry>()
                .pending
                .as_ref()
                .expect("row")
                .starts_with("1,1.100,0,none,0,0,0,0,")
        );
    }
}
