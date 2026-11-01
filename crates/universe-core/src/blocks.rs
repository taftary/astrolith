//! Markers made in blocks near the camera (#1104, Spec v2 Goals 1, 2, 4).
//!
//! A cell that holds more markers than any Marker detail level draws (L1
//! with about 310,000 regions, L5 with about 140,000 stars) is split into
//! a cube grid of blocks. The markers of a block depend only on the cell
//! seed and the block coordinates, and a marker only on its block and
//! slot, so its ID ([`BlockGrid::encode`]) is a permanent address: the
//! same marker comes back with the same bits whatever the route, the visit
//! order or the Marker detail, and [`marker`] rebuilds an off-screen one
//! from its ID alone (saved places). The camera only decides which blocks
//! are resident ([`resident`]), with a wider drop radius than load radius
//! so crossing a block edge does not churn. Pure (`E-CORE-NO-BEVY`,
//! `E-DET-TIERS`).

use crate::r#gen::Point;
use crate::seed::hash_triple;

/// Stream tag of block seeds (keeps them apart from other per-cell
/// streams of the same seed).
pub const BLOCK_STREAM_TAG: u64 = 0xb10c_0000_1104;

/// Drop radius over load radius: a resident block stays until the camera
/// is this much farther than where it was made.
pub const HYSTERESIS: f64 = 1.25;

/// Coordinates of one block, each in `0..per_axis`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockKey(pub [u32; 3]);

/// A cube grid of blocks over a cell (`[-0.5, 0.5]` per axis), each with a
/// fixed number of marker slots, IDs starting at `base`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockGrid {
    /// Blocks per axis.
    pub per_axis: u32,
    /// Marker slots per block (a slot may stay empty).
    pub slots: u32,
    /// First ID of the grid; IDs below it belong to the cell's fixture.
    pub base: u32,
}

/// What a block holds: the content rule of one streamed level.
pub trait BlockContent {
    /// Marker in `slot` of block `key` of the cell seeded `seed`, if any.
    ///
    /// Must be a pure function of its arguments and the content's own
    /// fields; [`block_seed`] gives the block's stream seed.
    fn make(&self, grid: &BlockGrid, seed: u64, key: BlockKey, slot: u32) -> Option<Point>;
}

impl BlockGrid {
    /// Side of one block, in cells.
    #[must_use]
    pub fn side(&self) -> f64 {
        1.0 / f64::from(self.per_axis.max(1))
    }

    /// Number of blocks.
    #[must_use]
    pub fn blocks(&self) -> u64 {
        u64::from(self.per_axis).pow(3)
    }

    /// Lower corner of block `key`, in cells.
    #[must_use]
    pub fn corner(&self, key: BlockKey) -> [f64; 3] {
        let side = self.side();
        let [x, y, z] = key.0;
        [
            f64::from(x) * side - 0.5,
            f64::from(y) * side - 0.5,
            f64::from(z) * side - 0.5,
        ]
    }

    /// Whether `key` lies on the grid.
    #[must_use]
    pub fn holds(&self, key: BlockKey) -> bool {
        key.0.iter().all(|&c| c < self.per_axis)
    }

    /// Linear index of `key` (x fastest).
    #[must_use]
    pub fn linear(&self, key: BlockKey) -> u64 {
        let n = u64::from(self.per_axis);
        let [x, y, z] = key.0;
        u64::from(x) + n * (u64::from(y) + n * u64::from(z))
    }

    /// Stable ID of `slot` in block `key`; `None` off the grid or past the
    /// `u32` range.
    #[must_use]
    pub fn encode(&self, key: BlockKey, slot: u32) -> Option<u32> {
        if !self.holds(key) || slot >= self.slots {
            return None;
        }
        let id = u64::from(self.base) + self.linear(key) * u64::from(self.slots) + u64::from(slot);
        u32::try_from(id).ok()
    }

    /// Block and slot of `id`; `None` for a fixture ID or one past the grid.
    #[must_use]
    pub fn decode(&self, id: u32) -> Option<(BlockKey, u32)> {
        let offset = id.checked_sub(self.base)?;
        let slots = self.slots.max(1);
        let slot = offset % slots;
        let linear = u64::from(offset / slots);
        if linear >= self.blocks() {
            return None;
        }
        let n = u64::from(self.per_axis);
        let part = |v: u64| u32::try_from(v % n).ok();
        Some((
            BlockKey([part(linear)?, part(linear / n)?, part(linear / (n * n))?]),
            slot,
        ))
    }

