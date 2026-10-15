//! Surface ages and craters: chronology counts, profiles, softening, rays
//! and secondaries (#685, T5).
//!
//! Guides: `guides/universe/L10/generation/documents/height-maps.md`
//! (building block 6: profiles, the `D^-2` counts, the `1/g` transition,
//! spatial-hash placement) and `landforms.md` (rays, secondaries and
//! chains). Craters pile onto the base plus plates inside
//! [`crate::world::relief::total_relief_m`]; T6 stamps volcanoes beside
//! them, T7 colours the rays, T11 replaces the provisional system age
//! with the star-lifetime draw.
//!
//! Placement follows the guides: per size class a 3D lattice holds at
//! most one crater per cell, found through a spatial hash, so each sample
//! checks nearby craters only. Cell sizes scale with the class diameter,
//! so equal per-cell odds reproduce the `D^-2` law across classes
//! automatically. Everything is `f64` and deterministic (`E-DET-TIERS`);
//! sampling stays allocation-free (`E-HOT-NOALLOC`).

use super::classify::draw_body;
use super::noise3::smoothstep;
use super::plates::{BoundaryKind, Plates, boundary_at, field_for_relief, plate_index_at};
use super::relief::{gravity_for, relief_scale};
use crate::icosphere::normalize;
use crate::seed::hash_triple;
use std::f64::consts::TAU;

/// Crater class radii in metres (1 km to 256 km diameters; larger basins
/// flood with lava in T6).
pub const CLASS_RADII_M: [f64; 5] = [500.0, 2_000.0, 8_000.0, 32_000.0, 128_000.0];

/// Crater class count (matches `CLASS_RADII_M` by the assertion below).
pub const CRATER_CLASSES: u32 = 5;

/// Crater class count matches its radii table.
const _: () = assert!(CRATER_CLASSES as usize == CLASS_RADII_M.len());

/// Lattice cell size in crater radii (one crater per cell at most).
pub const CELL_RADII: f64 = 8.0;

/// Crater influence in crater radii (rim plus `(R/r)^3` ejecta).
pub const INFLUENCE_RADII: f64 = 2.5;

/// Cell visits per crater sample (every class checks its 27 neighbours).
pub const CRATER_CELL_VISITS_PER_SAMPLE: u32 = CRATER_CLASSES * 27;

/// Classes feeding the ray field (freyed ejecta reads at 16 km up).
pub const RAY_MIN_R_M: f64 = 8_000.0;

/// Ray length in crater radii (seeded 6-12, "several to tens").
pub const RAY_MIN_LEN_RADII: f64 = 6.0;
/// Ray length span in crater radii.
pub const RAY_LEN_SPAN_RADII: f64 = 6.0;

/// Ray neighbourhood half-width in cells (covers the longest ray).
pub const RAY_CELL_RADIUS: i64 = 2;

/// Cell visits per ray sample (large classes check 125 neighbours).
pub const RAY_CELL_VISITS_PER_SAMPLE: u32 = 2 * 125;

/// Ray orientations per crater (ten bright wedges, the 6-20 row).
pub const RAY_ORIENTATIONS: u32 = 5;

/// Rays fade over about a billion years (lunar rays, GEN-S072).
pub const RAY_AGE_GYR: f64 = 1.0;

/// Secondaries appear around primaries younger than this, in Gyr.
pub const SECONDARY_AGE_GYR: f64 = 0.5;

/// Secondaries appear around primaries at least this radius, in metres.
pub const SECONDARY_MIN_R_M: f64 = 2_000.0;

/// Softening time in Gyr (old craters relax toward the floor below).
pub const SOFTEN_TAU_GYR: f64 = 1.0;

/// Softening floor: the oldest craters stay faintly visible.
pub const SOFTEN_FLOOR: f64 = 0.15;

/// Young crust near a divergent boundary, in Gyr (new ocean floor).
pub const RIDGE_YOUTH_GYR: f64 = 0.3;

/// Divergent youth reaches this far past the edge, in radians.
pub const RIDGE_YOUTH_RAD: f64 = 0.05;

/// Largest crater radius as a share of the planet radius (bigger belongs
/// to basins, T6).
pub const MAX_CRATER_PLANET_SHARE: f64 = 0.5;

