//! L10 planet record, body recipe, and sampler (#770: surface removed,
//! rebuild in #774-#780).
//!
//! The old flat-faced icosphere surface is gone (ADR 0031). This module keeps
//! the cell-uniform planet recipe (size, tilt, spin, air), the moon sampling,
//! and the scenery markers. No planet mesh is drawn until the cube-sphere
//! rebuild.

use crate::r#gen::{
    AirInfo, Constraints, Form, Generated, Generator, MarkerKind, MoonInfo, Point, SurfaceInfo,
};
use crate::seed::Rng;
use std::f64::consts::PI;

/// Domain-separation tag folded into surface-sampling streams.
///
/// Surface draws use `hash_triple(cell_seed, SURFACE_STREAM_TAG, index)`,
/// never the position stream, so connecting them never moves a marker.
pub const SURFACE_STREAM_TAG: u64 = 0x5FAC_1560_5FAC_1560;

/// Domain-separation tag folded into moon-sampling streams.
pub const MOON_STREAM_TAG: u64 = 0xA00A_1560_A00A_1560;

/// Domain-separation tag folded into tilt/spin sampling streams.
pub const TILT_STREAM_TAG: u64 = 0x7177_1560_7177_1560;

/// Earth equatorial radius in km (IAU home-catalog reference).
///
/// Catalog only: it feeds no scale. The frame and gravity scale lives in
/// `planet_frame::EARTH_RADIUS_M` (mean 6,371 km; review #774 item 5).
pub const EARTH_RADIUS_KM: f64 = 6_378.137;

/// Moon radius in km (home reference).
pub const MOON_RADIUS_KM: f64 = 1_738.0;

/// Moon orbit radius in km (home reference).
pub const MOON_ORBIT_KM: f64 = 384_400.0;

/// Moon orbital period in days (home reference).
pub const MOON_PERIOD_DAYS: f64 = 27.3;

/// Earth axial tilt in degrees (home reference).
pub const EARTH_TILT_DEG: f64 = 23.4;

/// Earth day length in hours (home reference).
pub const EARTH_SPIN_HOURS: f64 = 23.9;

/// Highest tilt sampled in degrees (retrograde Venus regime).
pub const MAX_TILT_DEG: f64 = 177.0;

/// Moon counts of the Solar template by planet order (Mercury to Neptune).
///
/// 0, 0, 1, 2, 95, 274, 28, 16: procedural giants draw up to their class
/// cap, always clipped to the remaining cell budget by the caller.
pub const SOLAR_MOON_COUNTS: [u32; 8] = [0, 0, 1, 2, 95, 274, 28, 16];

/// Earth visible air thickness in km (home reference, #394).
pub const EARTH_AIR_THICKNESS_KM: f64 = 100.0;

/// Pale blue tint of thin Earth-like air (#394).
pub const THIN_AIR_TINT: [f64; 3] = [0.55, 0.75, 1.0];

/// Pale amber tint of thick giant air (#394).
pub const THICK_AIR_TINT: [f64; 3] = [1.0, 0.85, 0.6];

/// Planet radius below which worlds are airless, in Earth radii (#394).
pub const AIRLESS_RADIUS_EARTH: f64 = 0.5;

/// Planet radius at and above which air reads thick, in Earth radii (#394).
pub const GIANT_RADIUS_EARTH: f64 = 3.0;

/// Atmosphere for a planet of `radius_earth` Earth radii (#394).
///
/// Airless below [`AIRLESS_RADIUS_EARTH`], thin pale blue to
/// [`GIANT_RADIUS_EARTH`], thick pale amber above; thickness scales with
/// planet size (`100 km` per Earth radius, so Earth holds `100 km`). Pure:
/// the same radius always yields the same air.
#[must_use]
pub fn air_for_radius_earth(radius_earth: f64) -> Option<AirInfo> {
    if !radius_earth.is_finite() || radius_earth < AIRLESS_RADIUS_EARTH {
        return None;
    }
    let thickness_km = EARTH_AIR_THICKNESS_KM * radius_earth;
    if radius_earth >= GIANT_RADIUS_EARTH {
        Some(AirInfo {
            thickness_km,
            tint: THICK_AIR_TINT,
        })
    } else {
        Some(AirInfo {
            thickness_km,
            tint: THIN_AIR_TINT,
        })
    }
}

