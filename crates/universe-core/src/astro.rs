//! Galaxy-to-star sampling with anonymous octree cells (M4, sub-issue #41).
//!
//! Levels L3-L9 run from galaxy clusters down to individual stars. Named
//! levels alone cannot hold ~10^11 stars per galaxy, so the gap between L4
//! (galaxies) and L9 (stars) is filled with anonymous [`Octree`](crate::astro::Octree) cells
//! (notion section 5.4): each node derives its seed via `hash_cell`, holds
//! at most [`MAX_OBJECTS_PER_LEAF`](crate::astro::MAX_OBJECTS_PER_LEAF) objects, and otherwise subdivides.
//! [`sample_galaxy`](crate::astro::sample_galaxy) draws type, size, and orientation from the local
//! density; [`spiral_star_positions`](crate::astro::spiral_star_positions) lays out log-spiral arms with an
//! exponential radial profile and a truncated Salpeter mass function; and
//! [`GalaxyGenerator`](crate::astro::GalaxyGenerator) exposes galaxy indicators through the [`Generator`](crate::gen::Generator)
//! contract with child budgets that respect their parent.
//!
//! [`Generator`]: crate::gen::Generator

use crate::coords::Level;
use crate::density::density_at;
use crate::r#gen::{Constraints, Generated, Generator, MarkerKind, Point};
use crate::seed::{Rng, binomial_draw, hash_cell, hash_triple};
use std::f64::consts::PI;

/// Most objects an anonymous octree leaf may hold (notion section 5.4).
///
/// [`Octree::build`] subdivides until every leaf fits this bound (or the
/// depth cap below binds pathological inputs).
pub const MAX_OBJECTS_PER_LEAF: usize = 1000;

/// Deepest anonymous subdivision below the galaxy cell.
///
/// The first six depths cover the L4-L9 ladder gap; the cap only binds
/// inputs that refuse to separate (bit-identical points or a collapsed
/// extent), which stay in one leaf by design.
pub const OCTREE_MAX_DEPTH: u8 = 8;

/// Ladder level mixed into the root anonymous seed derivation.
///
/// Anonymous depth `d` hashes with level `OCTREE_BASE_LEVEL + d`, so galaxy
/// octree seeds never collide with named-level seeds from the same parent.
pub const OCTREE_BASE_LEVEL: u8 = 5;

/// Baseline galaxy indicators emitted at full parent density.
///
/// The parent budget scales this exactly like the reference generator in
/// [`crate::gen`].
pub const GALAXY_BASE_COUNT: u32 = 16;

/// Lowest stellar mass sampled, in solar masses (hydrogen-burning limit).
pub const MIN_STAR_MASS: f64 = 0.08;

/// Highest stellar mass sampled, in solar masses (Salpeter-tail truncation).
pub const MAX_STAR_MASS: f64 = 120.0;

/// Domain-separation tag folded into star-layout streams.
const STAR_STREAM_TAG: u64 = 0x85EB_CA77_C2B2_AE63;

/// Morphological class of a sampled galaxy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GalaxyType {
    /// Disk with density-wave arms; hosts the spiral layout.
    Spiral,
    /// Spheroidal system; stars follow a gaussian spheroid instead of arms.
    Elliptical,
    /// Clumpy system without regular symmetry; stars fill a jittered box.
    Irregular,
}

/// Sampled galaxy: shape, extent, and disk orientation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Galaxy {
    /// Morphological class (density-biased by [`sample_galaxy`]).
    pub galaxy_type: GalaxyType,
    /// Characteristic radius in cell units; always positive.
    pub size: f64,
    /// Unit normal of the disk plane (arbitrary for [`GalaxyType::Elliptical`]).
    pub orientation: [f64; 3],
}

