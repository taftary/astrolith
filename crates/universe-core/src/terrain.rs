//! L10 cube-sphere terrain with level of detail (M5, sub-issue #42; bare
//! planet since #375).
//!
//! Planets carry a heightmap on a cube-sphere: each of the six cube faces
//! samples the SAME 3D fractal field through [`height_at`](crate::terrain::height_at), so a shared edge
//! or corner maps to identical 3D points from every adjacent face and borders
//! agree by construction. [`lod_for`](crate::terrain::lod_for) selects the subdivision depth from the camera
//! distance (monotonic: nearer cameras never select coarser detail).

use crate::r#gen::{
    AirInfo, Constraints, Form, Generated, Generator, MarkerKind, MoonInfo, Point, SurfaceInfo,
};
use crate::coords::Level;
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

/// Deepest terrain subdivision (camera at or inside the surface).
pub const MAX_LOD: u8 = 8;

/// Reference camera distance, in meters, holding [`MAX_LOD`].
///
/// Each doubling of distance above this drops one LOD rung.
pub const LOD_REFERENCE_DISTANCE: f64 = 2_000.0;

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

/// Selects the terrain subdivision for a camera `distance`, in meters.
///
/// [`MAX_LOD`] at or below [`LOD_REFERENCE_DISTANCE`], minus one rung per
/// doubling above it, floored at `0`. Monotonic non-increasing in distance:
/// nearer cameras never select coarser detail. Non-finite or non-positive
/// distances (camera at or inside the surface) request [`MAX_LOD`].
#[must_use]
pub fn lod_for(distance: f64) -> u8 {
    if !distance.is_finite() || distance <= 0.0 {
        return MAX_LOD;
    }
    let mut lod = MAX_LOD;
    let mut threshold = LOD_REFERENCE_DISTANCE;
    while distance > threshold && lod > 0 {
        threshold *= 2.0;
        lod -= 1;
    }
    lod
}

/// Oblate flattening of Earth-like planets (equatorial 6,378.137 km).
///
/// Spheres squash their polar axis by this fraction; gas giants reuse the
/// same constant until per-class shapes land.
pub const EARTH_FLATTENING: f64 = 1.0 / 298.0;

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

/// Standoff kept above the relief and the ground (#430), in open-cell units.
///
/// Window dives and window free flight never dip below the surface plus
/// this: it clears the relief jitter (`±0.005`), the polar squash (at most
/// `PLANET_RADIUS_CELL * 0.05 ~= 0.018`), and the tail ground jitter, while
/// staying far below the landing rest (about 3.58 child units above a
/// portal), so landings never touch it.
pub const GROUND_STANDOFF: f64 = 0.03;

/// Relief radius under `direction` for the L10 cell `seed` (#430).
///
/// The same fractal field the sampler and the drawn mesh read
/// ([`surface_height`] through [`face_uv_for`]), so the clamp agrees with
/// the body on screen. Falls back to [`PLANET_RADIUS_CELL`] for degenerate
/// directions.
#[must_use]
pub fn relief_radius(seed: u64, direction: [f64; 3]) -> f64 {
    let (face, u, v) = face_uv_for(direction);
    let height = surface_height(seed, face, u, v);
    PLANET_RADIUS_CELL + (height - 0.5) * 2.0 * RELIEF_RANGE_CELL
}

/// Ground height under `(x, z)` at a tail level (#430).
///
/// The settled bowl `y = -(x^2 + z^2) / (2R)` the tail generator lays
/// points on, with `R` the true planet radius at this level's scale.
/// Returns `0.0` outside L11-L13 or when the radius is degenerate.
#[must_use]
pub fn ground_height(level: Level, x: f64, z: f64) -> f64 {
    if !(11..=13).contains(&level.get()) {
        return 0.0;
    }
    if !x.is_finite() || !z.is_finite() {
        return 0.0;
    }
    let Some(radius) = crate::frame::planet_radius_cells(level) else {
        return 0.0;
    };
    if !radius.is_finite() || radius <= 0.0 {
        return 0.0;
    }
    -(x * x + z * z) / (2.0 * radius)
}

/// Camera clamped above the relief and the ground (#430).
///
/// L10 keeps `|camera|` above the relief under it plus [`GROUND_STANDOFF`]
/// (radial push-out, `+Y` when on the centre); L11-L13 keep `camera.y`
/// above [`ground_height`] plus the standoff. Everywhere else, and for
/// non-finite inputs, the camera passes through unchanged. Pure and
/// deterministic: the window dive and window free flight share it, replays
/// never call it (`E-GOLDEN`).
#[must_use]
pub fn clamp_above_ground(level: Level, seed: u64, camera: [f64; 3]) -> [f64; 3] {
    clamp_above_ground_with_cap(level, seed, camera, None)
}

