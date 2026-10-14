//! Rivers-first networks per land region, with deltas (#685, T6).
//!
//! Guides: `guides/universe/L10/generation/documents/landforms.md`
//! (Strahler order, Hack's law, meanders, Galloway/Nienhuis deltas) and
//! the rivers-first technique. Each land region grows its courses from
//! the coast (or a rift lake) inland: mouths snap to the shoreline,
//! sources sit on the high ground, tributaries join downstream with
//! Strahler order, meanders grow on flat reaches, and basin areas fit
//! Hack's law. The water surface along a course is monotonic by
//! construction (an interpolation from source to mouth); the ground
//! carves toward it in a V, so valleys read downhill. T7 colours the
//! water, T11 ages the basins.
//!
//! Everything is `f64` and deterministic (`E-DET-TIERS`); courses build
//! once per region and are cached with it (`E-HOT-NOALLOC`).

use super::lakes::{RiftLake, region_sample_dirs};
use super::plates::{Plates, unit01_share};
use super::relief::base_unclamped_m;
use crate::icosphere::normalize;
use std::f64::consts::TAU;

/// Most rivers per land region.
pub const MAX_RIVERS: usize = 5;

/// Most tributaries per river.
pub const MAX_TRIBS: usize = 3;

/// Course step in radians (about 50 km at Earth size).
pub const COURSE_STEP_RAD: f64 = 0.008;

/// Most course points per stem.
pub const MAX_COURSE_POINTS: u32 = 80;

/// Fewest course points per stem.
pub const MIN_COURSE_POINTS: u32 = 12;

/// Base meander amplitude in radians.
pub const MEANDER_AMP_RAD: f64 = 0.004;

/// Meander cap in radians (flat reaches wander furthest).
pub const MEANDER_MAX_RAD: f64 = 0.012;

/// Gentle slope for full meanders (rise over run).
pub const MEANDER_FLAT_SLOPE: f64 = 0.0015;

/// Meander wavelength in channel widths (Leopold and Wolman, GEN-S058).
pub const MEANDER_WAVELENGTH_WIDTHS: f64 = 10.0;

/// Hack's-law exponent (Gray 1961, GEN-S057).
pub const HACK_EXPONENT: f64 = 0.568;

/// Hack's-law constant for kilometres (fit once, per the guides).
pub const HACK_CONSTANT: f64 = 1.0;

/// Hack fit tolerance for grown basins (length within 35 percent).
pub const HACK_TOLERANCE: f64 = 0.35;

/// Valley wall slope (rise over run of the V).
pub const VALLEY_WALL_SLOPE: f64 = 0.06;

/// Fan deposit height in metres (river-dominated deltas).
pub const FAN_M: f64 = 3.5;

/// Wave-arc deposit height in metres.
pub const WAVE_FAN_M: f64 = 2.0;

/// Tide-estuary carve depth in metres.
pub const ESTUARY_M: f64 = 8.0;

/// Stream tag of the river draws.
const RIVER_TAG: u64 = 0x91BE_9685_0000_0000;

/// Delta kind by dominant force (Galloway triangle, GEN-S059).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DeltaKind {
    /// Elongated bird's-foot (the Mississippi).
    River,
    /// Smooth arc shore (the Nile).
    Wave,
    /// Estuary bays with flow-aligned islands (the Ganges).
    Tide,
}

/// Delta class from the Nienhuis ratios (GEN-S071): tidal flux over river
/// above one reads tide-dominated; river over waves above one reads
/// river-dominated; the rest is wave-dominated.
#[must_use]
pub(crate) fn classify_delta(river_wave: f64, tide_river: f64) -> DeltaKind {
    if tide_river > 1.0 {
        DeltaKind::Tide
    } else if river_wave > 1.0 {
        DeltaKind::River
    } else {
        DeltaKind::Wave
    }
}

