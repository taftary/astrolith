//! Plates: regimes, Voronoi partitions, Euler motions, boundary relief
//! (#685, T4).
//!
//! Guides: `guides/universe/L10/generation/documents/plates.md` (the
//! regime trigger, the Voronoi recipe, the boundary table, the ridge
//! `√age` law). T4 owns the trigger, the partition, the motions and the
//! boundary profiles; T5 stamps craters on top, T6 volcanoes and rift
//! lakes, T7 refines the ice regimes with climate, T10 the tidal heating,
//! T11 the age-driven activity fade. Until then the heat, activity and ice
//! inputs are seeded rolls, documented on each field of
//! [`TectonicInput`].
//!
//! Everything is `f64` and deterministic (`E-DET-TIERS`): the same seed
//! and direction always give the same regime, plates and relief. Plates
//! live in stack arrays ([`MAX_PLATES`]), never the heap, so sampling
//! stays allocation-free (`E-HOT-NOALLOC`); the warp adds
//! [`PLATE_NOISE_EVALS_PER_SAMPLE`] noise evaluations per sample.

use super::classify::draw_body;
use super::noise3::{fbm_3d, smoothstep};
use super::relief::{OCEAN_FLOOR_M, gravity_for, relief_scale};
use crate::icosphere::normalize;
use crate::seed::hash_triple;
use std::f64::consts::TAU;

/// Most plates on one world (plates guide: 6 to 20).
pub const MAX_PLATES: usize = 20;

/// Fewest plates on one world.
pub const MIN_PLATES: usize = 6;

/// Slowest plate motion, in metres per year (plates guide: 1-10 cm/yr).
pub const MIN_PLATE_SPEED_M_PER_YR: f64 = 0.01;

/// Fastest plate motion, in metres per year.
pub const MAX_PLATE_SPEED_M_PER_YR: f64 = 0.10;

/// Motions below this relative speed read as transform (1 cm/yr).
pub const TRANSFORM_TOL_M_PER_YR: f64 = 0.01;

/// Slowest spreading used by the ridge age law, in metres per year.
pub const MIN_SPREAD_M_PER_YR: f64 = 0.005;

/// Boundary influence radius, in radians (about 1,270 km at Earth size):
/// past it the plates read as interiors. Profile widths below sit well
/// inside, and a fade band ([`PLATE_FADE_LO`], here) closes the seam.
pub const INFLUENCE_RAD: f64 = 0.20;

/// Influence fade starts here, in radians of boundary gap.
pub const PLATE_FADE_LO: f64 = 0.12;

/// Trench half-width, in radians (about 25 km at Earth size).
pub const TRENCH_W_RAD: f64 = 0.004;

/// Trench floor below the mean radius, in metres (plates guide: 6-8 km).
pub const TRENCH_TARGET_M: f64 = -7_000.0;

/// New ocean floor below the mean radius at the ridge, in metres.
pub const RIDGE_NEW_M: f64 = 2_500.0;

/// Seafloor deepening per root-million-years (the `√age` law, GEN-S030).
pub const RIDGE_AGE_SLOPE: f64 = 350.0;

/// Oldest seafloor depth below the mean radius, in metres (about 6,400 m).
pub const RIDGE_OLD_M: f64 = 6_400.0;

/// Ridge falloff width, in radians.
pub const RIDGE_FALLOFF_RAD: f64 = 0.15;

/// Rift trough half-width, in radians, and depth, in metres.
pub const RIFT_W_RAD: f64 = 0.008;
/// Rift trough depth below its shoulders, in metres (1-3 km row).
pub const RIFT_TROUGH_M: f64 = -2_000.0;

/// Rift shoulder offset and width, in radians, and height, in metres.
pub const RIFT_SHOULDER_OFF_RAD: f64 = 0.015;
/// Rift shoulder width.
pub const RIFT_SHOULDER_W_RAD: f64 = 0.008;
/// Rift shoulder height.
pub const RIFT_SHOULDER_M: f64 = 800.0;

/// Volcanic-range offset inland, width, and height (Andes row: 4-6 km,
/// 100-300 km inland).
pub const RANGE_OFF_RAD: f64 = 0.030;
/// Volcanic-range width.
pub const RANGE_W_RAD: f64 = 0.020;
/// Volcanic-range height.
pub const RANGE_M: f64 = 5_000.0;

/// Island-arc offset, width, and height.
pub const ARC_OFF_RAD: f64 = 0.020;
/// Island-arc width.
pub const ARC_W_RAD: f64 = 0.012;
/// Island-arc height.
pub const ARC_M: f64 = 2_500.0;

/// Collision-belt width and height (Himalaya row: 5-9 km).
pub const BELT_W_RAD: f64 = 0.030;
/// Collision-belt height.
pub const BELT_M: f64 = 7_000.0;

/// Collision-plateau width and height (Tibet row: 4-5 km).
pub const PLATEAU_W_RAD: f64 = 0.080;
/// Collision-plateau height.
pub const PLATEAU_M: f64 = 4_500.0;

/// Transform scarp half-width, in radians, and step, in metres.
pub const SCARP_W_RAD: f64 = 0.003;
/// Transform scarp step.
pub const SCARP_M: f64 = 150.0;

/// Mean Earth radius in metres (the reference for spreading ages).
pub const EARTH_RADIUS_M: f64 = crate::terrain::EARTH_RADIUS_KM * 1000.0;

