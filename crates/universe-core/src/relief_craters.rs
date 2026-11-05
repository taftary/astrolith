//! Craters as real height on airless worlds (#1161 T3, Spec v2 Q4 A).
//!
//! Split from `relief` (`E-FILE-SIZE`). Worlds that keep air never read
//! this module, so their heights keep every bit (criterion 4).
//!
//! Following the height-maps guide's "Building block 6":
//! - **Placement:** one size class per octave. Each class lays a 3D lattice
//!   of cells over the unit direction; a cell holds at most one crater, by
//!   its hash, with its centre jittered inside the cell; the sphere cuts
//!   each crater's ball of influence, so centres near the surface give the
//!   full crater and others a smaller one. The count per class grows with the surface area, as 4 per
//!   halving of size, which is the saturated `N(>D) ~ D^-2` law.
//! - **Shape:** depth `0.2 D` for simple craters, `1.044 D^0.301` km for
//!   complex ones (with a flat floor), the switch at
//!   `D_tr = 16533.8 g^-1.0303` m. There is a rim of `0.04 D` (at most
//!   half the depth) and ejecta falling as `(R/r)^3` out to two radii.
//! - **Lookup:** a crater's reach (two radii) stays within half a cell, so
//!   each point checks only the 2 x 2 x 2 nearest cells per class: 8
//!   hashes per class, and no allocation (`E-HOT-NOALLOC`).
//!
//! Pure functions of the unit direction and the seed, so the heights match
//! on both sides of every cube edge and at every detail level, and the
//! ground the player stands on is the drawn ground (`E-DET-TIERS`).

use crate::seed::hash_triple;

/// Size classes (octaves), the largest first.
pub const CRATER_CLASSES: usize = 7;

/// Cell size of the largest class in unit-direction units (one cell is a
/// quarter of the radius).
pub const CRATER_CELL_0: f64 = 0.25;

/// Share of cells that hold a crater.
pub const CRATER_FILL: f64 = 0.5;

/// Largest crater radius as a share of its cell (reach of two radii stays
/// inside half a cell).
pub const CRATER_RADIUS_MAX: f64 = 0.25;

/// Smallest crater radius as a share of its cell.
pub const CRATER_RADIUS_MIN: f64 = 0.10;

/// Seed tag of the crater lattice.
const TAG_CRATER: u64 = 0x1161_0003;

/// Crater layer of one airless world: its seed, size and gravity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Craters {
    /// Seed of the crater lattice.
    seed: u64,
    /// Planet radius in metres.
    radius_m: f64,
    /// Simple-to-complex transition diameter in metres.
    transition_m: f64,
}

impl Craters {
    /// The crater layer for a world of `radius_m` metres and surface
    /// gravity `gravity` m/s², under `seed`.
    #[must_use]
    pub fn new(seed: u64, radius_m: f64, gravity: f64) -> Craters {
        let g = if gravity.is_finite() && gravity > 0.0 {
            gravity
        } else {
            9.81
        };
        Craters {
            seed: hash_triple(seed, TAG_CRATER, 0),
            radius_m,
            transition_m: 16_533.8 * g.powf(-1.030_3),
        }
    }

    /// Depth and rim height in metres of a crater of diameter `d_m`.
    #[must_use]
    pub fn depth_rim_m(&self, d_m: f64) -> (f64, f64) {
        let depth = if d_m < self.transition_m {
            0.2 * d_m
        } else {
            1_044.0 * (d_m / 1_000.0).powf(0.301)
        };
        (depth, (0.04 * d_m).min(0.5 * depth))
    }

    /// The deepest and highest the layer can reach in metres, for the
    /// height bounds: every class's largest crater at once.
    #[must_use]
    pub fn bounds_m(&self) -> (f64, f64) {
        let mut low = 0.0;
        let mut high = 0.0;
        let mut cell = CRATER_CELL_0;
        for _ in 0..CRATER_CLASSES {
            let d_m = 2.0 * CRATER_RADIUS_MAX * cell * self.radius_m;
            let (depth, rim) = self.depth_rim_m(d_m);
            low -= depth;
            high += rim;
            cell *= 0.5;
        }
        (low, high)
    }

