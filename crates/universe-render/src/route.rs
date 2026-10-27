//! Route files and their summaries for `--route <file> <dir>` (#1022 T2,
//! Spec v3 R4).
//!
//! A route is a list of timed legs, each a shot line (`shot_file`) plus
//! `for=<seconds>` and `scenario=<name>`. The camera moves from a leg's
//! pose to the next leg's over `for` seconds of real time, the way a
//! player covers ground: a slow frame moves it further. Places, looks and
//! the sky's time are interpolated; a height grows or shrinks by a constant
//! factor, so a descent spends its time evenly across the scales. Between
//! two levels, or into a `journey` leg, the camera jumps. A `journey` leg
//! flies the autopilot from L1 for its seconds instead.
//!
//! ```text
//! target 60
//! set planet_detail = low
//! warm     L10 lat=10 lon=20 h=2 heading=0  for=60  scenario=warmup
//! zoom     journey                         for=120 scenario=universe
//! high     L10 lat=0 lon=0 h=2000000 heading=0 tilt=-90 for=120 scenario=descent
//! low      L10 lat=0 lon=0 h=2 heading=0           for=0   scenario=end
//! ```
//!
//! `target <fps>` sets the frame rate the summary judges against (60 by
//! default); `set <key> = <value>` lines are settings file lines applied to
//! the run's default settings. A `warmup` scenario is reported apart from
//! the scored time. Pure: the window side is `route_run`.

use crate::shot_file::{ShotLook, ShotPlace, ShotSpec, parse_line};

/// Scenario name kept out of the scored time.
pub const WARMUP: &str = "warmup";

/// What a leg does.
#[derive(Debug, Clone, PartialEq)]
pub enum LegKind {
    /// Move from this pose toward the next leg's.
    Pose(ShotSpec),
    /// Fly the autopilot journey from L1.
    Journey,
}

/// One leg of a route.
#[derive(Debug, Clone, PartialEq)]
pub struct Leg {
    /// Leg name, as in a shot file.
    pub name: String,
    /// Scenario the leg's frames count toward.
    pub scenario: String,
    /// Real seconds the leg lasts.
    pub secs: f64,
    /// What it does.
    pub kind: LegKind,
}

/// A parsed route file.
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    /// Frame rate the summary judges against.
    pub target_fps: f64,
    /// Settings file lines for the run (`set` lines).
    pub settings: String,
    /// The legs in order.
    pub legs: Vec<Leg>,
}

/// Reads a route file.
///
/// # Errors
///
/// A wrong line, named by its number, or a route with no legs.
pub fn parse_route(text: &str) -> Result<Route, String> {
    let mut route = Route {
        target_fps: 60.0,
        settings: String::new(),
        legs: Vec::new(),
    };
    for (number, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        parse_route_line(line, &mut route).map_err(|e| format!("line {}: {e}", number + 1))?;
    }
    if route.legs.is_empty() {
        return Err("the route has no legs".to_string());
    }
    Ok(route)
}

fn parse_route_line(line: &str, route: &mut Route) -> Result<(), String> {
    if let Some(rest) = line.strip_prefix("target ") {
        let fps = rest
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|f| *f >= 1.0 && *f <= 240.0);
        route.target_fps =
            fps.ok_or_else(|| format!("target '{}' must be 1 to 240", rest.trim()))?;
        return Ok(());
    }
    if let Some(rest) = line.strip_prefix("set ") {
        if !rest.contains('=') {
            return Err(format!("set '{rest}' needs key = value"));
        }
        route.settings.push_str(rest.trim());
        route.settings.push('\n');
        return Ok(());
    }
    let (mut secs, mut scenario) = (None, None);
    let mut shot_words = Vec::new();
    for word in line.split_whitespace() {
        if let Some(v) = word.strip_prefix("for=") {
            let s = v.parse::<f64>().ok().filter(|s| s.is_finite() && *s >= 0.0);
            secs = Some(s.ok_or_else(|| format!("for={v} must be seconds, 0 or more"))?);
        } else if let Some(v) = word.strip_prefix("scenario=") {
            scenario = Some(v.to_string());
        } else {
            shot_words.push(word);
        }
    }
    let secs = secs.ok_or("for=<seconds> is missing")?;
    let scenario = scenario
        .filter(|s| !s.is_empty())
        .ok_or("scenario=<name> is missing")?;
    let kind = match shot_words.as_slice() {
        [_, journey] if *journey == "journey" => LegKind::Journey,
        _ => LegKind::Pose(parse_line(&shot_words.join(" "))?),
    };
    let name = shot_words.first().copied().unwrap_or_default().to_string();
    route.legs.push(Leg {
        name,
        scenario,
        secs,
        kind,
    });
    Ok(())
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Heights in between by a constant factor; linear when one is zero.
fn lerp_height(a: f64, b: f64, t: f64) -> f64 {
    if a > 0.0 && b > 0.0 {
        a * (b / a).powf(t)
    } else {
        lerp(a, b, t)
    }
}

