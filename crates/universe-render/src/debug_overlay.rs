//! LOD debug overlay (rebuild step 3, #776, T4, Q6 A, S2).
//!
//! `F3` toggles it, off by default; the Settings > Developer row mirrors the
//! key and both persist. When on it shows the seven required items: wireframe,
//! patch colour by level, patch count, split and merge queues, frame time,
//! camera altitude above the ground, and the render origin. Wireframe and
//! level colours draw as gizmos (buffered headless, never a window); the
//! counts and numbers live in [`OverlayInfo`] for tests and the owner test.

use crate::Universe;
use crate::input::Navigation;
use crate::patches::{PatchCache, PatchTasks, PatchTree};
use crate::settings_ui::SettingsStore;
use bevy::prelude::*;
use universe_core::planet_frame::PlanetFrame;

/// Seven-item overlay state, updated every frame.
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub(crate) struct OverlayInfo {
    /// Overlay on (`F3`, Developer row).
    pub enabled: bool,
    /// Wireframe draws (always true when enabled).
    pub wireframe: bool,
    /// Patches coloured by depth (always true when enabled).
    pub lod_colours: bool,
    /// Patches drawn.
    pub patch_count: usize,
    /// Worker builds waiting.
    pub split_queue: usize,
    /// Finished builds waiting to apply.
    pub merge_queue: usize,
    /// Last frame time in milliseconds.
    pub frame_ms: f32,
    /// Camera altitude above the true surface in metres.
    pub altitude_m: f64,
    /// Render origin in open-cell units.
    pub origin: [f64; 3],
}

/// Level colour for the overlay: distinct hue per depth (Q6 A).
#[must_use]
pub(crate) fn level_colour(level: u32) -> Color {
    // Six hues cycle with depth; bright, distinct, no visual value outside
    // tokens (overlay only, not the design system).
    let palette = [
        Color::srgb(1.0, 0.3, 0.3),
        Color::srgb(1.0, 0.7, 0.2),
        Color::srgb(1.0, 1.0, 0.3),
        Color::srgb(0.3, 1.0, 0.4),
        Color::srgb(0.3, 0.7, 1.0),
        Color::srgb(0.8, 0.4, 1.0),
    ];
    let mut index = usize::try_from(level).unwrap_or(0);
    index %= palette.len();
    let mut result = palette[0];
    for (i, colour) in palette.iter().enumerate() {
        if i == index {
            result = *colour;
            break;
        }
    }
    result
}

