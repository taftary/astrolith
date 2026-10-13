//! The planet surface before you enter it, and its fade (#603).
//!
//! While the level that holds a planet is open (L9), the planet's own
//! flat-faced icosphere surface (`surface_mesh`, #593) fades in over the last
//! part of the approach to its marker: from half the opening size to full
//! strength exactly when the planet cell opens ([`approach_fade`], Q2 A).
//! It is the planet cell's own [`SurfaceFrame`], split by the same rule as
//! inside (#598, Q3 A) with the camera mapped into the planet's units the
//! way the open maps it, and drawn scaled back by the cell ratio. So when
//! the cell opens, the same triangles stay in the same place. Inside the
//! planet cell (L10) the same fade runs on the way out, so leaving fades
//! the surface instead of dropping it. Everything here is `pub(crate)`.

use crate::Universe;
use crate::input::Navigation;
use universe_core::frame::angular_radius;
use universe_core::nest::{approach_fade, child_ratio, generate_cell};
use universe_core::surface::SurfaceFrame;

/// The surface the window draws this frame: the open cell's own, or the
/// planet being approached from the level above.
pub(crate) struct SurfaceHost {
    /// The surface cell (the open L10-L13 cell, or the approached planet).
    pub(crate) frame: SurfaceFrame,
    /// The approached planet's marker in the open cell (`None` inside).
    pub(crate) marker: Option<u32>,
    /// Where the surface cell's centre sits in open-cell units.
    pub(crate) offset: [f64; 3],
    /// Surface-cell units per open-cell unit, inverted: one surface-cell
    /// unit is `ratio` open-cell units (`1.0` inside).
    pub(crate) ratio: f64,
    /// Strength the surface draws at, `0.0..=1.0`.
    pub(crate) fade: f64,
    /// The target in surface-cell units, if one leads through it.
    pub(crate) target: Option<[f64; 3]>,
}

impl SurfaceHost {
    /// An open-cell point in the surface cell's units (identity up: the
    /// planet opens upright, like [`universe_core::nav::Universe::open`]).
    #[must_use]
    pub(crate) fn to_surface(&self, point: [f64; 3]) -> [f64; 3] {
        [
            (point[0] - self.offset[0]) / self.ratio,
            (point[1] - self.offset[1]) / self.ratio,
            (point[2] - self.offset[2]) / self.ratio,
        ]
    }
}

/// Strength of the planet surface seen from `camera` in its own cell's
/// units: the fade of the cell shell (radius `0.5`) at that distance.
#[must_use]
pub(crate) fn shell_fade(camera: [f64; 3]) -> f64 {
    let distance = (camera[0] * camera[0] + camera[1] * camera[1] + camera[2] * camera[2]).sqrt();
    approach_fade(angular_radius(0.5, distance))
}

/// Brightness share of a planet portal's marker outline at the level above
/// a planet (#603, Q4 A): `1.0` while its surface is hidden, falling to
/// `0.0` as the surface fades in. `1.0` for every other marker and level.
#[must_use]
pub(crate) fn marker_share(universe: &Universe, point: &universe_core::r#gen::Point) -> f64 {
    let level = universe.level();
    if point.kind != universe_core::r#gen::MarkerKind::Portal
        || level.deeper().map(universe_core::coords::Level::get) != Some(10)
    {
        return 1.0;
    }
    let Some(ratio) = child_ratio(level) else {
        return 1.0;
    };
    let camera = universe.path.offset();
    let at = point.portal_position();
    1.0 - shell_fade([
        (camera[0] - at[0]) / ratio,
        (camera[1] - at[1]) / ratio,
        (camera[2] - at[2]) / ratio,
    ])
}

