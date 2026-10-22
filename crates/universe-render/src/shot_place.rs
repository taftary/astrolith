//! Placement for shot-file shots (#863) and the built-in player shots
//! ([`player_for_shot`]), split from `screens` under `E-FILE-SIZE`.
//!
//! Each [`ShotSpec`] becomes a template [`Shot`] (the level's dive, a
//! player shot when it has a figure) and is then placed by
//! [`place_file_shot`], once per shot after the built-in setup:
//! - The universe opens the demo journey to the shot's level.
//! - On the planet a `lat`/`lon` place is a direction in the planet frame
//!   (`+Y` is the pole, longitude grows east):
//!   `(cos lat cos lon, sin lat, -cos lat sin lon)`, at the radius plus the
//!   relief height there plus `h`. A heading turns from north toward east
//!   in the local frame there, and a tilt lifts it.
//! - `x y z` are planet metres at L10 (through [`PlanetFrame::to_cell`])
//!   and the open cell's own units above it.
//! - With a figure, the walking figure stands at the spot facing the
//!   heading, and the third-person camera sits behind it.

use crate::menu::Screen;
use crate::screens::{Sequencer, Shot, WIDE};
use crate::shot_file::{ShotFile, ShotLook, ShotPlace, ShotSpec};
use bevy::prelude::*;
use universe_core::planet_frame::{LocalFrame, PlanetFrame, PlanetPos};

/// The built-in shot a file shot runs as: the level's dive (or a player
/// shot with a figure) in the wide window. Its name comes from the spec.
#[must_use]
pub(crate) fn template(spec: &ShotSpec) -> Shot {
    Shot {
        file: "",
        screen: Screen::Universe,
        size: WIDE,
        panel: None,
        popup: false,
        dive: usize::from(spec.level.saturating_sub(1)),
        touch: false,
        player: spec.figure.then_some(true),
        overlay: spec.overlay,
        ground_mm: None,
        sky: None,
    }
}

fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let len = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    (len.is_finite() && len > 0.0).then(|| [a[0] / len, a[1] / len, a[2] / len])
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The planet-frame direction of latitude `lat` and longitude `lon`
/// (degrees): `+Y` is the pole, longitude grows east. Pure.
#[must_use]
pub(crate) fn lat_lon_dir(lat: f64, lon: f64) -> [f64; 3] {
    let (sl, cl) = lat.to_radians().sin_cos();
    let (so, co) = lon.to_radians().sin_cos();
    [cl * co, sl, -cl * so]
}

/// The look along compass `heading` (degrees from north toward east) and
/// `tilt` (degrees up) in `local`. Pure.
#[must_use]
pub(crate) fn heading_tilt_dir(local: &LocalFrame, heading: f64, tilt: f64) -> [f64; 3] {
    let (sh, ch) = heading.to_radians().sin_cos();
    let (st, ct) = tilt.to_radians().sin_cos();
    let flat = [
        local.north[0] * ch + local.east[0] * sh,
        local.north[1] * ch + local.east[1] * sh,
        local.north[2] * ch + local.east[2] * sh,
    ];
    [
        flat[0] * ct + local.up[0] * st,
        flat[1] * ct + local.up[1] * st,
        flat[2] * ct + local.up[2] * st,
    ]
}

/// The body yaw that faces `forward` on the ground with `up`
/// ([`universe_core::body::heading_basis`]'s convention). Pure.
#[must_use]
pub(crate) fn yaw_facing(up: [f64; 3], forward: [f64; 3]) -> f64 {
    let (ahead, _) = universe_core::body::heading_basis(up, 0.0);
    let side = cross(ahead, up);
    (-dot(forward, side)).atan2(dot(forward, ahead))
}

/// A camera pose: open-cell offset, forward and up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Pose {
    /// Camera position in open-cell units.
    pub offset: [f64; 3],
    /// Unit look direction.
    pub forward: [f64; 3],
    /// Unit up.
    pub up: [f64; 3],
}

/// The pose of `spec` in a universe opened to its level (`frame` and
/// `relief` at L10). Pure; `None` when the look has no direction.
#[must_use]
pub(crate) fn pose_for(
    spec: &ShotSpec,
    planet: Option<(&PlanetFrame, &universe_core::relief::Relief)>,
) -> Option<Pose> {
    match (spec.place, planet) {
        (ShotPlace::Planet { lat, lon, h }, Some((frame, relief))) => {
            let dir = lat_lon_dir(lat, lon);
            let r = frame.radius_m() + relief.height_m(dir) + h;
            let at = PlanetPos([dir[0] * r, dir[1] * r, dir[2] * r]);
            let local = LocalFrame::at(at);
            let forward = match spec.look {
                ShotLook::HeadingTilt { heading, tilt } => heading_tilt_dir(&local, heading, tilt),
                ShotLook::Dir(d) => d,
            };
            Some(Pose {
                offset: frame.to_cell(at),
                forward: unit(forward)?,
                up: local.up,
            })
        }
        (ShotPlace::Xyz(xyz), Some((frame, _))) => {
            let up = unit(xyz).unwrap_or([0.0, 1.0, 0.0]);
            let ShotLook::Dir(d) = spec.look else {
                return None;
            };
            Some(Pose {
                offset: frame.to_cell(PlanetPos(xyz)),
                forward: unit(d)?,
                up,
            })
        }
        (ShotPlace::Xyz(xyz), None) => {
            let ShotLook::Dir(d) = spec.look else {
                return None;
            };
            Some(Pose {
                offset: xyz,
                forward: unit(d)?,
                up: [0.0, 1.0, 0.0],
            })
        }
        (ShotPlace::Planet { .. }, None) => None,
    }
}