/// Stream tag of the crater lattice (shared with the colour pass).
pub(crate) const CRATER_TAG: u64 = 0xC9A7_E685_0000_0000;
/// Stream tag of the surface-age draw.
const AGE_TAG: u64 = 0xA6E5_E685_0000_0000;
/// Stream tag of the resurfacing events.
const EVENT_TAG: u64 = 0xE5CA_E685_0000_0000;

/// A share in 0..1 from a hash value (shared with the landform and
/// colour passes).
pub(crate) fn share01(hash: u64) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: 53-bit hash share as a unit float, intended"
    )]
    let share = (hash >> 11) as f64 / 9_007_199_254_740_992.0;
    share.clamp(0.0, 1.0 - f64::EPSILON)
}

/// Hash of one lattice cell under the class seed.
pub(crate) fn cell_hash(class_seed: u64, cell: [i64; 3]) -> u64 {
    let [cx, cy, cz] = cell;
    #[expect(
        clippy::cast_sign_loss,
        reason = "E-CAST: lattice coords reinterpreted as bits in the hash"
    )]
    let hash = hash_triple(class_seed, CRATER_TAG, cx as u64);
    #[expect(
        clippy::cast_sign_loss,
        reason = "E-CAST: lattice coords reinterpreted as bits in the hash"
    )]
    let hash = hash_triple(hash, cy as u64, cz as u64);
    hash
}

/// Angular distance between unit directions, in radians.
fn ang_dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
        .clamp(-1.0, 1.0)
        .acos()
}

/// Surface age in Gyr of `region` on the world seeded `seed`: the system
/// age `system_age_gyr` (capped by the star's lifetime by the caller)
/// reset by up to three seeded resurfacing events (lava floods, ice
/// resurfacing, young plates). Younger systems read younger surfaces; at
/// and past the mature reference the longstanding values hold exactly.
#[must_use]
pub fn surface_age_gyr(seed: u64, region: u64, system_age_gyr: f64) -> f64 {
    let provisional = 0.1 + share01(hash_triple(seed, AGE_TAG, 0)) * 4.4;
    let system = if system_age_gyr.is_finite() {
        provisional.min(system_age_gyr.max(0.05))
    } else {
        provisional
    };
    let events = hash_triple(seed, EVENT_TAG, region) % 4;
    let mut last: f64 = 0.0;
    for i in 0..events {
        let at = share01(hash_triple(seed, EVENT_TAG, region * 16 + i + 1)) * system;
        last = last.max(at);
    }
    (system - last).max(0.05)
}

/// Simple-to-complex transition diameter in metres at surface gravity
/// `g` (height-maps guide: `D_tr ≈ 16533.8 g^-1.0303` m, about 10 km on
/// the Moon, about 1.6 km on Earth). Degenerate gravity reads Earth's.
#[must_use]
pub fn transition_diameter_m(gravity: f64) -> f64 {
    if !gravity.is_finite() || gravity <= 0.0 {
        return transition_diameter_m(9.81);
    }
    16_533.8 * gravity.powf(-1.0303)
}

/// Softening of a crater `age_gyr` old: fresh craters sharp, old ones
/// relaxed toward the floor (height-maps guide: old craters soft, young
/// ones sharp with rays and secondaries).
#[must_use]
pub fn soften_factor(crater_age_gyr: f64) -> f64 {
    if !crater_age_gyr.is_finite() || crater_age_gyr <= 0.0 {
        return 1.0;
    }
    (-crater_age_gyr / SOFTEN_TAU_GYR).exp().max(SOFTEN_FLOOR)
}

/// Shape of one crater: its depth and whether it is complex.
pub(crate) struct CraterShape {
    /// Bowl depth in metres.
    depth_m: f64,
    /// Complex craters wear a flat floor and a central peak.
    complex: bool,
}

/// Shape of a crater `diameter_m` wide at surface gravity `gravity`:
/// simple bowls (`0.2 D` deep) below the transition, complex ones
/// (`d = 1.044 D^0.301` km) above (height-maps guide, GEN-S007).
pub(crate) fn shape_for(diameter_m: f64, gravity: f64) -> CraterShape {
    if diameter_m < transition_diameter_m(gravity) {
        CraterShape {
            depth_m: 0.2 * diameter_m,
            complex: false,
        }
    } else {
        CraterShape {
            depth_m: 1_044.0 * (diameter_m / 1_000.0).powf(0.301),
            complex: true,
        }
    }
}

