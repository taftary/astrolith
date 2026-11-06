//! Places on a world's water, for shot lists and routes (#1162 T1).
//!
//! `universe-app --list-water <chain>` prints these so the pictures and the
//! timing routes can name a coast, shallow water, open sea or polar sea by
//! `lat`/`lon` (the shot file's planet-frame convention: `+Y` is the pole,
//! longitude grows east, `(cos lat cos lon, sin lat, -cos lat sin lon)`).
//! The sea surface is at 0 m, where the relief's continent mask crosses
//! from land to ocean floor. Pure and deterministic: the same relief
//! always lists the same places (`E-DET-TIERS`); transcendental calls stay
//! in core (`E-TRANSCENDENTAL`).

use crate::relief::{HeightSampler, Relief};

/// What a listed place is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceKind {
    /// The water's edge: land on one side, sea on the other.
    Coast,
    /// Sea a few tens of metres deep, near land.
    Shallow,
    /// Open sea more than 3 km deep.
    Deep,
    /// Sea near a pole (latitude 72 degrees or more).
    Polar,
    /// The centre of a lake filled to its rim (#1162 T3).
    Lake,
    /// The centre of a partly filled lake, with its dry ring (#1162 T3).
    PartlyLake,
}

impl PlaceKind {
    /// The listing word for this kind.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            PlaceKind::Coast => "coast",
            PlaceKind::Shallow => "shallow",
            PlaceKind::Deep => "deep",
            PlaceKind::Polar => "polar",
            PlaceKind::Lake => "lake",
            PlaceKind::PartlyLake => "lake-partly",
        }
    }
}

/// One place on the water.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterPlace {
    /// What the place is.
    pub kind: PlaceKind,
    /// Latitude in degrees.
    pub lat_deg: f64,
    /// Longitude in degrees, east positive.
    pub lon_deg: f64,
    /// Terrain height there in metres (negative under the sea).
    pub height_m: f64,
    /// For a coast: the compass bearing toward the sea, degrees from north
    /// toward east (the shot file's `heading`).
    pub sea_bearing_deg: Option<f64>,
}

/// Fewest metres of sea depth for an open-sea place.
const DEEP_M: f64 = 3_000.0;
/// Shallow water lies between these depths in metres.
const SHALLOW_M: (f64, f64) = (10.0, 60.0);
/// Coast search: land points this low (metres) are checked for sea nearby.
const COAST_LAND_M: f64 = 200.0;
/// Coast search reach in metres.
const COAST_REACH_M: f64 = 2_000.0;
/// Bearings checked around a coast candidate, degrees.
const BEARINGS: [f64; 8] = [0.0, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0];
/// Polar latitude in degrees.
const POLAR_DEG: f64 = 72.0;
/// Coasts, shallows and open sea are listed below this latitude, where the
/// sun stands high enough for pictures.
const MID_DEG: f64 = 55.0;
/// Lattice stride for a scattered visiting order (a prime, so it is coprime
/// to sample counts that are not its multiples).
const STRIDE: usize = 104_729;

/// The planet-frame unit direction at `lat_deg`, `lon_deg`.
#[must_use]
pub fn lat_lon_dir(lat_deg: f64, lon_deg: f64) -> [f64; 3] {
    let (sl, cl) = lat_deg.to_radians().sin_cos();
    let (so, co) = lon_deg.to_radians().sin_cos();
    [cl * co, sl, -cl * so]
}

/// Latitude and longitude in degrees of unit `dir` (inverse of
/// [`lat_lon_dir`]).
#[must_use]
pub fn dir_lat_lon(dir: [f64; 3]) -> (f64, f64) {
    let lat = dir[1].clamp(-1.0, 1.0).asin().to_degrees();
    let lon = (-dir[2]).atan2(dir[0]).to_degrees();
    (lat, lon)
}

/// The direction `dist_m` from `(lat, lon)` along compass `bearing_deg` on a
/// ball of `radius_m` (tangent step, then back onto the ball).
fn step(lat_deg: f64, lon_deg: f64, bearing_deg: f64, dist_m: f64, radius_m: f64) -> [f64; 3] {
    let d = lat_lon_dir(lat_deg, lon_deg);
    let (sl, cl) = lat_deg.to_radians().sin_cos();
    let (so, co) = lon_deg.to_radians().sin_cos();
    let north = [-sl * co, cl, sl * so];
    let east = [-so, 0.0, -co];
    let (sb, cb) = bearing_deg.to_radians().sin_cos();
    let t = dist_m / radius_m;
    let p = [
        d[0] + t * (cb * north[0] + sb * east[0]),
        d[1] + t * (cb * north[1] + sb * east[1]),
        d[2] + t * (cb * north[2] + sb * east[2]),
    ];
    let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
    [p[0] / len, p[1] / len, p[2] / len]
}

/// `n` directions spread evenly over the sphere (Fibonacci lattice).
#[must_use]
pub fn fibonacci_dir(i: usize, n: usize) -> [f64; 3] {
    let golden = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: sample counts far below 2^52"
    )]
    let (fi, fnn) = (i as f64, n as f64);
    let y = 1.0 - 2.0 * (fi + 0.5) / fnn;
    let r = (1.0 - y * y).max(0.0).sqrt();
    [r * (golden * fi).cos(), y, r * (golden * fi).sin()]
}

