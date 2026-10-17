//! Limb preview brightness for the L10 planet approach (#430).
//!
//! Indicators only. Everything here is `pub(crate)`: only this crate uses it.

/// Brightness of the L10 limb preview around the targeted region (#430).
///
/// Fades in by the target's angular size times the kept-context level, so
/// at the open instant it equals the post-entry limb brightness
/// (`SHELL_FLOOR`) and the mesh swap shows no pop; invisible while the
/// region is a point. Pure.
#[must_use]
pub(crate) fn limb_preview_brightness(target_angular: f64) -> f32 {
    use universe_core::frame::{children_brightness, shell_brightness};
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: render-domain narrowing of a brightness, intended"
    )]
    let faded =
        children_brightness(target_angular) as f32 * shell_brightness(target_angular) as f32;
    faded
}

#[cfg(test)]
mod tests {
    use super::*;
    use universe_core::nest::{OPEN_ANGLE, PREVIEW_ANGLE, SHELL_FLOOR};

    #[test]
    fn limb_preview_fades_in_and_meets_the_kept_limb_at_open() {
        assert_eq!(limb_preview_brightness(PREVIEW_ANGLE), 0.0);
        assert_eq!(limb_preview_brightness(PREVIEW_ANGLE * 0.5), 0.0);
        assert_eq!(limb_preview_brightness(f64::NAN), 0.0);
        let at_open = limb_preview_brightness(OPEN_ANGLE);
        assert!((f64::from(at_open) - SHELL_FLOOR).abs() < 1e-6);
        let past_open = limb_preview_brightness(OPEN_ANGLE * 1.5);
        assert!(
            past_open <= at_open,
            "the preview never outshines the kept limb"
        );
        assert!(past_open > 0.0, "still lit just past open");
        assert!((0.0..=1.0).contains(&at_open));
    }
}