/// Camera clamped above the relief and the ground, with a landing cap (#430).
///
/// `floor_cap` lowers the floor to the landing point's height when set: a
/// window landing dive passes its target portal's rest height
/// ([`crate::landing::landing_distance`] above the portal), which sits
/// below the standoff at deep tail levels (L12) yet always above the
/// surface itself, so the cap keeps the dive reachable without ever
/// letting the camera under the ground. `None` keeps the full standoff
/// (centre dives, free flight, transitions). The cap is in the same units
/// as the floor: radial distance at L10, height at L11-L13.
#[must_use]
pub fn clamp_above_ground_with_cap(
    level: Level,
    seed: u64,
    camera: [f64; 3],
    floor_cap: Option<f64>,
) -> [f64; 3] {
    if !camera.iter().all(|c| c.is_finite()) {
        return camera;
    }
    match level.get() {
        10 => {
            let distance = (camera[0] * camera[0] + camera[1] * camera[1] + camera[2] * camera[2])
                .sqrt();
            let mut floor = relief_radius(seed, camera) + GROUND_STANDOFF;
            if let Some(cap) = floor_cap
                && cap.is_finite()
            {
                floor = floor.min(cap);
            }
            if !floor.is_finite() {
                return camera;
            }
            if distance < floor {
                if distance > 1e-12 {
                    let push = floor / distance;
                    [camera[0] * push, camera[1] * push, camera[2] * push]
                } else {
                    [0.0, floor, 0.0]
                }
            } else {
                camera
            }
        }
        11..=13 => {
            let mut floor = ground_height(level, camera[0], camera[2]) + GROUND_STANDOFF;
            if let Some(cap) = floor_cap
                && cap.is_finite()
            {
                floor = floor.min(cap);
            }
            if !floor.is_finite() {
                return camera;
            }
            if camera[1] < floor {
                [camera[0], floor, camera[2]]
            } else {
                camera
            }
        }
        _ => camera,
    }
}

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
    /// Oblate flattening (Earth `1/298`).
    pub flattening: f64,
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
/// Dominant-axis projection, the inverse of [`sphere_point`] up to the polar
/// squash: drawn mesh vertices sample the same [`surface_info`] field as the
/// sampler grid through this map, so bodies and points always agree.
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

