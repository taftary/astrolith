//! Graphics backend choice for the window builds (#499 ADR 0022, #780
//! ADR 0041).
//!
//! Windows uses Vulkan (#780 F3 A): on the dev PC, DirectX 12 corrupts the
//! engine's atmosphere tables. Vulkan's known swapchain warning lines are
//! accepted by the owner. Other platforms keep Bevy's default (Vulkan on
//! Linux). The `WGPU_BACKEND` environment variable always wins, so
//! DirectX 12 stays a manual fallback. Normal play and `--screens` both
//! build their render plugin and frame pacing here, so the two can't drift
//! apart. `--verify` and `--capture` open no window.
//!
//! Frame pacing ([`FramePacingPlugin`]): on Vulkan the dev PC lets the
//! CPU queue frames far ahead of a slower GPU, so the picture lagged tens
//! of seconds behind while the ground streamed in. On Vulkan the render
//! thread waits for the GPU at the end of each frame.

use bevy::prelude::*;
use bevy::render::renderer::{RenderAdapterInfo, RenderDevice};
use bevy::render::settings::{Backends, WgpuSettings};
use bevy::render::{Render, RenderApp, RenderPlugin, RenderSystems};

/// Backends to use (pure): the override, else Vulkan on Windows, else
/// the given default.
#[must_use]
fn backends_for(
    env_override: Option<Backends>,
    windows: bool,
    default: Option<Backends>,
) -> Option<Backends> {
    env_override.or(if windows {
        Some(Backends::VULKAN)
    } else {
        default
    })
}

/// Render plugin for a window build, with the backend chosen above and every
/// other setting at Bevy's default.
#[must_use]
pub(crate) fn render_plugin() -> RenderPlugin {
    let mut settings = WgpuSettings::default();
    settings.backends = backends_for(Backends::from_env(), cfg!(windows), settings.backends);
    RenderPlugin {
        render_creation: settings.into(),
        ..Default::default()
    }
}

/// Whether the render thread waits for the GPU each frame on `backend`
/// (pure): Vulkan only; DirectX 12 and the others pace themselves.
#[must_use]
fn paces_frames(backend: Backends) -> bool {
    backend == Backends::VULKAN
}

/// Waits for this frame's GPU work after it is submitted, on Vulkan.
fn wait_for_gpu(info: Res<RenderAdapterInfo>, device: Res<RenderDevice>) {
    if paces_frames(Backends::from(info.backend)) {
        let _ = device.poll(bevy::render::render_resource::PollType::wait_indefinitely());
    }
}

/// Frame pacing for the window builds (see the module notes).
pub(crate) struct FramePacingPlugin;

impl Plugin for FramePacingPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.add_systems(Render, wait_for_gpu.in_set(RenderSystems::Cleanup));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_without_override_uses_vulkan() {
        // #780 F3 A: DirectX 12 corrupts the atmosphere on the dev PC.
        assert_eq!(
            backends_for(None, true, Some(Backends::all())),
            Some(Backends::VULKAN)
        );
    }

    #[test]
    fn override_wins_on_windows() {
        assert_eq!(
            backends_for(Some(Backends::DX12), true, Some(Backends::all())),
            Some(Backends::DX12)
        );
    }

    #[test]
    fn only_vulkan_waits_for_the_gpu() {
        assert!(paces_frames(Backends::VULKAN));
        assert!(!paces_frames(Backends::DX12));
        assert!(!paces_frames(Backends::GL));
    }

    #[test]
    fn other_platforms_keep_the_default() {
        assert_eq!(
            backends_for(None, false, Some(Backends::all())),
            Some(Backends::all())
        );
        assert_eq!(
            backends_for(Some(Backends::GL), false, Some(Backends::all())),
            Some(Backends::GL)
        );
    }

    #[test]
    fn both_window_builds_use_this_render_plugin() {
        for (name, source) in [
            ("main.rs", include_str!("main.rs")),
            ("screens.rs", include_str!("screens.rs")),
        ] {
            let code = source.split("#[cfg(test)]").next().expect("code part");
            assert!(
                code.contains(".set(render_plugin())"),
                "{name} does not use gpu::render_plugin"
            );
            assert!(
                code.contains("FramePacingPlugin"),
                "{name} does not pace its frames"
            );
        }
    }
}
