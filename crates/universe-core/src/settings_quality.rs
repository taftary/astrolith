//! Graphics quality preferences of the settings (#1022 T4, Spec v3 S1-S8;
//! `E-FILE-SIZE` split from `settings`). Re-exported from
//! [`crate::settings`].
//!
//! These are the player's saved choices. In Auto the game shows and uses
//! effective values of its own (`quality`), never written back here; in
//! Manual the saved choices are what the game draws (Q6, Q9, Q27).

/// The step after `x` in `all`, wrapping around (pure).
fn next_of<T: Copy + PartialEq, const N: usize>(all: [T; N], x: T) -> T {
    let i = all.iter().position(|v| *v == x).unwrap_or(0);
    all.get((i + 1) % N).copied().unwrap_or(x)
}

/// Auto or Manual graphics (S1, Q1 of the clarifications).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum QualityMode {
    /// The game adapts the graphics toward the target frame rate.
    #[default]
    Auto,
    /// The saved graphics choices stay as they are.
    Manual,
}

impl QualityMode {
    /// Every choice in cycle order.
    pub const ALL: [QualityMode; 2] = [QualityMode::Auto, QualityMode::Manual];

    /// Next choice when the setting is clicked.
    #[must_use]
    pub fn next(self) -> QualityMode {
        next_of(Self::ALL, self)
    }

    /// File id (`quality_mode = <id>`).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            QualityMode::Auto => "auto",
            QualityMode::Manual => "manual",
        }
    }

    /// Display name shown in Settings.
    #[must_use]
    pub const fn display(self) -> &'static str {
        match self {
            QualityMode::Auto => "Auto",
            QualityMode::Manual => "Manual",
        }
    }

    /// The choice from a file id.
    #[must_use]
    pub fn from_id(id: &str) -> Option<QualityMode> {
        Self::ALL.into_iter().find(|m| m.id() == id)
    }
}

/// Target frame rate, in both modes (S2, Q5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TargetFps {
    /// 30 frames a second: less power and heat.
    Fps30,
    /// 60 frames a second (the desktop default).
    #[default]
    Fps60,
}

impl TargetFps {
    /// Every choice in cycle order.
    pub const ALL: [TargetFps; 2] = [TargetFps::Fps30, TargetFps::Fps60];

    /// Next choice when the setting is clicked.
    #[must_use]
    pub fn next(self) -> TargetFps {
        next_of(Self::ALL, self)
    }

    /// Frames a second.
    #[must_use]
    pub const fn fps(self) -> u32 {
        match self {
            TargetFps::Fps30 => 30,
            TargetFps::Fps60 => 60,
        }
    }

    /// The target frame interval in milliseconds.
    #[must_use]
    pub fn interval_ms(self) -> f64 {
        1000.0 / f64::from(self.fps())
    }

    /// File id (`target_fps = <id>`).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            TargetFps::Fps30 => "30",
            TargetFps::Fps60 => "60",
        }
    }

    /// Display text shown in Settings.
    #[must_use]
    pub const fn display(self) -> &'static str {
        match self {
            TargetFps::Fps30 => "30 FPS",
            TargetFps::Fps60 => "60 FPS",
        }
    }

    /// The choice from a file id.
    #[must_use]
    pub fn from_id(id: &str) -> Option<TargetFps> {
        Self::ALL.into_iter().find(|t| t.id() == id)
    }
}

/// A Low / Normal / High tier: the Auto ceiling (S3) and the atmosphere
/// quality (S7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub enum Tier {
    /// Low.
    Low,
    /// Normal (the default).
    #[default]
    Normal,
    /// High.
    High,
}

impl Tier {
    /// Every tier, lowest first.
    pub const ALL: [Tier; 3] = [Tier::Low, Tier::Normal, Tier::High];

    /// Next tier when the setting is clicked.
    #[must_use]
    pub fn next(self) -> Tier {
        next_of(Self::ALL, self)
    }

    /// File id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Tier::Low => "low",
            Tier::Normal => "normal",
            Tier::High => "high",
        }
    }

    /// Display name shown in Settings.
    #[must_use]
    pub const fn display(self) -> &'static str {
        match self {
            Tier::Low => "Low",
            Tier::Normal => "Normal",
            Tier::High => "High",
        }
    }

    /// The tier from a file id.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Tier> {
        Self::ALL.into_iter().find(|t| t.id() == id)
    }
}

/// Manual 3D resolution: a height in physical pixels on the window's
/// shorter side, or Native (S4, Q25).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Resolution3d {
    /// 720 pixels on the shorter side.
    Short720,
    /// 900 pixels on the shorter side.
    Short900,
    /// 1080 pixels on the shorter side.
    Short1080,
    /// The window's own resolution (the default).
    #[default]
    Native,
}

impl Resolution3d {
    /// Every choice, smallest first.
    pub const ALL: [Resolution3d; 4] = [
        Resolution3d::Short720,
        Resolution3d::Short900,
        Resolution3d::Short1080,
        Resolution3d::Native,
    ];

    /// The shorter side in physical pixels, `None` for Native.
    #[must_use]
    pub const fn short_side(self) -> Option<u32> {
        match self {
            Resolution3d::Short720 => Some(720),
            Resolution3d::Short900 => Some(900),
            Resolution3d::Short1080 => Some(1080),
            Resolution3d::Native => None,
        }
    }

    /// The choices a window whose shorter side is `native_short` offers:
    /// none above native and none equal to it (Native covers that), so a
    /// 1600 x 900 window offers 720 and Native, and one under 720 Native
    /// only (Q25).
    #[must_use]
    pub fn offered(native_short: u32) -> Vec<Resolution3d> {
        Self::ALL
            .into_iter()
            .filter(|r| r.short_side().is_none_or(|s| s < native_short))
            .collect()
    }

