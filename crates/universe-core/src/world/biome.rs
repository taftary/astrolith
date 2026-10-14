//! Biomes and ground colours: Whittaker lookup, palettes, ice caps, ray
//! colours and vertex colours (#685, T7).
//!
//! Guides: `guides/universe/L10/generation/documents/climate-and-look.md`
//! (Whittaker/Köppen lookup, the ten palettes, plant colour under other
//! stars as a speculative tint, ice caps and snow lines, crater rays as
//! a colour layer). Vegetation needs air and water; airless ground reads
//! rock or ice, waterless ground mineral deserts. Borders wiggle through
//! noise-jittered lookup inputs rather than hard lines; snow follows
//! temperature with height through the lapse rate. Giants wear band
//! fields from [`super::giants`]: their visible surface is the cloud top.
//!
//! Everything is `f64` and deterministic (`E-DET-TIERS`).

use super::classify::WorldKind;
use super::craters::{derive_hit, profile_at, shape_for};
use super::lakes::{BOWL_LAKE_MIN_R_M, LakeKind, lake_kind};
use super::plates::field_for_relief;
use super::relief::base_unclamped_m;
use crate::r#gen::{PlanetInfo, StarKind};
use crate::icosphere::normalize;
use crate::seed::hash_triple;
use crate::surface::SurfaceFrame;

/// Grey fallback with no orbit context (tool-built frames).
pub const GREY_FALLBACK: [f64; 3] = [0.45, 0.45, 0.45];

/// Water share above which a world reads as wet (vegetation, lakes).
pub const WET_WATER_01: f64 = 0.05;

/// Sea-ice forms below this surface temperature in °C.
pub const SEA_ICE_C: f64 = -1.0;

/// Shallow-sea depth in metres for the turquoise end of the gradient.
pub const SHALLOW_M: f64 = 200.0;

/// Deep-sea depth in metres for the navy end of the gradient.
pub const DEEP_M: f64 = 3_000.0;

/// Biome border wiggle: lookup noise octaves and frequency.
pub const BIOME_JITTER_OCTAVES: u32 = 2;
/// Biome jitter field frequency.
pub const BIOME_JITTER_FREQ: f64 = 8.0;

/// Noise evaluations per colour sample (the border wiggle).
pub const BIOME_NOISE_EVALS_PER_SAMPLE: u32 = BIOME_JITTER_OCTAVES;

/// Stream tag of the biome border wiggle.
const BIOME_TAG: u64 = 0xB10E_E685_0000_0000;

/// Land biome (plus rock and ice for airless ground).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Biome {
    /// Mean temperature well below freezing.
    IceCap,
    /// Cold, any rain.
    Tundra,
    /// Cool, moderate rain.
    Boreal,
    /// Mild, wet.
    TempForest,
    /// Mild to warm, moderate rain.
    Grassland,
    /// Warm, dryish.
    Shrubland,
    /// Warm, below the aridity line.
    HotDesert,
    /// Cold, below the aridity line.
    ColdDesert,
    /// Hot, seasonal rain.
    Savanna,
    /// Hot, very wet.
    TropForest,
    /// Airless warm rock.
    Rock,
}

/// Whether `biome` is vegetation (needs air and water; the validator's
/// "no biomes" check reads this).
#[must_use]
pub const fn is_vegetation(biome: Biome) -> bool {
    matches!(
        biome,
        Biome::Tundra
            | Biome::Boreal
            | Biome::TempForest
            | Biome::Grassland
            | Biome::Shrubland
            | Biome::Savanna
            | Biome::TropForest
    )
}