/// Stream tag of the plate-seed draw.
const PLATE_TAG: u64 = 0x91A7_E685_0000_0000;
/// Stream tag of the plate-kind draw.
const KIND_TAG: u64 = 0x1410_E685_0000_0000;
/// Stream tag of the Euler-pole draw.
const AXIS_TAG: u64 = 0xA115_E685_0000_0000;
/// Stream tag of the motion draw.
const RATE_TAG: u64 = 0x9A7E_E685_0000_0000;
/// Stream tag of the regime roll.
const REGIME_TAG: u64 = 0x9E61_E685_0000_0000;
/// Stream tag of the provisional heat roll (T10 refines it).
const HEAT_TAG: u64 = 0x4EA7_E685_0000_0000;
/// Stream tag of the provisional activity roll (T11 refines it).
const ACTIVITY_TAG: u64 = 0xAC71_E685_0000_0000;
/// Stream tag of the provisional ice roll (T7 refines it).
const ICE_TAG: u64 = 0x1CE0_E685_0000_0000;
/// Stream tag of the relief warp (edges bend, never straight arcs).
const WARP_TAG: u64 = 0x9A91_E685_0000_0000;

/// Warp field octaves (the only noise the plates sample).
pub const PLATE_WARP_OCTAVES: u32 = 3;

/// Warp field base frequency.
pub const PLATE_WARP_FREQ: f64 = 3.0;

/// Warp strength in direction units.
pub const PLATE_WARP_AMP: f64 = 0.10;

/// Noise evaluations per plate sample (the warp field).
pub const PLATE_NOISE_EVALS_PER_SAMPLE: u32 = PLATE_WARP_OCTAVES;

/// Oceanic or continental plate (plates guide, step 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlateKind {
    /// Light crust, carries land.
    Continental,
    /// Dense crust, carries ocean.
    Oceanic,
}

/// Tectonic regime of one world (plates guide, the trigger).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Regime {
    /// Moving plates with boundaries below.
    Mobile,
    /// Old worn belts plus young volcanic plains (relief at 0.4).
    Episodic,
    /// One rigid shell: no belts (Mars-, Venus-, Moon-like).
    Stagnant,
    /// Resurfaced volcanic plains, no plates (Io-like).
    HeatPipe,
    /// Ridges and cracks in an ice shell (Europa-like).
    Ice,
}

/// What the regime trigger reads about one world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TectonicInput {
    /// Mass in Earth masses (drives the mobile probability).
    pub mass_earth: f64,
    /// Surface water share 0..1 (no water stiffens the lid).
    pub water_01: f64,
    /// Tidal and volcanic heating 0..1. Provisional seeded roll (moon
    /// resonances heat moons in T10; the plate heat roll stays).
    pub heat_01: f64,
    /// Interior activity 0..1, 1 young: the roll capped by the lifetime
    /// fade, so young worlds read the roll and old ones fade (T11).
    pub activity_01: f64,
    /// Ice-shell share 0..1. Provisional seeded roll until T7 wires climate.
    pub ice_01: f64,
}

/// One world's plates: `count` seeds, kinds and Euler motions.
#[derive(Debug, Clone, PartialEq)]
pub struct Plates {
    /// Seed the partition draws from.
    pub seed: u64,
    /// Plate count, `MIN_PLATES..=MAX_PLATES`.
    pub count: usize,
    /// The regime that owns them (Mobile or Episodic).
    pub regime: Regime,
    /// Planet radius in metres (Euler rates read it).
    pub radius_m: f64,
    /// Plate seed points (unit directions).
    pub seeds: [[f64; 3]; MAX_PLATES],
    /// Plate kinds.
    pub kinds: [PlateKind; MAX_PLATES],
    /// Euler poles (unit axes through the centre).
    pub axes: [[f64; 3]; MAX_PLATES],
    /// Euler rates in radians per year (signed by spin).
    pub rates: [f64; MAX_PLATES],
    /// Surface speeds in metres per year (1-10 cm/yr).
    pub speeds: [f64; MAX_PLATES],
}

/// One boundary class (plates guide, steps 4-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoundaryKind {
    /// Opening: ridge under ocean, rift under continent.
    Divergent,
    /// Ocean under continent: trench plus coastal range.
    ConvergentOceanContinent,
    /// Ocean under ocean: trench plus island arc.
    ConvergentOceanOcean,
    /// Continent against continent: wide belt plus plateau.
    ConvergentContinentContinent,
    /// Sideways: scarp and offset ridges.
    Transform,
}

/// The boundary nearest one direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundarySample {
    /// Boundary class.
    pub kind: BoundaryKind,
    /// Distance to the edge in radians (half the seed gap).
    pub edge_dist_rad: f64,
    /// Closing speed in metres per year (negative when opening).
    pub closing_m_per_yr: f64,
    /// Edge normal at the direction (from this plate to the other).
    pub normal: [f64; 3],
    /// This direction's plate kind.
    pub side: PlateKind,
    /// The other plate's kind.
    pub other: PlateKind,
    /// This direction's plate overrides (island arc side).
    pub arc_here: bool,
}

/// A share in 0..1 from the hash stream (deterministic per lane).
fn unit01(seed: u64, tag: u64, lane: u64) -> f64 {
    unit01_share(seed, tag, lane)
}