/// Main-stream length in kilometres for a basin of `area_km2` (Hack's
/// law, `L = c A^h`).
#[must_use]
#[cfg(test)]
fn hack_length_km(area_km2: f64) -> f64 {
    if !area_km2.is_finite() || area_km2 <= 0.0 {
        return 0.0;
    }
    HACK_CONSTANT * area_km2.powf(HACK_EXPONENT)
}

/// One watercourse: meandered points with a falling floor and orders.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Course {
    /// Points from source to mouth (unit directions).
    pub(crate) points: Vec<[f64; 3]>,
    /// Water surface in unscaled base metres (monotonic by construction).
    pub(crate) floors: Vec<f64>,
    /// Strahler order per point.
    pub(crate) orders: Vec<u8>,
}

/// One river: a main stem with tributaries draining one basin.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct River {
    /// Main stem from the highlands to the mouth.
    pub(crate) stem: Course,
    /// Tributaries with their join index on the stem.
    pub(crate) tribs: Vec<(Course, usize)>,
    /// Areas of the tributary sub-basins in km².
    pub(crate) trib_areas_km2: Vec<f64>,
    /// Drained basin area in km² (tributaries included).
    pub(crate) basin_area_km2: f64,
    /// Mouth width in metres.
    pub(crate) width_mouth_m: f64,
    /// Deepest valley carve in metres.
    pub(crate) depth_max_m: f64,
    /// Mouth (unit direction on the coast or a lake rim).
    pub(crate) mouth: [f64; 3],
    /// Delta class at a sea mouth (`None` into a lake).
    pub(crate) delta: Option<DeltaKind>,
    /// Rift-lake index targeted (`None` for the sea).
    pub(crate) lake_target: Option<usize>,
}

/// Angular distance between unit directions, in radians.
fn ang_dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
        .clamp(-1.0, 1.0)
        .acos()
}

/// Polyline length in radians.
#[cfg(test)]
fn polyline_len(points: &[[f64; 3]]) -> f64 {
    let mut total = 0.0;
    for pair in points.windows(2) {
        let (Some(a), Some(b)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        total += ang_dist(*a, *b);
    }
    total
}

/// Great-arc interpolation from `a` to `b` at `t` (slerp).
fn slerp(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    let cos = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
    let angle = cos.acos();
    if angle < 1e-9 {
        return normalize([
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
        ]);
    }
    let (s0, s1) = (((1.0 - t) * angle).sin(), (t * angle).sin());
    let denom = angle.sin();
    normalize([
        (a[0] * s0 + b[0] * s1) / denom,
        (a[1] * s0 + b[1] * s1) / denom,
        (a[2] * s0 + b[2] * s1) / denom,
    ])
}

/// One watercourse plan: endpoints, floors, order and width.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CourseSpec {
    /// Highland source (unit direction).
    source: [f64; 3],
    /// Mouth on the coast or a lake rim (unit direction).
    mouth: [f64; 3],
    /// Water surface at the source in unscaled base metres.
    floor0: f64,
    /// Water surface at the mouth in unscaled base metres.
    floor1: f64,
    /// Strahler order at the source.
    order0: u8,
    /// Mouth width in metres.
    width_m: f64,
    /// Point count.
    count: u32,
}

