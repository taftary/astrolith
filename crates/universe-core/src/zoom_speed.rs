//! One zoom speed both ways (#998 build T1, B1).
//!
//! Zooming in toward a target, the dive's step is a share of the gap to the
//! target's surface, so the speed shrinks smoothly as you arrive. Zooming
//! out of a cell's centre used the distance to the centre instead, so after
//! a level closed the camera was suddenly far from the new centre and sped
//! up as much as 14 times (the #998 study). Going out, the step now follows
//! the distance to the nearest drawn thing: the nearest marker's surface in
//! the open cell or in its parent (the level you came from is drawn too),
//! or the centre when that is nearer, so the speed carries across a close.
//! It never falls below [`CENTRE_SHARE`](crate::zoom_speed::CENTRE_SHARE) of the distance to the centre, so
//! rising past a marker on the way out never stalls, and each step changes
//! by at most [`EASE`](crate::zoom_speed::EASE) from the last one, so a nearer marker coming into
//! view after a close slows the zoom over a few frames instead of at once.
//! Pure (`E-CORE-NO-BEVY`, `E-DET-TIERS`).

use crate::coords::OpenUnits;
use crate::frame::Opened;
use crate::r#gen::Point;

fn length3(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// The most a rising step changes from the last, per frame (5%).
pub const EASE: f64 = 1.05;

/// The least share of the distance to the cell's centre a rising step
/// follows, so passing near a marker never stalls the zoom.
pub const CENTRE_SHARE: f64 = 0.05;

/// The distance from `camera` to the nearest drawn thing in the open cell:
/// the nearest of `points`' surfaces (radius at least `marker_radius`) and
/// the cell's centre, and never below [`CENTRE_SHARE`] of the distance to
/// the centre (nor a millionth of `marker_radius`).
#[must_use]
pub fn nearest_gap(camera: [f64; 3], points: &[Point], marker_radius: f64) -> f64 {
    let mut gap = length3(camera);
    for point in points {
        let d = [
            camera[0] - point.position[0],
            camera[1] - point.position[1],
            camera[2] - point.position[2],
        ];
        let surface = length3(d) - point.radius.max(marker_radius);
        if surface.is_finite() && surface < gap {
            gap = surface;
        }
    }
    gap.max(length3(camera) * CENTRE_SHARE)
        .max(marker_radius * 1e-6)
        .max(f64::MIN_POSITIVE)
}

/// The parent level as seen from the open cell: its markers, the frame the
/// open cell was entered through, and the parent's marker radius.
#[derive(Clone, Copy, Debug)]
pub struct ParentView<'a> {
    /// The parent cell's markers (parent units).
    pub points: &'a [Point],
    /// The open cell's entry.
    pub entered: Opened,
    /// The parent level's marker radius (parent units).
    pub marker_radius: f64,
}

/// [`nearest_gap`] including the parent's markers, re-expressed in the open
/// cell's units through the entry (their radii scaled by the entry ratio).
#[must_use]
pub fn nearest_gap_with_parent(
    camera: [f64; 3],
    points: &[Point],
    marker_radius: f64,
    parent: Option<ParentView<'_>>,
) -> f64 {
    let mut gap = nearest_gap(camera, points, marker_radius);
    if let Some(view) = parent
        && view.entered.ratio > 0.0
    {
        let in_parent = view.entered.to_parent_point(camera);
        for point in view.points {
            let d = [
                in_parent[0] - point.position[0],
                in_parent[1] - point.position[1],
                in_parent[2] - point.position[2],
            ];
            let surface = (length3(d) - point.radius.max(view.marker_radius)) / view.entered.ratio;
            if surface.is_finite() && surface > 0.0 && surface < gap {
                gap = surface;
            }
        }
    }
    gap.max(length3(camera) * CENTRE_SHARE)
}

/// One step out from the cell's centre at zoom `factor` (above 1): along
/// the line from the centre, by the nearest gap (open cell and parent)
/// times `factor - 1`, eased to within [`EASE`] of `last` (the last rising
/// step, in this cell's units, when there was one). Returns the new place
/// and the step length taken, in cell units.
#[must_use]
pub fn rise_step(
    camera: OpenUnits,
    points: &[Point],
    marker_radius: f64,
    parent: Option<ParentView<'_>>,
    factor: f64,
    last: Option<f64>,
) -> (OpenUnits, f64) {
    let distance = length3(camera.0);
    let direction = if distance > 0.0 {
        [
            camera.0[0] / distance,
            camera.0[1] / distance,
            camera.0[2] / distance,
        ]
    } else {
        [0.0, 0.0, 1.0]
    };
    let want = nearest_gap_with_parent(camera.0, points, marker_radius, parent) * (factor - 1.0);
    let step = match last.filter(|l| l.is_finite() && *l > 0.0) {
        Some(last) => want.clamp(last / EASE, last * EASE),
        None => want,
    };
    (
        OpenUnits([
            camera.0[0] + direction[0] * step,
            camera.0[1] + direction[1] * step,
            camera.0[2] + direction[2] * step,
        ]),
        step,
    )
}