/// Cell-uniform planet recipe shared by every surface sample of one cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyRecipe {
    /// Axial tilt in degrees, `0.0..=177.0`.
    pub tilt_deg: f64,
    /// Day length in hours.
    pub spin_hours: f64,
    /// Planet radius in Earth radii (home `1.0`).
    pub radius_earth: f64,
    /// Atmosphere, or `None` when airless.
    pub air: Option<AirInfo>,
}

/// Draws an axial tilt in degrees (`0.0..=177.0`).
///
/// Uniform across the observed span with the sideways and retrograde cases
/// included; chaotic exceptions ride in the same span by design.
#[must_use]
pub fn tilt_deg_for(rng: &mut Rng) -> f64 {
    rng.next_f64() * MAX_TILT_DEG
}

/// Draws a day length in hours (4 to 60, Earth 23.9 inside).
#[must_use]
pub fn spin_hours_for(rng: &mut Rng) -> f64 {
    4.0 + rng.next_f64() * 56.0
}

/// Draws a moon count for a planet of `radius_earth` Earth radii.
///
/// Rocky bodies hold 0 to 2 (Solar inner-system range), giants draw up to
/// their template cap; the caller always clips to the remaining budget.
#[must_use]
pub fn moon_count_for(rng: &mut Rng, radius_earth: f64) -> u32 {
    if !radius_earth.is_finite() || radius_earth < 2.0 {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: rocky moon count saturates like `as`, range 0-2"
        )]
        #[expect(
            clippy::cast_sign_loss,
            reason = "E-CAST: draw is non-negative, saturates like `as`"
        )]
        return (rng.next_f64() * 3.0) as u32;
    }
    let cap = if radius_earth >= 9.0 {
        274
    } else if radius_earth >= 3.5 {
        28
    } else {
        2
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: giant moon count saturates like `as`, clipped by caller"
    )]
    #[expect(
        clippy::cast_sign_loss,
        reason = "E-CAST: draw is non-negative, saturates like `as`"
    )]
    return (rng.next_f64() * f64::from(cap + 1)) as u32;
}

/// Builds the [`SurfaceInfo`] for the planet record.
///
/// The cell recipe (tilt, spin, planet size, air) rides along for draw and
/// proof. Pure in its input: the same recipe always yields the same record.
#[must_use]
pub fn surface_info(body: &BodyRecipe) -> SurfaceInfo {
    SurfaceInfo {
        tilt_deg: body.tilt_deg,
        spin_hours: body.spin_hours,
        radius_earth: body.radius_earth,
        air: body.air,
    }
}

/// Builds the [`MoonInfo`] for one moon at `orbit_km` (catalog or sampled).
#[must_use]
pub fn moon_info(radius_km: f64, orbit_km: f64, period_days: f64) -> MoonInfo {
    MoonInfo {
        radius_km,
        orbit_km,
        period_days,
    }
}

/// Scenery points per L10 cell: the equator ring and the spin axis (#384).
///
/// Both trim from the record budget, so the total never moves.
pub const SCENERY_POINTS: u32 = 2;

/// Surface records per planet cell (#543).
///
/// The body recipe is cell-uniform, so one record carries it.
pub const SURFACE_RECORDS: u32 = 1;

/// Equator ring band width as a share of the former planet radius.
const EQUATOR_BAND: f64 = 0.02;

