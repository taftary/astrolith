//! Pure priority/hysteresis checks on real generated identities and drawn queries.
//! A chosen work owner is not evidence of player travel, landing or readiness.

use super::*;
use crate::body::Body;
use crate::moon_catalog::PlanetIdentity;
use crate::nav::{MarkerIndex, Seed};

fn opened(chain: &[u32]) -> (Universe, SurfaceCandidates) {
    let mut universe = Universe::new(42);
    for &id in chain {
        assert!(universe.open(MarkerIndex(id)));
    }
    let host = PlanetIdentity::from_open_cell(&universe).unwrap();
    let candidates = SurfaceCandidates::for_host(host).unwrap();
    (universe, candidates)
}

fn near(universe: &Universe, owner: SurfaceIdentity) -> OpenUnits {
    let frame = SurfaceFrame::for_owner(universe, owner).unwrap();
    frame
        .to_cell(
            frame
                .position([frame.radius().get() + 10.0, 0.0, 0.0])
                .unwrap(),
        )
        .unwrap()
}

#[test]
fn every_existing_moon_can_win_approach_including_small_sample_and_rich_host() {
    for chain in [
        [7, 2, 7, 1, 2, 0, 0, 0, 5],
        [7, 2, 7, 3, 2894, 0, 0, 0, 1],
        [7, 2, 7, 1, 2, 0, 0, 0, 2],
    ] {
        let (mut universe, candidates) = opened(&chain);
        let planet = SurfaceIdentity::Planet(PlanetIdentity::from_open_cell(&universe).unwrap());
        let snapshot = crate::nest::generate_cell(42, &chain);
        assert!(candidates.owners().count() <= LOCAL_SURFACE_LIMIT);
        for time in [0.0, 86_400.0] {
            universe.advance(time);
            let frozen = universe.clone();
            for owner in candidates.owners() {
                let at = near(&universe, owner);
                let choice = candidates
                    .choose(
                        &universe,
                        Some(planet),
                        None,
                        Some(at),
                        near(&universe, planet),
                    )
                    .unwrap();
                assert_eq!(choice.owner, owner);
                assert_eq!(choice.kind, SurfaceInterestKind::PlayerApproach);
                let camera = candidates.choose(&universe, None, None, None, at).unwrap();
                assert_eq!(camera.owner, owner);
                assert_eq!(camera.kind, SurfaceInterestKind::Camera);
            }
            assert_eq!(
                universe, frozen,
                "selection does not advance time or travel"
            );
        }
        assert_eq!(snapshot, crate::nest::generate_cell(42, &chain));
    }
}

#[test]
fn hysteresis_holds_near_the_boundary_but_allows_planet_moon_return() {
    let (universe, candidates) = opened(&[7, 2, 7, 1, 2, 0, 0, 0, 5]);
    let owners: Vec<_> = candidates.owners().collect();
    let planet = owners[0];
    let moon = owners[1];
    let pf = SurfaceFrame::for_owner(&universe, planet).unwrap();
    let mf = SurfaceFrame::for_owner(&universe, moon).unwrap();
    let centre = pf.from_cell(mf.centre()).unwrap().metres();
    let d = centre[0].hypot(centre[1]).hypot(centre[2]);
    let midpoint = (d + pf.radius().get() - mf.radius().get()) * 0.5;
    let corridor = (d - pf.radius().get() - mf.radius().get()) * 0.5;
    let at = pf
        .to_cell(
            pf.position(centre.map(|v| v / d * (midpoint + corridor * 0.01)))
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        candidates
            .choose(&universe, None, None, Some(at), at)
            .unwrap()
            .owner,
        moon
    );
    assert_eq!(
        candidates
            .choose(&universe, Some(planet), None, Some(at), at)
            .unwrap()
            .owner,
        planet
    );
    let at = near(&universe, moon);
    assert_eq!(
        candidates
            .choose(&universe, Some(planet), None, Some(at), at)
            .unwrap()
            .owner,
        moon
    );
    let at = near(&universe, planet);
    assert_eq!(
        candidates
            .choose(&universe, Some(moon), None, Some(at), at)
            .unwrap()
            .owner,
        planet
    );
}

#[test]
fn only_matching_body_contact_can_lock_interest_against_a_detached_camera() {
    let (universe, candidates) = opened(&[7, 2, 7, 1, 2, 0, 0, 0, 5]);
    let owners: Vec<_> = candidates.owners().collect();
    let planet = owners[0];
    let moon = owners[1];
    let mut feet = Body::standing(near(&universe, planet).0, 0.0);
    // Explicit contract inputs, not a simulated landing or a proximity-derived
    // contact flag: test the support validator independently of body integration.
    feet.on_ground = true;
    let observation = GroundObservation::from_ground(
        planet,
        crate::body::Ground {
            point: feet.position,
            up: [1.0, 0.0, 0.0],
        },
        OpenUnits(feet.position),
        crate::planet_frame::metres_per_cell_l10(),
    )
    .unwrap();
    let support = SurfaceSupport::from_contact(&feet, observation).unwrap();
    let choice = candidates
        .choose(
            &universe,
            Some(moon),
            Some(support),
            Some(OpenUnits(feet.position)),
            near(&universe, moon),
        )
        .unwrap();
    assert_eq!(
        choice,
        SurfaceInterest {
            owner: planet,
            kind: SurfaceInterestKind::Support
        }
    );
    feet.flying = true;
    assert!(SurfaceSupport::from_contact(&feet, observation).is_none());
    feet.flying = false;
    feet.on_ground = false;
    assert!(SurfaceSupport::from_contact(&feet, observation).is_none());
    feet.on_ground = true;
    feet.position[0] += 1.0;
    assert!(SurfaceSupport::from_contact(&feet, observation).is_none());
    let mut hovering = observation;
    hovering.clearance.0 = 10.0;
    feet.position = observation.feet.0;
    assert!(SurfaceSupport::from_contact(&feet, hovering).is_none());
    hovering.clearance.0 = f64::NAN;
    assert!(SurfaceSupport::from_contact(&feet, hovering).is_none());
    let at = near(&universe, moon);
    assert_eq!(
        candidates
            .choose(
                &universe,
                Some(planet),
                None,
                Some(at),
                near(&universe, planet)
            )
            .unwrap()
            .owner,
        moon
    );
}

#[test]
fn mismatched_cell_or_invalid_active_body_is_not_replaced_with_camera_interest() {
    let (universe, candidates) = opened(&[7, 2, 7, 1, 2, 0, 0, 0, 5]);
    let at = near(&universe, candidates.owners().next().unwrap());
    assert!(
        candidates
            .choose(&universe, None, None, Some(OpenUnits([f64::NAN; 3])), at)
            .is_none()
    );
    assert!(!candidates.matches(&Universe::new(42)));
    assert!(
        candidates
            .choose(&Universe::new(42), None, None, None, at)
            .is_none()
    );
    let foreign = PlanetIdentity::new(Seed(42), &[7, 2, 7, 1, 2, 0, 0, 0, 2]).unwrap();
    assert!(
        candidates
            .choose(
                &universe,
                Some(SurfaceIdentity::Planet(foreign)),
                None,
                None,
                at
            )
            .is_some()
    );
}
