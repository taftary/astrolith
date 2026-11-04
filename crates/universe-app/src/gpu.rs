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
//! next, and no more than two frames are ever queued. The developer
//! prototype switch (#1085 T3, [`crate::prototype`]) may change that limit,
//! turn the wait off, or let it adapt between one and two with VSync on
//! (#1085 T6, [`crate::adaptive`]); without it the plugin is exactly the
//! #819 pacing.
//!
//! Threads ([`task_pool_plugin`], #918 T8): ground patches build on the
//! async compute pool, which Bevy sizes at a quarter of the cores (two on
//! the dev PC). Three quarters of the cores build patches, one thread
//! serves file I/O, and the rest run the frame: on the dev PC the frame
//! time stayed at the `main` baseline while the ring got ready about twice
//! as fast as with half (#918). At least two threads stay with the frame
//! (#1022 T1): with one, the render world's wait for the window image
//! stalled the main world. `--build-workers <n>` sets the build
//! threads to exactly `n` instead, for the phone-like timing run (#994).

use bevy::prelude::*;
use bevy::render::render_resource::PollType;
use bevy::render::renderer::{RenderAdapterInfo, RenderDevice, RenderQueue};
use bevy::render::settings::{Backends, WgpuSettings};
use bevy::render::view::window::{ExtractedWindows, prepare_windows};
use bevy::render::{Render, RenderApp, RenderPlugin, RenderSystems};
use bevy::window::PresentMode;

/// Backends to use (pure): the override, else the engine's automatic
/// default, on every platform (#848 Q1 B).
#[must_use]
fn backends_for(env_override: Option<Backends>, default: Option<Backends>) -> Option<Backends> {
    env_override.or(default)
}

/// Share of the cores that build ground patches (#918 T8).
const BUILD_SHARE: f32 = 0.75;

/// Threads always left to the frame (#1022 T1). With one, the render
/// world's wait for the window image held the only frame thread, and the
/// main world's systems queued behind it: on the dev PC the main schedules
/// took the whole 40 ms frame for about 7 ms of work.
const FRAME_THREADS: usize = 2;

/// Most ground build threads on `cores` (pure): all but the file I/O
/// thread and [`FRAME_THREADS`], at least one.
#[must_use]
const fn max_build_threads(cores: usize) -> usize {
    let left = cores.saturating_sub(1 + FRAME_THREADS);
    if left == 0 { 1 } else { left }
}

/// Thread pools for the window builds (#918 T8): three quarters of the
/// cores for ground patch builds, one for file I/O, the rest for the frame,
/// never fewer than [`FRAME_THREADS`] (#1022 T1). `workers` sets the build
/// threads to exactly that many instead (#994).
#[must_use]
pub(crate) fn task_pool_plugin(workers: Option<usize>) -> TaskPoolPlugin {
    let mut options = TaskPoolOptions::default();
    options.io.max_threads = 1;
    options.async_compute.percent = BUILD_SHARE;
    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    options.async_compute.max_threads = max_build_threads(cores);
    if let Some(n) = workers {
        options.async_compute.min_threads = n;
        options.async_compute.max_threads = n;
    }
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

/// Submissions of the last frames, oldest first (Vulkan pacing). Generic
/// over the engine's submission index, which Bevy does not re-export: the
/// two functions fix it, so no direct graphics dependency is added.
#[derive(Resource)]
struct InFlight<T> {
    /// Submissions not yet waited for, oldest first.
    queued: std::collections::VecDeque<T>,
    /// Frames allowed in flight before a frame waits.
    limit: usize,
    /// The adaptive limit (#1085 T6) and when the last frame began.
    adaptive: Option<(crate::adaptive::Adaptive, Option<std::time::Instant>)>,
    /// Marks the frame's last submission.
    submit: fn(&RenderQueue) -> T,
    /// Blocks until a submission's GPU work is done.
    wait: fn(&RenderDevice, T),
}

/// Whether a frame must first wait for the oldest submission (pure): yes
/// once `in_flight` frames reach `limit` (2 in the #819 baseline).
#[must_use]
const fn must_wait(in_flight: usize, limit: usize) -> bool {
    in_flight >= limit
}

/// Before a frame takes its window image, waits until fewer than the limit
/// are in flight (the baseline: the frame before the previous one, so at
/// most two frames are ever queued).
fn wait_for_gpu<T: Send + Sync + 'static>(
    device: Res<RenderDevice>,
    mut in_flight: ResMut<InFlight<T>>,
    windows: Option<Res<ExtractedWindows>>,
) {
    if let Some((adaptive, last)) = in_flight.adaptive.as_mut() {
        let now = std::time::Instant::now();
        let vsync = windows.is_some_and(|w| w.windows.values().any(|w| vsync_on(w.present_mode)));
        let interval_ms = last.map_or(0.0, |t| now.duration_since(t).as_secs_f64() * 1000.0);
        *last = Some(now);
        let before = adaptive.limit();
        let limit = adaptive.frame(interval_ms, vsync);
        if limit != before {
            eprintln!("ADAPTIVE limit={limit} changes={}", adaptive.changes);
        }
        in_flight.limit = limit;
    }
    while must_wait(in_flight.queued.len(), in_flight.limit) {
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
            // Inside the frame work's wait region (#1022 T1).
            wait_for_gpu::<T>
                .in_set(RenderSystems::PrepareViews)
                .after(universe_render::FrameWorkSet::WaitBegins)
                .before(prepare_windows),
            record_submission::<T>.in_set(RenderSystems::Cleanup),
        )
            .run_if(on_vulkan),
    );
}

