//! L1 holds the observable universe's regions at their real count (#1104,
//! Spec v2 Goal 1).
//!
//! An L1 cell is the observable universe, about 93 billion light years
//! across (ladder R5), and an L2 cell is 1.48e-2 of it, so about 309,030
//! L2-sized regions fill it. They sit on a 68 x 68 x 68 lattice (314,432,
//! within a factor of 2), each jittered a little inside its lattice cell,
//! and every one opens into an L2 cosmic web. They are made in blocks near
//! the camera ([`crate::blocks`]): 17 blocks per axis of 4 x 4 x 4 regions.
//!
//! The eight regions numbered 0 to 7 before #1104 (the octants, #151) stay
//! the fixture: the lattice cells holding the old octant centres keep IDs
//! 0 to 7, so every journey (the autopilot picks among them alone) and
//! every place below them keep their seeds and names. Block IDs start at
//! [`FIXTURE`](crate::regions_l1::FIXTURE). Pure (`E-CORE-NO-BEVY`, `E-DET-TIERS`).

use crate::blocks::{BlockContent, BlockGrid, BlockKey};
use crate::coords::Level;
use crate::r#gen::{Constraints, Form, Generated, Generator, MarkerKind, OctantGenerator, Point};
use crate::nest::marker_radius;
use crate::seed::{Rng, hash_triple};

/// Regions per lattice axis.
pub const PER_AXIS: u32 = 68;

/// Regions per block axis.
pub const BLOCK_SIDE: u32 = 4;

/// IDs of the fixture regions (the old octants) are below this.
pub const FIXTURE: u32 = 8;

/// The block grid of L1: block IDs start after the fixture.
pub const GRID: BlockGrid = BlockGrid {
    per_axis: PER_AXIS / BLOCK_SIDE,
    slots: BLOCK_SIDE * BLOCK_SIDE * BLOCK_SIDE,
    base: FIXTURE,
};

/// Largest offset of a region from its lattice centre, in lattice pitches
/// either way (keeps neighbouring regions from overlapping much).
pub const JITTER: f64 = 0.15;

/// Stream tag of region positions.
const REGION_STREAM_TAG: u64 = 0x1e91_0000_1104;

/// Lattice pitch, in cells.
#[must_use]
pub fn pitch() -> f64 {
    1.0 / f64::from(PER_AXIS)
}

/// Lattice cell holding the old octant `id` (0 to 7, bit `4*x + 2*y + z`).
#[must_use]
pub fn fixture_lattice(id: u32) -> [u32; 3] {
    let at = |bit: u32| lattice_axis(if bit == 0 { -0.25 } else { 0.25 });
    [at((id >> 2) & 1), at((id >> 1) & 1), at(id & 1)]
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "E-CAST: clamped to the lattice before the cast"
)]
fn lattice_axis(value: f64) -> u32 {
    let top = f64::from(PER_AXIS - 1);
    ((value + 0.5) / pitch()).floor().clamp(0.0, top) as u32
}

fn fixture_id(lattice: [u32; 3]) -> Option<u32> {
    (0..FIXTURE).find(|&id| fixture_lattice(id) == lattice)
}

/// Lattice cell of `slot` in block `key`.
#[must_use]
pub fn lattice_of(key: BlockKey, slot: u32) -> [u32; 3] {
    let [bx, by, bz] = key.0;
    let local = [
        slot % BLOCK_SIDE,
        (slot / BLOCK_SIDE) % BLOCK_SIDE,
        slot / (BLOCK_SIDE * BLOCK_SIDE),
    ];
    [
        bx * BLOCK_SIDE + local[0],
        by * BLOCK_SIDE + local[1],
        bz * BLOCK_SIDE + local[2],
    ]
}

/// The region of lattice cell `lattice` in the L1 cell seeded `seed`.
#[must_use]
pub fn region(seed: u64, lattice: [u32; 3]) -> Point {
    let [x, y, z] = lattice;
    let n = u64::from(PER_AXIS);
    let linear = u64::from(x) + n * (u64::from(y) + n * u64::from(z));
    let mut rng = Rng::new(hash_triple(seed, REGION_STREAM_TAG, linear));
    let p = pitch();
    let mut axis =
        |c: u32| (f64::from(c) + 0.5) * p - 0.5 + (rng.next_f64() * 2.0 - 1.0) * JITTER * p;
    let position = [axis(x), axis(y), axis(z)];
    let radius = marker_radius(Level::MIN).unwrap_or(p / 2.0);
    Point {
        form: Form::Spheroid,
        ..Point::bare(position, radius, MarkerKind::Portal)
    }
}

/// Block content of L1: one region per lattice cell, the fixture's cells
/// left to the fixture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Regions;

impl BlockContent for Regions {
    fn make(&self, _grid: &BlockGrid, seed: u64, key: BlockKey, slot: u32) -> Option<Point> {
        let lattice = lattice_of(key, slot);
        if fixture_id(lattice).is_some() {
            return None;
        }
        Some(region(seed, lattice))
    }
}

/// The L1 fixture cell: the eight fixture regions, IDs 0 to 7, with the
/// octant child budgets (#151). Blocks join it near the camera.
#[must_use]
pub fn generate(seed: u64, parent: &Constraints) -> Generated {
    let octants = OctantGenerator.generate(seed, parent);
    if octants.points.is_empty() {
        return octants;
    }
    Generated {
        points: (0..FIXTURE)
            .map(|id| region(seed, fixture_lattice(id)))
            .collect(),
        child_constraints: octants.child_constraints,
        ids: (0..FIXTURE).collect(),
    }
}

/// Region `id` of the L1 cell seeded `seed`, whatever is resident.
#[must_use]
pub fn marker(seed: u64, id: u32) -> Option<Point> {
    if id < FIXTURE {
        return Some(region(seed, fixture_lattice(id)));
    }
    crate::blocks::marker(&Regions, &GRID, seed, id)
}

/// Every region of the L1 cell seeded `seed`, fixture first (the spacing
/// probe counts the whole cell, #1104 T1).
#[must_use]
pub fn all(seed: u64) -> Vec<Point> {
    let mut out: Vec<Point> = (0..FIXTURE)
        .map(|id| region(seed, fixture_lattice(id)))
        .collect();
    let n = GRID.per_axis;
    for z in 0..n {
        for y in 0..n {
            for x in 0..n {
                let key = BlockKey([x, y, z]);
                out.extend(
                    crate::blocks::block(&Regions, &GRID, seed, key)
                        .into_iter()
                        .map(|(_, point)| point),
                );
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "regions_l1_tests.rs"]
mod tests;
