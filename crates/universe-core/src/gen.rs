//! Pure procedural generation contracts.
//!
//! The ladder rule `(contents, child_constraints) = G(seed, parent)`
//! (`docs/universe/ladder.md`) becomes the [`Generator`](crate::gen::Generator) trait: a pure
//! function of a `u64` seed plus the parent [`Constraints`](crate::gen::Constraints), returning
//! [`Generated`](crate::gen::Generated) content. [`respects`](crate::gen::respects) checks that a child budget fits inside
//! its parent budget. [`UniformGenerator`](crate::gen::UniformGenerator) is the reference implementation
//! used by the headless checks; content levels (M3+) provide their own.

use crate::coords::HALF_BOUND;
use crate::seed::Rng;

/// Budget passed from a parent cell to its children (`K` in the notion).
///
/// Plain data: a density multiplier plus allowed count and extent ranges.
/// Units are per-cell (positions live in `[-HALF_BOUND, HALF_BOUND)`), so the
/// same struct describes every level without rescaling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraints {
    /// Fraction of the baseline content to emit, in `[0.0, 1.0]`.
    pub density_multiplier: f64,
    /// Fewest points a generation of this cell may emit.
    pub min_count: u32,
    /// Most points a generation of this cell may emit.
    pub max_count: u32,
    /// Half-extent of the allowed region per axis, each in `(0.0, 0.5]`.
    pub allowed_extent: [f64; 3],
}

impl Constraints {
    /// Builds constraints, returning `None` when any field is out of range
    /// (density outside `[0.0, 1.0]`, `min_count > max_count`, an extent
    /// outside `(0.0, 0.5]`, or any non-finite float).
    #[must_use]
    pub fn new(
        density_multiplier: f64,
        min_count: u32,
        max_count: u32,
        allowed_extent: [f64; 3],
    ) -> Option<Constraints> {
        let candidate = Constraints {
            density_multiplier,
            min_count,
            max_count,
            allowed_extent,
        };
        if candidate.is_valid() {
            Some(candidate)
        } else {
            None
        }
    }

    /// Returns `true` when every field is in its documented range.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.density_multiplier.is_finite()
            && (0.0..=1.0).contains(&self.density_multiplier)
            && self.min_count <= self.max_count
            && self
                .allowed_extent
                .iter()
                .all(|extent| extent.is_finite() && *extent > 0.0 && *extent <= HALF_BOUND)
    }
}

/// What a generated point means for navigation (portal/population split, #151).
///
/// A portal opens the next cell when targeted; a population point is shown
/// and counted but never opens, highlights, or takes the target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MarkerKind {
    /// Opens the next cell when targeted (the historical marker behavior).
    Portal,
    /// Shown and counted; never opens, highlights, or takes the target.
    Population,
}

/// Morphological tag of one L4 galaxy, carried as data (#154).
///
/// The sampler (`astro`) owns the full shape; this tag is the contract-level
/// summary the dive, snapshots, and (later, #157) rendering share.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GalaxyKind {
    /// Disk with arms and usually a bar; hosts the spiral layout.
    Spiral,
    /// Spheroidal system; stars follow a flattened ball.
    Elliptical,
    /// Clumpy system without regular symmetry.
    Irregular,
}

/// Per-galaxy shape data connected to the dive (#154, data only).
///
/// `None` on every `Point` outside L4: counts, kinds, and portal order are
/// untouched, and rendering ignores these fields until #157. Snapshots print
/// them as trailing tokens (kind tag, orientation at fixed precision,
/// `bar`/`nobar`, `far`/`near`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GalaxyInfo {
    /// Morphological class.
    pub kind: GalaxyKind,
    /// Unit normal of the disk plane (arbitrary for ellipticals).
    pub orientation: [f64; 3],
    /// Central bar in the spiral layout (forced on for the Milky Way).
    pub barred: bool,
    /// Simplified impostor record past the far-view gate (#154 T3).
    pub far_view: bool,
}