/// The planet recipe for a cell seeded `seed` (#543).
///
/// Pure in `(seed, home)`: the sampler and the tail's air context read the
/// same values without generating the cell. The class draw on the
/// [`SURFACE_STREAM_TAG`] stream picks the planet size (small, Earth-like,
/// or giant); tilt and spin come from [`TILT_STREAM_TAG`]. The home cell
/// takes the Earth catalog values.
#[must_use]
pub fn body_recipe(seed: u64, home: bool) -> BodyRecipe {
    use crate::seed::hash_triple;
    if home {
        return BodyRecipe {
            tilt_deg: EARTH_TILT_DEG,
            spin_hours: EARTH_SPIN_HOURS,
            radius_earth: 1.0,
            air: air_for_radius_earth(1.0),
        };
    }
    let mut tilt_stream = Rng::new(hash_triple(seed, TILT_STREAM_TAG, 0));
    let mut class_stream = Rng::new(hash_triple(seed, SURFACE_STREAM_TAG, u64::MAX));
    let class = class_stream.next_f64();
    let tilt_deg = tilt_deg_for(&mut tilt_stream);
    let spin_hours = spin_hours_for(&mut tilt_stream);
    let radius_earth = if class < 0.2 {
        0.3
    } else if class < 0.5 {
        1.0
    } else {
        11.0
    };
    BodyRecipe {
        tilt_deg,
        spin_hours,
        radius_earth,
        air: air_for_radius_earth(radius_earth),
    }
}

/// Generator adapter sampling the planet record, moon, and scenery
/// points (#770: region portals removed; moons stay as markers).
///
/// One record carries the cell-uniform recipe; moons orbit compressed into
/// the outer shell band wearing their orbit circles (display compression,
/// the ladder precedent: true moon orbits span dozens of planet radii and
/// never fit one cell); the equator ring and the spin axis draw as scenery.
/// Moons take at most half the budget, scenery [`SCENERY_POINTS`], and
/// the record [`SURFACE_RECORDS`]. Children receive halved budgets, so
/// `respects` holds for every child against the parent the cell was
/// generated with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainSampler {
    /// Earth-Moon home fixture when true (catalog values, L10 home path).
    home: bool,
}

impl TerrainSampler {
    /// Creates a procedural planet sampler (count comes from the parent budget).
    #[must_use]
    pub const fn new() -> TerrainSampler {
        TerrainSampler { home: false }
    }

    /// Creates the Earth-Moon home sampler for the L10 journey cell.
    #[must_use]
    pub const fn home() -> TerrainSampler {
        TerrainSampler { home: true }
    }
}

impl Default for TerrainSampler {
    /// Creates a default procedural planet sampler.
    fn default() -> TerrainSampler {
        TerrainSampler::new()
    }
}

