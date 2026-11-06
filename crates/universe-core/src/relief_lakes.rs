//! Lake basins on the land of Earth-like and ice worlds (#1162 T3, Q5 A,
//! Q8 A).
//!
//! Split from `relief` (`E-FILE-SIZE`). Dry worlds never read this module,
//! and outside each basin the terrain keeps every bit (criterion 7).
//!
//! - **Placement:** one lattice of cells over the unit direction, as the
//!   #1161 craters. A cell holds at most one lake, by its hash. Its centre
//!   is a jittered point of the cell projected onto the sphere, kept only
//!   when the projection stays inside the cell. A lake's reach is at most a
//!   quarter cell, so each point checks the 2 x 2 x 2 nearest cells: 8
//!   hashes, no allocation (`E-HOT-NOALLOC`), and no lake is ever cut at a
//!   cell face.
//! - **Kinds:** round crater lakes (1.5 to 4 km), round calderas (4 to 9
//!   km, flat floor and steep walls) and long valley lakes (8 to 18 km by
//!   1.2 to 2.5 km), all scaled by the relief scale in depth.
//! - **Level:** from the terrain at the centre and at 24 points on the rim,
//!   so the water sits below the lowest of them. Lakes are kept only on
//!   land well above the sea and not on steep slopes. Full lakes fill to
//!   just under that; partly filled ones stand lower, with a pale dry ring
//!   up to the full line.
//! - **Shape:** inside the basin the terrain is drawn toward a bowl under
//!   the full line, blending back into the terrain toward the rim; at and
//!   beyond the rim the terrain is untouched.
//!
//! Pure functions of the unit direction and the seed, so heights match on
//! both sides of every cube edge and at every detail level
//! (`E-DET-TIERS`).

use crate::seed::hash_triple;

/// Lattice cell size in unit-direction units (about 127 km on Earth).
pub const LAKE_CELL: f64 = 0.02;

/// Share of cells that try a lake.
pub const LAKE_FILL: f64 = 0.05;

/// Longest lake reach as a share of the cell (stays inside half a cell).
const REACH_MAX: f64 = 0.25;

/// Lowest centre height for a lake, in metres at Earth gravity.
pub const LAKE_LAND_M: f64 = 120.0;

/// Gap between the full line and the lowest rim point, metres at Earth
/// gravity.
const MARGIN_M: f64 = 6.0;

/// How far the pale ring reaches over the full line, metres at Earth
/// gravity.
const SHORE_M: f64 = 1.5;

/// Points on the rim the level is read from.
const RIM_POINTS: usize = 24;

/// Seed tag of the lake lattice.
const TAG_LAKE: u64 = 0x1162_0001;

/// What a lake basin looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LakeKind {
    /// Round bowl.
    Crater,
    /// Larger round basin with a flat floor and steep walls.
    Caldera,
    /// Long, narrow basin.
    Valley,
}

/// One lake candidate from its lattice cell, before the terrain is read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LakeSeed {
    /// Lattice cell key (for the sampler's memo).
    pub key: u64,
    /// Unit direction of the centre.
    pub centre: [f64; 3],
    /// Along-basin unit tangent at the centre.
    pub along: [f64; 3],
    /// Across-basin unit tangent at the centre.
    pub across: [f64; 3],
    /// Semi-axes along and across, in unit-direction units.
    pub semi: [f64; 2],
    /// Water depth at the centre of a full lake, metres (scaled).
    pub depth_m: f64,
    /// Share of the depth a partly filled lake stands below its full line
    /// (`0` for a full lake).
    pub drop: f64,
    /// Kind of basin.
    pub kind: LakeKind,
}

impl LakeSeed {
    /// Basin radius at unit direction `unit`: `0` at the centre, `1` on
    /// the rim (an ellipse in the tangent plane, the normal part added so
    /// far points never read as near).
    #[must_use]
    pub fn radius(&self, unit: [f64; 3]) -> f64 {
        let d = [
            unit[0] - self.centre[0],
            unit[1] - self.centre[1],
            unit[2] - self.centre[2],
        ];
        let x = dot(d, self.along) / self.semi[0];
        let y = dot(d, self.across) / self.semi[1];
        let z = dot(d, self.centre) / self.semi[1];
        (x * x + y * y + z * z).sqrt()
    }

    /// The `i`-th of the rim points the level is read from.
    #[must_use]
    pub fn rim_point(&self, i: usize) -> [f64; 3] {
        #[expect(clippy::cast_precision_loss, reason = "E-CAST: a dozen rim points")]
        let a = std::f64::consts::TAU * i as f64 / RIM_POINTS as f64;
        let (s, c) = a.sin_cos();
        let p = [
            self.centre[0] + c * self.semi[0] * self.along[0] + s * self.semi[1] * self.across[0],
            self.centre[1] + c * self.semi[0] * self.along[1] + s * self.semi[1] * self.across[1],
            self.centre[2] + c * self.semi[0] * self.along[2] + s * self.semi[1] * self.across[2],
        ];
        normalise(p).unwrap_or(self.centre)
    }

