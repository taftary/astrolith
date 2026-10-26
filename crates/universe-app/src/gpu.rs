//! Graphics backend choice for the window builds (#499 ADR 0022, #780
//! ADR 0041, #848 ADR 0042).
//!
//! Every platform uses the engine's automatic choice (#848 Q1 B); on the
//! dev PC that is Vulkan, the backend that draws the atmosphere correctly
//! there (#780 F3 A). The `WGPU_BACKEND` environment variable always wins,
//! so a developer can force DirectX 12 or Vulkan. Normal play and
//! `--screens` both build their render plugin and frame pacing here, so the
//! two can't drift apart. `--verify` and `--capture` open no window.
//!
//! Frame pacing ([`FramePacingPlugin`]): on Vulkan the dev PC lets the
//! CPU queue frames far ahead of a slower GPU, so the picture lagged tens
//! of seconds behind while the ground streamed in. On Vulkan each frame,
//! before it takes its window image, waits for the frame before the
//! previous one (#819): the GPU draws one frame while the CPU prepares the
//! next, and no more than two frames are ever queued.
//!
//! Threads ([`task_pool_plugin`], #918 T8): ground patches build on the
//! async compute pool, which Bevy sizes at a quarter of the cores (two on
//! the dev PC). Three quarters of the cores build patches, one thread
//! serves file I/O, and the rest run the frame: on the dev PC the frame
//! time stayed at the `main` baseline while the ring got ready about twice
//! as fast as with half (#918).

use bevy::prelude::*;
use bevy::render::render_resource::PollType;
use bevy::render::renderer::{RenderAdapterInfo, RenderDevice, RenderQueue};
use bevy::render::settings::{Backends, WgpuSettings};
use bevy::render::view::window::prepare_windows;
use bevy::render::{Render, RenderApp, RenderPlugin, RenderSystems};

/// Backends to use (pure): the override, else the engine's automatic
/// default, on every platform (#848 Q1 B).
#[must_use]
fn backends_for(env_override: Option<Backends>, default: Option<Backends>) -> Option<Backends> {
    env_override.or(default)
}

/// Share of the cores that build ground patches (#918 T8).
const BUILD_SHARE: f32 = 0.75;

/// Thread pools for the window builds (#918 T8): three quarters of the
/// cores for ground patch builds, one for file I/O, the rest for the frame.
#[must_use]
pub(crate) fn task_pool_plugin() -> TaskPoolPlugin {
    let mut options = TaskPoolOptions::default();
    options.io.max_threads = 1;
    options.async_compute.percent = BUILD_SHARE;
    options.async_compute.max_threads = usize::MAX;
    TaskPoolPlugin {
        task_pool_options: options,
    }
}

/// Render plugin for a window build, with the backend chosen above and every
/// other setting at Bevy's default.
#[must_use]
pub(crate) fn render_plugin() -> RenderPlugin {
    let mut settings = WgpuSettings::default();
    settings.backends = backends_for(Backends::from_env(), settings.backends);
    // Measuring aid for timing runs (#920 T4): the GPU pass timers need the
    // timestamp and statistics queries.
    if gpu_timers() {
        use bevy::render::settings::WgpuFeatures;
        settings.features |= WgpuFeatures::TIMESTAMP_QUERY
            | WgpuFeatures::TIMESTAMP_QUERY_INSIDE_ENCODERS
            | WgpuFeatures::TIMESTAMP_QUERY_INSIDE_PASSES
            | WgpuFeatures::PIPELINE_STATISTICS_QUERY;
    }
    RenderPlugin {
        render_creation: settings.into(),
        ..Default::default()
    }
}

/// Whether `ASTROLITH_GPU_TIMERS=1` asks a timing run for GPU pass timers
/// (#920 T4, a developer measuring aid; ADR 0044 asked for them).
#[must_use]
pub(crate) fn gpu_timers() -> bool {
    std::env::var("ASTROLITH_GPU_TIMERS").is_ok_and(|v| v == "1")
}

/// Whether the render thread waits for the GPU each frame on `backend`
/// (pure): Vulkan only; DirectX 12 and the others pace themselves.
#[must_use]
fn paces_frames(backend: Backends) -> bool {
    backend == Backends::VULKAN
}

/// GPU frames allowed in flight on Vulkan: the one being drawn plus the one
/// just submitted (#819).
const FRAMES_IN_FLIGHT: usize = 2;