/// A share in 0..1 from the hash stream, shared with the landform
/// builders (deterministic per lane).
pub(crate) fn unit01_share(seed: u64, tag: u64, lane: u64) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: 53-bit hash share as a unit float, intended"
    )]
    let share = (hash_triple(seed, tag, lane) >> 11) as f64 / 9_007_199_254_740_992.0;
    share.clamp(0.0, 1.0 - f64::EPSILON)
}

/// A uniform unit direction from two hash lanes (shared with the relief
/// pass for seeded axes).
pub(crate) fn sphere_dir(seed: u64, tag: u64, rise_lane: u64, around_lane: u64) -> [f64; 3] {
    let rise = unit01(seed, tag, rise_lane) * 2.0 - 1.0;
    let around = unit01(seed, tag, around_lane) * TAU;
    let ring = (1.0 - rise * rise).max(0.0).sqrt();
    [ring * around.cos(), rise, ring * around.sin()]
}

/// Angular distance between unit directions, in radians.
fn angular_dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
        .clamp(-1.0, 1.0)
        .acos()
}

/// Gaussian bump of `height` at `offset` with `width` (all radians/metres
/// consistently): the boundary profiles.
fn bump(height: f64, offset: f64, width: f64, x: f64) -> f64 {
    let t = (x - offset) / width;
    height * (-t * t).exp()
}

/// Mobile-plate probability by mass (plates guide, the trigger): the size
/// research disagrees, so the disagreement is a seeded probability, not a
/// rule. Giants (over 10 Earth masses) hold no solid plates.
#[must_use]
pub fn mobile_probability(mass_earth: f64) -> f64 {
    if !mass_earth.is_finite() || mass_earth > 10.0 {
        0.0
    } else if mass_earth >= 2.0 {
        0.4
    } else if mass_earth >= 0.5 {
        0.6
    } else {
        0.1
    }
}

/// The tectonic regime of the world described by `input`, drawn from
/// `seed` (plates guide, the trigger).
#[must_use]
pub fn regime_for(seed: u64, input: &TectonicInput) -> Regime {
    if input.ice_01 > 0.6 && input.heat_01 > 0.3 {
        return Regime::Ice;
    }
    if input.heat_01 > 0.85 {
        return Regime::HeatPipe;
    }
    if input.water_01 < 0.05 || input.activity_01 < 0.15 || input.mass_earth > 10.0 {
        return Regime::Stagnant;
    }
    let probability = mobile_probability(input.mass_earth);
    if unit01(seed, REGIME_TAG, 0) < probability {
        Regime::Mobile
    } else if unit01(seed, REGIME_TAG, 1) < 0.25 {
        Regime::Episodic
    } else {
        Regime::Stagnant
    }
}

/// The trigger input drawn from `seed`: mass and water from the body
/// draw, heat, activity and ice as seeded rolls. The age-free draw the
/// unit tests pin; the relief path reads [`tectonic_input_for_age`].
#[must_use]
pub fn tectonic_input_for(seed: u64) -> TectonicInput {
    let body = draw_body(seed);
    TectonicInput {
        mass_earth: body.mass_earth,
        water_01: body.water,
        heat_01: unit01(seed, HEAT_TAG, 0),
        activity_01: unit01(seed, ACTIVITY_TAG, 0),
        ice_01: unit01(seed, ICE_TAG, 0),
    }
}

/// The trigger input at `system_age_gyr` on a world of `radius_earth`
/// Earth radii: the seeded roll capped by the lifetime fade
/// ([`super::lifetime::activity_for`]), so young worlds read the roll and
/// old small worlds fade toward a stagnant lid. At the mature reference
/// on Earth-size worlds the cap sits above every roll that matters to the
/// trigger, so the tuned relief holds.
#[must_use]
pub fn tectonic_input_for_age(seed: u64, system_age_gyr: f64, radius_earth: f64) -> TectonicInput {
    let mut input = tectonic_input_for(seed);
    let fade = super::lifetime::activity_for(radius_earth, system_age_gyr);
    input.activity_01 = input.activity_01.min(fade);
    input
}

