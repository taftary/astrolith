//! Camera presets of the settings (`E-FILE-SIZE` split from `settings`,
//! #899): normal-mode look smoothing (#756 S1) and the behind-the-player
//! follow (#899 S1). Re-exported from [`crate::settings`].

/// How the mouse look smooths in normal mode (#756, S1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CameraSmoothing {
    /// The raw mouse, as today.
    Off,
    /// Gentle smoothing (default): about 30 ms.
    #[default]
    Low,
    /// Stronger smoothing: about 60 ms.
    Medium,
    /// Strongest smoothing: about 120 ms.
    High,
}

impl CameraSmoothing {
    /// Every preset in cycle order.
    pub const ALL: [CameraSmoothing; 4] = [
        CameraSmoothing::Off,
        CameraSmoothing::Low,
        CameraSmoothing::Medium,
        CameraSmoothing::High,
    ];

    /// Next preset when the setting steps up.
    #[must_use]
    pub fn higher(self) -> CameraSmoothing {
        match self {
            CameraSmoothing::Off => CameraSmoothing::Low,
            CameraSmoothing::Low => CameraSmoothing::Medium,
            CameraSmoothing::Medium => CameraSmoothing::High,
            CameraSmoothing::High => CameraSmoothing::High,
        }
    }

    /// Next preset when the setting steps down.
    #[must_use]
    pub fn lower(self) -> CameraSmoothing {
        match self {
            CameraSmoothing::Off => CameraSmoothing::Off,
            CameraSmoothing::Low => CameraSmoothing::Off,
            CameraSmoothing::Medium => CameraSmoothing::Low,
            CameraSmoothing::High => CameraSmoothing::Medium,
        }
    }

    /// File id (`camera_smoothing = <id>`).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            CameraSmoothing::Off => "off",
            CameraSmoothing::Low => "low",
            CameraSmoothing::Medium => "medium",
            CameraSmoothing::High => "high",
        }
    }

    /// Display name shown in Settings.
    #[must_use]
    pub fn display(self) -> &'static str {
        match self {
            CameraSmoothing::Off => "Off",
            CameraSmoothing::Low => "Low",
            CameraSmoothing::Medium => "Medium",
            CameraSmoothing::High => "High",
        }
    }

    /// Preset from a file id, or `None` for anything else.
    #[must_use]
    pub fn from_id(id: &str) -> Option<CameraSmoothing> {
        match id {
            "off" => Some(CameraSmoothing::Off),
            "low" => Some(CameraSmoothing::Low),
            "medium" => Some(CameraSmoothing::Medium),
            "high" => Some(CameraSmoothing::High),
            _ => None,
        }
    }

    /// Smoothing time constant, seconds: 0 for Off (the raw mouse).
    #[must_use]
    pub fn tau(self) -> f64 {
        match self {
            CameraSmoothing::Off => crate::flight::LOOK_TAU_OFF,
            CameraSmoothing::Low => crate::flight::LOOK_TAU_LOW,
            CameraSmoothing::Medium => crate::flight::LOOK_TAU_MEDIUM,
            CameraSmoothing::High => crate::flight::LOOK_TAU_HIGH,
        }
    }
}

/// How the behind-the-player camera follows turns (#899, S1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BehindFollow {
    /// No follow: the camera stays where the mouse left it, as today.
    Off,
    /// Quick follow: about a third of a second to settle behind.
    Quick,
    /// Smooth follow (default): about a second to glide behind.
    #[default]
    Smooth,
}

impl BehindFollow {
    /// Every preset in cycle order.
    pub const ALL: [BehindFollow; 3] =
        [BehindFollow::Off, BehindFollow::Quick, BehindFollow::Smooth];

    /// Next preset when the setting steps up.
    #[must_use]
    pub fn higher(self) -> BehindFollow {
        match self {
            BehindFollow::Off => BehindFollow::Quick,
            BehindFollow::Quick => BehindFollow::Smooth,
            BehindFollow::Smooth => BehindFollow::Smooth,
        }
    }

    /// Next preset when the setting steps down.
    #[must_use]
    pub fn lower(self) -> BehindFollow {
        match self {
            BehindFollow::Off => BehindFollow::Off,
            BehindFollow::Quick => BehindFollow::Off,
            BehindFollow::Smooth => BehindFollow::Quick,
        }
    }

    /// File id (`behind_follow = <id>`).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            BehindFollow::Off => "off",
            BehindFollow::Quick => "quick",
            BehindFollow::Smooth => "smooth",
        }
    }

    /// Display name shown in Settings.
    #[must_use]
    pub fn display(self) -> &'static str {
        match self {
            BehindFollow::Off => "Off",
            BehindFollow::Quick => "Quick",
            BehindFollow::Smooth => "Smooth",
        }
    }

    /// Preset from a file id, or `None` for anything else.
    #[must_use]
    pub fn from_id(id: &str) -> Option<BehindFollow> {
        match id {
            "off" => Some(BehindFollow::Off),
            "quick" => Some(BehindFollow::Quick),
            "smooth" => Some(BehindFollow::Smooth),
            _ => None,
        }
    }

    /// Follow rate, radians per second: 0 for Off (today's fixed camera).
    #[must_use]
    pub fn rate(self) -> f64 {
        match self {
            BehindFollow::Off => 0.0,
            BehindFollow::Quick => crate::motion::BEHIND_FOLLOW_QUICK_RATE,
            BehindFollow::Smooth => crate::motion::BEHIND_FOLLOW_SMOOTH_RATE,
        }
    }
}