/// Submissions of the last frames, oldest first (Vulkan pacing). Generic
/// over the engine's submission index, which Bevy does not re-export: the
/// two functions fix it, so no direct graphics dependency is added.
#[derive(Resource)]
struct InFlight<T> {
    /// Submissions not yet waited for, oldest first.
    queued: std::collections::VecDeque<T>,
    /// Marks the frame's last submission.
    submit: fn(&RenderQueue) -> T,
    /// Blocks until a submission's GPU work is done.
    wait: fn(&RenderDevice, T),
}

/// Whether a frame must first wait for the oldest submission (pure): yes
/// once `in_flight` frames reach [`FRAMES_IN_FLIGHT`].
#[must_use]
const fn must_wait(in_flight: usize) -> bool {
    in_flight >= FRAMES_IN_FLIGHT
}

/// Before a frame takes its window image, waits for the frame before the
/// previous one, so at most two frames are ever queued.
fn wait_for_gpu<T: Send + Sync + 'static>(
    device: Res<RenderDevice>,
    mut in_flight: ResMut<InFlight<T>>,
) {
    while must_wait(in_flight.queued.len()) {
        let Some(oldest) = in_flight.queued.pop_front() else {
            break;
        };
        (in_flight.wait)(&device, oldest);
    }
}

/// Records this frame's submission and hands finished GPU work (screenshot
/// read-backs) to its callbacks without waiting.
fn record_submission<T: Send + Sync + 'static>(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut in_flight: ResMut<InFlight<T>>,
) {
    let index = (in_flight.submit)(&queue);
    in_flight.queued.push_back(index);
    let _ = device.poll(PollType::Poll);
}

/// Adds the pacing systems for `in_flight` to the render app.
fn add_pacing<T: Send + Sync + 'static>(render: &mut SubApp, in_flight: InFlight<T>) {
    render.insert_resource(in_flight);
    render.add_systems(
        Render,
        (
            wait_for_gpu::<T>
                .in_set(RenderSystems::PrepareViews)
                .before(prepare_windows),
            record_submission::<T>.in_set(RenderSystems::Cleanup),
        )
            .run_if(on_vulkan),
    );
}

/// Run condition: the adapter is Vulkan.
fn on_vulkan(info: Res<RenderAdapterInfo>) -> bool {
    paces_frames(Backends::from(info.backend))
}

/// Frame pacing for the window builds (see the module notes).
pub(crate) struct FramePacingPlugin;

impl Plugin for FramePacingPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            add_pacing(
                render,
                InFlight {
                    queued: std::collections::VecDeque::with_capacity(FRAMES_IN_FLIGHT + 1),
                    submit: |queue| queue.submit(std::iter::empty()),
                    wait: |device, index| {
                        let _ = device.poll(PollType::Wait {
                            submission_index: Some(index),
                            timeout: None,
                        });
                    },
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_builds_use_the_automatic_choice() {
        // #848 Q1 B: no backend is forced; the engine's default (every
        // backend it supports) decides, on Windows too.
        assert_eq!(
            backends_for(None, Some(Backends::all())),
            Some(Backends::all())
        );
        let code = include_str!("gpu.rs");
        let code = code.split("#[cfg(test)]").next().expect("code part");
        assert!(!code.contains("cfg!(windows)"), "no Windows special case");
    }

    #[test]
    fn override_wins_on_windows() {
        // #848 AC2: `WGPU_BACKEND=dx12` still forces DirectX 12.
        assert_eq!(
            backends_for(Some(Backends::DX12), Some(Backends::all())),
            Some(Backends::DX12)
        );
        assert_eq!(
            backends_for(Some(Backends::VULKAN), Some(Backends::all())),
            Some(Backends::VULKAN)
        );
    }

    #[test]
    fn at_most_two_frames_are_queued() {
        // #819: a frame waits once two are in flight, never before.
        assert!(!must_wait(0));
        assert!(!must_wait(1));
        assert!(must_wait(2));
    }

    #[test]
    fn only_vulkan_waits_for_the_gpu() {
        assert!(paces_frames(Backends::VULKAN));
        assert!(!paces_frames(Backends::DX12));
        assert!(!paces_frames(Backends::GL));
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
            assert!(
                code.contains(".set(task_pool_plugin())"),
                "{name} does not size the build threads"
            );
        }
    }
}
