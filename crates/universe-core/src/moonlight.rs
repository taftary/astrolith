//! Night light from the moons (#981 correction round 2, Spec v4).
//!
//! The owner's rule: the moons are the main night lights. Each moon above
//! the horizon lights the ground from its direction like a very dim sun,
//! with a strength that follows its phase (full when the sun lights the
//! face toward the planet, nothing at new moon); several moons add up. The
//! stars add only a faint even glow, so a night with no moon up is dark.
//! By day the even fill follows the sun's height, so daylight looks as it
//! did with the old constant floor ([`DAY_FILL_SHARE`]).
//!
//! Shares are of the sunlight ([`crate::light::SUN_ILLUMINANCE`]), and
//! [`NightLight`] (Display > Scene, S2) scales the moon and star light.
//! Pure, headless and allocation-free (`E-CORE-NO-BEVY`, `E-DET-TIERS`,
//! `E-HOT-NOALLOC`).

/// The even fill by day, as a share of the sunlight: the old night floor
/// (#908 Q1 A), so the day keeps its look.
pub const DAY_FILL_SHARE: f64 = 0.28;

/// A full moon straight overhead at Night light 100%, as a share of the
/// sunlight. At the default 50% it gives about the old floor's light, but
/// from one side, so slopes keep their shape.
pub const MOON_FULL_SHARE: f64 = 0.5;

/// The stars' even glow at Night light 100%, as a share of the sunlight:
/// a moonless night is dark.
pub const STAR_SHARE: f64 = 0.04;

/// Most moon lights drawn at once.
pub const MAX_MOON_LIGHTS: usize = 4;

/// Night light setting at first launch, after Reset, and when the file has
/// no value, percent (S2: default 50%).
pub const NIGHT_LIGHT_DEFAULT: u8 = 50;

/// Night light change per step, percent.
pub const NIGHT_LIGHT_STEP: u8 = 10;

/// How bright the moon and star light is drawn, percent, a multiple of
/// [`NIGHT_LIGHT_STEP`] in `0..=100` (S2). 0 means no night light.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NightLight(u8);

impl NightLight {
    /// Night light holding `percent`, or `None` when out of range or
    /// off-step.
    #[must_use]
    pub fn new(percent: u8) -> Option<NightLight> {
        (percent <= 100 && percent.is_multiple_of(NIGHT_LIGHT_STEP)).then_some(NightLight(percent))
    }

    /// The percent.
    #[must_use]
    pub fn get(self) -> u8 {
        self.0
    }

    /// The scale, `0.0..=1.0`.
    #[must_use]
    pub fn scale(self) -> f64 {
        f64::from(self.0) / 100.0
    }

    /// One step brighter, clamped at 100%.
    #[must_use]
    pub fn brighter(self) -> NightLight {
        NightLight((self.0 + NIGHT_LIGHT_STEP).min(100))
    }

    /// One step darker, clamped at 0%.
    #[must_use]
    pub fn darker(self) -> NightLight {
        NightLight(self.0.saturating_sub(NIGHT_LIGHT_STEP))
    }
}

impl Default for NightLight {
    fn default() -> NightLight {
        NightLight(NIGHT_LIGHT_DEFAULT)
    }
}

/// One moon's light at the viewer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoonLight {
    /// Unit direction toward the moon (open-cell axes).
    pub direction: [f64; 3],
    /// Strength as a share of the sunlight, after phase, horizon and the
    /// Night light setting.
    pub share: f64,
}

/// The night light at one viewer.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SkyLight {
    /// The even fill (day fill plus star glow), a share of the sunlight.
    pub ambient_share: f64,
    /// The moons' lights, brightest first; `None` past the last.
    pub moons: [Option<MoonLight>; MAX_MOON_LIGHTS],
}

