//! Validated moon-local fixture directions, gaps and shared times (#1238).
//!
//! These are preparation data, not ground support, player travel or a new moon
//! coordinate simulation. Production state supplies the frame and moving centre.

/// Malformed preparation geometry or time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApproachError {
    /// Direction has non-finite components or no finite nonzero length.
    Direction,
    /// A geometric approach gap must be finite and nonnegative.
    Gap,
    /// Shared time must be finite and nonnegative.
    Time,
    /// Look angles are not finite or pitch exceeds the existing normal-mode limit.
    Look,
    /// The supplied permanent identity does not match the current open cell.
    Context,
    /// The current production moon marker or planet geometry is absent.
    Geometry,
}

impl std::fmt::Display for ApproachError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Direction => "approach direction needs a finite nonzero length",
            Self::Gap => "approach gap needs finite nonnegative metres",
            Self::Time => "shared time needs finite nonnegative seconds",
            Self::Look => "look needs finite degrees and pitch within the normal-mode limit",
            Self::Context => "approach identity does not match the open production cell",
            Self::Geometry => "approach production geometry is absent",
        })
    }
}

impl std::error::Error for ApproachError {}

/// Unit direction in a named body's production local frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceDirection([f64; 3]);

impl SurfaceDirection {
    /// Normalizes finite nonzero input; a direction is not a world position.
    ///
    /// # Errors
    /// Refuses non-finite values and zero/overflowing lengths.
    pub fn new(value: [f64; 3]) -> Result<Self, ApproachError> {
        let length = value.iter().map(|v| v * v).sum::<f64>().sqrt();
        if !value.iter().all(|v| v.is_finite()) || !length.is_finite() || length <= 0.0 {
            return Err(ApproachError::Direction);
        }
        Ok(Self(value.map(|v| v / length)))
    }

    /// Unit components, still requiring the body's named production frame.
    #[must_use]
    pub const fn components(self) -> [f64; 3] {
        self.0
    }
}

/// Nonnegative geometric offset from a reference sphere, not collision clearance.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct ReferenceGapMetres(f64);

impl ReferenceGapMetres {
    /// Validates a finite nonnegative geometric gap.
    ///
    /// # Errors
    /// Refuses a negative or non-finite offset.
    pub fn new(value: f64) -> Result<Self, ApproachError> {
        if value.is_finite() && value >= 0.0 {
            Ok(Self(value))
        } else {
            Err(ApproachError::Gap)
        }
    }

    /// The reference-sphere offset, in metres.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Shared sky time used once to prepare a fixture, in game seconds.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct FixtureTimeSeconds(f64);

impl FixtureTimeSeconds {
    /// Validates a finite nonnegative shared time.
    ///
    /// # Errors
    /// Refuses negative or non-finite game seconds.
    pub fn new(value: f64) -> Result<Self, ApproachError> {
        if value.is_finite() && value >= 0.0 {
            Ok(Self(value))
        } else {
            Err(ApproachError::Time)
        }
    }

    /// The shared game time in seconds, not elapsed wall time.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Validated initial look in the production heading basis, stored in radians.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FixtureLook {
    yaw_rad: f64,
    pitch_rad: f64,
}

impl FixtureLook {
    /// Accepts finite degrees, with the existing normal-camera pitch bounds.
    ///
    /// # Errors
    /// Refuses non-finite/overflowing angles and out-of-range pitch.
    pub fn from_degrees(yaw: f64, pitch: f64) -> Result<Self, ApproachError> {
        let (yaw_rad, pitch_rad) = (yaw.to_radians(), pitch.to_radians());
        if !yaw_rad.is_finite()
            || !pitch_rad.is_finite()
            || pitch_rad.abs() > crate::flight::FREE_PITCH_LIMIT
        {
            return Err(ApproachError::Look);
        }
        Ok(Self { yaw_rad, pitch_rad })
    }

    /// Initial production yaw, radians around the actual local up.
    #[must_use]
    pub const fn yaw_rad(self) -> f64 {
        self.yaw_rad
    }

    /// Initial production look pitch in radians.
    #[must_use]
    pub const fn pitch_rad(self) -> f64 {
        self.pitch_rad
    }