/// Meandered course from `spec.source` to `spec.mouth`: a great arc bent
/// sideways by a sine (zero at both ends, so mouth and source hold),
/// wavelength about ten channel widths on flat reaches (GEN-S058).
/// Floors run from source to mouth monotonically; orders start at
/// `spec.order0`.
fn meandered_course(seed: u64, lane: u64, radius_m: f64, spec: CourseSpec) -> Course {
    let arc = ang_dist(spec.source, spec.mouth).max(1e-6);
    let axis = normalize([
        spec.source[1] * spec.mouth[2] - spec.source[2] * spec.mouth[1],
        spec.source[2] * spec.mouth[0] - spec.source[0] * spec.mouth[2],
        spec.source[0] * spec.mouth[1] - spec.source[1] * spec.mouth[0],
    ]);
    let len_m = arc * radius_m;
    let slope = (spec.floor0 - spec.floor1).max(0.0) / len_m.max(1.0);
    let flat_bonus = if slope < MEANDER_FLAT_SLOPE { 2.5 } else { 1.0 };
    let step_m = len_m / f64::from(spec.count.max(2) - 1);
    let period = (MEANDER_WAVELENGTH_WIDTHS * spec.width_m / step_m.max(1.0))
        .round()
        .max(4.0);
    // Meander amplitude stays a small share of the wavelength, so grown
    // courses fit Hack's law without wild bends.
    let cap = (0.15 * period * step_m / radius_m).min(MEANDER_MAX_RAD);
    let amp = (MEANDER_AMP_RAD * flat_bonus).min(cap);
    let phase = TAU * unit01_share(seed, RIVER_TAG, lane);
    let mut points = Vec::new();
    let mut floors = Vec::new();
    for i in 0..spec.count {
        let t = f64::from(i) / f64::from(spec.count.max(2) - 1);
        let pos = slerp(spec.source, spec.mouth, t);
        // Sideways bend off the great arc (zero at both ends, so mouth
        // and source hold).
        let side = normalize([
            axis[1] * pos[2] - axis[2] * pos[1],
            axis[2] * pos[0] - axis[0] * pos[2],
            axis[0] * pos[1] - axis[1] * pos[0],
        ]);
        let bend =
            amp * (f64::from(i) / period * TAU + phase).sin() * (t * std::f64::consts::PI).sin();
        let pos = normalize([
            pos[0] + side[0] * bend,
            pos[1] + side[1] * bend,
            pos[2] + side[2] * bend,
        ]);
        points.push(pos);
        floors.push(spec.floor0 + (spec.floor1 - spec.floor0) * t);
    }
    if let Some(first) = floors.first_mut() {
        *first = spec.floor0;
    }
    if let Some(last) = floors.last_mut() {
        *last = spec.floor1;
    }
    let count_points = floors.len();
    Course {
        points,
        floors,
        orders: vec![spec.order0; count_points],
    }
}

