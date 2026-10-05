//! L1-L3 density field and cluster placement (M3, sub-issue #40;
//! spectrum-shaped Gaussian field with typed environments, #153).
//!
//! Levels L1-L3 describe the cosmic web as one continuous density field: the
//! same [`density_at`](crate::density::density_at) spectrum-weighted Gaussian
//! field (lognormal-mapped) is sampled by every cell, so two
//! neighbouring cells evaluating their shared face in global coordinates
//! read bit-identical densities and borders agree by construction.
//! [`clusters_in_cell`](crate::density::clusters_in_cell) turns the field into indicator [`Generated`](crate::gen::Generated) content:
//! each candidate is sampled once: field values at or above the threshold
//! become cluster portals, the rest become small void populations, so the
//! point count always honors the parent budget. [`DensityGenerator`](crate::density::DensityGenerator) fixes the cell
//! for one place so the headless checks can use the [`Generator`](crate::gen::Generator) contract.
//!
//! [`Generated`]: crate::gen::Generated
//! [`Generator`]: crate::gen::Generator

use crate::coords::{CellPos, Level};
use crate::r#gen::{Constraints, Generated, Generator, MarkerKind, Point};
use crate::noise::value_noise_3d;
use crate::seed::{Rng, binomial_draw, hash_cell};

/// Spectrum octave count of the cosmic-web field (#153).
///
/// Five octaves span a factor of 16 in wavelength: the base octave sets the
/// void scale (order a tenth of a cell, toward ~1 Gly at L1-L2), higher
/// octaves add filament-scale detail with decaying power.
pub const SPECTRUM_OCTAVES: u32 = 5;

/// Power weights per spectrum octave, base octave first (#153).
///
/// A cosmological shape in model form: the peak sits at the second octave
/// (the void scale), the base octave is attenuated (no structure above
/// ~30-200 Mpc, the End of Greatness `S1`), and small scales decay
/// (cosmological falloff). Values are model settings, not measurements;
/// volume shares are verified by test, not by these numbers.
const SPECTRUM_WEIGHTS: [f64; SPECTRUM_OCTAVES as usize] = [0.30, 0.34, 0.20, 0.11, 0.05];

/// Lognormal width of the density map (#153).
///
/// The standardized field `g` (~N(0,1)) maps to `rho = exp(SIGMA * g -
/// SIGMA^2 / 2)` (standard initial-conditions practice per `S5`):
/// `SIGMA = 1.0` boosts dense nodes several times over the median while
/// suppressing voids, matching the skewed cosmic density contrast.
const LOGNORMAL_SIGMA: f64 = 1.0;

/// Octave seed stride (splitmix64 golden ratio; mirrors `noise`, #153).
///
/// Each spectrum octave advances the hash seed by this stride so octaves
/// decorrelate without sharing state.
const OCTAVE_STRIDE: u64 = 0x9E37_79B9_7F4A_7C15;

/// Baseline indicator points emitted at full parent density.
///
/// The parent budget scales this (`round(BASE * density)`, clamped to
/// `[min_count, max_count]`), exactly like the reference generator in
/// [`crate::gen`].
pub const DENSITY_BASE_COUNT: u32 = 32;

/// Indicator radius of a cluster member, in cell units.
const CLUSTER_RADIUS: f64 = 0.02;

/// Indicator radius of a void marker, in cell units.
const VOID_RADIUS: f64 = 0.008;

/// Decorrelation salt for the L2 supercluster-portal count draw (#151).
const SALT_PORTALS_L2: u64 = 0x2C1A_8E45_91F3_04D7;

/// Decorrelation salt for the L3 cluster-portal count draw (#151).
const SALT_PORTALS_L3_CLUSTERS: u64 = 0x8B3D_2A17_4C6E_91F0;

/// Decorrelation salt for the L3 group-portal count draw (#151).
const SALT_PORTALS_L3_GROUPS: u64 = 0x51F0_77AA_03BC_9E21;

