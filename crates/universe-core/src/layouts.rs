//! L4-L6 marker layouts: galaxy forms, cloud rings, star radii (#384).
//!
//! Split from [`crate::sysgen`] under `E-FILE-SIZE`: [`crate::sysgen`]
//! keeps counts, portal order, and the home fixtures; this module owns
//! where L4-L6 points sit and what forms they wear. The home fixtures
//! (Milky Way shape, Alpha Centauri triple) apply here on `sysgen`'s
//! behalf with the same rules: positions, sizes, ranks, and portal mapping
//! never move, only shapes and companions land. Everything here is pure
//! generation math (`E-TRANSCENDENTAL`: `log10` and `powf` stay in core).

use crate::astro::{cross, orthonormal_tangent, spiral_disk_point, unit_or_default};
use crate::r#gen::{
    CloudInfo, Constraints, Form, GalaxyInfo, GalaxyKind, MarkerKind, Point, StarInfo, StarKind,
    clamp_to_sphere,
};
use crate::nest::autopilot_marker;
use crate::seed::{Rng, hash_triple};
use crate::system::{
    ALPHA_CEN_TRIPLE, CLOUD_STREAM_TAG, PLANET_STREAM_TAG, STAR_STREAM_TAG, roll_companions,
    sample_cloud_mass, sample_star, star_tint,
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
pub(crate) fn galaxy_form_and_portal(
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

/// Arm frame for L5 clouds: disk normal plus bar flag (#155 T3).
///
/// One cell, one frame: orientation uniform on the sphere and an
/// even bar coin from a fixed stream lane, so every cloud in the cell
/// shares the galaxy plane the #154 layouts define. The host type is
/// unknown this deep in the contract (no parent shape crosses it), so
/// every L5 cell reads as a star-forming spiral by documented design.
pub(crate) fn l5_arm_frame(seed: u64) -> ([f64; 3], bool) {
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
pub(crate) fn l5_scenery(seed: u64, parent: &Constraints) -> [Point; 3] {
    let (orientation, barred) = l5_arm_frame(seed);
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
pub(crate) fn attach_l5_clouds(seed: u64, parent: &Constraints, points: &mut [Point]) {
    let (orientation, barred) = l5_arm_frame(seed);
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
        let outer = cloud_ring_outer(mass);
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
pub(crate) fn star_radius(mass_solar: f64) -> f64 {
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
#[expect(
    clippy::cast_possible_truncation,
    reason = "E-CAST: portal prefix stays far below u32::MAX by budget"
)]
pub(crate) fn attach_l6_systems(
    seed: u64,
    parent: &Constraints,
    system_home: bool,
    portals: usize,
    points: &mut Vec<Point>,
) {
    for (index, point) in points.iter_mut().enumerate() {
        let lane = index as u64;
        let mut stream = Rng::new(hash_triple(seed, STAR_STREAM_TAG, lane));
        let info = sample_star(&mut stream);
        point.tint = Some(star_tint(info.kind));
        point.star = Some(info);
        point.radius = star_radius(info.mass_solar);
    }
    let mut extras: Vec<Point> = Vec::new();
    if system_home {
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
            primary.radius = star_radius(alpha_a.1);
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
                    radius: star_radius(mass),
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
                    radius: star_radius(info.mass_solar),
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
