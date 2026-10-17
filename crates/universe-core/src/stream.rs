//! Streaming working set: bound accounting with proximity-first unload (#152, #742).
//!
//! The ledger tracks the working set (open-chain cells plus preview chains)
//! by marker chain, each with its per-cell stream seed, last-seen tick, and
//! camera distance. Each sync batch marks the current set with today's
//! distances, then unloads farthest-first beyond `STREAM_CAP`. Ties break
//! by recency, then chain order, so eviction stays deterministic
//! (`E-DET-TIERS`).

use crate::r#gen::Point;
use crate::nest::{MarkerPath, path_seed};
use crate::preview::PreviewCache;
use std::collections::BTreeMap;

/// Most live cells in the streaming working set: open chain, horizon, and
/// previews (#152).
///
/// Beyond it the longest-unseen cell unloads first; the headless proof and
/// the window mirror both assert the bound on every step and frame.
pub const STREAM_CAP: usize = 32;

/// Working set for the bound proof (#152, #742): every open-chain prefix
/// at distance zero (the camera sits inside them) plus one chain per
/// previewed marker at its camera gap in open-cell units.
pub(crate) fn working_chains(
    path: &MarkerPath,
    offset: [f64; 3],
    points: &[Point],
    previews: &PreviewCache,
) -> Vec<(Vec<u32>, f64)> {
    let indices = path.indices();
    let mut chains: Vec<(Vec<u32>, f64)> = Vec::new();
    let mut prefix: Vec<u32> = Vec::new();
    chains.push((prefix.clone(), 0.0));
    for &marker in &indices {
        prefix.push(marker);
        chains.push((prefix.clone(), 0.0));
    }
    for (marker, _) in previews.entries() {
        let mut chain = prefix.clone();
        chain.push(*marker);
        let distance = points
            .get(*marker as usize)
            .map(|point| {
                let at = point.portal_position();
                ((offset[0] - at[0]).powi(2)
                    + (offset[1] - at[1]).powi(2)
                    + (offset[2] - at[2]).powi(2))
                .sqrt()
            })
            .unwrap_or(f64::INFINITY);
        chains.push((chain, distance));
    }
    chains
}

/// One live cell in the streaming working set (#152, #742).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveCell {
    /// Stream seed of this chain: `path_seed` under the ledger's root, so
    /// every live cell names the seeded stream it regenerates from.
    pub seed: u64,
    /// Tick of the sync batch that last marked this chain seen.
    pub last_seen: u64,
    /// Camera gap in open-cell units at the last touch (`0.0` on the open
    /// chain, the marker gap for previews).
    pub distance: f64,
}

/// Live-cell ledger for the streaming budget (#152, #742).
///
/// Tracks the working set (open-chain cells plus preview chains) by marker
/// chain, each with its per-cell stream seed, last-seen tick, and camera
/// distance. Each sync batch marks the current set with today's distances,
/// then unloads farthest-first beyond [`STREAM_CAP`]. Distance ties break
/// by recency (longest-unseen first), then chain order (`BTreeMap`
/// iteration with `total_cmp`, NaN sorting farthest), so eviction is
/// deterministic (`E-DET-TIERS`). Content still lives in
/// `Universe`/`PreviewCache`; the ledger is the bound accounting read by
/// both the headless proof and the window mirror.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StreamLedger {
    cells: BTreeMap<Vec<u32>, LiveCell>,
    tick: u64,
    /// Total cells unloaded by the cap (test/verify counter).
    pub evictions: u64,
}

impl StreamLedger {
    /// Number of live cells (always `<= STREAM_CAP` after a sync).
    #[must_use]
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Returns `true` when no cell is tracked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Returns `true` when `chain` is live. Recency is unchanged.
    #[must_use]
    pub fn contains(&self, chain: &[u32]) -> bool {
        self.cells.contains_key(chain)
    }

    /// Stream seed recorded for `chain`, if live.
    #[must_use]
    pub fn seed_of(&self, chain: &[u32]) -> Option<u64> {
        self.cells.get(chain).map(|cell| cell.seed)
    }

    /// Tick that last marked `chain` seen, if live.
    #[must_use]
    pub fn last_seen_of(&self, chain: &[u32]) -> Option<u64> {
        self.cells.get(chain).map(|cell| cell.last_seen)
    }

    /// Camera gap recorded for `chain` at its last touch, if live.
    #[must_use]
    pub fn distance_of(&self, chain: &[u32]) -> Option<f64> {
        self.cells.get(chain).map(|cell| cell.distance)
    }

    /// Advances to a new tick; call once per sync batch (frame or step).
    pub fn begin_sync(&mut self) {
        self.tick = self.tick.saturating_add(1);
    }

    /// Marks one live chain seen at `distance` from the camera, loading it
    /// if new. The latest distance wins: proximity is always today's.
    pub fn touch(&mut self, root: u64, chain: &[u32], distance: f64) {
        let tick = self.tick;
        self.cells
            .entry(chain.to_vec())
            .and_modify(|cell| {
                cell.last_seen = tick;
                cell.distance = distance;
            })
            .or_insert(LiveCell {
                seed: path_seed(root, chain),
                last_seen: tick,
                distance,
            });
    }