/// The pose a fraction `t` (0 to 1) of the way from `a` to `b` (pure).
/// Only like with like moves: across levels, or between a planet place
/// and raw coordinates, or two kinds of look, `a` holds.
#[must_use]
pub(crate) fn interpolate(a: &ShotSpec, b: &ShotSpec, t: f64) -> ShotSpec {
    let t = t.clamp(0.0, 1.0);
    let mut out = a.clone();
    if a.level != b.level {
        return out;
    }
    out.place = match (a.place, b.place) {
        (
            ShotPlace::Planet { lat, lon, h },
            ShotPlace::Planet {
                lat: lat2,
                lon: lon2,
                h: h2,
            },
        ) => ShotPlace::Planet {
            lat: lerp(lat, lat2, t),
            lon: lerp(lon, lon2, t),
            h: lerp_height(h, h2, t),
        },
        (ShotPlace::Xyz(p), ShotPlace::Xyz(q)) => ShotPlace::Xyz([
            lerp(p[0], q[0], t),
            lerp(p[1], q[1], t),
            lerp(p[2], q[2], t),
        ]),
        _ => return out,
    };
    out.look = match (a.look, b.look) {
        (
            ShotLook::HeadingTilt { heading, tilt },
            ShotLook::HeadingTilt {
                heading: h2,
                tilt: t2,
            },
        ) => ShotLook::HeadingTilt {
            heading: lerp(heading, h2, t),
            tilt: lerp(tilt, t2, t),
        },
        (ShotLook::Dir(p), ShotLook::Dir(q)) => ShotLook::Dir([
            lerp(p[0], q[0], t),
            lerp(p[1], q[1], t),
            lerp(p[2], q[2], t),
        ]),
        _ => a.look,
    };
    out.time_h = lerp(a.time_h, b.time_h, t);
    out
}

/// One recorded frame of a route run.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RouteFrame {
    /// Seconds since the first leg began.
    pub t_s: f64,
    /// Index of the leg.
    pub leg: usize,
    /// Frame interval, milliseconds.
    pub interval_ms: f64,
    /// CPU work without waits ([`crate::WorkSample`]), milliseconds.
    pub work_ms: f64,
    /// Main-world work, milliseconds.
    pub main_ms: f64,
    /// Render work, milliseconds.
    pub render_ms: f64,
    /// Waits, milliseconds.
    pub wait_ms: f64,
}

/// The `frames.txt` line of one frame.
#[must_use]
pub fn frame_line(frame: &RouteFrame, scenario: &str) -> String {
    format!(
        "{:.3} {scenario} {:.2} {:.2} {:.2} {:.2} {:.2}",
        frame.t_s, frame.interval_ms, frame.work_ms, frame.main_ms, frame.render_ms, frame.wait_ms
    )
}

/// Frames above this count as stalls in the summary, milliseconds.
pub const STALL_MS: f64 = 100.0;

/// Judged share: intervals within this factor of the target interval.
pub(crate) const WITHIN: f64 = 1.1;

/// Seconds at each end of the scored time reported on their own.
pub(crate) const END_SECS: f64 = 300.0;

