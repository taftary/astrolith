//! Owner-qualified recipes for the existing shared terrain pipeline (#1215).
//!
//! Construction may regenerate immutable catalog data once on owner changes.
//! Sampling and frame updates do not regenerate the universe. A recipe is not
//! contact: only production drawn geometry can supply a supporting surface.

use crate::moon_catalog::{CatalogError, ListedMoon, moon_record};
use crate::relief::Relief;
use crate::surface_frame::SurfaceRadiusMetres;
use crate::surface_observation::SurfaceIdentity;
use crate::world_type::WorldType;

/// Immutable geometry identity, to be combined with patch and quality epoch.
/// Radius bits are canonical: construction accepts positive finite metres only,
/// so neither NaN nor signed zero can enter equality/hash (`E-FLOAT-SORT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceRecipeKey {
    owner: SurfaceIdentity,
    seed: u64,
    kind: WorldType,
    radius_bits: u64,
    revision: u32,
}

impl SurfaceRecipeKey {
    /// Permanent geometry owner, not just a cell seed or a patch coordinate.
    #[must_use]
    pub const fn owner(self) -> SurfaceIdentity {
        self.owner
    }
}

/// Resolved physical/type recipe without a new universe population or UI.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceRecipe {
    key: SurfaceRecipeKey,
    relief: Relief,
    moon: Option<ListedMoon>,
}

impl SurfaceRecipe {
    /// Resolves an existing permanent identity. Keep the result across frames;
    /// this constructor is preparation, not travel or a per-frame hot path.
    ///
    /// # Errors
    /// Refuses missing/inconsistent generated physical/type records.
    pub fn for_owner(owner: SurfaceIdentity) -> Result<Self, CatalogError> {
        let (relief, kind, seed, moon) = match owner {
            SurfaceIdentity::Planet(host) => {
                let kind = crate::world_type::planet_type(host.seed().0, host.chain())
                    .ok_or(CatalogError::MissingPlanet)?;
                let relief = Relief::for_planet(host.seed().0, host.chain());
                let seed = relief.seed();
                (relief, kind, seed, None)
            }
            SurfaceIdentity::Moon(identity) => {
                let record = moon_record(identity)?;
                let radius = SurfaceRadiusMetres::new(record.info.radius_km * 1_000.0)
                    .ok_or(CatalogError::MissingMoon)?;
                let relief = Relief::for_moon(record.paint_seed, record.kind, radius)
                    .ok_or(CatalogError::MissingMoon)?;
                (relief, record.kind, record.paint_seed, Some(record))
            }
        };
        let radius =
            SurfaceRadiusMetres::new(relief.radius_m()).ok_or(CatalogError::MissingPlanet)?;
        Ok(Self {
            key: SurfaceRecipeKey {
                owner,
                seed,
                kind,
                radius_bits: radius.get().to_bits(),
                revision: 1,
            },
            relief,
            moon,
        })
    }

    /// Geometry-qualified identity; patch coordinates alone cannot key workers.
    #[must_use]
    pub const fn key(&self) -> SurfaceRecipeKey {
        self.key
    }

    /// Existing shared-grid sampling contract, with unmodified planet behavior.
    #[must_use]
    pub const fn relief(&self) -> &Relief {
        &self.relief
    }

    /// Production type, including ice/lava/iron on actual generated moons.
    #[must_use]
    pub const fn kind(&self) -> WorldType {
        self.key.kind
    }

    /// Separate evidence, instance origin and supplied/generated properties;
    /// absent for a planet, never manufactured from fixture placement.
    #[must_use]
    pub const fn moon_record(&self) -> Option<&ListedMoon> {
        self.moon.as_ref()
    }
}

#[cfg(test)]
#[path = "surface_recipe_tests.rs"]
mod tests;