/// Spectral class of one star, hot blue giants to cool red dwarfs.
///
/// The sampler (`system`) owns the O-M mix; this tag is the contract-level
/// summary the dive, snapshots, and (later, #157) rendering share.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StarKind {
    /// Over 30,000 K blue-violet giants; rarest of all.
    O,
    /// Blue-white giants; a tenth of a percent of stars.
    B,
    /// White stars; about half a percent.
    A,
    /// Yellow-white stars; about three percent.
    F,
    /// Yellow dwarfs like the Sun; about eight percent.
    G,
    /// Orange dwarfs; about twelve percent.
    K,
    /// Red dwarfs; three in four stars.
    M,
}

/// Per-star data connected to the dive (#155, data only).
///
/// `Some` on every L6 system, L7/L8 central star, and L9 companion point;
/// `None` elsewhere. Counts, kinds, and portal order are untouched, and
/// rendering ignores these fields until #157. Snapshots print them as
/// trailing tokens (class letter, mass at fixed precision).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StarInfo {
    /// Spectral class from the observed O-M mix.
    pub kind: StarKind,
    /// Stellar mass in solar masses, inside the class band.
    pub mass_solar: f64,
}

/// Per-planet data connected to the dive (#155, data only).
///
/// `Some` on every L8/L9 planet point; `None` elsewhere. Orbits are carried
/// as data (positions now, motion later); rendering ignores these fields
/// until #157. Snapshots print them as trailing tokens (`P`, radius in
/// Earth radii, orbit in AU, period in days, all at fixed precision).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanetInfo {
    /// Planetary radius in Earth radii (the 1.5-2 valley stays empty).
    pub radius_earth: f64,
    /// Semi-major axis in AU from the Solar-template spacing.
    pub orbit_au: f64,
    /// Orbital period in days from Kepler's law for the host mass.
    pub period_days: f64,
}

/// Per-cloud data connected to the dive (#155, data only).
///
/// `Some` on every L5 cloud point; `None` elsewhere. Sizes stay sampled;
/// the mass spectrum rides here for #157. Snapshots print it as a trailing
/// token (`C`, mass in solar masses at fixed precision).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CloudInfo {
    /// Cloud mass in solar masses (10 to 10 million).
    pub mass_solar: f64,
}

/// Air profile of one L10 planet (#156).
///
/// The sampler owns the profile choice; this tag is the contract-level
/// summary the dive, snapshots, and rendering share.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AirKind {
    /// No appreciable atmosphere (faint rim only).
    None,
    /// Thin envelope (faint rim).
    Thin,
    /// Earth-like nitrogen-oxygen envelope (full scattering treatment).
    Earth,
}

/// Per-surface-sample data on L10 points (#156).
///
/// `Some` on every L10 surface point; `None` elsewhere. Cell-uniform values
/// (flattening, sea level, tilt, spin, air) repeat on each point the way
/// `GalaxyInfo.barred` repeats on L4 points. Snapshots print them as
/// trailing tokens (`T`, height, biome letter, flattening, tilt, spin, air
/// letter, all at fixed precision).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceInfo {
    /// Height sample in `[0.0, 1.0]` (`0.0` abyss, `1.0` peak).
    pub height: f64,
    /// Climate tag from heat plus wetness.
    pub biome: crate::terrain::Biome,
    /// Oblate flattening (Earth `1/298`).
    pub flattening: f64,
    /// Sea level in height units (Earth-like `0.42`).
    pub sea_level: f64,
    /// Axial tilt in degrees, `0.0..=177.0`.
    pub tilt_deg: f64,
    /// Day length in hours.
    pub spin_hours: f64,
    /// Atmosphere profile driving the rim treatment.
    pub air: AirKind,
}

