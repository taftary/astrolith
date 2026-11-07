//! Body-centred metre frames for an actual planet or moon (#1215, ADR 0058).
//!
//! The L10 containing cell stays planet-fixed. Moon axes are the existing paint
//! axes translated to the moving centre, with no invented spin/tidal locking.
//! Frames are snapshots: a frozen conversion never advances the shared clock.

use crate::coords::OpenUnits;
use crate::frame_diagnostic::CellDirection;
use crate::moon_approach::SurfaceDirection;
use crate::nav::Universe;
use crate::planet_frame::{PlanetFrame, PlanetPos, metres_per_cell_l10};
use crate::surface_observation::SurfaceIdentity;

/// Positive finite reference radius, in metres, without a moon-size cutoff.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SurfaceRadiusMetres(f64);

impl SurfaceRadiusMetres {
    /// Refuses nonpositive/non-finite radii, not small valid bodies.
    #[must_use]
    pub fn new(metres: f64) -> Option<Self> {
        (metres.is_finite() && metres > 0.0).then_some(Self(metres))
    }

    /// Physical reference radius, not a terrain height or collision reading.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Position relative to a named surface's centre, in f64 metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfacePos {
    owner: SurfaceIdentity,
    metres: [f64; 3],
}

impl SurfacePos {
    /// Permanent owner carried by this encoding, not inferred from proximity.
    #[must_use]
    pub const fn owner(self) -> SurfaceIdentity {
        self.owner
    }

    /// Body-centred components; cannot be treated as containing-cell units.
    #[must_use]
    pub const fn metres(self) -> [f64; 3] {
        self.metres
    }
}

/// Validated production geometry at one shared-clock instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceFrame {
    owner: SurfaceIdentity,
    centre: OpenUnits,
    radius: SurfaceRadiusMetres,
    time_s: f64,
}

impl SurfaceFrame {
    /// Resolves current production geometry only in its actual containing cell.
    /// No regeneration, allocations, navigation changes or clock advance.
    #[must_use]
    pub fn for_owner(universe: &Universe, owner: SurfaceIdentity) -> Option<Self> {
        let host = owner.host();
        if universe.root != host.seed()
            || !universe.path.matches_indices(host.chain())
            || crate::nav::planet_frame(universe).is_none()
        {
            return None;
        }
        let (centre, radius_m) = match owner {
            SurfaceIdentity::Planet(_) => (
                OpenUnits([0.0; 3]),
                crate::nav::planet_frame(universe)?.radius_m(),
            ),
            SurfaceIdentity::Moon(moon) => {
                let point = universe.open.point_by_id(moon.marker().0)?;
                let info = point.moon?;
                (OpenUnits(point.position), info.radius_km * 1_000.0)
            }
        };
        let radius = SurfaceRadiusMetres::new(radius_m)?;
        let time_s = universe.time_s();
        (centre.0.iter().all(|v| v.is_finite()) && time_s.is_finite() && time_s >= 0.0).then_some(
            Self {
                owner,
                centre,
                radius,
                time_s,
            },
        )
    }

    /// The permanent surface identity, never the nearest marker.
    #[must_use]
    pub const fn owner(self) -> SurfaceIdentity {
        self.owner
    }

    /// Actual centre in the containing cell at this snapshot's time.
    #[must_use]
    pub const fn centre(self) -> OpenUnits {
        self.centre
    }

    /// Actual reference radius, independent of surface relief.
    #[must_use]
    pub const fn radius(self) -> SurfaceRadiusMetres {
        self.radius
    }

    /// Shared game seconds at which the production centre was read.
    #[must_use]
    pub const fn time_s(self) -> f64 {
        self.time_s
    }

    /// Validates and tags a body-centred position with this owner.
    #[must_use]
    pub fn position(self, metres: [f64; 3]) -> Option<SurfacePos> {
        metres.iter().all(|v| v.is_finite()).then_some(SurfacePos {
            owner: self.owner,
            metres,
        })
    }

    /// Subtracts the true centre before converting to metres, all in f64.
    #[must_use]
    pub fn from_cell(self, point: OpenUnits) -> Option<SurfacePos> {
        let [x, y, z] = point.0;
        let [cx, cy, cz] = self.centre.0;
        let mpc = metres_per_cell_l10();
        self.position([(x - cx) * mpc, (y - cy) * mpc, (z - cz) * mpc])
    }

    /// Encodes the same owner-local spot in this snapshot's containing cell.
    /// Refuses another body's position rather than silently attaching it here.
    #[must_use]
    pub fn to_cell(self, position: SurfacePos) -> Option<OpenUnits> {
        if position.owner != self.owner {
            return None;
        }
        let [x, y, z] = position.metres;
        let [cx, cy, cz] = self.centre.0;
        let mpc = metres_per_cell_l10();
        let cell = [cx + x / mpc, cy + y / mpc, cz + z / mpc];
        cell.iter()
            .all(|v| v.is_finite())
            .then_some(OpenUnits(cell))
    }

    /// Body-local heading uses exactly the production paint axes, no new spin.
    #[must_use]
    pub fn from_cell_direction(self, direction: CellDirection) -> Option<SurfaceDirection> {
        SurfaceDirection::new(direction.0).ok()
    }

    /// Translation changes no orientation; normalizes only at input validation.
    #[must_use]
    pub fn to_cell_direction(self, direction: SurfaceDirection) -> CellDirection {
        CellDirection(direction.components())
    }

    /// Adapts centred metres to the existing pure patch/tangent-frame routines.
    /// This does not pretend those metres include the moon's cell translation.
    #[must_use]
    pub fn patch_position(self, position: SurfacePos) -> Option<PlanetPos> {
        (position.owner == self.owner).then_some(PlanetPos(position.metres))
    }

    /// The existing patch metric at this body's exact radius, not its centre.
    #[must_use]
    pub fn patch_frame(self) -> PlanetFrame {
        PlanetFrame::for_radius_metres(self.radius)
    }
}

#[cfg(test)]
#[path = "surface_frame_tests.rs"]
mod tests;