/// Parks the universe at the seeded spot in normal mode for a player shot.
///
/// Opens the spot's chain to L10, places the body there, and picks third or
/// first person. The camera itself is posed by `sync_camera` from this state,
/// so the third-person shot shows the whole figure and the first-person one
/// looks down at the torso (#755).
pub(crate) fn player_for_shot(
    universe: &mut crate::Universe,
    nav: &mut crate::input::Navigation,
    flight: &mut crate::input::Flight,
    normal: &mut crate::player_state::PlayerState,
    third: bool,
) {
    use universe_core::nav::{DEMO_SEED, MarkerIndex};
    let spot = universe_core::player::PlayerSpot::seeded(DEMO_SEED)
        .unwrap_or_else(universe_core::player::PlayerSpot::at_root);
    universe.0 = universe_core::nav::Universe::new(DEMO_SEED);
    for &marker in spot.chain() {
        if !universe.0.open(MarkerIndex(marker)) {
            break;
        }
    }
    let up = universe_core::body::level_up(&universe.0, spot.position);
    let mut body = universe_core::body::Body::standing(spot.position, spot.yaw);
    // Standing; it drops onto the drawn ground once the patches are built
    // (#779 Q7 A).
    body.flying = false;
    normal.spot = spot;
    normal.seeded = spot;
    normal.body = body;
    normal.active = true;
    normal.third_person = third;
    normal.handover_from = None;
    normal.ground_pending = true;
    normal.anchor = universe_core::body_frame::BodyAnchor::default();
    normal.handover_t = 0.0;
    flight.mode = crate::input::FlightMode::Normal;
    flight.yaw = spot.yaw;
    flight.target_yaw = spot.yaw;
    // Third person looks down at the arrival tilt so the whole figure stays
    // in frame; first person looks almost straight down so the torso, arms
    // and legs fill the frame without the head.
    flight.pitch = if third {
        crate::player_camera::arrival_tilt()
    } else {
        -1.5
    };
    flight.target_pitch = flight.pitch;
    nav.forward = universe_core::body::heading_basis(up, spot.yaw).0;
    nav.up = up;
    nav.target = None;
}

/// Opens the demo journey to `level` (1 to 10) from a fresh universe.
fn open_to_level(universe: &mut crate::Universe, level: u8) {
    use universe_core::nav::{DEMO_SEED, MarkerIndex};
    let chain = universe_core::nest::autopilot_path(DEMO_SEED);
    universe.0 = universe_core::nav::Universe::new(DEMO_SEED);
    for &marker in chain.iter().take(usize::from(level.saturating_sub(1))) {
        if !universe.0.open(MarkerIndex(marker)) {
            break;
        }
    }
}

/// The planet frame and relief of the open L10 cell, if any.
fn planet_of(universe: &crate::Universe) -> Option<(PlanetFrame, universe_core::relief::Relief)> {
    if universe.0.level().get() != 10 {
        return None;
    }
    let chain = universe.0.path.indices();
    let l10 = chain.get(..9)?;
    Some((
        PlanetFrame::for_planet(universe.0.root.0, l10),
        universe_core::relief::Relief::for_planet(universe.0.root.0, l10),
    ))
}