/// Height in metres at `rho_m` metres from the centre of a crater of
/// radius `radius_m`: the bowl (`-d (1 - (r/R)^2)`, flat floor and
/// central peak when complex), a raised rim of about `0.04 D`, and ejecta
/// falling as `(R/r)^3` outside (height-maps guide). Unsoftened: the
/// caller scales by [`soften_factor`].
pub(crate) fn profile_at(shape: &CraterShape, rho_m: f64, radius_m: f64) -> f64 {
    if radius_m <= 0.0 || !rho_m.is_finite() {
        return 0.0;
    }
    let t = rho_m / radius_m;
    let bowl = if shape.complex {
        -shape.depth_m * (1.0 - smoothstep(0.6, 1.0, t))
    } else {
        -shape.depth_m * (1.0 - t * t).max(0.0)
    };
    let peak = if shape.complex {
        let u = t / 0.15;
        0.3 * shape.depth_m * (-u * u).exp()
    } else {
        0.0
    };
    let v = (t - 1.0) / 0.15;
    let rim = (0.08 * radius_m).min(0.5 * shape.depth_m) * (-v * v).exp();
    let ejecta = if t > 1.0 {
        (0.04 * radius_m).min(0.25 * shape.depth_m) / (t * t * t)
    } else {
        0.0
    };
    bowl + peak + rim + ejecta
}

/// Tangent basis at unit `centre`: two perpendicular unit directions.
fn tangent_basis(centre: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let reference = if centre[1].abs() > 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let first = normalize([
        centre[1] * reference[2] - centre[2] * reference[1],
        centre[2] * reference[0] - centre[0] * reference[2],
        centre[0] * reference[1] - centre[1] * reference[0],
    ]);
    let second = [
        centre[1] * first[2] - centre[2] * first[1],
        centre[2] * first[0] - centre[0] * first[2],
        centre[0] * first[1] - centre[1] * first[0],
    ];
    (first, second)
}

/// One crater under evaluation: centre, size, age and softening.
pub(crate) struct Hit {
    /// Centre (unit direction).
    pub(crate) centre: [f64; 3],
    /// Radius in radians.
    pub(crate) radius_ang: f64,
    /// Radius in metres.
    pub(crate) radius_m: f64,
    /// Age in Gyr.
    pub(crate) age_gyr: f64,
    /// Softening from the age.
    pub(crate) soft: f64,
}

/// The crater of one lattice cell, if the chronology puts one there:
/// existence rolls against the density for `sample_age`, the centre
/// jittered inside the cell, the radius jittered around the class, the
/// age drawn over the surface age (older stamped first, younger over).
/// Shared by the relief, ray and colour passes, so rays sit on real
/// craters and water sits in real bowls.
pub(crate) fn derive_hit(
    class_seed: u64,
    lattice: [i64; 3],
    cell: f64,
    class_r_m: f64,
    radius_m_planet: f64,
    sample_age: f64,
) -> Option<Hit> {
    let density = (sample_age / 4.0).clamp(0.0, 1.0);
    let hash = cell_hash(class_seed, lattice);
    if share01(hash) >= density {
        return None;
    }
    let [cx, cy, cz] = lattice;
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: lattice coords near the origin, exactly representable"
    )]
    let origin = [cx as f64, cy as f64, cz as f64];
    let jitter = [
        share01(hash_triple(hash, CRATER_TAG, 0)),
        share01(hash_triple(hash, CRATER_TAG, 1)),
        share01(hash_triple(hash, CRATER_TAG, 2)),
    ];
    let centre = normalize([
        (origin[0] + 0.5 + (jitter[0] - 0.5) * 0.5) * cell,
        (origin[1] + 0.5 + (jitter[1] - 0.5) * 0.5) * cell,
        (origin[2] + 0.5 + (jitter[2] - 0.5) * 0.5) * cell,
    ]);
    let radius_m = class_r_m * (0.7 + 0.6 * share01(hash_triple(hash, CRATER_TAG, 3)));
    let age_gyr = share01(hash_triple(hash, CRATER_TAG, 4)) * sample_age;
    Some(Hit {
        centre,
        radius_ang: radius_m / radius_m_planet,
        radius_m,
        age_gyr,
        soft: soften_factor(age_gyr),
    })
}

