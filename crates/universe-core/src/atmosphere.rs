//! The planet's air as one atmosphere model (#780 Q3 A).
//!
//! [`AirModel`](crate::atmosphere::AirModel) turns a planet recipe's
//! [`AirInfo`](crate::terrain::BodyRecipe::air) into the numbers a physical sky needs:
//! the planet and air radii in metres, Rayleigh and Mie scattering with
//! their scale heights, and an ozone layer. Earth-like thin air uses Earth's
//! values; thick air scatters more and blue less (amber); airless worlds have
//! none. The window's atmosphere is built from the same numbers.
//!
//! A small single-scattering reference evaluates the same model on the CPU
//! at any height: the sky straight overhead, the ground light under the sun
//! and the haze toward the horizon, per unit of sunlight. The headless
//! `LIGHT` line reads it frame by frame. Pure and deterministic
//! (`E-CORE-NO-BEVY`, `E-DET-TIERS`).

use crate::r#gen::AirInfo;

/// Earth's Rayleigh scattering at sea level, per metre (linear RGB).
pub const EARTH_RAYLEIGH: [f64; 3] = [5.802e-6, 13.558e-6, 33.1e-6];

/// Earth's Rayleigh scale height in metres.
pub const EARTH_RAYLEIGH_HEIGHT_M: f64 = 8_000.0;

/// Earth's Mie scattering at sea level, per metre.
pub const EARTH_MIE_SCATTERING: f64 = 3.996e-6;

/// Earth's Mie absorption at sea level, per metre.
pub const EARTH_MIE_ABSORPTION: f64 = 0.444e-6;

/// Earth's Mie scale height in metres.
pub const EARTH_MIE_HEIGHT_M: f64 = 1_200.0;

/// Mie phase asymmetry (forward scattering).
pub const MIE_ASYMMETRY: f64 = 0.8;

/// Earth's ozone absorption at the layer peak, per metre.
pub const EARTH_OZONE: [f64; 3] = [0.650e-6, 1.881e-6, 0.085e-6];

/// Earth's ozone layer centre and half width in metres.
pub const EARTH_OZONE_CENTRE_M: f64 = 25_000.0;

/// Earth's ozone layer half width in metres.
pub const EARTH_OZONE_HALF_WIDTH_M: f64 = 15_000.0;

/// Thick air: Rayleigh share of Earth's (less blue).
pub const THICK_RAYLEIGH_SHARE: f64 = 0.35;

/// Thick air: Mie share of Earth's (more haze).
pub const THICK_MIE_SHARE: f64 = 6.0;

/// Average ground albedo used for multiple scattering (linear RGB).
pub const GROUND_ALBEDO: [f64; 3] = [0.3, 0.3, 0.3];

/// Samples along a view ray in the reference.
const VIEW_SAMPLES: u32 = 32;

/// Samples along a sun ray in the reference.
const SUN_SAMPLES: u32 = 12;

/// One planet's air (metres, per-metre coefficients).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AirModel {
    /// Planet mean radius (sea level).
    pub sea_radius_m: f64,
    /// Bottom of the air: the lowest the relief can reach, so no ground
    /// lies below it (ocean floors are ground, there is no water yet).
    pub inner_radius_m: f64,
    /// Top of the air.
    pub outer_radius_m: f64,
    /// Rayleigh scattering at sea level and below (linear RGB).
    pub rayleigh: [f64; 3],
    /// Rayleigh scale height.
    pub rayleigh_height_m: f64,
    /// Mie scattering at the ground.
    pub mie_scattering: f64,
    /// Mie absorption at the ground (linear RGB, amber for thick air).
    pub mie_absorption: [f64; 3],
    /// Mie scale height.
    pub mie_height_m: f64,
    /// Ozone absorption at the layer peak (linear RGB).
    pub ozone: [f64; 3],
    /// Ozone layer centre height above sea level.
    pub ozone_centre_m: f64,
    /// Ozone layer half width (tent profile).
    pub ozone_half_width_m: f64,
    /// Ground albedo for multiple scattering.
    pub ground_albedo: [f64; 3],
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn mul(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}

fn exp3(a: [f64; 3]) -> [f64; 3] {
    [(-a[0]).exp(), (-a[1]).exp(), (-a[2]).exp()]
}

