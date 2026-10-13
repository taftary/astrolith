//! Planet levels L11-L14 as shells around the planet (#632, Spec v2).
//!
//! From the planet down, every level is the whole planet: a shell of
//! radius `R + a_l` around its centre. The height above the ground alone
//! picks the level, anywhere on the planet and in every mode (Q1, Q6c).
//! On Earth the enter heights are 1,000 km (L11), 100 km (L12), 1 km (L13)
//! and 10 m (L14); L11-L13 scale with the planet's radius and L14 is always
//! 10 m, so the player has the same scale everywhere (Q2, Q3). Going up, a
//! level is left only above [`LEAVE_FACTOR`](crate::shell::LEAVE_FACTOR)
//! times its height, so hovering at a boundary never flickers (Q6a).
//! Climbing out of L11 returns to L10 (Q6d).
//!
//! Pure and allocation-free (`E-DET-TIERS`, `E-HOT-NOALLOC`):
//! [`ShellLevel`](crate::shell::ShellLevel) is a `Copy` value updated from
//! a height in metres.

use crate::coords::Level;

/// Earth enter heights above the ground, in metres, for L11, L12 and L13.
pub const EARTH_SHELL_M: [f64; 3] = [1.0e6, 1.0e5, 1.0e3];

/// L14 enter height above the ground on every planet, in metres (Q3).
pub const ROOM_SHELL_M: f64 = 10.0;

/// A level is left above this multiple of its enter height (Q6a).
pub const LEAVE_FACTOR: f64 = 1.5;

/// The planet level, above every shell.
const PLANET_LEVEL: u8 = 10;

/// The deepest shell level.
const ROOM_LEVEL: u8 = 14;

/// Enter height of shell `level` (L11-L14) above the ground, in metres,
/// on a planet of `radius_earth` Earth radii.
///
/// L11-L13 scale with the radius; L14 is always [`ROOM_SHELL_M`]. A
/// scaled height never drops below the next shell's (tiny planets keep
/// the order). `None` outside L11-L14 or for a non-finite or
/// non-positive radius.
#[must_use]
pub fn enter_height_m(level: Level, radius_earth: f64) -> Option<f64> {
    if !radius_earth.is_finite() || radius_earth <= 0.0 {
        return None;
    }
    let n = level.get();
    if n == ROOM_LEVEL {
        return Some(ROOM_SHELL_M);
    }
    let index = usize::from(n.checked_sub(PLANET_LEVEL + 1)?);
    let earth = *EARTH_SHELL_M.get(index)?;
    let below = Level::new(n + 1).and_then(|next| enter_height_m(next, radius_earth))?;
    Some((earth * radius_earth).max(below * LEAVE_FACTOR))
}

/// Leave height of shell `level` above the ground, in metres:
/// [`LEAVE_FACTOR`] times its enter height.
#[must_use]
pub fn leave_height_m(level: Level, radius_earth: f64) -> Option<f64> {
    enter_height_m(level, radius_earth).map(|height| height * LEAVE_FACTOR)
}

/// Shown order of magnitude of a shell level, for the readout: the
/// rounded log of its Earth enter height (6, 5, 3, 1). `None` outside
/// L11-L14.
#[must_use]
pub fn shell_exponent(level: Level) -> Option<f64> {
    enter_height_m(level, 1.0).map(f64::log10)
}

/// Metres per open-planet (L10) cell unit: the one metre the body, the
/// squares and the planet LOD already use ([`crate::body::metre`]), so a
/// 10 m shell and a 1.8 m body read on the same scale.
#[must_use]
pub fn metres_per_unit() -> f64 {
    1.0 / crate::body::metre(Level::new(PLANET_LEVEL).unwrap_or(Level::MAX))
}

