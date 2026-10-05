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
//! [`GalaxyGenerator`](crate::sysgen::GalaxyGenerator) exposes galaxy indicators through the [`Generator`](crate::gen::Generator)
//! contract with child budgets that respect their parent.
//!
//! [`Generator`]: crate::gen::Generator

use crate::seed::{Rng, hash_cell};
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
///
/// Canonical value lives in [`crate::system`]; re-exported so existing
/// paths keep working.
pub use crate::system::MIN_STAR_MASS;

/// Highest stellar mass sampled, in solar masses (Salpeter-tail truncation).
///
/// Canonical value lives in [`crate::system`]; re-exported so existing
/// paths keep working.
pub use crate::system::MAX_STAR_MASS;

/// Domain-separation tag folded into star-layout streams.
const STAR_STREAM_TAG: u64 = 0x85EB_CA77_C2B2_AE63;

/// Share of the spiral roll band that draws a central bar (#154 Q5).
///
/// About two in three spirals are barred; the Milky Way is always barred
/// (forced, not drawn).
pub const BARRED_BAND_SHARE: f64 = 2.0 / 3.0;

/// Share of non-bulge disk draws laid along the bar of a barred spiral.
///
/// The rest trace the two log-spiral arms; the bulge ball takes its own
/// quarter first.
pub const BAR_DISK_SHARE: f64 = 0.3;

/// Fixed disk tilt of the Milky Way home portal (#154 Q5).
///
/// Unit by construction (`0.6^2 + 0.8^2 = 1`); procedural neighbours use
/// sampled orientations and never move it.
pub const MILKY_WAY_ORIENTATION: [f64; 3] = [0.0, 0.6, 0.8];

/// Galaxy size below which only the impostor record ships (#154 T3).
///
/// Cell-unit sizes proxy distance (smaller on screen means farther): below
/// this gate a galaxy carries position, size, and type only, with default
/// orientation and no bar. About one galaxy in six degrades, so generation
/// stays inside the frame budget while every near galaxy keeps full layout
/// detail. The render-side impostor that reads this flag lands in #157.
pub const FAR_VIEW_SIZE: f64 = 0.035;

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
    /// Central bar in the spiral layout (#154: most spirals, always the Milky Way).
    pub barred: bool,
}

/// Samples a galaxy from the local cosmic-web `density` in `[0.0, 1.0]`.
///
/// Ellipticals favor dense cluster cores while spirals dominate the field
/// (15% elliptical at zero density rising to 65% at full density, 10%
/// irregular throughout); about two in three spirals are barred
/// ([`BARRED_BAND_SHARE`] of the spiral roll band, no extra draw, so sizes
/// and orientations replay exactly as before); size is log-normal around
/// 0.05 cell units and the orientation is uniform on the sphere.
/// Non-finite densities read as the mid-grey `0.5`. Draws only from `rng`,
/// so a fresh stream replays the same galaxy every time.
pub fn sample_galaxy(rng: &mut Rng, density: f64) -> Galaxy {
    let clamped = if density.is_finite() {
        density.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let roll = rng.next_f64();
    let elliptical_edge = 0.15 + 0.5 * clamped;
    let galaxy_type = if roll < elliptical_edge {
        GalaxyType::Elliptical
    } else if roll < 0.9 {
        GalaxyType::Spiral
    } else {
        GalaxyType::Irregular
    };
    let barred = galaxy_type == GalaxyType::Spiral
        && roll < elliptical_edge + BARRED_BAND_SHARE * (0.9 - elliptical_edge);
    let bell = rng.next_f64() + rng.next_f64() + rng.next_f64() - 1.5;
    let size = 0.05 * 2.0f64.powf(bell);
    let z = rng.next_f64() * 2.0 - 1.0;
    let angle = rng.next_f64() * 2.0 * PI;
    let ring = (1.0 - z * z).max(0.0).sqrt();
    Galaxy {
        galaxy_type,
        size,
        orientation: [ring * angle.cos(), ring * angle.sin(), z],
        barred,
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
            GalaxyType::Spiral => spiral_disk_point(&mut rng, galaxy.size, galaxy.barred),
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

/// Draws one spiral-disk offset: bulge ball, bar segment, or an arm point.
pub(crate) fn spiral_disk_point(rng: &mut Rng, size: f64, barred: bool) -> [f64; 3] {
    let scale = (size * 0.33).max(1e-9);
    if rng.next_f64() < 0.25 {
        let spread = size.max(1e-9) * 0.15;
        return [
            gaussian(rng) * spread,
            gaussian(rng) * spread,
            gaussian(rng) * spread * 0.7,
        ];
    }
    if barred && rng.next_f64() < BAR_DISK_SHARE {
        // Central bar along local x: a uniform segment half the galaxy
        // across with a thin gaussian waist (the shared bar layout, #154).
        let half = (size * 0.5).max(1e-9);
        let waist = size.max(1e-9) * 0.06;
        return [
            (rng.next_f64() * 2.0 - 1.0) * half,
            gaussian(rng) * waist * 0.5,
            gaussian(rng) * waist * 0.5,
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
///
/// Canonical implementation lives in [`crate::system`]; this alias keeps
/// the galaxy layouts calling the same stream-aligned draw.
fn salpeter_mass(rng: &mut Rng) -> f64 {
    crate::system::salpeter_mass(rng)
}

/// Normalizes `v`, falling back to +z for zero or non-finite inputs.
pub(crate) fn unit_or_default(v: [f64; 3]) -> [f64; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length.is_finite() && length > 1e-12 {
        [v[0] / length, v[1] / length, v[2] / length]
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// Builds a unit vector perpendicular to the unit `normal`.
pub(crate) fn orthonormal_tangent(normal: [f64; 3]) -> [f64; 3] {
    let helper = if normal[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    unit_or_default(cross(helper, normal))
}

/// Cross product of two 3-vectors.
pub(crate) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
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

#[cfg(test)]
mod tests {
    use super::*;

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
            barred: galaxy_type == GalaxyType::Spiral,
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
            barred: true,
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
    fn bars_land_on_spirals_only_at_two_in_three() {
        let mut barred = 0u32;
        let mut spirals = 0u32;
        for stream in 0..300u64 {
            let galaxy = galaxy_at(stream, 0.5);
            if galaxy.galaxy_type == GalaxyType::Spiral {
                spirals += 1;
                barred += u32::from(galaxy.barred);
            } else {
                assert!(!galaxy.barred, "bar outside spirals");
            }
        }
        assert!(spirals > 150, "too few spirals to judge: {spirals}");
        let share = f64::from(barred) / f64::from(spirals);
        assert!(
            (0.55..0.80).contains(&share),
            "barred share out of band: {share}"
        );
    }

    #[test]
    fn barred_layouts_trace_a_thin_central_segment() {
        let size = 0.05;
        let barred = Galaxy {
            galaxy_type: GalaxyType::Spiral,
            size,
            orientation: [0.0, 0.0, 1.0],
            barred: true,
        };
        let stars = spiral_star_positions(9, &barred, 400);
        assert_eq!(stars.len(), 400);
        let bar_stars = stars
            .iter()
            .filter(|star| {
                star.position[0].abs() > size * 0.3
                    && star.position[1].abs() < size * 0.1
                    && star.position[2].abs() < size * 0.1
            })
            .count();
        assert!(bar_stars > 0, "no bar-segment stars in a barred spiral");
        let plain = Galaxy {
            barred: false,
            ..barred
        };
        assert_ne!(
            spiral_star_positions(9, &barred, 400),
            spiral_star_positions(9, &plain, 400),
            "bar flag must change the layout"
        );
    }
}