/// Per-moon data on L10 moon population points (#156).
///
/// `Some` on moon points appended after the surface samples; `None`
/// elsewhere. Moons are shown and counted but never open. Snapshots print
/// them as trailing tokens (`N`, radius in km, orbit in km, period in days,
/// all at fixed precision).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoonInfo {
    /// Moon radius in km (Moon `1,738`).
    pub radius_km: f64,
    /// Orbit radius in km (Moon `384,400`).
    pub orbit_km: f64,
    /// Orbital period in days (Moon `27.3`).
    pub period_days: f64,
}

/// Per-object display color derived from the point's kind data (#157).
///
/// `Some` on star points (from the O-M blackbody table, white Sun) and
/// galaxy points (from kind plus density); `None` elsewhere. Planets keep
/// their biome colors, so surface and moon points never carry this field.
/// Pure data: the samplers own the tables, rendering maps these values to
/// emissive output. Snapshots print them as trailing tokens (`V`, red,
/// green, blue, brightness, all at fixed precision).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorInfo {
    /// Linear display red in `[0.0, 1.0]`.
    pub red: f64,
    /// Linear display green in `[0.0, 1.0]`.
    pub green: f64,
    /// Linear display blue in `[0.0, 1.0]`.
    pub blue: f64,
    /// Relative emissive strength (`1.0` is the Sun).
    pub brightness: f64,
}

/// One generated indicator point: a position, a marker size, and its kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// Cell-local position in `[-HALF_BOUND, HALF_BOUND)` per axis.
    pub position: [f64; 3],
    /// Marker radius in cell units; always positive.
    pub radius: f64,
    /// Portal (opens deeper) or population (shown only).
    pub kind: MarkerKind,
    /// Per-galaxy shape data; `Some` only for L4 galaxy markers (#154).
    pub galaxy: Option<GalaxyInfo>,
    /// Per-star data; `Some` for L6 systems, L7/L8 stars, L9 companions (#155).
    pub star: Option<StarInfo>,
    /// Per-planet data; `Some` for L8/L9 planet markers (#155).
    pub planet: Option<PlanetInfo>,
    /// Per-cloud data; `Some` only for L5 cloud markers (#155).
    pub cloud: Option<CloudInfo>,
    /// Per-surface-sample data; `Some` only for L10 surface points (#156).
    pub surface: Option<SurfaceInfo>,
    /// Per-moon data; `Some` only for L10 moon points (#156).
    pub moon: Option<MoonInfo>,
    /// Per-object display color; `Some` for star and galaxy points (#157).
    pub tint: Option<ColorInfo>,
}

/// Output of generating one cell: indicator points plus child budgets.
///
/// `child_constraints[i]` is the budget for octant `i` (bit `4*x + 2*y + z`
/// of the octant `[u8; 3]`); [`respects`] must hold for each against the
/// parent constraints the cell was generated with.
#[derive(Clone, Debug, PartialEq)]
pub struct Generated {
    /// Indicator points (grids, vectors, points) inside this cell.
    pub points: Vec<Point>,
    /// One budget per child octant, index `4*x + 2*y + z`.
    pub child_constraints: Vec<Constraints>,
}

/// Pure generator contract `G(seed, parent) -> Generated`.
///
/// Implementations must be pure functions of their arguments: no global or
/// interior-mutable state, no I/O, no clock reads. The same `(seed, parent)`
/// must yield byte-identical output on every call and every platform (see
/// [`crate::verify::check_determinism`]). When `parent` is invalid, return an
/// empty [`Generated`] rather than panicking.
pub trait Generator {
    /// Generates the cell contents for `seed` under the `parent` budget.
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated;
}

/// Returns `true` when `child` fits inside the budget `parent` allows.
///
/// The rule: both sides valid, the child density does not exceed the parent
/// density, and the child count ceiling does not exceed the parent ceiling.
/// (Extents are per-cell units, so they are range-checked by validity rather
/// than compared across levels.)
#[must_use]
pub fn respects(child: &Constraints, parent: &Constraints) -> bool {
    child.is_valid()
        && parent.is_valid()
        && child.density_multiplier <= parent.density_multiplier
        && child.max_count <= parent.max_count
}

