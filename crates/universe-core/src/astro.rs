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
use crate::r#gen::{
    CloudInfo, Constraints, GalaxyInfo, GalaxyKind, Generated, Generator, MarkerKind, Point,
    StarInfo, StarKind,
};
use crate::nest::autopilot_marker;
use crate::seed::{Rng, binomial_draw, hash_cell, hash_triple};
use crate::system::{
    ALPHA_CEN_TRIPLE, CLOUD_STREAM_TAG, OORT_STREAM_TAG, PLANET_STREAM_TAG, roll_companions,
    sample_cloud_mass, sample_star, system_planet, system_star,
};
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
fn spiral_disk_point(rng: &mut Rng, size: f64, barred: bool) -> [f64; 3] {
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

/// Contract-level shape summary of a sampled `galaxy` (#154, data only).
///
/// Counts, kinds, and portal order are untouched; rendering ignores the
/// summary until #157. `far_view` comes from the [`FAR_VIEW_SIZE`] gate.
fn galaxy_info(galaxy: &Galaxy, far_view: bool) -> GalaxyInfo {
    GalaxyInfo {
        kind: match galaxy.galaxy_type {
            GalaxyType::Spiral => GalaxyKind::Spiral,
            GalaxyType::Elliptical => GalaxyKind::Elliptical,
            GalaxyType::Irregular => GalaxyKind::Irregular,
        },
        orientation: galaxy.orientation,
        barred: galaxy.barred,
        far_view,
    }
}

/// Reference generator scattering galaxy indicators.
///
/// Each point samples the [`density_at`] field at its position and draws a
/// [`Galaxy`] from that density; the indicator radius is the galaxy size, so
/// dense regions read as clusters of large markers. At L4 every point also
/// carries its shape summary ([`GalaxyInfo`]); other levels keep sizes only.
/// Placement and sizes are uniform per the parent budget; the per-level
/// portal/ population split of #151 (counts, kinds, rich/poor) is assigned
/// after sampling, so this generator never decides what opens. Each of the 8
/// child octants receives half the parent density and half the count
/// ceiling, so [`respects`](crate::gen::respects) holds for every child. An
/// invalid parent yields empty output rather than panicking.
///
/// The home L4 cell (`home`, set by the ladder from the journey chain)
/// forces the journey's pick to the barred Milky Way at a fixed tilt (#154
/// Q5): position and size stay sampled, so rank order, counts, portal
/// mapping, and the downstream journey never move. Other cells sample every
/// shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GalaxyGenerator {
    /// Level this generator is fixed to (drives the #151 kind model).
    pub level: Level,
    /// Rich-cluster content (L4 Virgo-analogs: thousands of members).
    pub rich: bool,
    /// Milky Way fixture for the home L4 cell (ignored elsewhere).
    pub home: bool,
    /// Home-system fixture for L6-L9 cells on the journey path (#155 Q4).
    ///
    /// Fixes the Alpha Centauri triple at L6, the Sun at L7, and the Solar
    /// catalog planets at L8/L9; ignored off the home path.
    pub system_home: bool,
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
    /// content at L4, ignored elsewhere; `home` marks the Milky Way cell;
    /// `system_home` marks L6-L9 cells on the journey path).
    #[must_use]
    pub const fn new(level: Level, rich: bool, home: bool, system_home: bool) -> GalaxyGenerator {
        GalaxyGenerator {
            level,
            rich,
            home,
            system_home,
        }
    }

    /// Point total for the level: rich L4 cells hold thousands, L9 holds
    /// its planets (the rare companion appends separately), everything else
    /// fills the era budget. The caller caps against `parent.max_count`.
    fn model_total(&self, seed: u64) -> u32 {
        match self.level.get() {
            4 if self.rich => RICH_CLUSTER_TOTAL,
            9 => Self::l9_counts(seed).0,
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

    /// Arm frame for L5 clouds: disk normal plus bar flag (#155 T3).
    ///
    /// One cell, one frame: orientation uniform on the sphere and an
    /// even bar coin from a fixed stream lane, so every cloud in the cell
    /// shares the galaxy plane the #154 layouts define. The host type is
    /// unknown this deep in the contract (no parent shape crosses it), so
    /// every L5 cell reads as a star-forming spiral by documented design.
    fn l5_arm_frame(seed: u64) -> ([f64; 3], bool) {
        let mut rng = Rng::new(hash_triple(seed, CLOUD_STREAM_TAG, u64::MAX));
        let z = rng.next_f64() * 2.0 - 1.0;
        let angle = rng.next_f64() * 2.0 * PI;
        let ring = (1.0 - z * z).max(0.0).sqrt();
        let barred = rng.next_f64() < 0.5;
        ([ring * angle.cos(), ring * angle.sin(), z], barred)
    }

    /// Aligns L5 clouds to the cell arm frame with spectrum masses (#155 T3).
    ///
    /// Positions follow the shared spiral layout (bulge, bar, two arms over
    /// an exponential profile) scaled to the allowed extent; sizes keep
    /// their sampled values, so the portal prefix and the journey never
    /// move. Every cloud carries its log-uniform mass for #157.
    fn attach_l5_clouds(seed: u64, parent: &Constraints, points: &mut [Point]) {
        let (orientation, barred) = Self::l5_arm_frame(seed);
        let normal = unit_or_default(orientation);
        let tangent = orthonormal_tangent(normal);
        let bitangent = cross(normal, tangent);
        let floor = parent
            .allowed_extent
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let size = (floor * 0.5).max(1e-9);
        for (index, point) in points.iter_mut().enumerate() {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: cell totals stay far below u32::MAX by budget"
            )]
            let lane = index as u64;
            let mut stream = Rng::new(hash_triple(seed, CLOUD_STREAM_TAG, lane));
            let disk = spiral_disk_point(&mut stream, size, barred);
            point.position = [
                tangent[0] * disk[0] + bitangent[0] * disk[1] + normal[0] * disk[2],
                tangent[1] * disk[0] + bitangent[1] * disk[1] + normal[1] * disk[2],
                tangent[2] * disk[0] + bitangent[2] * disk[1] + normal[2] * disk[2],
            ];
            point.cloud = Some(CloudInfo {
                mass_solar: sample_cloud_mass(&mut stream),
            });
        }
    }

    /// Stratifies L7 populations into Oort shells around the star (#155 T3).
    ///
    /// One inner shell in [0.04, 0.16] of the cell extent (the 2,000-5,000
    /// AU heart) and one outer shell in [0.3, 1.0] (out to the 100,000 AU
    /// edge), directions uniform on the sphere from per-point lanes. Counts
    /// and the centered star never move; shells are populations that never
    /// open.
    fn layout_oort_shells(seed: u64, parent: &Constraints, points: &mut [Point]) {
        let floor = parent
            .allowed_extent
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        for (index, point) in points.iter_mut().enumerate().skip(1) {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: cell totals stay far below u32::MAX by budget"
            )]
            let lane = index as u64;
            let mut stream = Rng::new(hash_triple(seed, OORT_STREAM_TAG, lane));
            let (low, high) = if stream.next_f64() < 0.2 {
                (0.04, 0.16)
            } else {
                (0.3, 1.0)
            };
            let radius = (low + stream.next_f64() * (high - low)) * floor;
            let z = stream.next_f64() * 2.0 - 1.0;
            let angle = stream.next_f64() * 2.0 * PI;
            let ring = (1.0 - z * z).max(0.0).sqrt();
            point.position = [
                ring * angle.cos() * radius,
                ring * angle.sin() * radius,
                z * radius,
            ];
        }
    }

    /// Attaches star data and companions to L6 systems (#155 T2).
    ///
    /// Every system carries class plus banded mass from its own stream lane,
    /// so the position/size stream never shifts. Portal systems roll
    /// companions from the observed fractions; companions append as
    /// populations beside their host and never open. The home cell fixes the
    /// journey pick to Alpha Centauri A and appends B plus Proxima. Assembly
    /// keeps the portal prefix first, then companions, then populations
    /// trimmed to the same total, so portal indices and the journey never
    /// move.
    fn attach_l6_systems(
        seed: u64,
        parent: &Constraints,
        system_home: bool,
        portals: usize,
        points: &mut Vec<Point>,
    ) {
        for (index, point) in points.iter_mut().enumerate() {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: cell totals stay far below u32::MAX by budget"
            )]
            let lane = index as u64;
            let mut stream = Rng::new(hash_triple(seed, crate::system::STAR_STREAM_TAG, lane));
            point.star = Some(sample_star(&mut stream));
        }
        let mut extras: Vec<Point> = Vec::new();
        if system_home {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: portal prefix stays far below u32::MAX by budget"
            )]
            let orders: Vec<u32> = (0..portals as u32).collect();
            if let Some(pick) = autopilot_marker(seed, &orders)
                && let Some(primary) = points.get_mut(pick as usize)
            {
                primary.star = Some(StarInfo {
                    kind: ALPHA_CEN_TRIPLE[0].0,
                    mass_solar: ALPHA_CEN_TRIPLE[0].1,
                });
                for (member, nudge) in [(1usize, [0.02, 0.0, 0.0]), (2usize, [-0.015, 0.01, 0.0])]
                {
                    let mut position = primary.position;
                    for ((slot, shift), extent) in position
                        .iter_mut()
                        .zip(nudge.iter())
                        .zip(parent.allowed_extent.iter())
                    {
                        *slot = (*slot + shift).clamp(-extent, *extent);
                    }
                    extras.push(Point {
                        position,
                        radius: (primary.radius * 0.5).max(1e-4),
                        kind: MarkerKind::Population,
                        galaxy: None,
                        star: Some(StarInfo {
                            kind: ALPHA_CEN_TRIPLE[member].0,
                            mass_solar: ALPHA_CEN_TRIPLE[member].1,
                        }),
                        planet: None,
                        cloud: None,
                    });
                }
            }
        } else {
            for index in 0..portals {
                let (position, radius, star) = match points.get(index) {
                    Some(point) => (point.position, point.radius, point.star),
                    None => break,
                };
                let class = star.map(|info| info.kind).unwrap_or(StarKind::G);
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "E-CAST: cell totals stay far below u32::MAX by budget"
                )]
                let lane = index as u64;
                let mut stream = Rng::new(hash_triple(seed, PLANET_STREAM_TAG, lane));
                let mut remaining = roll_companions(&mut stream, class);
                while remaining > 0 {
                    remaining -= 1;
                    let mut position = position;
                    for (slot, extent) in
                        position.iter_mut().zip(parent.allowed_extent.iter())
                    {
                        let shift = (stream.next_f64() * 2.0 - 1.0) * radius;
                        *slot = (*slot + shift).clamp(-extent, *extent);
                    }
                    extras.push(Point {
                        position,
                        radius: (radius * 0.5).max(1e-4),
                        kind: MarkerKind::Population,
                        galaxy: None,
                        star: Some(sample_star(&mut stream)),
                        planet: None,
                        cloud: None,
                    });
                }
            }
        }
        let cap = points.len();
        let keep_companions = extras.len().min(cap.saturating_sub(portals));
        let mut folded = Vec::with_capacity(cap);
        folded.extend(points.iter().take(portals).copied());
        folded.extend(extras.iter().take(keep_companions).copied());
        let room = cap.saturating_sub(folded.len());
        folded.extend(points.iter().skip(portals).take(room).copied());
        *points = folded;
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
        let mut samples: Vec<([f64; 3], Galaxy)> = Vec::with_capacity(total);
        for _ in 0..total {
            let mut position = [0.0; 3];
            for (slot, extent) in position.iter_mut().zip(parent.allowed_extent.iter()) {
                *slot = (rng.next_f64() * 2.0 - 1.0) * extent;
            }
            let density = density_at(seed, position[0], position[1], position[2]);
            let galaxy = sample_galaxy(&mut rng, density);
            samples.push((position, galaxy));
        }
        // Largest first (stable: ties keep sample order, so rebuilds agree
        // bit for bit); the portal prefix then doubles as the tier order.
        samples.sort_by(|a, b| b.1.size.total_cmp(&a.1.size));
        let portals = self.portal_count(seed, total);
        let mut points = Vec::with_capacity(total);
        for (order, (position, galaxy)) in samples.into_iter().enumerate() {
            let kind = if order < portals {
                MarkerKind::Portal
            } else {
                MarkerKind::Population
            };
            let info = if self.level.get() == 4 {
                let far = galaxy.size < FAR_VIEW_SIZE;
                let mut info = galaxy_info(&galaxy, far);
                if far {
                    // Impostor record past the gate: position, size, and
                    // type only. Layout detail resolves when the dive comes
                    // closer; #157 renders the flag.
                    info.orientation = [0.0, 0.0, 1.0];
                    info.barred = false;
                }
                Some(info)
            } else {
                None
            };
            points.push(Point {
                position,
                radius: galaxy.size,
                kind,
                galaxy: info,
                star: None,
                planet: None,
                cloud: None,
            });
        }
        // Milky Way home portal (#154 Q5): the journey's pick in the home
        // cell is the barred Milky Way at a fixed tilt. Position and size
        // stay sampled, so rank order, counts, portal mapping, and the
        // downstream journey never move; only the shape is fixed. The pick
        // rule is shared with the autopilot (`autopilot_marker`), never
        // reimplemented here.
        if self.home && self.level.get() == 4 && !self.rich {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: portal prefix stays far below u32::MAX by budget"
            )]
            let orders: Vec<u32> = (0..portals as u32).collect();
            if let Some(pick) = autopilot_marker(seed, &orders)
                && let Some(point) = points.get_mut(pick as usize)
                && let Some(galaxy) = point.galaxy.as_mut()
            {
                galaxy.kind = GalaxyKind::Spiral;
                galaxy.barred = true;
                galaxy.orientation = MILKY_WAY_ORIENTATION;
                galaxy.far_view = false;
            }
        }
        // L7/L8 are entered through their star: it sits at the cell center
        // as the reference point (#151; physical layouts land in #155).
        if (self.level.get() == 7 || self.level.get() == 8)
            && let Some(star) = points.first_mut()
        {
            star.position = [0.0, 0.0, 0.0];
        }
        // Star and planet data (#155 T1-T2): companions, planets, and orbits
        // ride on the sorted samples without moving them. Counts follow the
        // existing portal prefix, portal indices never shift, and journey
        // picks never change: every new point is appended as a population
        // and populations trim to the same total.
        match self.level.get() {
            5 => Self::attach_l5_clouds(seed, parent, &mut points),
            6 => Self::attach_l6_systems(seed, parent, self.system_home, portals, &mut points),
            7 => {
                if let Some(star) = points.first_mut() {
                    star.star = Some(if self.system_home {
                        StarInfo {
                            kind: StarKind::G,
                            mass_solar: 1.0,
                        }
                    } else {
                        let mut stream =
                            Rng::new(hash_triple(seed, crate::system::STAR_STREAM_TAG, 0));
                        sample_star(&mut stream)
                    });
                }
                Self::layout_oort_shells(seed, parent, &mut points);
            }
            8 => {
                let system_seed = hash_cell(seed, 9, 0, 0, 0);
                let host = system_star(system_seed, self.system_home);
                if let Some(star) = points.first_mut() {
                    star.star = Some(host);
                }
                let count = Self::planet_count(system_seed) as usize;
                for (slot, point) in points.iter_mut().skip(1).take(count).enumerate() {
                    point.planet = Some(system_planet(
                        system_seed,
                        slot,
                        count,
                        host.mass_solar,
                        self.system_home,
                    ));
                }
            }
            9 => {
                let (planets, companions) = Self::l9_counts(seed);
                let host = system_star(seed, self.system_home);
                let count = planets as usize;
                for (slot, point) in points.iter_mut().take(count).enumerate() {
                    point.planet = Some(system_planet(
                        seed,
                        slot,
                        count,
                        host.mass_solar,
                        self.system_home,
                    ));
                }
                if companions > 0 && !points.is_empty() {
                    let mut stream =
                        Rng::new(hash_triple(seed, PLANET_STREAM_TAG, u64::MAX));
                    let mut position = [0.0; 3];
                    for (slot, extent) in
                        position.iter_mut().zip(parent.allowed_extent.iter())
                    {
                        *slot = (stream.next_f64() * 2.0 - 1.0) * extent;
                    }
                    let floor = points
                        .iter()
                        .map(|point| point.radius)
                        .fold(f64::INFINITY, f64::min);
                    points.push(Point {
                        position,
                        radius: (floor * 0.5).max(1e-4),
                        kind: MarkerKind::Population,
                        galaxy: None,
                        star: Some(sample_star(&mut stream)),
                        planet: None,
                        cloud: None,
                    });
                }
            }
            _ => {}
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
    fn galaxy_children_respect_parent() {
        let generator = GalaxyGenerator::new(Level::new(5).expect("L5"), false, false, false);
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

    #[test]
    fn far_view_gate_degrades_small_galaxies_deterministically() {
        let level = Level::new(4).expect("L4");
        let parent = parent_constraints();
        let first = GalaxyGenerator::new(level, false, false, false).generate(99, &parent);
        let second = GalaxyGenerator::new(level, false, false, false).generate(99, &parent);
        assert_eq!(first, second, "same seed must replay bit for bit");
        let mut far = 0u32;
        let mut near = 0u32;
        for seed in 1..=40u64 {
            let out = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            for point in &out.points {
                let g = point.galaxy.expect("L4 carries shape data");
                if g.far_view {
                    far += 1;
                    assert!(point.radius < FAR_VIEW_SIZE);
                    assert_eq!(g.orientation, [0.0, 0.0, 1.0]);
                    assert!(!g.barred);
                } else {
                    near += 1;
                }
            }
        }
        let share = f64::from(far) / f64::from(far + near);
        assert!(
            (0.03..0.35).contains(&share),
            "far share out of band: {share}"
        );
    }

    #[test]
    fn star_data_rides_l6_systems_and_central_stars() {
        use crate::r#gen::StarKind;
        let parent = parent_constraints();
        // L6: every system carries class plus banded mass, deterministically.
        let level = Level::new(6).expect("L6");
        let first = GalaxyGenerator::new(level, false, false, false).generate(4242, &parent);
        let second = GalaxyGenerator::new(level, false, false, false).generate(4242, &parent);
        assert_eq!(first, second, "L6 star data must replay bit for bit");
        assert!(!first.points.is_empty(), "L6 keeps its systems");
        for point in &first.points {
            let star = point.star.expect("every L6 system is a star");
            assert!(star.mass_solar >= MIN_STAR_MASS, "mass below ignition");
            assert!(point.planet.is_none() && point.cloud.is_none());
        }
        // L7/L8: the centered star carries data, shells and belt do not.
        for level_n in [7u8, 8] {
            let level = Level::new(level_n).expect("ladder level");
            let out = GalaxyGenerator::new(level, false, false, false).generate(4242, &parent);
            let head = out.points.first().expect("L7/L8 keep their star");
            assert_eq!(head.position, [0.0, 0.0, 0.0], "star stays centered");
            head.star.expect("central star carries data");
            assert!(
                out.points.iter().skip(1).all(|point| point.star.is_none()),
                "L{level_n} shells and belt stay bare"
            );
        }
        // L4/L5 carry no star data (clouds land in T3).
        for level_n in [4u8, 5] {
            let level = Level::new(level_n).expect("ladder level");
            let out = GalaxyGenerator::new(level, false, false, false).generate(4242, &parent);
            assert!(
                out.points.iter().all(|point| point.star.is_none()),
                "L{level_n} stays star-free in T1"
            );
        }
        // M dwarfs dominate the L6 population across cells.
        let mut dwarfs = 0u32;
        let mut total = 0u32;
        for seed in 0..200u64 {
            let out = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            for point in &out.points {
                total += 1;
                dwarfs += u32::from(point.star.expect("L6 star").kind == StarKind::M);
            }
        }
        let share = f64::from(dwarfs) / f64::from(total);
        assert!(
            (0.60..0.90).contains(&share),
            "L6 M share out of band: {share}"
        );
    }

    #[test]
    fn l8_planets_carry_valley_radii_and_kepler_periods() {
        use crate::system::{SOLAR_AXES_AU, SOLAR_RADII_EARTH};
        let parent = parent_constraints();
        let level = Level::new(8).expect("L8");
        for seed in [11u64, 4242, 99_999] {
            let out =
                GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            let host = out.points.first().expect("L8 keeps its star");
            let host_mass = host.star.expect("L8 star").mass_solar;
            let mut seen = 0u32;
            for planet in out.points.iter().skip(1) {
                if planet.kind != MarkerKind::Portal {
                    break;
                }
                seen += 1;
                let data = planet.planet.expect("L8 planet portal carries data");
                assert!(
                    !(1.5..=2.0).contains(&data.radius_earth),
                    "valley filled: {}",
                    data.radius_earth
                );
                assert!(
                    (0.39..=30.1).contains(&data.orbit_au),
                    "orbit outside template: {}",
                    data.orbit_au
                );
                let check = data.period_days * data.period_days
                    / (data.orbit_au * data.orbit_au * data.orbit_au);
                let expect = 365.25 * 365.25 / host_mass;
                assert!(
                    (check / expect - 1.0).abs() < 1e-9,
                    "Kepler broke: {check} vs {expect}"
                );
            }
            assert!(seen > 0, "seed {seed} holds no planets");
        }
        // Home cells read the catalog through the same slots.
        let home = GalaxyGenerator::new(level, false, false, true).generate(7, &parent);
        let system_seed = hash_cell(7, 9, 0, 0, 0);
        let count = GalaxyGenerator::planet_count(system_seed) as usize;
        let catalog: Vec<_> = home.points.iter().skip(1).take(count).collect();
        assert_eq!(catalog.len(), count, "home planets attach in order");
        for (slot, point) in catalog.iter().enumerate() {
            assert_eq!(point.kind, MarkerKind::Portal, "slot {slot} opens");
            let data = point.planet.expect("home planet carries data");
            if count == SOLAR_AXES_AU.len() {
                assert!(
                    (data.orbit_au - SOLAR_AXES_AU[slot]).abs() < 1e-12,
                    "slot {slot} axis drifted: {}",
                    data.orbit_au
                );
                assert!(
                    (data.radius_earth - SOLAR_RADII_EARTH[slot]).abs() < 1e-12,
                    "slot {slot} radius drifted: {}",
                    data.radius_earth
                );
            }
        }
        let sun = home.points.first().expect("home star").star.expect("Sun");
        assert_eq!(sun.mass_solar, 1.0, "home star is one solar mass");
    }

    #[test]
    fn l9_planets_match_l8_data_through_the_shared_seed() {
        let parent = parent_constraints();
        let l8 = Level::new(8).expect("L8");
        let l9 = Level::new(9).expect("L9");
        for seed in [11u64, 4242, 99_999] {
            let system_seed = hash_cell(seed, 9, 0, 0, 0);
            let eight =
                GalaxyGenerator::new(l8, false, false, false).generate(seed, &parent);
            let nine =
                GalaxyGenerator::new(l9, false, false, false).generate(system_seed, &parent);
            let eight_data: Vec<_> = eight
                .points
                .iter()
                .skip(1)
                .filter(|point| point.kind == MarkerKind::Portal)
                .filter_map(|point| point.planet)
                .collect();
            let nine_data: Vec<_> = nine
                .points
                .iter()
                .filter(|point| point.kind == MarkerKind::Portal)
                .filter_map(|point| point.planet)
                .collect();
            assert_eq!(
                eight_data.len(),
                nine_data.len(),
                "seed {seed} planet counts disagree"
            );
            assert_eq!(eight_data, nine_data, "seed {seed} planet data disagrees");
        }
    }

    #[test]
    fn l9_companion_appends_as_a_single_population() {
        let parent = parent_constraints();
        let level = Level::new(9).expect("L9");
        let mut probed = 0u32;
        for seed in 0..200u64 {
            let out =
                GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            let (planets, companions) = GalaxyGenerator::l9_counts(seed);
            let portal_data = out
                .points
                .iter()
                .filter(|point| point.kind == MarkerKind::Portal)
                .count();
            assert_eq!(
                portal_data,
                planets as usize,
                "seed {seed} portal count moved"
            );
            let populations: Vec<_> = out
                .points
                .iter()
                .filter(|point| point.kind == MarkerKind::Population)
                .collect();
            assert_eq!(
                populations.len(),
                companions as usize,
                "seed {seed} companion drift"
            );
            for companion in populations {
                companion.star.expect("companion is a star");
                assert!(companion.planet.is_none(), "companion carries no planet");
            }
            if companions > 0 {
                probed += 1;
                let tail = out.points.last().expect("companion appends last");
                assert_eq!(tail.kind, MarkerKind::Population);
                tail.star.expect("appended companion is a star");
            }
        }
        assert!(probed > 10, "too few companion cells probed: {probed}");
    }

    #[test]
    fn l6_companions_append_as_populations_within_the_cap() {
        let parent = parent_constraints();
        let level = Level::new(6).expect("L6");
        let cap = parent.max_count as usize;
        let mut probed = 0u32;
        for seed in 0..300u64 {
            let first =
                GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            let second =
                GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            assert_eq!(first, second, "seed {seed} L6 must replay");
            assert!(first.points.len() <= cap, "seed {seed} over the cap");
            let portals = first
                .points
                .iter()
                .filter(|point| point.kind == MarkerKind::Portal)
                .count();
            assert!((1..=2).contains(&portals), "seed {seed} portal count moved");
            for (index, point) in first.points.iter().enumerate() {
                if index < portals {
                    assert_eq!(point.kind, MarkerKind::Portal, "prefix broke");
                }
                if point.kind == MarkerKind::Population && point.star.is_some() {
                    probed += 1;
                }
            }
        }
        assert!(probed > 10, "too few companion points probed: {probed}");
    }

    #[test]
    fn l6_portal_counts_follow_the_poisson_mean_with_a_floor() {
        // Binomial(1000, 3/10000) is the integer-only Poisson(0.3): mean 0.3
        // systems per 4.37-ly cell, floored at one portal so the dive never
        // strands (Spec v1 Q3). The packing cap only binds above two.
        let parent = parent_constraints();
        let level = Level::new(6).expect("L6");
        let mut ones = 0u32;
        let mut twos = 0u32;
        for seed in 0..2000u64 {
            let out =
                GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            let portals = out
                .points
                .iter()
                .filter(|point| point.kind == MarkerKind::Portal)
                .count();
            match portals {
                1 => ones += 1,
                2 => twos += 1,
                other => panic!("L6 portal count broke: {other}"),
            }
        }
        let share = f64::from(twos) / 2000.0;
        assert!(
            (0.01..0.10).contains(&share),
            "L6 two-system share out of band: {share}"
        );
        assert!(ones > twos * 10, "L6 usually holds one system");
    }

    #[test]
    fn l5_clouds_align_to_thin_arms_with_spectrum_masses() {
        let parent = parent_constraints();
        let level = Level::new(5).expect("L5");
        let extent = parent.allowed_extent[0];
        for seed in [11u64, 4242, 99_999] {
            let first =
                GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            let second =
                GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            assert_eq!(first, second, "seed {seed} clouds must replay");
            let (orientation, _) = GalaxyGenerator::l5_arm_frame(seed);
            let length = (orientation[0] * orientation[0]
                + orientation[1] * orientation[1]
                + orientation[2] * orientation[2])
                .sqrt();
            assert!((length - 1.0).abs() < 1e-12, "arm frame not unit");
            let mut thin = 0u32;
            for point in &first.points {
                let mass = point.cloud.expect("every L5 cloud carries mass").mass_solar;
                assert!(
                    (10.0..=10_000_000.0).contains(&mass),
                    "cloud mass out of spectrum: {mass}"
                );
                assert!(point.star.is_none() && point.planet.is_none());
                for (axis, limit) in parent.allowed_extent.iter().enumerate() {
                    assert!(
                        point.position[axis].abs() <= *limit,
                        "cloud outside extent"
                    );
                }
                let height = (point.position[0] * orientation[0]
                    + point.position[1] * orientation[1]
                    + point.position[2] * orientation[2])
                    .abs();
                thin += u32::from(height < 0.1 * extent);
            }
            let share = f64::from(thin) / f64::from(first.points.len() as u32);
            assert!(
                share > 0.5,
                "seed {seed} clouds not thin about the arms: {share}"
            );
        }
    }

    #[test]
    fn l7_shells_stratify_into_inner_and_outer_bands() {
        let parent = parent_constraints();
        let level = Level::new(7).expect("L7");
        let extent = parent.allowed_extent[0];
        for seed in [11u64, 4242, 99_999] {
            let first =
                GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            let second =
                GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            assert_eq!(first, second, "seed {seed} shells must replay");
            assert_eq!(first.points.len(), 32, "shell count never moves");
            let head = first.points.first().expect("L7 keeps its star");
            assert_eq!(head.position, [0.0, 0.0, 0.0]);
            head.star.expect("central star carries data");
            let mut inner = 0u32;
            for point in first.points.iter().skip(1) {
                assert_eq!(point.kind, MarkerKind::Population, "shells never open");
                assert!(point.star.is_none() && point.planet.is_none());
                let radius = (point.position[0] * point.position[0]
                    + point.position[1] * point.position[1]
                    + point.position[2] * point.position[2])
                    .sqrt();
                assert!(radius <= extent, "shell outside the cell: {radius}");
                if radius < 0.16 * extent {
                    inner += 1;
                    assert!(radius >= 0.04 * extent, "inner shell too deep: {radius}");
                } else {
                    assert!(
                        (0.3 * extent..=extent).contains(&radius),
                        "outer shell out of band: {radius}"
                    );
                }
            }
            assert!(
                (1..=12).contains(&inner),
                "seed {seed} inner shell count broke: {inner}"
            );
        }
    }

    #[test]
    fn milky_way_fixture_fixes_the_journey_pick_shape_only() {
        use crate::nest::{autopilot_candidates, autopilot_marker};
        let level = Level::new(4).expect("L4");
        let parent = parent_constraints();
        for seed in [7u64, 42, 12345] {
            let plain = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            let home = GalaxyGenerator::new(level, false, true, false).generate(seed, &parent);
            assert_eq!(plain.points.len(), home.points.len());
            let candidates = autopilot_candidates(level, seed, &home.points);
            let pick = autopilot_marker(seed, &candidates).expect("L4 has portals");
            assert_eq!(
                home.points[pick as usize].kind,
                MarkerKind::Portal,
                "journey pick must open"
            );
            for (index, (a, b)) in plain.points.iter().zip(home.points.iter()).enumerate() {
                assert_eq!(a.position, b.position, "seed {seed} positions move");
                assert_eq!(a.radius, b.radius, "seed {seed} sizes move");
                assert_eq!(a.kind, b.kind, "seed {seed} kinds move");
                if index == pick as usize {
                    let g = b.galaxy.expect("L4 carries shape data");
                    assert_eq!(g.kind, GalaxyKind::Spiral);
                    assert!(g.barred, "Milky Way is barred");
                    assert_eq!(g.orientation, MILKY_WAY_ORIENTATION);
                    assert_ne!(a.galaxy, b.galaxy, "fixture must change the pick");
                } else {
                    assert_eq!(a.galaxy, b.galaxy, "neighbours must not move");
                }
            }
        }
    }
}