    /// Look direction in the current production heading basis, not a moon-up fix.
    #[must_use]
    pub fn forward(self, up: [f64; 3]) -> [f64; 3] {
        let [ax, ay, az] = crate::body::heading_basis(up, self.yaw_rad).0;
        let [ux, uy, uz] = up;
        let (sin, cos) = self.pitch_rad.sin_cos();
        [
            ax * cos + ux * sin,
            ay * cos + uy * sin,
            az * cos + uz * sin,
        ]
    }
}

fn same_context(
    universe: &crate::nav::Universe,
    host: crate::moon_catalog::PlanetIdentity,
) -> bool {
    universe.root == host.seed() && universe.path.indices() == host.chain()
}

/// A moon-local approach in the production proxy's untranslated host-cell axes.
/// It uses the actual moving centre at `universe.time_s()`, never supplies ground.
///
/// # Errors
/// Refuses a different open seed/planet or an absent production moon marker.
pub fn moon_reference_position(
    universe: &crate::nav::Universe,
    moon: crate::moon_catalog::MoonIdentity,
    direction: SurfaceDirection,
    gap: ReferenceGapMetres,
) -> Result<crate::coords::OpenUnits, ApproachError> {
    if !same_context(universe, moon.planet()) {
        return Err(ApproachError::Context);
    }
    let point = universe
        .open
        .point_by_id(moon.marker().0)
        .filter(|p| p.moon.is_some())
        .ok_or(ApproachError::Geometry)?;
    let radius = point.radius + gap.get() * crate::body::metre(universe.level());
    let [dx, dy, dz] = direction.components();
    let [x, y, z] = point.position;
    let position = [x + radius * dx, y + radius * dy, z + radius * dz];
    if !position.iter().all(|v| v.is_finite()) {
        return Err(ApproachError::Geometry);
    }
    Ok(crate::coords::OpenUnits(position))
}

/// Initial planet position above its production relief. This is setup geometry,
/// not a collision measurement: production drawn-ground settling still follows.
///
/// # Errors
/// Refuses a different context or absent production frame/relief.
pub fn planet_relief_position(
    universe: &crate::nav::Universe,
    host: crate::moon_catalog::PlanetIdentity,
    direction: SurfaceDirection,
    gap: ReferenceGapMetres,
) -> Result<crate::coords::OpenUnits, ApproachError> {
    if !same_context(universe, host) {
        return Err(ApproachError::Context);
    }
    let frame = crate::nav::planet_frame(universe).ok_or(ApproachError::Geometry)?;
    let relief = crate::nav::planet_relief(universe).ok_or(ApproachError::Geometry)?;
    let direction = direction.components();
    let radius = frame.radius_m() + relief.height_m(direction) + gap.get();
    Ok(crate::coords::OpenUnits(frame.to_cell(
        crate::planet_frame::PlanetPos(direction.map(|v| v * radius)),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approach_data_normalizes_directions_but_never_invents_ground() {
        assert_eq!(
            SurfaceDirection::new([2.0, 0.0, 0.0]).unwrap().components(),
            [1.0, 0.0, 0.0]
        );
        for bad in [[0.0; 3], [f64::NAN, 1.0, 0.0], [f64::MAX; 3]] {
            assert!(SurfaceDirection::new(bad).is_err());
        }
        assert_eq!(ReferenceGapMetres::new(10.0).unwrap().get(), 10.0);
        assert_eq!(FixtureTimeSeconds::new(86_400.0).unwrap().get(), 86_400.0);
        for bad in [-1.0, f64::NAN, f64::INFINITY] {
            assert!(ReferenceGapMetres::new(bad).is_err());
            assert!(FixtureTimeSeconds::new(bad).is_err());
        }
    }

    #[test]
    fn moon_approaches_use_actual_timed_centres_and_keep_seam_pole_axes() {
        use crate::moon_catalog::{PlanetIdentity, moons_for_planet};
        let host =
            PlanetIdentity::new(crate::nav::Seed(42), &crate::nest::autopilot_path(42)).unwrap();
        let moon = moons_for_planet(host).unwrap()[0];
        let mut universe = crate::nav::Universe::new(42);
        for &id in host.chain() {
            assert!(universe.open(crate::nav::MarkerIndex(id)));
        }
        let gap = ReferenceGapMetres::new(10.0).unwrap();
        for time in [0.0, 100_000.0] {
            universe.advance(time);
            for dir in [[1.0, 1.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 1.0]] {
                let direction = SurfaceDirection::new(dir).unwrap();
                let position =
                    moon_reference_position(&universe, moon.identity, direction, gap).unwrap();
                let point = universe.open.point_by_id(moon.identity.marker().0).unwrap();
                let radius = crate::nav::length3(crate::nav::sub3(position.0, point.position));
                let actual_gap = (radius - point.radius) / crate::body::metre(universe.level());
                assert!((actual_gap - 10.0).abs() < 1e-6);
                assert_eq!(
                    position,
                    moon_reference_position(&universe, moon.identity, direction, gap).unwrap()
                );
            }
        }
        assert!(
            moon_reference_position(
                &crate::nav::Universe::new(99),
                moon.identity,
                SurfaceDirection::new([1.0, 0.0, 0.0]).unwrap(),
                gap
            )
            .is_err()
        );
    }

    #[test]
    fn look_degrees_are_checked_and_use_the_existing_heading_basis() {
        let look = FixtureLook::from_degrees(90.0, 0.0).unwrap();
        assert_eq!(
            look.forward([0.0, 1.0, 0.0]),
            crate::body::heading_basis([0.0, 1.0, 0.0], look.yaw_rad()).0
        );
        assert!(FixtureLook::from_degrees(0.0, 91.0).is_err());
        assert!(FixtureLook::from_degrees(f64::INFINITY, 0.0).is_err());
        assert!(FixtureLook::from_degrees(0.0, f64::NAN).is_err());
    }
}
