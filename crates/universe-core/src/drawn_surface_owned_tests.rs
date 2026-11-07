//! Same real shader/morph geometry query, translated through an actual moon frame.
//! These pure checks do not demonstrate a rendered handover or a player landing.

use super::*;
use crate::coords::OpenUnits;
use crate::moon_catalog::{MoonIdentity, PlanetIdentity};
use crate::nav::{MarkerIndex, Universe};
use crate::surface_frame::SurfaceFrame;
use crate::surface_observation::SurfaceIdentity;

#[test]
fn actual_moon_frame_translates_the_same_drawn_triangles_at_seams_and_poles() {
    let mut universe = Universe::new(42);
    for &id in &[7, 2, 7, 1, 2, 0, 0, 0, 5] {
        assert!(universe.open(MarkerIndex(id)));
    }
    let host = PlanetIdentity::from_open_cell(&universe).unwrap();
    let moon = SurfaceIdentity::Moon(MoonIdentity::new(host, MarkerIndex(1)).unwrap());
    let first = SurfaceFrame::for_owner(&universe, moon).unwrap();
    let recipe = crate::surface_recipe::SurfaceRecipe::for_owner(moon).unwrap();
    let patches: Vec<_> = crate::cube_sphere::Face::all()
        .into_iter()
        .map(|face| {
            let key = PatchKey::root(face);
            (
                key,
                Arc::new(
                    crate::lod_mesh::build_patch(key, first.radius().get(), recipe.relief())
                        .unwrap(),
                ),
                0,
            )
        })
        .collect();
    let camera = first
        .position([first.radius().get() + 10.0, 0.0, 0.0])
        .unwrap();
    let mut owned = DrawnSurface::for_surface(first, camera, 4.0, patches.clone()).unwrap();
    let centred = DrawnSurface::new(
        first.patch_frame(),
        first.patch_position(camera).unwrap(),
        4.0,
        patches,
    );
    for time in [0.0, 86_400.0] {
        universe.advance(time);
        let frame = SurfaceFrame::for_owner(&universe, moon).unwrap();
        assert!(owned.set_owner_frame(frame));
        assert_eq!(owned.owner_frame(), Some(frame));
        for input in [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
        ] {
            let dir = crate::moon_approach::SurfaceDirection::new(input)
                .unwrap()
                .components();
            let point = frame
                .position(dir.map(|v| v * (frame.radius().get() + 100.0)))
                .unwrap();
            let cell = frame.to_cell(point).unwrap();
            let query = owned.patch_position(cell).unwrap();
            let expected = centred.ground_at_planet(query).unwrap();
            let actual = owned.ground_at(cell.0).unwrap();
            assert_eq!(actual.up, expected.1);
            assert_eq!(actual.point, owned.cell_position(expected.0).unwrap().0);
            let read_back = owned.patch_position(OpenUnits(actual.point)).unwrap();
            let delta = [
                read_back.0[0] - expected.0.0[0],
                read_back.0[1] - expected.0.0[1],
                read_back.0[2] - expected.0.0[2],
            ];
            assert!(delta[0].hypot(delta[1]).hypot(delta[2]) <= 0.01);
            assert_eq!(owned.drawn_triangle_at(query).unwrap().point, expected.0);
            assert_eq!(owned.place_on_ground(cell.0), Some(actual.point));
        }
    }
    let planet = SurfaceFrame::for_owner(&universe, SurfaceIdentity::Planet(host)).unwrap();
    assert!(!owned.set_owner_frame(planet));
    assert!(DrawnSurface::for_surface(planet, camera, 4.0, Vec::new()).is_none());
    let ordinary = DrawnSurface::new(planet.patch_frame(), PlanetPos([0.0; 3]), 4.0, Vec::new());
    assert!(ordinary.owner_frame().is_none());
}