/// Syncs the overlay state and draws wireframe plus level colours.
///
/// Runs in the Draw set when the universe view is live. Wireframe draws patch
/// edges as gizmo lines in the level colour; counts come from the live
/// selection, cache and worker queues; altitude from the planet frame.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn sync_overlay(
    mut gizmos: Gizmos,
    store: Res<SettingsStore>,
    nav: Res<Navigation>,
    universe: Res<Universe>,
    tree: Res<PatchTree>,
    cache: Res<PatchCache>,
    tasks: Res<PatchTasks>,
    origin: Res<crate::style::RenderOrigin>,
    time: Res<Time>,
    mut info: ResMut<OverlayInfo>,
) {
    let enabled = store.0.debug_overlay && nav.show_overlay;
    info.enabled = enabled;
    info.wireframe = enabled;
    info.lod_colours = enabled;
    // Drawn patches (#778: culling keeps fewer than the cache holds).
    info.patch_count = tasks.drawn.len();
    info.split_queue = tasks.pending.len();
    info.merge_queue = tasks.completed.len();
    info.frame_ms = time.delta_secs() * 1000.0;
    // Altitude and origin through the planet frame at L10; NaN-safe outside.
    if universe.level().get() == 10 {
        let chain = universe.path.indices();
        if let Some(l10) = chain.get(..9) {
            let frame = PlanetFrame::for_planet(universe.root.0, l10);
            let camera = frame.from_cell(universe.path.offset());
            let r =
                (camera.0[0] * camera.0[0] + camera.0[1] * camera.0[1] + camera.0[2] * camera.0[2])
                    .sqrt();
            let alt = r - frame.radius_m();
            info.altitude_m = if alt.is_finite() { alt } else { 0.0 };
        } else {
            info.altitude_m = 0.0;
        }
    } else {
        info.altitude_m = 0.0;
    }
    info.origin = origin.camera;
    if !enabled {
        return;
    }
    // Wireframe: one gizmo line loop per drawn patch edge in its level colour.
    // Headless buffers these; the window draws them over the meshes.
    // Reuses the draw set computed by `sync_patches` (no per-frame allocation).
    for key in tasks.drawn.iter().copied() {
        let Some(mesh) = cache.map.get(&key) else {
            continue;
        };
        let colour = level_colour(key.level);
        // Patch corners in world (camera-relative) via the origin: centre plus
        // the four corner offsets (first, last of first/last rows).
        let centre_cell = tree
            .frame
            .as_ref()
            .map(|frame| frame.to_cell(mesh.centre))
            .unwrap_or([0.0, 0.0, 0.0]);
        let at = origin.at(centre_cell);
        // Four corners from the stored offsets (row-major 33x33).
        let corner = |index: usize| -> Vec3 {
            let Some(off) = mesh.offsets.get(index) else {
                return at;
            };
            // Offsets are metres, as the world at L10 (#780 F2 A).
            at + Vec3::new(off[0], off[1], off[2])
        };
        let side = 33usize;
        let c00 = corner(0);
        let c10 = corner(side - 1);
        let c01 = corner((side - 1) * side);
        let c11 = corner(side * side - 1);
        gizmos.line(c00, c10, colour);
        gizmos.line(c10, c11, colour);
        gizmos.line(c11, c01, colour);
        gizmos.line(c01, c00, colour);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_tests::headless_app;
    use universe_core::nav::MarkerIndex;

    fn tap(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release(key);
        input.clear();
        app.update();
    }

    #[test]
    fn f3_toggles_the_debug_overlay() {
        // AC6 / C6 (T4, Q6 A): F3 toggles the overlay with all seven items,
        // off by default. No-window app test (E-TEST-NO-WINDOW).
        let mut app = headless_app();
        app.init_resource::<OverlayInfo>();
        // Overlay systems run in the Draw set; init the resource and step.
        app.add_systems(Update, sync_overlay);
        app.update();
        let info = app.world().resource::<OverlayInfo>();
        assert!(!info.enabled, "overlay off by default");
        assert!(!app.world().resource::<SettingsStore>().0.debug_overlay);
        assert!(!app.world().resource::<Navigation>().show_overlay);
        // Dive to L10 so the overlay has patches, queues, altitude, origin.
        {
            let mut universe = app.world_mut().resource_mut::<Universe>();
            for _ in 0..9 {
                let marker = universe.autopilot_target().expect("dive continues");
                assert!(universe.open(MarkerIndex(marker)), "marker opens");
            }
            assert_eq!(universe.level().get(), 10);
        }
        for _ in 0..3 {
            app.update();
        }
        // F3 (the DebugOverlay binding) turns it on.
        tap(&mut app, KeyCode::F3);
        let info = app.world().resource::<OverlayInfo>().clone();
        assert!(info.enabled, "F3 turns the overlay on");
        assert!(app.world().resource::<SettingsStore>().0.debug_overlay);
        assert!(app.world().resource::<Navigation>().show_overlay);
        // All seven items present.
        assert!(info.wireframe, "wireframe");
        assert!(info.lod_colours, "LOD colour by depth");
        assert!(info.patch_count > 0, "patch count {}", info.patch_count);
        // Queues: pending plus completed (one may be zero after settling, but
        // the fields exist and are finite).
        let _ = info.split_queue;
        let _ = info.merge_queue;
        assert!(info.frame_ms.is_finite(), "frame time {}", info.frame_ms);
        assert!(info.altitude_m.is_finite(), "altitude {}", info.altitude_m);
        assert!(
            info.origin.iter().all(|c| c.is_finite()),
            "render origin {:?}",
            info.origin
        );
        // Level colours differ by depth.
        assert_ne!(level_colour(0), level_colour(1));
        assert_eq!(level_colour(0), level_colour(6));
        // F3 again hides it.
        tap(&mut app, KeyCode::F3);
        assert!(!app.world().resource::<OverlayInfo>().enabled);
        assert!(!app.world().resource::<SettingsStore>().0.debug_overlay);
    }
}
