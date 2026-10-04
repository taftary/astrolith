//! Streaming working set: bound accounting with longest-unseen-first unload (#152).
//!
//! The ledger tracks the working set (open-chain cells plus preview chains)
//! by marker chain, each with its per-cell stream seed and last-seen tick.
//! Each sync batch marks the current set, then unloads longest-unseen-first
//! beyond `STREAM_CAP`. Ordering is key order, so eviction ties break
//! deterministically (`E-DET-TIERS`).

use crate::nest::{MarkerPath, path_seed};
use crate::preview::PreviewCache;
use std::collections::BTreeMap;

/// Most live cells in the streaming working set: open chain, horizon, and
/// previews (#152).
///
/// Beyond it the longest-unseen cell unloads first; the headless proof and
/// the window mirror both assert the bound on every step and frame.
pub const STREAM_CAP: usize = 32;

/// Working-set chains for the bound proof (#152): every open-chain prefix
/// plus one chain per previewed marker.
pub(crate) fn working_chains(path: &MarkerPath, previews: &PreviewCache) -> Vec<Vec<u32>> {
    let indices = path.indices();
    let mut chains: Vec<Vec<u32>> = Vec::new();
    let mut prefix: Vec<u32> = Vec::new();
    chains.push(prefix.clone());
    for &marker in &indices {
        prefix.push(marker);
        chains.push(prefix.clone());
    }
    for (marker, _) in previews.entries() {
        let mut chain = prefix.clone();
        chain.push(*marker);
        chains.push(chain);
    }
    chains
}

/// One live cell in the streaming working set (#152).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveCell {
    /// Stream seed of this chain: `path_seed` under the ledger's root, so
    /// every live cell names the seeded stream it regenerates from.
    pub seed: u64,
    /// Tick of the sync batch that last marked this chain seen.
    pub last_seen: u64,
}

/// Live-cell ledger for the streaming budget (#152).
///
/// Tracks the working set (open-chain cells plus preview chains) by marker
/// chain, each with its per-cell stream seed and last-seen tick. Each sync
/// batch marks the current set, then unloads longest-unseen-first beyond
/// [`STREAM_CAP`]. Ties break by chain order (`BTreeMap` iteration), so
/// eviction is deterministic (`E-DET-TIERS`). Content still lives in
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

    /// Advances to a new tick; call once per sync batch (frame or step).
    pub fn begin_sync(&mut self) {
        self.tick = self.tick.saturating_add(1);
    }

    /// Marks one live chain seen at the current tick, loading it if new.
    pub fn touch(&mut self, root: u64, chain: &[u32]) {
        let tick = self.tick;
        self.cells
            .entry(chain.to_vec())
            .and_modify(|cell| cell.last_seen = tick)
            .or_insert(LiveCell {
                seed: path_seed(root, chain),
                last_seen: tick,
            });
    }

    /// Unloads longest-unseen-first beyond [`STREAM_CAP`].
    ///
    /// Returns the number of cells unloaded. Ties (same tick) evict the
    /// smallest chain first, deterministically.
    pub fn enforce_cap(&mut self) -> usize {
        let mut unloaded = 0;
        while self.cells.len() > STREAM_CAP {
            let mut victim: Option<(Vec<u32>, u64)> = None;
            for (chain, cell) in &self.cells {
                let replace = match &victim {
                    None => true,
                    Some((_, seen)) => cell.last_seen < *seen,
                };
                if replace {
                    victim = Some((chain.clone(), cell.last_seen));
                }
            }
            let Some((chain, _)) = victim else {
                break;
            };
            self.cells.remove(&chain);
            self.evictions += 1;
            unloaded += 1;
        }
        unloaded
    }

    /// Marks `chains` seen at a new tick, then enforces the cap.
    ///
    /// Returns true when membership changed (loads or unloads).
    pub fn sync(&mut self, root: u64, chains: &[Vec<u32>]) -> bool {
        let before = self.cells.len();
        self.begin_sync();
        for chain in chains {
            self.touch(root, chain);
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
        let full: Vec<Vec<u32>> = (0..STREAM_CAP as u32).map(|m| vec![m]).collect();
        assert!(ledger.sync(DEMO_SEED, &full));
        assert_eq!(ledger.len(), STREAM_CAP);
        assert_eq!(ledger.evictions, 0);
        // Re-touch one chain without loading anything new: nothing unloads.
        assert!(!ledger.sync(DEMO_SEED, &[vec![0]]));
        assert_eq!(ledger.evictions, 0);
        // Overload with 31 fresh chains: exactly the 31 tick-1 chains go
        // while the re-touched chain and the fresh ones survive.
        let fresh: Vec<Vec<u32>> = (32..63).map(|m| vec![m]).collect();
        assert!(ledger.sync(DEMO_SEED, &fresh));
        assert_eq!(ledger.len(), STREAM_CAP);
        assert_eq!(ledger.evictions, 31);
        assert!(ledger.contains(&[0]));
        assert!(!ledger.contains(&[1]));
        assert!(ledger.contains(&[62]));
    }

    #[test]
    fn ledger_eviction_ties_break_by_chain_order() {
        let run = || {
            let mut ledger = StreamLedger::default();
            let batch: Vec<Vec<u32>> = (0..40u32).map(|m| vec![m]).collect();
            ledger.sync(7, &batch);
            (0..40u32)
                .filter(|m| ledger.contains(&[*m]))
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run(), "eviction must be deterministic");
        // All 40 chains share tick 1: the 8 smallest unload first.
        assert_eq!(run(), (8..40u32).collect::<Vec<_>>());
    }

    #[test]
    fn ledger_records_per_cell_stream_seeds() {
        let mut ledger = StreamLedger::default();
        assert!(ledger.sync(DEMO_SEED, &[vec![], vec![3], vec![3, 5]]));
        assert_eq!(ledger.seed_of(&[]), Some(path_seed(DEMO_SEED, &[])));
        assert_eq!(ledger.seed_of(&[3, 5]), Some(path_seed(DEMO_SEED, &[3, 5])));
        assert_eq!(ledger.seed_of(&[9]), None);
        assert_eq!(ledger.last_seen_of(&[3]), Some(1));
        assert!(!ledger.is_empty());
    }
}