impl Generator for TerrainSampler {
    /// Samples the planet record, moons, and scenery up to
    /// `parent.max_count` points.
    ///
    /// The recipe comes from [`body_recipe`], moons from
    /// [`MOON_STREAM_TAG`], so each lane moves independently. Returns empty
    /// output for an invalid parent rather than panicking.
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        if !parent.is_valid() {
            return Generated {
                points: Vec::new(),
                child_constraints: Vec::new(),
            };
        }
        use crate::seed::hash_triple;
        let body = body_recipe(seed, self.home);
        let moon_target: u32 = if self.home {
            1
        } else {
            let mut moon_stream = Rng::new(hash_triple(seed, MOON_STREAM_TAG, 0));
            moon_count_for(&mut moon_stream, body.radius_earth)
        };
        let budget = parent.max_count;
        let moon_share = moon_target.min(budget / 2);
        let scenery_share = SCENERY_POINTS.min(budget.saturating_sub(moon_share));
        let record_share = SURFACE_RECORDS.min(
            budget
                .saturating_sub(moon_share)
                .saturating_sub(scenery_share),
        );
        let mut points = Vec::new();
        if record_share > 0 {
            // No surface until the rebuild (#770): the record sits at the
            // cell middle.
            points.push(Point {
                position: [0.0, 0.0, 0.0],
                surface: Some(surface_info(&body)),
                ..Point::bare([0.0; 3], 0.01, MarkerKind::Population)
            });
        }
        let mut moon_stream = Rng::new(hash_triple(seed, MOON_STREAM_TAG, 1));
        // Display-compressed moon shell, kept per owner Q7 A ("moons keep
        // their places", review #774 item 3): true orbits span dozens of
        // planet radii and never fit one cell, and the true surface itself
        // nearly fills it (Earth 0.4946 of 0.5 half-bound; giants exceed it),
        // so a surface-scaled band cannot fit either. Moons stay compressed
        // indicators inside the true surface until moon rendering is reworked
        // with the planet surface in a later step; nothing draws the surface
        // yet, so this is invisible.
        for rank in 0..moon_share {
            if points.len() >= budget as usize {
                break;
            }
            let rank_f = f64::from(rank);
            let angle = moon_stream.next_f64() * 2.0 * PI;
            let shell = 0.42 + 0.07 * (rank_f + 1.0) / (f64::from(moon_share) + 1.0);
            let (orbit_km, radius_km, period_days) = if self.home {
                (MOON_ORBIT_KM, MOON_RADIUS_KM, MOON_PERIOD_DAYS)
            } else {
                let orbit = 150_000.0 + rank_f * 150_000.0 + moon_stream.next_f64() * 50_000.0;
                let period = MOON_PERIOD_DAYS * (orbit / MOON_ORBIT_KM).powf(1.5);
                (orbit, 200.0 + moon_stream.next_f64() * 1_500.0, period)
            };
            points.push(Point {
                position: [angle.cos() * shell, angle.sin() * shell, 0.0],
                radius: 0.005,
                kind: MarkerKind::Population,
                moon: Some(moon_info(radius_km, orbit_km, period_days)),
                form: Form::Orbit {
                    normal: [0.0, 0.0, 1.0],
                },
                ..Point::bare([0.0; 3], 0.005, MarkerKind::Population)
            });
        }
        // Equator ring and spin axis as scenery (#770: kept as knowledge,
        // Q7 A; #774 Q7 A: the ring sits at the true radius from this cell's
        // own record). The thread pierces both poles. Like the moons above,
        // the thread keeps its compressed half-length (0.45): a
        // surface-scaled thread cannot fit the cell (Earth's true radius
        // alone is 0.4946 of 0.5 half-bound). Both are reworked with the
        // planet surface in a later step (review #774 item 3).
        let tilt = body.tilt_deg * PI / 180.0;
        let spin_axis = [tilt.sin(), tilt.cos(), 0.0];
        let ring_inner = crate::frame::planet_radius_cells_for(body.radius_earth);
        let scenery = [
            Point {
                position: [0.0, 0.0, 0.0],
                radius: 0.005,
                kind: MarkerKind::Population,
                form: Form::Ring {
                    normal: spin_axis,
                    inner: ring_inner,
                    outer: ring_inner + ring_inner * EQUATOR_BAND,
                },
                ..Point::bare([0.0; 3], 0.005, MarkerKind::Population)
            },
            Point {
                position: [0.0, 0.0, 0.0],
                radius: 0.01,
                kind: MarkerKind::Population,
                form: Form::Thread {
                    to: [
                        spin_axis[0] * 0.45,
                        spin_axis[1] * 0.45,
                        spin_axis[2] * 0.45,
                    ],
                    via: [
                        -spin_axis[0] * 0.45,
                        -spin_axis[1] * 0.45,
                        -spin_axis[2] * 0.45,
                    ],
                },
                ..Point::bare([0.0; 3], 0.01, MarkerKind::Population)
            },
        ];
        for (slot, piece) in scenery.iter().enumerate() {
            if slot < scenery_share as usize && points.len() < budget as usize {
                points.push(*piece);
            }
        }
        let child = Constraints {
            density_multiplier: parent.density_multiplier / 2.0,
            min_count: parent.min_count.min(parent.max_count / 2),
            max_count: parent.max_count / 2,
            allowed_extent: parent.allowed_extent,
        };
        Generated {
            points,
            child_constraints: vec![child; 8],
        }
    }
}

#[cfg(test)]
mod sampler_tests {
    use super::*;
    use crate::r#gen::respects;
    use crate::snapshot::snapshot_generated;

    /// Builds the standard demo budget (mirrors the render demo for L10).
    fn demo_budget() -> Constraints {
        Constraints::new(1.0, 8, 24, [0.5, 0.5, 0.5]).expect("valid test budget")
    }

