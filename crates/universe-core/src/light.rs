//! The one light model (#755, #780 Q1 A).
//!
//! [`star_direction`](crate::light::star_direction) is the unit vector
//! toward the nearest star in open-cell units, and
//! [`sun_direction`](crate::light::sun_direction) the planet's sun, moving
//! with the sky's clock (#981).
//! [`SUN_ILLUMINANCE`](crate::light::SUN_ILLUMINANCE) and
//! [`NIGHT_SHARE`](crate::light::NIGHT_SHARE) set the sunlight and the night
//! floor that light the ground, the moons and the figure in the window.
//! Pure, headless and allocation-free (`E-CORE-NO-BEVY`, `E-DET-TIERS`,
//! `E-HOT-NOALLOC`).

/// Night floor: the light on the night side as a share of the zenith
/// daylight on flat ground. A moonlit night (#908 Q1 A, 25-30%): clearly
/// darker than day, but the ground and shapes stay visible. Was 7.5%
/// (#780 Q2 A), which read near black.
pub const NIGHT_SHARE: f64 = 0.28;

/// Share of the stops between the view's light and noon that the eyes make
/// up (#908 Q1 C): half, so the adapted night stays clearly darker than day.
pub const ADAPT_SHARE: f64 = 0.5;

/// Eye adaptation time constant, seconds (#908 Q2 B): about 95% of the way
/// in 5 seconds, smooth, never a jump.
pub const ADAPT_TAU_S: f64 = 5.0 / 3.0;

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
/// It moves with the sky's clock (#981 T3; was fixed, #780 Q5 A): the
/// planet goes around its star and spins, so it turns about the pole once
/// a day, but never depends on the camera or the body.
#[must_use]
pub fn sun_direction(universe: &crate::nav::Universe) -> Option<[f64; 3]> {
    if universe.level().get() != 10 {
        return None;
    }
    let dir = star_direction(universe);
    (dir != FALLBACK_STAR_DIRECTION).then_some(dir)
}

/// The unit direction `angle` radians from unit `sun`, turned toward a
/// fixed side (world `+Y`, or `+X` when the sun is near `+Y`): the place on
/// the planet where the sun stands `angle` from straight overhead (0 noon,
/// about 90 degrees the terminator, beyond it night). Pure.
#[must_use]
pub fn from_sun(sun: [f64; 3], angle: f64) -> [f64; 3] {
    let helper = if sun[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let along = dot(helper, sun);
    let side = unit([
        helper[0] - sun[0] * along,
        helper[1] - sun[1] * along,
        helper[2] - sun[2] * along,
    ])
    .unwrap_or(FALLBACK_STAR_DIRECTION);
    let (s, c) = angle.sin_cos();
    unit([
        sun[0] * c + side[0] * s,
        sun[1] * c + side[1] * s,
        sun[2] * c + side[2] * s,
    ])
    .unwrap_or(sun)
}

/// Light on flat ground under a camera whose up is `up`, as a share of the
/// noon light (#908): the night floor plus the sun's height above the
/// horizon, `max(0, sun . up)`, over noon's `1 + NIGHT_SHARE`. In `(0, 1]`;
/// degenerate inputs read as noon (no adaptation). Pure.
#[must_use]
pub fn view_light_share(sun: [f64; 3], up: [f64; 3]) -> f64 {
    let (Some(sun), Some(up)) = (unit(sun), unit(up)) else {
        return 1.0;
    };
    let height = dot(sun, up).max(0.0);
    let share = (NIGHT_SHARE + height) / (1.0 + NIGHT_SHARE);
    if share.is_finite() && share > 0.0 {
        share.min(1.0)
    } else {
        1.0
    }
}

/// Exposure offset in EV (added to the camera's `ev100`) the eyes settle
/// at for a view `share` of the noon light (#908 Q1 C): negative brightens,
/// [`ADAPT_SHARE`] of the stops between the view and noon. Zero at noon.
/// Pure.
#[must_use]
pub fn adapted_ev_offset(share: f64) -> f64 {
    if !share.is_finite() || share <= 0.0 {
        return 0.0;
    }
    ADAPT_SHARE * share.min(1.0).log2()
}

/// Eases the exposure offset `current` toward `target` over `dt` seconds
/// with [`ADAPT_TAU_S`] (#908 Q2 B): exponential, so it never jumps or
/// overshoots. Degenerate inputs hold `current`. Pure.
#[must_use]
pub fn ease_ev(current: f64, target: f64, dt: f64) -> f64 {
    if !current.is_finite() {
        return if target.is_finite() { target } else { 0.0 };
    }
    if !target.is_finite() || !dt.is_finite() || dt <= 0.0 {
        return current;
    }
    let share = 1.0 - (-dt / ADAPT_TAU_S).exp();
    current + (target - current) * share.clamp(0.0, 1.0)
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
    }

    #[test]
    fn night_is_moonlit() {
        // C1 (#908 Q1 A): the night floor is 25-30% of the noon light.
        assert!((0.25..=0.30).contains(&NIGHT_SHARE), "{NIGHT_SHARE}");
        // On the night side the view's light is that floor, over noon.
        let sun = [0.0, 1.0, 0.0];
        let night = view_light_share(sun, [0.0, -1.0, 0.0]);
        assert!((night - NIGHT_SHARE / (1.0 + NIGHT_SHARE)).abs() < 1e-12);
        assert_eq!(view_light_share(sun, [0.0, 1.0, 0.0]), 1.0, "noon");
    }

    #[test]
    fn eyes_adapt_at_the_planet() {
        // C2 (#908 Q1 C, Q2 B): brighter at night, darker by day, about 5 s
        // to get most of the way, smooth with no jump.
        let sun = [0.0, 1.0, 0.0];
        let night = adapted_ev_offset(view_light_share(sun, [0.0, -1.0, 0.0]));
        let noon = adapted_ev_offset(view_light_share(sun, [0.0, 1.0, 0.0]));
        assert!(night < -0.5, "the night brightens: {night} EV");
        assert_eq!(noon, 0.0, "noon is today's exposure");
        // The adapted night stays darker than day: half the stops.
        let stops = (NIGHT_SHARE / (1.0 + NIGHT_SHARE)).log2();
        assert!((night - ADAPT_SHARE * stops).abs() < 1e-12);
        // Day to night: after 5 s, at least 90% of the way; every 1/60 s
        // step is small and moves toward the target.
        let dt = 1.0 / 60.0;
        let mut ev = noon;
        let mut last = ev;
        for _ in 0..300 {
            ev = ease_ev(ev, night, dt);
            assert!(ev <= last && ev >= night, "smooth, no overshoot");
            assert!((last - ev).abs() < 0.05, "no jump: {} EV", last - ev);
            last = ev;
        }
        assert!(
            (ev - night).abs() <= 0.1 * night.abs(),
            "most of the way in 5 s: {ev}"
        );
        // And back to day.
        for _ in 0..300 {
            ev = ease_ev(ev, noon, dt);
        }
        assert!(ev.abs() <= 0.1 * night.abs(), "back to day in 5 s: {ev}");
        assert_eq!(ease_ev(0.3, f64::NAN, dt), 0.3);
        assert_eq!(adapted_ev_offset(f64::NAN), 0.0);
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
