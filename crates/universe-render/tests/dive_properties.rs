//! Property test for the dive step (M2 for #85).
//!
//! Lives in `universe-render` because `dive_step` lives here until M3 moves
//! it to `universe-core::nav` together with this test ("tests move with
//! their names"). `proptest` is a dev-dependency only (`E-PROTEST`).

use proptest::prelude::*;
use universe_render::dive_step;

proptest! {
    /// `dive_step` never crosses the surface for any factor in (0, 1]: the
    /// stepped camera stays at least `radius` from the center.
    #[test]
    fn dive_step_never_crosses_the_surface_for_factor_in_0_1(
        camera in (-10.0f64..10.0, -10.0f64..10.0, -10.0f64..10.0),
        center in (-10.0f64..10.0, -10.0f64..10.0, -10.0f64..10.0),
        radius in 0.001f64..10.0,
        factor in 0.0001f64..=1.0,
    ) {
        let camera = [camera.0, camera.1, camera.2];
        let center = [center.0, center.1, center.2];
        let next = dive_step(camera, center, radius, factor);
        let distance = next
            .iter()
            .zip(center.iter())
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt();
        prop_assert!(
            distance + 1e-9 >= radius,
            "crossed the surface: distance {distance} < radius {radius}",
        );
    }
}
