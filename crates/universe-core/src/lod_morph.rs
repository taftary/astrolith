//! Geomorph blending between LOD levels (rebuild step 4, #777 T2).
//!
//! CDLOD-style morph in core (Q2 A, Q6 A, Q7 A): every vertex stores its
//! parent-level position and normal, and blends by distance over the last
//! 30% of its level's distance band. The vertex shader (T3) runs the same
//! math; this module is the single formula both sides mirror. Pure and
//! deterministic (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use crate::lod::{PatchKey, patch_quad_sag_m};
use crate::precision::focal_px;

/// Share of the distance band used for blending (CDLOD default, Q7 A).
pub const MORPH_ZONE_SHARE: f64 = 0.3;

/// Distance band for `key` in metres: `(start, end)`.
///
/// `err_m` is `max(quad_sag, relief_error)` for the patch. With `detail_px`
/// (Planet detail, S1) and the headless focal length, `start` is where the
/// patch splits (`err / dist * focal == detail`) and `end` is where its
/// parent merges (`== detail / 2`, twice as far). Returns `None` for a bad
/// radius, error, threshold or focal length.
#[must_use]
pub fn morph_band_m(
    key: PatchKey,
    radius_m: f64,
    err_m: f64,
    detail_px: f64,
) -> Option<(f64, f64)> {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return None;
    }
    if !err_m.is_finite() || err_m <= 0.0 {
        return None;
    }
    if !detail_px.is_finite() || detail_px <= 0.0 {
        return None;
    }
    let _ = patch_quad_sag_m(key, radius_m)?;
    let focal = focal_px();
    if !focal.is_finite() || focal <= 0.0 {
        return None;
    }
    let start = err_m / detail_px * focal;
    let end = err_m / (detail_px / 2.0) * focal;
    if !start.is_finite() || !end.is_finite() || start <= 0.0 || end <= start {
        return None;
    }
    Some((start, end))
}

/// Blending factor for `distance_m` inside `(band_start_m, band_end_m)`.
///
/// 0 outside the zone (own shape), 1 at the far end (parent shape), linear
/// in between over the last 30% of the band. Returns 0 for a bad band.
#[must_use]
pub fn morph_factor(distance_m: f64, band_start_m: f64, band_end_m: f64) -> f64 {
    if !distance_m.is_finite()
        || !band_start_m.is_finite()
        || !band_end_m.is_finite()
        || band_start_m <= 0.0
        || band_end_m <= band_start_m
    {
        return 0.0;
    }
    let width = band_end_m - band_start_m;
    if !width.is_finite() || width <= 0.0 {
        return 0.0;
    }
    let zone = width * MORPH_ZONE_SHARE;
    if !zone.is_finite() || zone <= 0.0 {
        return 0.0;
    }
    let morph_start = band_end_m - zone;
    if distance_m <= morph_start {
        return 0.0;
    }
    if distance_m >= band_end_m {
        return 1.0;
    }
    let t = (distance_m - morph_start) / zone;
    if !t.is_finite() {
        return 0.0;
    }
    t.clamp(0.0, 1.0)
}

/// Whether grid index `i` (0..=32) sits on the parent grid (even).
#[must_use]
pub const fn is_parent_grid(i: u32) -> bool {
    i.is_multiple_of(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factor_is_zero_outside_and_one_at_the_end() {
        let (start, end) = (100.0, 200.0);
        assert_eq!(morph_factor(50.0, start, end), 0.0);
        assert_eq!(morph_factor(100.0, start, end), 0.0);
        // Zone is last 30%: 170..200.
        assert_eq!(morph_factor(170.0, start, end), 0.0);
        let mid = morph_factor(185.0, start, end);
        assert!((mid - 0.5).abs() < 1e-12, "mid {mid}");
        assert_eq!(morph_factor(200.0, start, end), 1.0);
        assert_eq!(morph_factor(300.0, start, end), 1.0);
        assert_eq!(morph_factor(f64::NAN, start, end), 0.0);
        assert_eq!(morph_factor(185.0, 200.0, 100.0), 0.0);
    }
}
