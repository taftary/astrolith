//! L10 planet terrain: the relief field, the body recipe, and the sampler
//! (M5, sub-issue #42; bare planet since #375; one icosphere since #458).
//!
//! The relief is one 3D fractal field read through
//! [`height_at`](crate::terrain::height_at). Any direction maps to it through
//! [`face_uv_for`](crate::terrain::face_uv_for) (a cube-face parameterization
//! of the same field, so neighbouring directions always agree), and
//! [`surface_radius`](crate::terrain::surface_radius) turns it into the
//! radius the one icosphere surface (ADR 0024), its portals, and the ground
//! all share. The drawn mesh splits by viewing angle in the renderer; there
//! is no distance level-of-detail table here (#543).

use crate::r#gen::{
    AirInfo, Constraints, Form, Generated, Generator, MarkerKind, MoonInfo, Point, SurfaceInfo,
};
use crate::noise::fbm_3d;
use crate::seed::Rng;
use std::f64::consts::PI;

/// Fractal octaves summed by [`height_at`].
///
/// Five octaves span continent swells down to mountain-range detail at
/// indicator resolution.
pub const TERRAIN_OCTAVES: u32 = 5;

/// Frequency scaling cube coordinates (`[-1.0, 1.0]`) into field space.
pub const TERRAIN_FREQUENCY: f64 = 2.0;

/// Samples the L10 heightmap at `(u, v)` on cube `face`.
///
/// Faces are `0..=5` (`+X`, `-X`, `+Y`, `-Y`, `+Z`, `-Z`); `u`/`v` span
/// `[0.0, 1.0]` edge to edge, with out-of-range values still mapping
/// deterministically through the global field. The `(face, u, v)` triple
/// maps to a 3D cube point sampled with fractal value noise over
/// [`TERRAIN_OCTAVES`] octaves, so the result is deterministic on every
/// platform in `[0.0, 1.0]` (`0.0` abyss, `1.0` peak). Faces outside `0..=5`
/// have no cube point and yield `0.0`; non-finite `u`/`v` yield the neutral
/// `0.5` instead of propagating `NaN`.
///
/// Edge layout (with `a = 2u - 1`, `b = 2v - 1`): face 0 is `(1, a, b)`,
/// face 1 is `(-1, a, b)`, face 2 is `(a, 1, b)`, face 3 is `(a, -1, b)`,
/// face 4 is `(a, b, 1)`, face 5 is `(a, b, -1)`.
#[must_use]
pub fn height_at(seed: u64, face: u8, u: f64, v: f64) -> f64 {
    let Some(point) = cube_point(face, u, v) else {
        return 0.0;
    };
    fbm_3d(
        seed,
        point[0] * TERRAIN_FREQUENCY,
        point[1] * TERRAIN_FREQUENCY,
        point[2] * TERRAIN_FREQUENCY,
        TERRAIN_OCTAVES,
    )
}

/// Maps `(face, u, v)` onto the cube surface; `None` for faces above 5.
fn cube_point(face: u8, u: f64, v: f64) -> Option<[f64; 3]> {
    let a = u * 2.0 - 1.0;
    let b = v * 2.0 - 1.0;
    match face {
        0 => Some([1.0, a, b]),
        1 => Some([-1.0, a, b]),
        2 => Some([a, 1.0, b]),
        3 => Some([a, -1.0, b]),
        4 => Some([a, b, 1.0]),
        5 => Some([a, b, -1.0]),
        _ => None,
    }
}

/// Planet body radius in cell units for L10 surface points.
///
/// Surface relief rides on top of this radius (see
/// [`RELIEF_EXAGGERATION`); moons orbit outside it.
pub const PLANET_RADIUS_CELL: f64 = 0.35;

/// Declared vertical exaggeration of surface relief (visible by design).
///
/// Heights in `[0.0, 1.0]` map to `radius +/- RELIEF_RANGE_CELL`, about
/// eight times true scale on an Earth-sized body, so ranges read at all.
pub const RELIEF_RANGE_CELL: f64 = 0.05;

/// Domain-separation tag folded into surface-sampling streams.
///
/// Surface draws use `hash_triple(cell_seed, SURFACE_STREAM_TAG, index)`,
/// never the position stream, so connecting them never moves a marker.
pub const SURFACE_STREAM_TAG: u64 = 0x5FAC_1560_5FAC_1560;

