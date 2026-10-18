//! Headless `RELIEF` line for the terrain relief (rebuild step 3b, #799, T4).
//!
//! Split from [`crate::headless`] under `E-FILE-SIZE` (`headless.rs` is 1100
//! lines). Prints one `RELIEF` line with the highest and lowest point, land
//! share, height modes, gravity bound, and seam/pole checks on Earth, plus
//! `PASS`/`FAIL`. Byte-identical on every run (`E-DET-TIERS`).

use crate::cube_sphere::{Face, grid_point};
use crate::relief::Relief;

/// Fibonacci sphere direction `i` of `n` (deterministic, no tables).
fn fibonacci_dir(i: usize, n: usize) -> [f64; 3] {
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: headless sphere indices below 2^53, exact enough"
    )]
    let fi = i as f64;
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: headless sphere counts below 2^53, exact enough"
    )]
    let fn_ = n as f64;
    let y = 1.0 - 2.0 * (fi + 0.5) / fn_;
    let r = (1.0 - y * y).max(0.0).sqrt();
    let phi = fi * 2.399963229728653;
    [r * phi.cos(), y, r * phi.sin()]
}

/// Earth relief for the journey home cell.
fn earth_relief() -> Relief {
    let journey = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
    let home_chain: &[u32] = journey.get(..9).unwrap_or(&[]);
    Relief::for_planet(crate::nav::DEMO_SEED, home_chain)
}

