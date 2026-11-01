//! L4 is the inside of one galaxy: its molecular clouds (#996, Spec v5
//! Goal 3).
//!
//! An L4 cell is the size of the Milky Way (ladder R5), so you are inside
//! one galaxy and its markers are clouds, each up to the size of an L5
//! cell. How many: the Milky Way holds 8,107 (Miville-Deschênes, Murray and
//! Lee 2017, study source R7); another spiral holds that many per disk area
//! (by its size against a typical galaxy); irregulars and ellipticals hold
//! far less cold gas (the LMC's 272 clouds, Fukui et al. 2008; ellipticals
//! after ATLAS3D, Young et al. 2011). The disk lies in the host galaxy's
//! plane, so the clouds line up with the galaxy you entered. The biggest
//! one in fifty can be entered; on the home journey the region around the
//! Sun is the pick, 8.2 kpc from the centre. Pure (`E-CORE-NO-BEVY`,
//! `E-DET-TIERS`).

use crate::astro::{cross, orthonormal_tangent, spiral_disk_point, unit_or_default};
use crate::coords::Level;
use crate::r#gen::{CloudInfo, Form, GalaxyInfo, GalaxyKind, MarkerKind, Point, clamp_to_sphere};
use crate::nest::{marker_radius, scale_exponent};
use crate::seed::{Rng, hash_triple};
use crate::system::{CLOUD_STREAM_TAG, sample_cloud_mass};

/// Level whose markers are clouds.
pub const CLOUD_LEVEL: u8 = 4;

/// Molecular clouds in the Milky Way (study R7).
pub const MILKY_WAY_CLOUDS: u32 = 8107;

/// Most clouds a cell holds: twice the Milky Way, for the biggest spirals.
pub const MAX_CLOUDS: u32 = 2 * MILKY_WAY_CLOUDS;

/// Fewest clouds a cell holds.
pub const MIN_CLOUDS: u32 = 8;

/// Clouds per Milky Way cloud in an irregular (LMC: 272 of 8,107).
const IRREGULAR_SHARE: f64 = 272.0 / 8107.0;

/// Clouds per Milky Way cloud in an elliptical: most hold no cold gas,
/// the gas-rich fifth hold far less than a spiral (ATLAS3D).
const ELLIPTICAL_SHARE: f64 = 0.02;

/// One cloud in fifty is a complex big enough to enter.
const PORTAL_SHARE: usize = 50;

/// Typical sampled galaxy size (`astro::sample_galaxy`, the bell's middle).
const TYPICAL_SAMPLED_SIZE: f64 = 0.05;

/// Disk layout scale for a typical galaxy, in L4 cells (the #155 layout).
const DISK_SIZE: f64 = 0.25;

/// The Sun's distance from the Milky Way's centre, 8.2 kpc, in L4 cells.
pub const SUN_RADIUS_CELLS: f64 = 8.2e3 * 3.0857e16 / 9.55e20;

/// The galaxy an L4 cell is the inside of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Host {
    /// Its shape summary from the L3 cell.
    pub galaxy: GalaxyInfo,
    /// Its sampled size (`astro::sample_galaxy`).
    pub size: f64,
}

/// Clouds inside `host`; the Milky Way's number at home.
#[must_use]
pub fn cloud_count(host: Option<Host>, home: bool) -> u32 {
    if home {
        return MILKY_WAY_CLOUDS;
    }
    let Some(host) = host else {
        return MIN_CLOUDS;
    };
    let relative = (host.size / TYPICAL_SAMPLED_SIZE).powi(2);
    let share = match host.galaxy.kind {
        GalaxyKind::Spiral => 1.0,
        GalaxyKind::Irregular => IRREGULAR_SHARE,
        GalaxyKind::Elliptical => ELLIPTICAL_SHARE,
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "E-CAST: clamped to [MIN_CLOUDS, MAX_CLOUDS] before the cast"
    )]
    let count = (f64::from(MILKY_WAY_CLOUDS) * relative * share)
        .round()
        .clamp(f64::from(MIN_CLOUDS), f64::from(MAX_CLOUDS)) as u32;
    count
}