/// Maps `(face, u, v)` onto the oblate spheroid surface.
///
/// The cube point normalizes to the unit sphere and the polar axis squashes
/// by `flattening`, so shared edges and corners agree exactly like the flat
/// field: normalization is continuous across every seam. Faces outside
/// `0..=5` yield `None`; a non-positive radius scale yields the center.
#[must_use]
pub fn sphere_point(face: u8, u: f64, v: f64, flattening: f64) -> Option<[f64; 3]> {
    let cube = cube_point(face, u, v)?;
    let length = (cube[0] * cube[0] + cube[1] * cube[1] + cube[2] * cube[2]).sqrt();
    if !length.is_finite() || length <= 0.0 {
        return Some([0.0, 0.0, 0.0]);
    }
    let squash = if flattening.is_finite() {
        (1.0 - flattening).clamp(0.5, 1.0)
    } else {
        1.0
    };
    Some([
        cube[0] / length,
        cube[1] / length * squash,
        cube[2] / length,
    ])
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
/// Height comes from [`surface_height`]; the cell recipe (flattening, tilt,
/// spin, planet size, air) rides along for draw and proof. Pure in its inputs:
/// the same sample always yields the same surface.
#[must_use]
pub fn surface_info(seed: u64, face: u8, u: f64, v: f64, body: &BodyRecipe) -> SurfaceInfo {
    SurfaceInfo {
        height: surface_height(seed, face, u, v),
        flattening: body.flattening,
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

    #[test]
    fn lod_is_monotonic_in_distance() {
        assert_eq!(lod_for(0.0), MAX_LOD);
        assert_eq!(lod_for(-100.0), MAX_LOD);
        assert_eq!(lod_for(f64::NAN), MAX_LOD);
        assert_eq!(lod_for(f64::INFINITY), MAX_LOD);
        assert_eq!(lod_for(LOD_REFERENCE_DISTANCE), MAX_LOD);
        assert_eq!(lod_for(LOD_REFERENCE_DISTANCE + 1.0), MAX_LOD - 1);
        assert_eq!(lod_for(1.0e12), 0);
        let mut previous = MAX_LOD;
        let mut distance = 1.0;
        while distance < 1.0e12 {
            let lod = lod_for(distance);
            assert!(
                lod <= previous,
                "LOD rose with distance: {lod} after {previous}"
            );
            previous = lod;
            distance *= 1.5;
        }
        assert_eq!(previous, 0);
    }
}

/// Grid samples per cube-face axis taken by [`TerrainSampler`].
///
/// Six faces of `SAMPLER_GRID x SAMPLER_GRID` give 54 candidates; the sampler
/// emits at most the parent budget from this fixed order.
pub const SAMPLER_GRID: u32 = 3;

/// Region portals per L10 cell: the six cube-face centers open into L11 (#375).
pub const REGION_PORTALS: u32 = 6;

/// Scenery points per L10 cell: the equator ring and the spin axis (#384).
///
/// Both trim from the surface budget, so the total never moves.
pub const SCENERY_POINTS: u32 = 2;

/// Generator adapter sampling planet surface, moon, and region-portal points
/// (M5 demo/`--verify`; portals since #375; forms since #384).
///
/// Surface samples ride the oblate spheroid (`sphere_point` at
/// [`PLANET_RADIUS_CELL`] plus relief to [`RELIEF_RANGE_CELL`]); moons orbit
/// compressed into the outer shell band wearing their orbit circles
/// (display compression, the ladder precedent: true moon orbits span dozens
/// of planet radii and never fit one cell); region portals sit on the
/// surface at the six face centers as patches, so the planet always opens
/// onward; the equator ring and the spin axis draw as scenery. Moons take at
/// most half the budget, portals take at most [`REGION_PORTALS`] slots, and
/// scenery takes [`SCENERY_POINTS`] slots, so the surface always resolves.
/// Children receive halved budgets, so `respects` holds for every child
/// against the parent the cell was generated with.
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
    /// Samples spheroid surface points plus moon points up to
    /// `parent.max_count` points.
    ///
    /// Surface draws come from the [`SURFACE_STREAM_TAG`] stream, tilt/spin
    /// from [`TILT_STREAM_TAG`], moons from [`MOON_STREAM_TAG`], so each
    /// lane moves independently. Returns empty output for an invalid parent
    /// rather than panicking.
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        if !parent.is_valid() {
            return Generated {
                points: Vec::new(),
                child_constraints: Vec::new(),
            };
        }
        use crate::seed::hash_triple;
        let mut tilt_stream = Rng::new(hash_triple(seed, TILT_STREAM_TAG, 0));
        let mut class_stream = Rng::new(hash_triple(seed, SURFACE_STREAM_TAG, u64::MAX));
        let flattening = if self.home {
            EARTH_FLATTENING
        } else {
            class_stream.next_f64() * 0.05
        };
        let tilt_deg = if self.home {
            EARTH_TILT_DEG
        } else {
            tilt_deg_for(&mut tilt_stream)
        };
        let spin_hours = if self.home {
            EARTH_SPIN_HOURS
        } else {
            spin_hours_for(&mut tilt_stream)
        };
        let radius_earth = if self.home {
            1.0
        } else if flattening < 0.01 {
            0.3
        } else if flattening < 0.025 {
            1.0
        } else {
            11.0
        };
        let air = air_for_radius_earth(radius_earth);
        let moon_target: u32 = if self.home {
            1
        } else {
            let mut moon_stream = Rng::new(hash_triple(seed, MOON_STREAM_TAG, 0));
            moon_count_for(&mut moon_stream, radius_earth)
        };
        let budget = parent.max_count;
        let moon_room = moon_target.min(budget / 2);
        let portal_room = REGION_PORTALS.min(budget.saturating_sub(moon_room));
        let scenery_room =
            SCENERY_POINTS.min(budget.saturating_sub(moon_room).saturating_sub(portal_room));
        let surface_cap = budget
            .saturating_sub(moon_room)
            .saturating_sub(portal_room)
            .saturating_sub(scenery_room) as usize;
        let mut points = Vec::new();
        'faces: for face in 0..6u8 {
            for iu in 0..SAMPLER_GRID {
                for iv in 0..SAMPLER_GRID {
                    if points.len() >= surface_cap {
                        break 'faces;
                    }
                    let u = (f64::from(iu) + 0.5) / f64::from(SAMPLER_GRID);
                    let v = (f64::from(iv) + 0.5) / f64::from(SAMPLER_GRID);
                    let body = BodyRecipe {
                        flattening,
                        tilt_deg,
                        spin_hours,
                        radius_earth,
                        air,
                    };
                    let info = surface_info(seed, face, u, v, &body);
                    let direction = sphere_point(face, u, v, flattening).unwrap_or([0.0, 1.0, 0.0]);
                    let radius = PLANET_RADIUS_CELL + (info.height - 0.5) * 2.0 * RELIEF_RANGE_CELL;
                    points.push(Point {
                        position: [
                            direction[0] * radius,
                            direction[1] * radius,
                            direction[2] * radius,
                        ],
                        radius: 0.01,
                        kind: MarkerKind::Population,
                        surface: Some(info),
                        ..Point::bare([0.0; 3], 0.01, MarkerKind::Population)
                    });
                }
            }
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
        // Region portals sit on the relief at distinct heights (#394): the
        // lowest, highest, and evenly spaced quantiles of a fine relief
        // scan, preferring distinct faces so regions spread across the
        // globe. Each portal lands on the relief shell with the outward
        // normal as its patch frame, so the region opens exactly there
        // with its ground tangent to the planet.
        {
            // Fine scan independent of the emitted surface budget, so full
            // portal rooms always resolve even when moons eat the surface
            // cap. Deterministic in the seed: no RNG, sorted by height.
            const PORTAL_GRID: u32 = 9;
            let mut scan: Vec<(u8, f64, [f64; 3])> = Vec::new();
            for face in 0..6u8 {
                for iu in 0..PORTAL_GRID {
                    for iv in 0..PORTAL_GRID {
                        // Face centres never host portals (#394): the region
                        // must sit on the relief, not on the old fixed spots.
                        if iu == PORTAL_GRID / 2 && iv == PORTAL_GRID / 2 {
                            continue;
                        }
                        let u = (f64::from(iu) + 0.5) / f64::from(PORTAL_GRID);
                        let v = (f64::from(iv) + 0.5) / f64::from(PORTAL_GRID);
                        let body = BodyRecipe {
                            flattening,
                            tilt_deg,
                            spin_hours,
                            radius_earth,
                            air,
                        };
                        let height = surface_info(seed, face, u, v, &body).height;
                        let direction =
                            sphere_point(face, u, v, flattening).unwrap_or([0.0, 1.0, 0.0]);
                        let radius = PLANET_RADIUS_CELL + (height - 0.5) * 2.0 * RELIEF_RANGE_CELL;
                        scan.push((
                            face,
                            height,
                            [
                                direction[0] * radius,
                                direction[1] * radius,
                                direction[2] * radius,
                            ],
                        ));
                    }
                }
            }
            scan.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            let total = scan.len();
            let mut picks: Vec<usize> = Vec::new();
            for slot in 0..portal_room as usize {
                let rank = if portal_room <= 1 {
                    0
                } else {
                    slot * total.saturating_sub(1) / (portal_room as usize - 1)
                };
                // Quantiles spread from abyss to peak; ranks are distinct
                // while the scan outnumbers the room, which it always does.
                if rank < total && !picks.contains(&rank) {
                    picks.push(rank);
                }
            }
            for rank in 0..total {
                if picks.len() >= portal_room as usize {
                    break;
                }
                if !picks.contains(&rank) {
                    picks.push(rank);
                }
            }
            // Prefer distinct faces: replace colliding picks with the
            // highest scan sample from each missing face.
            {
                let mut seen = [false; 6];
                let mut colliding: Vec<usize> = Vec::new();
                for (slot, &rank) in picks.iter().enumerate() {
                    let Some(sample) = scan.get(rank) else {
                        continue;
                    };
                    let face = sample.0;
                    let occupied = seen.get(face as usize).copied().unwrap_or(true);
                    if !occupied {
                        if let Some(seen_slot) = seen.get_mut(face as usize) {
                            *seen_slot = true;
                        }
                    } else {
                        colliding.push(slot);
                    }
                }
                if !colliding.is_empty() {
                    let mut best_per_face: [Option<(usize, f64)>; 6] =
                        [None, None, None, None, None, None];
                    for (rank, sample) in scan.iter().enumerate() {
                        let face_value = sample.0;
                        let height_value = sample.1;
                        let slot = usize::from(face_value);
                        if slot >= best_per_face.len() {
                            continue;
                        }
                        let Some(current) = best_per_face.get(slot) else {
                            continue;
                        };
                        let better = match current {
                            None => true,
                            Some((_, held)) => height_value > *held,
                        };
                        if better
                            && !picks.contains(&rank)
                            && let Some(entry) = best_per_face.get_mut(slot)
                        {
                            *entry = Some((rank, height_value));
                        }
                    }
                    for slot in colliding {
                        for face in 0..6u8 {
                            let occupied = seen.get(face as usize).copied().unwrap_or(true);
                            if occupied {
                                continue;
                            }
                            let index = usize::from(face);
                            let replacement = best_per_face.get(index).copied().flatten();
                            if let Some((rank, _)) = replacement {
                                if let Some(pick) = picks.get_mut(slot) {
                                    *pick = rank;
                                }
                                if let Some(seen_slot) = seen.get_mut(face as usize) {
                                    *seen_slot = true;
                                }
                                break;
                            }
                        }
                    }
                }
            }
            for &rank in &picks {
                if points.len() >= budget as usize {
                    break;
                }
                let Some(sample) = scan.get(rank) else {
                    continue;
                };
                let position = sample.2;
                let length = (position[0] * position[0]
                    + position[1] * position[1]
                    + position[2] * position[2])
                    .sqrt();
                let normal = if length.is_finite() && length > 0.0 {
                    [
                        position[0] / length,
                        position[1] / length,
                        position[2] / length,
                    ]
                } else {
                    [0.0, 1.0, 0.0]
                };
                points.push(Point {
                    position,
                    radius: 0.01,
                    kind: MarkerKind::Portal,
                    form: Form::Patch { normal },
                    ..Point::bare([0.0; 3], 0.01, MarkerKind::Portal)
                });
            }
        }
        // Equator ring and spin axis as scenery: the ring circles the belly
        // in the plane normal to the spin axis, the thread pierces both
        // poles. Both trim from the surface budget above, never from portals.
        let tilt = tilt_deg * PI / 180.0;
        let spin_axis = [tilt.sin(), tilt.cos(), 0.0];
        let scenery = [
            Point {
                position: [0.0, 0.0, 0.0],
                radius: 0.005,
                kind: MarkerKind::Population,
                form: Form::Ring {
                    normal: spin_axis,
                    inner: PLANET_RADIUS_CELL * 0.98,
                    outer: PLANET_RADIUS_CELL,
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
        assert!(first.contains("points=24"));
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
        assert_eq!(out.points.len(), 24, "the Moon takes one of 24 slots");
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
                assert_eq!(surface.flattening, EARTH_FLATTENING);
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
            assert_eq!(out.points.len(), 24, "surface fills what moons leave");
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
    fn sphere_edges_agree_across_faces() {
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let from_plus_x = sphere_point(0, 1.0, t, EARTH_FLATTENING);
            let from_plus_y = sphere_point(2, 1.0, t, EARTH_FLATTENING);
            assert_eq!(from_plus_x, from_plus_y, "edge mismatch at t={t}");
        }
        let corner = [
            sphere_point(0, 1.0, 1.0, EARTH_FLATTENING),
            sphere_point(2, 1.0, 1.0, EARTH_FLATTENING),
            sphere_point(4, 1.0, 1.0, EARTH_FLATTENING),
        ];
        assert_eq!(corner[0], corner[1]);
        assert_eq!(corner[1], corner[2]);
        assert_eq!(sphere_point(6, 0.5, 0.5, EARTH_FLATTENING), None);
    }

    #[test]
    fn face_uv_round_trips_grid_samples() {
        // Interior samples only: edges and corners are shared seams where any
        // adjacent face is a correct answer.
        for face in 0..6u8 {
            for (u, v) in [(0.25, 0.75), (0.5, 0.5), (0.75, 0.25)] {
                let direction = sphere_point(face, u, v, 0.0).expect("valid face");
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
    fn sphere_squashes_only_the_polar_axis() {
        let round = sphere_point(4, 0.75, 0.75, 0.0).expect("valid face");
        let length = (round[0] * round[0] + round[1] * round[1] + round[2] * round[2]).sqrt();
        assert!(
            (length - 1.0).abs() <= 1e-12,
            "unsquashed must be unit: {length}"
        );
        let flat = sphere_point(4, 0.75, 0.75, EARTH_FLATTENING).expect("valid face");
        assert_eq!(flat[0], round[0]);
        assert_eq!(flat[2], round[2]);
        assert!(
            flat[1] < round[1],
            "polar axis must squash: {} vs {}",
            flat[1],
            round[1]
        );
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
            flattening: EARTH_FLATTENING,
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
}

#[cfg(test)]
mod clearance_tests {
    use super::*;
    use crate::coords::Level;

    fn level(n: u8) -> Level {
        Level::new(n).expect("ladder level")
    }

    #[test]
    fn relief_radius_tracks_the_relief_range() {
        let seed = 42u64;
        for direction in [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.3, -0.5, 0.8],
        ] {
            let radius = relief_radius(seed, direction);
            assert!(
                (PLANET_RADIUS_CELL - RELIEF_RANGE_CELL..=PLANET_RADIUS_CELL + RELIEF_RANGE_CELL)
                    .contains(&radius),
                "relief radius {radius} out of range"
            );
        }
        assert_eq!(relief_radius(seed, [0.0; 3]), relief_radius(seed, [1.0, 0.0, 0.0]));
        assert_eq!(
            relief_radius(seed, [1.0, 0.0, 0.0]),
            relief_radius(seed, [1.0, 0.0, 0.0])
        );
    }

    #[test]
    fn l10_clamp_pushes_out_and_leaves_height_alone() {
        let seed = 42u64;
        let l10 = level(10);
        let buried = [0.05, 0.02, -0.03];
        let out = clamp_above_ground(l10, seed, buried);
        let distance = (out[0] * out[0] + out[1] * out[1] + out[2] * out[2]).sqrt();
        assert!(distance >= relief_radius(seed, buried) + GROUND_STANDOFF - 1e-12);
        // Radial push: direction unchanged.
        let pushed = distance / (buried[0] * buried[0] + buried[1] * buried[1] + buried[2] * buried[2]).sqrt();
        for (got, want) in out.iter().zip([buried[0] * pushed, buried[1] * pushed, buried[2] * pushed]) {
            assert!((got - want).abs() < 1e-12);
        }
        let centre = clamp_above_ground(l10, seed, [0.0; 3]);
        assert_eq!(centre[0], 0.0);
        assert!(centre[1] >= PLANET_RADIUS_CELL - RELIEF_RANGE_CELL + GROUND_STANDOFF);
        let high = [0.0, 0.0, 3.58];
        assert_eq!(clamp_above_ground(l10, seed, high), high);
    }

    #[test]
    fn tail_clamp_raises_to_the_bowl_plus_standoff() {
        let l11 = level(11);
        let sag = ground_height(l11, 0.4, 0.0);
        assert!(sag < 0.0, "the bowl sags below zero");
        let under = [0.4, sag - 0.5, 0.0];
        let out = clamp_above_ground(l11, 42, under);
        assert_eq!(out[0], under[0]);
        assert_eq!(out[2], under[2]);
        assert!((out[1] - (sag + GROUND_STANDOFF)).abs() < 1e-12);
        let above = [0.4, sag + 1.0, 0.0];
        assert_eq!(clamp_above_ground(l11, 42, above), above);
        assert_eq!(ground_height(level(10), 0.4, 0.0), 0.0);
        assert_eq!(ground_height(level(14), 0.4, 0.0), 0.0);
    }

    #[test]
    fn clamp_passes_through_elsewhere_and_on_bad_input() {
        let bad = [f64::NAN, 0.0, 0.0];
        let out = clamp_above_ground(level(10), 42, bad);
        assert!(out[0].is_nan() && out[1] == 0.0 && out[2] == 0.0);
        let deep = [0.0, -5.0, 0.0];
        assert_eq!(clamp_above_ground(level(5), 42, deep), deep);
        assert_eq!(clamp_above_ground(level(14), 42, deep), deep);
    }

    #[test]
    fn standoff_sits_far_below_the_landing_rest() {
        // Landings end ~3.58 child units above a portal; the standoff must
        // never touch them. In L10 parent units the rest is the marker
        // radius over sin(OPEN_ANGLE), an order above the standoff.
        assert!(GROUND_STANDOFF < 0.1);
        let l10 = level(10);
        let rest_parent = 0.5 * crate::nest::child_ratio(l10).expect("L10 ratio")
            / crate::nest::OPEN_ANGLE.sin();
        assert!(rest_parent > GROUND_STANDOFF * 2.0);
    }
}