/// Snap a hashed direction to the shoreline: scan the meridian, then a
/// parallel, for a land-to-sea sign change in the base relief, and bisect
/// it. `None` with no crossing nearby (deep ocean or deep interior).
fn snap_to_coast(seed: u64, dir: [f64; 3], kind: super::classify::WorldKind) -> Option<[f64; 3]> {
    let lat0 = dir[1].clamp(-1.0, 1.0).asin();
    let lon0 = dir[2].atan2(dir[0]);
    for pass in 0..2u32 {
        let mut prev: Option<([f64; 3], f64)> = None;
        for i in 0..32u32 {
            let s = -1.0 + 2.0 * f64::from(i) / 31.0;
            let (lat, lon) = if pass == 0 {
                (s * 1.4, lon0)
            } else {
                (lat0, s * std::f64::consts::PI)
            };
            let ring = (1.0 - lat.sin() * lat.sin()).max(0.0).sqrt();
            let point = normalize([ring * lon.cos(), lat.sin(), ring * lon.sin()]);
            let height = base_unclamped_m(seed, point, kind);
            if let Some((at, was)) = prev
                && (was > 0.0) != (height > 0.0)
            {
                let mut lo = at;
                let mut hi = point;
                for _ in 0..6u32 {
                    let mid = normalize([
                        (lo[0] + hi[0]) * 0.5,
                        (lo[1] + hi[1]) * 0.5,
                        (lo[2] + hi[2]) * 0.5,
                    ]);
                    if (base_unclamped_m(seed, mid, kind) > 0.0) == (was > 0.0) {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                return Some(normalize([
                    (lo[0] + hi[0]) * 0.5,
                    (lo[1] + hi[1]) * 0.5,
                    (lo[2] + hi[2]) * 0.5,
                ]));
            }
            prev = Some((point, height));
        }
    }
    None
}

/// Rivers of one land region: mouths on the coast (or a rift lake),
/// sources on the high ground, tributaries with Strahler order, basin
/// areas fitting Hack's law, delta classes at sea mouths.
#[must_use]
pub(crate) fn build_rivers(
    seed: u64,
    region: usize,
    radius_m: f64,
    field: Option<&Plates>,
    rift_lakes: &[RiftLake],
    kind: super::classify::WorldKind,
) -> Vec<River> {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return Vec::new();
    }
    let land: Vec<[f64; 3]> = region_sample_dirs(field, seed, region, 96, RIVER_TAG)
        .into_iter()
        .filter(|dir| base_unclamped_m(seed, *dir, kind) > 0.0)
        .collect();
    if land.len() < 4 {
        return Vec::new();
    }
    // Mouths snapped to the coast, kept apart and in-region.
    let mut mouths = Vec::new();
    for dir in region_sample_dirs(field, seed, region, 24, RIVER_TAG + 1) {
        if mouths.len() >= MAX_RIVERS {
            break;
        }
        let Some(shore) = snap_to_coast(seed, dir, kind) else {
            continue;
        };
        let owned = field.is_none_or(|plates| {
            super::plates::plate_index_at(plates, shore).is_some_and(|index| index == region)
        });
        if !owned {
            continue;
        }
        if mouths.iter().any(|kept| ang_dist(*kept, shore) < 0.05) {
            continue;
        }
        mouths.push(shore);
    }
    if mouths.is_empty() {
        return Vec::new();
    }
    // Basin areas from arc lengths (Hack inverted): a river drains what
    // its length implies, so the grown basins fit the law with bounded
    // meanders; undrained interfluves read as dry gaps.
    let mut rivers = Vec::new();
    for (r, mouth) in mouths.iter().enumerate() {
        if rivers.len() >= MAX_RIVERS {
            break;
        }
        let lane = r as u64 * 16;
        let mut peak = land.first().copied().unwrap_or([0.0, 1.0, 0.0]);
        let mut peak_height = f64::NEG_INFINITY;
        for dir in region_sample_dirs(field, seed, region, 48, RIVER_TAG + 10 + r as u64) {
            let height = base_unclamped_m(seed, dir, kind);
            if height > peak_height {
                peak = dir;
                peak_height = height;
            }
        }
        let source = peak;
        // Lake target or the sea.
        let mut lake_target = None;
        let mut end = *mouth;
        let mut floor1 = -25.0;
        let mut delta = None;
        if !rift_lakes.is_empty() && unit01_share(seed, RIVER_TAG, lane) < 0.35 {
            let mut near = (0usize, f64::INFINITY);
            for (i, lake) in rift_lakes.iter().enumerate() {
                let gap = ang_dist(source, lake.centre);
                if gap < near.1 {
                    near = (i, gap);
                }
            }
            let Some(lake) = rift_lakes.get(near.0).copied() else {
                continue;
            };
            let dot = source[0] * lake.centre[0]
                + source[1] * lake.centre[1]
                + source[2] * lake.centre[2];
            let toward = normalize([
                source[0] - lake.centre[0] * dot,
                source[1] - lake.centre[1] * dot,
                source[2] - lake.centre[2] * dot,
            ]);
            end = normalize([
                lake.centre[0] + toward[0] * lake.radius_ang,
                lake.centre[1] + toward[1] * lake.radius_ang,
                lake.centre[2] + toward[2] * lake.radius_ang,
            ]);
            floor1 = lake.level_m;
            lake_target = Some(near.0);
        }
        let arc = ang_dist(source, end).max(1e-6);
        // Basin area from the arc length (Hack inverted): the river drains
        // what its length implies, so the grown course fits the law with
        // bounded meanders; the rest stays interfluve.
        let basin = (arc * radius_m / 1_000.0)
            .powf(1.0 / HACK_EXPONENT)
            .max(1.0e3);
        if lake_target.is_none() {
            let lat = mouth[1].clamp(-1.0, 1.0).asin();
            // Proxies (landforms guide): runoff from the drained area,
            // waves from the latitude storm belt, tides from a seeded
            // resonance. Scaled so most rivers read wave-dominated with
            // about a tenth each tide- and river-dominated (GEN-S071).
            let runoff = (basin / 4.0e6).powf(0.7);
            let waves = 0.2 + 0.8 * (2.0 * lat).sin().abs();
            let tides = 0.5 * unit01_share(seed, RIVER_TAG, lane + 1);
            delta = Some(classify_delta(
                runoff / waves.max(0.05),
                tides / runoff.max(0.05),
            ));
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: course points below 80 always fit"
        )]
        #[expect(
            clippy::cast_sign_loss,
            reason = "E-CAST: course points non-negative, saturates like `as`"
        )]
        let count = ((arc / COURSE_STEP_RAD) as u32).clamp(MIN_COURSE_POINTS, MAX_COURSE_POINTS);
        let width_mouth_m = 150.0 + 550.0 * unit01_share(seed, RIVER_TAG, lane + 2);
        let floor0 = base_unclamped_m(seed, source, kind) + 300.0;
        let stem = meandered_course(
            seed,
            lane + 3,
            radius_m,
            CourseSpec {
                source,
                mouth: end,
                floor0,
                floor1,
                order0: 1,
                width_m: width_mouth_m,
                count,
            },
        );
        let mut river = River {
            basin_area_km2: basin,
            trib_areas_km2: Vec::new(),
            width_mouth_m,
            depth_max_m: 180.0,
            mouth: end,
            delta,
            lake_target,
            stem,
            tribs: Vec::new(),
        };
        // Tributaries join downstream with Strahler order (gated attempts
        // keep branching varied; every attempt either joins or is skipped).
        for t in 0..MAX_TRIBS {
            if unit01_share(seed, RIVER_TAG, lane + 4 + t as u64) < 0.25 {
                continue;
            }
            let join_t = 0.35 + 0.4 * unit01_share(seed, RIVER_TAG, lane + 8 + t as u64);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: join index below 80 always fits"
            )]
            #[expect(
                clippy::cast_sign_loss,
                reason = "E-CAST: join index non-negative, saturates like `as`"
            )]
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: course points below 80, exactly representable"
            )]
            let join_at = (join_t * river.stem.points.len() as f64) as usize;
            let Some(join_pos) = river.stem.points.get(join_at).copied() else {
                continue;
            };
            let join_floor = river.stem.floors.get(join_at).copied().unwrap_or(floor1);
            let side = tangent_side(&river.stem.points, join_at);
            let mut option: Option<[f64; 3]> = None;
            for s in 0..2u32 {
                let reach = 0.05
                    + 0.1 * unit01_share(seed, RIVER_TAG, lane + 8 + t as u64 * 2 + u64::from(s));
                let corner = normalize([
                    join_pos[0] + side[0] * reach * if s == 0 { 1.0 } else { -1.0 },
                    join_pos[1] + side[1] * reach * if s == 0 { 1.0 } else { -1.0 },
                    join_pos[2] + side[2] * reach * if s == 0 { 1.0 } else { -1.0 },
                ]);
                if base_unclamped_m(seed, corner, kind) > -50.0
                    && option.is_none_or(|best: [f64; 3]| {
                        base_unclamped_m(seed, corner, kind) > base_unclamped_m(seed, best, kind)
                    })
                {
                    option = Some(corner);
                }
            }
            let Some(origin) = option else {
                continue;
            };
            let trib_arc = ang_dist(origin, join_pos).max(1e-6);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: course points below 80 always fit"
            )]
            #[expect(
                clippy::cast_sign_loss,
                reason = "E-CAST: course points non-negative, saturates like `as`"
            )]
            let trib_count_pts = ((trib_arc / COURSE_STEP_RAD) as u32).clamp(8, MAX_COURSE_POINTS);
            let trib = meandered_course(
                seed,
                lane + 20 + t as u64,
                radius_m,
                CourseSpec {
                    source: origin,
                    mouth: join_pos,
                    floor0: base_unclamped_m(seed, origin, kind) + 150.0,
                    floor1: join_floor,
                    order0: 1,
                    width_m: width_mouth_m * 0.4,
                    count: trib_count_pts,
                },
            );
            let order_here = river.stem.orders.get(join_at).copied().unwrap_or(1);
            for o in river.stem.orders.iter_mut().skip(join_at) {
                *o = (*o).max(order_here + u8::from(order_here == 1));
            }
            river.trib_areas_km2.push(basin * 0.25);
            river
                .tribs
                .push((trib, join_at.min(river.stem.points.len().saturating_sub(1))));
        }
        let mouth_order = river.stem.orders.last().copied().unwrap_or(1);
        river.depth_max_m = 120.0 + 60.0 * f64::from(mouth_order);
        rivers.push(river);
    }
    rivers
}