/// Portal count draws, resolved for #153 (#151 set interim means).
///
/// The draws overshoot the non-void share on purpose: [`clusters_in_cell`]
/// demotes void-band samples to populations, so empty cells end up with
/// fewer portals (fewer choices where there is less) while dense cells keep
/// the full drawn tiers. The `1 +` terms plus the degenerate fallback below
/// guarantee the journey never strands.
///
/// Returns `(clusters, groups)` portal counts for `n` sampled points:
/// the densest `clusters` become cluster (L3) or supercluster (L2)
/// portals, the next `groups` become group portals (L3 only), the rest
/// populations. Points stay rank-ordered in the output vector, so the tier
/// of a marker is its index: `[0..clusters)` tier one, the next `groups`
/// tier two. Counts are Binomial draws (integer-only, hence identical on
/// every platform); point order never changes, only kinds.
pub(crate) fn portal_tiers(level: Level, seed: u64, points: usize) -> (usize, usize) {
    use crate::seed::binomial_draw;
    match level.get() {
        2 => {
            let clusters = 1 + binomial_draw(seed, SALT_PORTALS_L2, 64, 3, 64) as usize;
            (clusters.min(points), 0)
        }
        3 => {
            let clusters = 1 + binomial_draw(seed, SALT_PORTALS_L3_CLUSTERS, 48, 5, 48) as usize;
            let clusters = clusters.min(points);
            // Groups are common: every supercluster interior shows at least
            // one, so the autopilot always has a group portal to pick (#151).
            let groups = 1 + binomial_draw(seed, SALT_PORTALS_L3_GROUPS, 48, 5, 48) as usize;
            (clusters, groups.min(points - clusters))
        }
        _ => (points, 0),
    }
}

/// Cosmic-web environment of one field sample (#153).
///
/// Bands of the [`density_at`] field value, calibrated to the SpineWeb
/// volume shares (voids 77%, walls 20%, filaments 2%, nodes under 1%, `S5`):
/// every marker in L1-L3 carries one of these four kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Environment {
    /// Vast underdense region: shown small and faint, never opens.
    Void,
    /// Sheet-like overdensity hosting groups: opens as a group portal.
    Wall,
    /// Thread-like overdensity hosting clusters: opens as a cluster portal.
    Filament,
    /// Dense core hosting rich clusters: opens as a cluster portal.
    Node,
}

/// Field value below which a sample is a void (#153).
///
/// Empirically the 77th percentile of the spectrum field (8000 grid samples
/// over 5 seeds: q77 ~= 0.50); re-measure if the weights or `SIGMA` change.
pub const ENV_VOID_WALL: f64 = 0.50;

/// Field value below which a sample is a wall rather than a filament (#153).
///
/// Empirically the 97th percentile (same probe: q97 ~= 0.67).
pub const ENV_WALL_FIL: f64 = 0.67;

/// Field value below which a sample is a filament rather than a node (#153).
///
/// Empirically the 99th percentile (same probe: q99 ~= 0.72).
pub const ENV_FIL_NODE: f64 = 0.72;

/// Classifies a [`density_at`] field value into its environment (#153).
///
/// Pure threshold compare on the global field bands, so neighbouring cells
/// agree on shared samples by construction.
#[must_use]
pub fn environment_of(density: f64) -> Environment {
    if density < ENV_VOID_WALL {
        Environment::Void
    } else if density < ENV_WALL_FIL {
        Environment::Wall
    } else if density < ENV_FIL_NODE {
        Environment::Filament
    } else {
        Environment::Node
    }
}

/// Classifies the field at `(x, y, z)` into its environment (#153).
///
/// Convenience over [`environment_of`] for validators and tests; generation
/// classifies the already-sampled value instead of re-reading the field.
#[must_use]
pub fn environment_at(seed: u64, x: f64, y: f64, z: f64) -> Environment {
    environment_of(density_at(seed, x, y, z))
}

/// Index of the densest portal in `points`, or `None` when there is none.
///
/// Densities are recomputed from `seed` at the lattice origin `anchor_x`
/// (mirroring [`clusters_in_cell`]), compared with a total order
/// (`E-FLOAT-SORT`), ties keeping the smaller index. The prominence rule
/// (#151): a cell entered through its densest portal shows the rich
/// content, so the anchor object of a rich family is always rich.
#[must_use]
pub fn densest_portal_index(seed: u64, anchor_x: i64, points: &[Point]) -> Option<u32> {
    use std::cmp::Ordering;
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: lattice anchor is a small integer, exactly representable"
    )]
    let origin_x = anchor_x as f64;
    let mut best: Option<(u32, f64)> = None;
    for (index, point) in points.iter().enumerate() {
        if point.kind == MarkerKind::Portal {
            let density = density_at(
                seed,
                origin_x + point.position[0],
                point.position[1],
                point.position[2],
            );
            let better =
                best.is_none_or(|(_, known)| density.total_cmp(&known) == Ordering::Greater);
            if better {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "E-CAST: marker index into a budgeted cell, always fits u32"
                )]
                let marker = index as u32;
                best = Some((marker, density));
            }
        }
    }
    best.map(|(marker, _)| marker)
}

