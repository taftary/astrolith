//! Auto quality in the game (#1022 T10, Spec v4 points 4 and 7-9).
//!
//! In Auto, every frame of active play feeds the controller
//! ([`universe_core::quality_controller`]) with its interval (real time,
//! waits included) and its work ([`FrameWork`], waits left out), and the
//! controller's quality becomes the [`EffectiveGraphics`] and the
//! [`RenderScale`]. It starts at the floor at launch and on each switch
//! from Manual. It pauses in the menus and while the window is hidden, and
//! settles two seconds after; a paused sky clock is no pause. A level
//! change voids the comparison in progress. The ground's memory
//! protection (a coarsening bias or a shrunk ring) is the memory
//! restriction. Every change is logged.
//!
//! Added by the window runs only (`--route` and the plain window); without
//! it the saved choices drive the picture in both modes.

use crate::effective::EffectiveGraphics;
use crate::frame_work::{FrameWork, FrameWorkSet};
use crate::menu::Screen;
use crate::render_scale::RenderScale;
use crate::{SettingsStore, Universe};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowOccluded};
use universe_core::quality::{Direction, Limits};
use universe_core::quality_controller::Controller;
use universe_core::quality_window::FrameSample;
use universe_core::settings::{QualityMode, TargetFps, Tier};

/// The Auto controller and what it last saw of the game.
#[derive(Resource, Debug)]
pub struct AutoQuality {
    /// The controller.
    pub controller: Controller,
    /// The mode last frame (`None` before the first).
    mode: Option<QualityMode>,
    target: TargetFps,
    ceiling: Tier,
    native_short: u32,
    level: Option<u8>,
    hidden: bool,
}

impl Default for AutoQuality {
    fn default() -> Self {
        AutoQuality {
            controller: Controller::new(TargetFps::default(), Limits::new(Tier::default(), 0)),
            mode: None,
            target: TargetFps::default(),
            ceiling: Tier::default(),
            native_short: 0,
            level: None,
            hidden: false,
        }
    }
}

/// The word for a direction in the log.
const fn way(dir: Direction) -> &'static str {
    match dir {
        Direction::Down => "down",
        Direction::Up => "up",
    }
}

/// Feeds the frame to the controller and applies its quality (see the
/// module notes).
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
fn drive_auto(
    store: Option<Res<SettingsStore>>,
    screen: Option<Res<State<Screen>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut occlusion: MessageReader<WindowOccluded>,
    (time, work): (Res<Time<Real>>, Option<Res<FrameWork>>),
    (universe, tasks): (Res<Universe>, Option<Res<crate::patches::PatchTasks>>),
    mut auto: ResMut<AutoQuality>,
    (mut effective, mut scale): (ResMut<EffectiveGraphics>, ResMut<RenderScale>),
) {
    for event in occlusion.read() {
        auto.hidden = event.occluded;
    }
    let Some(store) = store else {
        return;
    };
    let saved = &store.0;
    let short = windows
        .single()
        .map_or(0, |w| w.physical_width().min(w.physical_height()));
    if saved.quality_mode == QualityMode::Manual {
        auto.mode = Some(QualityMode::Manual);
        return;
    }
    let auto = &mut *auto;
    if auto.mode != Some(QualityMode::Auto) {
        // Launch in Auto, or a switch from Manual: the floor (point 4).
        auto.controller = Controller::new(saved.target_fps, Limits::new(saved.auto_ceiling, short));
        auto.mode = Some(QualityMode::Auto);
        auto.target = saved.target_fps;
        auto.ceiling = saved.auto_ceiling;
        auto.native_short = short;
        info!("auto quality: starts at the floor");
    }
    let c = &mut auto.controller;
    if saved.target_fps != auto.target {
        auto.target = saved.target_fps;
        c.set_target(saved.target_fps);
    }
    if saved.auto_ceiling != auto.ceiling {
        auto.ceiling = saved.auto_ceiling;
        c.set_ceiling(saved.auto_ceiling);
    }
    if short != auto.native_short && short > 0 {
        auto.native_short = short;
        c.resize(short);
    }
    let level = universe.level().get();
    if auto.level.is_some_and(|l| l != level) {
        c.discard();
    }
    auto.level = Some(level);
    let in_menus = screen.is_some_and(|s| *s.get() != Screen::Universe);
    c.set_paused(in_menus || auto.hidden);
    // The ground exists at the planet only; elsewhere its state is a reset
    // or a leftover and means no pressure.
    c.set_memory_pressure(
        level == 10
            && tasks
                .is_some_and(|t| t.detail_bias > 1.0 || t.ring_m < universe_core::lod_ring::RING_M),
    );
    let interval_ms = time.delta_secs_f64() * 1000.0;
    if interval_ms > 0.0 {
        let frame = FrameSample {
            interval_ms,
            work_ms: work.map(|w| w.sample().work_ms()),
        };
        if let Some(change) = c.observe(frame) {
            info!(
                "auto quality: {} {}{}",
                change.control.name(),
                way(change.dir),
                if change.rollback {
                    " (rolled back)"
                } else {
                    ""
                }
            );
        }
    }
    let quality = *c.quality();
    if effective.0 != quality {
        effective.0 = quality;
    }
    let want = quality.scale(c.limits());
    if scale.0 != want {
        scale.0 = want;
    }
}

/// Auto quality for the window runs.
pub struct AutoQualityPlugin;

impl Plugin for AutoQualityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutoQuality>()
            .add_systems(Last, drive_auto.after(FrameWorkSet::MainEnds));
    }
}