/// Whether `mode` waits for the display's refresh (pure): VSync on.
#[must_use]
const fn vsync_on(mode: PresentMode) -> bool {
    matches!(
        mode,
        PresentMode::AutoVsync | PresentMode::Fifo | PresentMode::FifoRelaxed
    )
}

/// Run condition: the adapter is Vulkan.
fn on_vulkan(info: Res<RenderAdapterInfo>) -> bool {
    paces_frames(Backends::from(info.backend))
}

/// Frame pacing for the window builds (see the module notes).
pub(crate) struct FramePacingPlugin {
    /// Frames allowed in flight before a frame waits; `None` adds no pacing
    /// (a prototype, #1085 T3).
    pub in_flight: Option<usize>,
    /// Whether the limit adapts between one and two (#1085 T6).
    pub adaptive: bool,
}

impl Default for FramePacingPlugin {
    /// The #819 pacing: two frames in flight.
    fn default() -> Self {
        Self {
            in_flight: Some(crate::prototype::BASELINE_IN_FLIGHT),
            adaptive: false,
        }
    }
}

impl Plugin for FramePacingPlugin {
    fn build(&self, app: &mut App) {
        let Some(limit) = self.in_flight else {
            return;
        };
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            add_pacing(
                render,
                InFlight {
                    queued: std::collections::VecDeque::with_capacity(limit + 1),
                    limit,
                    adaptive: self
                        .adaptive
                        .then(|| (crate::adaptive::Adaptive::default(), None)),
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
    fn the_frame_keeps_two_threads() {
        // #1022 T1: the dev PC's eight threads give five to the builds
        // (three quarters would be six), one to file I/O, two to the frame.
        assert_eq!(max_build_threads(8), 5);
        assert_eq!(max_build_threads(16), 13);
        assert_eq!(max_build_threads(4), 1);
        assert_eq!(max_build_threads(2), 1);
    }

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
        let base = FramePacingPlugin::default().in_flight.expect("paced");
        assert_eq!(base, 2);
        assert!(!must_wait(0, base));
        assert!(!must_wait(1, base));
        assert!(must_wait(2, base));
        // A prototype limit (#1085 T3) moves the wait, nothing else.
        assert!(!must_wait(2, 3));
        assert!(must_wait(3, 3));
    }

    #[test]
    fn adaptive_reads_vsync_from_the_present_mode() {
        // #1085 T6: AutoVsync is the VSync-on setting, AutoNoVsync off.
        assert!(vsync_on(PresentMode::AutoVsync));
        assert!(vsync_on(PresentMode::Fifo));
        assert!(!vsync_on(PresentMode::AutoNoVsync));
        assert!(!vsync_on(PresentMode::Immediate));
        assert!(!vsync_on(PresentMode::Mailbox));
        assert!(!FramePacingPlugin::default().adaptive, "baseline is fixed");
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
                code.contains(".set(task_pool_plugin("),
                "{name} does not size the build threads"
            );
        }
        // Normal play, route runs and pictures pace through the prototype
        // switch (#1085 T3, T5), which is the #819 pacing when no prototype
        // is set.
        for (name, source) in [
            ("main.rs", include_str!("main.rs")),
            ("route.rs", include_str!("route.rs")),
            ("screens.rs", include_str!("screens.rs")),
        ] {
            let code = source.split("#[cfg(test)]").next().expect("code part");
            assert!(
                code.contains("prototype.pacing()"),
                "{name} does not pace its frames"
            );
            assert!(
                code.contains("desired_maximum_frame_latency: prototype.latency"),
                "{name} ignores the prototype's frame latency"
            );
        }
    }
}