/// Ray parameters `(near, far)` where the ray `p + t d` (unit `d`) meets the
/// sphere of radius `r` around the origin, if it does.
fn sphere_hits(p: [f64; 3], d: [f64; 3], r: f64) -> Option<(f64, f64)> {
    let b = dot(p, d);
    let c = dot(p, p) - r * r;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let root = disc.sqrt();
    Some((-b - root, -b + root))
}

impl AirModel {
    /// The model for a planet of `radius_m` with `air` (`None` when airless)
    /// whose lowest ground lies `floor_m` (zero or below) from sea level.
    ///
    /// Heights scale with the air's thickness against Earth's 100 km; thin
    /// air keeps Earth's coefficients, thick air (amber tint) scatters less
    /// blue and more haze, with absorption following the tint. Below sea
    /// level the air keeps its sea-level density.
    #[must_use]
    pub fn new(radius_m: f64, air: Option<AirInfo>, floor_m: f64) -> Option<AirModel> {
        let air = air?;
        if !(radius_m.is_finite() && radius_m > 0.0 && air.thickness_km > 0.0) {
            return None;
        }
        let floor_m = if floor_m.is_finite() {
            floor_m.min(0.0)
        } else {
            0.0
        };
        let stretch = air.thickness_km / crate::terrain::EARTH_AIR_THICKNESS_KM;
        let thick = air.tint == crate::terrain::THICK_AIR_TINT;
        let (rayleigh_share, mie_share) = if thick {
            (THICK_RAYLEIGH_SHARE, THICK_MIE_SHARE)
        } else {
            (1.0, 1.0)
        };
        // Absorption removes what the tint lacks: amber air eats blue.
        let tint_loss = [
            1.0 - air.tint[0].clamp(0.0, 1.0),
            1.0 - air.tint[1].clamp(0.0, 1.0),
            1.0 - air.tint[2].clamp(0.0, 1.0),
        ];
        let mie_absorption = if thick {
            scale(
                add([1.0; 3], scale(tint_loss, 4.0)),
                EARTH_MIE_ABSORPTION * mie_share,
            )
        } else {
            [EARTH_MIE_ABSORPTION; 3]
        };
        Some(AirModel {
            sea_radius_m: radius_m,
            inner_radius_m: radius_m + floor_m,
            outer_radius_m: radius_m + air.thickness_km * 1_000.0,
            rayleigh: scale(EARTH_RAYLEIGH, rayleigh_share),
            rayleigh_height_m: EARTH_RAYLEIGH_HEIGHT_M * stretch,
            mie_scattering: EARTH_MIE_SCATTERING * mie_share,
            mie_absorption,
            mie_height_m: EARTH_MIE_HEIGHT_M * stretch,
            ozone: EARTH_OZONE,
            ozone_centre_m: EARTH_OZONE_CENTRE_M * stretch,
            ozone_half_width_m: EARTH_OZONE_HALF_WIDTH_M * stretch,
            ground_albedo: GROUND_ALBEDO,
        })
    }

    /// The model for the L10 planet at `chain` under `root`, from the same
    /// recipe as its [`crate::planet_frame::PlanetFrame`].
    #[must_use]
    pub fn for_planet(root: u64, chain: &[u32]) -> Option<AirModel> {
        let seed = crate::nest::path_seed(root, chain);
        let home = crate::home::is_home_planet_cell(root, chain);
        let recipe = crate::terrain::body_recipe(seed, home);
        let frame = crate::planet_frame::PlanetFrame::for_radius_earth(recipe.radius_earth);
        let floor = crate::relief::Relief::for_planet(root, chain)
            .height_bounds_m()
            .0;
        AirModel::new(frame.radius_m(), recipe.air, floor)
    }

    /// Air depth from its bottom to its top, in metres.
    #[must_use]
    pub fn thickness_m(&self) -> f64 {
        self.outer_radius_m - self.inner_radius_m
    }

    /// Exponential density with scale height `scale_m` at `sea_h` above sea
    /// level: 1 at sea level and below, shifted to reach 0 at the top.
    fn exponential(&self, sea_h: f64, scale_m: f64) -> f64 {
        let top_h = self.outer_radius_m - self.sea_radius_m;
        let top = (-top_h / scale_m).exp();
        (((-sea_h.max(0.0) / scale_m).exp() - top) / (1.0 - top)).clamp(0.0, 1.0)
    }

