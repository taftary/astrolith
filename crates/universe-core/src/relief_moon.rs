//! Moon constructors for the shared relief contract, split for E-FILE-SIZE.

use super::Relief;

impl Relief {
    /// Moon geometry through the existing shared-grid sampler. The immutable
    /// feature field is shared across worker clones, not duplicated per patch.
    #[must_use]
    pub fn for_moon(
        seed: u64,
        kind: crate::world_type::WorldType,
        radius: crate::surface_frame::SurfaceRadiusMetres,
    ) -> Option<Self> {
        let moon = crate::moon_surface::MoonSurface::new(kind, seed, radius)?;
        let mut relief = Self::for_recipe(seed, radius.get() / crate::planet_frame::EARTH_RADIUS_M);
        relief.radius_m = radius.get();
        relief.relief_scale = (9.81 / moon.gravity_m_s2()).clamp(0.3, 40.0);
        relief.craters = None;
        relief.is_giant = false;
        relief.moon = Some(std::sync::Arc::new(moon));
        Some(relief)
    }

    /// Shared moon recipe when this is moon ground, never inferred from radius.
    #[must_use]
    pub fn moon_surface(&self) -> Option<&crate::moon_surface::MoonSurface> {
        self.moon.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_clones_share_one_immutable_moon_feature_set() {
        let radius = crate::surface_frame::SurfaceRadiusMetres::new(200_000.0).unwrap();
        let relief = Relief::for_moon(73, crate::world_type::WorldType::Ice, radius).unwrap();
        let cloned = relief.clone();
        assert!(std::sync::Arc::ptr_eq(
            relief.moon.as_ref().unwrap(),
            cloned.moon.as_ref().unwrap()
        ));
    }
}