/// Reference generator scattering uniform indicator points.
///
/// Emits `round(density * base_count)` points clamped to `[min, max]`, with
/// positions uniform in `[-extent, extent]` per axis drawn from an [`Rng`]
/// seeded by the cell seed. Each of the 8 child octants receives half the
/// parent density and half the count ceiling, so [`respects`] holds for every
/// child against the parent the cell was generated with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UniformGenerator {
    /// Baseline point count at full density.
    pub base_count: u32,
}

impl UniformGenerator {
    /// Creates a uniform generator emitting up to `base_count` points.
    #[must_use]
    pub const fn new(base_count: u32) -> UniformGenerator {
        UniformGenerator { base_count }
    }
}

impl Generator for UniformGenerator {
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        if !parent.is_valid() {
            return Generated {
                points: Vec::new(),
                child_constraints: Vec::new(),
            };
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: wanted count saturates like `as`; clamped below"
        )]
        #[expect(
            clippy::cast_sign_loss,
            reason = "E-CAST: wanted count saturates like `as`; clamped below"
        )]
        let wanted = (f64::from(self.base_count) * parent.density_multiplier).round() as u32;
        let count = wanted.clamp(parent.min_count, parent.max_count);
        let mut rng = Rng::new(seed);
        let mut points = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let mut position = [0.0; 3];
            for (slot, extent) in position.iter_mut().zip(parent.allowed_extent.iter()) {
                *slot = (rng.next_f64() * 2.0 - 1.0) * extent;
            }
            points.push(Point {
                position,
                radius: 0.01,
                kind: MarkerKind::Portal,
                galaxy: None,
                star: None,
                planet: None,
                cloud: None,
                surface: None,
                moon: None,
                tint: None,
            });
        }
        let child_density = parent.density_multiplier / 2.0;
        let child_max = parent.max_count / 2;
        let child_min = parent.min_count.min(child_max);
        // SAFETY: halving keeps every field in range when the parent is valid.
        let child = Constraints {
            density_multiplier: child_density,
            min_count: child_min,
            max_count: child_max,
            allowed_extent: parent.allowed_extent,
        };
        Generated {
            points,
            child_constraints: vec![child; 8],
        }
    }
}

/// Fixed generator for the L1 root cell: one portal per octant (#151).
///
/// Eight portals at the octant centers (`±0.25` per axis), so the universe
/// cell subdivides space-fillingly with no sampling at all. Deterministic
/// by construction; children mirror the reference generator's halved
/// budgets so [`respects`] holds for every child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OctantGenerator;