/// The plates of a Mobile or Episodic world (`None` otherwise): `count`
/// seed points on the sphere (more on bigger planets), continental kinds
/// at about the land share, and one Euler pole plus 1-10 cm/yr motion
/// each. A degenerate radius reads Earth's.
#[must_use]
pub fn plate_field_for(seed: u64, radius_m: f64, input: &TectonicInput) -> Option<Plates> {
    let regime = regime_for(seed, input);
    if !matches!(regime, Regime::Mobile | Regime::Episodic) {
        return None;
    }
    let radius = if radius_m.is_finite() && radius_m > 0.0 {
        radius_m
    } else {
        EARTH_RADIUS_M
    };
    let base = if input.mass_earth >= 2.0 { 10u64 } else { 6u64 };
    let span = (MAX_PLATES + 1) as u64 - base;
    let count_u64 = base + hash_triple(seed, PLATE_TAG, 0) % span;
    let count = usize::try_from(count_u64)
        .unwrap_or(MAX_PLATES)
        .min(MAX_PLATES);
    let mut field = Plates {
        seed,
        count,
        regime,
        radius_m: radius,
        seeds: [[0.0; 3]; MAX_PLATES],
        kinds: [PlateKind::Oceanic; MAX_PLATES],
        axes: [[0.0; 3]; MAX_PLATES],
        rates: [0.0; MAX_PLATES],
        speeds: [0.0; MAX_PLATES],
    };
    for (i, slot) in field.seeds.iter_mut().enumerate().take(count) {
        let lane = i as u64;
        *slot = sphere_dir(seed, PLATE_TAG, 2 * lane + 1, 2 * lane + 2);
    }
    for (i, slot) in field.kinds.iter_mut().enumerate().take(count) {
        let lane = i as u64;
        *slot = if unit01(seed, KIND_TAG, lane) < 0.3 {
            PlateKind::Continental
        } else {
            PlateKind::Oceanic
        };
    }
    for (i, slot) in field.axes.iter_mut().enumerate().take(count) {
        let lane = i as u64;
        *slot = sphere_dir(seed, AXIS_TAG, 2 * lane, 2 * lane + 1);
    }
    for (i, slot) in field.speeds.iter_mut().enumerate().take(count) {
        let lane = i as u64;
        *slot = MIN_PLATE_SPEED_M_PER_YR
            + unit01(seed, RATE_TAG, 2 * lane)
                * (MAX_PLATE_SPEED_M_PER_YR - MIN_PLATE_SPEED_M_PER_YR);
    }
    for (i, slot) in field.rates.iter_mut().enumerate().take(count) {
        let lane = i as u64;
        let sign = if hash_triple(seed, RATE_TAG, 2 * lane + 1).is_multiple_of(2) {
            1.0
        } else {
            -1.0
        };
        let speed = field.speeds.get(i).copied().unwrap_or(0.0);
        *slot = sign * speed / radius;
    }
    Some(field)
}

/// Index of the plate owning `direction` (`None` for a non-finite
/// direction or an empty field). Ties keep the lowest index, so the
/// partition is deterministic and every direction belongs to exactly one
/// plate.
#[must_use]
pub fn plate_index_at(field: &Plates, direction: [f64; 3]) -> Option<usize> {
    if !direction.iter().all(|c| c.is_finite()) {
        return None;
    }
    let count = field.count.min(MAX_PLATES);
    if count == 0 {
        return None;
    }
    let unit = normalize(direction);
    let mut best = 0usize;
    let mut nearest = f64::INFINITY;
    for (i, seed) in field.seeds.iter().enumerate().take(count) {
        let gap = angular_dist(unit, *seed);
        if gap < nearest {
            nearest = gap;
            best = i;
        }
    }
    Some(best)
}

/// The boundary nearest `direction` (`None` past the influence radius,
/// for a non-finite direction, or on a field of under two plates).
/// Directions are warped first, so plate edges bend instead of running as
/// straight arcs (plates guide, step 1).
#[must_use]
pub fn boundary_at(field: &Plates, direction: [f64; 3]) -> Option<BoundarySample> {
    if !direction.iter().all(|c| c.is_finite()) {
        return None;
    }
    let count = field.count.min(MAX_PLATES);
    if count < 2 {
        return None;
    }
    let lane = hash_triple(field.seed, WARP_TAG, 0);
    let unit = normalize(direction);
    let warped = normalize([
        unit[0] + PLATE_WARP_AMP * fbm_warp(lane, unit, 0),
        unit[1] + PLATE_WARP_AMP * fbm_warp(lane, unit, 1),
        unit[2] + PLATE_WARP_AMP * fbm_warp(lane, unit, 2),
    ]);
    let mut first = 0usize;
    let mut second = 1usize;
    let mut near = f64::INFINITY;
    let mut next = f64::INFINITY;
    for (i, seed) in field.seeds.iter().enumerate().take(count) {
        let gap = angular_dist(warped, *seed);
        if gap < near {
            next = near;
            second = first;
            near = gap;
            first = i;
        } else if gap < next {
            next = gap;
            second = i;
        }
    }
    let gap = next - near;
    if gap >= INFLUENCE_RAD {
        return None;
    }
    let (Some(seed_a), Some(seed_b)) = (
        field.seeds.get(first).copied(),
        field.seeds.get(second).copied(),
    ) else {
        return None;
    };
    let chord = [
        seed_b[0] - seed_a[0],
        seed_b[1] - seed_a[1],
        seed_b[2] - seed_a[2],
    ];
    let along = chord[0] * warped[0] + chord[1] * warped[1] + chord[2] * warped[2];
    let side = [
        chord[0] - along * warped[0],
        chord[1] - along * warped[1],
        chord[2] - along * warped[2],
    ];
    let reach = (side[0] * side[0] + side[1] * side[1] + side[2] * side[2]).sqrt();
    if !reach.is_finite() || reach < 1e-12 {
        return None;
    }
    let normal = [side[0] / reach, side[1] / reach, side[2] / reach];
    let kind_a = field
        .kinds
        .get(first)
        .copied()
        .unwrap_or(PlateKind::Oceanic);
    let kind_b = field
        .kinds
        .get(second)
        .copied()
        .unwrap_or(PlateKind::Oceanic);
    let axis_a = field.axes.get(first).copied().unwrap_or([0.0, 1.0, 0.0]);
    let axis_b = field.axes.get(second).copied().unwrap_or([0.0, 1.0, 0.0]);
    let rate_a = field.rates.get(first).copied().unwrap_or(0.0);
    let rate_b = field.rates.get(second).copied().unwrap_or(0.0);
    // Surface velocities (radians per year): v = rate * (axis x d).
    let spin_a = [
        axis_a[1] * warped[2] - axis_a[2] * warped[1],
        axis_a[2] * warped[0] - axis_a[0] * warped[2],
        axis_a[0] * warped[1] - axis_a[1] * warped[0],
    ];
    let spin_b = [
        axis_b[1] * warped[2] - axis_b[2] * warped[1],
        axis_b[2] * warped[0] - axis_b[0] * warped[2],
        axis_b[0] * warped[1] - axis_b[1] * warped[0],
    ];
    let closing_rad = (rate_a * spin_a[0] - rate_b * spin_b[0]) * normal[0]
        + (rate_a * spin_a[1] - rate_b * spin_b[1]) * normal[1]
        + (rate_a * spin_a[2] - rate_b * spin_b[2]) * normal[2];
    let tolerance = TRANSFORM_TOL_M_PER_YR / field.radius_m;
    let kind = match (kind_a, kind_b) {
        (PlateKind::Oceanic, PlateKind::Oceanic) if closing_rad > tolerance => {
            BoundaryKind::ConvergentOceanOcean
        }
        (PlateKind::Continental, PlateKind::Continental) if closing_rad > tolerance => {
            BoundaryKind::ConvergentContinentContinent
        }
        _ if closing_rad > tolerance => BoundaryKind::ConvergentOceanContinent,
        _ if closing_rad < -tolerance => BoundaryKind::Divergent,
        _ => BoundaryKind::Transform,
    };
    Some(BoundarySample {
        kind,
        edge_dist_rad: gap / 2.0,
        closing_m_per_yr: closing_rad * field.radius_m,
        normal,
        side: kind_a,
        other: kind_b,
        arc_here: first < second,
    })
}

