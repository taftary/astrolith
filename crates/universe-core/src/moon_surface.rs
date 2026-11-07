//! Synthetic/procedural moon relief sharing the production paint's features.
//!
//! Shape and colour use the same crater centres/radii and ice/maria fields.
//! This is a pure recipe, not a claim of displayed geometry or contact.

use crate::surface_frame::SurfaceRadiusMetres;
use crate::world_type::{MoonPaint, WorldType};

/// A moon's immutable relief/paint; built once per owner, cheap to share by Arc.
#[derive(Debug, Clone, PartialEq)]
pub struct MoonSurface {
    kind: WorldType,
    paint: MoonPaint,
    radius: SurfaceRadiusMetres,
    gravity_m_s2: f64,
}

impl MoonSurface {
    /// Builds shape data for an existing airless moon type and positive radius.
    /// No procedural-domain cutoff: home and boundary recipes use this same path.
    /// This constructor does not assign a fabricated generated identity.
    #[must_use]
    pub fn new(kind: WorldType, seed: u64, radius: SurfaceRadiusMetres) -> Option<Self> {
        if !matches!(
            kind,
            WorldType::Barren | WorldType::Ice | WorldType::Lava | WorldType::Iron
        ) {
            return None;
        }
        let r = radius.get();
        let gravity_m_s2 = crate::gravity::G * crate::gravity::moon_mass_kg(r) / (r * r);
        (gravity_m_s2.is_finite() && gravity_m_s2 > 0.0).then(|| Self {
            kind,
            paint: MoonPaint::new(kind, seed),
            radius,
            gravity_m_s2,
        })
    }

    /// Actual classified material type, without a new moon material taxonomy.
    #[must_use]
    pub const fn kind(&self) -> WorldType {
        self.kind
    }

    /// The existing production paint; distant and close colour sample this field.
    #[must_use]
    pub const fn paint(&self) -> &MoonPaint {
        &self.paint
    }

    /// Reference-surface gravity from the existing moon mass/radius model.
    #[must_use]
    pub const fn gravity_m_s2(&self) -> f64 {
        self.gravity_m_s2
    }

    /// Continuous height in metres above the reference radius. Invalid/zero
    /// directions read as zero, just as planet relief does.
    #[must_use]
    pub fn height_m(&self, direction: [f64; 3]) -> f64 {
        self.sample(direction).map_or(0.0, |(unit, craters)| {
            let field = self.paint.sea_field(unit);
            let strength = if self.kind == WorldType::Barren {
                1.0
            } else {
                0.4
            };
            let basin = smooth(0.46, 0.64, field) * strength;
            // Broad ice/maria domains agree with paint; no planet sea or lakes.
            let base = -self.radius.get() * 0.001 * basin;
            base + craters
        })
    }

    /// Additive bowls and rims at the very same positions as painted craters.
    #[must_use]
    pub fn crater_height_m(&self, direction: [f64; 3]) -> f64 {
        self.sample(direction).map_or(0.0, |(_, height)| height)
    }

    /// Conservative analytic bounds of the entire field, including overlaps.
    #[must_use]
    pub fn height_bounds_m(&self) -> (f64, f64) {
        // Bowl min and rim max are each bounded by 0.004 R. The basin adds
        // at most 0.001 R downward; bowls are selected by min, not summed.
        (-0.005 * self.radius.get(), 0.004 * self.radius.get())
    }

    fn sample(&self, direction: [f64; 3]) -> Option<([f64; 3], f64)> {
        let unit = crate::moon_approach::SurfaceDirection::new(direction)
            .ok()?
            .components();
        let height = self
            .paint
            .crater_relief_m(unit, self.radius, self.gravity_m_s2);
        Some((unit, height))
    }
}

pub(crate) fn smooth(a: f64, b: f64, value: f64) -> f64 {
    let t = ((value - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