/// Secondary pits around a young primary: two radial chains plus
/// scattered pits (landforms guide: secondaries line the rays in chains
/// radiating from the primary).
fn secondaries(hash: u64, primary: &Hit, soft: f64, direction: [f64; 3]) -> f64 {
    let (first, second) = tangent_basis(primary.centre);
    let mut sum = 0.0;
    let pit = |centre: [f64; 3], radius_m: f64| -> f64 {
        let rho = ang_dist(direction, centre);
        if rho > INFLUENCE_RADII * radius_m / primary.radius_m * primary.radius_ang {
            return 0.0;
        }
        let depth = 0.2 * 2.0 * radius_m;
        profile_at(
            &CraterShape {
                depth_m: depth,
                complex: false,
            },
            rho * primary.radius_m / primary.radius_ang,
            radius_m,
        ) * soft
    };
    // Two chains radiating outward, pits shrinking with distance.
    for c in 0..2u32 {
        let phi = TAU * share01(hash_triple(hash, CRATER_TAG, 10 + u64::from(c)));
        let along = [
            first[0] * phi.cos() + second[0] * phi.sin(),
            first[1] * phi.cos() + second[1] * phi.sin(),
            first[2] * phi.cos() + second[2] * phi.sin(),
        ];
        for j in 0..4u32 {
            let jf = f64::from(j);
            let dist = (2.0 + 0.55 * jf) * primary.radius_ang;
            let reach = dist.min(4.0 * primary.radius_ang);
            let pos = normalize([
                primary.centre[0] + along[0] * reach,
                primary.centre[1] + along[1] * reach,
                primary.centre[2] + along[2] * reach,
            ]);
            sum += pit(pos, primary.radius_m * (0.13 - 0.025 * jf));
        }
    }
    // Scattered pits around the ejecta.
    for k in 0..6u32 {
        let lane = u64::from(k);
        if share01(hash_triple(hash, CRATER_TAG, 20 + lane)) >= 0.7 {
            continue;
        }
        let phi = TAU * share01(hash_triple(hash, CRATER_TAG, 30 + lane));
        let along = [
            first[0] * phi.cos() + second[0] * phi.sin(),
            first[1] * phi.cos() + second[1] * phi.sin(),
            first[2] * phi.cos() + second[2] * phi.sin(),
        ];
        let dist =
            (1.5 + 1.5 * share01(hash_triple(hash, CRATER_TAG, 40 + lane))) * primary.radius_ang;
        let pos = normalize([
            primary.centre[0] + along[0] * dist,
            primary.centre[1] + along[1] * dist,
            primary.centre[2] + along[2] * dist,
        ]);
        sum += pit(
            pos,
            primary.radius_m * (0.04 + 0.08 * share01(hash_triple(hash, CRATER_TAG, 50 + lane))),
        );
    }
    sum
}

