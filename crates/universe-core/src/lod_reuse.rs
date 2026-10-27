//! Parent-to-child height reuse (#994 T3).
//!
//! A build samples its patch at double resolution (65 x 65) to find its
//! relief error. Those points are exactly the 33 x 33 vertex grids of its
//! four children, with most of their normals ring: the same grid indices on
//! the same face give the same direction bits, so the same height bits. A
//! build keeps that grid as a [`FineGrid`], and a child built later reads
//! its own grid points from it instead of sampling them again. The ground
//! is the same, bit for bit; only the work changes.
//!
//! [`HeightCache`] holds the most recent grids, bounded in count. Pure and
//! deterministic (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use std::collections::VecDeque;
use std::sync::Arc;

use crate::cube_sphere::grid_point;
use crate::lod::{PATCH_QUADS, PatchKey};
use crate::relief::{HeightSampler, Relief};

/// Points per side of a fine grid (65).
pub const FINE_SIDE: usize = (PATCH_QUADS as usize) * 2 + 1;

/// Fine grids a [`HeightCache`] holds by default: about 8.7 MB, a few
/// seconds of builds, so a patch that splits as the camera comes down
/// still finds its grid.
pub const DEFAULT_GRIDS: usize = 256;

/// One build's double-resolution heights (65 x 65, row-major, metres).
#[derive(Debug, Clone, PartialEq)]
pub struct FineGrid {
    /// The patch sampled.
    key: PatchKey,
    /// The relief seed it was sampled on.
    seed: u64,
    /// Planet radius bits it was built for.
    radius_bits: u64,
    /// Heights at the patch's double-resolution grid points.
    heights: Vec<f64>,
}

impl FineGrid {
    /// A grid of `key` on `relief` at `radius_m`, `None` unless `heights`
    /// holds 65 x 65 values.
    #[must_use]
    pub fn new(key: PatchKey, relief: &Relief, radius_m: f64, heights: Vec<f64>) -> Option<Self> {
        (heights.len() == FINE_SIDE * FINE_SIDE).then(|| Self {
            key,
            seed: relief.seed(),
            radius_bits: radius_m.to_bits(),
            heights,
        })
    }

    /// The patch this grid samples.
    #[must_use]
    pub fn key(&self) -> PatchKey {
        self.key
    }

    /// Whether this grid is `child`'s parent's, on the same relief.
    fn parents(&self, child: PatchKey, relief: &Relief, radius_m: f64) -> bool {
        child.level > 0
            && self.key.face == child.face
            && self.key.level + 1 == child.level
            && self.key.x == child.x / 2
            && self.key.y == child.y / 2
            && self.seed == relief.seed()
            && self.radius_bits == radius_m.to_bits()
    }

    /// The height at grid point `(gi, gj)` of the child level, when this
    /// grid covers it.
    fn at(&self, gi: u32, gj: u32) -> Option<f64> {
        let fi = i64::from(gi) - i64::from(self.key.x) * i64::from(PATCH_QUADS) * 2;
        let fj = i64::from(gj) - i64::from(self.key.y) * i64::from(PATCH_QUADS) * 2;
        let fi = usize::try_from(fi).ok().filter(|i| *i < FINE_SIDE)?;
        let fj = usize::try_from(fj).ok().filter(|j| *j < FINE_SIDE)?;
        self.heights.get(fj * FINE_SIDE + fi).copied()
    }
}

/// The most recent fine grids, oldest dropped first (#994 T3).
#[derive(Debug, Clone)]
pub struct HeightCache {
    /// Most grids held.
    cap: usize,
    /// Held grids, oldest first.
    grids: VecDeque<Arc<FineGrid>>,
}

impl Default for HeightCache {
    fn default() -> Self {
        Self::new(DEFAULT_GRIDS)
    }
}

impl HeightCache {
    /// An empty cache holding at most `cap` grids.
    #[must_use]
    pub fn new(cap: usize) -> Self {
        Self {
            cap,
            grids: VecDeque::with_capacity(cap),
        }
    }

    /// Grids held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.grids.len()
    }

    /// Whether no grid is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.grids.is_empty()
    }

    /// `child`'s parent's grid on `relief` at `radius_m`, when held.
    #[must_use]
    pub fn parent_of(
        &self,
        child: PatchKey,
        relief: &Relief,
        radius_m: f64,
    ) -> Option<Arc<FineGrid>> {
        self.grids
            .iter()
            .rev()
            .find(|grid| grid.parents(child, relief, radius_m))
            .cloned()
    }

    /// Keeps `grid`, replacing an older one of the same patch and dropping
    /// the oldest beyond the cap.
    pub fn insert(&mut self, grid: Arc<FineGrid>) {
        self.grids.retain(|held| held.key != grid.key);
        if self.cap == 0 {
            return;
        }
        while self.grids.len() >= self.cap {
            self.grids.pop_front();
        }
        self.grids.push_back(grid);
    }
}

/// A build's reads on its own level's grid (#994 T3): from the parent's
/// fine grid where it covers the point, else sampled.
pub(crate) struct GridReads<'s, 'r> {
    /// The build's sampler.
    pub(crate) sampler: &'s mut HeightSampler<'r>,
    /// The parent's fine grid, when held.
    parent: Option<&'s FineGrid>,
    /// The patch built.
    key: PatchKey,
    /// Grid points per face edge at the patch's level.
    n: u32,
}

impl<'s, 'r> GridReads<'s, 'r> {
    /// Reads for `key` (with `n` grid points per face edge); `parent` is
    /// used only when it is the patch's parent.
    pub(crate) fn new(
        sampler: &'s mut HeightSampler<'r>,
        parent: Option<&'s FineGrid>,
        key: PatchKey,
        radius_m: f64,
        n: u32,
    ) -> Self {
        let relief = sampler.relief();
        let parent = parent.filter(|grid| grid.parents(key, relief, radius_m));
        Self {
            sampler,
            parent,
            key,
            n,
        }
    }

    /// The direction and height of grid point `(gi, gj)` on the patch's face.
    pub(crate) fn point(&mut self, gi: u32, gj: u32) -> Option<([f64; 3], f64)> {
        let dir = grid_point(self.key.face, gi, gj, self.n)?;
        let h = match self.parent.and_then(|grid| grid.at(gi, gj)) {
            Some(h) => h,
            None => self.sampler.height_m(dir),
        };
        Some((dir, h))
    }
}

#[cfg(test)]
#[path = "lod_reuse_tests.rs"]
mod tests;
