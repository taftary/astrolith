//! The window side of `--route <file> <dir>` (#1022 T2): drives the camera
//! along a [`Route`] in real time and records every frame.
//!
//! On entering a pose leg the universe opens the demo journey to its level
//! (once per level) in dive mode; every frame then places the camera at the
//! leg's pose interpolated toward the next leg's ([`interpolate`]), with the
//! sky's time held there. A journey leg opens a fresh universe at L1 and
//! flies the autopilot. Each frame's interval and work ([`FrameWork`]) are
//! kept for the summary; each L10 leg also records how long the ground's
//! ring and its whole wanted set took to build from the leg's start.

use crate::frame_work::FrameWork;
use crate::route::{LegKind, Route, RouteFrame, interpolate};
use crate::shot_file::ShotSpec;
use bevy::prelude::*;

/// Frames kept without growing: 25 minutes at 60 frames a second.
const FRAMES_RESERVED: usize = 25 * 60 * 60;

/// State of a route run: present only then.
#[derive(Resource, Debug, Clone)]
pub struct RouteRun {
    /// The route.
    pub route: Route,
    /// Every recorded frame.
    pub frames: Vec<RouteFrame>,
    /// One `ROUTE-LEG` line per finished L10 leg.
    pub legs: Vec<String>,
    /// Milliseconds from `launched` to the first leg.
    pub launch_ms: f64,
    /// When the app was started, for the launch time.
    pub launched: Option<std::time::Instant>,
    /// The route has run to its end.
    pub done: bool,
    pub(crate) leg: usize,
    leg_t: f64,
    pub(crate) t: f64,
    started: bool,
    leg_frames: u32,
    entered: Option<usize>,
    level: Option<u8>,
    ring_ms: Option<f64>,
    ready_ms: Option<f64>,
}

impl RouteRun {
    /// A run of `route`, not started.
    #[must_use]
    pub fn new(route: Route) -> Self {
        Self {
            route,
            frames: Vec::with_capacity(FRAMES_RESERVED),
            legs: Vec::new(),
            launch_ms: 0.0,
            launched: None,
            leg_frames: 0,
            done: false,
            leg: 0,
            leg_t: 0.0,
            t: 0.0,
            started: false,
            entered: None,
            level: None,
            ring_ms: None,
            ready_ms: None,
        }
    }

    /// The same run, its launch timed from `at`.
    #[must_use]
    pub const fn launched_at(mut self, at: std::time::Instant) -> Self {
        self.launched = Some(at);
        self
    }

    /// The pose of the current leg now, if it is a pose leg (pure).
    fn pose_now(&self) -> Option<ShotSpec> {
        let leg = self.route.legs.get(self.leg)?;
        let LegKind::Pose(a) = &leg.kind else {
            return None;
        };
        let next = self.route.legs.get(self.leg + 1).map(|l| &l.kind);
        let b = match next {
            Some(LegKind::Pose(b)) => b,
            _ => a,
        };
        let t = if leg.secs > 0.0 {
            self.leg_t / leg.secs
        } else {
            1.0
        };
        Some(interpolate(a, b, t))
    }

    /// Closes the current leg: its `ROUTE-LEG` line when it was on the
    /// planet.
    fn close_leg(&mut self) {
        let Some(leg) = self.route.legs.get(self.leg) else {
            return;
        };
        // A zero-length leg is only a waypoint.
        if let LegKind::Pose(spec) = &leg.kind
            && spec.level == 10
            && leg.secs > 0.0
        {
            let ms = |t: Option<f64>| t.map_or_else(|| "none".to_string(), |ms| format!("{ms:.0}"));
            self.legs.push(format!(
                "ROUTE-LEG name={} scenario={} ring_ms={} ready_ms={}",
                leg.name,
                leg.scenario,
                ms(self.ring_ms),
                ms(self.ready_ms)
            ));
        }
    }

    /// Moves on by `dt` seconds, closing legs whose time is up; the end is
    /// a last leg of zero seconds or running out of legs.
    fn advance(&mut self, dt: f64) {
        self.t += dt;
        self.leg_t += dt;
        while let Some(leg) = self.route.legs.get(self.leg) {
            let last = self.leg + 1 == self.route.legs.len();
            if last && leg.secs == 0.0 {
                self.done = true;
                return;
            }
            if self.leg_t < leg.secs {
                return;
            }
            self.leg_t -= leg.secs;
            self.close_leg();
            self.leg += 1;
        }
        self.done = true;
    }
}

/// The optional resources the route touches: the body, the sky's clock and
/// the ground's readiness.
type RouteExtras<'w> = (
    Option<ResMut<'w, crate::player_state::PlayerState>>,
    Option<ResMut<'w, crate::sky::SkyClock>>,
    Option<Res<'w, crate::patches::GroundSurface>>,
);

