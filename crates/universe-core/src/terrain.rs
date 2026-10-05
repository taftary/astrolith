//! L10 cube-sphere terrain with level of detail (M5, sub-issue #42).
//!
//! Planets carry a heightmap on a cube-sphere: each of the six cube faces
//! samples the SAME 3D fractal field through [`height_at`](crate::terrain::height_at), so a shared edge
//! or corner maps to identical 3D points from every adjacent face and borders
//! agree by construction. [`biome_for`](crate::terrain::biome_for) tags a sample from its height and
//! latitude, and [`lod_for`](crate::terrain::lod_for) selects the subdivision depth from the camera
//! distance (monotonic: nearer cameras never select coarser detail).

use crate::r#gen::{
    AirKind, Constraints, Generated, Generator, MarkerKind, MoonInfo, Point, SurfaceInfo,
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

/// Deepest terrain subdivision (camera at or inside the surface).
pub const MAX_LOD: u8 = 8;

/// Reference camera distance, in meters, holding [`MAX_LOD`].
///
/// Each doubling of distance above this drops one LOD rung.
pub const LOD_REFERENCE_DISTANCE: f64 = 2_000.0;

/// Surface biome tag from height and latitude (see [`biome_for`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Biome {
    /// Deep water.
    Ocean,
    /// Shallow shelf at the waterline.
    Coast,
    /// Hot arid lowland.
    Desert,
    /// Temperate grassland.
    Grassland,
    /// Humid woodland.
    Forest,
    /// Cold treeless high latitude.
    Tundra,
    /// Polar ice.
    IceCap,
    /// High peaks.
    Mountain,
}

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