    /// The basin's bowl below its full line at radius `r`, as a share of
    /// the depth: `1` at the centre, `0` at the rim, with no step.
    #[must_use]
    pub fn bowl(&self, r: f64) -> f64 {
        if r >= 1.0 {
            return 0.0;
        }
        match self.kind {
            LakeKind::Caldera => 1.0 - smoothstep(0.55, 0.9, r),
            LakeKind::Crater | LakeKind::Valley => {
                let t = 1.0 - r * r;
                t * t
            }
        }
    }
}

/// A lake whose levels are known (its terrain read).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lake {
    /// The candidate it came from.
    pub seed: LakeSeed,
    /// Full line in metres: below the centre and every rim point.
    pub full_m: f64,
    /// Water level in metres (the full line, or lower when partly filled).
    pub level_m: f64,
}

impl Lake {
    /// The lake of `seed` given the terrain at its centre and the lowest
    /// terrain on its rim, `None` when it is not kept (too low, or on a
    /// steep slope). `scale` is the relief scale.
    #[must_use]
    pub fn settle(seed: LakeSeed, centre_m: f64, rim_min_m: f64, scale: f64) -> Option<Lake> {
        if !(centre_m.is_finite() && rim_min_m.is_finite()) || centre_m < LAKE_LAND_M * scale {
            return None;
        }
        if rim_min_m < centre_m - seed.depth_m.max(40.0 * scale) {
            return None;
        }
        let full_m = centre_m.min(rim_min_m) - MARGIN_M * scale;
        let level_m = full_m - seed.drop * seed.depth_m;
        Some(Lake {
            seed,
            full_m,
            level_m,
        })
    }

    /// The terrain inside the basin at radius `r` (`r < 1`) over the
    /// original terrain `base_m`: drawn toward the bowl, blending back into
    /// the terrain from 0.6 radii to the rim.
    #[must_use]
    pub fn carve_m(&self, r: f64, base_m: f64) -> f64 {
        let bowl = self.full_m - self.seed.depth_m * self.seed.bowl(r);
        let keep = smoothstep(0.6, 1.0, r);
        bowl + (base_m - bowl) * keep
    }

    /// The water level at radius `r`: flat, falling away only in the last
    /// twentieth of the radius so no water ever stands as a wall at the
    /// rim.
    #[must_use]
    pub fn level_at(&self, r: f64) -> f64 {
        self.level_m - 1.0e4 * smoothstep(0.95, 1.0, r)
    }

    /// Whether terrain at `terrain_m` (inside the basin, above the water)
    /// lies in the pale ring of a partly filled lake, up to just over its
    /// full line. Full lakes have none.
    #[must_use]
    pub fn in_ring(&self, terrain_m: f64, scale: f64) -> bool {
        self.seed.drop > 0.0
            && terrain_m >= self.level_m
            && terrain_m < self.full_m + SHORE_M * scale
    }
}

/// The lake layer of one wet world: its seed, size and relief scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lakes {
    /// Seed of the lake lattice.
    seed: u64,
    /// Planet radius in metres.
    radius_m: f64,
    /// Relief scale (depths).
    scale: f64,
}

impl Lakes {
    /// The lake layer of a world of `radius_m` metres under `seed`.
    #[must_use]
    pub fn new(seed: u64, radius_m: f64, scale: f64) -> Lakes {
        Lakes {
            seed: hash_triple(seed, TAG_LAKE, 0),
            radius_m,
            scale,
        }
    }

    /// The relief scale the depths use.
    #[must_use]
    pub fn scale(&self) -> f64 {
        self.scale
    }

    /// The candidates whose reach can cover `unit`, from the 2 x 2 x 2
    /// nearest cells (at most 8; `None` slots are empty cells).
    #[must_use]
    pub fn near(&self, unit: [f64; 3]) -> [Option<LakeSeed>; 8] {
        let [sx, sy, sz] = unit.map(|c| c / LAKE_CELL);
        let (bx, by, bz) = (sx.floor(), sy.floor(), sz.floor());
        let near = |s: f64, b: f64| if s - b < 0.5 { b - 1.0 } else { b + 1.0 };
        let (nx, ny, nz) = (near(sx, bx), near(sy, by), near(sz, bz));
        let mut out = [None; 8];
        let mut i = 0;
        for x in [bx, nx] {
            for y in [by, ny] {
                for z in [bz, nz] {
                    if let Some(slot) = out.get_mut(i) {
                        *slot = self.seed_at([x, y, z]);
                    }
                    i += 1;
                }
            }
        }
        out
    }

