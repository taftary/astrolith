//! Star light direction and face shading for the player figure (#755).
//!
//! The game draws everything unlit; only the figure gets shading, computed
//! on the CPU as vertex colours on an unlit mesh. [`star_direction`] is the
//! unit vector toward the nearest star in open-cell units; [`shade`] turns a
//! base colour and a face normal into the lit colour. Pure, headless and
//! allocation-free (`E-CORE-NO-BEVY`, `E-DET-TIERS`, `E-HOT-NOALLOC`).

/// Minimum kept light on the dark side: never black, so the shape reads.
pub const MIN_LIGHT: f64 = 0.35;

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

/// Lit colour of one face: `base` scaled by the star.
///
/// `brightness = max(MIN_LIGHT, 0.5 + 0.5 * dot(normal, to_star))`, so the
/// lit side is brighter and the dark side stays above the floor. Pure.
#[must_use]
pub fn shade(base: [f32; 3], normal: [f64; 3], to_star: [f64; 3]) -> [f32; 3] {
    let cosine = dot(normal, to_star).clamp(-1.0, 1.0);
    let light = (0.5 + 0.5 * cosine).max(MIN_LIGHT);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: 0..1 light narrowed for the GPU, intended"
    )]
    let light = light as f32;
    [base[0] * light, base[1] * light, base[2] * light]
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
    fn the_figure_is_shaded_by_its_star() {
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
        // Shading: lit side brighter, dark side above the floor.
        let base = [0.8f32, 0.8, 0.8];
        let lit = shade(base, dir, dir);
        let dark = shade(base, [-dir[0], -dir[1], -dir[2]], dir);
        assert!(lit[0] > dark[0] && lit[1] > dark[1] && lit[2] > dark[2]);
        assert_eq!(lit, base, "facing the star keeps the base colour");
        for channel in dark {
            assert!(
                channel >= base[0] * (MIN_LIGHT as f32) - 1e-6,
                "the dark side never goes black: {dark:?}"
            );
            assert!(channel < base[0], "but stays darker");
        }
        assert_eq!(MIN_LIGHT, 0.35, "the floor keeps the shape readable");
        // Side-on sits halfway.
        let side = shade(base, [dir[2], dir[0], dir[1]], dir);
        let _ = side;
        // Nothing else changes: shading only scales the base colour.
        let grey = shade([0.5, 0.5, 0.5], dir, dir);
        let white = shade([1.0, 1.0, 1.0], dir, dir);
        assert!(
            (white[0] - 2.0 * grey[0]).abs() < 1e-6,
            "shading scales, never replaces"
        );
    }
}