/// Nearest-rank 95th percentile of `values` (0 when empty); sorts them.
fn p95(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let rank = (values.len() * 95).div_ceil(100).max(1) - 1;
    values.get(rank).copied().unwrap_or(0.0)
}

/// One summary line for `frames` named `label`, against `target_fps`.
#[must_use]
pub(crate) fn summary_line(label: &str, frames: &[&RouteFrame], target_fps: f64) -> String {
    let limit = 1000.0 / target_fps * WITHIN;
    let within = frames.iter().filter(|f| f.interval_ms <= limit).count();
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: frame counts as floats for a percentage, intended"
    )]
    let within_pct = if frames.is_empty() {
        0.0
    } else {
        within as f64 * 100.0 / frames.len() as f64
    };
    let mut intervals: Vec<f64> = frames.iter().map(|f| f.interval_ms).collect();
    let mut work: Vec<f64> = frames.iter().map(|f| f.work_ms).collect();
    let worst = intervals.iter().copied().fold(0.0, f64::max);
    let stalls = intervals.iter().filter(|ms| **ms > STALL_MS).count();
    format!(
        "{label} frames={} within_110_pct={within_pct:.1} p95_ms={:.2} worst_ms={worst:.2} over_100ms={stalls} work_p95_ms={:.2}",
        frames.len(),
        p95(&mut intervals),
        p95(&mut work),
    )
}