/// Places the current file shot once, after the built-in setup (#863).
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn place_file_shot(
    seq: Res<Sequencer>,
    file: Option<Res<ShotFile>>,
    universe: Option<ResMut<crate::Universe>>,
    nav: Option<ResMut<crate::input::Navigation>>,
    flight: Option<ResMut<crate::input::Flight>>,
    normal: Option<ResMut<crate::player_state::PlayerState>>,
    mut placed: Local<Option<usize>>,
) {
    let (Some(file), Some(mut universe), Some(mut nav), Some(mut flight)) =
        (file, universe, nav, flight)
    else {
        return;
    };
    let Some(spec) = file.0.get(seq.shot) else {
        return;
    };
    if *placed == Some(seq.shot) {
        return;
    }
    *placed = Some(seq.shot);
    if !spec.figure {
        open_to_level(&mut universe, spec.level);
        flight.mode = crate::input::FlightMode::Dive;
    }
    let planet = planet_of(&universe);
    let Some(pose) = pose_for(spec, planet.as_ref().map(|(f, r)| (f, r))) else {
        return;
    };
    if spec.figure {
        // The player shot set up the body at the seeded spot; move it here,
        // facing the heading, and keep the third-person camera behind it.
        let Some(mut normal) = normal else {
            return;
        };
        let yaw = yaw_facing(pose.up, pose.forward);
        let mut body = universe_core::body::Body::standing(pose.offset, yaw);
        body.flying = false;
        normal.body = body;
        normal.ground_pending = true;
        flight.yaw = yaw;
        flight.target_yaw = yaw;
        nav.forward = universe_core::body::heading_basis(pose.up, yaw).0;
        nav.up = pose.up;
        nav.target = None;
        return;
    }
    universe.0.path.set_offset(pose.offset);
    nav.target = None;
    nav.forward = pose.forward;
    nav.up = pose.up;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shot_file::parse_shot_file;

    fn earth() -> (PlanetFrame, universe_core::relief::Relief) {
        let chain = universe_core::nest::autopilot_path(universe_core::nav::DEMO_SEED);
        let l10 = chain.get(..9).expect("journey");
        (
            PlanetFrame::for_planet(universe_core::nav::DEMO_SEED, l10),
            universe_core::relief::Relief::for_planet(universe_core::nav::DEMO_SEED, l10),
        )
    }

    #[test]
    fn planet_terms_place_the_camera() {
        // C2 (#863 AC2): lat, lon and h put the camera h metres above the
        // drawn ground in that direction; heading 0 looks north, 90 east,
        // tilt lifts the look.
        let (frame, relief) = earth();
        let spec =
            &parse_shot_file("a L10 lat=12.5 lon=-40 h=2 heading=270 tilt=-5").expect("reads")[0];
        let pose = pose_for(spec, Some((&frame, &relief))).expect("pose");
        let at = frame.from_cell(pose.offset);
        let dir = lat_lon_dir(12.5, -40.0);
        let above = at.0[0] * dir[0] + at.0[1] * dir[1] + at.0[2] * dir[2]
            - (frame.radius_m() + relief.height_m(dir));
        assert!((above - 2.0).abs() < 0.01, "2 m above the ground: {above}");
        let local = LocalFrame::at(at);
        // Heading 270 is west: against east, along the horizon, tilted 5 down.
        let west = -dot(pose.forward, local.east);
        let up = dot(pose.forward, local.up);
        assert!((west - 5f64.to_radians().cos()).abs() < 1e-9, "west {west}");
        assert!((up + 5f64.to_radians().sin()).abs() < 1e-9, "down {up}");
        // Latitude 90 is the pole (+Y), longitude 90 points along -Z (east).
        let pole = lat_lon_dir(90.0, 0.0);
        assert!((pole[1] - 1.0).abs() < 1e-12);
        let east = lat_lon_dir(0.0, 90.0);
        assert!((east[2] + 1.0).abs() < 1e-12);
        let north = heading_tilt_dir(&local, 0.0, 0.0);
        assert!((dot(north, local.north) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn raw_coordinates_work_at_every_level() {
        // C3 (#863 AC3): x y z put the camera there and dir sets the look,
        // at L10 in planet metres and above it in cell units.
        let (frame, relief) = earth();
        let spec =
            &parse_shot_file("p L10 x=1200000 y=6200000 z=-300000 dir=0,-2,0").expect("reads")[0];
        let pose = pose_for(spec, Some((&frame, &relief))).expect("pose");
        let at = frame.from_cell(pose.offset);
        for (got, want) in at.0.iter().zip([1_200_000.0, 6_200_000.0, -300_000.0]) {
            assert!((got - want).abs() < 1e-3, "{got} vs {want}");
        }
        assert_eq!(pose.forward, [0.0, -1.0, 0.0]);
        for level in 1..=9u8 {
            let text = format!("g L{level} x=0.1 y=0.2 z=3 dir=0,0,-1");
            let spec = &parse_shot_file(&text).expect("reads")[0];
            let pose = pose_for(spec, None).expect("pose above the planet");
            assert_eq!(pose.offset, [0.1, 0.2, 3.0]);
            assert_eq!(pose.forward, [0.0, 0.0, -1.0]);
            assert_eq!(template(spec).dive, usize::from(level - 1));
        }
    }

    #[test]
    fn figure_shot_stands_the_body() {
        // C4 (#863 AC4): a figure shot is a third-person player shot, and
        // the body faces the heading on the ground at the spot.
        let (frame, relief) = earth();
        let spec = &parse_shot_file("w L10 lat=30 lon=10 figure heading=90").expect("reads")[0];
        assert_eq!(template(spec).player, Some(true));
        let pose = pose_for(spec, Some((&frame, &relief))).expect("pose");
        let yaw = yaw_facing(pose.up, pose.forward);
        let (facing, _) = universe_core::body::heading_basis(pose.up, yaw);
        assert!(dot(facing, pose.forward) > 1.0 - 1e-9, "faces the heading");
        let local = LocalFrame::at(frame.from_cell(pose.offset));
        assert!(dot(facing, local.east) > 1.0 - 1e-9, "heading 90 is east");
    }
}
