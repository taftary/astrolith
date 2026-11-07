//! Read-only witnesses of the production body's coordinate conversions (#1238).
//!
//! Each comparison holds one physical pose frozen. Body integration, clock
//! motion and camera offsets are deliberately not part of conversion error.

use crate::coords::OpenUnits;
use crate::planet_frame::{LocalFrame, LocalPos, PlanetFrame};

/// A direction in the current open cell's axes, not a position or camera offset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellDirection(pub [f64; 3]);

/// A direction in the recorded local tangent frame's axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalDirection(pub [f64; 3]);

/// Finite nonnegative positional round-trip difference, in metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionErrorMetres(pub f64);

/// Finite nonnegative angular difference, in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionErrorDegrees(pub f64);

/// One actual encoding and inverse of the same frozen pose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrozenConversion {
    /// Physical point before the conversion, in open-cell units.
    pub cell_before: OpenUnits,
    /// Actual encoded point used by the production step, in local metres.
    pub encoded_local: LocalPos,
    /// The encoded point converted back, in open-cell units.
    pub cell_after: OpenUnits,
    /// Actual heading before encoding, in open-cell axes.
    pub forward_before: CellDirection,
    /// Actual local heading encoded by the production yaw conversion.
    pub encoded_forward: LocalDirection,
    /// Encoded local heading converted back into open-cell axes.
    pub forward_after: CellDirection,
    /// Open -> local -> open difference of that one point.
    pub cell_round_trip: PositionErrorMetres,
    /// Local -> open -> local difference of that one encoded point.
    pub local_round_trip: PositionErrorMetres,
    /// Difference of that one heading, never consecutive-frame rotation.
    pub direction_error: DirectionErrorDegrees,
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    let [ax, ay, az] = a;
    let [bx, by, bz] = b;
    ax * bx + ay * by + az * bz
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let [ax, ay, az] = a;
    let [bx, by, bz] = b;
    (ax - bx).hypot(ay - by).hypot(az - bz)
}

/// Angular difference in degrees; absent for a zero/non-finite direction.
#[must_use]
pub fn direction_error(a: CellDirection, b: CellDirection) -> Option<DirectionErrorDegrees> {
    let unit = |v| {
        let n = dot(v, v).sqrt();
        (n.is_finite() && n > 0.0).then(|| v.map(|v| v / n))
    };
    let [ax, ay, az] = unit(a.0)?;
    let [bx, by, bz] = unit(b.0)?;
    let cross = [ay * bz - az * by, az * bx - ax * bz, ax * by - ay * bx];
    let angle = dot(cross, cross)
        .sqrt()
        .atan2(ax * bx + ay * by + az * bz)
        .to_degrees();
    angle.is_finite().then_some(DirectionErrorDegrees(angle))
}

impl FrozenConversion {
    pub(crate) fn from_encoding(
        frame: &PlanetFrame,
        local: &LocalFrame,
        cell_before: OpenUnits,
        encoded_local: LocalPos,
        forward_before: CellDirection,
        encoded_forward: LocalDirection,
    ) -> Option<Self> {
        let cell_after = OpenUnits(frame.to_cell(local.to_planet(encoded_local)));
        let back_local = local.to_local(frame.from_cell(cell_after.0));
        let forward_after =
            CellDirection(crate::body_frame::to_planet_vec(local, encoded_forward.0));
        let cell_error = distance(cell_before.0, cell_after.0) * frame.metres_per_cell();
        let local_error = distance(encoded_local.0, back_local.0);
        if !cell_error.is_finite() || !local_error.is_finite() {
            return None;
        }
        Some(Self {
            cell_before,
            encoded_local,
            cell_after,
            forward_before,
            encoded_forward,
            forward_after,
            cell_round_trip: PositionErrorMetres(cell_error),
            local_round_trip: PositionErrorMetres(local_error),
            direction_error: direction_error(forward_before, forward_after)?,
        })
    }
}

/// Which production coordinate path this step actually took.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FramePath {
    /// No drawn near-ground frame was available; no numeric conversion witness.
    #[default]
    Cell,
    /// Entered the existing local near-ground frame.
    Entered,
    /// Continued in the same local frame.
    Retained,
    /// Re-anchored to an actual new ground point.
    Reanchored,
    /// Left the near-ground local frame and stepped in cell units.
    Left,
}

/// Bounded record supplied only when the caller explicitly requests diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FrameStepDiagnostic {
    /// Actual selected coordinate path, not an assertion's requested path.
    pub path: FramePath,
    /// Previous production anchor, if any.
    pub previous_anchor: Option<LocalFrame>,
    /// Anchor used for this step, if any.
    pub anchor: Option<LocalFrame>,
    /// One frozen pose before body integration.
    pub input: Option<FrozenConversion>,
    /// One frozen pose after body integration, compared only with itself.
    pub output: Option<FrozenConversion>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_pose_errors_detect_bad_encoding_not_motion_between_steps() {
        let frame = PlanetFrame::for_radius_earth(1.0);
        let position = crate::planet_frame::PlanetPos([frame.radius_m(), 2.0, 3.0]);
        let local = LocalFrame::at(position);
        let cell = OpenUnits(frame.to_cell(position));
        let heading = CellDirection(local.north);
        let encoded = local.to_local(position);
        let exact = FrozenConversion::from_encoding(
            &frame,
            &local,
            cell,
            encoded,
            heading,
            LocalDirection([0.0, 1.0, 0.0]),
        )
        .unwrap();
        assert!(exact.cell_round_trip.0 < 1e-6 && exact.local_round_trip.0 < 1e-6);
        assert!(exact.direction_error.0 < 1e-6);
        let wrong = FrozenConversion::from_encoding(
            &frame,
            &local,
            cell,
            LocalPos([1.0, 0.0, 0.0]),
            heading,
            LocalDirection([1.0, 0.0, 0.0]),
        )
        .unwrap();
        assert!((wrong.cell_round_trip.0 - 1.0).abs() < 1e-6);
        assert!((wrong.direction_error.0 - 90.0).abs() < 1e-9);
        assert!(direction_error(CellDirection([0.0; 3]), heading).is_none());
        assert!(direction_error(CellDirection([f64::NAN; 3]), heading).is_none());
    }
}