    /// The three density profiles `(rayleigh, mie, ozone)`, each 0 to 1, at
    /// `h` metres above the bottom of the air: the one place both the
    /// reference and the window's medium read them.
    #[must_use]
    pub fn densities(&self, h: f64) -> (f64, f64, f64) {
        let sea_h = h + self.inner_radius_m - self.sea_radius_m;
        let ozone = (1.0 - (sea_h - self.ozone_centre_m).abs() / self.ozone_half_width_m).max(0.0);
        (
            self.exponential(sea_h, self.rayleigh_height_m),
            self.exponential(sea_h, self.mie_height_m),
            ozone,
        )
    }

    /// The densities at Bevy's falloff parameter `p` (1 at the bottom of
    /// the air, 0 at its top, linear in height in between).
    #[must_use]
    pub fn densities_at_falloff(&self, p: f64) -> (f64, f64, f64) {
        self.densities((1.0 - p.clamp(0.0, 1.0)) * self.thickness_m())
    }

    /// Scattering (Rayleigh RGB, Mie) and extinction at `h` above the bottom.
    fn medium_at(&self, h: f64) -> ([f64; 3], f64, [f64; 3]) {
        let (dr, dm, dz) = self.densities(h.clamp(0.0, self.thickness_m()));
        let rayleigh = scale(self.rayleigh, dr);
        let mie = self.mie_scattering * dm;
        let extinction = add(
            add(rayleigh, [mie; 3]),
            add(scale(self.mie_absorption, dm), scale(self.ozone, dz)),
        );
        (rayleigh, mie, extinction)
    }

    /// The span of the ray `p + t d` inside the air, cut at the ground, and
    /// whether it ends on the ground.
    fn air_span(&self, p: [f64; 3], d: [f64; 3]) -> Option<(f64, f64, bool)> {
        let (near, far) = sphere_hits(p, d, self.outer_radius_m)?;
        let start = near.max(0.0);
        if far <= start {
            return None;
        }
        let ground = sphere_hits(p, d, self.inner_radius_m)
            .map(|(g, _)| g)
            .filter(|g| *g > 0.0);
        Some(match ground {
            Some(g) if g < far => (start, g.max(start), true),
            _ => (start, far, false),
        })
    }

    /// Transmittance from `p` toward the sun along unit `sun`; zero when the
    /// planet blocks it.
    fn sun_transmittance(&self, p: [f64; 3], sun: [f64; 3]) -> [f64; 3] {
        let Some((start, end, blocked)) = self.air_span(p, sun) else {
            // Above the air looking away, or past it: unattenuated unless
            // the planet is in the way.
            let shadowed = sphere_hits(p, sun, self.inner_radius_m).is_some_and(|(g, _)| g > 0.0);
            return if shadowed { [0.0; 3] } else { [1.0; 3] };
        };
        if blocked {
            return [0.0; 3];
        }
        let step = (end - start) / f64::from(SUN_SAMPLES);
        let mut depth = [0.0; 3];
        for i in 0..SUN_SAMPLES {
            let t = start + (f64::from(i) + 0.5) * step;
            let q = add(p, scale(sun, t));
            let h = dot(q, q).sqrt() - self.inner_radius_m;
            depth = add(depth, scale(self.medium_at(h).2, step));
        }
        exp3(depth)
    }

