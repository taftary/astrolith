//! Nested integer-cell coordinates and floating-origin frames.
//!
//! Bevy transforms are `f32`, which cannot span the 27 orders of magnitude of
//! the scale ladder (`docs/universes/ladder.md`). This module is the custom
//! coordinate layer required by the owner decision: places are named with
//! exact integers ([`CellPos`](crate::coords::CellPos)) while the observer holds a small [`Frame`](crate::coords::Frame)
//! whose float64 offset never grows large, because [`Frame::reanchor`](crate::coords::Frame::reanchor) moves
//! the origin into a neighbouring cell before precision is lost.

/// Half-size of a cell in units of that cell.
///
/// A settled offset component always lies in `[-HALF_BOUND, HALF_BOUND)`.
pub const HALF_BOUND: f64 = 0.5;

/// Shallowest level number (L1, observable universe).
pub const MIN_LEVEL: u8 = 1;

/// Deepest level number (L14, room; the L2/L3 merge of #151 retired one rung).
pub const MAX_LEVEL: u8 = 14;

/// Position in units of the open cell (the render origin frame).
///
/// Bare `[f64; 3]` positions in two unit frames used to mix silently; the
/// compiler now keeps them apart (`E-NEWTYPE`). Convert between frames
/// only through the named functions (`sibling_in_open_units`,
/// `child_world_position`); never do frame arithmetic at the call site.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpenUnits(pub [f64; 3]);

impl From<[f64; 3]> for OpenUnits {
    fn from(position: [f64; 3]) -> OpenUnits {
        OpenUnits(position)
    }
}

/// Position in units of the parent cell.
///
/// Companion to [`OpenUnits`]: markers, sibling inputs, and
/// `child_world_position` results live in this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParentUnits(pub [f64; 3]);

impl From<[f64; 3]> for ParentUnits {
    fn from(position: [f64; 3]) -> ParentUnits {
        ParentUnits(position)
    }
}

/// Level index `l` in `1..=14` (see `docs/universes/ladder.md`).
///
/// The range is enforced at construction: [`Level::new`] returns `None`
/// outside `1..=14`, so a `Level` value is always a valid ladder rung.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Level(u8);

impl Level {
    /// Shallowest level (L1, observable universe).
    pub const MIN: Level = Level(MIN_LEVEL);

    /// Deepest level (L14, room).
    pub const MAX: Level = Level(MAX_LEVEL);

    /// Builds a level, returning `None` when `value` is outside `1..=14`.
    #[must_use]
    pub fn new(value: u8) -> Option<Level> {
        if (MIN_LEVEL..=MAX_LEVEL).contains(&value) {
            Some(Level(value))
        } else {
            None
        }
    }

    /// Returns the raw level number in `1..=14`.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }

    /// Returns the next deeper level, or `None` at L14.
    #[must_use]
    pub fn deeper(self) -> Option<Level> {
        Level::new(self.0 + 1)
    }

    /// Returns the next shallower level, or `None` at L1.
    pub fn shallower(self) -> Option<Level> {
        self.0.checked_sub(1).and_then(Level::new)
    }
}

/// A single cell `C = (level, x, y, z)`: a level plus an integer grid index.
///
/// Coordinates are unbounded `i64` values, so the grid never runs out; only
/// the level is range-checked (via [`Level`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CellPos {
    /// Level this cell lives on.
    pub level: Level,
    /// Integer grid coordinates of the cell.
    pub x: i64,
    /// Integer grid coordinates of the cell.
    pub y: i64,
    /// Integer grid coordinates of the cell.
    pub z: i64,
}

impl CellPos {
    /// Builds the cell `(level, x, y, z)`.
    #[must_use]
    pub const fn new(level: Level, x: i64, y: i64, z: i64) -> CellPos {
        CellPos { level, x, y, z }
    }

    /// Returns the neighbour of this cell shifted by `(dx, dy, dz)`.
    ///
    /// Neighbour moves commute: stepping `+x` then `+y` names the same cell
    /// as stepping `+y` then `+x`. Coordinates wrap (rather than panic) on
    /// `i64` overflow; the grid is unbounded so wrapping only occurs for
    /// astronomic inputs, and it stays deterministic.
    #[must_use]
    pub fn neighbor(self, dx: i64, dy: i64, dz: i64) -> CellPos {
        CellPos {
            level: self.level,
            x: self.x.wrapping_add(dx),
            y: self.y.wrapping_add(dy),
            z: self.z.wrapping_add(dz),
        }
    }