/// The summary lines of a run: the warm-up, each scenario in first-seen
/// order, the whole scored time, and its first and final five minutes.
#[must_use]
pub fn summary(route: &Route, frames: &[RouteFrame]) -> Vec<String> {
    let scenario_of = |f: &RouteFrame| {
        route
            .legs
            .get(f.leg)
            .map_or("", |leg| leg.scenario.as_str())
    };
    let mut names: Vec<&str> = Vec::new();
    for leg in &route.legs {
        if !names.contains(&leg.scenario.as_str()) {
            names.push(&leg.scenario);
        }
    }
    let mut lines = Vec::new();
    for name in names {
        let of: Vec<&RouteFrame> = frames.iter().filter(|f| scenario_of(f) == name).collect();
        if of.is_empty() {
            continue;
        }
        let tag = if name == WARMUP {
            "ROUTE-WARMUP"
        } else {
            "ROUTE-SCENARIO"
        };
        lines.push(summary_line(
            &format!("{tag} name={name}"),
            &of,
            route.target_fps,
        ));
    }
    let scored: Vec<&RouteFrame> = frames.iter().filter(|f| scenario_of(f) != WARMUP).collect();
    lines.push(summary_line("ROUTE-ALL", &scored, route.target_fps));
    if let (Some(first), Some(last)) = (scored.first(), scored.last()) {
        let (start, end) = (first.t_s, last.t_s);
        let head: Vec<&RouteFrame> = scored
            .iter()
            .copied()
            .filter(|f| f.t_s < start + END_SECS)
            .collect();
        let tail: Vec<&RouteFrame> = scored
            .iter()
            .copied()
            .filter(|f| f.t_s > end - END_SECS)
            .collect();
        lines.push(summary_line("ROUTE-FIRST5", &head, route.target_fps));
        lines.push(summary_line("ROUTE-FINAL5", &tail, route.target_fps));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROUTE: &str = "\
# a test route
target 60
set planet_detail = low
warm  L10 lat=0 lon=0 h=2 heading=0 for=2 scenario=warmup
zoom  journey for=3 scenario=universe
high  L10 lat=0 lon=0 h=2000000 heading=0 tilt=-90 time=6 for=4 scenario=descent
low   L10 lat=10 lon=20 h=2 heading=360 tilt=0 time=18 for=0 scenario=end
";

    #[test]
    fn a_route_reads_its_legs_target_and_settings() {
        let route = parse_route(ROUTE).expect("reads");
        assert_eq!(route.target_fps, 60.0);
        assert_eq!(route.settings, "planet_detail = low\n");
        assert_eq!(route.legs.len(), 4);
        assert_eq!(route.legs[1].kind, LegKind::Journey);
        assert_eq!(
            (route.legs[2].scenario.as_str(), route.legs[2].secs),
            ("descent", 4.0)
        );
        for (wrong, why) in [
            ("a L10 lat=0 lon=0 heading=0 scenario=x", "for="),
            ("a L10 lat=0 lon=0 heading=0 for=1", "scenario="),
            ("a L10 lat=0 lon=0 heading=0 for=-1 scenario=x", "for=-1"),
            ("target 0", "target"),
            ("a L11 x=0 y=0 z=0 dir=0,0,1 for=1 scenario=x", "L1 to L10"),
            ("", "no legs"),
        ] {
            let err = parse_route(wrong).expect_err(wrong);
            assert!(err.contains(why), "{wrong}: {err}");
        }
    }

    #[test]
    fn a_leg_moves_toward_the_next_evenly_across_scales() {
        let route = parse_route(ROUTE).expect("reads");
        let (LegKind::Pose(high), LegKind::Pose(low)) = (&route.legs[2].kind, &route.legs[3].kind)
        else {
            panic!("poses");
        };
        let mid = interpolate(high, low, 0.5);
        let ShotPlace::Planet { lat, lon, h } = mid.place else {
            panic!("planet");
        };
        assert_eq!((lat, lon), (5.0, 10.0));
        // Halfway in the factor: sqrt(2,000,000 x 2) = 2,000 m.
        assert!((h - 2_000.0).abs() < 1e-6, "{h}");
        assert_eq!(
            mid.look,
            ShotLook::HeadingTilt {
                heading: 180.0,
                tilt: -45.0
            }
        );
        assert_eq!(mid.time_h, 12.0);
        assert_eq!(interpolate(high, low, 0.0), *high);
        assert_eq!(interpolate(high, low, 2.0).time_h, 18.0);
        // Across levels the leg holds its pose.
        let space = parse_line("s L4 x=0 y=0 z=1 dir=0,0,-1").expect("reads");
        assert_eq!(interpolate(&space, low, 0.5), space);
    }

    #[test]
    fn the_summary_judges_each_scenario_and_the_whole() {
        let route = parse_route(ROUTE).expect("reads");
        let frame = |t_s, leg, interval_ms| RouteFrame {
            t_s,
            leg,
            interval_ms,
            work_ms: interval_ms / 2.0,
            ..RouteFrame::default()
        };
        let frames = [
            frame(0.0, 0, 200.0),
            frame(1.0, 1, 16.0),
            frame(2.0, 1, 18.0),
            frame(3.0, 1, 19.0),
            frame(4.0, 2, 120.0),
            frame(5.0, 2, 17.0),
        ];
        let lines = summary(&route, &frames);
        assert_eq!(
            lines,
            [
                "ROUTE-WARMUP name=warmup frames=1 within_110_pct=0.0 p95_ms=200.00 worst_ms=200.00 over_100ms=1 work_p95_ms=100.00",
                "ROUTE-SCENARIO name=universe frames=3 within_110_pct=66.7 p95_ms=19.00 worst_ms=19.00 over_100ms=0 work_p95_ms=9.50",
                "ROUTE-SCENARIO name=descent frames=2 within_110_pct=50.0 p95_ms=120.00 worst_ms=120.00 over_100ms=1 work_p95_ms=60.00",
                "ROUTE-ALL frames=5 within_110_pct=60.0 p95_ms=120.00 worst_ms=120.00 over_100ms=1 work_p95_ms=60.00",
                "ROUTE-FIRST5 frames=5 within_110_pct=60.0 p95_ms=120.00 worst_ms=120.00 over_100ms=1 work_p95_ms=60.00",
                "ROUTE-FINAL5 frames=5 within_110_pct=60.0 p95_ms=120.00 worst_ms=120.00 over_100ms=1 work_p95_ms=60.00",
            ]
        );
        // 18.333... ms is within 110 % of 60 FPS, unrounded (R4).
        let edge = [frame(1.0, 1, 1000.0 / 60.0 * 1.1)];
        assert!(summary(&route, &edge)[0].contains("within_110_pct=100.0"));
        assert_eq!(
            frame_line(&frames[1], "universe"),
            "1.000 universe 16.00 8.00 0.00 0.00 0.00"
        );
    }
}