/// Samples the L1-L3 cosmic-web density field at `(x, y, z)`.
///
/// Spectrum-weighted Gaussian field, lognormal-mapped, over
/// [`SPECTRUM_OCTAVES`] octaves with `SPECTRUM_WEIGHTS` power: each octave
/// is hash-seeded value noise (no lookup tables, deterministic on every
/// platform), the weighted sum standardizes to ~N(0,1) (each octave is
/// ~U(0,1) with mean 1/2 and variance 1/12), and `rho/(1 + rho)` with
/// `rho = exp(SIGMA * g - SIGMA^2 / 2)` keeps the output in `[0.0, 1.0]`
/// with the median toward the old mid-grey, so downstream morphology mixes
/// barely move. Transcendental calls (`sqrt`, `exp`) stay inside this core
/// generator per `E-TRANSCENDENTAL`. The field is `C1`-continuous, so nearby
/// samples agree within a small epsilon (neighbour continuity). Non-finite
/// coordinates yield the neutral value `0.5` instead of propagating `NaN`.
#[must_use]
pub fn density_at(seed: u64, x: f64, y: f64, z: f64) -> f64 {
    if !x.is_finite() || !y.is_finite() || !z.is_finite() {
        return 0.5;
    }
    let mut weighted = 0.0;
    let mut weight_sum = 0.0;
    let mut weight_sq = 0.0;
    let mut frequency = 1.0;
    let mut octave_seed = seed;
    for weight in SPECTRUM_WEIGHTS {
        weighted +=
            weight * value_noise_3d(octave_seed, x * frequency, y * frequency, z * frequency);
        weight_sum += weight;
        weight_sq += weight * weight;
        frequency *= 2.0;
        octave_seed = octave_seed.wrapping_add(OCTAVE_STRIDE);
    }
    let mean = 0.5 * weight_sum;
    // Every weight is positive, so weight_sq > 0 and the divisor is finite.
    let std = (weight_sq / 12.0).sqrt();
    let gauss = (weighted - mean) / std;
    let rho = (LOGNORMAL_SIGMA * gauss - 0.5 * LOGNORMAL_SIGMA * LOGNORMAL_SIGMA).exp();
    rho / (1.0 + rho)
}

/// Expected global mean of [`density_at`] (#153).
///
/// Measured 0.39 over 8000 grid samples on 5 seeds; the count below
/// normalizes by it so an average-density cell keeps the full baseline
/// budget. Re-measure if the spectrum weights or `SIGMA` change.
const FIELD_MEAN: f64 = 0.39;

/// Decorrelation salt for the L1-L3 count scatter draw (#153).
const SALT_COUNT_L123: u64 = 0xC153_0D15_7CA7_7E12;

/// Counts a cell from its density times its volume (#153).
///
/// Eight fixed probes at the octant centers read the shared field (no RNG:
/// the count is a pure function of the cell's place in the field); the mean
/// sets a factor in `[0.25, 3.0]` around [`FIELD_MEAN`], and an integer-only
/// Binomial scatter (mean 0, range ±8) keeps neighbouring densities from
/// collapsing onto identical counts. Clamped to the parent budget, so the
/// packing cap still holds; portals are guaranteed separately by the caller,
/// so density never strands the journey.
fn density_count(seed: u64, cell_seed: u64, cell: CellPos, parent: &Constraints) -> u32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: cell coordinates are small integers, exactly representable"
    )]
    let origin = [cell.x as f64, cell.y as f64, cell.z as f64];
    let mut probe_sum = 0.0;
    for ox in [-0.25, 0.25] {
        for oy in [-0.25, 0.25] {
            for oz in [-0.25, 0.25] {
                probe_sum += density_at(seed, origin[0] + ox, origin[1] + oy, origin[2] + oz);
            }
        }
    }
    let factor = (probe_sum / 8.0 / FIELD_MEAN).clamp(0.25, 3.0);
    let budgeted = f64::from(DENSITY_BASE_COUNT) * factor * parent.density_multiplier;
    let scatter = i64::from(binomial_draw(cell_seed, SALT_COUNT_L123, 16, 1, 2)) - 8;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: budgeted is at most 3x base times density 1.0, far below i64::MAX"
    )]
    let total = budgeted.round() as i64 + scatter;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: clamped into [min_count, max_count], both u32"
    )]
    #[expect(
        clippy::cast_sign_loss,
        reason = "E-CAST: clamped at or above min_count which is >= 0"
    )]
    let count = total.clamp(i64::from(parent.min_count), i64::from(parent.max_count)) as u32;
    count
}