/// Clouds that open among `total`: the biggest one in fifty, at least one.
#[must_use]
pub fn portals(total: usize) -> usize {
    (total / PORTAL_SHARE).max(1).min(total)
}

/// Lays the clouds of an L4 cell in the host's disk, sized by mass.
///
/// Positions follow the shared spiral layout in the host's plane (the
/// #155 cloud layout, moved up from L5); each cloud's radius follows the
/// size-mass relation of molecular clouds (about 20 pc at 1e5 solar
/// masses, growing as the square root of the mass). Ranks and kinds are
/// left to the caller.
pub fn lay_out(seed: u64, host: Option<Host>, points: &mut [Point]) {
    let (orientation, barred, size) = match host {
        Some(h) => (
            h.galaxy.orientation,
            h.galaxy.barred,
            DISK_SIZE * (h.size / TYPICAL_SAMPLED_SIZE).clamp(0.4, 1.8),
        ),
        None => (crate::layouts::l5_arm_frame(seed).0, false, DISK_SIZE),
    };
    let normal = unit_or_default(orientation);
    let tangent = orthonormal_tangent(normal);
    let bitangent = cross(normal, tangent);
    let cell_pc = 10f64.powf(scale_exponent(level())) / 3.0857e16;
    for (index, point) in points.iter_mut().enumerate() {
        let mut stream = Rng::new(hash_triple(seed, CLOUD_STREAM_TAG, index as u64));
        let disk = spiral_disk_point(&mut stream, size, barred);
        point.position = clamp_to_sphere([
            tangent[0] * disk[0] + bitangent[0] * disk[1] + normal[0] * disk[2],
            tangent[1] * disk[0] + bitangent[1] * disk[1] + normal[1] * disk[2],
            tangent[2] * disk[0] + bitangent[2] * disk[1] + normal[2] * disk[2],
        ]);
        let mass = sample_cloud_mass(&mut stream);
        let outer = (20.0 * (mass / 1e5).sqrt()).clamp(1.0, 150.0) / cell_pc;
        point.cloud = Some(CloudInfo { mass_solar: mass });
        point.radius = outer;
        point.galaxy = None;
        point.tint = None;
        point.portal = None;
        point.form = Form::Ring {
            normal,
            inner: outer * 0.7,
            outer,
        };
    }
}

/// Puts the journey's pick at the Sun's place: 8.2 kpc out in the disk,
/// a region one L5 cell across.
pub fn place_sun_region(points: &mut [Point], pick: u32, host: Option<Host>) {
    let Some(point) = points.get_mut(pick as usize) else {
        return;
    };
    let normal = unit_or_default(host.map_or([0.0, 0.0, 1.0], |h| h.galaxy.orientation));
    let tangent = orthonormal_tangent(normal);
    point.position = [
        tangent[0] * SUN_RADIUS_CELLS,
        tangent[1] * SUN_RADIUS_CELLS,
        tangent[2] * SUN_RADIUS_CELLS,
    ];
    point.radius = marker_radius(level()).unwrap_or(point.radius);
    point.cloud = None;
    point.form = Form::Dot;
    point.kind = MarkerKind::Portal;
}

/// The galaxy the L4 cell at `chain` is the inside of: the opened
/// marker of its L3 parent.
#[must_use]
pub fn host_of(root: u64, chain: &[u32]) -> Option<Host> {
    let (&opened, parent) = chain.split_last()?;
    let cell = crate::nest::generate_cell(root, parent);
    let point = cell.points.get(opened as usize)?;
    let unit = crate::galaxy_l3::radius(1.0);
    Some(Host {
        galaxy: point.galaxy?,
        size: if unit > 0.0 {
            point.radius / unit
        } else {
            TYPICAL_SAMPLED_SIZE
        },
    })
}

fn level() -> Level {
    Level::new(CLOUD_LEVEL).unwrap_or(Level::MIN)
}

#[cfg(test)]
#[path = "clouds_l4_tests.rs"]
mod tests;
