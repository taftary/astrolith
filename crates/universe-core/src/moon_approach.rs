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
}

impl std::fmt::Display for ApproachError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Direction => "approach direction needs a finite nonzero length",
            Self::Gap => "approach gap needs finite nonnegative metres",
            Self::Time => "shared time needs finite nonnegative seconds",
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
}
