//! Contrast, colour-vision, and brightness-ladder rules (#446).
//!
//! Pure WCAG luminance math plus the protanopia simulation behind Spec v1
//! criteria 3-5. Everything here is `f64` on sRGB triples in `0.0..=1.0`,
//! deterministic on every platform (`E-DET-TIERS`). Display code maps core
//! tints through [`crate::contrast::display_tint`] before drawing so snapshots stay
//! byte-identical (`E-GOLDEN`) while dark reds read as orange-red on screen.

use crate::r#gen::ColorInfo;

/// Minimum WCAG contrast of anything actionable against black (Spec v1 AC3).
pub const CONTRAST_FLOOR: f64 = 3.0;

/// Dimmest brightness multiplier an actionable marker is drawn at.
///
/// Even white fails 3:1 below `0.35`, so the drawers clamp portal, selection,
/// portal-mark, and label brightness to at least this. Populations and
/// background context keep the shell curve down to zero.
pub const ACTIONABLE_BRIGHTNESS_FLOOR: f64 = 0.6;

/// Ladder steps by role (Spec v1 AC5): the entered shell brightest, children
/// just below it, background context at least two stops (four times) under
/// the children.
pub const LADDER_SHELL: f64 = 1.0;

/// Ladder step for previewed children, just under the shell.
pub const LADDER_CHILD: f64 = 0.85;

/// Ladder step for background context, over two stops under the children.
pub const LADDER_CONTEXT: f64 = 0.08;

/// Contrast below which a protan-mapped tint counts as near black (AC4).
pub const PROTAN_BLACK_LIMIT: f64 = 1.5;