/// Samples a galaxy from the local cosmic-web `density` in `[0.0, 1.0]`.
///
/// Ellipticals favor dense cluster cores while spirals dominate the field
/// (15% elliptical at zero density rising to 65% at full density, 10%
/// irregular throughout); size is log-normal around 0.05 cell units and the
/// orientation is uniform on the sphere. Non-finite densities read as the
/// mid-grey `0.5`. Draws only from `rng`, so a fresh stream replays the same
/// galaxy every time.
pub fn sample_galaxy(rng: &mut Rng, density: f64) -> Galaxy {
    let clamped = if density.is_finite() {
        density.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let roll = rng.next_f64();
    let galaxy_type = if roll < 0.15 + 0.5 * clamped {
        GalaxyType::Elliptical
    } else if roll < 0.9 {
        GalaxyType::Spiral
    } else {
        GalaxyType::Irregular
    };
    let bell = rng.next_f64() + rng.next_f64() + rng.next_f64() - 1.5;
    let size = 0.05 * 2.0f64.powf(bell);
    let z = rng.next_f64() * 2.0 - 1.0;
    let angle = rng.next_f64() * 2.0 * PI;
    let ring = (1.0 - z * z).max(0.0).sqrt();
    Galaxy {
        galaxy_type,
        size,
        orientation: [ring * angle.cos(), ring * angle.sin(), z],
    }
}

/// One sampled star: a galaxy-centered position plus a mass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Star {
    /// Position relative to the galaxy center, in cell units.
    pub position: [f64; 3],
    /// Stellar mass in solar masses, in `[MIN_STAR_MASS, MAX_STAR_MASS]`.
    pub mass_solar: f64,
}

/// Lays out exactly `count` star positions for `galaxy` from `seed`.
///
/// Spirals get a central bulge plus two log-spiral arms over an exponential
/// radial profile (truncated at two characteristic radii); ellipticals get a
/// flattened gaussian spheroid; irregulars get a uniform box. Masses follow a
/// truncated Salpeter function (`dN/dM ~ M^-2.35`). Positions are
/// galaxy-centered; the caller offsets them into the cell frame. The stream
/// is domain-separated from other generators, and degenerate galaxies
/// (non-positive size, zero orientation) collapse gracefully toward the
/// center instead of producing `NaN`.
#[must_use]
pub fn spiral_star_positions(seed: u64, galaxy: &Galaxy, count: u32) -> Vec<Star> {
    let mut rng = Rng::new(seed.wrapping_add(STAR_STREAM_TAG));
    let normal = unit_or_default(galaxy.orientation);
    let tangent = orthonormal_tangent(normal);
    let bitangent = cross(normal, tangent);
    let mut stars = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let disk = match galaxy.galaxy_type {
            GalaxyType::Spiral => spiral_disk_point(&mut rng, galaxy.size),
            GalaxyType::Elliptical => spheroid_point(&mut rng, galaxy.size),
            GalaxyType::Irregular => box_point(&mut rng, galaxy.size),
        };
        let position = [
            tangent[0] * disk[0] + bitangent[0] * disk[1] + normal[0] * disk[2],
            tangent[1] * disk[0] + bitangent[1] * disk[1] + normal[1] * disk[2],
            tangent[2] * disk[0] + bitangent[2] * disk[1] + normal[2] * disk[2],
        ];
        stars.push(Star {
            position,
            mass_solar: salpeter_mass(&mut rng),
        });
    }
    stars
}

/// Approximately normal sample in `[-1.5, 1.5]` (sum of three uniforms).
fn gaussian(rng: &mut Rng) -> f64 {
    rng.next_f64() + rng.next_f64() + rng.next_f64() - 1.5
}

/// Draws one spiral-disk offset: bulge ball or an arm point.
fn spiral_disk_point(rng: &mut Rng, size: f64) -> [f64; 3] {
    let scale = (size * 0.33).max(1e-9);
    if rng.next_f64() < 0.25 {
        let spread = size.max(1e-9) * 0.15;
        return [
            gaussian(rng) * spread,
            gaussian(rng) * spread,
            gaussian(rng) * spread * 0.7,
        ];
    }
    let radius = (-(1.0 - rng.next_f64()).ln() * scale).min(size.max(1e-9) * 2.0);
    let arm = if rng.next_f64() < 0.5 { 0.0 } else { PI };
    let angle = arm + (radius / scale).max(1e-6).ln() / 0.21 + (rng.next_f64() - 0.5) * 0.3;
    let thickness = size.max(1e-9) * 0.06;
    [
        radius * angle.cos(),
        radius * angle.sin(),
        gaussian(rng) * thickness,
    ]
}