/// Sideways unit direction at course point `at` (for tributary sources).
fn tangent_side(points: &[[f64; 3]], at: usize) -> [f64; 3] {
    let before = points
        .get(at.saturating_sub(1))
        .copied()
        .unwrap_or([1.0, 0.0, 0.0]);
    let after = points.get(at + 1).copied().unwrap_or([-1.0, 0.0, 0.0]);
    let along = normalize([
        after[0] - before[0],
        after[1] - before[1],
        after[2] - before[2],
    ]);
    let radial = points.get(at).copied().unwrap_or([0.0, 1.0, 0.0]);
    normalize([
        along[1] * radial[2] - along[2] * radial[1],
        along[2] * radial[0] - along[0] * radial[2],
        along[0] * radial[1] - along[1] * radial[0],
    ])
}

/// Lowest channel target in unscaled base metres near `direction`
/// (`None` past every valley): the V floor plus wall rise. Landforms
/// take the minimum with the ground, so water reads downhill.
#[must_use]
pub(crate) fn channel_for_rivers(
    rivers: &[River],
    direction: [f64; 3],
    radius_m: f64,
) -> Option<f64> {
    if !direction.iter().all(|c| c.is_finite()) || !radius_m.is_finite() || radius_m <= 0.0 {
        return None;
    }
    let unit = normalize(direction);
    let mut best: Option<f64> = None;
    let mut consider =
        |points: &[[f64; 3]], floors: &[f64], width_mouth_m: f64, depth_max_m: f64| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: course points below 80, exactly representable"
            )]
            let last = points.len().saturating_sub(1) as f64;
            for (i, point) in points.iter().enumerate() {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "E-CAST: course points below 80, exactly representable"
                )]
                let t = if last > 0.0 { i as f64 / last } else { 1.0 };
                let width = width_mouth_m * (0.3 + 0.7 * t.sqrt());
                let reach = (width * 6.0 + depth_max_m / VALLEY_WALL_SLOPE) / radius_m + 0.002;
                let gap = (unit[0] - point[0]).powi(2)
                    + (unit[1] - point[1]).powi(2)
                    + (unit[2] - point[2]).powi(2);
                if gap > reach * reach {
                    continue;
                }
                let floor = floors.get(i).copied().unwrap_or(0.0);
                let lateral_m = gap.sqrt().min(reach) * radius_m;
                let target = floor + VALLEY_WALL_SLOPE * lateral_m;
                best = Some(best.map_or(target, |held: f64| held.min(target)));
            }
        };
    for river in rivers {
        consider(
            &river.stem.points,
            &river.stem.floors,
            river.width_mouth_m,
            river.depth_max_m,
        );
        for (trib, _) in &river.tribs {
            consider(&trib.points, &trib.floors, river.width_mouth_m * 0.4, 120.0);
        }
    }
    best
}