    /// Unloads farthest-first beyond [`STREAM_CAP`].
    ///
    /// Returns the number of cells unloaded. Distance ties evict the
    /// longest-unseen first, then the smallest chain, deterministically.
    pub fn enforce_cap(&mut self) -> usize {
        let mut unloaded = 0;
        while self.cells.len() > STREAM_CAP {
            let mut victim: Option<(Vec<u32>, f64, u64)> = None;
            for (chain, cell) in &self.cells {
                let replace = match &victim {
                    None => true,
                    Some((_, far, seen)) => {
                        cell.distance.total_cmp(far) == std::cmp::Ordering::Greater
                            || (cell.distance.total_cmp(far) == std::cmp::Ordering::Equal
                                && cell.last_seen < *seen)
                    }
                };
                if replace {
                    victim = Some((chain.clone(), cell.distance, cell.last_seen));
                }
            }
            let Some((chain, _, _)) = victim else {
                break;
            };
            self.cells.remove(&chain);
            self.evictions += 1;
            unloaded += 1;
        }
        unloaded
    }

    /// Marks `entries` seen at a new tick, then enforces the cap.
    ///
    /// Returns true when membership changed (loads or unloads).
    pub fn sync(&mut self, root: u64, entries: &[(Vec<u32>, f64)]) -> bool {
        let before = self.cells.len();
        self.begin_sync();
        for (chain, distance) in entries {
            self.touch(root, chain, *distance);
        }
        let unloaded = self.enforce_cap();
        unloaded > 0 || self.cells.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::DEMO_SEED;

    #[test]
    fn ledger_unloads_longest_unseen_first_beyond_the_cap() {
        let mut ledger = StreamLedger::default();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: test chains stay below the 32-cell cap"
        )]
        let full: Vec<(Vec<u32>, f64)> = (0..STREAM_CAP as u32).map(|m| (vec![m], 0.0)).collect();
        assert!(ledger.sync(DEMO_SEED, &full));
        assert_eq!(ledger.len(), STREAM_CAP);
        assert_eq!(ledger.evictions, 0);
        // Re-touch one chain without loading anything new: nothing unloads.
        assert!(!ledger.sync(DEMO_SEED, &[(vec![0], 0.0)]));
        assert_eq!(ledger.evictions, 0);
        // Overload with 31 fresh chains at the same distance: exactly the
        // 31 tick-1 chains go while the re-touched chain and the fresh
        // ones survive.
        let fresh: Vec<(Vec<u32>, f64)> = (32..63).map(|m| (vec![m], 0.0)).collect();
        assert!(ledger.sync(DEMO_SEED, &fresh));
        assert_eq!(ledger.len(), STREAM_CAP);
        assert_eq!(ledger.evictions, 31);
        assert!(ledger.contains(&[0]));
        assert!(!ledger.contains(&[1]));
        assert!(ledger.contains(&[62]));
    }

    #[test]
    fn ledger_evicts_farthest_first_within_the_cap() {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: test chains stay below the 32-cell cap"
        )]
        let cap = STREAM_CAP as u32;
        let mut ledger = StreamLedger::default();
        // Fill the cap up close: every chain rides distance zero.
        let near: Vec<(Vec<u32>, f64)> = (0..cap).map(|m| (vec![m], 0.0)).collect();
        assert!(ledger.sync(DEMO_SEED, &near));
        assert_eq!(ledger.len(), STREAM_CAP);
        // One fresh far chain crowds in: it unloads at once, newest tick
        // or not, while every near chain survives.
        let mut crowd = near.clone();
        crowd.push((vec![100], 50.0));
        assert!(ledger.sync(DEMO_SEED, &crowd));
        assert_eq!(ledger.len(), STREAM_CAP);
        assert_eq!(ledger.evictions, 1);
        assert!(!ledger.contains(&[100]), "the far chain goes first");
        for m in 0..cap {
            assert!(ledger.contains(&[m]), "near chain {m} survives");
        }
        // Distances refresh on touch: a near chain re-seen farther out
        // keeps its place but carries today's gap.
        assert!(!ledger.sync(DEMO_SEED, &[(vec![5], 3.0)]));
        assert_eq!(ledger.distance_of(&[5]), Some(3.0));
        assert_eq!(ledger.len(), STREAM_CAP);
    }

    #[test]
    fn ledger_eviction_ties_break_by_chain_order() {
        let run = || {
            let mut ledger = StreamLedger::default();
            let batch: Vec<(Vec<u32>, f64)> = (0..40u32).map(|m| (vec![m], 0.0)).collect();
            ledger.sync(7, &batch);
            (0..40u32)
                .filter(|m| ledger.contains(&[*m]))
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run(), "eviction must be deterministic");
        // All 40 chains share tick 1 at distance zero: the 8 smallest
        // unload first.
        assert_eq!(run(), (8..40u32).collect::<Vec<_>>());
    }

    #[test]
    fn ledger_records_per_cell_stream_seeds() {
        let mut ledger = StreamLedger::default();
        assert!(ledger.sync(
            DEMO_SEED,
            &[(vec![], 0.0), (vec![3], 1.5), (vec![3, 5], 2.5)]
        ));
        assert_eq!(ledger.seed_of(&[]), Some(path_seed(DEMO_SEED, &[])));
        assert_eq!(ledger.seed_of(&[3, 5]), Some(path_seed(DEMO_SEED, &[3, 5])));
        assert_eq!(ledger.seed_of(&[9]), None);
        assert_eq!(ledger.last_seen_of(&[3]), Some(1));
        assert_eq!(ledger.distance_of(&[3]), Some(1.5));
        assert!(!ledger.is_empty());
    }
}
