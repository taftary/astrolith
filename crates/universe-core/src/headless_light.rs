//! Headless `LIGHT` line for light and atmosphere (rebuild step 7, #780
//! Q6 A).
//!
//! On the demo Earth, under its fixed sun, the camera runs the #777
//! scripted path: 2,000 km down to 1 m and back up at 60 frames a second
//! (height shrinking by `e^(-1/60)` a frame), over a morning point on the
//! day side. Every frame the one atmosphere model
//! ([`crate::atmosphere::sample`]) gives the sky straight overhead, the
//! ground below and the haze toward the horizon. The largest change
//! between two frames, as a share of that quantity's range over the whole
//! path, must stay under [`STEP_LIMIT`](crate::headless_light::STEP_LIMIT):
//! no jump at any height. The night-side ground against the day-side
//! ground, 1 m above each, gives `night_ratio` (Q2 A). The whole run is
//! repeated and must match bit for bit. Deterministic (`E-DET-TIERS`), std
//! only.

use crate::atmosphere::{AirModel, SkySample, sample};
use crate::nav::{DEMO_SEED, MarkerIndex, Universe};

/// Largest frame-to-frame change as a share of the path's range (Q6 A).
pub const STEP_LIMIT: f64 = 0.02;

/// Night-to-day ground brightness bounds (Q2 A).
pub const NIGHT_RATIO_BOUNDS: (f64, f64) = (0.05, 0.10);

/// Angle of the morning point from the sub-solar point, in radians.
const MORNING_RAD: f64 = 0.7;

/// One `LIGHT` run's numbers.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LightMetrics {
    /// Largest sky-overhead step (any channel), share of its range.
    pub sky_step: f64,
    /// Largest ground step (luminance), share of its range.
    pub ground_step: f64,
    /// Largest haze step (luminance), share of its range.
    pub haze_step: f64,
    /// Night ground over day ground, 1 m above each.
    pub night_ratio: f64,
    /// Frames on the path.
    pub frames: usize,
}

fn luminance(c: [f64; 3]) -> f64 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let len = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    [a[0] / len, a[1] / len, a[2] / len]
}

/// Largest step between neighbours as a share of the values' range.
fn worst_step(values: &[f64]) -> f64 {
    let (lo, hi) = values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(*v), hi.max(*v))
        });
    let range = hi - lo;
    if !(range.is_finite() && range > 0.0) {
        return 0.0;
    }
    values
        .windows(2)
        .map(|w| ((w[1] - w[0]) / range).abs())
        .fold(0.0, f64::max)
}

/// The demo Earth opened, its air, relief and fixed sun.
fn earth() -> Option<(AirModel, crate::relief::Relief, [f64; 3])> {
    let journey = crate::nest::autopilot_path(DEMO_SEED);
    let chain: &[u32] = journey.get(..9)?;
    let mut universe = Universe::new(DEMO_SEED);
    for &marker in chain {
        if !universe.open(MarkerIndex(marker)) {
            return None;
        }
    }
    let sun = crate::light::sun_direction(&universe)?;
    let air = AirModel::for_planet(DEMO_SEED, chain)?;
    Some((
        air,
        crate::relief::Relief::for_planet(DEMO_SEED, chain),
        sun,
    ))
}

/// The scripted heights above the ground: down from 2,000 km to 1 m and
/// back, as the #777 path.
fn heights() -> Vec<f64> {
    let ratio = (-1.0f64 / 60.0).exp();
    let mut out = Vec::new();
    let mut height = 2_000_000.0f64;
    while height > 1.0 {
        out.push(height);
        height *= ratio;
    }
    out.push(1.0);
    let climb: Vec<f64> = out.iter().rev().skip(1).copied().collect();
    out.extend(climb);
    out
}

/// A point `angle` radians from `sun` toward any side.
fn tilted_from(sun: [f64; 3], angle: f64) -> [f64; 3] {
    let helper = if sun[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let d = helper[0] * sun[0] + helper[1] * sun[1] + helper[2] * sun[2];
    let side = unit([
        helper[0] - sun[0] * d,
        helper[1] - sun[1] * d,
        helper[2] - sun[2] * d,
    ]);
    let (s, c) = angle.sin_cos();
    unit([
        sun[0] * c + side[0] * s,
        sun[1] * c + side[1] * s,
        sun[2] * c + side[2] * s,
    ])
}

/// Runs the path and the day/night pair once.
#[must_use]
pub fn light_metrics() -> LightMetrics {
    let Some((air, relief, sun)) = earth() else {
        return LightMetrics {
            sky_step: f64::INFINITY,
            ..LightMetrics::default()
        };
    };
    let up = tilted_from(sun, MORNING_RAD);
    let ground_m = relief.height_m(up);
    let samples: Vec<SkySample> = heights()
        .into_iter()
        .map(|h| sample(Some(&air), up, ground_m + h, ground_m, sun))
        .collect();
    let channel = |i: usize| -> Vec<f64> {
        samples
            .iter()
            .map(|s| s.overhead.get(i).copied().unwrap_or(0.0))
            .collect()
    };
    let sky_step = (0..3).map(|i| worst_step(&channel(i))).fold(0.0, f64::max);
    let ground: Vec<f64> = samples.iter().map(|s| luminance(s.ground)).collect();
    let haze: Vec<f64> = samples.iter().map(|s| luminance(s.haze)).collect();
    let night_dir = [-sun[0], -sun[1], -sun[2]];
    let day_ground = relief.height_m(sun);
    let night_ground = relief.height_m(night_dir);
    let day = luminance(sample(Some(&air), sun, day_ground + 1.0, day_ground, sun).ground);
    let night =
        luminance(sample(Some(&air), night_dir, night_ground + 1.0, night_ground, sun).ground);
    LightMetrics {
        sky_step,
        ground_step: worst_step(&ground),
        haze_step: worst_step(&haze),
        night_ratio: if day > 0.0 { night / day } else { 0.0 },
        frames: samples.len(),
    }
}

/// Checks the path twice and prints one `LIGHT` line (percent steps).
pub fn verify_light(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let m = light_metrics();
    let repeat = light_metrics() == m;
    let (lo, hi) = NIGHT_RATIO_BOUNDS;
    let passed = m.sky_step < STEP_LIMIT
        && m.ground_step < STEP_LIMIT
        && m.haze_step < STEP_LIMIT
        && (lo..=hi).contains(&m.night_ratio)
        && m.frames > 0
        && repeat;
    let _ = writeln!(
        out,
        "LIGHT frames={} sky_step_pct={:.3} ground_step_pct={:.3} haze_step_pct={:.3} night_ratio={:.3} repeat={} {}",
        m.frames,
        m.sky_step * 100.0,
        m.ground_step * 100.0,
        m.haze_step * 100.0,
        m.night_ratio,
        if repeat { "identical" } else { "DIFFERENT" },
        if passed { "PASS" } else { "FAIL" },
    );
    passed
}
