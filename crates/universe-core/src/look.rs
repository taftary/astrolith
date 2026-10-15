//! The Planet look and the world it shows (#685, Q3).
//!
//! Settings > Display > Scene > Planet look has three choices. Normal and
//! Squares show today's universe unchanged (the Earth-Moon home, the
//! three-class planet sizes, a smooth surface); Generated shows the
//! generated universe, where every planet and moon, the home one included
//! (Q4), comes from the generator. [`WorldMode`](crate::look::WorldMode)
//! is what the core reads; [`PlanetLook`](crate::look::PlanetLook) is what
//! the player picks. Relief height (S1) scales the generated relief.

/// The Planet look the player picks (Display > Scene).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlanetLook {
    /// The smooth planet in its normal tones (as before #685).
    #[default]
    Normal,
    /// Black-and-white 100 m squares (#626).
    Squares,
    /// The generated universe (#685).
    Generated,
}

impl PlanetLook {
    /// Every look, in the order a click cycles through them.
    pub const ALL: [PlanetLook; 3] = [
        PlanetLook::Normal,
        PlanetLook::Squares,
        PlanetLook::Generated,
    ];

    /// The next look when the row is clicked.
    #[must_use]
    pub const fn next(self) -> PlanetLook {
        match self {
            PlanetLook::Normal => PlanetLook::Squares,
            PlanetLook::Squares => PlanetLook::Generated,
            PlanetLook::Generated => PlanetLook::Normal,
        }
    }

    /// Its word in the settings row and the settings file.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            PlanetLook::Normal => "normal",
            PlanetLook::Squares => "squares",
            PlanetLook::Generated => "generated",
        }
    }

    /// The look named `key` in the settings file, if any.
    #[must_use]
    pub fn parse(key: &str) -> Option<PlanetLook> {
        PlanetLook::ALL.into_iter().find(|look| look.key() == key)
    }

    /// Whether the squares shader is on.
    #[must_use]
    pub const fn squares(self) -> bool {
        matches!(self, PlanetLook::Squares)
    }

    /// The world this look shows.
    #[must_use]
    pub const fn world(self) -> WorldMode {
        match self {
            PlanetLook::Normal | PlanetLook::Squares => WorldMode::Classic,
            PlanetLook::Generated => WorldMode::Generated,
        }
    }
}

/// Which universe the core builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WorldMode {
    /// Today's universe, unchanged (Normal and Squares looks).
    #[default]
    Classic,
    /// The generated universe (#685).
    Generated,
}

/// Lowest relief height, in tenths (0.5×).
pub const RELIEF_MIN: u32 = 5;

/// Highest relief height, in tenths (2×).
pub const RELIEF_MAX: u32 = 20;

/// Default relief height, in tenths (1×).
pub const RELIEF_DEFAULT: u32 = 10;

/// Relief height (S1): how much the generated relief is scaled, in tenths
/// from [`RELIEF_MIN`] (0.5×) to [`RELIEF_MAX`] (2×).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ReliefHeight(u32);

impl ReliefHeight {
    /// The height holding `tenths`, or `None` outside the range.
    #[must_use]
    pub fn new(tenths: u32) -> Option<ReliefHeight> {
        (RELIEF_MIN..=RELIEF_MAX)
            .contains(&tenths)
            .then_some(ReliefHeight(tenths))
    }

    /// The step, in tenths.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// The scale factor (0.5 to 2.0).
    #[must_use]
    pub fn factor(self) -> f64 {
        f64::from(self.0) / 10.0
    }

    /// One step up or down (`dir` is −1 or +1), held in range.
    #[must_use]
    pub fn stepped(self, dir: i32) -> ReliefHeight {
        let next = self
            .0
            .saturating_add_signed(dir)
            .clamp(RELIEF_MIN, RELIEF_MAX);
        ReliefHeight(next)
    }
}

impl Default for ReliefHeight {
    fn default() -> ReliefHeight {
        ReliefHeight(RELIEF_DEFAULT)
    }
}

/// What the core needs from the settings to build the world.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WorldOptions {
    /// Classic or generated universe.
    pub mode: WorldMode,
    /// Relief height factor (0.5 to 2.0); `1.0` by default.
    pub relief: ReliefFactor,
}

impl WorldOptions {
    /// The classic world at default relief: what every frame reads until
    /// the settings pick Generated. `const` so frame constructors stay
    /// `const`.
    #[must_use]
    pub const fn classic() -> WorldOptions {
        WorldOptions {
            mode: WorldMode::Classic,
            relief: ReliefFactor(1.0),
        }
    }
}