    /// Returns the parent cell one level shallower, or `None` at L1.
    ///
    /// Parent coordinates use Euclidean division by two, so negative child
    /// coordinates map to the correct parent (e.g. child `-1` has parent `-1`,
    /// child `-2` has parent `-1`).
    #[must_use]
    pub fn parent(self) -> Option<CellPos> {
        self.level.shallower().map(|level| CellPos {
            level,
            x: self.x.div_euclid(2),
            y: self.y.div_euclid(2),
            z: self.z.div_euclid(2),
        })
    }

    /// Returns the child cell one level deeper in the given octant, or `None`.
    ///
    /// Each `octant` component must be `0` or `1`; anything else returns
    /// `None`. Returns `None` at L14 and on `i64` overflow.
    /// [`CellPos::parent`] inverts this: `cell.child(o).parent() == cell`.
    pub fn child(self, octant: [u8; 3]) -> Option<CellPos> {
        if octant.iter().any(|&bit| bit > 1) {
            return None;
        }
        let level = self.level.deeper()?;
        let (x, y, z) = (self.x, self.y, self.z);
        let [bx, by, bz] = octant.map(i64::from);
        Some(CellPos {
            level,
            x: x.checked_mul(2)?.checked_add(bx)?,
            y: y.checked_mul(2)?.checked_add(by)?,
            z: z.checked_mul(2)?.checked_add(bz)?,
        })
    }
}

/// Observer frame: an integer chain of cells plus a small float64 offset.
///
/// `chain[i]` holds the `(x, y, z)` grid index of the cell at level `i + 1`,
/// so `chain.len()` is both the active depth and the deepest level number.
/// `offset` is the position inside the deepest cell in units of that cell,
/// valid in `[-HALF_BOUND, HALF_BOUND)` once [`Frame::reanchor`] has run.
///
/// The integer chain carries unbounded range; the float64 offset carries
/// sub-cell precision. Neither part alone spans the ladder, together they do.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// Cell indices from L1 down to the deepest active cell.
    chain: Vec<(i64, i64, i64)>,
    /// Position inside the deepest cell, in units of that cell.
    offset: [f64; 3],
}

impl Frame {
    /// Builds a frame, returning `None` when the chain is empty, longer than
    /// 14 entries, or the offset is non-finite.
    #[must_use]
    pub fn new(chain: Vec<(i64, i64, i64)>, offset: [f64; 3]) -> Option<Frame> {
        if chain.is_empty()
            || chain.len() > usize::from(MAX_LEVEL)
            || offset.iter().any(|component| !component.is_finite())
        {
            return None;
        }
        Some(Frame { chain, offset })
    }