/// Generates cluster/void indicator points for one cell.
///
/// `seed` is the density-field seed shared by neighbouring cells; `cell`
/// locates this cell in the global lattice, so a sample at local offset `o`
/// reads `density_at(seed, cell + o)` and the shared face of two adjacent
/// cells maps to identical field coordinates. Candidates scatter uniformly
/// in `parent.allowed_extent` from a stream seeded by `hash_cell`, and each
/// is sampled once (#151: retries existed only to suppress voids; with
/// voids as first-class populations the single try is the honest sample).
/// Kinds are assigned by density rank over environments: at L2 the densest
/// non-void candidates become supercluster portals, at L3 the densest become
/// cluster portals, the next tier group portals, and the rest populations;
/// void-band samples never open (they demote to populations), so empty cells
/// show fewer portals. Counts scale with the cell's mean field density
/// (density times volume, integer-only scatter, clamped to the parent
/// budget). Portal counts are Binomial draws (#151, resolved #153). Each of the 8 child
/// octants receives half the parent density and half the count ceiling, so
/// ceiling, so [`respects`](crate::gen::respects) holds for every child.
/// An invalid `parent` yields empty output rather than panicking.
#[must_use]
pub fn clusters_in_cell(seed: u64, cell: CellPos, parent: &Constraints) -> Generated {
    if !parent.is_valid() {
        return Generated {
            points: Vec::new(),
            child_constraints: Vec::new(),
        };
    }
    let cell_seed = hash_cell(seed, cell.level.get(), cell.x, cell.y, cell.z);
    let count = density_count(seed, cell_seed, cell, parent);
    let mut rng = Rng::new(cell_seed);
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: cell coordinates are small integers, exactly representable"
    )]
    let origin = [cell.x as f64, cell.y as f64, cell.z as f64];
    let mut samples: Vec<([f64; 3], f64)> = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let mut position = [0.0; 3];
        for (slot, extent) in position.iter_mut().zip(parent.allowed_extent.iter()) {
            *slot = (rng.next_f64() * 2.0 - 1.0) * extent;
        }
        let density = density_at(
            seed,
            origin[0] + position[0],
            origin[1] + position[1],
            origin[2] + position[2],
        );
        samples.push((position, density));
    }
    // Rank by density, densest first (stable sort: ties keep sample order,
    // so rebuilds agree bit for bit; E-FLOAT-SORT total order).
    samples.sort_by(|a, b| b.1.total_cmp(&a.1));
    let (clusters, groups) = portal_tiers(cell.level, seed, samples.len());
    let want = (clusters + groups).min(samples.len());
    let mut points = Vec::with_capacity(samples.len());
    let mut portals = 0usize;
    for (order, (position, density)) in samples.iter().enumerate() {
        // Tier one (densest) and tier two (L3 groups) open, except void-band
        // samples never open (#153): they demote to populations. The points
        // stay rank-ordered, so a marker's tier is its index range, and the
        // demotion only ever trims the tail (voids are the lowest densities).
        let kind = if order < want && environment_of(*density) != Environment::Void {
            portals += 1;
            MarkerKind::Portal
        } else {
            MarkerKind::Population
        };
        // Radius follows kind: portals read prominent, populations faint.
        // (L3 clusters and groups share the portal radius; the tier split is
        // carried by rank, visible in snapshots through the kind prefix.)
        let radius = if kind == MarkerKind::Portal {
            CLUSTER_RADIUS
        } else {
            VOID_RADIUS
        };
        points.push(Point {
            position: *position,
            radius,
            kind,
            galaxy: None,
            star: None,
            planet: None,
            cloud: None,
            surface: None,
            moon: None,
            tint: None,
        });
    }
    // Degenerate all-void cell: keep the densest sample as the single portal
    // so the journey never strands. This is the only case where a void-band
    // sample opens (documented exception to void-never-opens, Spec v1 Q3/Q4).
    if portals == 0
        && let Some(first) = points.first_mut()
    {
        first.kind = MarkerKind::Portal;
        first.radius = CLUSTER_RADIUS;
    }
    let child_max = parent.max_count / 2;
    let child = Constraints {
        density_multiplier: parent.density_multiplier / 2.0,
        min_count: parent.min_count.min(child_max),
        max_count: child_max,
        allowed_extent: parent.allowed_extent,
    };
    Generated {
        points,
        child_constraints: vec![child; 8],
    }
}