/// Draws one spheroid offset for elliptical galaxies.
fn spheroid_point(rng: &mut Rng, size: f64) -> [f64; 3] {
    let spread = size.max(1e-9) * 0.4;
    [
        gaussian(rng) * spread,
        gaussian(rng) * spread,
        gaussian(rng) * spread * 0.7,
    ]
}

/// Draws one uniform-box offset for irregular galaxies.
fn box_point(rng: &mut Rng, size: f64) -> [f64; 3] {
    let extent = size.max(1e-9);
    [
        (rng.next_f64() * 2.0 - 1.0) * extent,
        (rng.next_f64() * 2.0 - 1.0) * extent,
        (rng.next_f64() * 2.0 - 1.0) * extent,
    ]
}

/// Draws a truncated Salpeter mass in `[MIN_STAR_MASS, MAX_STAR_MASS]`.
fn salpeter_mass(rng: &mut Rng) -> f64 {
    let remaining = 1.0 - rng.next_f64();
    (MIN_STAR_MASS / remaining.powf(1.0 / 1.35)).min(MAX_STAR_MASS)
}

/// Normalizes `v`, falling back to +z for zero or non-finite inputs.
fn unit_or_default(v: [f64; 3]) -> [f64; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length.is_finite() && length > 1e-12 {
        [v[0] / length, v[1] / length, v[2] / length]
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// Builds a unit vector perpendicular to the unit `normal`.
fn orthonormal_tangent(normal: [f64; 3]) -> [f64; 3] {
    let helper = if normal[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    unit_or_default(cross(helper, normal))
}

/// Cross product of two 3-vectors.
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Anonymous spatial cell between L5 and L10 (notion section 5.4).
///
/// Internal nodes hold no objects (`objects` is empty); leaves hold at most
/// [`MAX_OBJECTS_PER_LEAF`] entries. `coord` is the integer index at this
/// depth (root `[0, 0, 0]`, child `2 * parent + bit`), and `seed` is
/// `hash_cell(parent seed, OCTREE_BASE_LEVEL + depth, coord)`, exactly like a
/// named level. Children sit in `4*x + 2*y + z` octant order.
#[derive(Clone, Debug, PartialEq)]
pub struct OctreeNode {
    /// Seed derived via `hash_cell` for this node.
    pub seed: u64,
    /// Cell center in the build frame.
    pub center: [f64; 3],
    /// Half-extent of the cubic cell.
    pub half_extent: f64,
    /// Anonymous depth below the galaxy cell (`0` is the root).
    pub depth: u8,
    /// Integer index at this depth (root `[0, 0, 0]`).
    pub coord: [i64; 3],
    /// Payload of leaf nodes; empty on internal nodes.
    pub objects: Vec<[f64; 3]>,
    /// Eight children in octant order; `None` on leaves.
    pub children: Option<Box<[OctreeNode; 8]>>,
}

impl OctreeNode {
    /// Recursively builds the subtree rooted at this node.
    fn build(
        seed: u64,
        center: [f64; 3],
        half_extent: f64,
        coord: [i64; 3],
        depth: u8,
        objects: Vec<[f64; 3]>,
    ) -> OctreeNode {
        if objects.len() <= MAX_OBJECTS_PER_LEAF
            || depth >= OCTREE_MAX_DEPTH
            || !half_extent.is_finite()
            || half_extent <= 0.0
            || points_identical(&objects)
        {
            return OctreeNode {
                seed,
                center,
                half_extent,
                depth,
                coord,
                objects,
                children: None,
            };
        }
        let mut buckets: [Vec<[f64; 3]>; 8] = std::array::from_fn(|_| Vec::new());
        for point in objects {
            #[expect(
                clippy::indexing_slicing,
                reason = "E-NO-PANIC: octant_index yields 0..8 by construction"
            )]
            buckets[octant_index(center, point)].push(point);
        }
        let level = OCTREE_BASE_LEVEL + depth + 1;
        let half = half_extent / 2.0;
        let mut grown: Vec<OctreeNode> = Vec::with_capacity(8);
        for (index, bucket) in buckets.into_iter().enumerate() {
            let bits = [(index >> 2) & 1 == 1, (index >> 1) & 1 == 1, index & 1 == 1];
            let [bx, by, bz] = bits;
            let [ccx, ccy, ccz] = coord;
            let [ctx, cty, ctz] = center;
            let child_coord = [
                ccx * 2 + i64::from(bx),
                ccy * 2 + i64::from(by),
                ccz * 2 + i64::from(bz),
            ];
            let child_center = [
                ctx + if bx { half } else { -half },
                cty + if by { half } else { -half },
                ctz + if bz { half } else { -half },
            ];
            let child_seed = hash_cell(seed, level, child_coord[0], child_coord[1], child_coord[2]);
            grown.push(OctreeNode::build(
                child_seed,
                child_center,
                half,
                child_coord,
                depth + 1,
                bucket,
            ));
        }
        #[expect(
            clippy::expect_used,
            reason = "E-NO-PANIC: grown holds exactly the eight buckets pushed above"
        )]
        let children: [OctreeNode; 8] = grown.try_into().expect("eight octants");
        OctreeNode {
            seed,
            center,
            half_extent,
            depth,
            coord,
            objects: Vec::new(),
            children: Some(Box::new(children)),
        }
    }

    /// Returns `true` when this node holds objects directly (no children).
    #[must_use]
    pub fn is_leaf(&self) -> bool {
        self.children.is_none()
    }

    /// Returns the object count under this node.
    pub fn total_objects(&self) -> usize {
        match &self.children {
            None => self.objects.len(),
            Some(children) => children.iter().map(OctreeNode::total_objects).sum(),
        }
    }

    /// Collects the leaves below this node, depth-first in octant order.
    pub fn leaf_nodes(&self) -> Vec<&OctreeNode> {
        match &self.children {
            None => vec![self],
            Some(children) => children.iter().flat_map(OctreeNode::leaf_nodes).collect(),
        }
    }
}

