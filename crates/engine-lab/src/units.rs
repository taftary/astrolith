//! Real metres for every readout (#640, Q7).
//!
//! Distances in the scene are open-planet (L10) cell units, the game's own
//! frame; readouts turn them into metres with the game's planet metre
//! ([`universe_core::shell::metres_per_unit`], one metre per planet, #648),
//! so the lab's heights, distances and shell levels read exactly as the
//! game's and the ground sits at the planet's real radius. Planet sizes
//! read the game's planet record (`radius_earth`) against the game's one
//! Earth radius ([`universe_core::terrain::EARTH_RADIUS_KM`]).

/// Metres per scene (L10 cell) unit on a planet of `radius_earth` Earth
/// radii, as the game measures heights.
#[must_use]
pub fn metres_per_unit(radius_earth: f64) -> f64 {
    universe_core::shell::metres_per_unit(radius_earth)
}

/// Metres per scene unit where `universe` is open: the planet's own metre
/// on a planet (#648), the level's ladder metre above it (#654).
#[must_use]
pub fn scene_metres(universe: &universe_core::nav::Universe) -> f64 {
    universe
        .path
        .planet_metres_per_unit()
        .unwrap_or_else(|| 1.0 / universe_core::body::metre(universe.level()))
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
        group((radius_earth * universe_core::terrain::EARTH_RADIUS_KM).round())
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
        let ten = 10.0 / metres_per_unit(1.0);
        let height = universe_core::shell::height_above_ground_m(
            [
                0.0,
                universe_core::terrain::surface_radius([0.0, 1.0, 0.0]) + ten,
                0.0,
            ],
            1.0,
        );
        assert!((height - 10.0).abs() < 1e-6, "{height} m");
    }

    #[test]
    fn the_lab_ground_reads_the_planet_size() {
        // C1-C2 / AC1-AC2 of #648: on every planet of the home star, the
        // overlay's distance to the centre at a ground corner reads the
        // planet's size readout (Earth: 6,371 km, not 4,509 km).
        use crate::stats::{FrameTimes, SceneReadout, overlay_lines};
        let mut scene = crate::scene::LabScene::default_home();
        let corner = universe_core::icosphere::all_at_depth(universe_core::terrain::FACET_DEPTH)
            .first()
            .and_then(|tri| universe_core::icosphere::tri_corners(*tri))
            .map(|corners| corners[0])
            .expect("a face");
        let mut sizes = Vec::new();
        for index in 0..scene.planet_count() {
            scene.select(index);
            let universe = scene.universe();
            let Some(surface) = universe.open.points.iter().find_map(|point| point.surface) else {
                continue;
            };
            let ground = corner.map(|c| c * universe_core::terrain::surface_radius(corner));
            let readout = SceneReadout {
                triangles: 0,
                meshes: 0,
                position_m: ground.map(|unit| unit * scene_metres(&universe)),
                height_m: 0.0,
                shell: 14,
                size: None,
            };
            let lines = overlay_lines(true, &FrameTimes::default(), &readout);
            let size = radius_text(surface.radius_earth);
            let km = size.split(" (").next().unwrap_or_default();
            assert_eq!(lines[3], format!("distance {km}"), "{size}");
            sizes.push(surface.radius_earth);
        }
        assert!(sizes.contains(&1.0), "Earth is there: {sizes:?}");
        assert!(
            sizes.iter().any(|r| *r != 1.0),
            "and another planet: {sizes:?}"
        );
        assert!(scene_metres(&crate::scene::LabScene::default_home().universe()).is_finite());
    }

    #[test]
    fn one_earth_radius_everywhere() {
        // C7 / AC7 of #648: the lab and the game share one Earth radius,
        // the mean 6,371 km, and the ground of Earth sits on it.
        assert_eq!(universe_core::terrain::EARTH_RADIUS_KM, 6_371.0);
        assert_eq!(universe_core::gravity::EARTH_RADIUS_M, 6_371_000.0);
        assert_eq!(radius_text(1.0), "6,371 km (1.00 Earth)");
        let corner_m = universe_core::terrain::PLANET_RADIUS_CELL * metres_per_unit(1.0);
        assert_eq!(length_text(corner_m), "6,371 km");
    }
}