impl Generator for OctantGenerator {
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        let _ = seed;
        if !parent.is_valid() {
            return Generated {
                points: Vec::new(),
                child_constraints: Vec::new(),
            };
        }
        let mut points = Vec::with_capacity(8);
        for x in [-0.25, 0.25] {
            for y in [-0.25, 0.25] {
                for z in [-0.25, 0.25] {
                    points.push(Point {
                        position: [x, y, z],
                        radius: 0.01,
                        kind: MarkerKind::Portal,
                        galaxy: None,
                        star: None,
                        planet: None,
                        cloud: None,
                        surface: None,
                        moon: None,
                        tint: None,
                    });
                }
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
mod tests {
    use super::*;

    /// Builds typical mid-ladder constraints for tests.
    fn parent_constraints() -> Constraints {
        Constraints::new(1.0, 2, 64, [0.5, 0.5, 0.5]).expect("valid test constraints")
    }

    #[test]
    fn same_seed_generates_identical_output() {
        let generator = UniformGenerator::new(32);
        let parent = parent_constraints();
        assert_eq!(
            generator.generate(7, &parent),
            generator.generate(7, &parent)
        );
    }

    #[test]
    fn generation_order_does_not_matter() {
        let generator = UniformGenerator::new(32);
        let parent = parent_constraints();
        let seeds = [1u64, 2, 3, 99, 1024];
        let forward: Vec<_> = seeds
            .iter()
            .map(|s| generator.generate(*s, &parent))
            .collect();
        let backward: Vec<_> = seeds
            .iter()
            .rev()
            .map(|s| generator.generate(*s, &parent))
            .collect();
        let mut backward_sorted = backward.clone();
        backward_sorted.reverse();
        assert_eq!(forward, backward_sorted);
        // Distinct seeds give distinct content.
        assert_ne!(forward[0], forward[1]);
    }

    #[test]
    fn octants_cover_all_eight_cells_as_portals() {
        let out = OctantGenerator.generate(12345, &parent_constraints());
        assert_eq!(out.points.len(), 8);
        for point in &out.points {
            assert_eq!(point.kind, MarkerKind::Portal);
            for axis in 0..3 {
                assert!(
                    point.position[axis] == -0.25 || point.position[axis] == 0.25,
                    "off-center octant point: {:?}",
                    point.position
                );
            }
        }
        let mut sorted: Vec<[f64; 3]> = out.points.iter().map(|p| p.position).collect();
        sorted.sort_by(|a, b| {
            a[0].total_cmp(&b[0])
                .then(a[1].total_cmp(&b[1]))
                .then(a[2].total_cmp(&b[2]))
        });
        sorted.dedup();
        assert_eq!(sorted.len(), 8, "octant centers must be distinct");
        for child in &out.child_constraints {
            assert!(respects(child, &parent_constraints()));
        }
    }

    #[test]
    fn respects_accepts_tighter_and_rejects_looser_children() {
        let parent = parent_constraints();
        let tighter = Constraints::new(0.5, 0, 32, [0.25; 3]).expect("valid");
        assert!(respects(&tighter, &parent));
        // Denser than the parent: rejected.
        let denser = Constraints {
            density_multiplier: 1.5,
            ..tighter
        };
        assert!(!respects(&denser, &parent));
        // Higher ceiling than the parent: rejected (bypasses `new` on purpose).
        let hungrier = Constraints {
            density_multiplier: 0.5,
            min_count: 0,
            max_count: parent.max_count + 1,
            allowed_extent: [0.25; 3],
        };
        assert!(!respects(&hungrier, &parent));
        assert!(!respects(&parent, &tighter));
    }

    #[test]
    fn uniform_output_honors_parent_ranges() {
        let generator = UniformGenerator::new(32);
        let parent = parent_constraints();
        let out = generator.generate(11, &parent);
        assert!(
            (parent.min_count as usize..=parent.max_count as usize).contains(&out.points.len())
        );
        for point in &out.points {
            assert!(point.radius > 0.0);
            for (axis, extent) in parent.allowed_extent.iter().enumerate() {
                assert!(
                    point.position[axis].abs() <= *extent,
                    "point outside allowed extent"
                );
            }
        }
        assert_eq!(out.child_constraints.len(), 8);
        for child in &out.child_constraints {
            assert!(respects(child, &parent));
        }
    }

    #[test]
    fn invalid_parent_yields_empty_output() {
        let generator = UniformGenerator::new(32);
        let bad = Constraints {
            density_multiplier: 2.0,
            min_count: 5,
            max_count: 2,
            allowed_extent: [0.5; 3],
        };
        assert!(!bad.is_valid());
        assert_eq!(generator.generate(3, &bad).points.len(), 0);
        assert_eq!(Constraints::new(2.0, 0, 1, [0.5; 3]), None);
        assert_eq!(Constraints::new(0.5, 9, 1, [0.5; 3]), None);
        assert_eq!(Constraints::new(0.5, 0, 1, [0.0; 3]), None);
    }
}