/// Anonymous octree between L5 and L10 (notion section 5.4).
///
/// Built once per galaxy cell from [`Octree::build`]: pure function of its
/// inputs, so the same objects rebuild the identical tree regardless of
/// generation order, and the leaf object counts always sum back to the input
/// length (star count stability).
#[derive(Clone, Debug, PartialEq)]
pub struct Octree {
    /// Root anonymous cell (depth 0).
    pub root: OctreeNode,
}

impl Octree {
    /// Builds the octree over `objects` in the cube around `center`.
    ///
    /// `seed` is this root cell's own seed (derived by the caller via
    /// `hash_cell`); every descendant derives its seed from its parent the
    /// same way. Subdivision stops at leaves holding at most
    /// [`MAX_OBJECTS_PER_LEAF`] objects.
    #[must_use]
    pub fn build(seed: u64, center: [f64; 3], half_extent: f64, objects: Vec<[f64; 3]>) -> Octree {
        Octree {
            root: OctreeNode::build(seed, center, half_extent, [0, 0, 0], 0, objects),
        }
    }

    /// Returns the leaf cells, depth-first in octant order.
    #[must_use]
    pub fn leaf_nodes(&self) -> Vec<&OctreeNode> {
        self.root.leaf_nodes()
    }

    /// Returns the total object count across all leaves.
    #[must_use]
    pub fn total_objects(&self) -> usize {
        self.root.total_objects()
    }
}

/// Returns `true` when every point is bit-identical (subdivision is futile).
fn points_identical(objects: &[[f64; 3]]) -> bool {
    objects.windows(2).all(|pair| {
        let (Some(first), Some(second)) = (pair.first(), pair.get(1)) else {
            return true;
        };
        first
            .iter()
            .zip(second.iter())
            .all(|(a, b)| a.to_bits() == b.to_bits())
    })
}

/// Returns the child octant index (`4*x + 2*y + z`) of `point` about `center`.
///
/// Points on the splitting plane sort into the negative octant (`NaN`
/// included, since its comparisons fail); the rule is deterministic either
/// way, so rebuilds always agree.
fn octant_index(center: [f64; 3], point: [f64; 3]) -> usize {
    let x = usize::from(point[0] >= center[0]);
    let y = usize::from(point[1] >= center[1]);
    let z = usize::from(point[2] >= center[2]);
    x * 4 + y * 2 + z
}