/// Domain-separation tag folded into moon-sampling streams.
pub const MOON_STREAM_TAG: u64 = 0xA00A_1560_A00A_1560;

/// Domain-separation tag folded into tilt/spin sampling streams.
pub const TILT_STREAM_TAG: u64 = 0x7177_1560_7177_1560;

/// Earth equatorial radius in km (home reference).
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

/// Maps a spheroid direction back onto `(face, u, v)` cube coordinates.
///
/// Dominant-axis projection onto the cube faces of the relief field, so
/// every direction reads the same [`height_at`] field and neighbouring
/// directions always agree.
#[must_use]
pub fn face_uv_for(direction: [f64; 3]) -> (u8, f64, f64) {
    let x = direction[0].abs();
    let y = direction[1].abs();
    let z = direction[2].abs();
    if !(x > 0.0 || y > 0.0 || z > 0.0) {
        return (0, 0.5, 0.5);
    }
    if x >= y && x >= z {
        let face = if direction[0] >= 0.0 { 0 } else { 1 };
        (
            face,
            direction[1] / x * 0.5 + 0.5,
            direction[2] / x * 0.5 + 0.5,
        )
    } else if y >= z {
        let face = if direction[1] >= 0.0 { 2 } else { 3 };
        (
            face,
            direction[0] / y * 0.5 + 0.5,
            direction[2] / y * 0.5 + 0.5,
        )
    } else {
        let face = if direction[2] >= 0.0 { 4 } else { 5 };
        (
            face,
            direction[0] / z * 0.5 + 0.5,
            direction[1] / z * 0.5 + 0.5,
        )
    }
}

/// Samples one surface height in `[0.0, 1.0]`.
///
/// The single-peak fractal field reads straight through [`height_at`]:
/// relief without seas or climate bands (#375). Pure in `(seed, face, u,
/// v)`: grid samples and drawn mesh vertices always agree.
#[must_use]
pub fn surface_height(seed: u64, face: u8, u: f64, v: f64) -> f64 {
    height_at(seed, face, u, v)
}