/// One warp lane over the direction at the plate frequency.
fn fbm_warp(seed: u64, unit: [f64; 3], lane: u64) -> f64 {
    let lane_seed = hash_triple(seed, WARP_TAG, lane + 1);
    let (value, _) = fbm_3d(
        lane_seed,
        [
            unit[0] * PLATE_WARP_FREQ,
            unit[1] * PLATE_WARP_FREQ,
            unit[2] * PLATE_WARP_FREQ,
        ],
        PLATE_WARP_OCTAVES,
        2.0,
        0.5,
    );
    value
}

/// Ocean-floor depth below the mean radius at `dist_m` metres past the
/// ridge, spreading at `spreading_m_per_yr`: `2500 + 350 √t` metres for
/// young floor (`t` in millions of years), flattening toward about
/// 6,400 m (plates guide, GEN-S030).
#[must_use]
pub fn ridge_depth_m(dist_m: f64, spreading_m_per_yr: f64) -> f64 {
    if !dist_m.is_finite() || !spreading_m_per_yr.is_finite() || spreading_m_per_yr <= 0.0 {
        return RIDGE_NEW_M;
    }
    let age_myr = dist_m.max(0.0) / (spreading_m_per_yr * 1.0e6);
    (RIDGE_NEW_M + RIDGE_AGE_SLOPE * age_myr.sqrt()).min(RIDGE_OLD_M)
}

/// Plate-boundary relief in metres above (or below) the base relief.
///
/// Ridges, rifts, trenches, ranges, arcs, belts and scarps from the
/// boundary table (plates guide, step 5), scaled by gravity like every
/// relief amplitude. `0.0` past the influence radius, off a boundary, on
/// a non-mobile regime, or for a non-finite direction.
#[must_use]
pub fn plate_delta_m(seed: u64, direction: [f64; 3], radius_m: f64, system_age_gyr: f64) -> f64 {
    field_for_relief(seed, radius_m, system_age_gyr)
        .map_or(0.0, |field| delta_for_field(&field, seed, direction))
}

/// The field of a Mobile or Episodic world for one relief sample: the
/// trigger input plus the partition, built once per sample and shared by
/// the plate and crater passes (`total_relief_m`).
pub(crate) fn field_for_relief(seed: u64, radius_m: f64, system_age_gyr: f64) -> Option<Plates> {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return None;
    }
    let radius_earth = radius_m / EARTH_RADIUS_M;
    plate_field_for(
        seed,
        radius_m,
        &tectonic_input_for_age(seed, system_age_gyr, radius_earth),
    )
}

