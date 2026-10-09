//! L4-L9 indicator generator: systems, stars, planets, clouds (#155).
//!
//! [`GalaxyGenerator`](crate::sysgen::GalaxyGenerator) turns a cell seed into
//! the sorted portal/population indicators the dive opens: galaxy shapes at
//! L4 (owned by [`crate::astro`]), arm-aligned clouds at L5, stellar systems
//! at L6, Oort shells at L7, ordered planets at L8, and the star close-up at
//! L9. Sampling math for stars, planets, and clouds lives in
//! [`crate::system`]; this module owns counts, portal order, and the home
//! fixtures (Alpha Centauri triple, Solar eight).
//!
//! [`Generator`]: crate::gen::Generator

use crate::astro::{
    FAR_VIEW_SIZE, GREEN_VALLEY, Galaxy, GalaxyType, MILKY_WAY_ORIENTATION, cross, galaxy_tint,
    orthonormal_tangent, sample_galaxy, spiral_disk_point, unit_or_default,
};
use crate::coords::Level;
use crate::density::density_at;
use crate::r#gen::{
    CloudInfo, ColorInfo, Constraints, Form, GalaxyInfo, GalaxyKind, Generated, Generator,
    MarkerKind, Point, StarInfo, StarKind, clamp_to_sphere,
};
use crate::nest::autopilot_marker;
use crate::seed::{Rng, binomial_draw, hash_cell, hash_triple};
use crate::system::{
    ALPHA_CEN_TRIPLE, CLOUD_STREAM_TAG, PLANET_STREAM_TAG, roll_companions, sample_cloud_mass,
    sample_star, star_tint,
};
use std::f64::consts::PI;

/// Sun's-lane fraction of the disk radius holding a galaxy portal (#384).
///
/// The portal (the child cell) sits in the disk off-centre, the way the
/// Sun's lane sits a quarter of the way out in the Milky Way; opening a
/// galaxy lands in its lane, closing returns to the same lane.
pub const PORTAL_LANE_FRACTION: f64 = 0.26;