/// Reference generator scattering galaxy indicators.
///
/// Each point samples the [`density_at`] field at its position and draws a
/// [`Galaxy`] from that density; the indicator radius is the galaxy size, so
/// dense regions read as clusters of large markers. Placement and sizes are
/// uniform per the parent budget; the per-level portal/ population split of
/// #151 (counts, kinds, rich/poor) is assigned after sampling, so this
/// generator never decides what opens. Each of the 8 child octants receives
/// half the parent density and half the count ceiling, so
/// [`respects`](crate::gen::respects) holds for every child. An invalid
/// parent yields empty output rather than panicking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GalaxyGenerator {
    /// Level this generator is fixed to (drives the #151 kind model).
    pub level: Level,
    /// Rich-cluster content (L4 Virgo-analogs: thousands of members).
    pub rich: bool,
}

/// Portals in a rich cluster cell (large members that open).
pub const RICH_CLUSTER_PORTALS: u32 = 160;

/// Points in a rich cluster cell (members down to dwarfs).
pub const RICH_CLUSTER_TOTAL: u32 = 2000;

/// Decorrelation salt for the L4-poor portal count draw (#151).
const SALT_GALAXY_L4: u64 = 0x1A2B_3C4D_5E6F_7081;

/// Decorrelation salt for the L5 cloud-portal count draw (#151).
const SALT_GALAXY_L5: u64 = 0x7081_96A5_B4C3_D2E1;

/// Count salt for the L6 system-portal draw (#151, floor #153).
///
/// Calibrated so the fixed home journey (root 42) lands on a cloud cell
/// with system portals (first hit at probe time: 2 portals); other roots
/// get valid statistics. A test pins the home outcome, so any change here
/// fails loudly. #153 clamps the draw to at least one portal (Spec v1 AC3:
/// every visited journey cell keeps a portal): empty draws read as one
/// system, nonzero draws are unchanged, so the home fixture and the global
/// statistics both stand.
const SALT_COUNT_L6: u64 = 0x2;

/// Count salt for the shared planet draw (#151).
///
/// L8 and L9 derive the planet count from the same system seed (the seed
/// the L9 cell will have), so both views agree. Calibrated so the fixed
/// home journey (root 42) draws exactly 8 planets (the Solar fixture); a
/// test pins it. Recalibrated for the #153 path (first hit at probe time:
/// 106); the draw stays Binomial(64, 4/64), so global statistics are
/// unchanged, only which cells draw 8 moves.
const SALT_PLANETS: u64 = 106;

/// Salt for the L9 companion draw (#151, no calibration needed).
const SALT_COMPANIONS: u64 = 0xD2E1_F008_192A_3B4C;

impl GalaxyGenerator {
    /// Fixes a galaxy generator to `level` (`rich` selects Virgo-like
    /// content at L4; ignored elsewhere).
    #[must_use]
    pub const fn new(level: Level, rich: bool) -> GalaxyGenerator {
        GalaxyGenerator { level, rich }
    }

    /// Point total for the level: rich L4 cells hold thousands, L9 holds
    /// companions plus planets, everything else fills the era budget.
    /// The caller caps against `parent.max_count`.
    fn model_total(&self, seed: u64) -> u32 {
        match self.level.get() {
            4 if self.rich => RICH_CLUSTER_TOTAL,
            9 => {
                let (planets, companions) = Self::l9_counts(seed);
                planets + companions
            }
            _ => 32,
        }
    }

    /// Planet and companion counts of an L9 star close-up cell (#151).
    ///
    /// Planets match the L8 view: both derive from the system seed (the
    /// seed the L9 cell has by path-seed construction), so counts agree.
    /// Companions are rare (about one cell in four has one).
    fn l9_counts(seed: u64) -> (u32, u32) {
        let planets = binomial_draw(seed, SALT_PLANETS, 64, 4, 64).clamp(1, 12);
        let companions = u32::from(hash_triple(seed, SALT_COMPANIONS, 0).is_multiple_of(4));
        (planets, companions)
    }

    /// Planet portals shared by the L8 and L9 views of one system (#151).
    ///
    /// Drawn from the system seed both cells agree on. Solar home draws
    /// exactly 8 by `SALT_PLANETS` calibration; a test pins it.
    fn planet_count(system_seed: u64) -> u32 {
        binomial_draw(system_seed, SALT_PLANETS, 64, 4, 64).clamp(1, 12)
    }