    /// Single-scattered light reaching `p` along `-d` (looking along unit
    /// `d`) up to `limit` metres, per unit sun irradiance, and the
    /// transmittance to the end of the ray (the ground, or space).
    fn inscatter(
        &self,
        p: [f64; 3],
        d: [f64; 3],
        sun: [f64; 3],
        limit: f64,
    ) -> ([f64; 3], [f64; 3], bool) {
        let Some((start, end, ground)) = self.air_span(p, d) else {
            return ([0.0; 3], [1.0; 3], false);
        };
        let (end, ground) = if limit < end {
            (limit, true)
        } else {
            (end, ground)
        };
        if end <= start {
            return ([0.0; 3], [1.0; 3], ground);
        }
        let mu = dot(d, sun);
        let phase_r = 3.0 / (16.0 * std::f64::consts::PI) * (1.0 + mu * mu);
        let g = MIE_ASYMMETRY;
        let phase_m =
            (1.0 - g * g) / (4.0 * std::f64::consts::PI * (1.0 + g * g - 2.0 * g * mu).powf(1.5));
        let step = (end - start) / f64::from(VIEW_SAMPLES);
        let mut light = [0.0; 3];
        let mut depth = [0.0; 3];
        for i in 0..VIEW_SAMPLES {
            let t = start + (f64::from(i) + 0.5) * step;
            let q = add(p, scale(d, t));
            let h = dot(q, q).sqrt() - self.inner_radius_m;
            let (rayleigh, mie, extinction) = self.medium_at(h);
            let half = scale(extinction, step * 0.5);
            let view = exp3(add(depth, half));
            let scatter = add(scale(rayleigh, phase_r), [mie * phase_m; 3]);
            let lit = mul(mul(view, self.sun_transmittance(q, sun)), scatter);
            light = add(light, scale(lit, step));
            depth = add(depth, scale(extinction, step));
        }
        (light, exp3(depth), ground)
    }

    /// Light on flat ground at `ground_up`, `ground_m` from sea level, from
    /// the sun (unit `sun`): direct sunlight after the air, plus the night
    /// floor ([`crate::light::NIGHT_SHARE`] of the zenith daylight), per
    /// unit sun.
    #[must_use]
    pub fn ground_light(&self, ground_up: [f64; 3], ground_m: f64, sun: [f64; 3]) -> [f64; 3] {
        let radius = (self.sea_radius_m + ground_m).max(self.inner_radius_m) + 0.01;
        let at = scale(ground_up, radius);
        let cosine = dot(ground_up, sun).max(0.0);
        let direct = scale(self.sun_transmittance(at, sun), cosine);
        let zenith = self.sun_transmittance(at, ground_up);
        add(direct, scale(zenith, crate::light::NIGHT_SHARE))
    }
}

/// What the camera sees at one point, per unit sun (linear RGB).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SkySample {
    /// Sky straight overhead (black above the air).
    pub overhead: [f64; 3],
    /// The ground straight below as seen from the camera: its light (times
    /// the albedo) through the air, plus the air's own glow on the way.
    pub ground: [f64; 3],
    /// Haze toward the horizon, toward the sun's side.
    pub haze: [f64; 3],
}