    #[test]
    fn sampler_is_deterministic() {
        let sampler = TerrainSampler::new();
        let first = snapshot_generated(&sampler.generate(42, &demo_budget()));
        let second = snapshot_generated(&sampler.generate(42, &demo_budget()));
        assert_eq!(first, second);
        assert!(first.contains("points="));
    }

    #[test]
    fn one_surface_record_carries_the_recipe() {
        for (seed, home) in [(7u64, true), (7, false), (42, false), (1 << 33, false)] {
            let sampler = if home {
                TerrainSampler::home()
            } else {
                TerrainSampler::new()
            };
            let out = sampler.generate(seed, &demo_budget());
            let records: Vec<_> = out
                .points
                .iter()
                .filter_map(|point| point.surface)
                .collect();
            assert_eq!(records.len(), 1, "one record per planet (seed {seed})");
            let body = body_recipe(seed, home);
            let record = records[0];
            assert_eq!(record.tilt_deg, body.tilt_deg);
            assert_eq!(record.spin_hours, body.spin_hours);
            assert_eq!(record.radius_earth, body.radius_earth);
            assert_eq!(record.air, body.air);
        }
    }

    #[test]
    fn equator_ring_clears_the_surface() {
        // #774 Q7 A: the ring sits at the true radius from the cell's own
        // record (moons keep their compressed places, so the ring may sit
        // above them for Earth-sized worlds until the rebuild).
        for seed in [1u64, 7, 42, 999] {
            let out = TerrainSampler::new().generate(seed, &demo_budget());
            let body = body_recipe(seed, false);
            let want = crate::frame::planet_radius_cells_for(body.radius_earth);
            let ring = out
                .points
                .iter()
                .find_map(|point| match point.form {
                    Form::Ring { inner, outer, .. } => Some((inner, outer)),
                    _ => None,
                })
                .expect("every planet wears its equator ring");
            assert!(
                (ring.0 - want).abs() < 1e-12,
                "ring at the true radius: {ring:?} vs {want}"
            );
            assert!(ring.1 > ring.0, "ring has width: {ring:?}");
        }
    }

    #[test]
    fn sampler_honors_parent_budget() {
        let sampler = TerrainSampler::new();
        let tight = Constraints::new(1.0, 2, 10, [0.5, 0.5, 0.5]).expect("valid");
        let out = sampler.generate(7, &tight);
        assert!((tight.min_count as usize..=tight.max_count as usize).contains(&out.points.len()));
        for point in &out.points {
            for axis in point.position {
                assert!((-0.5..0.5).contains(&axis), "out of cell: {axis}");
            }
            assert!(point.radius > 0.0);
        }
    }

    #[test]
    fn sampler_children_respect_parent() {
        let sampler = TerrainSampler::new();
        let parent = demo_budget();
        for child in &sampler.generate(7, &parent).child_constraints {
            assert!(respects(child, &parent));
        }
    }

    #[test]
    fn sampler_rejects_invalid_parent() {
        let sampler = TerrainSampler::new();
        let bad = Constraints {
            density_multiplier: 2.0,
            min_count: 9,
            max_count: 3,
            allowed_extent: [0.5, 0.5, 0.5],
        };
        assert!(!bad.is_valid());
        assert!(sampler.generate(7, &bad).points.is_empty());
    }

    #[test]
    fn home_sampler_holds_earth_one_moon_and_six_portals() {
        let out = TerrainSampler::home().generate(7, &demo_budget());
        assert_eq!(
            out.points.len(),
            4,
            "one record, the Moon, two scenery (#770: portals removed)"
        );
        let moons = out
            .points
            .iter()
            .filter(|point| point.moon.is_some())
            .count();
        assert_eq!(moons, 1);
        let portals = out
            .points
            .iter()
            .filter(|point| point.kind == MarkerKind::Portal)
            .count();
        assert_eq!(portals, 0, "#770: region portals removed");
        for point in &out.points {
            if let Some(surface) = point.surface {
                assert_eq!(surface.tilt_deg, EARTH_TILT_DEG);
                assert_eq!(surface.spin_hours, EARTH_SPIN_HOURS);
                assert_eq!(surface.radius_earth, 1.0);
                let air = surface.air.expect("home planet holds air");
                assert_eq!(air.thickness_km, EARTH_AIR_THICKNESS_KM);
                assert_eq!(air.tint, THIN_AIR_TINT);
            }
        }
    }