/// Form and portal offset for one L4 galaxy point (#384).
///
/// Spirals draw tilted disks with bars, ellipticals nested spheroids,
/// irregulars clumps, each at the sampled `size`; far-view impostors stay
/// dots with centred portals. Portals sit in the disk plane at
/// [`PORTAL_LANE_FRACTION`] of the body radius along the disk tangent, so
/// the child cell has an address inside its parent. Pure in its inputs.
fn galaxy_form_and_portal(
    summary: GalaxyInfo,
    size: f64,
    is_portal: bool,
) -> (Form, Option<[f64; 3]>) {
    if summary.far_view {
        return (Form::Dot, None);
    }
    let normal = unit_or_default(summary.orientation);
    let form = match summary.kind {
        GalaxyKind::Spiral => Form::Disk {
            normal,
            barred: summary.barred,
        },
        GalaxyKind::Elliptical => Form::Spheroid,
        GalaxyKind::Irregular => Form::Clump,
    };
    let portal = if is_portal {
        let tangent = orthonormal_tangent(normal);
        let lane = size * PORTAL_LANE_FRACTION;
        Some([tangent[0] * lane, tangent[1] * lane, tangent[2] * lane])
    } else {
        None
    };
    (form, portal)
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
/// test pins it. Recalibrated for the #384 path (first hit at probe time:
/// 32); the draw stays Binomial(64, 4/64), so global statistics are
/// unchanged, only which cells draw 8 moves.
const SALT_PLANETS: u64 = 32;

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

    /// Ring outer radius for a cloud of `mass_solar` (#384).
    ///
    /// Log-mapped over the sampled 10-to-10M-solar-mass spectrum: the
    /// smallest clouds read as dots, giant complexes as wide rings. Pure.
    fn cloud_ring_outer(mass_solar: f64) -> f64 {
        let span = (mass_solar.log10() - 1.0).clamp(0.0, 6.0) / 6.0;
        0.01 + 0.03 * span
    }

    /// Disk-frame scenery for one L5 cell: the disk plus its two arms (#384).
    ///
    /// Three population points at the cell centre sharing the cell arm
    /// frame, so the arms read before any cloud resolves. The caller splices
    /// them behind the portal prefix and trims populations to the same
    /// total, so portal indices and the journey never move.
    fn l5_scenery(seed: u64, parent: &Constraints) -> [Point; 3] {
        let (orientation, barred) = Self::l5_arm_frame(seed);
        let normal = unit_or_default(orientation);
        let floor = parent
            .allowed_extent
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let size = (floor * 0.5).max(1e-9);
        let disk = Point {
            position: [0.0, 0.0, 0.0],
            radius: size,
            kind: MarkerKind::Population,
            form: Form::Disk { normal, barred },
            ..Point::bare([0.0; 3], size, MarkerKind::Population)
        };
        let arm = |phase: f64| Point {
            position: [0.0, 0.0, 0.0],
            radius: size,
            kind: MarkerKind::Population,
            form: Form::Arm { normal, phase },
            ..Point::bare([0.0; 3], size, MarkerKind::Population)
        };
        [disk, arm(0.0), arm(PI)]
    }

    /// Aligns L5 clouds to the cell arm frame with spectrum masses (#155 T3).
    ///
    /// Positions follow the shared spiral layout (bulge, bar, two arms over
    /// an exponential profile) scaled to the allowed extent; the rank sort
    /// still runs on the sampled sizes, so the portal prefix and the journey
    /// never move. Every cloud carries its log-uniform mass for #157. Clouds
    /// draw rings in the disk plane sized by mass (#384).
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
            let lane = index as u64;
            let mut stream = Rng::new(hash_triple(seed, CLOUD_STREAM_TAG, lane));
            let disk = spiral_disk_point(&mut stream, size, barred);
            point.position = clamp_to_sphere([
                tangent[0] * disk[0] + bitangent[0] * disk[1] + normal[0] * disk[2],
                tangent[1] * disk[0] + bitangent[1] * disk[1] + normal[1] * disk[2],
                tangent[2] * disk[0] + bitangent[2] * disk[1] + normal[2] * disk[2],
            ]);
            let mass = sample_cloud_mass(&mut stream);
            point.cloud = Some(CloudInfo { mass_solar: mass });
            let outer = Self::cloud_ring_outer(mass);
            point.radius = outer;
            point.form = Form::Ring {
                normal,
                inner: outer * 0.7,
                outer,
            };
        }
    }

    /// Indicator radius for a star of `mass_solar` (#384).
    ///
    /// Main-sequence mass-radius rule (`R ~ M^0.8`) anchored at 0.02 cell
    /// units for one solar mass, clamped so dwarfs stay visible and giants
    /// fit the cell. Pure.
    fn star_radius(mass_solar: f64) -> f64 {
        (0.02 * mass_solar.powf(0.8)).clamp(0.005, 0.05)
    }

    /// Attaches star data and companions to L6 systems (#155 T2).
    ///
    /// Every system carries class plus banded mass from its own stream lane,
    /// so the position/size stream never shifts. Systems draw dots sized by
    /// mass in their class tint; companions append as populations beside
    /// their host with an orbit arc and never open (#384). The home cell
    /// fixes the journey pick to Alpha Centauri A and appends B plus
    /// Proxima. Assembly keeps the portal prefix first, then companions,
    /// then populations trimmed to the same total, so portal indices and
    /// the journey never move.
    fn attach_l6_systems(
        seed: u64,
        parent: &Constraints,
        system_home: bool,
        portals: usize,
        points: &mut Vec<Point>,
    ) {
        for (index, point) in points.iter_mut().enumerate() {
            let lane = index as u64;
            let mut stream = Rng::new(hash_triple(seed, crate::system::STAR_STREAM_TAG, lane));
            let info = sample_star(&mut stream);
            point.tint = Some(star_tint(info.kind));
            point.star = Some(info);
            point.radius = Self::star_radius(info.mass_solar);
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
                let [alpha_a, alpha_b, proxima] = ALPHA_CEN_TRIPLE;
                primary.star = Some(StarInfo {
                    kind: alpha_a.0,
                    mass_solar: alpha_a.1,
                });
                primary.tint = Some(star_tint(alpha_a.0));
                primary.radius = Self::star_radius(alpha_a.1);
                for ((kind, mass), nudge) in [alpha_b, proxima]
                    .into_iter()
                    .zip([[0.02, 0.0, 0.0], [-0.015, 0.01, 0.0]])
                {
                    let mut position = primary.position;
                    for ((slot, shift), extent) in position
                        .iter_mut()
                        .zip(nudge.iter())
                        .zip(parent.allowed_extent.iter())
                    {
                        *slot = (*slot + shift).clamp(-extent, *extent);
                    }
                    position = clamp_to_sphere(position);
                    extras.push(Point {
                        position: clamp_to_sphere(position),
                        radius: Self::star_radius(mass),
                        kind: MarkerKind::Population,
                        star: Some(StarInfo {
                            kind,
                            mass_solar: mass,
                        }),
                        tint: Some(star_tint(kind)),
                        form: Form::Arc {
                            normal: [0.0, 1.0, 0.0],
                            span: 1.2,
                        },
                        ..Point::bare([0.0; 3], 0.01, MarkerKind::Population)
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
                let lane = index as u64;
                let mut stream = Rng::new(hash_triple(seed, PLANET_STREAM_TAG, lane));
                let mut remaining = roll_companions(&mut stream, class);
                while remaining > 0 {
                    remaining -= 1;
                    let mut position = position;
                    for (slot, extent) in position.iter_mut().zip(parent.allowed_extent.iter()) {
                        let shift = (stream.next_f64() * 2.0 - 1.0) * radius;
                        *slot = (*slot + shift).clamp(-extent, *extent);
                    }
                    position = clamp_to_sphere(position);
                    let info = sample_star(&mut stream);
                    extras.push(Point {
                        position: clamp_to_sphere(position),
                        radius: Self::star_radius(info.mass_solar),
                        kind: MarkerKind::Population,
                        star: Some(info),
                        tint: Some(star_tint(info.kind)),
                        form: Form::Arc {
                            normal: [0.0, 1.0, 0.0],
                            span: 1.2,
                        },
                        ..Point::bare([0.0; 3], 0.01, MarkerKind::Population)
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
            // The sample lives at its clamped position: morphology is drawn
            // from the field where the point is drawn (#384).
            position = clamp_to_sphere(position);
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
            let tint = info.map(|summary| galaxy_tint(summary.kind));
            let (form, portal) = if self.level.get() == 4 {
                match info {
                    Some(summary) => {
                        galaxy_form_and_portal(summary, galaxy.size, kind == MarkerKind::Portal)
                    }
                    None => (Form::Dot, None),
                }
            } else {
                (Form::Dot, None)
            };
            points.push(Point {
                position,
                radius: galaxy.size,
                kind,
                galaxy: info,
                tint,
                form,
                portal,
                ..Point::bare([0.0; 3], galaxy.size, kind)
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
                point.tint = Some(ColorInfo {
                    red: GREEN_VALLEY[0],
                    green: GREEN_VALLEY[1],
                    blue: GREEN_VALLEY[2],
                    brightness: 1.0,
                });
                // The fixed shape needs its fixed form: recompute from the
                // catalog values, so the Milky Way draws its tilted barred
                // disk with the portal in the Sun's lane (#384).
                let (form, portal) =
                    galaxy_form_and_portal(*galaxy, point.radius, point.kind == MarkerKind::Portal);
                point.form = form;
                point.portal = portal;
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
            5 => {
                Self::attach_l5_clouds(seed, parent, &mut points);
                // Disk-frame scenery behind the portal prefix; populations
                // trim to the same total, so counts and the journey hold.
                let total = points.len();
                let scenery = Self::l5_scenery(seed, parent);
                let mut folded = Vec::with_capacity(total + scenery.len());
                folded.extend(points.iter().take(portals).copied());
                folded.extend(scenery.iter().copied());
                let room = total.saturating_sub(folded.len());
                folded.extend(points.iter().skip(portals).take(room).copied());
                // Pathological budgets keep the portal prefix; scenery and
                // populations trim from the tail. No-op at real budgets.
                folded.truncate(total);
                points = folded;
            }
            6 => Self::attach_l6_systems(seed, parent, self.system_home, portals, &mut points),
            7 => crate::orbits::layout_l7(seed, parent, &mut points, self.system_home),
            8 => {
                let system_seed = hash_cell(seed, 9, 0, 0, 0);
                let count = Self::planet_count(system_seed) as usize;
                crate::orbits::layout_l8(
                    seed,
                    system_seed,
                    parent,
                    &mut points,
                    count,
                    self.system_home,
                );
            }
            9 => {
                let (planets, companions) = Self::l9_counts(seed);
                crate::orbits::layout_l9(
                    seed,
                    parent,
                    &mut points,
                    planets as usize,
                    companions > 0,
                    self.system_home,
                );
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
    use crate::system::MIN_STAR_MASS;

    /// Builds typical mid-ladder constraints for tests.
    fn parent_constraints() -> Constraints {
        Constraints::new(1.0, 2, 64, [0.5, 0.5, 0.5]).expect("valid test constraints")
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
    fn l4_galaxies_draw_forms_with_portals_in_the_lane() {
        use crate::coords::HALF_BOUND;
        use crate::r#gen::{Form, GalaxyKind};
        let level = Level::new(4).expect("L4");
        let parent = parent_constraints();
        let mut kinds = 0u32;
        for seed in [11u64, 99, 4242] {
            let out = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            for point in &out.points {
                let summary = point.galaxy.expect("L4 carries shape data");
                match (summary.kind, summary.far_view) {
                    (_, true) => assert_eq!(point.form, Form::Dot, "impostors stay dots"),
                    (GalaxyKind::Spiral, false) => {
                        kinds += 1;
                        let Form::Disk { normal, barred } = point.form else {
                            panic!("spiral must draw a disk: {:?}", point.form);
                        };
                        // Unit, parallel to the sampled orientation (the
                        // normalizer rounds the last ulp, so compare loose).
                        let length =
                            (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2])
                                .sqrt();
                        let dot = normal[0] * summary.orientation[0]
                            + normal[1] * summary.orientation[1]
                            + normal[2] * summary.orientation[2];
                        assert!(
                            (length - 1.0).abs() < 1e-12 && (dot - 1.0).abs() < 1e-9,
                            "disk normal drifted: {normal:?} vs {:?}",
                            summary.orientation
                        );
                        assert_eq!(barred, summary.barred);
                    }
                    (GalaxyKind::Elliptical, false) => {
                        kinds += 1;
                        assert_eq!(point.form, Form::Spheroid);
                    }
                    (GalaxyKind::Irregular, false) => {
                        kinds += 1;
                        assert_eq!(point.form, Form::Clump);
                    }
                }
                assert!(
                    point.radius > 0.0 && point.radius <= HALF_BOUND,
                    "galaxy radius out of cell: {}",
                    point.radius
                );
                if point.kind == MarkerKind::Portal && !summary.far_view {
                    let offset = point.portal.expect("portals sit in the lane");
                    let lane =
                        (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2])
                            .sqrt();
                    assert!(
                        (lane - point.radius * PORTAL_LANE_FRACTION).abs() < 1e-12,
                        "portal lane drifted: {lane} vs {}",
                        point.radius
                    );
                }
            }
        }
        assert!(kinds > 0, "the probe seeds must show near galaxies");
    }

    #[test]
    fn l6_stars_size_by_mass_with_companion_arcs() {
        use crate::r#gen::Form;
        let parent = parent_constraints();
        let level = Level::new(6).expect("L6");
        let mut arcs = 0u32;
        for seed in [4242u64, 11, 99, 7] {
            let out = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            for point in &out.points {
                let star = point.star.expect("every L6 system is a star");
                let want = GalaxyGenerator::star_radius(star.mass_solar);
                assert_eq!(
                    point.radius, want,
                    "seed {seed} sizes by mass: {} vs {want}",
                    point.radius
                );
                if point.kind == MarkerKind::Portal {
                    assert_eq!(point.form, Form::Dot, "suns read as dots");
                } else if !matches!(point.form, Form::Dot) {
                    arcs += 1;
                    assert!(
                        matches!(point.form, Form::Arc { .. }),
                        "companions carry arcs: {:?}",
                        point.form
                    );
                }
            }
        }
        assert!(arcs > 0, "the probe seeds must roll companions");
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
            let out = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
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
    fn solar_fixture_draws_exactly_eight_for_root_42() {
        use crate::nest::{autopilot_path, path_seed};
        let chain = autopilot_path(42);
        assert!(chain.len() >= 8, "root-42 journey must reach L9");
        let seed = path_seed(42, &chain[..7]);
        let system_seed = hash_cell(seed, 9, 0, 0, 0);
        assert_eq!(
            GalaxyGenerator::planet_count(system_seed),
            8,
            "SALT_PLANETS calibration broke: Solar home must draw 8"
        );
    }

    #[test]
    fn l9_planets_match_l8_data_through_the_shared_seed() {
        let parent = parent_constraints();
        let l8 = Level::new(8).expect("L8");
        let l9 = Level::new(9).expect("L9");
        for seed in [11u64, 4242, 99_999] {
            let system_seed = hash_cell(seed, 9, 0, 0, 0);
            let eight = GalaxyGenerator::new(l8, false, false, false).generate(seed, &parent);
            let nine = GalaxyGenerator::new(l9, false, false, false).generate(system_seed, &parent);
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
            let out = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            let (planets, companions) = GalaxyGenerator::l9_counts(seed);
            let portal_data = out
                .points
                .iter()
                .filter(|point| point.kind == MarkerKind::Portal)
                .count();
            assert_eq!(
                portal_data, planets as usize,
                "seed {seed} portal count moved"
            );
            // Populations are the companion (when drawn) plus the star body
            // appended last (#384): the star never opens.
            let populations: Vec<_> = out
                .points
                .iter()
                .filter(|point| point.kind == MarkerKind::Population)
                .collect();
            assert_eq!(
                populations.len(),
                companions as usize + 1,
                "seed {seed} population drift"
            );
            let star = out.points.last().expect("the star appends last");
            assert_eq!(star.kind, MarkerKind::Population);
            assert_eq!(star.form, Form::Body);
            star.star.expect("appended body is a star");
            assert!(star.planet.is_none(), "the star carries no planet");
            for companion in populations.iter().take(companions as usize) {
                companion.star.expect("companion is a star");
                assert!(companion.planet.is_none(), "companion carries no planet");
            }
            if companions > 0 {
                probed += 1;
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
            let first = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            let second = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
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
            let out = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
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
            let first = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            let second = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            assert_eq!(first, second, "seed {seed} clouds must replay");
            let (orientation, _) = GalaxyGenerator::l5_arm_frame(seed);
            let length = (orientation[0] * orientation[0]
                + orientation[1] * orientation[1]
                + orientation[2] * orientation[2])
                .sqrt();
            assert!((length - 1.0).abs() < 1e-12, "arm frame not unit");
            // Disk-frame scenery rides behind the portal prefix (#384).
            let mut disks = 0u32;
            let mut arms = 0u32;
            for point in &first.points {
                match point.form {
                    Form::Disk { .. } => disks += 1,
                    Form::Arm { .. } => arms += 1,
                    _ => {}
                }
            }
            assert_eq!(disks, 1, "seed {seed} keeps its disk frame");
            assert_eq!(arms, 2, "seed {seed} keeps both arms");
            let mut thin = 0u32;
            let mut clouds = 0u32;
            for point in &first.points {
                let Some(cloud) = point.cloud else {
                    continue;
                };
                clouds += 1;
                assert!(
                    (10.0..=10_000_000.0).contains(&cloud.mass_solar),
                    "cloud mass out of spectrum: {}",
                    cloud.mass_solar
                );
                assert!(point.star.is_none() && point.planet.is_none());
                for (axis, limit) in parent.allowed_extent.iter().enumerate() {
                    assert!(point.position[axis].abs() <= *limit, "cloud outside extent");
                }
                let height = (point.position[0] * orientation[0]
                    + point.position[1] * orientation[1]
                    + point.position[2] * orientation[2])
                    .abs();
                thin += u32::from(height < 0.1 * extent);
            }
            let share = f64::from(thin) / f64::from(clouds.max(1));
            assert!(
                share > 0.5,
                "seed {seed} clouds not thin about the arms: {share}"
            );
        }
    }
    #[test]
    fn l7_shells_stratify_into_inner_and_outer_bands() {
        use crate::coords::HALF_BOUND;
        use crate::r#gen::Form;
        use crate::orbits::{AU_MAX_L7, AU_MIN_L7, kuiper_band, orbit_radius_cell, scattered_band};
        let parent = parent_constraints();
        let level = Level::new(7).expect("L7");
        let (kuiper_lo, kuiper_hi) = kuiper_band();
        let (scattered_lo, scattered_hi) = scattered_band();
        let oort_lo = orbit_radius_cell(2_000.0, AU_MIN_L7, AU_MAX_L7);
        for seed in [11u64, 4242, 99_999] {
            let first = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            let second = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            assert_eq!(first, second, "seed {seed} shells must replay");
            assert_eq!(first.points.len(), 32, "shell count never moves");
            let head = first.points.first().expect("L7 keeps its star");
            assert_eq!(head.position, [0.0, 0.0, 0.0]);
            head.star.expect("central star carries data");
            let mut bands = [0u32; 3];
            for point in first.points.iter().skip(1) {
                assert_eq!(point.kind, MarkerKind::Population, "shells never open");
                assert!(point.star.is_none() && point.planet.is_none());
                let radius = (point.position[0] * point.position[0]
                    + point.position[1] * point.position[1]
                    + point.position[2] * point.position[2])
                    .sqrt();
                // Bands are planar: the doughnut thickness lifts dots above
                // the band edge in 3D, so rings assert the planar radius.
                let planar = (point.position[0] * point.position[0]
                    + point.position[2] * point.position[2])
                    .sqrt();
                let flat = point.position[1].abs() / radius.max(1e-9);
                match point.form {
                    Form::Ring { inner, outer, .. } if inner == kuiper_lo && outer == kuiper_hi => {
                        bands[0] += 1;
                        assert!(
                            (kuiper_lo..=kuiper_hi).contains(&planar),
                            "Kuiper dot off its band: {planar}"
                        );
                        assert!(flat <= 0.09, "Kuiper doughnut must read flat: {flat}");
                    }
                    Form::Ring { inner, outer, .. }
                        if inner == scattered_lo && outer == scattered_hi =>
                    {
                        bands[1] += 1;
                        assert!(
                            (scattered_lo..=scattered_hi).contains(&planar),
                            "scattered dot off its band: {planar}"
                        );
                        assert!(flat <= 0.31, "scattered disc must read tilted: {flat}");
                    }
                    Form::Shell => {
                        bands[2] += 1;
                        assert!(
                            (oort_lo..=HALF_BOUND).contains(&radius),
                            "Oort dot off the reservoir: {radius}"
                        );
                    }
                    ref other => panic!("seed {seed} L7 wears a wrong form: {other:?}"),
                }
            }
            assert!(
                bands.iter().all(|count| *count > 0),
                "seed {seed} must show every band: {bands:?}"
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

    #[test]
    fn tints_ride_star_and_galaxy_points_with_home_fixtures() {
        use crate::astro::{GREEN_VALLEY, galaxy_tint};
        use crate::nest::{autopilot_candidates, autopilot_marker};
        use crate::system::star_tint;
        let parent = parent_constraints();
        // L4: every galaxy point carries its kind tint; the Milky Way home
        // pick carries the green-valley value instead, moving nothing else.
        for seed in [7u64, 42, 12345] {
            let level = Level::new(4).expect("L4");
            let plain = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            let home = GalaxyGenerator::new(level, false, true, false).generate(seed, &parent);
            let candidates = autopilot_candidates(level, seed, &home.points);
            let pick = autopilot_marker(seed, &candidates).expect("L4 pick");
            for (index, (a, b)) in plain.points.iter().zip(home.points.iter()).enumerate() {
                assert_eq!(a.position, b.position, "seed {seed} positions move");
                assert_eq!(a.radius, b.radius, "seed {seed} sizes move");
                assert_eq!(a.kind, b.kind, "seed {seed} kinds move");
                if index == pick as usize {
                    let tint = b.tint.expect("home pick tinted");
                    assert_eq!(
                        [tint.red, tint.green, tint.blue],
                        GREEN_VALLEY,
                        "Milky Way reads green valley"
                    );
                } else {
                    assert_eq!(a.galaxy, b.galaxy, "neighbours must not move");
                    assert_eq!(a.tint, b.tint, "neighbour tints must not move");
                }
            }
            for point in &plain.points {
                let g = point.galaxy.expect("L4 carries shape data");
                assert_eq!(point.tint, Some(galaxy_tint(g.kind)));
            }
        }
        // L6-L9: every star point carries its class tint; home Suns are white.
        for level_n in [6u8, 7, 8, 9] {
            let level = Level::new(level_n).expect("ladder level");
            for seed in [7u64, 42, 4242] {
                let plain =
                    GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
                let again =
                    GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
                assert_eq!(plain, again, "L{level_n} tints replay bit for bit");
                for point in &plain.points {
                    if let Some(star) = point.star {
                        assert_eq!(point.tint, Some(star_tint(star.kind)));
                    }
                }
                let home = GalaxyGenerator::new(level, false, false, true).generate(seed, &parent);
                let head = home.points.first().expect("home keeps its head");
                if head.star.is_some() {
                    let tint = head.tint.expect("home star tinted");
                    assert_eq!(
                        (tint.red, tint.green, tint.blue, tint.brightness),
                        (1.0, 1.0, 1.0, 1.0),
                        "L{level_n} home star reads white"
                    );
                }
            }
        }
    }
}