    /// Block holding the cell-unit position `at`, clamped onto the grid.
    #[must_use]
    pub fn key_of(&self, at: [f64; 3]) -> BlockKey {
        let [x, y, z] = at;
        BlockKey([self.axis_of(x), self.axis_of(y), self.axis_of(z)])
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "E-CAST: clamped to 0..per_axis before the cast"
    )]
    fn axis_of(&self, value: f64) -> u32 {
        let top = f64::from(self.per_axis.saturating_sub(1));
        ((value + 0.5) / self.side()).floor().clamp(0.0, top) as u32
    }

    /// Distance from `at` to the nearest point of block `key`, in cells.
    #[must_use]
    pub fn distance_to(&self, key: BlockKey, at: [f64; 3]) -> f64 {
        let [lx, ly, lz] = self.corner(key);
        let [x, y, z] = at;
        let side = self.side();
        let gap = |p: f64, lo: f64| (lo - p).max(p - (lo + side)).max(0.0);
        let (dx, dy, dz) = (gap(x, lx), gap(y, ly), gap(z, lz));
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Stream seed of block `key` in the cell seeded `seed`.
#[must_use]
pub fn block_seed(grid: &BlockGrid, seed: u64, key: BlockKey) -> u64 {
    hash_triple(seed, BLOCK_STREAM_TAG, grid.linear(key))
}

/// Markers of block `key`, with their IDs, in slot order.
#[must_use]
pub fn block<C: BlockContent>(
    content: &C,
    grid: &BlockGrid,
    seed: u64,
    key: BlockKey,
) -> Vec<(u32, Point)> {
    (0..grid.slots)
        .filter_map(|slot| {
            let id = grid.encode(key, slot)?;
            content.make(grid, seed, key, slot).map(|point| (id, point))
        })
        .collect()
}

/// Marker `id` rebuilt from its ID alone, whatever is resident.
#[must_use]
pub fn marker<C: BlockContent>(content: &C, grid: &BlockGrid, seed: u64, id: u32) -> Option<Point> {
    let (key, slot) = grid.decode(id)?;
    content.make(grid, seed, key, slot)
}

/// Blocks resident with the camera at `camera` (cell units): every block
/// within `load` of it, plus each block of `held` still within
/// `load * HYSTERESIS`. Sorted by key, so the set never depends on the
/// order blocks were made in.
#[must_use]
pub fn resident(grid: &BlockGrid, camera: [f64; 3], load: f64, held: &[BlockKey]) -> Vec<BlockKey> {
    let mut out: Vec<BlockKey> = Vec::new();
    let reach = load * HYSTERESIS;
    let [cx, cy, cz] = camera;
    let BlockKey([x0, y0, z0]) = grid.key_of([cx - reach, cy - reach, cz - reach]);
    let BlockKey([x1, y1, z1]) = grid.key_of([cx + reach, cy + reach, cz + reach]);
    for z in z0..=z1 {
        for y in y0..=y1 {
            for x in x0..=x1 {
                let key = BlockKey([x, y, z]);
                let distance = grid.distance_to(key, camera);
                if distance <= load || (distance <= reach && held.contains(&key)) {
                    out.push(key);
                }
            }
        }
    }
    out.sort_unstable();
    out
}

/// Markers of `keys` with their IDs, in key then slot order: `points` and
/// `ids` parallel, ready for [`crate::r#gen::Generated`].
#[must_use]
pub fn build<C: BlockContent>(
    content: &C,
    grid: &BlockGrid,
    seed: u64,
    keys: &[BlockKey],
) -> (Vec<Point>, Vec<u32>) {
    let mut points = Vec::new();
    let mut ids = Vec::new();
    for &key in keys {
        for (id, point) in block(content, grid, seed, key) {
            ids.push(id);
            points.push(point);
        }
    }
    (points, ids)
}

#[cfg(test)]
#[path = "blocks_tests.rs"]
mod tests;