    #[test]
    fn air_follows_planet_size_with_earth_pinned() {
        assert!(
            air_for_radius_earth(0.3).is_none(),
            "small rocks are airless"
        );
        assert!(air_for_radius_earth(f64::NAN).is_none());
        let thin = air_for_radius_earth(1.0).expect("Earth holds thin air");
        assert_eq!(thin.thickness_km, EARTH_AIR_THICKNESS_KM);
        assert_eq!(thin.tint, THIN_AIR_TINT);
        let thick = air_for_radius_earth(11.0).expect("giants hold thick air");
        assert_eq!(thick.thickness_km, EARTH_AIR_THICKNESS_KM * 11.0);
        assert_eq!(thick.tint, THICK_AIR_TINT);
    }

    #[test]
    fn procedural_moons_fit_the_shell_and_budget() {
        let parent = Constraints::new(1.0, 8, 24, [0.5, 0.5, 0.5]).expect("valid");
        for seed in [1u64, 7, 42, 999, 1 << 33] {
            let out = TerrainSampler::new().generate(seed, &parent);
            let moons = out
                .points
                .iter()
                .filter(|point| point.moon.is_some())
                .count();
            assert_eq!(
                out.points.len(),
                1 + moons + 2,
                "record, moons, scenery (#770: portals removed)"
            );
            let portals = out
                .points
                .iter()
                .filter(|point| point.kind == MarkerKind::Portal)
                .count();
            assert_eq!(portals, 0, "#770: region portals removed");
            for point in &out.points {
                for axis in point.position {
                    assert!((-0.5..0.5).contains(&axis), "out of cell: {axis}");
                }
                if point.moon.is_some() {
                    assert_eq!(point.kind, MarkerKind::Population);
                    assert!(point.surface.is_none(), "a point is either surface or moon");
                    let reach = (point.position[0] * point.position[0]
                        + point.position[1] * point.position[1]
                        + point.position[2] * point.position[2])
                        .sqrt();
                    assert!(
                        (0.42..0.49).contains(&reach),
                        "moons orbit outside the body: {:?}",
                        point.position
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod surface_tests {
    use super::*;

    #[test]
    fn tilt_and_spin_stay_in_their_spans() {
        let mut stream = Rng::new(0x7117_1560_7117_1560);
        for _ in 0..500 {
            let tilt = tilt_deg_for(&mut stream);
            assert!(
                (0.0..=MAX_TILT_DEG).contains(&tilt),
                "tilt out of span: {tilt}"
            );
            let spin = spin_hours_for(&mut stream);
            assert!((4.0..=60.0).contains(&spin), "spin out of span: {spin}");
        }
    }

    #[test]
    fn moon_counts_follow_the_template_caps() {
        let mut stream = Rng::new(0xA00A_1560_00AA_1560);
        for _ in 0..200 {
            assert!(moon_count_for(&mut stream, 1.0) <= 2, "rocky cap is 2");
            assert!(moon_count_for(&mut stream, 11.2) <= 274, "giant cap is 274");
            assert!(
                moon_count_for(&mut stream, 4.0) <= 28,
                "ice-giant cap is 28"
            );
        }
    }

    #[test]
    fn surface_info_is_deterministic() {
        let body = BodyRecipe {
            tilt_deg: EARTH_TILT_DEG,
            spin_hours: EARTH_SPIN_HOURS,
            radius_earth: 1.0,
            air: air_for_radius_earth(1.0),
        };
        assert_eq!(surface_info(&body), surface_info(&body));
        assert_eq!(surface_info(&body).tilt_deg, EARTH_TILT_DEG);
    }
}