/// Drives the route by one frame (see the module notes).
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn drive_route(
    mut run: ResMut<RouteRun>,
    time: Res<Time<Real>>,
    work: Option<Res<FrameWork>>,
    mut universe: ResMut<crate::Universe>,
    mut nav: ResMut<crate::input::Navigation>,
    mut flight: ResMut<crate::input::Flight>,
    mut autopilot: ResMut<crate::input::Autopilot>,
    mut extra: RouteExtras,
) {
    if run.done {
        return;
    }
    // Real time: the game's clock caps a frame at 250 ms, which would hide
    // a longer stall.
    let dt = f64::from(time.delta_secs());
    if run.started {
        let sample = work.as_ref().map_or_else(Default::default, |w| w.sample());
        let frame = RouteFrame {
            t_s: run.t,
            leg: run.leg,
            interval_ms: dt * 1000.0,
            work_ms: sample.work_ms(),
            main_ms: sample.main_ms,
            render_ms: sample.render_ms,
            wait_ms: sample.wait_ms,
        };
        run.frames.push(frame);
        // Readiness of the ground since the leg began (#918 AC3 signals):
        // the first frames place the camera and start the builds, so the
        // flags count from the third frame on, as in a timing run.
        run.leg_frames = run.leg_frames.saturating_add(1);
        if let Some(ground) = extra.2.as_ref().filter(|_| run.leg_frames >= 3) {
            let since = run.leg_t * 1000.0;
            if ground.1 && run.ring_ms.is_none() {
                run.ring_ms = Some(since);
            }
            if ground.2 && run.ready_ms.is_none() {
                run.ready_ms = Some(since);
            }
        }
        run.advance(dt);
        if run.done {
            *autopilot = crate::input::Autopilot::Idle;
            return;
        }
    } else {
        run.started = true;
        run.launch_ms = run.launched.map_or_else(
            || time.elapsed_secs_f64() * 1000.0,
            |at| at.elapsed().as_secs_f64() * 1000.0,
        );
    }
    let Some(leg) = run.route.legs.get(run.leg).cloned() else {
        return;
    };
    if run.entered != Some(run.leg) {
        run.entered = Some(run.leg);
        run.leg_frames = 0;
        run.ring_ms = None;
        run.ready_ms = None;
        if let Some(normal) = extra.0.as_mut() {
            normal.active = false;
        }
        flight.mode = crate::input::FlightMode::Dive;
        nav.target = None;
        match &leg.kind {
            LegKind::Journey => {
                crate::shot_place::open_to_level(&mut universe, 1);
                run.level = None;
                *autopilot = crate::input::Autopilot::Flying;
            }
            LegKind::Pose(spec) => {
                *autopilot = crate::input::Autopilot::Idle;
                if run.level != Some(spec.level) {
                    crate::shot_place::open_to_level(&mut universe, spec.level);
                    run.level = Some(spec.level);
                }
            }
        }
    }
    let Some(spec) = run.pose_now() else {
        return;
    };
    let game_s = spec.time_h * 3_600.0;
    if let Some(clock) = extra.1.as_mut() {
        clock.0.game_s = game_s;
        clock.0.playing = false;
    }
    universe.0.advance(game_s);
    let planet = crate::shot_place::planet_of(&universe);
    if let Some(pose) = crate::shot_place::pose_for(&spec, planet.as_ref().map(|(f, r)| (f, r))) {
        universe.0.path.set_offset(pose.offset);
        nav.forward = pose.forward;
        nav.up = pose.up;
    }
}

/// Route mode: drives a [`RouteRun`] inserted by the app.
pub struct RoutePlugin;

impl Plugin for RoutePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, drive_route.run_if(resource_exists::<RouteRun>));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::route::parse_route;

    const ROUTE: &str = "\
a L10 lat=0 lon=0 h=1000 heading=0 for=2 scenario=one
b L10 lat=0 lon=10 h=10 heading=90 for=1 scenario=two
c L4 x=0 y=0 z=1 dir=0,0,-1 for=0 scenario=end
";

    #[test]
    fn the_run_walks_the_legs_in_real_time() {
        let mut run = RouteRun::new(parse_route(ROUTE).expect("reads"));
        let spec = run.pose_now().expect("pose");
        assert_eq!(
            spec.place,
            crate::shot_file::ShotPlace::Planet {
                lat: 0.0,
                lon: 0.0,
                h: 1000.0
            }
        );
        run.advance(1.0);
        let crate::shot_file::ShotPlace::Planet { lon, h, .. } =
            run.pose_now().expect("pose").place
        else {
            panic!("planet");
        };
        assert_eq!(lon, 5.0);
        assert!((h - 100.0).abs() < 1e-9, "{h}");
        // A slow frame carries over into the next leg; a leg into another
        // level holds its pose.
        run.ring_ms = Some(800.0);
        run.advance(1.5);
        assert_eq!((run.leg, run.leg_t), (1, 0.5));
        assert_eq!(
            run.legs,
            ["ROUTE-LEG name=a scenario=one ring_ms=800 ready_ms=none"]
        );
        assert_eq!(
            run.pose_now().expect("pose").place,
            crate::shot_file::ShotPlace::Planet {
                lat: 0.0,
                lon: 10.0,
                h: 10.0
            }
        );
        assert!(!run.done);
        run.advance(0.5);
        assert!(run.done, "the last leg of zero seconds ends the run");
    }
}