    /// Height in metres the craters add at unit direction `unit`.
    #[must_use]
    pub fn height_m(&self, unit: [f64; 3]) -> f64 {
        let mut total = 0.0;
        let mut cell = CRATER_CELL_0;
        for class in 0..CRATER_CLASSES {
            total += self.class_height_m(unit, cell, class as u64);
            cell *= 0.5;
        }
        if total.is_finite() { total } else { 0.0 }
    }

    /// The height one size class adds at `unit` (cell size `cell`).
    fn class_height_m(&self, unit: [f64; 3], cell: f64, class: u64) -> f64 {
        let [sx, sy, sz] = unit.map(|c| c / cell);
        let (bx, by, bz) = (sx.floor(), sy.floor(), sz.floor());
        // The nearer neighbour along each axis: the 2 x 2 x 2 block whose
        // cells can reach this point.
        let near = |s: f64, b: f64| if s - b < 0.5 { b - 1.0 } else { b + 1.0 };
        let (nx, ny, nz) = (near(sx, bx), near(sy, by), near(sz, bz));
        let class_seed = hash_triple(self.seed, class, 0);
        let mut total = 0.0;
        for x in [bx, nx] {
            for y in [by, ny] {
                for z in [bz, nz] {
                    total += self.crater_at(unit, [x, y, z], cell, class_seed);
                }
            }
        }
        total
    }

    /// The height the crater of lattice cell `c` (if any) adds at `unit`.
    fn crater_at(&self, unit: [f64; 3], c: [f64; 3], cell: f64, class_seed: u64) -> f64 {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: lattice cells are small integers stored in f64"
        )]
        let key = c.map(|v| (v as i64).cast_unsigned());
        let h = hash_triple(hash_triple(class_seed, key[0], key[1]), key[2], 0);
        let unit_of = |shift: u32| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: 16 hash bits are exactly representable in f64"
            )]
            let bits = ((h >> shift) & 0xFFFF) as f64;
            bits / 65_536.0
        };
        if unit_of(0) >= CRATER_FILL {
            return 0.0;
        }
        let centre = [
            (c[0] + 0.25 + 0.5 * unit_of(16)) * cell,
            (c[1] + 0.25 + 0.5 * unit_of(32)) * cell,
            (c[2] + 0.25 + 0.5 * unit_of(48)) * cell,
        ];
        let share = CRATER_RADIUS_MIN + (CRATER_RADIUS_MAX - CRATER_RADIUS_MIN) * unit_of(8);
        let radius = share * cell;
        // Distance in 3D, not to the centre's projection: a centre off the
        // sphere then shows as a smaller cross-section, and the reach stays
        // inside the 2 x 2 x 2 block, so no crater is ever cut at a cell
        // face (no seams).
        let dx = unit[0] - centre[0];
        let dy = unit[1] - centre[1];
        let dz = unit[2] - centre[2];
        let r = (dx * dx + dy * dy + dz * dz).sqrt() / radius;
        if r >= 2.0 {
            return 0.0;
        }
        let d_m = 2.0 * radius * self.radius_m;
        let (depth, rim) = self.depth_rim_m(d_m);
        profile_m(r, depth, rim, d_m >= self.transition_m)
    }
}

/// The crater profile at `r` radii from the centre: a bowl (with a flat
/// floor inside 0.4 radii for complex craters) up to the rim at `r = 1`,
/// then ejecta falling as `(1/r)^3` to zero at two radii.
#[must_use]
pub fn profile_m(r: f64, depth: f64, rim: f64, complex: bool) -> f64 {
    if r < 1.0 {
        let floor_r = if complex { 0.4 } else { 0.0 };
        let t = ((r - floor_r) / (1.0 - floor_r)).max(0.0);
        -depth + (depth + rim) * t * t
    } else if r < 2.0 {
        let fall = (1.0 / (r * r * r) - 0.125) / (1.0 - 0.125);
        rim * fall
    } else {
        0.0
    }
}

#[cfg(test)]
#[path = "relief_craters_tests.rs"]
mod tests;