/// Radius of the icosphere surface under `direction` for `seed` (#458).
///
/// The same fractal field the sampler and the drawn mesh read
/// ([`surface_height`] through [`face_uv_for`]) with the same
/// [`RELIEF_RANGE_CELL`] at every level (Q6), so orbit and ground agree
/// (C2). Falls back to [`PLANET_RADIUS_CELL`] for degenerate directions.
#[must_use]
pub fn surface_radius(seed: u64, direction: [f64; 3]) -> f64 {
    let (face, u, v) = face_uv_for(direction);
    let height = surface_height(seed, face, u, v);
    if !height.is_finite() {
        return PLANET_RADIUS_CELL;
    }
    PLANET_RADIUS_CELL + (height - 0.5) * 2.0 * RELIEF_RANGE_CELL
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

/// Builds the [`SurfaceInfo`] for one L10 surface sample.
///
/// Height comes from [`surface_height`]; the cell recipe (tilt, spin,
/// planet size, air) rides along for draw and proof. Pure in its inputs:
/// the same sample always yields the same surface.
#[must_use]
pub fn surface_info(seed: u64, face: u8, u: f64, v: f64, body: &BodyRecipe) -> SurfaceInfo {
    SurfaceInfo {
        height: surface_height(seed, face, u, v),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_is_deterministic_and_seed_sensitive() {
        let seed = 0xCAFE_F00D;
        assert_eq!(
            height_at(seed, 4, 0.25, 0.75).to_bits(),
            height_at(seed, 4, 0.25, 0.75).to_bits()
        );
        assert_ne!(
            height_at(seed, 4, 0.25, 0.75).to_bits(),
            height_at(seed ^ 0x1, 4, 0.25, 0.75).to_bits()
        );
    }

    #[test]
    fn height_stays_in_unit_range() {
        let seed = 4242;
        for face in 0..6u8 {
            for t in [0.0, 0.25, 0.5, 0.75, 1.0, -0.5, 1.5] {
                let height = height_at(seed, face, t, 1.0 - t);
                assert!((0.0..=1.0).contains(&height), "out of range: {height}");
            }
        }
        assert_eq!(
            height_at(seed, 0, f64::NAN, 0.5).to_bits(),
            0.5f64.to_bits()
        );
    }

    #[test]
    fn shared_edges_and_corners_agree_across_faces() {
        let seed = 0xCAFE_F00D;
        // Edge (1, 1, b) shared by faces 0 (+X, u=1) and 2 (+Y, u=1).
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let from_plus_x = height_at(seed, 0, 1.0, t);
            let from_plus_y = height_at(seed, 2, 1.0, t);
            assert!(
                (from_plus_x - from_plus_y).abs() <= 1e-12,
                "edge mismatch at t={t}: {from_plus_x} vs {from_plus_y}"
            );
        }
        // Edge (-1, -1, b) shared by faces 1 (-X, u=0) and 3 (-Y, u=0).
        for t in [0.0, 0.5, 1.0] {
            let from_minus_x = height_at(seed, 1, 0.0, t);
            let from_minus_y = height_at(seed, 3, 0.0, t);
            assert!(
                (from_minus_x - from_minus_y).abs() <= 1e-12,
                "edge mismatch at t={t}: {from_minus_x} vs {from_minus_y}"
            );
        }
        // Corner (1, 1, 1) shared by faces 0, 2, and 4.
        let corner = [
            height_at(seed, 0, 1.0, 1.0),
            height_at(seed, 2, 1.0, 1.0),
            height_at(seed, 4, 1.0, 1.0),
        ];
        for pair in corner.windows(2) {
            assert!(
                (pair[0] - pair[1]).abs() <= 1e-12,
                "corner mismatch: {} vs {}",
                pair[0],
                pair[1]
            );
        }
        // Corner (-1, -1, -1) shared by faces 1, 3, and 5.
        let opposite = [
            height_at(seed, 1, 0.0, 0.0),
            height_at(seed, 3, 0.0, 0.0),
            height_at(seed, 5, 0.0, 0.0),
        ];
        for pair in opposite.windows(2) {
            assert!(
                (pair[0] - pair[1]).abs() <= 1e-12,
                "corner mismatch: {} vs {}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn unknown_face_yields_zero() {
        assert_eq!(height_at(7, 6, 0.5, 0.5).to_bits(), 0.0f64.to_bits());
        assert_eq!(height_at(7, u8::MAX, 0.5, 0.5).to_bits(), 0.0f64.to_bits());
    }
}

/// Region portals per L10 cell: six icosphere clusters open into L11 (#375, #458).
pub const REGION_PORTALS: u32 = 6;

/// Scenery points per L10 cell: the equator ring and the spin axis (#384).
///
/// Both trim from the surface budget, so the total never moves.
pub const SCENERY_POINTS: u32 = 2;

/// Surface records per planet cell (#543).
///
/// The body recipe is cell-uniform, so one surface point carries it; the
/// relief itself is the icosphere mesh, never these points.
pub const SURFACE_RECORDS: u32 = 1;

/// Equator ring band width as a share of [`PLANET_RADIUS_CELL`].
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

/// Generator adapter sampling the planet record, moon, and region-portal
/// points (M5 demo/`--verify`; portals since #375; forms since #384;
/// icosphere clusters since #458; one record since #543).
///
/// One surface record carries the cell-uniform recipe at the north pole on
/// the relief; moons orbit compressed into the outer shell band wearing
/// their orbit circles (display compression, the ladder precedent: true
/// moon orbits span dozens of planet radii and never fit one cell); region
/// portals sit on the one icosphere surface at the enterable cluster anchor
/// tiles as patches (ADR 0024), so the planet always opens onward and its
/// regions nest; the equator ring (lifted above the highest relief) and the
/// spin axis draw as scenery. Moons take at most half the budget, portals
/// take at most [`REGION_PORTALS`] slots, scenery [`SCENERY_POINTS`], and
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
    /// Samples the planet record, moons, portals, and scenery up to
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
        let moon_room = moon_target.min(budget / 2);
        let portal_room = REGION_PORTALS.min(budget.saturating_sub(moon_room));
        let scenery_room =
            SCENERY_POINTS.min(budget.saturating_sub(moon_room).saturating_sub(portal_room));
        let record_room = SURFACE_RECORDS.min(
            budget
                .saturating_sub(moon_room)
                .saturating_sub(portal_room)
                .saturating_sub(scenery_room),
        );
        let mut points = Vec::new();
        if record_room > 0 {
            // The one record sits on the relief at the north pole.
            let pole = [0.0, 1.0, 0.0];
            let (face, u, v) = face_uv_for(pole);
            let radius = surface_radius(seed, pole);
            points.push(Point {
                position: [0.0, radius, 0.0],
                surface: Some(surface_info(seed, face, u, v, &body)),
                ..Point::bare([0.0; 3], 0.01, MarkerKind::Population)
            });
        }
        let mut moon_stream = Rng::new(hash_triple(seed, MOON_STREAM_TAG, 1));
        for rank in 0..moon_room {
            if points.len() >= budget as usize {
                break;
            }
            let rank_f = f64::from(rank);
            let angle = moon_stream.next_f64() * 2.0 * PI;
            let shell = 0.42 + 0.07 * (rank_f + 1.0) / (f64::from(moon_room) + 1.0);
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
        // Region portals sit on the one icosphere surface at the enterable
        // cluster anchor tiles (ADR 0024), the same way in solar-system
        // interiors and open planets (#543): each lands on the relief with
        // the anchor direction as its patch normal, so the region opens
        // exactly there with its ground tangent to the planet.
        let frame = crate::surface::SurfaceFrame::planet(seed);
        let anchors = frame
            .clusters()
            .into_iter()
            .filter(|cluster| cluster.enterable)
            .take(portal_room as usize);
        for cluster in anchors {
            if points.len() >= budget as usize {
                break;
            }
            // Anchors are member tiles of a grown cluster, so they always
            // have a centre; a malformed one is skipped, never invented.
            let Some(direction) = crate::icosphere::tri_centre(cluster.anchor) else {
                continue;
            };
            let radius = surface_radius(seed, direction);
            let [dx, dy, dz] = direction;
            points.push(Point {
                position: [dx * radius, dy * radius, dz * radius],
                radius: 0.01,
                kind: MarkerKind::Portal,
                form: Form::Patch { normal: direction },
                ..Point::bare([0.0; 3], 0.01, MarkerKind::Portal)
            });
        }
        // Equator ring and spin axis as scenery: the ring circles the belly
        // in the plane normal to the spin axis just above the highest
        // relief, so the surface never hides it (#543); the thread pierces
        // both poles. Both trim from the record budget, never from portals.
        let tilt = body.tilt_deg * PI / 180.0;
        let spin_axis = [tilt.sin(), tilt.cos(), 0.0];
        let ring_inner = PLANET_RADIUS_CELL + RELIEF_RANGE_CELL;
        let scenery = [
            Point {
                position: [0.0, 0.0, 0.0],
                radius: 0.005,
                kind: MarkerKind::Population,
                form: Form::Ring {
                    normal: spin_axis,
                    inner: ring_inner,
                    outer: ring_inner + PLANET_RADIUS_CELL * EQUATOR_BAND,
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
            if slot < scenery_room as usize && points.len() < budget as usize {
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
    fn equator_ring_clears_the_highest_relief() {
        for seed in [1u64, 7, 42, 999] {
            let out = TerrainSampler::new().generate(seed, &demo_budget());
            let ring = out
                .points
                .iter()
                .find_map(|point| match point.form {
                    Form::Ring { inner, outer, .. } => Some((inner, outer)),
                    _ => None,
                })
                .expect("every planet wears its equator ring");
            assert!(
                ring.0 >= PLANET_RADIUS_CELL + RELIEF_RANGE_CELL,
                "ring inside the relief: {ring:?}"
            );
            assert!(
                ring.1 > ring.0 && ring.1 < 0.42,
                "ring under the moons: {ring:?}"
            );
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
            10,
            "one record, the Moon, six portals, two scenery"
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
        assert_eq!(portals, 6, "every planet offers six region portals");
        for point in &out.points {
            if point.kind == MarkerKind::Portal {
                assert!(
                    point.surface.is_none() && point.moon.is_none(),
                    "region portals carry no data"
                );
            } else if let Some(surface) = point.surface {
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
    fn region_portals_ride_the_icosphere_surface() {
        let seed = 7u64;
        let out = TerrainSampler::home().generate(seed, &demo_budget());
        let portals: Vec<_> = out
            .points
            .iter()
            .filter(|point| point.kind == MarkerKind::Portal)
            .collect();
        assert_eq!(portals.len(), 6, "every planet offers six region portals");
        for point in portals {
            let [x, y, z] = point.position;
            let distance = (x * x + y * y + z * z).sqrt();
            let expected = surface_radius(seed, point.position);
            assert!(
                (distance - expected).abs() < 1e-9,
                "portal off the surface: {distance} vs {expected}"
            );
            // Patch normal stays radial: the region ground is tangent.
            if let Form::Patch { normal } = point.form {
                let [nx, ny, nz] = normal;
                let length = (nx * nx + ny * ny + nz * nz).sqrt();
                assert!((length - 1.0).abs() < 1e-12, "portal normal off unit");
                assert!(
                    (nx * x + ny * y + nz * z) / distance > 1.0 - 1e-12,
                    "portal normal leaves radial"
                );
            } else {
                panic!("region portal wears {:?}", point.form);
            }
        }
        // Same seed, same portals on every run.
        let again = TerrainSampler::home().generate(seed, &demo_budget());
        assert_eq!(
            snapshot_generated(&out),
            snapshot_generated(&again),
            "region clusters move between runs"
        );
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
                1 + moons + 6 + 2,
                "record, moons, portals, scenery"
            );
            let portals = out
                .points
                .iter()
                .filter(|point| point.kind == MarkerKind::Portal)
                .count();
            assert_eq!(portals, 6, "every planet offers six region portals");
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
    fn face_uv_round_trips_grid_samples() {
        // Interior samples only: edges and corners are shared seams where any
        // adjacent face is a correct answer.
        for face in 0..6u8 {
            for (u, v) in [(0.25, 0.75), (0.5, 0.5), (0.75, 0.25)] {
                let cube = cube_point(face, u, v).expect("valid face");
                let direction = crate::icosphere::normalize(cube);
                let (back_face, back_u, back_v) = face_uv_for(direction);
                assert_eq!(back_face, face, "face must survive the round trip");
                assert!(
                    (back_u - u).abs() <= 1e-12 && (back_v - v).abs() <= 1e-12,
                    "uv must survive the round trip: ({back_u}, {back_v}) vs ({u}, {v})"
                );
            }
        }
        assert_eq!(face_uv_for([0.0, 0.0, 0.0]), (0, 0.5, 0.5));
    }

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
    fn surface_info_is_deterministic_and_in_range() {
        let body = BodyRecipe {
            tilt_deg: EARTH_TILT_DEG,
            spin_hours: EARTH_SPIN_HOURS,
            radius_earth: 1.0,
            air: air_for_radius_earth(1.0),
        };
        let first = surface_info(7, 4, 0.25, 0.75, &body);
        let second = surface_info(7, 4, 0.25, 0.75, &body);
        assert_eq!(first, second);
        assert!((0.0..=1.0).contains(&first.height));
        assert_ne!(
            surface_info(7, 4, 0.25, 0.75, &body),
            surface_info(7, 4, 0.75, 0.75, &body)
        );
    }

    #[test]
    fn surface_height_agrees_from_orbit_to_ground() {
        let seed = 0xCAFE_F00D;
        for direction in [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.3, -0.5, 0.8],
            [-0.6, 0.2, -0.4],
        ] {
            let (face, u, v) = face_uv_for(direction);
            let expected = PLANET_RADIUS_CELL
                + (surface_height(seed, face, u, v) - 0.5) * 2.0 * RELIEF_RANGE_CELL;
            assert_eq!(
                surface_radius(seed, direction).to_bits(),
                expected.to_bits()
            );
            // Scaling the direction (orbit vs ground distance) changes nothing.
            let far = [
                direction[0] * 10.0,
                direction[1] * 10.0,
                direction[2] * 10.0,
            ];
            assert_eq!(
                surface_radius(seed, far).to_bits(),
                expected.to_bits(),
                "orbit and ground disagree"
            );
        }
        assert_eq!(
            surface_radius(seed, [0.0; 3]),
            surface_radius(seed, [1.0, 0.0, 0.0])
        );
    }
}