/// Whittaker-style lookup (climate guide, step 3): mean temperature in
/// °C against rain 0..1 into about ten biomes. Vegetation needs both air
/// and water; airless ground reads ice or rock, waterless ground mineral
/// deserts or rock.
#[must_use]
pub fn biome_at(t_c: f64, rain_01: f64, has_air: bool, has_water: bool) -> Biome {
    if !has_air {
        return if t_c < 0.0 {
            Biome::IceCap
        } else {
            Biome::Rock
        };
    }
    if !has_water {
        if t_c >= 18.0 {
            return Biome::HotDesert;
        }
        return if t_c < 0.0 {
            Biome::ColdDesert
        } else {
            Biome::Rock
        };
    }
    if t_c < -10.0 {
        Biome::IceCap
    } else if t_c < 0.0 {
        if rain_01 < 0.2 {
            Biome::ColdDesert
        } else {
            Biome::Tundra
        }
    } else if t_c < 8.0 {
        if rain_01 < 0.2 {
            Biome::ColdDesert
        } else if rain_01 < 0.55 {
            Biome::Boreal
        } else {
            Biome::TempForest
        }
    } else if t_c < 18.0 {
        if rain_01 < 0.15 {
            Biome::Grassland
        } else {
            Biome::TempForest
        }
    } else if t_c < 25.0 {
        if rain_01 < 0.2 {
            Biome::HotDesert
        } else if rain_01 < 0.4 {
            Biome::Shrubland
        } else if rain_01 < 0.6 {
            Biome::Grassland
        } else if rain_01 < 0.8 {
            Biome::Savanna
        } else {
            Biome::TempForest
        }
    } else if rain_01 < 0.2 {
        Biome::HotDesert
    } else if rain_01 < 0.4 {
        Biome::Shrubland
    } else if rain_01 < 0.7 {
        Biome::Savanna
    } else {
        Biome::TropForest
    }
}

/// Vegetation tint under other stars (climate guide, step 4, GEN-S048):
/// green for Sun-like stars, bluer for hotter ones, redder and darker
/// for cool red stars. An artistic reading of the research, speculative
/// in any in-game text (noted for the lab readout and the guides).
#[must_use]
pub fn vegetation_tint(star: StarKind) -> [f64; 3] {
    match star {
        StarKind::O | StarKind::B | StarKind::A => [0.85, 0.92, 1.05],
        StarKind::F => [0.9, 0.95, 1.05],
        StarKind::G => [1.0, 1.0, 1.0],
        StarKind::K => [1.0, 0.78, 0.62],
        StarKind::M => [0.6, 0.5, 0.55],
    }
}

/// Ground palette in linear RGB (climate guide, step 3 table).
#[must_use]
pub fn palette(biome: Biome, star: StarKind) -> [f64; 3] {
    let tint = vegetation_tint(star);
    let tinted = |rgb: [f64; 3]| {
        if is_vegetation(biome) {
            [rgb[0] * tint[0], rgb[1] * tint[1], rgb[2] * tint[2]]
        } else {
            rgb
        }
    };
    tinted(match biome {
        Biome::IceCap => [0.92, 0.95, 1.0],
        Biome::Tundra => [0.45, 0.47, 0.32],
        Biome::Boreal => [0.10, 0.28, 0.12],
        Biome::TempForest => [0.13, 0.42, 0.16],
        Biome::Grassland => [0.55, 0.62, 0.25],
        Biome::Shrubland => [0.5, 0.48, 0.3],
        Biome::HotDesert => [0.85, 0.65, 0.4],
        Biome::ColdDesert => [0.6, 0.58, 0.5],
        Biome::Savanna => [0.65, 0.55, 0.28],
        Biome::TropForest => [0.05, 0.32, 0.1],
        Biome::Rock => [0.45, 0.42, 0.4],
    })
}

/// Shallow-sea colour (turquoise over the shelf).
pub const SEA_SHALLOW: [f64; 3] = [0.1, 0.45, 0.55];

/// Deep-sea colour (navy over the abyss).
pub const SEA_DEEP: [f64; 3] = [0.02, 0.12, 0.3];

/// Sea-ice colour.
pub const SEA_ICE: [f64; 3] = [0.85, 0.92, 0.97];

/// Dark basalt ground (lava worlds).
pub const BASALT: [f64; 3] = [0.16, 0.13, 0.13];

/// Fresh lake blue.
pub const LAKE_FRESH: [f64; 3] = [0.08, 0.3, 0.6];

/// Salty closed-lake pale.
pub const LAKE_SALTY: [f64; 3] = [0.6, 0.75, 0.7];

/// Dry playa salt.
pub const PLAYA_SALT: [f64; 3] = [0.88, 0.85, 0.75];