/// Boundary relief of the built `field` at `direction` (the profile half
/// of [`plate_delta_m`).
pub(crate) fn delta_for_field(field: &Plates, seed: u64, direction: [f64; 3]) -> f64 {
    if !direction.iter().all(|c| c.is_finite()) {
        return 0.0;
    }
    let worn = match field.regime {
        Regime::Mobile => 1.0,
        Regime::Episodic => 0.4,
        Regime::Stagnant | Regime::HeatPipe | Regime::Ice => return 0.0,
    };
    let Some(edge) = boundary_at(field, direction) else {
        return 0.0;
    };
    let fade = 1.0 - smoothstep(PLATE_FADE_LO, INFLUENCE_RAD, edge.edge_dist_rad * 2.0);
    if fade <= 0.0 {
        return 0.0;
    };
    let x = edge.edge_dist_rad;
    let radius_m = field.radius_m;
    let profile = match edge.kind {
        BoundaryKind::Divergent if edge.side == PlateKind::Oceanic => {
            let opening = (-edge.closing_m_per_yr).max(MIN_SPREAD_M_PER_YR);
            let target = -ridge_depth_m(x * radius_m, opening);
            (target - OCEAN_FLOOR_M) * (-(x / RIDGE_FALLOFF_RAD) * (x / RIDGE_FALLOFF_RAD)).exp()
        }
        BoundaryKind::Divergent => {
            bump(RIFT_TROUGH_M, 0.0, RIFT_W_RAD, x)
                + bump(
                    RIFT_SHOULDER_M,
                    RIFT_SHOULDER_OFF_RAD,
                    RIFT_SHOULDER_W_RAD,
                    x,
                )
        }
        BoundaryKind::ConvergentOceanContinent if edge.side == PlateKind::Oceanic => {
            (TRENCH_TARGET_M - OCEAN_FLOOR_M) * (-(x / TRENCH_W_RAD) * (x / TRENCH_W_RAD)).exp()
        }
        BoundaryKind::ConvergentOceanContinent => bump(RANGE_M, RANGE_OFF_RAD, RANGE_W_RAD, x),
        BoundaryKind::ConvergentOceanOcean => {
            let trench = (TRENCH_TARGET_M - OCEAN_FLOOR_M)
                * (-(x / TRENCH_W_RAD) * (x / TRENCH_W_RAD)).exp();
            let arc = if edge.arc_here {
                bump(ARC_M, ARC_OFF_RAD, ARC_W_RAD, x)
            } else {
                0.0
            };
            trench + arc
        }
        BoundaryKind::ConvergentContinentContinent => {
            bump(BELT_M, 0.0, BELT_W_RAD, x) + bump(PLATEAU_M, 0.0, PLATEAU_W_RAD, x)
        }
        BoundaryKind::Transform => {
            SCARP_M * (smoothstep(-SCARP_W_RAD, SCARP_W_RAD, -x) - 0.5) * 2.0
        }
    };
    profile * relief_scale(gravity_for(seed)) * worn * fade
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icosphere::normalize;

    /// Spiral directions covering the sphere without a pole pinch.
    fn sweep(count: u32) -> Vec<[f64; 3]> {
        let mut out = Vec::new();
        for i in 0..count {
            let step = f64::from(i);
            let total = f64::from(count);
            let y = 1.0 - 2.0 * (step + 0.5) / total;
            let r = (1.0 - y * y).max(0.0).sqrt();
            let angle = step * 2.399_963_229_728_653;
            out.push([r * angle.cos(), y, r * angle.sin()]);
        }
        out
    }

    /// First seed in `range` whose regime is `want`.
    fn first_seed_with(range: u64, want: Regime) -> u64 {
        for seed in 0..range {
            if regime_for(seed, &tectonic_input_for(seed)) == want {
                return seed;
            }
        }
        panic!("no {want:?} seed in 0..{range}");
    }

    #[test]
    fn regime_trigger_covers_mobile_and_lid() {
        let mut seen = std::collections::BTreeSet::new();
        for seed in 0..400u64 {
            seen.insert(regime_for(seed, &tectonic_input_for(seed)) as u8);
        }
        assert!(seen.contains(&(Regime::Mobile as u8)), "no mobile worlds");
        assert!(seen.contains(&(Regime::Stagnant as u8)), "no stagnant lids");
        assert!(
            seen.contains(&(Regime::Episodic as u8)),
            "no episodic worlds"
        );
        // Explicit inputs reach every regime, deterministically.
        let wet = TectonicInput {
            mass_earth: 1.0,
            water_01: 0.5,
            heat_01: 0.2,
            activity_01: 0.8,
            ice_01: 0.0,
        };
        assert_eq!(regime_for(11, &wet), regime_for(11, &wet));
        assert_eq!(
            regime_for(
                5,
                &TectonicInput {
                    heat_01: 0.5,
                    ice_01: 0.8,
                    ..wet
                }
            ),
            Regime::Ice
        );
        assert_eq!(
            regime_for(
                5,
                &TectonicInput {
                    heat_01: 0.9,
                    ..wet
                }
            ),
            Regime::HeatPipe
        );
        assert_eq!(
            regime_for(
                5,
                &TectonicInput {
                    water_01: 0.0,
                    ..wet
                }
            ),
            Regime::Stagnant
        );
        assert_eq!(
            regime_for(
                5,
                &TectonicInput {
                    activity_01: 0.0,
                    ..wet
                }
            ),
            Regime::Stagnant
        );
        assert_eq!(
            regime_for(
                5,
                &TectonicInput {
                    mass_earth: 300.0,
                    ..wet
                }
            ),
            Regime::Stagnant
        );
        assert_eq!(mobile_probability(1.0), 0.6);
        assert_eq!(mobile_probability(5.0), 0.4);
        assert_eq!(mobile_probability(0.2), 0.1);
        assert_eq!(mobile_probability(300.0), 0.0);
    }

    #[test]
    fn plates_cover_the_sphere_at_plate_speeds() {
        let seed = first_seed_with(200, Regime::Mobile);
        let input = tectonic_input_for(seed);
        let field = plate_field_for(seed, EARTH_RADIUS_M, &input).expect("mobile plates");
        assert!(
            (MIN_PLATES..=MAX_PLATES).contains(&field.count),
            "count {}",
            field.count
        );
        assert_eq!(field.radius_m, EARTH_RADIUS_M);
        assert_eq!(
            plate_field_for(seed, EARTH_RADIUS_M, &input),
            plate_field_for(seed, EARTH_RADIUS_M, &input)
        );
        for speed in field.speeds.iter().take(field.count) {
            assert!(
                (MIN_PLATE_SPEED_M_PER_YR..=MAX_PLATE_SPEED_M_PER_YR).contains(speed),
                "speed {speed} m/yr outside 1-10 cm/yr"
            );
        }
        // Every direction belongs to exactly one plate, and every plate
        // owns ground.
        let mut owners = std::collections::BTreeSet::new();
        for dir in sweep(2_000) {
            let index = plate_index_at(&field, dir).expect("every direction owned");
            assert!(index < field.count, "plate {index} past the count");
            assert_eq!(plate_index_at(&field, dir), Some(index), "ties replay");
            owners.insert(index);
        }
        assert_eq!(owners.len(), field.count, "a plate owns no ground");
        assert_eq!(plate_index_at(&field, [f64::NAN, 0.0, 0.0]), None);
    }

    /// A boundary of the wanted shape, with its edge point and sample.
    struct Found {
        seed: u64,
        point: [f64; 3],
        sample: BoundarySample,
    }

    /// First boundary of one kind on the wanted plate pair whose whole
    /// `±0.06` corridor stays on the same pair, with its edge point and
    /// sample. Walks crossing into another pair are skipped, so profile
    /// walks read one boundary only.
    fn find_boundary(
        want: BoundaryKind,
        side: Option<PlateKind>,
        other: Option<PlateKind>,
    ) -> Found {
        for seed in 0..400u64 {
            let input = tectonic_input_for(seed);
            if !matches!(regime_for(seed, &input), Regime::Mobile | Regime::Episodic) {
                continue;
            }
            let Some(field) = plate_field_for(seed, EARTH_RADIUS_M, &input) else {
                continue;
            };
            for dir in sweep(96) {
                let Some(sample) = boundary_at(&field, dir) else {
                    continue;
                };
                if sample.edge_dist_rad >= 0.01
                    || sample.kind != want
                    || side.is_some_and(|kind| sample.side != kind)
                    || other.is_some_and(|kind| sample.other != kind)
                    || !walk_stays(&field, dir, sample.normal, want)
                {
                    continue;
                }
                return Found {
                    seed,
                    point: dir,
                    sample,
                };
            }
        }
        panic!("no {want:?} boundary in range");
    }

    /// Whether the `±0.06` walk around `at` along `normal` stays on one
    /// boundary of `want`: the kind holds and each normal runs parallel to
    /// the edge (either way: crossing the edge flips the normal).
    fn walk_stays(field: &Plates, at: [f64; 3], normal: [f64; 3], want: BoundaryKind) -> bool {
        for i in 0..=24u32 {
            let alpha = -0.06 + 0.12 * f64::from(i) / 24.0;
            let bent = normalize([
                at[0] + normal[0] * alpha,
                at[1] + normal[1] * alpha,
                at[2] + normal[2] * alpha,
            ]);
            let Some(next) = boundary_at(field, bent) else {
                return false;
            };
            let aligned = next.normal[0] * normal[0]
                + next.normal[1] * normal[1]
                + next.normal[2] * normal[2];
            if next.kind != want || aligned.abs() < 0.9 {
                return false;
            }
        }
        true
    }

    /// Deltas along the edge normal around `at`, `±reach` in steps.
    fn walk(seed: u64, at: [f64; 3], normal: [f64; 3], reach: f64, steps: u32) -> Vec<f64> {
        let mut out = Vec::new();
        for i in 0..=2 * steps {
            let alpha = -reach + 2.0 * reach * f64::from(i) / f64::from(2 * steps);
            let bent = normalize([
                at[0] + normal[0] * alpha,
                at[1] + normal[1] * alpha,
                at[2] + normal[2] * alpha,
            ]);
            out.push(plate_delta_m(
                seed,
                bent,
                EARTH_RADIUS_M,
                super::super::lifetime::MATURE_AGE_GYR,
            ));
        }
        out
    }

    #[test]
    fn ridge_depth_follows_the_root_age_law() {
        // GEN-S030: 2500 m at the ridge, +350 per root-Myr, about 6,400 max.
        assert_eq!(ridge_depth_m(0.0, 0.03), RIDGE_NEW_M);
        assert_eq!(ridge_depth_m(-100.0, 0.03), RIDGE_NEW_M);
        assert_eq!(ridge_depth_m(100.0, 0.0), RIDGE_NEW_M);
        let at_100myr = ridge_depth_m(100.0e6 * 0.03, 0.03);
        assert!(
            (at_100myr - 6_000.0).abs() < 1.0,
            "100 Myr reads {at_100myr} m"
        );
        assert_eq!(ridge_depth_m(1.0e9, 0.03), RIDGE_OLD_M);
        let mut prev = 0.0;
        for metres in [0.0, 1.0e5, 5.0e5, 1.0e6, 3.0e6] {
            let now = ridge_depth_m(metres, 0.03);
            assert!(now >= prev, "floor shallows backwards at {metres} m");
            prev = now;
        }
    }

    #[test]
    fn plate_relief_follows_boundaries() {
        // C7 / AC7: ranges and trenches only where plates collide, rifts
        // and ridges only where they separate; stagnant lids wear no belts.
        let divergent = find_boundary(
            BoundaryKind::Divergent,
            Some(PlateKind::Oceanic),
            Some(PlateKind::Oceanic),
        );
        let scale = relief_scale(gravity_for(divergent.seed));
        let ridge = walk(
            divergent.seed,
            divergent.point,
            divergent.sample.normal,
            0.06,
            12,
        );
        // The ridge peaks at the edge, which sits a few steps off the walk
        // centre: peak from the walk itself, then fall away on both sides.
        let peak_at = ridge
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map_or(0, |(i, _)| i);
        let peak = ridge[peak_at] / scale;
        assert!(peak > 1_000.0, "no ridge at the edge: {peak}");
        let mut prev = peak;
        for delta in ridge.iter().skip(peak_at + 1).map(|d| d / scale) {
            assert!(
                delta <= prev + 5.0,
                "floor shallows backwards past the ridge"
            );
            prev = prev.min(delta);
        }
        prev = peak;
        for delta in ridge[..peak_at].iter().rev().map(|d| d / scale) {
            assert!(
                delta <= prev + 5.0,
                "floor shallows backwards past the ridge"
            );
            prev = prev.min(delta);
        }
        for end in [ridge[0] / scale, ridge[24] / scale] {
            assert!(
                end < peak * 0.9 - 10.0,
                "floor never deepens: {end} vs {peak}"
            );
        }

        // Convergent flavours carry their own trench/range signature.
        for kinds in [
            BoundaryKind::ConvergentOceanContinent,
            BoundaryKind::ConvergentOceanOcean,
            BoundaryKind::ConvergentContinentContinent,
        ] {
            let found = find_boundary(kinds, None, None);
            let scale = relief_scale(gravity_for(found.seed));
            // Fine walk for the narrow trench, wide walk for the range.
            let near: Vec<f64> = walk(found.seed, found.point, found.sample.normal, 0.02, 10)
                .iter()
                .map(|d| d / scale)
                .collect();
            let far: Vec<f64> = walk(found.seed, found.point, found.sample.normal, 0.06, 12)
                .iter()
                .map(|d| d / scale)
                .collect();
            let deep = near.iter().cloned().fold(f64::INFINITY, f64::min);
            let high = far.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            match kinds {
                BoundaryKind::ConvergentOceanContinent => {
                    assert!(deep < -2_000.0, "no trench: {deep}");
                    assert!(high > 3_500.0, "no range: {high}");
                }
                BoundaryKind::ConvergentOceanOcean => {
                    assert!(deep < -2_000.0, "no trench: {deep}");
                    assert!(high > 1_500.0, "no arc: {high}");
                }
                _ => assert!(high > 4_000.0, "no collision belt: {high}"),
            }
        }

        // Transform scarps stay scarps: no trench, no range.
        let shifted = find_boundary(BoundaryKind::Transform, None, None);
        let scale = relief_scale(gravity_for(shifted.seed));
        for delta in walk(shifted.seed, shifted.point, shifted.sample.normal, 0.02, 10) {
            assert!(
                (delta / scale).abs() <= 200.0,
                "transform scarp runs past its step: {delta}"
            );
        }

        // Away from convergence nothing runs extreme.
        for seed in 0..20u64 {
            let input = tectonic_input_for(seed);
            if !matches!(regime_for(seed, &input), Regime::Mobile | Regime::Episodic) {
                continue;
            }
            let Some(field) = plate_field_for(seed, EARTH_RADIUS_M, &input) else {
                continue;
            };
            let scale = relief_scale(gravity_for(seed));
            for dir in sweep(64) {
                let Some(edge) = boundary_at(&field, dir) else {
                    continue;
                };
                if matches!(
                    edge.kind,
                    BoundaryKind::ConvergentOceanContinent
                        | BoundaryKind::ConvergentOceanOcean
                        | BoundaryKind::ConvergentContinentContinent
                ) {
                    continue;
                }
                let delta = plate_delta_m(
                    seed,
                    dir,
                    EARTH_RADIUS_M,
                    super::super::lifetime::MATURE_AGE_GYR,
                ) / scale;
                assert!(
                    delta.abs() < 3_000.0,
                    "non-convergent extreme {delta} at seed {seed}"
                );
            }
        }

        // Stagnant lids wear no belts at all.
        let lid = first_seed_with(200, Regime::Stagnant);
        for dir in sweep(128) {
            assert_eq!(
                plate_delta_m(
                    lid,
                    dir,
                    EARTH_RADIUS_M,
                    super::super::lifetime::MATURE_AGE_GYR
                ),
                0.0,
                "lid belt"
            );
        }
    }

    #[test]
    fn plate_cost_stays_within_the_operation_budget() {
        // E-TEST-HERMETIC: the warp is the only noise the plates sample.
        assert_eq!(PLATE_NOISE_EVALS_PER_SAMPLE, PLATE_WARP_OCTAVES);
        const _: () = assert!(
            super::super::relief::NOISE_EVALS_PER_SAMPLE + PLATE_NOISE_EVALS_PER_SAMPLE <= 40,
            "ground schedule grew past the budget"
        );
    }
}