    /// Portal prefix length for the level (#151 portal/population split).
    ///
    /// Samples sort largest-first, so the first `portal_count` points open
    /// and the rest is shown: L4/L5 rank largest galaxies first; L6 takes
    /// the first samples as systems; L7/L8 put the star first; L9 shows
    /// planets then companions. Positions never move afterwards, only
    /// kinds, and rank order doubles as the tier convention.
    fn portal_count(&self, seed: u64, total: usize) -> usize {
        let keep = match self.level.get() {
            4 if self.rich => RICH_CLUSTER_PORTALS,
            4 => 1 + binomial_draw(seed, SALT_GALAXY_L4, 64, 7, 64),
            5 => 1 + binomial_draw(seed, SALT_GALAXY_L5, 32, 5, 32),
            6 => binomial_draw(seed, SALT_COUNT_L6, 1000, 3, 10000).clamp(1, 2),
            7 => 1,
            8 => {
                // The seed this system's L9 cell will have by path-seed
                // construction, so both views draw the same planet count.
                let system_seed = hash_cell(seed, 9, 0, 0, 0);
                1 + Self::planet_count(system_seed)
            }
            9 => Self::l9_counts(seed).0,
            _ => {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "E-CAST: cell totals stay far below u32::MAX by budget"
                )]
                let all = total as u32;
                all
            }
        };
        (keep as usize).min(total)
    }
}