/// Samples the model at `up` (unit, from the planet centre) with the camera
/// `height_m` and the ground under it `ground_m` from sea level, and the
/// sun along unit `sun`. Without air the sky is black and the ground gets
/// the plain sunlight plus the night floor.
#[must_use]
pub fn sample(
    air: Option<&AirModel>,
    up: [f64; 3],
    height_m: f64,
    ground_m: f64,
    sun: [f64; 3],
) -> SkySample {
    let Some(air) = air else {
        let lit = dot(up, sun).max(0.0) + crate::light::NIGHT_SHARE;
        return SkySample {
            overhead: [0.0; 3],
            ground: scale(GROUND_ALBEDO, lit),
            haze: [0.0; 3],
        };
    };
    let height_m = height_m.max(ground_m);
    let p = scale(up, air.sea_radius_m + height_m);
    let (overhead, _, _) = air.inscatter(p, up, sun, f64::INFINITY);
    let down = scale(up, -1.0);
    let (glow, through, _) = air.inscatter(p, down, sun, height_m - ground_m);
    let ground = add(
        mul(
            mul(air.ground_light(up, ground_m, sun), air.ground_albedo),
            through,
        ),
        glow,
    );
    // Toward the horizon on the sun's side (any side when the sun is
    // straight up or down).
    let along = add(sun, scale(up, -dot(sun, up)));
    let len = dot(along, along).sqrt();
    let side = if len > 1e-9 {
        scale(along, 1.0 / len)
    } else {
        let pick = if up[0].abs() < 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        };
        let a = add(pick, scale(up, -dot(pick, up)));
        scale(a, 1.0 / dot(a, a).sqrt())
    };
    let (haze, _, _) = air.inscatter(p, side, sun, f64::INFINITY);
    SkySample {
        overhead,
        ground,
        haze,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::{THICK_AIR_TINT, THIN_AIR_TINT};

    const EARTH_R: f64 = 6_371_000.0;

    fn earth() -> AirModel {
        AirModel::new(
            EARTH_R,
            Some(AirInfo {
                thickness_km: 100.0,
                tint: THIN_AIR_TINT,
            }),
            -11_000.0,
        )
        .expect("earth air")
    }

    #[test]
    fn atmosphere_follows_the_planet_air() {
        // Q3 A: Earth thin blue, thick amber, airless none.
        let thin = earth();
        assert_eq!(thin.rayleigh, EARTH_RAYLEIGH, "Earth keeps Earth's air");
        assert!((thin.outer_radius_m - thin.sea_radius_m - 100_000.0).abs() < 1e-6);
        assert!(
            (thin.sea_radius_m - thin.inner_radius_m - 11_000.0).abs() < 1e-6,
            "the air reaches the deepest floor"
        );
        // Below sea level the air keeps its sea-level density.
        let below = thin.densities(1_000.0);
        let sea = thin.densities(11_000.0);
        assert!((below.0 - sea.0).abs() < 1e-12 && (below.1 - sea.1).abs() < 1e-12);
        assert!(thin.densities_at_falloff(0.0).0 < 1e-5, "thin at the top");
        let thick = AirModel::new(
            3.5 * EARTH_R,
            Some(AirInfo {
                thickness_km: 350.0,
                tint: THICK_AIR_TINT,
            }),
            0.0,
        )
        .expect("thick air");
        assert!(thick.rayleigh[2] < thin.rayleigh[2], "thick air less blue");
        assert!(
            thick.mie_scattering > thin.mie_scattering,
            "thick air hazier"
        );
        assert!(
            thick.mie_absorption[2] > thick.mie_absorption[0],
            "amber air eats blue"
        );
        assert!(
            AirModel::new(EARTH_R, None, 0.0).is_none(),
            "airless: no air"
        );
        let up = [0.0, 1.0, 0.0];
        let noon = up;
        // The day sky on Earth's ground is blue; thick air's is warmer.
        let blue = sample(Some(&thin), up, 2.0, 0.0, noon).overhead;
        assert!(blue[2] > blue[1] && blue[1] > blue[0], "blue sky: {blue:?}");
        let amber = sample(Some(&thick), up, 2.0, 0.0, noon).overhead;
        assert!(
            amber[0] / amber[2] > blue[0] / blue[2],
            "thick sky warmer: {amber:?}"
        );
        // From orbit the sky overhead is black; an airless sky always is.
        let orbit = sample(Some(&thin), up, 2_000_000.0, 0.0, noon).overhead;
        assert!(orbit.iter().all(|c| *c < 1e-12), "black above the air");
        let none = sample(None, up, 2.0, 0.0, noon);
        assert_eq!(none.overhead, [0.0; 3]);
        // The demo Earth has air from its recipe.
        let chain = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
        let demo = AirModel::for_planet(crate::nav::DEMO_SEED, chain.get(..9).unwrap_or(&[]));
        assert!(demo.is_some(), "the demo planet has air");
    }

    #[test]
    fn night_ground_is_a_dim_share_of_the_day() {
        // #908 Q1 A (replacing #780 Q2 A, 5-10%): a moonlit night. The
        // floor is NIGHT_SHARE (28%) of the noon sun; against the full day
        // ground light (sun plus sky) it reads about a fifth.
        let air = earth();
        let up = [0.0, 1.0, 0.0];
        let day = sample(Some(&air), up, -4_299.0, -4_300.0, up).ground;
        let night = sample(Some(&air), up, -4_299.0, -4_300.0, [0.0, -1.0, 0.0]).ground;
        let y = |c: [f64; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        let ratio = y(night) / y(day);
        assert!((0.15..=0.30).contains(&ratio), "night ratio {ratio}");
        // A low sun gives reddened light (sunset colours).
        let low = [(1.0f64 - 0.03 * 0.03).sqrt(), 0.03, 0.0];
        let dusk = air.ground_light(up, 0.0, low);
        let noon = air.ground_light(up, 0.0, up);
        assert!(
            dusk[0] / dusk[2] > noon[0] / noon[2],
            "low sun reddens: {dusk:?}"
        );
    }
}
