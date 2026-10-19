//! The one light model (#755, #780 Q1 A).
//!
//! [`star_direction`](crate::light::star_direction) is the unit vector
//! toward the nearest star in open-cell units, and
//! [`sun_direction`](crate::light::sun_direction) the planet's fixed sun.
//! [`SUN_ILLUMINANCE`](crate::light::SUN_ILLUMINANCE) and
//! [`NIGHT_SHARE`](crate::light::NIGHT_SHARE) set the sunlight and the night
//! floor that light the ground, the moons and the figure in the window.
//! Pure, headless and allocation-free (`E-CORE-NO-BEVY`, `E-DET-TIERS`,
//! `E-HOT-NOALLOC`).

/// Night floor: the light on the night side as a share of the zenith
/// daylight on flat ground (#780 Q2 A), so the night side reads at about 7%
/// of the day side on every planet.
pub const NIGHT_SHARE: f64 = 0.075;

/// Floor off the planet (L1-L9), where no sun lights the ground: the share
/// of the sunlight the figure keeps on its dark side, so it still reads.
pub const OFF_PLANET_FLOOR: f64 = 0.35;

/// Sun strength for the window's light, in lux at the top of the air.
///
/// Not physical: a lit diffuse surface facing the sun shows
/// `base x E / pi x exposure`, and the game's fixed exposure (`ev100` 0,
/// factor `1 / 1.2`) then needs `E = 1.2 pi` to show its base colour, as
/// the unlit ground did before, so nothing jumps when the planet opens
/// (#780). The sky scales linearly with it.
pub const SUN_ILLUMINANCE: f64 = 1.2 * std::f64::consts::PI;

/// Fallback star direction when no star is near (at the star or above):
/// the camera's up-right, a fixed labelled default so the figure still reads.
pub const FALLBACK_STAR_DIRECTION: [f64; 3] = {
    let inv = 0.5773502691896258;
    [inv, inv, inv]
};

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(v: [f64; 3]) -> f64 {
    dot(v, v).sqrt()
}

fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let n = length(v);
    (n.is_finite() && n > 0.0).then_some([v[0] / n, v[1] / n, v[2] / n])
}

/// Unit vector toward the nearest star in open-cell units (#755).
///
/// On the planet (L10) it is the direction from the planet to its star: the
/// planet marker in the L9 parent cell with the star at its centre, rotated
/// into the open cell through the entry frame. At the star or above (L9 and
/// shallower, or any missing data) it is [`FALLBACK_STAR_DIRECTION`].
/// Pure, no allocation.
#[must_use]
pub fn star_direction(universe: &crate::nav::Universe) -> [f64; 3] {
    if universe.level().get() != 10 {
        return FALLBACK_STAR_DIRECTION;
    }
    let indices = universe.path.indices();
    if indices.len() != 9 {
        return FALLBACK_STAR_DIRECTION;
    }
    let Some(parent) = universe.parent.as_ref() else {
        return FALLBACK_STAR_DIRECTION;
    };
    let Some(&marker) = indices.get(8) else {
        return FALLBACK_STAR_DIRECTION;
    };
    let marker = marker as usize;
    let Some(point) = parent.points.get(marker) else {
        return FALLBACK_STAR_DIRECTION;
    };
    let at = point.portal_position();
    let to_star_parent = [-at[0], -at[1], -at[2]];
    let Some(unit_parent) = unit(to_star_parent) else {
        return FALLBACK_STAR_DIRECTION;
    };
    let Some(entered) = universe.path.entered() else {
        return FALLBACK_STAR_DIRECTION;
    };
    // Anonymous milestones are identity frames, so folding every chain entry
    // from the L9 step onward is exact; at L10 only the last entry turns.
    let mut dir = unit_parent;
    for opened in universe.path.chain().iter().skip(8) {
        if opened.anonymous {
            continue;
        }
        dir = opened.to_child_direction(dir);
    }
    let _ = entered;
    unit(dir).unwrap_or(FALLBACK_STAR_DIRECTION)
}

/// The planet's sun direction (unit, toward the star; open-cell and planet
/// axes agree), or `None` off the planet (#780).
///
/// Fixed (Q5 A): the planet does not spin, so it depends only on which
/// planet is open, never on time, the camera or the body.
#[must_use]
pub fn sun_direction(universe: &crate::nav::Universe) -> Option<[f64; 3]> {
    if universe.level().get() != 10 {
        return None;
    }
    let dir = star_direction(universe);
    (dir != FALLBACK_STAR_DIRECTION).then_some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::{DEMO_SEED, MarkerIndex};

    fn opened_l10() -> crate::nav::Universe {
        let chain = crate::nest::autopilot_path(DEMO_SEED);
        let mut universe = crate::nav::Universe::new(DEMO_SEED);
        for &marker in chain.iter().take(9) {
            assert!(universe.open(MarkerIndex(marker)), "journey opens");
        }
        assert_eq!(universe.level().get(), 10);
        universe
    }

    #[test]
    fn sun_direction_is_fixed() {
        // Q5 A: no spin; the same planet always gives the same sun, wherever
        // the camera is and however long it runs.
        let mut universe = opened_l10();
        let sun = sun_direction(&universe).expect("a sun at the planet");
        assert!((length(sun) - 1.0).abs() < 1e-12);
        universe.path.set_offset([0.2, -0.1, 0.4]);
        assert_eq!(sun_direction(&universe), Some(sun), "camera-independent");
        assert_eq!(sun_direction(&opened_l10()), Some(sun), "repeat runs agree");
        assert_eq!(
            sun_direction(&crate::nav::Universe::new(DEMO_SEED)),
            None,
            "no sun off the planet"
        );
        assert!((0.05..=0.10).contains(&NIGHT_SHARE));
    }

    #[test]
    fn the_planet_sees_its_own_star() {
        // On the planet the direction is real, unit, and deterministic.
        let universe = opened_l10();
        let dir = star_direction(&universe);
        assert!(
            (length(dir) - 1.0).abs() < 1e-12,
            "unit direction toward the star: {dir:?}"
        );
        assert!(dir.iter().all(|c| c.is_finite()));
        assert_ne!(
            dir, FALLBACK_STAR_DIRECTION,
            "the planet sees its own star, not the fallback"
        );
        let again = star_direction(&universe);
        assert_eq!(dir, again, "deterministic per universe");
        // At the star or above the fallback holds.
        let root = crate::nav::Universe::new(DEMO_SEED);
        assert_eq!(star_direction(&root), FALLBACK_STAR_DIRECTION);
        let mut l9 = crate::nav::Universe::new(DEMO_SEED);
        let chain = crate::nest::autopilot_path(DEMO_SEED);
        for &marker in chain.iter().take(8) {
            assert!(l9.open(MarkerIndex(marker)));
        }
        assert_eq!(l9.level().get(), 9);
        assert_eq!(star_direction(&l9), FALLBACK_STAR_DIRECTION);
    }
}