/// Lava lake glow.
pub const LAVA_GLOW: [f64; 3] = [0.95, 0.28, 0.06];

/// Fresh-ray brightening (whitens ground under young ejecta).
pub const RAY_WHITE: [f64; 3] = [0.95, 0.95, 0.93];

/// Water colour for a lake kind (lava on heat-pipe worlds).
fn lake_water(kind: LakeKind, lava: bool) -> [f64; 3] {
    if lava {
        return LAVA_GLOW;
    }
    match kind {
        LakeKind::Fresh => LAKE_FRESH,
        LakeKind::Salty => LAKE_SALTY,
        LakeKind::Dry => PLAYA_SALT,
        LakeKind::Lava => LAVA_GLOW,
    }
}

/// The generator recipe for a frame with orbit context (`None` for
/// tool-built frames): the planet's type, size, air, relief and moons
/// inputs, shared by the colour pass and its tests.
#[must_use]
pub fn recipe_for_frame(frame: &SurfaceFrame) -> Option<super::WorldRecipe> {
    let orbit = frame.orbit()?;
    let info = PlanetInfo {
        radius_earth: frame.radius_earth(),
        orbit_au: orbit.orbit_au,
        period_days: orbit.period_days,
    };
    Some(super::planet_recipe(
        frame.planet_seed(),
        &info,
        orbit.system_seed,
        orbit.home,
    ))
}