/// Height above the ground, in metres, of `camera` in open-planet (L10)
/// units.
///
/// The ground is the flat-faced surface under the camera (#593); a
/// camera under it reads a negative height, the centre reads minus the
/// surface radius.
#[must_use]
pub fn height_above_ground_m(camera: [f64; 3]) -> f64 {
    let [x, y, z] = camera;
    let reach = (x * x + y * y + z * z).sqrt();
    let ground = crate::terrain::surface_radius(camera);
    (reach - ground) * metres_per_unit()
}

/// Brightness share of the planet's scenery (equator ring, spin axis) at
/// `height_m` above the ground of a planet of `radius_earth` Earth radii
/// (#632): full above the regions shell's leave height, none below its
/// enter height, a smooth step between, so the shells show only the
/// planet and nothing pops at a level change.
#[must_use]
pub fn scenery_fade(height_m: f64, radius_earth: f64) -> f64 {
    let regions = Level::new(PLANET_LEVEL + 1).unwrap_or(Level::MAX);
    let (Some(low), Some(high)) = (
        enter_height_m(regions, radius_earth),
        leave_height_m(regions, radius_earth),
    ) else {
        return 1.0;
    };
    if !height_m.is_finite() {
        return 1.0;
    }
    let t = ((height_m - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The level a camera on a planet is in: L10, or a shell L11-L14 (#632).
///
/// Updated from the height above the ground; the margin means the same
/// height can read two levels depending on the way you came.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShellLevel(Level);

impl Default for ShellLevel {
    fn default() -> ShellLevel {
        ShellLevel::planet()
    }
}

impl ShellLevel {
    /// The planet level, above every shell.
    #[must_use]
    pub fn planet() -> ShellLevel {
        ShellLevel(Level::new(PLANET_LEVEL).unwrap_or(Level::MAX))
    }

    /// The level now (L10-L14).
    #[must_use]
    pub const fn level(self) -> Level {
        self.0
    }

    /// The level for `height_m` above the ground of a planet of
    /// `radius_earth` Earth radii, coming from this level.
    ///
    /// Going down enters every shell whose enter height is above
    /// `height_m`; going up leaves every shell whose leave height is
    /// below it. Between the two the level holds. A non-finite height or
    /// radius keeps the level.
    #[must_use]
    pub fn update(self, height_m: f64, radius_earth: f64) -> ShellLevel {
        if !height_m.is_finite() || !radius_earth.is_finite() || radius_earth <= 0.0 {
            return self;
        }
        let mut n = self.0.get().clamp(PLANET_LEVEL, ROOM_LEVEL);
        while n < ROOM_LEVEL {
            match Level::new(n + 1).and_then(|next| enter_height_m(next, radius_earth)) {
                Some(enter) if height_m < enter => n += 1,
                _ => break,
            }
        }
        while n > PLANET_LEVEL {
            match Level::new(n).and_then(|here| leave_height_m(here, radius_earth)) {
                Some(leave) if height_m > leave => n -= 1,
                _ => break,
            }
        }
        ShellLevel(Level::new(n).unwrap_or(self.0))
    }

    /// The level for `height_m` reached straight from the planet level
    /// (no history): the shell whose enter height is the lowest above it.
    #[must_use]
    pub fn from_height(height_m: f64, radius_earth: f64) -> ShellLevel {
        ShellLevel::planet().update(height_m, radius_earth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(n: u8) -> Level {
        Level::new(n).expect("ladder level")
    }

    fn shown(height_m: f64) -> u8 {
        ShellLevel::from_height(height_m, 1.0).level().get()
    }

    #[test]
    fn the_level_follows_the_height_above_the_ground() {
        assert_eq!(shown(2.0e6), 10);
        assert_eq!(shown(999_000.0), 11);
        assert_eq!(shown(99_000.0), 12);
        assert_eq!(shown(999.0), 13);
        assert_eq!(shown(9.0), 14);
        assert_eq!(shown(0.0), 14);
        assert_eq!(shown(1.0e6 + 1.0), 10);
    }

    #[test]
    fn a_level_is_left_above_one_and_a_half_times_its_height() {
        let room = ShellLevel::from_height(5.0, 1.0);
        assert_eq!(room.level().get(), 14);
        assert_eq!(room.update(14.0, 1.0).level().get(), 14, "held under 15 m");
        assert_eq!(room.update(16.0, 1.0).level().get(), 13, "left above 15 m");
        let buildings = ShellLevel::from_height(500.0, 1.0);
        assert_eq!(buildings.update(1_400.0, 1.0).level().get(), 13);
        assert_eq!(buildings.update(1_600.0, 1.0).level().get(), 12);
        let regions = ShellLevel::from_height(900_000.0, 1.0);
        assert_eq!(regions.update(1_400_000.0, 1.0).level().get(), 11);
        assert_eq!(regions.update(1_600_000.0, 1.0).level().get(), 10);
        // Climbing from the room straight to orbit unwinds every shell.
        assert_eq!(room.update(1.0e7, 1.0).level().get(), 10);
    }

    #[test]
    fn the_heights_scale_with_the_planet_and_l14_stays_ten_metres() {
        for (n, earth) in [(11, 1.0e6), (12, 1.0e5), (13, 1.0e3)] {
            let half = enter_height_m(level(n), 0.5).expect("shell");
            assert!((half - earth * 0.5).abs() < 1e-6, "L{n} on a half-Earth");
            let double = enter_height_m(level(n), 2.0).expect("shell");
            assert!(
                (double - earth * 2.0).abs() < 1e-6,
                "L{n} on a double Earth"
            );
        }
        for radius in [0.1, 0.5, 1.0, 3.0, 11.0] {
            assert_eq!(enter_height_m(level(14), radius), Some(ROOM_SHELL_M));
            assert_eq!(ShellLevel::from_height(9.0, radius).level().get(), 14);
        }
        assert_eq!(ShellLevel::from_height(400.0, 0.5).level().get(), 13);
        assert_eq!(ShellLevel::from_height(600.0, 0.5).level().get(), 12);
    }

    #[test]
    fn shells_keep_their_order_on_tiny_planets() {
        let tiny = 1.0e-4;
        let mut previous = f64::INFINITY;
        for n in 11..=14 {
            let enter = enter_height_m(level(n), tiny).expect("shell");
            assert!(enter < previous, "L{n} below L{}", n - 1);
            previous = enter;
        }
    }

    #[test]
    fn only_shell_levels_have_heights() {
        assert_eq!(enter_height_m(level(10), 1.0), None);
        assert_eq!(enter_height_m(level(1), 1.0), None);
        assert_eq!(enter_height_m(level(11), 0.0), None);
        assert_eq!(enter_height_m(level(11), f64::NAN), None);
        let exponents: Vec<f64> = (11..=14)
            .filter_map(|n| shell_exponent(level(n)))
            .map(f64::round)
            .collect();
        assert_eq!(exponents, vec![6.0, 5.0, 3.0, 1.0]);
    }

    #[test]
    fn scenery_fades_out_across_the_regions_band() {
        assert_eq!(scenery_fade(2.0e6, 1.0), 1.0);
        assert_eq!(scenery_fade(1.0e6, 1.0), 0.0);
        assert_eq!(scenery_fade(10.0, 1.0), 0.0);
        let mid = scenery_fade(1.25e6, 1.0);
        assert!((mid - 0.5).abs() < 1e-12, "halfway: {mid}");
        let mut previous = 0.0;
        for step in 0..=100 {
            let height = 1.0e6 + 5.0e3 * f64::from(step);
            let fade = scenery_fade(height, 1.0);
            assert!(
                fade >= previous && fade - previous < 0.02,
                "smooth at {height}"
            );
            previous = fade;
        }
    }

    #[test]
    fn bad_input_keeps_the_level() {
        let room = ShellLevel::from_height(5.0, 1.0);
        assert_eq!(room.update(f64::NAN, 1.0), room);
        assert_eq!(room.update(5.0, f64::INFINITY), room);
        assert_eq!(ShellLevel::default(), ShellLevel::planet());
    }
}
