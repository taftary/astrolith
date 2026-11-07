//! Generic identity and sampled geometry of a real production drawn surface.
//!
//! A query result is not a contact flag. Neither a nearby moon nor a reference
//! sphere constructs this record; only the production ground supplies geometry.

use crate::body::Ground;
use crate::coords::OpenUnits;
use crate::frame_diagnostic::CellDirection;
use crate::moon_catalog::{MoonIdentity, PlanetIdentity};

/// Permanent identity of the body owning the queried production ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceIdentity {
    /// An existing planet's drawn ground.
    Planet(PlanetIdentity),
    /// An existing moon's drawn ground, only when production supplies it.
    Moon(MoonIdentity),
}

impl SurfaceIdentity {
    /// Containing planet/cell identity, independently of the ground owner kind.
    #[must_use]
    pub const fn host(self) -> PlanetIdentity {
        match self {
            Self::Planet(host) => host,
            Self::Moon(moon) => moon.planet(),
        }
    }
}

/// Signed feet clearance along the actual production up, in metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawnClearanceMetres(pub f64);

/// One query against the ground actually used by the production body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundObservation {
    /// Actual surface owner, not the requested fixture or closest marker.
    pub identity: SurfaceIdentity,
    /// Queried feet, in open-cell units.
    pub feet: OpenUnits,
    /// Point on the production drawn ground, in open-cell units.
    pub point: OpenUnits,
    /// Production ground up; this can differ from a triangle's geometric normal.
    pub up: CellDirection,
    /// Signed difference along up, not distance to a reference sphere.
    pub clearance: DrawnClearanceMetres,
}

impl GroundObservation {
    /// Validates and labels an actual ground query; does not infer contact.
    #[must_use]
    pub fn from_ground(
        identity: SurfaceIdentity,
        ground: Ground,
        feet: OpenUnits,
        metres_per_cell: f64,
    ) -> Option<Self> {
        if !metres_per_cell.is_finite()
            || metres_per_cell <= 0.0
            || !feet
                .0
                .iter()
                .chain(&ground.point)
                .chain(&ground.up)
                .all(|v| v.is_finite())
        {
            return None;
        }
        let [x, y, z] = feet.0;
        let [gx, gy, gz] = ground.point;
        let [ux, uy, uz] = ground.up;
        let length = ux.hypot(uy).hypot(uz);
        if length <= 0.0 || !length.is_finite() {
            return None;
        }
        let gap = ((x - gx) * ux + (y - gy) * uy + (z - gz) * uz) * metres_per_cell;
        gap.is_finite().then_some(Self {
            identity,
            feet,
            point: OpenUnits(ground.point),
            up: CellDirection(ground.up),
            clearance: DrawnClearanceMetres(gap),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_query_labels_actual_geometry_without_inventing_contact() {
        let host =
            PlanetIdentity::new(crate::nav::Seed(42), &crate::nest::autopilot_path(42)).unwrap();
        let ground = Ground {
            point: [1.0, 0.0, 0.0],
            up: [1.0, 0.0, 0.0],
        };
        let observed = GroundObservation::from_ground(
            SurfaceIdentity::Planet(host),
            ground,
            OpenUnits([1.25, 0.0, 0.0]),
            4.0,
        )
        .unwrap();
        assert_eq!(observed.clearance, DrawnClearanceMetres(1.0));
        assert_eq!(observed.identity, SurfaceIdentity::Planet(host));
        assert!(
            GroundObservation::from_ground(
                SurfaceIdentity::Planet(host),
                ground,
                OpenUnits([f64::NAN; 3]),
                4.0
            )
            .is_none()
        );
        assert!(
            GroundObservation::from_ground(
                SurfaceIdentity::Planet(host),
                ground,
                OpenUnits([1.0; 3]),
                0.0
            )
            .is_none()
        );
    }
}