/// Linearises one sRGB channel in `0.0..=1.0` (WCAG transfer).
#[must_use]
pub fn srgb_to_linear(channel: f64) -> f64 {
    if !channel.is_finite() || channel <= 0.0 {
        return 0.0;
    }
    if channel >= 1.0 {
        return 1.0;
    }
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// Relative luminance of an sRGB triple (WCAG definition).
#[must_use]
pub fn relative_luminance(rgb: [f64; 3]) -> f64 {
    let (r, g, b) = (
        srgb_to_linear(rgb[0]),
        srgb_to_linear(rgb[1]),
        srgb_to_linear(rgb[2]),
    );
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// WCAG contrast ratio of `rgb` against black.
#[must_use]
pub fn contrast_against_black(rgb: [f64; 3]) -> f64 {
    (relative_luminance(rgb) + 0.05) / 0.05
}

/// sRGB triple scaled by a brightness multiplier, clamped to the cube.
#[must_use]
pub fn apply_brightness(rgb: [f64; 3], brightness: f64) -> [f64; 3] {
    let scale = if brightness.is_finite() && brightness > 0.0 {
        brightness
    } else {
        0.0
    };
    [
        (rgb[0] * scale).clamp(0.0, 1.0),
        (rgb[1] * scale).clamp(0.0, 1.0),
        (rgb[2] * scale).clamp(0.0, 1.0),
    ]
}

/// Contrast against black of `rgb` drawn at `brightness`.
#[must_use]
pub fn contrast_at_brightness(rgb: [f64; 3], brightness: f64) -> f64 {
    contrast_against_black(apply_brightness(rgb, brightness))
}

/// Protanopia simulation of an sRGB triple (Machado et al. 2009 matrix).
///
/// Applied in sRGB as the guides' accessibility analysis does: enough to
/// prove no mapped tint collapses toward black, without a full appearance
/// model.
#[must_use]
pub fn protanopia(rgb: [f64; 3]) -> [f64; 3] {
    let (r, g, b) = (rgb[0], rgb[1], rgb[2]);
    [
        (0.567 * r + 0.433 * g).clamp(0.0, 1.0),
        (0.558 * r + 0.442 * g).clamp(0.0, 1.0),
        (0.242 * g + 0.758 * b).clamp(0.0, 1.0),
    ]
}

/// Whether a (possibly protan-mapped) tint reads as near black.
#[must_use]
pub fn is_near_black(rgb: [f64; 3]) -> bool {
    contrast_against_black(rgb) < PROTAN_BLACK_LIMIT
}

/// How many stops brighter `bright` is than `dim` (`log2` of the ratio).
///
/// Non-positive or non-finite inputs yield `0.0`.
#[must_use]
pub fn stops_apart(bright: f64, dim: f64) -> f64 {
    if !bright.is_finite() || !dim.is_finite() || bright <= 0.0 || dim <= 0.0 {
        return 0.0;
    }
    (bright / dim).log2()
}

/// Whether the three ladder steps keep their spacing (AC5).
///
/// The shell sits at or above the children, and the context rests at least
/// two stops (four times) under the children.
#[must_use]
pub fn ladder_ok(shell: f64, child: f64, context: f64) -> bool {
    shell >= child && stops_apart(child, context) >= 2.0
}

/// Display tint for one core tint (Spec v1 AC4).
///
/// Dark reds (red-dominant triples the protan simulation would sink toward
/// black at low brightness) become a lighter orange-red; everything else
/// crosses unchanged, brightness kept. Snapshots keep the sampler value;
/// only drawing maps through this.
#[must_use]
pub fn display_tint(tint: ColorInfo) -> ColorInfo {
    let dark_red = tint.red >= 0.8
        && tint.green <= 0.45
        && tint.blue <= 0.35
        && tint.red > tint.green
        && tint.green >= tint.blue;
    if dark_red {
        ColorInfo {
            red: 1.0,
            green: 0.45,
            blue: 0.20,
            brightness: tint.brightness,
        }
    } else {
        tint
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astro::{GALAXY_TINT, GREEN_VALLEY};
    use crate::system::BLACKBODY;

    /// sRGB triples of every actionable tint at full brightness.
    fn actionable_srgb() -> Vec<[f64; 3]> {
        let mut tints = Vec::new();
        for (_, rgb, _) in BLACKBODY {
            tints.push(rgb);
        }
        for (_, rgb, _) in GALAXY_TINT {
            let mapped = display_tint(ColorInfo {
                red: rgb[0],
                green: rgb[1],
                blue: rgb[2],
                brightness: 1.0,
            });
            tints.push([mapped.red, mapped.green, mapped.blue]);
        }
        tints.push(GREEN_VALLEY);
        // Era tints drawn by `universe-render` style levels.
        tints.push([0.3, 0.8, 1.0]);
        tints.push([1.0, 0.85, 0.4]);
        tints.push([0.4, 1.0, 0.5]);
        tints.push([0.7, 0.5, 1.0]);
        tints.push([1.0, 0.6, 0.2]);
        tints.push([0.3, 0.9, 0.8]);
        tints.push([1.0, 1.0, 1.0]);
        tints
    }

    #[test]
    fn every_actionable_tint_reaches_three_to_one_at_its_dimmest_step() {
        for tint in actionable_srgb() {
            let contrast = contrast_at_brightness(tint, ACTIONABLE_BRIGHTNESS_FLOOR);
            assert!(
                contrast >= CONTRAST_FLOOR,
                "tint {tint:?} reads {contrast:.2} at the floor"
            );
        }
    }

    #[test]
    fn no_tint_maps_near_black_under_protanopia() {
        for tint in actionable_srgb() {
            let dimmed = apply_brightness(tint, ACTIONABLE_BRIGHTNESS_FLOOR);
            let mapped = protanopia(dimmed);
            assert!(
                !is_near_black(mapped),
                "tint {tint:?} collapses under protanopia: {mapped:?}"
            );
        }
    }

    #[test]
    fn ladder_steps_sit_two_stops_apart() {
        assert!(ladder_ok(LADDER_SHELL, LADDER_CHILD, LADDER_CONTEXT));
        assert!(stops_apart(LADDER_CHILD, LADDER_CONTEXT) >= 2.0);
        const {
            assert!(LADDER_SHELL >= LADDER_CHILD);
        }
        assert!(!ladder_ok(1.0, 0.5, 0.2), "a close context must fail");
    }

    #[test]
    fn display_tint_lifts_dark_reds_and_keeps_the_rest() {
        let elliptical = display_tint(ColorInfo {
            red: 0.85,
            green: 0.38,
            blue: 0.30,
            brightness: 1.0,
        });
        assert_eq!(
            (elliptical.red, elliptical.green, elliptical.blue),
            (1.0, 0.45, 0.20)
        );
        assert_eq!(elliptical.brightness, 1.0);
        let dwarf = display_tint(ColorInfo {
            red: 1.0,
            green: 0.55,
            blue: 0.35,
            brightness: 0.45,
        });
        assert_eq!((dwarf.red, dwarf.green, dwarf.blue), (1.0, 0.55, 0.35));
        let spiral = display_tint(ColorInfo {
            red: 0.45,
            green: 0.65,
            blue: 1.0,
            brightness: 1.0,
        });
        assert_eq!((spiral.red, spiral.green, spiral.blue), (0.45, 0.65, 1.0));
    }

    #[test]
    fn transfer_is_total_and_monotone() {
        assert_eq!(srgb_to_linear(f64::NAN), 0.0);
        assert_eq!(srgb_to_linear(0.0), 0.0);
        assert_eq!(srgb_to_linear(1.0), 1.0);
        assert!(srgb_to_linear(0.5) > srgb_to_linear(0.25));
        assert_eq!(contrast_against_black([0.0; 3]), 1.0);
        assert!(contrast_against_black([1.0; 3]) > 20.0);
        assert_eq!(stops_apart(1.0, 0.25), 2.0);
        assert_eq!(stops_apart(f64::NAN, 0.5), 0.0);
    }
}