/// The coast crossing near land point `(lat, lon)`, if sea lies within
/// [`COAST_REACH_M`] along some bearing: bisected to the 0 m line.
fn coast_near(
    sampler: &mut HeightSampler<'_>,
    radius_m: f64,
    lat: f64,
    lon: f64,
) -> Option<WaterPlace> {
    for bearing in BEARINGS {
        let far = step(lat, lon, bearing, COAST_REACH_M, radius_m);
        if sampler.terrain_m(far) >= 0.0 {
            continue;
        }
        let (mut lo, mut hi) = (0.0, COAST_REACH_M);
        for _ in 0..24 {
            let mid = 0.5 * (lo + hi);
            if sampler.terrain_m(step(lat, lon, bearing, mid, radius_m)) >= 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let dir = step(lat, lon, bearing, lo, radius_m);
        let (lat_deg, lon_deg) = dir_lat_lon(dir);
        return Some(WaterPlace {
            kind: PlaceKind::Coast,
            lat_deg,
            lon_deg,
            height_m: sampler.terrain_m(dir),
            sea_bearing_deg: Some(bearing),
        });
    }
    None
}

/// Up to `per_kind` places of each kind on `relief`, from `samples` evenly
/// spread directions visited in a scattered order (the lattice index times
/// `STRIDE`), so places spread over the world. Giants list nothing.
#[must_use]
pub fn water_places(relief: &Relief, samples: usize, per_kind: usize) -> Vec<WaterPlace> {
    let mut out = Vec::new();
    if relief.is_giant() {
        return out;
    }
    let radius_m = relief.radius_m();
    let mut sampler = HeightSampler::new(relief);
    let mut counts = [0usize; 6];
    for i in 0..samples {
        if counts.iter().take(4).all(|&c| c >= per_kind) {
            break;
        }
        let dir = fibonacci_dir(i.wrapping_mul(STRIDE) % samples, samples);
        let h = sampler.terrain_m(dir);
        let (lat, lon) = dir_lat_lon(dir);
        let kind = if h < 0.0 && lat.abs() >= POLAR_DEG {
            Some(PlaceKind::Polar)
        } else if lat.abs() >= MID_DEG {
            None
        } else if h <= -DEEP_M {
            Some(PlaceKind::Deep)
        } else if -h >= SHALLOW_M.0 && -h <= SHALLOW_M.1 {
            Some(PlaceKind::Shallow)
        } else {
            None
        };
        if let Some(kind) = kind {
            let slot = kind as usize;
            if counts.get(slot).is_some_and(|&c| c < per_kind) {
                out.push(WaterPlace {
                    kind,
                    lat_deg: lat,
                    lon_deg: lon,
                    height_m: h,
                    sea_bearing_deg: None,
                });
                if let Some(c) = counts.get_mut(slot) {
                    *c += 1;
                }
            }
            continue;
        }
        let coast = PlaceKind::Coast as usize;
        if (0.0..COAST_LAND_M).contains(&h)
            && lat.abs() < MID_DEG
            && counts.get(coast).is_some_and(|&c| c < per_kind)
            && let Some(place) = coast_near(&mut sampler, radius_m, lat, lon)
        {
            out.push(place);
            if let Some(c) = counts.get_mut(coast) {
                *c += 1;
            }
        }
    }
    out.extend(lake_places(relief, samples, per_kind));
    out
}

/// Up to `per_kind` full and partly filled lakes on `relief` (#1162 T3),
/// each at its centre with its water level as the height: those below
/// `MID_DEG` first (better lit for pictures), in lattice order.
#[must_use]
pub fn lake_places(relief: &Relief, _samples: usize, per_kind: usize) -> Vec<WaterPlace> {
    let mut lakes: Vec<(bool, WaterPlace)> = relief
        .lakes()
        .into_iter()
        .map(|lake| {
            let (lat_deg, lon_deg) = dir_lat_lon(lake.seed.centre);
            let kind = if lake.seed.drop > 0.0 {
                PlaceKind::PartlyLake
            } else {
                PlaceKind::Lake
            };
            let place = WaterPlace {
                kind,
                lat_deg,
                lon_deg,
                height_m: lake.level_m,
                sea_bearing_deg: None,
            };
            (lat_deg.abs() >= MID_DEG, place)
        })
        .collect();
    lakes.sort_by_key(|(far, _)| *far);
    let mut out = Vec::new();
    for kind in [PlaceKind::Lake, PlaceKind::PartlyLake] {
        out.extend(
            lakes
                .iter()
                .filter(|(_, p)| p.kind == kind)
                .take(per_kind)
                .map(|(_, p)| *p),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lat_lon_round_trips() {
        for (lat, lon) in [(0.0, 0.0), (-30.0, 60.0), (45.0, -120.0), (80.0, 170.0)] {
            let (a, b) = dir_lat_lon(lat_lon_dir(lat, lon));
            assert!(
                (a - lat).abs() < 1e-9 && (b - lon).abs() < 1e-9,
                "{lat} {lon}"
            );
        }
    }

    #[test]
    fn home_lists_a_coast_with_sea_on_one_side() {
        let root = crate::nav::DEMO_SEED;
        let home = [7, 2, 7, 1, 2, 0, 0, 0, 5];
        let relief = Relief::for_planet(root, &home);
        let places = water_places(&relief, 4_000, 2);
        let coast = places
            .iter()
            .find(|p| p.kind == PlaceKind::Coast)
            .expect("a coast");
        assert!(coast.height_m.abs() < 5.0, "{coast:?}");
        let bearing = coast.sea_bearing_deg.expect("bearing");
        let r = relief.radius_m();
        let land = step(coast.lat_deg, coast.lon_deg, bearing + 180.0, 50.0, r);
        let sea = step(coast.lat_deg, coast.lon_deg, bearing, 50.0, r);
        assert!(relief.terrain_m(land) >= relief.terrain_m(sea), "{coast:?}");
        assert_eq!(places, water_places(&relief, 4_000, 2), "same places");
    }
}