/// Density-field generator fixed to one cell (M3 [`Generator`] adapter).
///
/// Delegates to [`clusters_in_cell`]: `seed` stays the shared field seed
/// while the stored cell provides the global lattice offset.
///
/// [`Generator`]: crate::gen::Generator
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DensityGenerator {
    /// Level of the cell this generator is fixed to.
    pub level: Level,
    /// Integer grid index of the cell along x.
    pub x: i64,
    /// Integer grid index of the cell along y.
    pub y: i64,
    /// Integer grid index of the cell along z.
    pub z: i64,
}

impl DensityGenerator {
    /// Fixes a density generator to the cell `(level, x, y, z)`.
    #[must_use]
    pub const fn new(level: Level, x: i64, y: i64, z: i64) -> DensityGenerator {
        DensityGenerator { level, x, y, z }
    }
}

impl Generator for DensityGenerator {
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        clusters_in_cell(
            seed,
            CellPos::new(self.level, self.x, self.y, self.z),
            parent,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r#gen::respects;

    /// Builds typical mid-ladder constraints for tests.
    fn parent_constraints() -> Constraints {
        Constraints::new(1.0, 2, 64, [0.5, 0.5, 0.5]).expect("valid test constraints")
    }

    /// Builds an L2 test cell at `(x, y, z)`.
    fn cell(x: i64, y: i64, z: i64) -> CellPos {
        CellPos::new(Level::new(2).expect("valid test level"), x, y, z)
    }

    #[test]
    fn density_is_deterministic_and_seed_sensitive() {
        let seed = 0x1234_5678_9ABC_DEF0;
        let (x, y, z) = (1.25, -0.5, 3.0);
        assert_eq!(
            density_at(seed, x, y, z).to_bits(),
            density_at(seed, x, y, z).to_bits()
        );
        assert_ne!(
            density_at(seed, x, y, z).to_bits(),
            density_at(seed ^ 0x1, x, y, z).to_bits()
        );
    }

    #[test]
    fn density_stays_in_unit_range() {
        let seed = 777;
        for x in [-256.0, -1.5, 0.0, 0.5, 40.0, 1.0e6] {
            for y in [-3.25, 0.0, 2.0] {
                let density = density_at(seed, x, y, x - y);
                assert!((0.0..=1.0).contains(&density), "out of range: {density}");
            }
        }
        assert_eq!(
            density_at(seed, f64::NAN, 0.0, 0.0).to_bits(),
            0.5f64.to_bits()
        );
        assert_eq!(
            density_at(seed, 0.0, f64::INFINITY, 0.0).to_bits(),
            0.5f64.to_bits()
        );
    }

    #[test]
    fn shared_face_samples_agree_across_neighbors() {
        let seed = 0x1234_5678_9ABC_DEF0;
        // Shared face between cells x=0 and x=1: local +0.5 meets local -0.5.
        for u in [-0.375, -0.125, 0.125, 0.375] {
            for v in [-0.375, -0.125, 0.125, 0.375] {
                let from_left = density_at(seed, 0.0 + 0.5, u, v);
                let from_right = density_at(seed, 1.0 - 0.5, u, v);
                assert!(
                    (from_left - from_right).abs() <= 1e-12,
                    "face mismatch at ({u}, {v}): {from_left} vs {from_right}"
                );
            }
        }
        // Field smoothness: millistep moves change the value only slightly.
        let base = density_at(seed, 1.25, -0.5, 3.0);
        for step in [[1e-3, 0.0, 0.0], [0.0, 1e-3, 0.0], [0.0, 0.0, 1e-3]] {
            let near = density_at(seed, 1.25 + step[0], -0.5 + step[1], 3.0 + step[2]);
            assert!(
                (base - near).abs() <= 0.05,
                "field jumps too fast: {base} vs {near}"
            );
        }
    }

    #[test]
    fn clusters_are_deterministic_and_seed_sensitive() {
        let parent = parent_constraints();
        let place = cell(3, -4, 5);
        assert_eq!(
            clusters_in_cell(99, place, &parent),
            clusters_in_cell(99, place, &parent)
        );
        assert_ne!(
            clusters_in_cell(99, place, &parent),
            clusters_in_cell(100, place, &parent)
        );
        let generator = DensityGenerator::new(place.level, place.x, place.y, place.z);
        assert_eq!(
            generator.generate(99, &parent),
            clusters_in_cell(99, place, &parent)
        );
    }

    #[test]
    fn clusters_honor_parent_and_children_respect_parent() {
        let parent = parent_constraints();
        let out = clusters_in_cell(99, cell(3, -4, 5), &parent);
        assert!(
            (parent.min_count as usize..=parent.max_count as usize).contains(&out.points.len())
        );
        for point in &out.points {
            assert!(point.radius > 0.0, "non-positive radius");
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
    fn single_try_sampling_yields_both_kinds() {
        let parent = parent_constraints();
        let out = clusters_in_cell(99, cell(3, -4, 5), &parent);
        let portals = out
            .points
            .iter()
            .filter(|point| point.kind == MarkerKind::Portal)
            .count();
        let populations = out.points.len() - portals;
        assert!(portals > 0, "no cluster portals sampled");
        assert!(populations > 0, "no void populations sampled");
    }

    #[test]
    fn environment_bands_follow_field_value() {
        assert_eq!(environment_of(0.0), Environment::Void);
        assert_eq!(environment_of(0.49), Environment::Void);
        assert_eq!(environment_of(0.50), Environment::Wall);
        assert_eq!(environment_of(0.66), Environment::Wall);
        assert_eq!(environment_of(0.67), Environment::Filament);
        assert_eq!(environment_of(0.71), Environment::Filament);
        assert_eq!(environment_of(0.72), Environment::Node);
        assert_eq!(environment_of(0.99), Environment::Node);
        // Ordering is total: voids sort below everything else.
        assert!(Environment::Void < Environment::Wall);
        assert!(Environment::Wall < Environment::Filament);
        assert!(Environment::Filament < Environment::Node);
    }

    #[test]
    fn volume_shares_near_spineweb_targets() {
        // 8000 grid samples over 5 seeds: voids ~77%, walls ~20%,
        // filaments ~2%, nodes under 1% [S5]. Bands are wide enough for
        // seed variation; a failure here means the field shape drifted.
        let mut counts = [0usize; 4];
        let mut total = 0usize;
        for seed in [7u64, 42, 99, 1234, 99991] {
            for xi in 0..40 {
                for yi in 0..40 {
                    let (x, y) = (f64::from(xi) * 0.37, f64::from(yi) * 0.53);
                    let env = environment_at(seed, x, y, 1.7);
                    counts[env as usize] += 1;
                    total += 1;
                }
            }
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: test share over small integers, exactness irrelevant"
        )]
        let share = |n: usize| n as f64 / total as f64;
        assert!(
            (0.70..=0.84).contains(&share(counts[0])),
            "void share drifted: {:.3}",
            share(counts[0])
        );
        assert!(
            (0.15..=0.25).contains(&share(counts[1])),
            "wall share drifted: {:.3}",
            share(counts[1])
        );
        assert!(
            (0.005..=0.04).contains(&share(counts[2])),
            "filament share drifted: {:.3}",
            share(counts[2])
        );
        assert!(
            share(counts[3]) <= 0.025,
            "node share drifted: {:.3}",
            share(counts[3])
        );
    }

    #[test]
    fn portals_never_void_outside_degenerate_cells() {
        let parent = parent_constraints();
        for seed in [7u64, 42, 99, 1234, 99991, 20261004] {
            for level_n in [2u8, 3] {
                let place = CellPos::new(Level::new(level_n).expect("valid test level"), 3, -4, 5);
                let out = clusters_in_cell(seed, place, &parent);
                if out.points.is_empty() {
                    continue;
                }
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "E-CAST: test cell coordinates are small integers"
                )]
                let origin = [place.x as f64, place.y as f64, place.z as f64];
                let envs: Vec<Environment> = out
                    .points
                    .iter()
                    .map(|point| {
                        environment_of(density_at(
                            seed,
                            origin[0] + point.position[0],
                            origin[1] + point.position[1],
                            origin[2] + point.position[2],
                        ))
                    })
                    .collect();
                let any_nonvoid = envs.iter().any(|env| *env != Environment::Void);
                let portals = out
                    .points
                    .iter()
                    .filter(|point| point.kind == MarkerKind::Portal)
                    .count();
                if any_nonvoid {
                    for (point, env) in out.points.iter().zip(envs.iter()) {
                        if *env == Environment::Void {
                            assert_eq!(
                                point.kind,
                                MarkerKind::Population,
                                "void-band sample opens (seed {seed} L{level_n})"
                            );
                        }
                    }
                    assert!(portals > 0, "non-degenerate cell lost all portals");
                } else {
                    // Degenerate all-void cell: the single densest sample
                    // keeps the journey going (documented exception).
                    assert_eq!(portals, 1, "degenerate cell must keep one portal");
                }
            }
        }
    }

