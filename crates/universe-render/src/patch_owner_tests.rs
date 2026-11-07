//! Production owner-interest and anchor adapter tests; not rendered handover proof.

use super::*;
use crate::player_state::PlayerState;
use universe_core::moon_catalog::{MoonIdentity, PlanetIdentity};
use universe_core::nav::{MarkerIndex, Universe};
use universe_core::surface_interest::SurfaceInterestKind;
use universe_core::surface_observation::SurfaceIdentity;

fn opened() -> (Universe, SurfaceIdentity, SurfaceIdentity) {
    let mut universe = Universe::new(42);
    for &id in &[7, 2, 7, 1, 2, 0, 0, 0, 5] {
        assert!(universe.open(MarkerIndex(id)));
    }
    let host = PlanetIdentity::from_open_cell(&universe).unwrap();
    let moon = MoonIdentity::new(host, MarkerIndex(1)).unwrap();
    (
        universe,
        SurfaceIdentity::Planet(host),
        SurfaceIdentity::Moon(moon),
    )
}

fn near(universe: &Universe, owner: SurfaceIdentity) -> [f64; 3] {
    let frame = SurfaceFrame::for_owner(universe, owner).unwrap();
    cell_at(frame, PlanetPos([frame.radius().get() + 10.0, 0.0, 0.0])).unwrap()
}

#[test]
fn active_player_approach_beats_camera_and_does_not_commit_visible_ownership() {
    let (universe, planet, moon) = opened();
    let mut tree = PatchTree::default();
    let recipe = universe_core::surface_recipe::SurfaceRecipe::for_owner(planet).unwrap();
    tree.recipe = Some(recipe.key());
    let mut player = PlayerState {
        root: 42,
        active: true,
        ..Default::default()
    };
    player.body.position = near(&universe, moon);
    player.body.flying = true;
    let camera = OpenUnits(near(&universe, planet));
    update_interest(
        &mut tree,
        &universe,
        &GroundSurface::default(),
        Some(&player),
        camera,
    );
    assert_eq!(tree.interest.unwrap().owner, moon);
    assert_eq!(
        tree.interest.unwrap().kind,
        SurfaceInterestKind::PlayerApproach
    );
    assert_eq!(
        tree.recipe,
        Some(recipe.key()),
        "request is not a safe visible handover"
    );
    let candidates = tree.candidates.clone();
    // Same-cell frame updates keep the cached identities, not regenerated recipes.
    update_interest(
        &mut tree,
        &universe,
        &GroundSurface::default(),
        Some(&player),
        camera,
    );
    assert_eq!(tree.candidates, candidates);
    player.active = false;
    update_interest(
        &mut tree,
        &universe,
        &GroundSurface::default(),
        Some(&player),
        camera,
    );
    assert_eq!(tree.interest.unwrap().owner, planet);
    assert_eq!(tree.interest.unwrap().kind, SurfaceInterestKind::Camera);
    update_interest(
        &mut tree,
        &Universe::new(42),
        &GroundSurface::default(),
        Some(&player),
        camera,
    );
    assert!(tree.candidates.is_none() && tree.interest.is_none());
}

