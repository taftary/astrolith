//! Sky shots for light and atmosphere (#780 Q6 A), split from `screens`
//! under `E-FILE-SIZE`.
//!
//! Each shot places the L10 camera `height_m` above the drawn ground at the
//! point where the planet's real, fixed sun stands `from_sun_deg` from
//! straight overhead ([`universe_core::light::from_sun`]), looking toward
//! the horizon on the sun's side, tilted by `pitch_deg` (negative looks
//! down). Orbit looks straight at the planet.

use crate::menu::Screen;
use crate::screens::{Shot, WIDE};

/// Where a sky shot stands and looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkyView {
    /// Height above the drawn ground in metres.
    pub height_m: u32,
    /// Angle of the spot from the sub-solar point in degrees (0 noon,
    /// about 90 the terminator, beyond it night).
    pub from_sun_deg: u16,
    /// View tilt from the horizon in degrees (negative looks down); `None`
    /// looks straight down at the planet's centre.
    pub pitch_deg: Option<i16>,
}

/// A wide L10 sky shot.
pub(crate) const fn sky_shot(file: &'static str, view: SkyView) -> Shot {
    Shot {
        file,
        screen: Screen::Universe,
        size: WIDE,
        panel: None,
        popup: false,
        dive: 9,
        touch: false,
        player: None,
        overlay: false,
        ground_mm: None,
        sky: Some(view),
    }
}

/// Orbit, 2,000 km up over the morning side, looking at the planet.
pub(crate) const SKY_ORBIT: Shot = sky_shot("sky-orbit.png", view(2_000_000, 60, None));
/// 100 km up, at the top of the air, by day.
pub(crate) const SKY_100KM: Shot = sky_shot("sky-100km.png", view(100_000, 40, Some(-20)));
/// 10 km up by day.
pub(crate) const SKY_10KM: Shot = sky_shot("sky-10km.png", view(10_000, 40, Some(-8)));
/// 1 km up by day.
pub(crate) const SKY_1KM: Shot = sky_shot("sky-1km.png", view(1_000, 40, Some(-4)));
/// On the ground by day.
pub(crate) const SKY_GROUND: Shot = sky_shot("sky-ground.png", view(2, 40, Some(5)));
/// On the ground at the terminator (sunset colours).
pub(crate) const SKY_TERMINATOR: Shot = sky_shot("sky-terminator.png", view(2, 88, Some(3)));
/// On the ground on the night side.
pub(crate) const SKY_NIGHT: Shot = sky_shot("sky-night.png", view(2, 150, Some(-10)));

const fn view(height_m: u32, from_sun_deg: u16, pitch_deg: Option<i16>) -> SkyView {
    SkyView {
        height_m,
        from_sun_deg,
        pitch_deg,
    }
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let len = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if len > 0.0 && len.is_finite() {
        [a[0] / len, a[1] / len, a[2] / len]
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// Places the L10 camera for `view` under the open planet's sun. Does
/// nothing off the planet or without a sun.
pub(crate) fn place_for_sky(
    universe: &mut crate::Universe,
    nav: &mut crate::input::Navigation,
    view: SkyView,
) {
    use universe_core::planet_frame::{PlanetFrame, PlanetPos};
    let Some(sun) = universe_core::light::sun_direction(&universe.0) else {
        return;
    };
    let chain = universe.0.path.indices();
    let Some(l10) = chain.get(..9) else {
        return;
    };
    let frame = PlanetFrame::for_planet(universe.0.root.0, l10);
    let relief = universe_core::relief::Relief::for_planet(universe.0.root.0, l10);
    let up = universe_core::light::from_sun(sun, f64::from(view.from_sun_deg).to_radians());
    let r = frame.radius_m() + relief.height_m(up) + f64::from(view.height_m);
    universe
        .0
        .path
        .set_offset(frame.to_cell(PlanetPos([up[0] * r, up[1] * r, up[2] * r])));
    nav.target = None;
    nav.up = up;
    nav.forward = match view.pitch_deg {
        None => [-up[0], -up[1], -up[2]],
        Some(pitch) => {
            // Toward the sun's side of the horizon.
            let d = sun[0] * up[0] + sun[1] * up[1] + sun[2] * up[2];
            let ahead = unit([sun[0] - up[0] * d, sun[1] - up[1] * d, sun[2] - up[2] * d]);
            let (s, c) = f64::from(pitch).to_radians().sin_cos();
            unit([
                ahead[0] * c + up[0] * s,
                ahead[1] * c + up[1] * s,
                ahead[2] * c + up[2] * s,
            ])
        }
    };
}