    #[test]
    fn field_mean_matches_count_normalization() {
        // Guards FIELD_MEAN: the global mean over a wide grid must stay near
        // the constant the count factor normalizes by.
        let mut sum = 0.0;
        let mut total = 0usize;
        for seed in [7u64, 42, 99, 1234, 99991] {
            for xi in 0..20 {
                for yi in 0..20 {
                    for zi in 0..20 {
                        let (x, y, z) = (
                            f64::from(xi) * 0.61,
                            f64::from(yi) * 0.43,
                            f64::from(zi) * 0.79,
                        );
                        sum += density_at(seed, x, y, z);
                        total += 1;
                    }
                }
            }
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: test mean over small integers, exactness irrelevant"
        )]
        let mean = sum / total as f64;
        assert!(
            (0.34..=0.44).contains(&mean),
            "global mean drifted: {mean:.4} (FIELD_MEAN = {FIELD_MEAN})"
        );
    }

    #[test]
    fn dense_cells_hold_more_than_void_cells() {
        // Density times volume: across many seeds, the quarter of cells with
        // the highest probe-mean density must clearly outcount the lowest
        // quarter. Scatter is +-8 integer-only; the factor range (0.25-3.0)
        // dominates it.
        let parent = parent_constraints();
        let mut ranked: Vec<(f64, usize)> = Vec::new();
        for seed in 0..120u64 {
            #[expect(
                clippy::cast_possible_wrap,
                reason = "E-CAST: test seeds 0..120, far below i64::MAX"
            )]
            let place = CellPos::new(
                Level::new(3).expect("valid test level"),
                seed as i64 - 60,
                0,
                0,
            );
            let out = clusters_in_cell(seed, place, &parent);
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: test cell coordinates are small integers"
            )]
            let origin = [place.x as f64, 0.0, 0.0];
            let mut probe = 0.0;
            for ox in [-0.25, 0.25] {
                for oy in [-0.25, 0.25] {
                    for oz in [-0.25, 0.25] {
                        probe += density_at(seed, origin[0] + ox, oy, oz);
                    }
                }
            }
            ranked.push((probe / 8.0, out.points.len()));
        }
        ranked.sort_by(|a, b| a.0.total_cmp(&b.0));
        let quarter = ranked.len() / 4;
        let low: usize = ranked[..quarter].iter().map(|(_, n)| n).sum();
        let high: usize = ranked[3 * quarter..].iter().map(|(_, n)| n).sum();
        assert!(
            high > low + ranked.len(),
            "counts ignore density: low-quarter total {low}, high-quarter total {high}"
        );
    }

    #[test]
    fn counts_stay_deterministic_and_bounded() {
        let parent = parent_constraints();
        for seed in [7u64, 42, 99, 1234] {
            for level_n in [2u8, 3] {
                let place = CellPos::new(Level::new(level_n).expect("valid test level"), -2, 9, 1);
                let first = clusters_in_cell(seed, place, &parent);
                let second = clusters_in_cell(seed, place, &parent);
                assert_eq!(first, second, "count nondeterminism");
                assert!(
                    (parent.min_count as usize..=parent.max_count as usize)
                        .contains(&first.points.len()),
                    "count outside parent budget"
                );
            }
        }
    }

    #[test]
    fn invalid_parent_yields_empty_output() {
        let bad = Constraints {
            density_multiplier: 2.0,
            min_count: 5,
            max_count: 2,
            allowed_extent: [0.5; 3],
        };
        assert!(!bad.is_valid());
        let out = clusters_in_cell(7, cell(0, 0, 0), &bad);
        assert!(out.points.is_empty());
        assert!(out.child_constraints.is_empty());
    }
}
