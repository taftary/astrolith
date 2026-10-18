//! Player up stopgap tests (#778 F2 A): at L10 with no ground, up points
//! away from the planet centre, so the eye stays above the drawn ground.

use crate::body::{Body, eye_of, level_up};
use crate::nav::{DEMO_SEED, MarkerIndex, Universe};
use crate::planet_frame::PlanetFrame;
use crate::player::PlayerSpot;
use crate::relief::Relief;

#[test]
fn player_up_is_radial_at_l10() {
    let spot = PlayerSpot::seeded(DEMO_SEED).expect("spot");
    let mut universe = Universe::new(DEMO_SEED);
    for &marker in spot.chain() {
        if !universe.open(MarkerIndex(marker)) {
            break;
        }
    }
    assert_eq!(universe.level().get(), 10, "spot chain reaches L10");
    let p = spot.position;
    let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
    let up = level_up(&universe, p);
    for (got, want) in up.iter().zip([p[0] / len, p[1] / len, p[2] / len]) {
        assert!((got - want).abs() < 1e-12, "up {up:?}");
    }
    // The eye is above the body's feet along that up, so above the drawn
    // ground (the spot floats 2 m over the relief).
    let chain = universe.path.indices();
    let l10 = chain.get(..9).expect("chain");
    let frame = PlanetFrame::for_planet(DEMO_SEED, l10);
    let relief = Relief::for_planet(DEMO_SEED, l10);
    let body = Body::standing(p, spot.yaw);
    let (eye, eye_up) = eye_of(&universe, &body);
    assert_eq!(eye_up, up);
    let feet = relief.height_above_m(frame.from_cell(p));
    let eye_above = relief.height_above_m(frame.from_cell(eye));
    assert!(
        eye_above > feet,
        "eye {eye_above} m not above feet {feet} m"
    );
    // Off the planet the fallback stays +Y.
    assert_eq!(
        level_up(&Universe::new(DEMO_SEED), [0.3, 0.4, 0.5]),
        [0.0, 1.0, 0.0]
    );
}