/// Ground colour in linear RGB under `direction` at `elev_m` metres above
/// the mean radius: seas by depth, lakes by kind, ice where freezing,
/// biomes by temperature and rain, rays whitening young ejecta. Needs the
/// frame's orbit context; tool-built frames without one read grey.
#[must_use]
pub fn ground_colour(frame: &SurfaceFrame, direction: [f64; 3], elev_m: f64) -> [f64; 3] {
    if !direction.iter().all(|c| c.is_finite()) || !elev_m.is_finite() {
        return GREY_FALLBACK;
    }
    let unit = normalize(direction);
    let seed = frame.planet_seed();
    let radius_m = frame.radius_earth() * super::plates::EARTH_RADIUS_M;
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return GREY_FALLBACK;
    }
    let orbit = frame.orbit();
    let recipe = recipe_for_frame(frame);
    let (Some(orbit), Some(recipe)) = (orbit, recipe) else {
        return GREY_FALLBACK;
    };
    // Giants wear band fields, not ground (T9): the visible surface is
    // the cloud top, coloured by spin, storms and day side (locking stays
    // unwired until T11, so the day side reads uniform).
    if matches!(
        recipe.kind,
        WorldKind::IceGiant | WorldKind::GasGiant | WorldKind::HotJupiter | WorldKind::SubNeptune
    ) {
        let body = crate::terrain::body_recipe(seed, orbit.home);
        let lat = unit[1].clamp(-1.0, 1.0).asin();
        let lon = unit[2].atan2(unit[0]);
        return super::giants::band_colour(seed, recipe.kind, lat, lon, body.spin_hours, None);
    }
    let air = super::air::air_for(
        seed,
        recipe.kind,
        &super::air::AirInputs {
            mass_earth: recipe.mass_earth,
            radius_earth: recipe.radius_earth,
            orbit_au: recipe.orbit_au,
            star_mass_solar: recipe.star_mass,
            t_eq_k: recipe.t_eq_k,
            water_01: recipe.water,
            age_gyr: recipe.age_gyr,
        },
    );
    let body = crate::terrain::body_recipe(seed, orbit.home);
    let star = crate::system::system_star(orbit.system_seed, orbit.home);
    let climate = super::climate::climate_for(&super::climate::ClimateInputs {
        t_eq_k: recipe.t_eq_k,
        greenhouse_k: air.greenhouse_k,
        pressure_bar: air.pressure_bar,
        water_01: recipe.water,
        spin_hours: body.spin_hours,
        tilt_deg: body.tilt_deg,
        locked: false,
    });
    let lat = unit[1].clamp(-1.0, 1.0).asin();
    let lava_world = recipe.kind == WorldKind::Lava
        || super::lakes::surface_liquid(seed) == super::lakes::LakeLiquid::Lava;
    // Seas by depth (frozen where freezing; lava seas on lava worlds).
    if elev_m < 0.0 {
        if lava_world {
            return LAVA_GLOW;
        }
        if !air.present {
            return palette(Biome::IceCap, star.kind);
        }
        let surface = super::climate::temperature_c(&climate, lat, 0.0, recipe.gravity, None);
        if surface < SEA_ICE_C {
            return SEA_ICE;
        }
        let depth = (-elev_m).clamp(0.0, DEEP_M) / DEEP_M;
        return [
            SEA_SHALLOW[0] + (SEA_DEEP[0] - SEA_SHALLOW[0]) * depth,
            SEA_SHALLOW[1] + (SEA_DEEP[1] - SEA_SHALLOW[1]) * depth,
            SEA_SHALLOW[2] + (SEA_DEEP[2] - SEA_SHALLOW[2]) * depth,
        ];
    }
    if lava_world {
        return BASALT;
    }
    // Still water: rift reaches by kind, then crater bowls re-derived
    // from the shared lattice (so water sits in real bowls).
    let field = field_for_relief(seed, radius_m);
    if let Some(colour) = rift_water(
        seed,
        field.as_ref(),
        unit,
        radius_m,
        elev_m,
        lava_world,
        recipe.kind,
    ) {
        return colour;
    }
    if let Some(colour) = bowl_water(
        field.as_ref(),
        seed,
        unit,
        radius_m,
        elev_m,
        lava_world,
        recipe.kind,
    ) {
        return colour;
    }
    // Land biomes by temperature and rain, borders wiggling with noise.
    let shadow = field.as_ref().map_or(0.0, |plates| {
        super::climate::rain_shadow_01(plates, unit, radius_m, climate.belt_lat_deg)
    });
    let rain = super::climate::rain_01(&climate, lat, shadow, elev_m, true);
    let (jitter, _) = super::noise3::fbm_3d(
        hash_triple(seed, BIOME_TAG, 0),
        [
            unit[0] * BIOME_JITTER_FREQ,
            unit[1] * BIOME_JITTER_FREQ,
            unit[2] * BIOME_JITTER_FREQ,
        ],
        BIOME_JITTER_OCTAVES,
        2.0,
        0.5,
    );
    let temp = super::climate::temperature_c(&climate, lat, elev_m, recipe.gravity, None);
    let biome = biome_at(
        temp * (1.0 + 0.05 * jitter),
        (rain * (1.0 + 0.1 * jitter)).clamp(0.0, 1.0),
        air.present,
        recipe.water > WET_WATER_01,
    );
    let mut colour = palette(biome, star.kind);
    // Bright rays whiten young ejecta (colour layer for the T5 field).
    let ray = super::craters::ray_field(seed, unit, radius_m).clamp(0.0, 1.0);
    if ray > 0.0 {
        let whiten = (ray * 0.6).min(1.0);
        colour = [
            colour[0] + (RAY_WHITE[0] - colour[0]) * whiten,
            colour[1] + (RAY_WHITE[1] - colour[1]) * whiten,
            colour[2] + (RAY_WHITE[2] - colour[2]) * whiten,
        ];
    }
    colour
}

/// Still water of the sample's region rift lakes, if `direction` lies
/// inside one below its level.
fn rift_water(
    seed: u64,
    field: Option<&super::plates::Plates>,
    unit: [f64; 3],
    radius_m: f64,
    elev_m: f64,
    lava: bool,
    kind: WorldKind,
) -> Option<[f64; 3]> {
    super::landforms::region_lakes(seed, unit, radius_m, field, kind)
        .into_iter()
        .find(|lake| {
            (unit[0] * lake.centre[0] + unit[1] * lake.centre[1] + unit[2] * lake.centre[2])
                .clamp(-1.0, 1.0)
                .acos()
                < lake.radius_ang
                && elev_m < lake.level_m + 1.0
        })
        .map(|lake| lake_water(lake.kind, lava))
}

