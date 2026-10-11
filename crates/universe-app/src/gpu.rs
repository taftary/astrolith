//! Graphics backend choice for the window builds (#499, ADR 0022).
//!
//! Windows uses DirectX 12: on the dev PC, Vulkan logs validation and
//! loader `ERROR` lines on every window run, and DirectX 12 logs none.
//! Other platforms keep Bevy's default (Vulkan on Linux). The `WGPU_BACKEND`
//! environment variable always wins, so a developer can still pick Vulkan.
//! Normal play and `--screens` both build their render plugin here, so the
//! two can't drift apart. `--verify` and `--capture` open no window.

use bevy::render::RenderPlugin;
use bevy::render::settings::{Backends, WgpuSettings};

/// Backends to use (pure): the override, else DirectX 12 on Windows, else
/// the given default.
#[must_use]
fn backends_for(
    env_override: Option<Backends>,
    windows: bool,
    default: Option<Backends>,
) -> Option<Backends> {
    env_override.or(if windows {
        Some(Backends::DX12)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_without_override_uses_dx12() {
        assert_eq!(
            backends_for(None, true, Some(Backends::all())),
            Some(Backends::DX12)
        );
    }

    #[test]
    fn override_wins_on_windows() {
        assert_eq!(
            backends_for(Some(Backends::VULKAN), true, Some(Backends::all())),
            Some(Backends::VULKAN)
        );
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
        }
    }
}
