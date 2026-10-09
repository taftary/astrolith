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
    CloudInfo, ColorInfo, Constraints, GalaxyInfo, GalaxyKind, Generated, Generator, MarkerKind,
    Point, StarInfo, StarKind, clamp_to_sphere,
};
use crate::nest::autopilot_marker;
use crate::seed::{Rng, binomial_draw, hash_cell, hash_triple};
use crate::system::{
    ALPHA_CEN_TRIPLE, CLOUD_STREAM_TAG, OORT_STREAM_TAG, PLANET_STREAM_TAG, roll_companions,
    sample_cloud_mass, sample_star, star_tint, system_planet, system_star,
};
use std::f64::consts::PI;

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
            let lane = index as u64;
            let mut stream = Rng::new(hash_triple(seed, CLOUD_STREAM_TAG, lane));
            let disk = spiral_disk_point(&mut stream, size, barred);
            point.position = clamp_to_sphere([
                tangent[0] * disk[0] + bitangent[0] * disk[1] + normal[0] * disk[2],
                tangent[1] * disk[0] + bitangent[1] * disk[1] + normal[1] * disk[2],
                tangent[2] * disk[0] + bitangent[2] * disk[1] + normal[2] * disk[2],
            ]);
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
            point.position = clamp_to_sphere([
                ring * angle.cos() * radius,
                ring * angle.sin() * radius,
                z * radius,
            ]);
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
            let lane = index as u64;
            let mut stream = Rng::new(hash_triple(seed, crate::system::STAR_STREAM_TAG, lane));
            let info = sample_star(&mut stream);
            point.tint = Some(star_tint(info.kind));
            point.star = Some(info);
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
                        radius: (primary.radius * 0.5).max(1e-4),
                        kind: MarkerKind::Population,
                        star: Some(StarInfo {
                            kind,
                            mass_solar: mass,
                        }),
                        tint: Some(star_tint(kind)),
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
                        radius: (radius * 0.5).max(1e-4),
                        kind: MarkerKind::Population,
                        star: Some(info),
                        tint: Some(star_tint(info.kind)),
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
            points.push(Point {
                position,
                radius: galaxy.size,
                kind,
                galaxy: info,
                tint,
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
                    let host = if self.system_home {
                        StarInfo {
                            kind: StarKind::G,
                            mass_solar: 1.0,
                        }
                    } else {
                        let mut stream =
                            Rng::new(hash_triple(seed, crate::system::STAR_STREAM_TAG, 0));
                        sample_star(&mut stream)
                    };
                    star.tint = Some(star_tint(host.kind));
                    star.star = Some(host);
                }
                Self::layout_oort_shells(seed, parent, &mut points);
            }
            8 => {
                let system_seed = hash_cell(seed, 9, 0, 0, 0);
                let host = system_star(system_seed, self.system_home);
                if let Some(star) = points.first_mut() {
                    star.tint = Some(star_tint(host.kind));
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
                    let mut stream = Rng::new(hash_triple(seed, PLANET_STREAM_TAG, u64::MAX));
                    let mut position = [0.0; 3];
                    for (slot, extent) in position.iter_mut().zip(parent.allowed_extent.iter()) {
                        *slot = (stream.next_f64() * 2.0 - 1.0) * extent;
                    }
                    position = clamp_to_sphere(position);
                    let floor = points
                        .iter()
                        .map(|point| point.radius)
                        .fold(f64::INFINITY, f64::min);
                    let info = sample_star(&mut stream);
                    points.push(Point {
                        position: clamp_to_sphere(position),
                        radius: (floor * 0.5).max(1e-4),
                        kind: MarkerKind::Population,
                        star: Some(info),
                        tint: Some(star_tint(info.kind)),
                        ..Point::bare([0.0; 3], 0.01, MarkerKind::Population)
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
            let mut thin = 0u32;
            for point in &first.points {
                let mass = point.cloud.expect("every L5 cloud carries mass").mass_solar;
                assert!(
                    (10.0..=10_000_000.0).contains(&mass),
                    "cloud mass out of spectrum: {mass}"
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
            let share = f64::from(thin)
                / f64::from(u32::try_from(first.points.len()).expect("cell fits u32"));
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
            let first = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
            let second = GalaxyGenerator::new(level, false, false, false).generate(seed, &parent);
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