#[test]
fn actual_held_moon_query_can_lock_interest_without_using_requested_owner_as_support() {
    let (universe, planet, moon) = opened();
    let frame = SurfaceFrame::for_owner(&universe, moon).unwrap();
    let recipe = universe_core::surface_recipe::SurfaceRecipe::for_owner(moon).unwrap();
    let camera = frame
        .position([frame.radius().get() + 10.0, 0.0, 0.0])
        .unwrap();
    let patches = universe_core::cube_sphere::Face::all()
        .into_iter()
        .map(|face| {
            let key = universe_core::lod::PatchKey::root(face);
            (
                key,
                std::sync::Arc::new(
                    universe_core::lod_mesh::build_patch(
                        key,
                        frame.radius().get(),
                        recipe.relief(),
                    )
                    .unwrap(),
                ),
                0,
            )
        })
        .collect();
    let drawn =
        universe_core::drawn_surface::DrawnSurface::for_surface(frame, camera, 4.0, patches)
            .unwrap();
    let feet = drawn.ground_at(near(&universe, moon)).unwrap().point;
    let mut ground = GroundSurface(Some(drawn), true, true, Some(moon));
    let mut tree = PatchTree::default();
    let mut player = PlayerState {
        root: 42,
        active: true,
        ..Default::default()
    };
    player.body.position = feet;
    // Explicit contact flag is a contract input here, not a simulated landing.
    player.body.on_ground = true;
    update_interest(
        &mut tree,
        &universe,
        &ground,
        Some(&player),
        OpenUnits(near(&universe, planet)),
    );
    assert_eq!(tree.interest.unwrap().owner, moon);
    assert_eq!(tree.interest.unwrap().kind, SurfaceInterestKind::Support);
    // Requested/outer metadata cannot relabel a different held owner's geometry.
    ground.3 = Some(planet);
    update_interest(
        &mut tree,
        &universe,
        &ground,
        Some(&player),
        OpenUnits(near(&universe, planet)),
    );
    assert_eq!(
        tree.interest.unwrap().kind,
        SurfaceInterestKind::PlayerApproach
    );
    ground.3 = Some(moon);
    player.body.flying = true;
    update_interest(
        &mut tree,
        &universe,
        &ground,
        Some(&player),
        OpenUnits(near(&universe, planet)),
    );
    assert_eq!(
        tree.interest.unwrap().kind,
        SurfaceInterestKind::PlayerApproach
    );
}

#[test]
fn real_moving_frame_anchor_conversion_has_no_parent_centre_alias() {
    let (mut universe, _, moon) = opened();
    let first = SurfaceFrame::for_owner(&universe, moon).unwrap();
    let point = PlanetPos([first.radius().get(), 2.0, 3.0]);
    let old = cell_at(first, point).unwrap();
    let read = patch_at(first, old).unwrap();
    let delta = [
        read.0[0] - point.0[0],
        read.0[1] - point.0[1],
        read.0[2] - point.0[2],
    ];
    assert!(delta[0].hypot(delta[1]).hypot(delta[2]) < 0.01);
    universe.advance(86_400.0);
    let next = SurfaceFrame::for_owner(&universe, moon).unwrap();
    assert_ne!(cell_at(next, point).unwrap(), old);
    assert!(patch_at(next, [f64::NAN; 3]).is_none());
    assert!(cell_at(next, PlanetPos([f64::INFINITY; 3])).is_none());
}

#[test]
fn production_schedule_requests_moon_detail_without_claiming_a_visible_handover() {
    let (universe, planet, moon) = opened();
    let mut app = crate::window_tests::headless_app();
    {
        let mut live = app.world_mut().resource_mut::<crate::Universe>();
        live.0 = universe.clone();
        // Explicit camera preparation, not measured player travel or contact.
        live.path.set_offset(near(&universe, moon));
    }
    app.update();
    let tree = app.world().resource::<PatchTree>();
    assert_eq!(tree.interest.unwrap().owner, moon);
    assert_eq!(tree.interest.unwrap().kind, SurfaceInterestKind::Camera);
    assert_eq!(tree.recipe.unwrap().owner(), planet);
    assert_eq!(tree.surface_frame.unwrap().owner(), planet);
    let ground = app.world().resource::<GroundSurface>();
    assert_eq!(ground.3, Some(planet));
    assert_eq!(
        ground.0.as_ref().unwrap().owner_frame().unwrap().owner(),
        planet
    );
    app.world_mut()
        .resource_mut::<crate::Universe>()
        .path
        .set_offset(near(&universe, planet));
    app.update();
    assert_eq!(
        app.world().resource::<PatchTree>().interest.unwrap().owner,
        planet
    );
}
