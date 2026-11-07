//! Unit-tagged view geometry from actual production sky positions (#1238).
//! This is a read-only description, not a second orbit model or visibility test.

use crate::coords::OpenUnits;
use crate::frame_diagnostic::CellDirection;

/// Production reference-sphere geometry viewed from the recorded camera.
/// The caller records whether the radius is physical or a display proxy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyGeometry {
    /// Actual current centre, in open-cell units.
    pub centre: OpenUnits,
    /// Actual current observer, not the fixture's requested approach.
    pub observer: OpenUnits,
    /// Camera-to-centre unit direction in that cell, absent at the exact centre.
    pub toward: Option<CellDirection>,
    /// Actual production reference/proxy radius in metres, not a catalog claim.
    pub radius_m: f64,
    /// Camera-to-centre distance in metres.
    pub distance_m: f64,
    /// Reference-sphere angular diameter in degrees, before marker-size floors.
    pub angular_diameter_deg: f64,
}

impl SkyGeometry {
    /// Labels existing positions/size using the production angular-size rule.
    #[must_use]
    pub fn new(
        centre: OpenUnits,
        observer: OpenUnits,
        radius_cells: f64,
        metres_per_cell: f64,
    ) -> Option<Self> {
        if !centre.0.iter().chain(&observer.0).all(|v| v.is_finite())
            || !radius_cells.is_finite()
            || radius_cells <= 0.0
            || !metres_per_cell.is_finite()
            || metres_per_cell <= 0.0
        {
            return None;
        }
        let [x, y, z] = centre.0;
        let [cx, cy, cz] = observer.0;
        let delta = [x - cx, y - cy, z - cz];
        let distance = crate::nav::length3(delta);
        let radius_m = radius_cells * metres_per_cell;
        let distance_m = distance * metres_per_cell;
        let angular_diameter_deg =
            2.0 * crate::frame::angular_radius(radius_cells, distance).to_degrees();
        if !radius_m.is_finite() || !distance_m.is_finite() || !angular_diameter_deg.is_finite() {
            return None;
        }
        Some(Self {
            centre,
            observer,
            toward: (distance > 0.0).then(|| CellDirection(delta.map(|v| v / distance))),
            radius_m,
            distance_m,
            angular_diameter_deg,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observed_geometry_is_relative_to_the_real_camera_not_the_requested_pose() {
        let centre = OpenUnits([3.0, 0.0, 0.0]);
        let first = SkyGeometry::new(centre, OpenUnits([0.0; 3]), 1.0, 10.0).unwrap();
        let next = SkyGeometry::new(centre, OpenUnits([1.0, 0.0, 0.0]), 1.0, 10.0).unwrap();
        assert_eq!((first.distance_m, next.distance_m), (30.0, 20.0));
        assert!(next.angular_diameter_deg > first.angular_diameter_deg);
        assert_eq!(next.toward, Some(CellDirection([1.0, 0.0, 0.0])));
        assert!(SkyGeometry::new(centre, OpenUnits([f64::NAN; 3]), 1.0, 10.0).is_none());
        assert!(
            SkyGeometry::new(centre, centre, 1.0, 10.0)
                .unwrap()
                .toward
                .is_none()
        );
    }
}