    /// Next choice the window offers when the setting is clicked.
    #[must_use]
    pub fn next_offered(self, native_short: u32) -> Resolution3d {
        let offered = Self::offered(native_short);
        let i = offered.iter().position(|r| *r == self);
        i.and_then(|i| offered.get((i + 1) % offered.len()).copied())
            .unwrap_or(Resolution3d::Native)
    }

    /// File id (`resolution_3d = <id>`).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Resolution3d::Short720 => "720",
            Resolution3d::Short900 => "900",
            Resolution3d::Short1080 => "1080",
            Resolution3d::Native => "native",
        }
    }

    /// The choice from a file id.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Resolution3d> {
        Self::ALL.into_iter().find(|r| r.id() == id)
    }
}

/// Planet detail presets for the LOD screen error (#776, S1).
///
/// Low trades sharpness for GPU work on weaker PCs; Normal is the spec
/// default (1.5 px at 1600x900); High is sharper with more patches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlanetDetail {
    /// Coarse: splits above 2 px, merges below 1 px.
    Low,
    /// Spec default: splits above 1.5 px, merges below 0.75 px.
    #[default]
    Normal,
    /// Fine: splits above 1 px, merges below 0.5 px.
    High,
}

impl PlanetDetail {
    /// Every preset in cycle order.
    pub const ALL: [PlanetDetail; 3] =
        [PlanetDetail::Low, PlanetDetail::Normal, PlanetDetail::High];

    /// Next preset when the setting is clicked.
    #[must_use]
    pub fn next(self) -> PlanetDetail {
        match self {
            PlanetDetail::Low => PlanetDetail::Normal,
            PlanetDetail::Normal => PlanetDetail::High,
            PlanetDetail::High => PlanetDetail::Low,
        }
    }

    /// Split threshold in pixels at 1600x900 (Q2 A, S1).
    #[must_use]
    pub fn detail_px(self) -> f64 {
        match self {
            PlanetDetail::Low => 2.0,
            PlanetDetail::Normal => 1.5,
            PlanetDetail::High => 1.0,
        }
    }

    /// Pixels one patch quad covers at the end of its level's range (#817
    /// F1 A): fewer pixels per quad means finer detail.
    #[must_use]
    pub fn quad_px(self) -> f64 {
        match self {
            PlanetDetail::Low => 16.0,
            PlanetDetail::Normal => 8.0,
            PlanetDetail::High => 4.0,
        }
    }

    /// File id (`planet_detail = <id>`).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            PlanetDetail::Low => "low",
            PlanetDetail::Normal => "normal",
            PlanetDetail::High => "high",
        }
    }

    /// Display name shown in Settings.
    #[must_use]
    pub fn display(self) -> &'static str {
        match self {
            PlanetDetail::Low => "Low",
            PlanetDetail::Normal => "Normal",
            PlanetDetail::High => "High",
        }
    }

    /// Preset from a file id, or `None` for anything else.
    #[must_use]
    pub fn from_id(id: &str) -> Option<PlanetDetail> {
        match id {
            "low" => Some(PlanetDetail::Low),
            "normal" => Some(PlanetDetail::Normal),
            "high" => Some(PlanetDetail::High),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_choices_read_and_cycle() {
        assert_eq!(QualityMode::default(), QualityMode::Auto);
        assert_eq!(QualityMode::Auto.next(), QualityMode::Manual);
        assert_eq!(QualityMode::Manual.next(), QualityMode::Auto);
        assert_eq!(TargetFps::default().fps(), 60);
        assert_eq!(TargetFps::Fps60.next(), TargetFps::Fps30);
        assert!((TargetFps::Fps30.interval_ms() - 33.333).abs() < 1e-3);
        assert_eq!(Tier::default(), Tier::Normal);
        assert_eq!(Tier::High.next(), Tier::Low);
        assert!(Tier::Low < Tier::Normal && Tier::Normal < Tier::High);
        for mode in QualityMode::ALL {
            assert_eq!(QualityMode::from_id(mode.id()), Some(mode));
        }
        for fps in TargetFps::ALL {
            assert_eq!(TargetFps::from_id(fps.id()), Some(fps));
        }
        for tier in Tier::ALL {
            assert_eq!(Tier::from_id(tier.id()), Some(tier));
        }
        for res in Resolution3d::ALL {
            assert_eq!(Resolution3d::from_id(res.id()), Some(res));
        }
        assert_eq!(Tier::from_id("ultra"), None);
    }

    #[test]
    fn manual_resolution_offers_no_choice_above_native() {
        // Q25: 1600 x 900 offers 720 and Native; 1920 x 1080 adds 900;
        // a window under the floor offers Native only.
        assert_eq!(
            Resolution3d::offered(900),
            [Resolution3d::Short720, Resolution3d::Native]
        );
        assert_eq!(
            Resolution3d::offered(1080),
            [
                Resolution3d::Short720,
                Resolution3d::Short900,
                Resolution3d::Native
            ]
        );
        assert_eq!(Resolution3d::offered(600), [Resolution3d::Native]);
        assert_eq!(
            Resolution3d::Short720.next_offered(900),
            Resolution3d::Native
        );
        assert_eq!(
            Resolution3d::Native.next_offered(900),
            Resolution3d::Short720
        );
        // A saved choice the window does not offer clicks on to Native.
        assert_eq!(
            Resolution3d::Short1080.next_offered(900),
            Resolution3d::Native
        );
    }
}
