//! A frame's CPU work without its waits (#1022 T1, Spec v3 R1).
//!
//! The frame interval mixes work with waiting: for the GPU (Vulkan pacing,
//! #819), for the window image (vsync), and later for the frame cap. The
//! quality controller and the timing runs need the work alone. Two threads
//! work on a frame at once (Bevy's pipelined rendering), so the work is the
//! longer of the two, never their sum:
//!
//! - main: the main world's schedules, from `First` to `Last`; the hand-over
//!   to the render thread and the event loop lie outside them;
//! - render: the render schedule from its first set to its last, minus the
//!   wait region, which runs from [`FrameWorkSet::WaitBegins`] (the GPU
//!   pacing wait comes after it) to just after the window image is taken
//!   (`prepare_windows`).
//!
//! The present call stays inside the render work: it does not block on
//! this PC's Vulkan driver, where the image acquire does. The render thread
//! hands its numbers over through atomics, so the main world reads the last
//! finished render frame's. No allocation per frame (`E-HOT-NOALLOC`).

use bevy::prelude::*;
use bevy::render::view::window::prepare_windows;
use bevy::render::{Render, RenderApp, RenderSystems};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// One frame's times in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WorkSample {
    /// Main-world schedules.
    pub main_ms: f64,
    /// Render schedule without its waits.
    pub render_ms: f64,
    /// Render waits: for the GPU and for the window image.
    pub wait_ms: f64,
    /// The frame cap's sleep in the main world: idle CPU, never work
    /// (#1022 T7).
    pub cap_ms: f64,
}

impl WorkSample {
    /// The frame's work: the longer of the two threads (pure).
    #[must_use]
    pub const fn work_ms(&self) -> f64 {
        self.main_ms.max(self.render_ms)
    }
}

/// Render-thread numbers in nanoseconds, handed to the main world.
#[derive(Debug, Default)]
struct Shared {
    render_ns: AtomicU64,
    wait_ns: AtomicU64,
}

/// The frame work clock in the main world.
#[derive(Resource, Debug, Clone, Default)]
pub struct FrameWork {
    shared: Arc<Shared>,
    /// When this frame's `First` began.
    begun: Option<Instant>,
    /// The last finished main-world frame, in milliseconds.
    main_ms: f64,
    /// The frame cap's last sleep, in milliseconds.
    cap_ms: f64,
}

impl FrameWork {
    /// The last finished frame on each thread.
    #[must_use]
    pub fn sample(&self) -> WorkSample {
        let ms = |a: &AtomicU64| nanos_to_ms(a.load(Ordering::Relaxed));
        WorkSample {
            main_ms: self.main_ms,
            render_ms: ms(&self.shared.render_ns),
            wait_ms: ms(&self.shared.wait_ns),
            cap_ms: self.cap_ms,
        }
    }

    /// Records the frame cap's sleep (#1022 T7).
    pub const fn set_cap_ms(&mut self, ms: f64) {
        self.cap_ms = ms;
    }
}

/// The render-world side of the clock.
#[derive(Resource, Debug)]
struct RenderClock {
    shared: Arc<Shared>,
    frame: Option<Instant>,
    wait: Option<Instant>,
    waited_ns: u64,
}

/// Where the render wait region begins: systems that wait on the GPU before
/// the window image order themselves after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum FrameWorkSet {
    /// Start of the wait region (before the GPU pacing wait).
    WaitBegins,
    /// The end of the main world's work in `Last`: the frame cap sleeps
    /// after it (#1022 T7).
    MainEnds,
}

#[expect(
    clippy::cast_precision_loss,
    reason = "E-CAST: nanoseconds of one frame fit a float's exact range"
)]
fn nanos_to_ms(ns: u64) -> f64 {
    ns as f64 / 1e6
}

fn nanos(since: Instant) -> u64 {
    u64::try_from(since.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn main_begins(mut work: ResMut<FrameWork>) {
    work.begun = Some(Instant::now());
}

fn main_ends(mut work: ResMut<FrameWork>) {
    if let Some(begun) = work.begun.take() {
        work.main_ms = nanos_to_ms(nanos(begun));
    }
}

fn render_begins(mut clock: ResMut<RenderClock>) {
    clock.frame = Some(Instant::now());
    clock.waited_ns = 0;
}

fn wait_begins(mut clock: ResMut<RenderClock>) {
    clock.wait = Some(Instant::now());
}

fn wait_ends(mut clock: ResMut<RenderClock>) {
    if let Some(wait) = clock.wait.take() {
        clock.waited_ns = clock.waited_ns.saturating_add(nanos(wait));
    }
}

fn render_ends(mut clock: ResMut<RenderClock>) {
    if let Some(frame) = clock.frame.take() {
        let total = nanos(frame);
        let shared = &clock.shared;
        shared
            .render_ns
            .store(total.saturating_sub(clock.waited_ns), Ordering::Relaxed);
        shared.wait_ns.store(clock.waited_ns, Ordering::Relaxed);
    }
}

/// Measures every frame's work (see the module notes).
pub struct FrameWorkPlugin;

impl Plugin for FrameWorkPlugin {
    fn build(&self, app: &mut App) {
        let work = FrameWork::default();
        let shared = Arc::clone(&work.shared);
        app.insert_resource(work)
            .add_systems(First, main_begins)
            .add_systems(Last, main_ends.in_set(FrameWorkSet::MainEnds));
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .insert_resource(RenderClock {
                    shared,
                    frame: None,
                    wait: None,
                    waited_ns: 0,
                })
                .configure_sets(
                    Render,
                    FrameWorkSet::WaitBegins
                        .in_set(RenderSystems::PrepareViews)
                        .before(prepare_windows),
                )
                .add_systems(
                    Render,
                    (
                        render_begins.in_set(RenderSystems::ExtractCommands),
                        wait_begins.in_set(FrameWorkSet::WaitBegins),
                        wait_ends
                            .in_set(RenderSystems::PrepareViews)
                            .after(prepare_windows),
                        render_ends.in_set(RenderSystems::PostCleanup),
                    ),
                );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_work_is_the_longer_thread_not_the_sum() {
        let sample = WorkSample {
            main_ms: 7.0,
            render_ms: 11.0,
            wait_ms: 22.0,
            cap_ms: 3.0,
        };
        assert_eq!(sample.work_ms(), 11.0);
        assert_eq!(WorkSample::default().work_ms(), 0.0);
        assert_eq!(nanos_to_ms(2_500_000), 2.5);
    }

    #[test]
    fn the_main_world_reads_the_render_threads_numbers() {
        let work = FrameWork::default();
        work.shared.render_ns.store(9_000_000, Ordering::Relaxed);
        work.shared.wait_ns.store(30_000_000, Ordering::Relaxed);
        let sample = work.sample();
        assert_eq!((sample.render_ms, sample.wait_ms), (9.0, 30.0));
    }
}