/// Tags a surface sample with a biome from height and latitude.
///
/// `height` is the [`height_at`] value clamped to `[0.0, 1.0]`; `latitude`
/// is radians clamped to `[-PI/2, PI/2]` (positive is north). Non-finite
/// heights read as shoreline, non-finite latitudes as the equator.
///
/// | Height | Latitude | Biome |
/// |---|---|---|
/// | `< 0.40` | any | [`Biome::Ocean`] |
/// | `< 0.45` | any | [`Biome::Coast`] |
/// | `>= 0.45` | `\|lat\| > 1.15` | [`Biome::IceCap`] |
/// | `> 0.80` | cooler than ice | [`Biome::Mountain`] |
/// | `>= 0.45` | `\|lat\| > 0.90` | [`Biome::Tundra`] |
/// | `< 0.55` | `\|lat\| < 0.35` | [`Biome::Desert`] |
/// | `< 0.62` | milder | [`Biome::Grassland`] |
/// | otherwise | otherwise | [`Biome::Forest`] |
#[must_use]
pub fn biome_for(height: f64, latitude: f64) -> Biome {
    let clamped_height = if height.is_finite() {
        height.clamp(0.0, 1.0)
    } else {
        0.42
    };
    let clamped_latitude = if latitude.is_finite() {
        latitude.clamp(-PI / 2.0, PI / 2.0)
    } else {
        0.0
    };
    let polar = clamped_latitude.abs();
    if clamped_height < 0.40 {
        Biome::Ocean
    } else if clamped_height < 0.45 {
        Biome::Coast
    } else if polar > 1.15 {
        Biome::IceCap
    } else if clamped_height > 0.80 {
        Biome::Mountain
    } else if polar > 0.90 {
        Biome::Tundra
    } else if clamped_height < 0.55 && polar < 0.35 {
        Biome::Desert
    } else if clamped_height < 0.62 {
        Biome::Grassland
    } else {
        Biome::Forest
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

/// Sea level in height units: samples below read as ocean.
pub const SEA_LEVEL: f64 = 0.42;

/// Share of Earth-like surface samples falling below [`SEA_LEVEL`].
///
/// The 71 percent ocean target: Earth-like draws land in the ocean band
/// with this probability and in the land band otherwise.
pub const OCEAN_FRACTION_EARTH: f64 = 0.71;

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

/// Cell-uniform planet recipe shared by every surface sample of one cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyRecipe {
    /// Bimodal Earth-like seas when true, single-peak field otherwise.
    pub earth_like: bool,
    /// Oblate flattening (Earth `1/298`).
    pub flattening: f64,
    /// Axial tilt in degrees, `0.0..=177.0`.
    pub tilt_deg: f64,
    /// Day length in hours.
    pub spin_hours: f64,
    /// Atmosphere profile driving the rim treatment.
    pub air: AirKind,
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
/// Earth-like draws land below [`SEA_LEVEL`] with probability
/// [`OCEAN_FRACTION_EARTH`] (the bimodal recipe: the shared fBm field
/// remaps into the ocean or the land band, so shores stay continuous);
/// other bodies read the single-peak field straight through `height_at`.
#[must_use]
pub fn surface_height(rng: &mut Rng, seed: u64, face: u8, u: f64, v: f64, earth_like: bool) -> f64 {
    let field = height_at(seed, face, u, v);
    if !earth_like {
        return field;
    }
    if rng.next_f64() < OCEAN_FRACTION_EARTH {
        field * SEA_LEVEL
    } else {
        (SEA_LEVEL + 0.03) + field * (1.0 - (SEA_LEVEL + 0.03))
    }
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
/// Height comes from [`surface_height`], the climate tag from [`biome_for`]
/// at the sample latitude (from the spheroid direction), and the cell recipe
/// (flattening, sea level, tilt, spin, air) rides along for draw and proof.
#[must_use]
pub fn surface_info(
    rng: &mut Rng,
    seed: u64,
    face: u8,
    u: f64,
    v: f64,
    body: &BodyRecipe,
) -> SurfaceInfo {
    let height = surface_height(rng, seed, face, u, v, body.earth_like);
    let direction = sphere_point(face, u, v, body.flattening).unwrap_or([0.0, 1.0, 0.0]);
    let latitude = direction[1].clamp(-1.0, 1.0).asin();
    SurfaceInfo {
        height,
        biome: biome_for(height, latitude),
        flattening: body.flattening,
        sea_level: SEA_LEVEL,
        tilt_deg: body.tilt_deg,
        spin_hours: body.spin_hours,
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
    fn biome_thresholds_match_documented_table() {
        assert_eq!(biome_for(0.20, 0.0), Biome::Ocean);
        assert_eq!(biome_for(0.43, 0.0), Biome::Coast);
        assert_eq!(biome_for(0.90, 0.0), Biome::Mountain);
        assert_eq!(biome_for(0.60, 1.30), Biome::IceCap);
        assert_eq!(biome_for(0.60, 1.00), Biome::Tundra);
        assert_eq!(biome_for(0.50, 0.00), Biome::Desert);
        assert_eq!(biome_for(0.60, 0.50), Biome::Grassland);
        assert_eq!(biome_for(0.70, 0.20), Biome::Forest);
        // Degenerate inputs degrade gracefully, never panic.
        assert_eq!(biome_for(f64::NAN, 0.0), Biome::Coast);
        assert_eq!(biome_for(0.60, f64::NAN), Biome::Grassland);
        assert_eq!(biome_for(5.0, 0.0), Biome::Mountain);
        assert_eq!(biome_for(-3.0, 0.0), Biome::Ocean);
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

/// Generator adapter sampling planet surface and moon points (M5 demo/`--verify`).
///
/// Surface samples ride the oblate spheroid (`sphere_point` at
/// [`PLANET_RADIUS_CELL`] plus relief to [`RELIEF_RANGE_CELL`]); moons orbit
/// compressed into the outer shell band (display compression, the ladder
/// precedent: true moon orbits span dozens of planet radii and never fit one
/// cell). Moons take at most half the budget so the surface always resolves.
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
        let earth_like = self.home || class_stream.next_f64() < 0.25;
        let flattening = if earth_like {
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
        let air = if earth_like {
            AirKind::Earth
        } else if class_stream.next_f64() < 0.5 {
            AirKind::Thin
        } else {
            AirKind::None
        };
        let giant = !earth_like && flattening >= 0.025;
        let moon_target: u32 = if self.home {
            1
        } else {
            let radius_earth = if giant { 11.0 } else { 1.0 };
            let mut moon_stream = Rng::new(hash_triple(seed, MOON_STREAM_TAG, 0));
            moon_count_for(&mut moon_stream, radius_earth)
        };
        let budget = parent.max_count;
        let moon_room = moon_target.min(budget / 2);
        let surface_cap = budget.saturating_sub(moon_room) as usize;
        let mut points = Vec::new();
        let mut surface_index: u64 = 0;
        'faces: for face in 0..6u8 {
            for iu in 0..SAMPLER_GRID {
                for iv in 0..SAMPLER_GRID {
                    if points.len() >= surface_cap {
                        break 'faces;
                    }
                    let u = (f64::from(iu) + 0.5) / f64::from(SAMPLER_GRID);
                    let v = (f64::from(iv) + 0.5) / f64::from(SAMPLER_GRID);
                    let mut stream = Rng::new(hash_triple(seed, SURFACE_STREAM_TAG, surface_index));
                    surface_index += 1;
                    let body = BodyRecipe {
                        earth_like,
                        flattening,
                        tilt_deg,
                        spin_hours,
                        air,
                    };
                    let info = surface_info(&mut stream, seed, face, u, v, &body);
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
                        galaxy: None,
                        star: None,
                        planet: None,
                        cloud: None,
                        surface: Some(info),
                        moon: None,
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
                galaxy: None,
                star: None,
                planet: None,
                cloud: None,
                surface: None,
                moon: Some(moon_info(radius_km, orbit_km, period_days)),
            });
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
    fn home_sampler_holds_earth_and_one_moon() {
        use crate::r#gen::AirKind;
        let out = TerrainSampler::home().generate(7, &demo_budget());
        assert_eq!(out.points.len(), 24, "the Moon takes one of 24 slots");
        let moons = out
            .points
            .iter()
            .filter(|point| point.moon.is_some())
            .count();
        assert_eq!(moons, 1);
        for point in &out.points {
            assert_eq!(point.kind, MarkerKind::Population, "L10 stays terminal");
            if let Some(surface) = point.surface {
                assert_eq!(surface.air, AirKind::Earth);
                assert_eq!(surface.tilt_deg, EARTH_TILT_DEG);
            }
        }
    }

    #[test]
    fn procedural_moons_fit_the_shell_and_budget() {
        let parent = Constraints::new(1.0, 8, 24, [0.5, 0.5, 0.5]).expect("valid");
        for seed in [1u64, 7, 42, 999, 1 << 33] {
            let out = TerrainSampler::new().generate(seed, &parent);
            assert_eq!(out.points.len(), 24, "surface fills what moons leave");
            for point in &out.points {
                assert_eq!(point.kind, MarkerKind::Population);
                for axis in point.position {
                    assert!((-0.5..0.5).contains(&axis), "out of cell: {axis}");
                }
                if point.moon.is_some() {
                    assert!(point.surface.is_none(), "a point is either shore or moon");
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
    fn earth_like_seas_cover_about_seven_tenths() {
        let mut ocean = 0u32;
        let mut stream = Rng::new(0x0CEA_1560_0CEA_1560);
        for face in 0..6u8 {
            for k in 0..400u32 {
                let t = f64::from(k) / 400.0;
                let height = surface_height(&mut stream, 99, face, t, 1.0 - t, true);
                assert!((0.0..=1.0).contains(&height), "out of range: {height}");
                if height < SEA_LEVEL {
                    ocean += 1;
                }
            }
        }
        let fraction = f64::from(ocean) / 2400.0;
        assert!(
            (0.65..=0.77).contains(&fraction),
            "ocean fraction must read ~71 percent: {fraction}"
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
        let build = |seed_stream: u64, u: f64| {
            let mut stream = Rng::new(seed_stream);
            let body = BodyRecipe {
                earth_like: true,
                flattening: EARTH_FLATTENING,
                tilt_deg: EARTH_TILT_DEG,
                spin_hours: EARTH_SPIN_HOURS,
                air: AirKind::Earth,
            };
            surface_info(&mut stream, 7, 4, u, 0.75, &body)
        };
        let first = build(1234, 0.25);
        let second = build(1234, 0.25);
        assert_eq!(first, second);
        assert!((0.0..=1.0).contains(&first.height));
        assert_ne!(build(1234, 0.25), build(1234, 0.75));
    }
}