impl SkyLight {
    /// The moons' total share.
    #[must_use]
    pub fn moon_share(&self) -> f64 {
        self.moons.iter().flatten().map(|m| m.share).sum()
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let n = dot(v, v).sqrt();
    (n.is_finite() && n > 0.0).then_some([v[0] / n, v[1] / n, v[2] / n])
}

/// `0..=1` smoothstep of `x` between `a` and `b`.
fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How much of a body above the horizon lights flat ground: its height
/// `sin(altitude)`, faded in over the last few degrees so a light never
/// switches on with a jump (about -3 to +6 degrees).
fn horizon(height: f64) -> f64 {
    height.max(0.0) * smooth(-0.05, 0.1, height)
}

/// The lit share of a moon's face toward the planet: `(1 + cos a) / 2` with
/// `a` the sun-moon-planet angle; `sun` and `moon` are unit directions from
/// the planet. 1 when the moon is opposite the sun (full), 0 in front of it
/// (new).
#[must_use]
pub fn phase(sun: [f64; 3], moon: [f64; 3]) -> f64 {
    (1.0 - dot(sun, moon)).clamp(0.0, 2.0) / 2.0
}

/// The light at a viewer whose up is `up`, with the sun toward `sun` and
/// moons toward `moons` (unit directions from the planet, any order), at
/// Night light `night`. Pure.
#[must_use]
pub fn sky_light(
    sun: [f64; 3],
    up: [f64; 3],
    moons: impl IntoIterator<Item = [f64; 3]>,
    night: NightLight,
) -> SkyLight {
    let (Some(sun), Some(up)) = (unit(sun), unit(up)) else {
        return SkyLight::default();
    };
    let scale = night.scale();
    let day = smooth(-0.1, 0.1, dot(sun, up));
    let mut light = SkyLight {
        ambient_share: DAY_FILL_SHARE * day + STAR_SHARE * scale * (1.0 - day),
        moons: [None; MAX_MOON_LIGHTS],
    };
    for moon in moons {
        let Some(moon) = unit(moon) else {
            continue;
        };
        let share = MOON_FULL_SHARE * scale * phase(sun, moon) * horizon(dot(moon, up));
        if !(share.is_finite() && share > 0.0) {
            continue;
        }
        let candidate = MoonLight {
            direction: moon,
            share,
        };
        // Keep the brightest, brightest first.
        let slot = light
            .moons
            .iter()
            .position(|held| held.is_none_or(|held| held.share < share));
        if let Some(tail) = slot.and_then(|slot| light.moons.get_mut(slot..)) {
            tail.rotate_right(1);
            if let Some(first) = tail.first_mut() {
                *first = Some(candidate);
            }
        }
    }
    light
}

/// The light at the viewer at `camera` (open-cell position, the cell is
/// planet-centred at L10) from the universe's sun and its planet's moons,
/// at Night light `night`; `None` off the planet. Pure, no allocation.
#[must_use]
pub fn sky_light_at(
    universe: &crate::nav::Universe,
    camera: [f64; 3],
    night: NightLight,
) -> Option<SkyLight> {
    let sun = crate::light::sun_direction(universe)?;
    let moons = universe
        .open
        .points
        .iter()
        .filter(|point| point.moon.is_some())
        .map(|point| point.position);
    Some(sky_light(sun, camera, moons, night))
}

/// Light on flat ground under a viewer whose up is `up`, as a share of the
/// noon light, for the eyes (#908): the even fill, the sun's height and the
/// moons over noon's `1 + DAY_FILL_SHARE`. In `(0, 1]`: a black night still
/// reads as a tiny share, so the eyes open up; degenerate inputs read as
/// noon. Pure.
#[must_use]
pub fn view_share(sun: [f64; 3], up: [f64; 3], sky: &SkyLight) -> f64 {
    let (Some(sun), Some(up)) = (unit(sun), unit(up)) else {
        return 1.0;
    };
    let lit = sky.ambient_share + dot(sun, up).max(0.0) + sky.moon_share();
    let share = lit / (1.0 + DAY_FILL_SHARE);
    if share.is_finite() {
        share.clamp(1.0e-3, 1.0)
    } else {
        1.0
    }
}

#[cfg(test)]
#[path = "moonlight_tests.rs"]
mod tests;