/// Crater relief in metres for one surface age: every class checks its 27
/// neighbour cells through the spatial hash, young primaries add their
/// secondaries, big bowls fill with still water, all scaled by gravity
/// like every relief amplitude.
fn relief_for_age_inner(
    seed: u64,
    direction: [f64; 3],
    radius_m: f64,
    sample_age: f64,
    base_m: f64,
    kind: super::classify::WorldKind,
) -> f64 {
    let gravity = gravity_for(seed);
    let water_01 = draw_body(seed).water;
    let air = super::lakes::has_air(radius_m);
    let unit = normalize(direction);
    let kind_here = kind;
    let mut total = 0.0;
    for (c, class_r) in CLASS_RADII_M.iter().enumerate() {
        let class_seed = hash_triple(seed, CRATER_TAG, c as u64);
        let radius_ang = class_r / radius_m;
        if radius_ang > MAX_CRATER_PLANET_SHARE {
            continue;
        }
        let cell = CELL_RADII * radius_ang;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: lattice coords near the origin always fit"
        )]
        let base = [
            (unit[0] / cell).floor() as i64,
            (unit[1] / cell).floor() as i64,
            (unit[2] / cell).floor() as i64,
        ];
        let cos_reach = (INFLUENCE_RADII * radius_ang + cell * 0.9).cos();
        for dx in -1i64..=1 {
            for dy in -1i64..=1 {
                for dz in -1i64..=1 {
                    let lattice = [base[0] + dx, base[1] + dy, base[2] + dz];
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "E-CAST: lattice coords near the origin, exactly representable"
                    )]
                    let approx = normalize([
                        (lattice[0] as f64 + 0.5) * cell,
                        (lattice[1] as f64 + 0.5) * cell,
                        (lattice[2] as f64 + 0.5) * cell,
                    ]);
                    if unit[0] * approx[0] + unit[1] * approx[1] + unit[2] * approx[2] < cos_reach {
                        continue;
                    }
                    let Some(hit) =
                        derive_hit(class_seed, lattice, cell, *class_r, radius_m, sample_age)
                    else {
                        continue;
                    };
                    let rho = ang_dist(unit, hit.centre);
                    if rho > INFLUENCE_RADII * hit.radius_ang {
                        continue;
                    }
                    let shape = shape_for(2.0 * hit.radius_m, gravity);
                    let profile = profile_at(&shape, rho * radius_m, hit.radius_m);
                    total += profile * hit.soft;
                    // Still water in big bowls (pure-lake rule): the bowl
                    // knows its own floor and rim, so the level is pure too.
                    if air && hit.radius_m >= super::lakes::BOWL_LAKE_MIN_R_M {
                        let floor_d = profile_at(&shape, 0.0, hit.radius_m) * hit.soft;
                        let rim_d = 0.08 * hit.radius_m * hit.soft;
                        let fill = super::lakes::lake_fill(water_01, 1.2);
                        let water = super::relief::base_unclamped_m(seed, hit.centre, kind_here)
                            + floor_d
                            + fill * (rim_d - floor_d);
                        total += (water - (base_m + profile * hit.soft)).max(0.0);
                    }
                    if hit.age_gyr < SECONDARY_AGE_GYR && hit.radius_m >= SECONDARY_MIN_R_M {
                        let hash = cell_hash(class_seed, lattice);
                        total += secondaries(hash, &hit, hit.soft, unit);
                    }
                }
            }
        }
    }
    total * relief_scale(gravity)
}

/// Surface age driving one sample: the age of the sample's plate region
/// (one region on a lid) under `system_age_gyr`, made young near divergent
/// boundaries where new crust spreads. Shared with the colour pass so
/// water fills the same bowls the relief stamps.
pub(crate) fn sample_age(
    field: Option<&Plates>,
    seed: u64,
    direction: [f64; 3],
    system_age_gyr: f64,
) -> f64 {
    let region = field
        .and_then(|plates| plate_index_at(plates, direction))
        .unwrap_or(0);
    let mut age = surface_age_gyr(seed, region as u64, system_age_gyr);
    if let Some(plates) = field
        && let Some(edge) = boundary_at(plates, direction)
        && edge.kind == BoundaryKind::Divergent
        && edge.edge_dist_rad < RIDGE_YOUTH_RAD
    {
        age = age.min(RIDGE_YOUTH_GYR);
    }
    age
}

/// Crater relief in metres for the built `field` (the crater half of the
/// height function; `None` reads one lid region). `base_m` is the base
/// relief at the sample (bowl water levels read it).
pub(crate) fn relief_for_field(
    field: Option<&Plates>,
    seed: u64,
    direction: [f64; 3],
    radius_m: f64,
    base_m: f64,
    kind: super::classify::WorldKind,
    system_age_gyr: f64,
) -> f64 {
    if !direction.iter().all(|c| c.is_finite()) || !radius_m.is_finite() || radius_m <= 0.0 {
        return 0.0;
    }
    relief_for_age_inner(
        seed,
        direction,
        radius_m,
        sample_age(field, seed, direction, system_age_gyr),
        base_m,
        kind,
    )
}

/// Crater relief in metres at one surface age in Gyr: the age-explicit
/// half of the height function (T11 sweeps one seed across ages with it).
/// `0.0` past the classes, off every crater, or for degenerate input.
#[must_use]
pub fn crater_relief_for_age(
    seed: u64,
    direction: [f64; 3],
    radius_m: f64,
    age_gyr: f64,
    kind: super::classify::WorldKind,
) -> f64 {
    if !direction.iter().all(|c| c.is_finite())
        || !radius_m.is_finite()
        || radius_m <= 0.0
        || !age_gyr.is_finite()
    {
        return 0.0;
    }
    relief_for_age_inner(
        seed,
        direction,
        radius_m,
        age_gyr.max(0.0),
        super::relief::base_unclamped_m(seed, direction, kind),
        kind,
    )
}