/// Checks the relief and appends one `RELIEF` line.
///
/// Highest/lowest (metres, 1 decimal), land share (3 decimals), modes,
/// gravity bound on a fixed seed list, seam mismatches (must be zero), and
/// pole-ring continuity. Returns whether every check passes.
#[must_use]
pub fn verify_relief(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let relief = earth_relief();
    // Global sample on Earth for highest/lowest/land/modes.
    const N: usize = 20_000;
    let mut highest: f64 = f64::NEG_INFINITY;
    let mut lowest: f64 = f64::INFINITY;
    let mut land = 0usize;
    let mut heights: Vec<f64> = Vec::with_capacity(N);
    for i in 0..N {
        let h = relief.height_m(fibonacci_dir(i, N));
        if h.is_finite() {
            if h > highest {
                highest = h;
            }
            if h < lowest {
                lowest = h;
            }
            if h > 0.0 {
                land += 1;
            }
            heights.push(h);
        }
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: headless sample counts below 2^53, exact enough"
    )]
    let land_share = land as f64 / N as f64;
    // Modes via a 30-bin histogram with 3-bin smoothing (same as the T2 test).
    const BINS: usize = 30;
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: headless bin count 30 is exactly representable"
    )]
    let bins_f = BINS as f64;
    let mut counts = vec![0usize; BINS];
    let span = (highest - lowest).max(1.0);
    for h in &heights {
        let t = ((h - lowest) / span).clamp(0.0, 1.0);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: headless histogram bins below 30, exact"
        )]
        #[expect(
            clippy::cast_sign_loss,
            reason = "E-CAST: histogram share is non-negative"
        )]
        let mut bin = (t * bins_f) as usize;
        if bin >= BINS {
            bin = BINS - 1;
        }
        if let Some(slot) = counts.get_mut(bin) {
            *slot += 1;
        }
    }
    let mut smooth = vec![0.0f64; BINS];
    for i in 0..BINS {
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: headless histogram counts below 2^53, exact enough"
        )]
        let mut s = counts.get(i).copied().unwrap_or(0) as f64;
        let mut n = 1.0f64;
        if i > 0 {
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: headless histogram counts below 2^53, exact enough"
            )]
            let prev = counts.get(i - 1).copied().unwrap_or(0) as f64;
            s += prev;
            n += 1.0;
        }
        if i + 1 < BINS {
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: headless histogram counts below 2^53, exact enough"
            )]
            let next = counts.get(i + 1).copied().unwrap_or(0) as f64;
            s += next;
            n += 1.0;
        }
        if let Some(slot) = smooth.get_mut(i) {
            *slot = s / n;
        }
    }
    let max_smooth = smooth.iter().cloned().fold(0.0f64, f64::max);
    let mut modes = 0usize;
    for i in 0..BINS {
        let cur = smooth.get(i).copied().unwrap_or(0.0);
        let left = if i > 0 {
            smooth.get(i - 1).copied().unwrap_or(0.0)
        } else {
            0.0
        };
        let right = if i + 1 < BINS {
            smooth.get(i + 1).copied().unwrap_or(0.0)
        } else {
            0.0
        };
        if cur > left && cur >= right && cur > max_smooth * 0.10 {
            modes += 1;
        }
    }
    // Gravity bound on a fixed seed list per rocky class.
    let mut gravity_ok = true;
    for seed in [1u64, 42, 999] {
        for radius_earth in [0.3f64, 1.0f64] {
            let probe = Relief::for_recipe(seed, radius_earth);
            let bound = 9_000.0 * probe.relief_scale();
            for i in 0..2000 {
                let h = probe.height_m(fibonacci_dir(i, 2000));
                if !h.is_finite() || h > bound {
                    gravity_ok = false;
                    break;
                }
            }
            if !gravity_ok {
                break;
            }
        }
        if !gravity_ok {
            break;
        }
    }
    // Seam agreement bit for bit (N=64, same pairs as the T2 test).
    const S: u32 = 64;
    let mut seams = 0u32;
    let mut pair = |a: Option<[f64; 3]>, b: Option<[f64; 3]>| match (a, b) {
        (Some(pa), Some(pb)) => {
            if relief.height_m(pa).to_bits() != relief.height_m(pb).to_bits() {
                seams += 1;
            }
        }
        _ => {
            seams += 1;
        }
    };
    for k in 0..=S {
        pair(
            grid_point(Face::PosX, S, k, S),
            grid_point(Face::PosY, S, k, S),
        );
        pair(
            grid_point(Face::PosX, 0, k, S),
            grid_point(Face::NegY, S, k, S),
        );
        pair(
            grid_point(Face::NegX, S, k, S),
            grid_point(Face::PosY, 0, k, S),
        );
        pair(
            grid_point(Face::NegX, 0, k, S),
            grid_point(Face::NegY, 0, k, S),
        );
        pair(
            grid_point(Face::PosX, k, S, S),
            grid_point(Face::PosZ, S, k, S),
        );
        pair(
            grid_point(Face::PosX, k, 0, S),
            grid_point(Face::NegZ, S, k, S),
        );
        pair(
            grid_point(Face::NegX, k, S, S),
            grid_point(Face::PosZ, 0, k, S),
        );
        pair(
            grid_point(Face::NegX, k, 0, S),
            grid_point(Face::NegZ, 0, k, S),
        );
        pair(
            grid_point(Face::PosY, k, S, S),
            grid_point(Face::PosZ, k, S, S),
        );
        pair(
            grid_point(Face::PosY, k, 0, S),
            grid_point(Face::NegZ, k, S, S),
        );
        pair(
            grid_point(Face::NegY, k, S, S),
            grid_point(Face::PosZ, k, 0, S),
        );
        pair(
            grid_point(Face::NegY, k, 0, S),
            grid_point(Face::NegZ, k, 0, S),
        );
    }
    // Pole rings: 128 steps at 2 degrees, no step over 2 km.
    let mut poles_ok = true;
    for sign in [1.0f64, -1.0] {
        let colat = 2.0 * std::f64::consts::PI / 180.0;
        let (sin_c, cos_c) = (colat.sin(), colat.cos());
        let mut prev: Option<f64> = None;
        for i in 0..=128u32 {
            let phi = f64::from(i) / 128.0 * 2.0 * std::f64::consts::PI;
            let dir = [sin_c * phi.cos(), sign * cos_c, sin_c * phi.sin()];
            let h = relief.height_m(dir);
            if !h.is_finite() {
                poles_ok = false;
                break;
            }
            if let Some(p) = prev
                && (h - p).abs() >= 2_000.0
            {
                poles_ok = false;
                break;
            }
            prev = Some(h);
        }
        if !poles_ok {
            break;
        }
    }
    let land_ok = (0.20..=0.45).contains(&land_share);
    let modes_ok = modes == 2;
    let high_ok = highest.is_finite() && highest <= 9_000.0;
    let low_ok = lowest.is_finite() && (-13_000.0..=-9_000.0).contains(&lowest);
    let seams_ok = seams == 0;
    let passed = land_ok && modes_ok && high_ok && low_ok && gravity_ok && seams_ok && poles_ok;
    let flag = |b: bool| if b { "PASS" } else { "FAIL" };
    let _ = writeln!(
        out,
        "RELIEF highest={highest:.1} lowest={lowest:.1} land={land_share:.3} modes={modes} gravity={} seams={seams} poles={} {}",
        flag(gravity_ok),
        flag(poles_ok),
        flag(passed),
    );
    passed
}