/// Still water of crater bowls, re-derived from the shared lattice
/// (same existence ages as the relief pass, so water sits in real bowls).
fn bowl_water(
    field: Option<&super::plates::Plates>,
    seed: u64,
    unit: [f64; 3],
    radius_m: f64,
    elev_m: f64,
    lava: bool,
    kind: WorldKind,
) -> Option<[f64; 3]> {
    use super::craters::CLASS_RADII_M;
    let gravity = super::relief::gravity_for(seed);
    if !super::lakes::has_air(radius_m) {
        return None;
    }
    let water_01 = super::classify::draw_body(seed).water;
    let sample = super::craters::sample_age(field, seed, unit);
    for (c, class_r) in CLASS_RADII_M.iter().enumerate() {
        let class_seed = hash_triple(seed, super::craters::CRATER_TAG, c as u64);
        let radius_ang = class_r / radius_m;
        if radius_ang > 0.5 {
            continue;
        }
        let cell = 8.0 * radius_ang;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: lattice coords near the origin always fit"
        )]
        let base = [
            (unit[0] / cell).floor() as i64,
            (unit[1] / cell).floor() as i64,
            (unit[2] / cell).floor() as i64,
        ];
        for dx in -1i64..=1 {
            for dy in -1i64..=1 {
                for dz in -1i64..=1 {
                    let lattice = [base[0] + dx, base[1] + dy, base[2] + dz];
                    let Some(hit) =
                        derive_hit(class_seed, lattice, cell, *class_r, radius_m, sample)
                    else {
                        continue;
                    };
                    if hit.radius_m < BOWL_LAKE_MIN_R_M {
                        continue;
                    }
                    let rho = (unit[0] * hit.centre[0]
                        + unit[1] * hit.centre[1]
                        + unit[2] * hit.centre[2])
                        .clamp(-1.0, 1.0)
                        .acos();
                    if rho > 2.5 * hit.radius_ang {
                        continue;
                    }
                    let shape = shape_for(2.0 * hit.radius_m, gravity);
                    let floor = profile_at(&shape, 0.0, hit.radius_m) * hit.soft;
                    let rim = 0.08 * hit.radius_m * hit.soft;
                    let fill = super::lakes::lake_fill(water_01, 1.2);
                    let centre_base = base_unclamped_m(seed, hit.centre, kind);
                    let level = centre_base + floor + fill * (rim - floor);
                    if elev_m < level {
                        let kind = lake_kind(fill);
                        return Some(lake_water(kind, lava));
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::look::{ReliefFactor, WorldMode, WorldOptions};
    use crate::surface::OrbitContext;

    /// A Generated frame with an Earth-like orbit context.
    fn earth_frame(seed: u64) -> SurfaceFrame {
        SurfaceFrame::planet(seed)
            .with_world(WorldOptions {
                mode: WorldMode::Generated,
                relief: ReliefFactor::default(),
            })
            .with_orbit(OrbitContext {
                orbit_au: 1.0,
                period_days: 365.25,
                system_seed: 9,
                home: false,
            })
    }

    /// Colours over a latitude sweep (fixed longitudes).
    fn sweep(frame: &SurfaceFrame, lat_deg: f64, count: u32) -> Vec<[f64; 3]> {
        use std::f64::consts::TAU;
        let recipe = recipe_for_frame(frame);
        let kind = recipe.map_or(WorldKind::EarthLike, |recipe| recipe.kind);
        let mut out = Vec::new();
        for i in 0..count {
            let lon = TAU * f64::from(i) / f64::from(count.max(1));
            let lat = lat_deg.to_radians();
            let dir = normalize([lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin()]);
            let elev = crate::world::relief::total_relief_m(
                frame.planet_seed(),
                dir,
                frame.radius_earth() * crate::world::plates::EARTH_RADIUS_M,
                Some(kind),
            );
            out.push(ground_colour(frame, dir, elev));
        }
        out
    }

    #[test]
    fn biomes_follow_climate() {
        // C10 / AC10 (land part): Whittaker rows, deserts in dry belts
        // and rain shadows, ice caps following cold, no vegetation
        // without air and water.
        assert_eq!(biome_at(25.0, 0.8, true, true), Biome::TropForest);
        assert_eq!(biome_at(25.0, 0.1, true, true), Biome::HotDesert);
        assert_eq!(biome_at(-15.0, 0.5, true, true), Biome::IceCap);
        assert_eq!(biome_at(10.0, 0.5, true, true), Biome::TempForest);
        assert_eq!(biome_at(20.0, 0.5, false, true), Biome::Rock);
        assert_eq!(biome_at(20.0, 0.5, true, false), Biome::HotDesert);
        assert_eq!(biome_at(-20.0, 0.0, false, false), Biome::IceCap);
        // Vegetation reads green under a Sun-like star.
        let leaf = palette(Biome::TempForest, StarKind::G);
        assert!(
            leaf[1] > leaf[0] + 0.15 && leaf[1] > leaf[2] + 0.1,
            "{leaf:?}"
        );
        assert_eq!(
            ground_colour(&earth_frame(7), [f64::NAN, 0.0, 0.0], 0.0),
            GREY_FALLBACK
        );
        // Ice caps whiten the poles, leaf colours the tropics, deserts
        // gather in the dry belts: counted over seeds, so the test proves
        // Earth-like worlds exist and look right (not one lucky seed).
        let mut good = 0u32;
        let mut belt_desert = 0u32;
        let mut low_desert = 0u32;
        for seed in 0..120u64 {
            let frame = earth_frame(seed);
            let mut polar_ice = 0u32;
            for colour in sweep(&frame, 80.0, 32) {
                if colour[0] > 0.8 && colour[1] > 0.8 && colour[2] > 0.8 {
                    polar_ice += 1;
                }
            }
            let mut tropical_green = 0u32;
            for colour in sweep(&frame, 5.0, 32) {
                if colour[1] > colour[0] + 0.1 && colour[1] > colour[2] + 0.05 {
                    tropical_green += 1;
                }
            }
            if polar_ice > 16 && tropical_green > 4 {
                good += 1;
                for lat in [25.0, 30.0, 35.0] {
                    for colour in sweep(&frame, lat, 16) {
                        if colour[0] > 0.7 && colour[0] > colour[1] && colour[1] > colour[2] {
                            belt_desert += 1;
                        }
                    }
                }
                for lat in [0.0, 55.0] {
                    for colour in sweep(&frame, lat, 16) {
                        if colour[0] > 0.7 && colour[0] > colour[1] && colour[1] > colour[2] {
                            low_desert += 1;
                        }
                    }
                }
            }
        }
        assert!(good >= 3, "no Earth-like worlds in range: {good}");
        assert!(belt_desert > low_desert, "deserts outside the belts");
        // Same seed, same map.
        let frame = earth_frame(7);
        assert_eq!(sweep(&frame, 12.0, 16), sweep(&frame, 12.0, 16));
    }

    #[test]
    fn colour_cost_stays_within_the_operation_budget() {
        // E-TEST-HERMETIC: noise evaluations per ground-plus-colour
        // sample, never wall time. Relief, plates, landform textures and
        // the biome border wiggle share one budget of 40.
        const _: () = assert!(
            super::super::relief::NOISE_EVALS_PER_SAMPLE
                + super::super::plates::PLATE_NOISE_EVALS_PER_SAMPLE
                + super::super::landforms::LANDFORM_NOISE_EVALS_PER_SAMPLE
                + BIOME_NOISE_EVALS_PER_SAMPLE
                <= 40,
            "ground schedule grew past the budget"
        );
    }

    #[test]
    fn airless_worlds_wear_no_vegetation() {
        // A Mercury-like orbit context loses its air: rock and ice only.
        let frame = SurfaceFrame::planet(11)
            .with_world(WorldOptions {
                mode: WorldMode::Generated,
                relief: ReliefFactor::default(),
            })
            .with_orbit(OrbitContext {
                orbit_au: 0.39,
                period_days: 88.0,
                system_seed: 5,
                home: false,
            });
        for lat in [-60.0, -20.0, 0.0, 20.0, 60.0] {
            for colour in sweep(&frame, lat, 32) {
                assert!(
                    !(colour[1] > colour[0] + 0.15 && colour[1] > colour[2] + 0.1),
                    "vegetation on an airless world: {colour:?}"
                );
            }
        }
    }
}
