//! Seas, sea ice and lakes on solid worlds (#1162).
//!
//! Which worlds hold water follows their #1161 type (Q1 A): Earth-like
//! worlds have liquid seas, ice worlds frozen ones, and desert, iron, lava
//! and airless worlds stay dry. The sea surface sits at 0 m, where the
//! relief's continent mask crosses from land to ocean floor; lakes fill
//! their basins to their own levels ([`crate::relief::lakes`], T3).
//!
//! The water is a floor (Q3 A): [`crate::relief::Relief::height_m`] is the
//! drawn ground, the higher of the terrain and the water there, so the
//! patches, the ground support, the player and the camera all stand on the
//! sea. [`crate::relief::Relief::terrain_m`] keeps the terrain itself, with
//! the ocean floor. On dry worlds the two are the same bits.
//!
//! Each patch vertex also carries a [`WaterCode`](crate::water::WaterCode), which the ground shader
//! reads for the water's colour: land, the pale dry ring of a partly filled
//! lake, or water with its depth. Pure and deterministic (`E-DET-TIERS`).

use crate::world_type::WorldType;

/// Sea level in metres above the mean radius.
pub const SEA_LEVEL_M: f64 = 0.0;

/// Deepest water the code tells apart, in metres: deeper reads as deep.
pub const CODE_DEPTH_M: f64 = 4_000.0;

/// What a world's water is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaterKind {
    /// Liquid seas and lakes, frozen within the polar caps (Earth-like).
    Liquid,
    /// Frozen seas and lakes everywhere (ice worlds).
    Frozen,
}

impl WaterKind {
    /// The water a world of `world` type holds (Q1 A): `None` when dry.
    #[must_use]
    pub fn for_type(world: WorldType) -> Option<WaterKind> {
        match world {
            WorldType::EarthLike => Some(WaterKind::Liquid),
            WorldType::Ice => Some(WaterKind::Frozen),
            WorldType::Desert | WorldType::Iron | WorldType::Lava | WorldType::Barren => None,
        }
    }
}

/// One world's water (Q1 A, Q4 A).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Water {
    /// Liquid or frozen.
    pub kind: WaterKind,
    /// Sine of the latitude from which the sea is frozen (Q4 A): the edge
    /// of the type's polar caps, as the ground shader draws them
    /// (`1 - 0.3 * cap_reach`). `0` freezes everything.
    pub ice_edge: f64,
}

impl Water {
    /// The water of a world of `world` type, `None` when dry.
    #[must_use]
    pub fn for_type(world: WorldType) -> Option<Water> {
        let kind = WaterKind::for_type(world)?;
        let ice_edge = match kind {
            WaterKind::Liquid => 1.0 - 0.3 * world.palette().cap_reach,
            WaterKind::Frozen => 0.0,
        };
        Some(Water { kind, ice_edge })
    }

    /// Whether water at unit direction `dir` is frozen: everywhere on ice
    /// worlds, and within the polar caps' reach on Earth-like ones. Fixed
    /// through time (Q4 A).
    #[must_use]
    pub fn frozen_at(&self, dir: [f64; 3]) -> bool {
        match self.kind {
            WaterKind::Frozen => true,
            WaterKind::Liquid => dir[1].abs() >= self.ice_edge,
        }
    }
}

/// What a vertex is for the ground shader: land, dry lake ring, or water
/// with its depth (one byte: `0` land, `1` dry ring, `2..=255` water whose
/// depth grows with the square of the code up to [`CODE_DEPTH_M`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WaterCode(pub u8);

impl WaterCode {
    /// Dry land.
    pub const LAND: WaterCode = WaterCode(0);
    /// The pale dry ring between a partly filled lake and its rim.
    pub const RING: WaterCode = WaterCode(1);

    /// Water `depth_m` deep (any positive depth gives at least code 2).
    #[must_use]
    pub fn water(depth_m: f64) -> WaterCode {
        let t = (depth_m.max(0.0) / CODE_DEPTH_M).min(1.0).sqrt();
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "E-CAST: t is in [0, 1], so the value is in [2, 255]"
        )]
        let code = (2.0 + t * 253.0).round() as u8;
        WaterCode(code.max(2))
    }

    /// Whether this is water.
    #[must_use]
    pub fn is_water(self) -> bool {
        self.0 >= 2
    }

    /// The depth in metres the code stands for (`0` on land and the ring).
    #[must_use]
    pub fn depth_m(self) -> f64 {
        if !self.is_water() {
            return 0.0;
        }
        let t = f64::from(self.0 - 2) / 253.0;
        t * t * CODE_DEPTH_M
    }
}

/// One drawn-ground sample: the surface stood on, the terrain under it,
/// and the shader's code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundSample {
    /// The drawn ground in metres: the higher of the terrain and the water.
    pub surface_m: f64,
    /// The terrain in metres (the ocean floor under the sea).
    pub terrain_m: f64,
    /// Land, dry ring or water depth.
    pub code: WaterCode,
}

impl GroundSample {
    /// Dry ground at `terrain_m`.
    #[must_use]
    pub fn dry(terrain_m: f64) -> GroundSample {
        GroundSample {
            surface_m: terrain_m,
            terrain_m,
            code: WaterCode::LAND,
        }
    }

    /// Ground at `terrain_m` under water standing at `level_m` (dry when
    /// the terrain is at or above it).
    #[must_use]
    pub fn under(terrain_m: f64, level_m: f64) -> GroundSample {
        if terrain_m >= level_m {
            return GroundSample::dry(terrain_m);
        }
        GroundSample {
            surface_m: level_m,
            terrain_m,
            code: WaterCode::water(level_m - terrain_m),
        }
    }
}

/// Period in wave seconds after which every wave pattern repeats: the
/// shader's wave periods all divide it, so wrapping the wave time here
/// never shows.
pub const WAVE_PERIOD_S: f64 = 960.0;

/// The waves' own time at game time `game_s` (Q6 A): game time slowed by
/// the sky's [`crate::clock::GAME_PER_REAL`], so at 1x the waves move at a
/// natural pace and speeding the sky speeds them, wrapped into
/// [`WAVE_PERIOD_S`].
#[must_use]
pub fn wave_time_s(game_s: f64) -> f64 {
    (game_s / crate::clock::GAME_PER_REAL).rem_euclid(WAVE_PERIOD_S)
}

/// What the ground shader needs to run the waves between updates: the wave
/// time `phase_s` at real time `real_s`, advancing at `rate` wave seconds
/// per real second (`0` when the sky is paused).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveClock {
    /// Wave time at the sync, seconds (`0..WAVE_PERIOD_S`).
    pub phase_s: f64,
    /// Real (shader) time at the sync, seconds.
    pub real_s: f64,
    /// Wave seconds per real second.
    pub rate: f64,
}

impl WaveClock {
    /// The wave clock for the sky `clock`, synced at shader time `real_s`.
    #[must_use]
    pub fn sync(clock: &crate::clock::Clock, real_s: f64) -> WaveClock {
        WaveClock {
            phase_s: wave_time_s(clock.game_s),
            real_s,
            rate: if clock.playing { clock.speed() } else { 0.0 },
        }
    }

    /// The wave time the shader works out at real time `real_s`.
    #[must_use]
    pub fn at(&self, real_s: f64) -> f64 {
        self.phase_s + (real_s - self.real_s) * self.rate
    }
}

#[cfg(test)]
#[path = "water_tests.rs"]
mod tests;
