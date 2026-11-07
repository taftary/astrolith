//! Geometry reads the existing paint's private feature records (#1215).

use super::MoonPaint;
use crate::surface_frame::SurfaceRadiusMetres;

impl MoonPaint {
    /// Same normalized-direction field used by production painted maria/ice.
    pub(crate) fn sea_field(&self, unit: [f64; 3]) -> f64 {
        let [x, y, z] = unit;
        let field = |f: f64| {
            crate::noise::value_noise_3d_aperiodic_memo(
                self.sea_seed,
                x * f,
                y * f,
                z * f,
                &mut crate::noise::CellMemo::default(),
            )
        };
        0.65 * field(2.0) + 0.35 * field(4.1)
    }

    /// Continuous bowls/rims at the exact painted centres and chord radii.
    pub(crate) fn crater_relief_m(
        &self,
        unit: [f64; 3],
        radius: SurfaceRadiusMetres,
        gravity_m_s2: f64,
    ) -> f64 {
        let [x, y, z] = unit;
        let mut floor = 0.0f64;
        let mut rim = 0.0f64;
        let gravity_scale = (1.62 / gravity_m_s2).sqrt().clamp(0.5, 2.0);
        for crater in &self.craters {
            let [cx, cy, cz] = crater.centre;
            let r = ((x - cx).powi(2) + (y - cy).powi(2) + (z - cz).powi(2)).sqrt() / crater.radius;
            if r >= 1.8 {
                continue;
            }
            // Shape scales with actual moon gravity but stays bounded by radius
            // on the smallest worlds. Both young and old craters have geometry.
            let depth =
                (crater.radius * radius.get() * 0.04 * gravity_scale).min(0.004 * radius.get());
            let shape = if r < 0.6 {
                -1.0 + 0.15 * crate::moon_surface::smooth(0.0, 0.6, r)
            } else if r < 1.0 {
                -0.85 + 1.2 * crate::moon_surface::smooth(0.6, 1.0, r)
            } else {
                0.35 * (1.0 - crate::moon_surface::smooth(1.0, 1.8, r))
            };
            let height = depth * shape;
            floor = floor.min(height);
            rim = rim.max(height);
        }
        floor + rim
    }

    #[cfg(test)]
    pub(crate) fn shape_centres(&self) -> impl Iterator<Item = [f64; 3]> + '_ {
        self.craters.iter().map(|c| c.centre)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_maria_ice_field_keeps_original_paint_arithmetic_bit_for_bit() {
        for seed in [0, 11, 73, u64::MAX] {
            for kind in [
                crate::world_type::WorldType::Barren,
                crate::world_type::WorldType::Ice,
            ] {
                let paint = MoonPaint::new(kind, seed);
                for input in [[1.0f64, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 1.0]] {
                    // Reference is the pre-extraction production formula.
                    let len =
                        (input[0] * input[0] + input[1] * input[1] + input[2] * input[2]).sqrt();
                    let unit = input.map(|v| v / len);
                    let field = |f: f64| {
                        crate::noise::value_noise_3d_aperiodic_memo(
                            paint.sea_seed,
                            unit[0] * f,
                            unit[1] * f,
                            unit[2] * f,
                            &mut crate::noise::CellMemo::default(),
                        )
                    };
                    let previous = 0.65 * field(2.0) + 0.35 * field(4.1);
                    assert_eq!(paint.sea_field(unit).to_bits(), previous.to_bits());
                }
            }
        }
    }
}
