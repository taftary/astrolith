//! Real metres for every readout (#640, Q7).
//!
//! Distances in the scene are open-planet (L10) cell units, the game's own
//! frame; readouts turn them into metres with the game's planet metre
//! ([`universe_core::shell::metres_per_unit`]), so the lab's heights and
//! shell levels read exactly as the game's. Planet sizes read the game's
//! planet record (`radius_earth`) against Earth's mean radius.

/// Earth's mean radius in kilometres (the lab's size readout).
pub const EARTH_MEAN_RADIUS_KM: f64 = 6_371.0;

/// Metres per scene (L10 cell) unit, as the game measures heights.
#[must_use]
pub fn metres_per_unit() -> f64 {
    universe_core::shell::metres_per_unit()
}

/// A length in metres as text: metres under 10 km, km above, grouped.
#[must_use]
pub fn length_text(metres: f64) -> String {
    if !metres.is_finite() {
        return "-".to_string();
    }
    if metres.abs() < 10_000.0 {
        format!("{} m", group(metres.round()))
    } else {
        format!("{} km", group((metres / 1_000.0).round()))
    }
}

/// A planet radius in Earth radii as km and Earth radii.
#[must_use]
pub fn radius_text(radius_earth: f64) -> String {
    format!(
        "{} km ({radius_earth:.2} Earth)",
        group((radius_earth * EARTH_MEAN_RADIUS_KM).round())
    )
}

/// A round grid spacing for a planet of `radius_m`: the power of ten
/// closest under a fifth of the radius (1,000 km for Earth).
#[must_use]
pub fn grid_spacing_m(radius_m: f64) -> f64 {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return 1.0;
    }
    10f64.powf((radius_m / 5.0).log10().floor())
}

/// `value` with thousands separators (`6,371`), no decimals.
#[must_use]
pub fn group(value: f64) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: a rounded readout, far inside i64"
    )]
    let whole = value as i64;
    let out = group_digits(&whole.unsigned_abs().to_string());
    if whole < 0 { format!("-{out}") } else { out }
}

/// A count with thousands separators (`6,000`).
#[must_use]
pub fn group_count(count: usize) -> String {
    group_digits(&count.to_string())
}

/// `digits` with a comma before every group of three from the right.
fn group_digits(digits: &str) -> String {
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readouts_are_real_metres() {
        // C14 / AC13: Earth reads 6,371 km, the grid is a round km value,
        // heights read in m below 10 km and km above.
        assert_eq!(radius_text(1.0), "6,371 km (1.00 Earth)");
        assert_eq!(grid_spacing_m(6_371_000.0), 1_000_000.0, "1,000 km grid");
        assert_eq!(length_text(5.0), "5 m");
        assert_eq!(length_text(999.6), "1,000 m");
        assert_eq!(length_text(1.0e6), "1,000 km");
        assert_eq!(length_text(-3.0), "-3 m");
        assert_eq!(group(1_234_567.0), "1,234,567");
        // The scene unit is the game's planet metre: 10 m reads L14.
        let ten = 10.0 / metres_per_unit();
        let height = universe_core::shell::height_above_ground_m([
            0.0,
            universe_core::terrain::surface_radius([0.0, 1.0, 0.0]) + ten,
            0.0,
        ]);
        assert!((height - 10.0).abs() < 1e-6, "{height} m");
    }
}