    /// Returns the deepest active level (the chain length as a [`Level`]).
    #[must_use]
    pub fn level(&self) -> Level {
        // SAFETY: `new` guarantees `1 <= len <= 14`.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: `new` guarantees len 1..=15, always fits u8"
        )]
        let len = self.chain.len() as u8;
        Level::new(len).unwrap_or(Level::MAX)
    }

    /// Returns the deepest active cell as a [`CellPos`].
    #[must_use]
    pub fn deepest(&self) -> CellPos {
        let &(x, y, z) = self.chain.last().unwrap_or(&(0, 0, 0));
        CellPos::new(self.level(), x, y, z)
    }

    /// Returns the cell chain from L1 down to the deepest active cell.
    #[must_use]
    pub fn chain(&self) -> &[(i64, i64, i64)] {
        &self.chain
    }

    /// Returns the offset inside the deepest cell, in units of that cell.
    #[must_use]
    pub fn offset(&self) -> [f64; 3] {
        self.offset
    }

    /// Replaces the offset; call [`Frame::reanchor`] afterwards to restore
    /// the `[-HALF_BOUND, HALF_BOUND)` invariant.
    pub fn set_offset(&mut self, offset: [f64; 3]) {
        if offset.iter().all(|component| component.is_finite()) {
            self.offset = offset;
        }
    }

    /// Moves the origin into a neighbouring cell until every offset component
    /// lies in `[-HALF_BOUND, HALF_BOUND)`.
    ///
    /// Multi-cell overflow is crossed in one call (the shift is computed with
    /// `floor`, not stepped). The move is exact for offsets within 2^52 cell
    /// units; beyond float64 integer precision the cell index saturates
    /// instead of panicking.
    pub fn reanchor(&mut self) {
        let Some(last) = self.chain.last_mut() else {
            return;
        };
        let cells = [&mut last.0, &mut last.1, &mut last.2];
        for (cell, off) in cells.into_iter().zip(self.offset.iter_mut()) {
            let shift = (*off + HALF_BOUND).floor();
            if shift != 0.0 {
                // `shift` is integral; `as` saturates on overflow by definition.
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "E-CAST: integral shift, saturating `as` is the documented behavior"
                )]
                let step = shift as i64;
                *cell = cell.wrapping_add(step);
                *off -= shift;
            }
        }
    }

    /// Moves the deepest cell by `(dx, dy, dz)`; the offset is unchanged.
    pub fn step_to_neighbor(&mut self, dx: i64, dy: i64, dz: i64) {
        if let Some(last) = self.chain.last_mut() {
            last.0 = last.0.wrapping_add(dx);
            last.1 = last.1.wrapping_add(dy);
            last.2 = last.2.wrapping_add(dz);
        }
    }

    /// Descends one level into the given octant (each component `0` or `1`).
    ///
    /// Returns `false` — leaving the frame untouched — at L14, for octant
    /// components above `1`, or on `i64` overflow. [`Frame::zoom_out`]
    /// inverts a successful call.
    pub fn zoom_in(&mut self, octant: [u8; 3]) -> bool {
        if self.chain.len() >= usize::from(MAX_LEVEL) {
            return false;
        }
        if octant.iter().any(|&bit| bit > 1) {
            return false;
        }
        let Some(&(px, py, pz)) = self.chain.last() else {
            return false;
        };
        let [bx, by, bz] = octant.map(i64::from);
        let Some(nx) = px.checked_mul(2).and_then(|v| v.checked_add(bx)) else {
            return false;
        };
        let Some(ny) = py.checked_mul(2).and_then(|v| v.checked_add(by)) else {
            return false;
        };
        let Some(nz) = pz.checked_mul(2).and_then(|v| v.checked_add(bz)) else {
            return false;
        };
        for (off, bit) in self.offset.iter_mut().zip(octant.iter()) {
            *off = *off * 2.0 - f64::from(*bit) + HALF_BOUND;
        }
        self.chain.push((nx, ny, nz));
        true
    }

    /// Ascends one level, returning `false` — leaving the frame untouched —
    /// at the root (chain length 1).
    ///
    /// This inverts [`Frame::zoom_in`]; after neighbour steps in between, the
    /// restored offset may itself need [`Frame::reanchor`].
    pub fn zoom_out(&mut self) -> bool {
        if self.chain.len() <= 1 {
            return false;
        }
        // SAFETY: length checked above, so `pop` yields a value.
        let (cx, cy, cz) = self.chain.pop().unwrap_or((0, 0, 0));
        for (coord, off) in [cx, cy, cz].into_iter().zip(self.offset.iter_mut()) {
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: rem_euclid(2) is 0 or 1, exactly representable"
            )]
            let bit = coord.rem_euclid(2) as f64;
            *off = (*off + bit - HALF_BOUND) / 2.0;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_accepts_only_1_to_14() {
        assert_eq!(Level::new(0), None);
        assert_eq!(Level::new(1).map(Level::get), Some(1));
        assert_eq!(Level::new(14).map(Level::get), Some(14));
        assert_eq!(Level::new(15), None);
        assert_eq!(Level::MIN.deeper().map(Level::get), Some(2));
        assert_eq!(Level::MAX.deeper(), None);
        assert_eq!(Level::MIN.shallower(), None);
        assert_eq!(Level::MAX.shallower().map(Level::get), Some(13));
    }

    #[test]
    fn same_inputs_build_identical_cells() {
        let level = Level::new(7).unwrap_or(Level::MIN);
        assert_eq!(CellPos::new(level, 3, -4, 5), CellPos::new(level, 3, -4, 5));
        assert_eq!(
            Frame::new(vec![(0, 0, 0), (3, -4, 5)], [0.1, 0.2, 0.3]),
            Frame::new(vec![(0, 0, 0), (3, -4, 5)], [0.1, 0.2, 0.3])
        );
    }

    #[test]
    fn neighbor_moves_commute() {
        let level = Level::new(5).unwrap_or(Level::MIN);
        let origin = CellPos::new(level, 10, -3, 0);
        let xy = origin.neighbor(1, 0, 0).neighbor(0, 1, 0);
        let yx = origin.neighbor(0, 1, 0).neighbor(1, 0, 0);
        assert_eq!(xy, yx);
        assert_eq!(xy, origin.neighbor(1, 1, 0));
    }

    #[test]
    fn parent_child_invert_each_other() {
        let level = Level::new(6).unwrap_or(Level::MIN);
        let cell = CellPos::new(level, 3, -4, 5);
        let child = cell.child([1, 0, 1]).expect("valid octant below L14");
        assert_eq!(child.parent(), Some(cell));
        let root = CellPos::new(Level::MIN, 0, 0, 0);
        assert_eq!(root.parent(), None);
        let bottom = CellPos::new(Level::MAX, 1, 2, 3);
        assert_eq!(bottom.child([0, 0, 0]), None);
        assert_eq!(cell.child([2, 0, 0]), None);
    }

    #[test]
    fn reanchor_moves_origin_into_bounds() {
        let mut frame = Frame::new(vec![(0, 0, 0)], [0.7, -0.6, 2.3]).expect("valid frame");
        frame.reanchor();
        assert_eq!(frame.deepest().x, 1);
        assert_eq!(frame.deepest().y, -1);
        assert_eq!(frame.deepest().z, 2);
        let [ox, oy, oz] = frame.offset();
        assert!((ox - (-0.3)).abs() < 1e-12, "ox = {ox}");
        assert!((oy - 0.4).abs() < 1e-12, "oy = {oy}");
        assert!((oz - 0.3).abs() < 1e-12, "oz = {oz}");
        // Settled: a second pass changes nothing.
        let settled = frame.clone();
        frame.reanchor();
        assert_eq!(frame, settled);
    }

    #[test]
    fn neighbor_steps_round_trip_around_reanchor() {
        let mut frame = Frame::new(vec![(4, 4, 4)], [1.7, 0.0, -1.2]).expect("valid frame");
        let before = frame.clone();
        frame.reanchor();
        // Undo the (+2, 0, -1) shift the re-anchor applied.
        frame.step_to_neighbor(-2, 0, 1);
        assert_eq!(frame.deepest(), before.deepest());
        // Restoring the original offset recovers the exact original frame.
        frame.set_offset(before.offset());
        assert_eq!(frame, before);
    }

    #[test]
    fn zoom_in_then_out_round_trips() {
        let mut frame =
            Frame::new(vec![(0, 0, 0), (3, -4, 5)], [0.1, -0.2, 0.0]).expect("valid frame");
        let before = frame.clone();
        assert!(frame.zoom_in([1, 0, 1]));
        assert_eq!(frame.level(), Level::new(3).unwrap_or(Level::MIN));
        assert!(frame.zoom_out());
        assert_eq!(frame.deepest(), before.deepest());
        for (got, want) in frame.offset().iter().zip(before.offset().iter()) {
            assert!((got - want).abs() < 1e-12, "got {got}, want {want}");
        }
        assert!(
            !Frame::new(vec![(0, 0, 0)], [0.0; 3])
                .expect("root")
                .zoom_out()
        );
    }

    #[test]
    fn frame_rejects_empty_oversized_and_nonfinite() {
        assert_eq!(Frame::new(vec![], [0.0; 3]), None);
        assert_eq!(Frame::new(vec![(0, 0, 0); 16], [0.0; 3]), None);
        assert_eq!(Frame::new(vec![(0, 0, 0)], [f64::NAN, 0.0, 0.0]), None);
        assert_eq!(Frame::new(vec![(0, 0, 0)], [f64::INFINITY, 0.0, 0.0]), None);
    }
}
