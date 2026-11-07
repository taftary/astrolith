//! Frozen conversions and explicit owner-following are different operations.

use super::*;
use crate::moon_catalog::{MoonIdentity, PlanetIdentity};
use crate::nav::{MarkerIndex, Seed};

fn opened(chain: &[u32]) -> (Universe, PlanetIdentity, MoonIdentity) {
    let host = PlanetIdentity::new(Seed(42), chain).unwrap();
    let moon = MoonIdentity::new(host, MarkerIndex(1)).unwrap();
    let mut universe = Universe::new(42);
    for &id in chain {
        assert!(universe.open(MarkerIndex(id)));
    }
    (universe, host, moon)
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2])
}

#[test]
fn frozen_moon_conversions_keep_position_and_heading_at_seams_and_poles() {
    for chain in [[7, 2, 7, 1, 2, 0, 0, 0, 5], [7, 2, 7, 3, 2894, 0, 0, 0, 1]] {
        let (mut universe, _, moon) = opened(&chain);
        for time in [0.0, 86_400.0, 2_000_000.0] {
            universe.advance(time);
            let frame = SurfaceFrame::for_owner(&universe, SurfaceIdentity::Moon(moon)).unwrap();
            assert_eq!(frame.time_s(), time);
            assert_eq!(frame.patch_frame().radius_m(), frame.radius().get());
            let saved_universe = universe.clone();
            for dir in [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, -1.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 1.0, 1.0],
                [-1.0, 0.0, -1.0],
            ] {
                let direction = SurfaceDirection::new(dir).unwrap();
                for height in [0.0, 1.75, 10.0, 100_000.0] {
                    let point = frame
                        .position(
                            direction
                                .components()
                                .map(|v| v * (frame.radius().get() + height)),
                        )
                        .unwrap();
                    let cell = frame.to_cell(point).unwrap();
                    let back = frame.from_cell(cell).unwrap();
                    assert!(distance(back.metres(), point.metres()) <= 0.01);
                    let cell_back = frame.to_cell(back).unwrap();
                    assert!(distance(cell.0, cell_back.0) * metres_per_cell_l10() <= 0.01);
                    let before = CellDirection(dir);
                    let encoded = frame.from_cell_direction(before).unwrap();
                    let after = frame.to_cell_direction(encoded);
                    assert!(
                        crate::frame_diagnostic::direction_error(before, after)
                            .unwrap()
                            .0
                            <= 0.05
                    );
                    let patch = frame.patch_position(back).unwrap();
                    assert_eq!(patch.0, back.metres());
                    let tangent = crate::planet_frame::LocalFrame::at(patch);
                    let local = tangent.to_local(patch);
                    let centred_back = tangent.to_planet(local);
                    let cell_from_tangent = frame
                        .to_cell(frame.position(centred_back.0).unwrap())
                        .unwrap();
                    assert!(distance(cell.0, cell_from_tangent.0) * metres_per_cell_l10() <= 0.01);
                }
            }
            assert_eq!(
                universe, saved_universe,
                "a conversion advances neither time nor body"
            );
        }
    }
}

#[test]
fn local_spot_can_follow_actual_clock_but_frozen_cell_position_does_not() {
    let (mut universe, _, moon) = opened(&[7, 2, 7, 1, 2, 0, 0, 0, 5]);
    let identity = SurfaceIdentity::Moon(moon);
    let first = SurfaceFrame::for_owner(&universe, identity).unwrap();
    assert_eq!(first.radius().get(), 1_738_000.0);
    let spot = first.position([first.radius().get(), 12.0, 3.0]).unwrap();
    let old_cell = first.to_cell(spot).unwrap();
    universe.advance(crate::terrain::MOON_PERIOD_DAYS * 86_400.0 / 4.0);
    let next = SurfaceFrame::for_owner(&universe, identity).unwrap();
    let carried = next.to_cell(spot).unwrap();
    assert!(distance(carried.0, old_cell.0) * metres_per_cell_l10() > 100_000.0);
    assert!(distance(next.from_cell(carried).unwrap().metres(), spot.metres()) < 0.01);
    assert!(distance(next.from_cell(old_cell).unwrap().metres(), spot.metres()) > 100_000.0);
    assert_ne!(first.centre(), next.centre());
    // The old frozen frame remains unchanged; transporting requires the caller
    // to explicitly encode the same owned spot through a new clock snapshot.
    assert_eq!(first.to_cell(spot).unwrap(), old_cell);
}