    /// Every candidate on the sphere, in lattice order (listings and
    /// tests; allocates, never on the height path).
    #[must_use]
    pub fn seeds(&self) -> Vec<LakeSeed> {
        let mut out = Vec::new();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: about 50 cells per unit, a small integer"
        )]
        let reach = (1.0 / LAKE_CELL).ceil() as i64 + 1;
        for x in -reach..reach {
            for y in -reach..reach {
                for z in -reach..reach {
                    #[expect(clippy::cast_precision_loss, reason = "E-CAST: small integers")]
                    let c = [x as f64, y as f64, z as f64];
                    // Nearest and farthest points of the cell from the centre.
                    let mut near = 0.0;
                    let mut far = 0.0;
                    for v in c {
                        let (a, b) = (v * LAKE_CELL, (v + 1.0) * LAKE_CELL);
                        let lo = if a > 0.0 {
                            a
                        } else if b < 0.0 {
                            -b
                        } else {
                            0.0
                        };
                        near += lo * lo;
                        far += a.abs().max(b.abs()).powi(2);
                    }
                    if near <= 1.0
                        && far >= 1.0
                        && let Some(seed) = self.seed_at(c)
                    {
                        out.push(seed);
                    }
                }
            }
        }
        out
    }

    /// The candidate of lattice cell `c`, if it holds one.
    #[must_use]
    pub fn seed_at(&self, c: [f64; 3]) -> Option<LakeSeed> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: lattice cells are small integers stored in f64"
        )]
        let key3 = c.map(|v| (v as i64).cast_unsigned());
        // One cheap mix first: 95% of cells hold no lake and stop here
        // (#1162 T7 speed: a height read checks 8 cells).
        let gate = fill_mix(
            self.seed
                ^ key3[0].wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ key3[1].wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
                ^ key3[2].wrapping_mul(0x1656_67B1_9E37_79F9),
        );
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: 16 hash bits are exactly representable in f64"
        )]
        let fill = ((gate >> 48) & 0xFFFF) as f64 / 65_536.0;
        if fill >= LAKE_FILL {
            return None;
        }
        let key = hash_triple(gate, key3[2], 0);
        let h2 = hash_triple(key, 1, 0);
        let u = |h: u64, shift: u32| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: 16 hash bits are exactly representable in f64"
            )]
            let bits = ((h >> shift) & 0xFFFF) as f64;
            bits / 65_536.0
        };
        let jitter = [
            (c[0] + u(key, 16)) * LAKE_CELL,
            (c[1] + u(key, 32)) * LAKE_CELL,
            (c[2] + u(key, 48)) * LAKE_CELL,
        ];
        let centre = normalise(jitter)?;
        // Kept only when the centre's own cell is this one.
        let own = centre.map(|v| (v / LAKE_CELL).floor());
        if own != c {
            return None;
        }
        let pick = u(h2, 0);
        let kind = if pick < 0.6 {
            LakeKind::Crater
        } else if pick < 0.85 {
            LakeKind::Caldera
        } else {
            LakeKind::Valley
        };
        let (a_m, b_m, depth_m) = match kind {
            LakeKind::Crater => {
                let a = 1_500.0 + 2_500.0 * u(h2, 16);
                (a, a, 30.0 + 60.0 * u(h2, 32))
            }
            LakeKind::Caldera => {
                let a = 4_000.0 + 5_000.0 * u(h2, 16);
                (a, a, 100.0 + 200.0 * u(h2, 32))
            }
            LakeKind::Valley => (
                8_000.0 + 10_000.0 * u(h2, 16),
                1_200.0 + 1_300.0 * u(h2, 48),
                25.0 + 55.0 * u(h2, 32),
            ),
        };
        let cap = REACH_MAX * LAKE_CELL;
        let semi = [
            (a_m / self.radius_m).min(cap),
            (b_m / self.radius_m).min(cap),
        ];
        let partly = u(h2, 8) < 0.4;
        let drop = if partly { 0.25 + 0.25 * u(key, 8) } else { 0.0 };
        let (along, across) = tangents(centre, std::f64::consts::PI * u(key, 24));
        Some(LakeSeed {
            key,
            centre,
            along,
            across,
            semi,
            depth_m: depth_m * self.scale,
            drop,
            kind,
        })
    }
}

/// A 64-bit finaliser (splitmix64's), for the lattice's fill gate.
const fn fill_mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Unit tangents at unit `n`, turned by `angle` about it.
fn tangents(n: [f64; 3], angle: f64) -> ([f64; 3], [f64; 3]) {
    let helper = if n[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let e1 = normalise(cross(helper, n)).unwrap_or([1.0, 0.0, 0.0]);
    let e2 = cross(n, e1);
    let (s, c) = angle.sin_cos();
    let along = [
        c * e1[0] + s * e2[0],
        c * e1[1] + s * e2[1],
        c * e1[2] + s * e2[2],
    ];
    (along, cross(n, along))
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalise(v: [f64; 3]) -> Option<[f64; 3]> {
    let len = dot(v, v).sqrt();
    (len.is_finite() && len > 0.0).then(|| [v[0] / len, v[1] / len, v[2] / len])
}

fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How many rim points a lake's level reads.
#[must_use]
pub fn rim_points() -> usize {
    RIM_POINTS
}

#[cfg(test)]
#[path = "relief_lakes_tests.rs"]
mod tests;