/// Crater relief in metres under the region age (one lid region without
/// plates). `None` reads the mature reference age.
#[must_use]
pub fn crater_relief_m(
    seed: u64,
    direction: [f64; 3],
    radius_m: f64,
    kind: super::classify::WorldKind,
    age: Option<f64>,
) -> f64 {
    let system_age = age.unwrap_or(super::lifetime::MATURE_AGE_GYR);
    relief_for_field(
        field_for_relief(seed, radius_m, system_age).as_ref(),
        seed,
        direction,
        radius_m,
        super::relief::base_unclamped_m(seed, direction, kind),
        kind,
        system_age,
    )
}

/// Bright ray ejecta around young craters, 0..1 (landforms guide: rays
/// are a colour layer for T7, reading from orbit on airless bodies).
/// Only craters of at least [`RAY_MIN_R_M`] throw rays far enough to
/// matter, 6-12 radii long in ten wedges, fading over [`RAY_AGE_GYR`].
#[must_use]
pub fn ray_field(seed: u64, direction: [f64; 3], radius_m: f64, system_age_gyr: f64) -> f64 {
    if !direction.iter().all(|c| c.is_finite()) || !radius_m.is_finite() || radius_m <= 0.0 {
        return 0.0;
    }
    let field = field_for_relief(seed, radius_m, system_age_gyr);
    let age = sample_age(field.as_ref(), seed, direction, system_age_gyr);
    let density = (age / 4.0).clamp(0.0, 1.0);
    if density <= 0.0 {
        return 0.0;
    }
    let unit = normalize(direction);
    let mut sum = 0.0;
    for (c, class_r) in [(3usize, 32_000.0), (4usize, 128_000.0)] {
        let class_seed = hash_triple(seed, CRATER_TAG, c as u64);
        let radius_ang = class_r / radius_m;
        if radius_ang > MAX_CRATER_PLANET_SHARE {
            continue;
        }
        let cell = CELL_RADII * radius_ang;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: lattice coords near the origin always fit"
        )]
        let base = [
            (unit[0] / cell).floor() as i64,
            (unit[1] / cell).floor() as i64,
            (unit[2] / cell).floor() as i64,
        ];
        let cos_reach = ((RAY_MIN_LEN_RADII + RAY_LEN_SPAN_RADII) * radius_ang + cell * 1.8).cos();
        for dx in -RAY_CELL_RADIUS..=RAY_CELL_RADIUS {
            for dy in -RAY_CELL_RADIUS..=RAY_CELL_RADIUS {
                for dz in -RAY_CELL_RADIUS..=RAY_CELL_RADIUS {
                    let lattice = [base[0] + dx, base[1] + dy, base[2] + dz];
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "E-CAST: lattice coords near the origin, exactly representable"
                    )]
                    let approx = normalize([
                        (lattice[0] as f64 + 0.5) * cell,
                        (lattice[1] as f64 + 0.5) * cell,
                        (lattice[2] as f64 + 0.5) * cell,
                    ]);
                    if unit[0] * approx[0] + unit[1] * approx[1] + unit[2] * approx[2] < cos_reach {
                        continue;
                    }
                    let Some(hit) = derive_hit(class_seed, lattice, cell, class_r, radius_m, age)
                    else {
                        continue;
                    };
                    if hit.age_gyr >= RAY_AGE_GYR || hit.radius_m < RAY_MIN_R_M {
                        continue;
                    }
                    let hash = cell_hash(class_seed, lattice);
                    let rho = ang_dist(unit, hit.centre);
                    let len = (RAY_MIN_LEN_RADII
                        + RAY_LEN_SPAN_RADII * share01(hash_triple(hash, CRATER_TAG, 5)))
                        * hit.radius_m
                        / radius_m;
                    if rho >= len {
                        continue;
                    }
                    // Ten bright wedges around the crater.
                    let (first, second) = tangent_basis(hit.centre);
                    let dot =
                        unit[0] * hit.centre[0] + unit[1] * hit.centre[1] + unit[2] * hit.centre[2];
                    let along = [
                        unit[0] - hit.centre[0] * dot,
                        unit[1] - hit.centre[1] * dot,
                        unit[2] - hit.centre[2] * dot,
                    ];
                    let reach =
                        (along[0] * along[0] + along[1] * along[1] + along[2] * along[2]).sqrt();
                    if !reach.is_finite() || reach < 1e-12 {
                        sum += 1.0 - hit.age_gyr / RAY_AGE_GYR;
                        continue;
                    }
                    let across = [along[0] / reach, along[1] / reach, along[2] / reach];
                    let mut streak = 0.0f64;
                    for k in 0..RAY_ORIENTATIONS {
                        let phi = TAU * share01(hash_triple(hash, CRATER_TAG, 10 + u64::from(k)));
                        let line = [
                            first[0] * phi.cos() + second[0] * phi.sin(),
                            first[1] * phi.cos() + second[1] * phi.sin(),
                            first[2] * phi.cos() + second[2] * phi.sin(),
                        ];
                        let alignment =
                            (across[0] * line[0] + across[1] * line[1] + across[2] * line[2]).abs();
                        let narrow = alignment * alignment;
                        let narrow = narrow * narrow;
                        streak = streak.max(narrow * narrow * narrow);
                    }
                    let youth = 1.0 - hit.age_gyr / RAY_AGE_GYR;
                    let fall = (-(rho / (len / 2.0)) * (rho / (len / 2.0))).exp();
                    sum += youth * streak * fall;
                }
            }
        }
    }
    sum.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spiral directions covering the sphere without a pole pinch.
    fn sweep(count: u32) -> Vec<[f64; 3]> {
        let mut out = Vec::new();
        for i in 0..count {
            let step = f64::from(i);
            let total = f64::from(count);
            let y = 1.0 - 2.0 * (step + 0.5) / total;
            let r = (1.0 - y * y).max(0.0).sqrt();
            let angle = step * 2.399_963_229_728_653;
            out.push([r * angle.cos(), y, r * angle.sin()]);
        }
        out
    }

    /// Cratered metres over a sweep (absolute sum plus deepest point).
    fn coverage(seed: u64, age: f64) -> (f64, f64, u32) {
        let mut sum = 0.0;
        let mut deepest = 0.0f64;
        let mut count = 0u32;
        for dir in sweep(512) {
            let h = crater_relief_for_age(
                seed,
                dir,
                super::super::plates::EARTH_RADIUS_M,
                age,
                super::super::classify::WorldKind::EarthLike,
            );
            sum += h.abs();
            deepest = deepest.min(h);
            if h.abs() > 1.0 {
                count += 1;
            }
        }
        (sum, deepest, count)
    }

    #[test]
    fn surface_ages_span_young_to_old() {
        for seed in [7u64, 42, 999] {
            let mut youngest = f64::INFINITY;
            let mut oldest = 0.0f64;
            for region in 0..8u64 {
                let age = surface_age_gyr(seed, region, super::super::lifetime::MATURE_AGE_GYR);
                assert!(
                    (0.05..=4.5).contains(&age),
                    "seed {seed} region {region}: {age} Gyr"
                );
                assert_eq!(
                    age,
                    surface_age_gyr(seed, region, super::super::lifetime::MATURE_AGE_GYR),
                    "replay differs"
                );
                youngest = youngest.min(age);
                oldest = oldest.max(age);
            }
            assert!(youngest < oldest, "no resurfacing spread at seed {seed}");
        }
        // Young systems read young surfaces (capped by the system age).
        for region in 0..8u64 {
            assert!(
                surface_age_gyr(7, region, 0.5) <= 0.5,
                "young system wears old ground"
            );
        }
    }

    #[test]
    fn transition_and_softening_follow_the_guides() {
        // The Moon (~1.62 m/s²) turns complex near 10 km, Earth near 1.6 km.
        let moon = transition_diameter_m(1.62);
        assert!((9_000.0..12_000.0).contains(&moon), "lunar turn: {moon} m");
        let earth = transition_diameter_m(9.81);
        assert!((1_400.0..1_900.0).contains(&earth), "earth turn: {earth} m");
        assert_eq!(transition_diameter_m(f64::NAN), earth);
        // Fresh craters sharp, old ones relaxed toward the floor.
        assert_eq!(soften_factor(0.0), 1.0);
        assert_eq!(soften_factor(-2.0), 1.0);
        assert!(soften_factor(0.5) > soften_factor(1.5));
        assert!(soften_factor(1.5) > soften_factor(8.0));
        assert_eq!(soften_factor(40.0), SOFTEN_FLOOR);
        // Bowls dip, rims rise, ejecta decays.
        let simple = shape_for(1_000.0, 9.81);
        assert!(!simple.complex && (simple.depth_m - 200.0).abs() < 1e-9);
        assert!((profile_at(&simple, 0.0, 500.0) + 200.0).abs() < 1e-9);
        assert!(profile_at(&simple, 500.0, 500.0) > 0.0, "rim rises");
        let near = profile_at(&simple, 1.2 * 500.0, 500.0);
        let far = profile_at(&simple, 2.2 * 500.0, 500.0);
        assert!(near > far && far > 0.0, "ejecta decays: {near} vs {far}");
        let complex = shape_for(100_000.0, 9.81);
        assert!(complex.complex);
        assert!((profile_at(&complex, 0.0, 50_000.0) + 0.7 * complex.depth_m).abs() < 1.0);
    }

    #[test]
    fn craters_follow_surface_age() {
        // C8 / AC8: crater counts grow with surface age; an old surface is
        // crowded, a young one clean.
        let (young_sum, young_deep, young_count) = coverage(11, 0.2);
        let (old_sum, old_deep, old_count) = coverage(11, 4.0);
        assert!(
            old_sum > 2.0 * young_sum,
            "old {old_sum} vs young {young_sum}"
        );
        assert!(
            old_count > young_count,
            "old {old_count} vs young {young_count} cratered samples"
        );
        // Same age replays; another seed differs. Old ground stays worn
        // and ponded: both keep sharp bowls, the old surface more of them.
        assert_eq!(coverage(11, 4.0), coverage(11, 4.0));
        assert_ne!(coverage(11, 4.0).0, coverage(12, 4.0).0);
        assert!(old_deep < -200.0, "old surface lost its bowls: {old_deep}");
        assert!(
            young_deep < -500.0,
            "young surface lost its bowls: {young_deep}"
        );
        // Degenerate input stays neutral.
        assert_eq!(
            crater_relief_for_age(
                11,
                [f64::NAN, 0.0, 0.0],
                super::super::plates::EARTH_RADIUS_M,
                4.0,
                super::super::classify::WorldKind::EarthLike
            ),
            0.0
        );
        assert_eq!(
            crater_relief_for_age(
                11,
                [0.0, 1.0, 0.0],
                -3.0,
                4.0,
                super::super::classify::WorldKind::EarthLike
            ),
            0.0
        );
        assert_eq!(
            crater_relief_for_age(
                11,
                [0.0, 1.0, 0.0],
                super::super::plates::EARTH_RADIUS_M,
                f64::NAN,
                super::super::classify::WorldKind::EarthLike
            ),
            0.0
        );
    }

    #[test]
    fn rays_light_young_craters_only() {
        // Rays read 0..1, replay, and gather around young big craters.
        let age = super::super::lifetime::MATURE_AGE_GYR;
        let young: f64 = sweep(512)
            .iter()
            .map(|dir| ray_field(11, *dir, super::super::plates::EARTH_RADIUS_M, age))
            .sum();
        assert!(young > 0.0, "no rays on a young surface");
        for dir in sweep(128) {
            let ray = ray_field(11, dir, super::super::plates::EARTH_RADIUS_M, age);
            assert!((0.0..=1.0).contains(&ray), "ray {ray} outside 0..1");
            assert_eq!(
                ray,
                ray_field(11, dir, super::super::plates::EARTH_RADIUS_M, age)
            );
        }
        assert_eq!(
            ray_field(
                11,
                [f64::NAN, 0.0, 0.0],
                super::super::plates::EARTH_RADIUS_M,
                age
            ),
            0.0
        );
    }

    #[test]
    fn crater_cost_stays_within_the_operation_budget() {
        // E-TEST-HERMETIC: cell visits, never wall time.
        assert_eq!(CRATER_CELL_VISITS_PER_SAMPLE, CRATER_CLASSES * 27);
        assert_eq!(RAY_CELL_VISITS_PER_SAMPLE, 2 * 125);
        const _: () = assert!(
            CRATER_CELL_VISITS_PER_SAMPLE + RAY_CELL_VISITS_PER_SAMPLE <= 500,
            "crater schedule grew past the budget"
        );
    }
}