#[test]
fn another_owner_or_cell_is_refused_instead_of_substituted() {
    let (universe, host, moon) = opened(&[7, 2, 7, 1, 2, 0, 0, 0, 5]);
    let planet = SurfaceFrame::for_owner(&universe, SurfaceIdentity::Planet(host)).unwrap();
    let moon_frame = SurfaceFrame::for_owner(&universe, SurfaceIdentity::Moon(moon)).unwrap();
    let point = moon_frame.position([1.0, 2.0, 3.0]).unwrap();
    assert!(planet.to_cell(point).is_none());
    assert!(planet.patch_position(point).is_none());
    assert!(SurfaceFrame::for_owner(&Universe::new(42), SurfaceIdentity::Moon(moon)).is_none());
    assert!(SurfaceFrame::for_owner(&Universe::new(99), SurfaceIdentity::Moon(moon)).is_none());
    assert!(moon_frame.position([f64::NAN; 3]).is_none());
    assert!(
        moon_frame
            .from_cell(OpenUnits([f64::INFINITY; 3]))
            .is_none()
    );
    assert!(
        moon_frame
            .from_cell_direction(CellDirection([0.0; 3]))
            .is_none()
    );
    assert!(
        moon_frame
            .from_cell_direction(CellDirection([f64::NAN; 3]))
            .is_none()
    );
}

#[test]
fn planet_adapter_preserves_existing_metric_without_a_cell_translation() {
    let (universe, host, _) = opened(&[7, 2, 7, 1, 2, 0, 0, 0, 5]);
    let frame = SurfaceFrame::for_owner(&universe, SurfaceIdentity::Planet(host)).unwrap();
    assert_eq!(frame.centre(), OpenUnits([0.0; 3]));
    let old = crate::nav::planet_frame(&universe).unwrap();
    assert_eq!(frame.patch_frame(), old);
    let cell = OpenUnits([0.1, -0.2, 0.3]);
    assert_eq!(
        frame.from_cell(cell).unwrap().metres(),
        old.from_cell(cell.0).0
    );
    assert!(universe.path.matches_indices(host.chain()));
    assert!(!universe.path.matches_indices(&host.chain()[..8]));
    assert!(!universe.path.matches_indices(&[0; 9]));
}

#[test]
fn finite_positive_radius_validation_does_not_impose_a_landing_cutoff() {
    for radius in [1.0, 200_000.0, 1_699_999.999, 1_738_000.0] {
        assert_eq!(SurfaceRadiusMetres::new(radius).unwrap().get(), radius);
    }
    for radius in [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(SurfaceRadiusMetres::new(radius).is_none());
    }
}

#[test]
fn resolved_moon_gravity_agrees_with_actual_production_strongest_pull() {
    for chain in [[7, 2, 7, 1, 2, 0, 0, 0, 5], [7, 2, 7, 3, 2894, 0, 0, 0, 1]] {
        let (universe, _, moon) = opened(&chain);
        let frame = SurfaceFrame::for_owner(&universe, SurfaceIdentity::Moon(moon)).unwrap();
        let r = frame.radius().get();
        let above = frame.position([r + 10.0, 0.0, 0.0]).unwrap();
        let cell = frame.to_cell(above).unwrap();
        let pull = crate::gravity::pull(&universe, cell.0);
        assert_eq!(
            pull.source,
            Some(crate::gravity::PullSource::Marker(moon.marker()))
        );
        let g = crate::gravity::G * crate::gravity::moon_mass_kg(r) / (r + 10.0).powi(2);
        assert!((pull.strongest - g).abs() < 1e-9);
        assert!(pull.strongest < 2.0);
    }
}