/// Fan and estuary deposits in metres at `direction` (`0.0` away from
/// every mouth): slight lobes at sea mouths, a drowned funnel for tides.
#[must_use]
pub(crate) fn fan_for_rivers(rivers: &[River], direction: [f64; 3], radius_m: f64) -> f64 {
    if !direction.iter().all(|c| c.is_finite()) || !radius_m.is_finite() || radius_m <= 0.0 {
        return 0.0;
    }
    let unit = normalize(direction);
    let mut sum = 0.0;
    for river in rivers {
        let Some(delta) = river.delta else {
            continue;
        };
        let rho = ang_dist(unit, river.mouth);
        let fan_r = 12.0 * river.width_mouth_m / radius_m;
        if rho > fan_r * 3.0 {
            continue;
        }
        let u = rho / fan_r;
        match delta {
            DeltaKind::River => sum += FAN_M * (-u * u).exp(),
            DeltaKind::Wave => sum += WAVE_FAN_M * (-u * u).exp(),
            DeltaKind::Tide => sum += 1.0 * (-u * u).exp(),
        }
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A region with rivers: first mobile seed/region with two open rivers.
    fn river_region() -> (u64, usize, Vec<River>) {
        for seed in 0..200u64 {
            let input = super::super::plates::tectonic_input_for(seed);
            if !matches!(
                super::super::plates::regime_for(seed, &input),
                super::super::plates::Regime::Mobile
            ) {
                continue;
            }
            let Some(field) = super::super::plates::plate_field_for(
                seed,
                super::super::plates::EARTH_RADIUS_M,
                &input,
            ) else {
                continue;
            };
            for region in 0..field.count {
                let lakes = super::super::lakes::find_rift_lakes(
                    seed,
                    region,
                    super::super::plates::EARTH_RADIUS_M,
                    Some(&field),
                    0.6,
                    super::super::classify::WorldKind::EarthLike,
                );
                let rivers = build_rivers(
                    seed,
                    region,
                    super::super::plates::EARTH_RADIUS_M,
                    Some(&field),
                    &lakes,
                    super::super::classify::WorldKind::EarthLike,
                );
                let open = rivers.iter().filter(|r| r.lake_target.is_none()).count();
                if rivers.len() >= 2 && open >= 1 {
                    return (seed, region, rivers);
                }
            }
        }
        panic!("no river region in range");
    }

    #[test]
    fn classify_delta_follows_nienhuis() {
        assert_eq!(classify_delta(0.2, 2.0), DeltaKind::Tide);
        assert_eq!(classify_delta(3.0, 0.2), DeltaKind::River);
        assert_eq!(classify_delta(0.4, 0.4), DeltaKind::Wave);
        assert_eq!(hack_length_km(0.0), 0.0);
        assert_eq!(hack_length_km(f64::NAN), 0.0);
        // The Amazon (~6.1e6 km², ~6,400 km) lands near the law.
        let amazon = hack_length_km(6.1e6);
        assert!(
            (3_000.0..12_000.0).contains(&amazon),
            "amazon reads {amazon} km"
        );
    }

    #[test]
    fn rivers_deltas_and_lakes_follow_rules() {
        // C9 / AC9: rivers run downhill to the sea or a closed lake;
        // deltas match their ratios; lakes sit flat at or below the rim.
        let (_seed, _, rivers) = river_region();
        let radius_m = super::super::plates::EARTH_RADIUS_M;
        // Water surfaces fall monotonically; mouths reach the sea.
        for river in &rivers {
            let mut prev = f64::INFINITY;
            for floor in &river.stem.floors {
                assert!(*floor <= prev + 1e-9, "stem rises downstream");
                prev = *floor;
            }
            let end = river.stem.floors.last().copied().unwrap_or(0.0);
            if river.lake_target.is_some() {
                assert!(
                    end.is_finite() && end > -1.0e6,
                    "lake mouth without a floor"
                );
            } else {
                assert!(
                    (end + 25.0).abs() < 1e-6,
                    "sea mouth misses sea level: {end}"
                );
                assert!(river.delta.is_some(), "sea mouth without a delta");
            }
            // Strahler orders grow downstream; meanders bend off the
            // great arc and hold the mouth.
            assert!(!river.stem.orders.is_empty());
            let mouth_order = river.stem.orders.last().copied().unwrap_or(1);
            assert!((1..=4).contains(&mouth_order), "order {mouth_order}");
            let n = river.stem.points.len();
            let (Some(first), Some(last)) = (river.stem.points.first(), river.stem.points.last())
            else {
                panic!("empty course");
            };
            let mut bend = 0.0f64;
            for (i, point) in river.stem.points.iter().enumerate() {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "E-CAST: course points below 80, exactly representable"
                )]
                let t = i as f64 / (n.saturating_sub(1).max(1)) as f64;
                bend = bend.max(ang_dist(*point, slerp(*first, *last, t)));
            }
            assert!(
                bend > 0.0005 && bend < 0.03,
                "meanders missing or wild: {bend}"
            );
        }
        // Basins fit Hack's law by construction (areas from arc lengths);
        // the check pins the constant and exponent against drift.
        let mut areas = 0.0;
        for river in &rivers {
            let length_km = polyline_len(&river.stem.points) * radius_m / 1_000.0;
            let want_km = hack_length_km(river.basin_area_km2);
            let ratio = length_km / want_km;
            assert!(
                (0.99..1.35).contains(&ratio),
                "basin breaks Hack: {length_km} vs {want_km} km"
            );
            areas += river.basin_area_km2;
            for (trib, _) in &river.tribs {
                let tribal = polyline_len(&trib.points) * radius_m / 1_000.0;
                assert!(
                    tribal < length_km,
                    "tributary outgrows its stem: {tribal} vs {length_km}"
                );
            }
        }
        assert!(areas > 0.0, "no drained area");
        // Endorheic targeting: with an explicit lake list, some river hits
        // the rim at exactly the lake level.
        let shore_lake = RiftLake {
            centre: normalize([0.2, 0.9, 0.1]),
            across: normalize([0.9, -0.2, 0.1]),
            radius_ang: 0.010,
            level_m: 640.0,
            kind: super::super::lakes::LakeKind::Fresh,
        };
        let mut targeted = 0u32;
        for seed in 0..40u64 {
            let input = super::super::plates::tectonic_input_for(seed);
            let Some(field) = super::super::plates::plate_field_for(
                seed,
                super::super::plates::EARTH_RADIUS_M,
                &input,
            ) else {
                continue;
            };
            for region in 0..field.count {
                for river in build_rivers(
                    seed,
                    region,
                    super::super::plates::EARTH_RADIUS_M,
                    Some(&field),
                    &[shore_lake],
                    super::super::classify::WorldKind::EarthLike,
                ) {
                    if river.lake_target.is_some() {
                        let end = river.stem.floors.last().copied().unwrap_or(0.0);
                        assert!(
                            (end - shore_lake.level_m).abs() < 1e-9,
                            "lake mouth misses its level: {end}"
                        );
                        assert!(river.delta.is_none(), "lake mouth wears a delta");
                        targeted += 1;
                    }
                }
            }
        }
        assert!(targeted > 0, "no river ever targets a lake");
        // Delta classes span the triangle over built rivers.
        let mut kinds = std::collections::BTreeSet::new();
        for seed in 0..40u64 {
            let input = super::super::plates::tectonic_input_for(seed);
            let Some(field) = super::super::plates::plate_field_for(
                seed,
                super::super::plates::EARTH_RADIUS_M,
                &input,
            ) else {
                continue;
            };
            for region in 0..field.count {
                for river in build_rivers(
                    seed,
                    region,
                    super::super::plates::EARTH_RADIUS_M,
                    Some(&field),
                    &[],
                    super::super::classify::WorldKind::EarthLike,
                ) {
                    if let Some(delta) = river.delta {
                        kinds.insert(delta as u8);
                    }
                }
            }
        }
        assert_eq!(kinds.len(), 3, "delta triangle missing a corner: {kinds:?}");
        // Lake rims hold: tested in lakes; endorheic targeting here.
    }
}