/// A relief scale factor held in `0.5..=2.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReliefFactor(f64);

impl ReliefFactor {
    /// The factor of a [`ReliefHeight`].
    #[must_use]
    pub fn of(height: ReliefHeight) -> ReliefFactor {
        ReliefFactor(height.factor())
    }

    /// The factor.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl Default for ReliefFactor {
    fn default() -> ReliefFactor {
        ReliefFactor(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planet_look_offers_normal_squares_and_generated() {
        // C1 / AC1: three looks, cycling in order; Normal by default;
        // Normal and Squares keep the classic world.
        assert_eq!(PlanetLook::default(), PlanetLook::Normal);
        let keys: Vec<&str> = PlanetLook::ALL.iter().map(|look| look.key()).collect();
        assert_eq!(keys, ["normal", "squares", "generated"]);
        let mut look = PlanetLook::Normal;
        for want in [
            PlanetLook::Squares,
            PlanetLook::Generated,
            PlanetLook::Normal,
        ] {
            look = look.next();
            assert_eq!(look, want);
        }
        for look in PlanetLook::ALL {
            assert_eq!(PlanetLook::parse(look.key()), Some(look));
        }
        assert_eq!(PlanetLook::parse("bogus"), None);
        assert_eq!(PlanetLook::Normal.world(), WorldMode::Classic);
        assert_eq!(PlanetLook::Squares.world(), WorldMode::Classic);
        assert_eq!(PlanetLook::Generated.world(), WorldMode::Generated);
        assert!(PlanetLook::Squares.squares() && !PlanetLook::Generated.squares());
        // The settings file default and the settings UI read the same.
        let settings = crate::settings::Settings::default();
        assert_eq!(settings.planet_look, PlanetLook::Normal);
        let text = settings.format();
        assert!(text.contains("planet_look = normal"));
        for look in PlanetLook::ALL {
            let mut s = settings;
            s.planet_look = look;
            assert_eq!(
                crate::settings::Settings::parse(&s.format()).planet_look,
                look
            );
        }
    }

    #[test]
    fn relief_height_and_clouds_settings() {
        // C16 / AC16 (settings part): Relief height 0.5x to 2x, default 1x;
        // Clouds on by default; both saved, restored, and reset with the
        // Display category.
        use crate::settings::{Category, Settings};
        let s = Settings::default();
        assert_eq!(s.relief_height.factor(), 1.0);
        assert!(s.clouds);
        let text = s.format();
        assert!(text.contains("relief_height = 10"));
        assert!(text.contains("clouds = on"));
        let mut changed = s;
        changed.relief_height = ReliefHeight::new(20).expect("in range");
        changed.clouds = false;
        let back = Settings::parse(&changed.format());
        assert_eq!(back.relief_height.factor(), 2.0);
        assert!(!back.clouds);
        let reset = back.reset(Category::Display);
        assert_eq!(reset.relief_height, ReliefHeight::default());
        assert!(reset.clouds);
        // A value out of range in the file keeps the default.
        let bad = Settings::parse(
            "relief_height = 99
clouds = maybe
",
        );
        assert_eq!(bad.relief_height, ReliefHeight::default());
        assert!(bad.clouds);
    }

    #[test]
    fn the_universe_follows_the_world_options() {
        // Switching to the generated world reloads the open cells; the
        // same options again change nothing.
        let mut universe = crate::nav::Universe::new(crate::nav::DEMO_SEED);
        assert_eq!(universe.world.mode, WorldMode::Classic);
        let generated = WorldOptions {
            mode: WorldMode::Generated,
            relief: ReliefFactor::default(),
        };
        universe.set_world(generated);
        assert_eq!(universe.world, generated);
        let before = universe.clone();
        universe.set_world(generated);
        assert_eq!(universe, before);
    }

    #[test]
    fn relief_height_steps_from_half_to_double() {
        assert_eq!(ReliefHeight::default().factor(), 1.0);
        assert_eq!(ReliefHeight::new(4), None);
        assert_eq!(ReliefHeight::new(21), None);
        assert_eq!(ReliefHeight::new(5).map(ReliefHeight::factor), Some(0.5));
        assert_eq!(ReliefHeight::new(20).map(ReliefHeight::factor), Some(2.0));
        let mut h = ReliefHeight::default();
        for _ in 0..30 {
            h = h.stepped(1);
        }
        assert_eq!(h.get(), RELIEF_MAX);
        for _ in 0..30 {
            h = h.stepped(-1);
        }
        assert_eq!(h.get(), RELIEF_MIN);
        assert_eq!(ReliefFactor::of(ReliefHeight::default()).get(), 1.0);
    }
}