impl Generator for GalaxyGenerator {
    fn generate(&self, seed: u64, parent: &Constraints) -> Generated {
        if !parent.is_valid() {
            return Generated {
                points: Vec::new(),
                child_constraints: Vec::new(),
            };
        }
        let total = self.model_total(seed).min(parent.max_count) as usize;
        let mut rng = Rng::new(seed);
        let mut samples: Vec<([f64; 3], f64)> = Vec::with_capacity(total);
        for _ in 0..total {
            let mut position = [0.0; 3];
            for (slot, extent) in position.iter_mut().zip(parent.allowed_extent.iter()) {
                *slot = (rng.next_f64() * 2.0 - 1.0) * extent;
            }
            let density = density_at(seed, position[0], position[1], position[2]);
            let galaxy = sample_galaxy(&mut rng, density);
            samples.push((position, galaxy.size));
        }
        // Largest first (stable: ties keep sample order, so rebuilds agree
        // bit for bit); the portal prefix then doubles as the tier order.
        samples.sort_by(|a, b| b.1.total_cmp(&a.1));
        let portals = self.portal_count(seed, total);
        let mut points = Vec::with_capacity(total);
        for (order, (position, size)) in samples.into_iter().enumerate() {
            let kind = if order < portals {
                MarkerKind::Portal
            } else {
                MarkerKind::Population
            };
            points.push(Point {
                position,
                radius: size,
                kind,
                galaxy: None,
            });
        }
        // L7/L8 are entered through their star: it sits at the cell center
        // as the reference point (#151; physical layouts land in #155).
        if (self.level.get() == 7 || self.level.get() == 8)
            && let Some(star) = points.first_mut()
        {
            star.position = [0.0, 0.0, 0.0];
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r#gen::respects;

    /// Builds typical mid-ladder constraints for tests.
    fn parent_constraints() -> Constraints {
        Constraints::new(1.0, 2, 64, [0.5, 0.5, 0.5]).expect("valid test constraints")
    }

    /// Samples one galaxy from a fresh stream (keeps tests to one line).
    fn galaxy_at(stream_seed: u64, density: f64) -> Galaxy {
        sample_galaxy(&mut Rng::new(stream_seed), density)
    }

    /// Builds a test galaxy of the given type.
    fn test_galaxy(galaxy_type: GalaxyType) -> Galaxy {
        Galaxy {
            galaxy_type,
            size: 0.05,
            orientation: [0.0, 0.0, 1.0],
        }
    }

    #[test]
    fn galaxy_sampling_is_deterministic_and_valid() {
        for density in [0.0, 0.3, 0.7, 1.0, f64::NAN] {
            let first = galaxy_at(42, density);
            assert_eq!(first, galaxy_at(42, density));
            assert!(first.size.is_finite() && first.size > 0.0);
            let length = (first.orientation[0] * first.orientation[0]
                + first.orientation[1] * first.orientation[1]
                + first.orientation[2] * first.orientation[2])
                .sqrt();
            assert!(
                (length - 1.0).abs() <= 1e-12,
                "orientation not unit: {length}"
            );
        }
        assert_ne!(galaxy_at(1, 0.5), galaxy_at(2, 0.5));
    }

    #[test]
    fn star_layouts_are_deterministic_and_bounded() {
        for galaxy_type in [
            GalaxyType::Spiral,
            GalaxyType::Elliptical,
            GalaxyType::Irregular,
        ] {
            let galaxy = test_galaxy(galaxy_type);
            let first = spiral_star_positions(11, &galaxy, 64);
            assert_eq!(first, spiral_star_positions(11, &galaxy, 64));
            assert_eq!(first.len(), 64);
            for star in &first {
                assert!(
                    (MIN_STAR_MASS..=MAX_STAR_MASS).contains(&star.mass_solar),
                    "mass out of range: {}",
                    star.mass_solar
                );
                assert!(
                    star.position.iter().all(|v| v.is_finite()),
                    "non-finite star position"
                );
            }
        }
        // Degenerate galaxies collapse gracefully instead of producing NaN.
        let collapsed = Galaxy {
            galaxy_type: GalaxyType::Spiral,
            size: 0.0,
            orientation: [0.0, 0.0, 0.0],
        };
        for star in spiral_star_positions(5, &collapsed, 16) {
            assert!(star.position.iter().all(|v| v.is_finite()));
        }
    }

    #[test]
    fn star_count_is_stable_across_seeds() {
        let galaxy = test_galaxy(GalaxyType::Spiral);
        for seed in [7u64, 8, 9] {
            for count in [0u32, 1, 100] {
                assert_eq!(
                    spiral_star_positions(seed, &galaxy, count).len(),
                    count as usize
                );
            }
        }
        assert!(spiral_star_positions(7, &galaxy, 0).is_empty());
    }

    #[test]
    fn octree_leaves_hold_at_most_1000_objects() {
        let mut rng = Rng::new(1_234);
        let mut objects = Vec::with_capacity(2_500);
        for _ in 0..2_500 {
            objects.push([
                rng.next_f64() - 0.5,
                rng.next_f64() - 0.5,
                rng.next_f64() - 0.5,
            ]);
        }
        let tree = Octree::build(777, [0.0, 0.0, 0.0], 0.5, objects);
        assert!(!tree.root.is_leaf());
        assert_eq!(tree.total_objects(), 2_500);
        for leaf in tree.leaf_nodes() {
            assert!(leaf.is_leaf());
            assert!(leaf.objects.len() <= MAX_OBJECTS_PER_LEAF);
        }
        // Small inputs stay in a single leaf.
        let small = Octree::build(777, [0.0, 0.0, 0.0], 0.5, vec![[0.1, 0.2, 0.3]]);
        assert!(small.root.is_leaf());
        assert_eq!(small.total_objects(), 1);
    }

    #[test]
    fn octree_seeds_derive_via_hash_cell() {
        let mut rng = Rng::new(99);
        let objects: Vec<[f64; 3]> = (0..1_500)
            .map(|_| {
                [
                    rng.next_f64() - 0.5,
                    rng.next_f64() - 0.5,
                    rng.next_f64() - 0.5,
                ]
            })
            .collect();
        let root_seed = 0xBEEF_u64;
        let first = Octree::build(root_seed, [0.0, 0.0, 0.0], 0.5, objects.clone());
        assert_eq!(
            first,
            Octree::build(root_seed, [0.0, 0.0, 0.0], 0.5, objects)
        );
        let children = first.root.children.as_ref().expect("split root");
        assert_eq!(children[0].seed, hash_cell(root_seed, 6, 0, 0, 0));
        assert_eq!(children[7].seed, hash_cell(root_seed, 6, 1, 1, 1));
        assert_ne!(children[0].seed, children[7].seed);
    }

    #[test]
    fn galaxy_children_respect_parent() {
        let generator = GalaxyGenerator::new(Level::new(5).expect("L5"), false);
        let parent = parent_constraints();
        let out = generator.generate(21, &parent);
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
        let bad = Constraints {
            density_multiplier: 2.0,
            min_count: 5,
            max_count: 2,
            allowed_extent: [0.5; 3],
        };
        assert!(generator.generate(21, &bad).points.is_empty());
    }
}