/// The surface to draw for `universe` this frame, if any (#603).
///
/// Inside L10-L13 it is the open cell's frame (fading on the way out at
/// L10). At the level above a planet it is the nearest planet portal whose
/// fade is above zero; `None` otherwise.
#[must_use]
pub(crate) fn surface_host(universe: &Universe, nav: &Navigation) -> Option<SurfaceHost> {
    let camera = universe.path.offset();
    if let Some(frame) = universe.frame.as_ref() {
        let fade = if frame.level().get() == 10 {
            shell_fade(camera)
        } else {
            1.0
        };
        let target = nav
            .target
            .and_then(|marker| universe.marker(universe_core::nav::MarkerIndex(marker)));
        return Some(SurfaceHost {
            frame: frame.clone(),
            marker: None,
            offset: [0.0; 3],
            ratio: 1.0,
            fade,
            target,
        });
    }
    let level = universe.level();
    if level.deeper().map(universe_core::coords::Level::get) != Some(10) {
        return None;
    }
    let ratio = child_ratio(level)?;
    let (marker, offset, fade) = universe
        .open
        .points
        .iter()
        .enumerate()
        .filter(|(_, point)| point.kind == universe_core::r#gen::MarkerKind::Portal)
        .filter_map(|(index, point)| {
            let at = point.portal_position();
            let inside = [
                (camera[0] - at[0]) / ratio,
                (camera[1] - at[1]) / ratio,
                (camera[2] - at[2]) / ratio,
            ];
            let fade = shell_fade(inside);
            (fade > 0.0).then(|| (u32::try_from(index).ok(), at, fade))
        })
        .filter_map(|(index, at, fade)| Some((index?, at, fade)))
        .max_by(|a, b| a.2.total_cmp(&b.2))?;
    let root = universe.root.0;
    let mut chain = universe.path.indices();
    chain.push(marker);
    let frame = SurfaceFrame::for_path(root, &chain)?;
    // The next region on the way, when the destination runs through here.
    let target = nav.destination.and_then(|destination| {
        let markers = destination.markers();
        let region = *markers.get(chain.len())?;
        if markers.get(..chain.len())? != chain.as_slice() {
            return None;
        }
        let planet = generate_cell(root, &chain);
        Some(
            planet
                .points
                .get(usize::try_from(region).ok()?)?
                .portal_position(),
        )
    });
    Some(SurfaceHost {
        frame,
        marker: Some(marker),
        offset,
        ratio,
        fade,
        target,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface_split::{SplitView, surface_triangles};
    use universe_core::nav::MarkerIndex;
    use universe_core::nest::OPEN_ANGLE;

    /// The demo universe opened down to L9 along the autopilot journey.
    fn at_l9() -> Universe {
        let mut universe = Universe(universe_core::nav::Universe::new(
            universe_core::nav::DEMO_SEED,
        ));
        for _ in 0..8 {
            let marker = universe.autopilot_target().expect("dive continues");
            assert!(universe.open(MarkerIndex(marker)));
        }
        assert_eq!(universe.level().get(), 9);
        universe
    }

    /// Camera offset (L9 units) on the line out of `marker` at a distance
    /// where the planet shell looks `angular` radians wide.
    fn camera_at(universe: &Universe, marker: u32, angular: f64) -> [f64; 3] {
        let at = universe.open.points[marker as usize].portal_position();
        let ratio = child_ratio(universe.level()).expect("ratio");
        let distance = 0.5 * ratio / angular.sin();
        [at[0], at[1], at[2] + distance]
    }

    #[test]
    fn far_away_no_planet_surface_shows() {
        let mut universe = at_l9();
        let marker = universe.autopilot_target().expect("planet");
        let camera = camera_at(&universe, marker, OPEN_ANGLE * 0.3);
        universe.path.set_offset(camera);
        assert!(surface_host(&universe, &Navigation::default()).is_none());
    }

    #[test]
    fn the_surface_fades_in_by_distance_and_out_again() {
        let mut universe = at_l9();
        let marker = universe.autopilot_target().expect("planet");
        let nav = Navigation::default();
        let mut fades = Vec::new();
        for share in [0.55, 0.7, 0.85, 0.99] {
            let camera = camera_at(&universe, marker, OPEN_ANGLE * share);
            universe.path.set_offset(camera);
            let host = surface_host(&universe, &nav).expect("approached planet");
            assert_eq!(host.marker, Some(marker));
            fades.push(host.fade);
        }
        assert!(
            fades.windows(2).all(|w| w[1] > w[0]),
            "grows as you approach: {fades:?}"
        );
        assert!(fades[3] > 0.99, "near full at the open");
        // Back out: weaker again.
        let camera = camera_at(&universe, marker, OPEN_ANGLE * 0.6);
        universe.path.set_offset(camera);
        let back = surface_host(&universe, &nav)
            .expect("still approached")
            .fade;
        assert!(back < fades[1], "moving away fades it");
    }

    #[test]
    fn the_approach_maps_like_the_open_and_draws_the_same_triangles() {
        let mut universe = at_l9();
        let marker = universe.autopilot_target().expect("planet");
        let camera = camera_at(&universe, marker, OPEN_ANGLE * 0.999);
        universe.path.set_offset(camera);
        let host = surface_host(&universe, &Navigation::default()).expect("approached");
        let mapped = host.to_surface(camera);
        let forward = [0.0, 0.0, -1.0];
        let fov = std::f64::consts::FRAC_PI_4;
        let before = surface_triangles(
            &host.frame,
            &SplitView::camera(mapped, forward, [0.0, 1.0, 0.0], fov, 16.0 / 9.0, 1080.0),
        );
        // Open the planet: the camera lands where the approach mapped it,
        // and the open frame is the same surface.
        assert!(universe.open(MarkerIndex(marker)));
        let inside = universe.path.offset();
        for axis in 0..3 {
            assert!(
                (inside[axis] - mapped[axis]).abs() < 1e-9,
                "the approach maps the camera like the open"
            );
        }
        let frame = universe.frame.as_ref().expect("planet frame");
        assert_eq!(frame, &host.frame, "the same surface cell");
        let after = surface_triangles(
            frame,
            &SplitView::camera(inside, forward, [0.0, 1.0, 0.0], fov, 16.0 / 9.0, 1080.0),
        );
        let a: Vec<_> = before.iter().map(|t| t.addr).collect();
        let b: Vec<_> = after.iter().map(|t| t.addr).collect();
        assert_eq!(a, b, "the same triangles on both sides of the open");
        // Inside, just past the open, the fade is full.
        let host = surface_host(&universe, &Navigation::default()).expect("inside");
        assert!(host.fade > 0.99, "full strength at the open");
    }

    #[test]
    fn leaving_the_planet_fades_the_surface() {
        let mut universe = at_l9();
        let marker = universe.autopilot_target().expect("planet");
        assert!(universe.open(MarkerIndex(marker)));
        let near = shell_fade([0.0, 0.0, 3.0]);
        let far = shell_fade([0.0, 0.0, 9.0]);
        assert_eq!(near, 1.0, "near the planet: full");
        assert_eq!(far, 0.0, "about to close: gone");
        universe.path.set_offset([0.0, 0.0, 6.0]);
        let host = surface_host(&universe, &Navigation::default()).expect("inside");
        assert!(host.fade > 0.0 && host.fade < 1.0, "on the way out: fading");
    }

    #[test]
    fn the_marker_outline_fades_out_as_the_surface_fades_in() {
        let mut universe = at_l9();
        let marker = universe.autopilot_target().expect("planet");
        let point = universe.open.points[marker as usize];
        for share in [0.3, 0.6, 0.8, 1.2] {
            let camera = camera_at(&universe, marker, OPEN_ANGLE * share);
            universe.path.set_offset(camera);
            let surface = surface_host(&universe, &Navigation::default()).map_or(0.0, |h| h.fade);
            let outline = marker_share(&universe, &point);
            assert!(
                (surface + outline - 1.0).abs() < 1e-12,
                "never both at full strength: {surface} + {outline}"
            );
        }
        // Populations and other levels keep their full outline.
        let population = universe
            .open
            .points
            .iter()
            .find(|p| p.kind == universe_core::r#gen::MarkerKind::Population);
        if let Some(population) = population {
            assert_eq!(marker_share(&universe, population), 1.0);
        }
    }
}
