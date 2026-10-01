//! Fixed-capacity least-recently-used cell cache.
//!
//! Generated cells are pure functions of their seed, so they can be dropped
//! and rebuilt at will; this cache keeps the nearby working set in memory
//! (acceptance criterion 3 of spec v4: nearby places kept, far ones dropped).
//! The live count is bounded by `capacity`, never by universe size. Reads
//! refresh recency, so eviction drops the least-recently-used entry; with no
//! reads this degenerates to insertion-order eviction.

use crate::coords::CellPos;
use std::collections::{HashMap, VecDeque};

/// In-memory store for generated cells, keyed by [`CellPos`].
///
/// The generic payload `V` is whatever the caller generates per cell (e.g.
/// [`crate::r#gen::Generated`]). `capacity` is a hard bound: `len() <=
/// capacity()` always holds, including for `capacity == 0` (which stores
/// nothing and only reports misses).
#[derive(Clone, Debug)]
pub struct CellCache<V> {
    /// Hard bound on live entries.
    capacity: usize,
    /// Payloads by cell.
    map: HashMap<CellPos, V>,
    /// Recency order, oldest first; every key of `map` appears exactly once.
    order: VecDeque<CellPos>,
}

impl<V> CellCache<V> {
    /// Creates an empty cache holding at most `capacity` entries.
    pub fn new(capacity: usize) -> CellCache<V> {
        CellCache {
            capacity,
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    /// Returns the maximum number of live entries.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the current number of live entries (always `<= capacity`).
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` when no entry is stored.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Returns `true` when `key` is stored. Recency is unchanged.
    pub fn contains(&self, key: &CellPos) -> bool {
        self.map.contains_key(key)
    }

    /// Returns the payload for `key`, promoting it to most-recently-used.
    pub fn get(&mut self, key: &CellPos) -> Option<&V> {
        if self.map.contains_key(key) {
            self.touch(key);
            self.map.get(key)
        } else {
            None
        }
    }

    /// Returns the payload for `key` without touching recency.
    pub fn peek(&self, key: &CellPos) -> Option<&V> {
        self.map.get(key)
    }

    /// Stores `value` under `key`, evicting the least-recently-used entry
    /// when full. Re-inserting a live key replaces its payload and promotes
    /// it. With `capacity == 0` the value is dropped immediately.
    pub fn insert(&mut self, key: CellPos, value: V) {
        if self.map.contains_key(&key) {
            self.touch(&key);
        } else {
            if self.capacity == 0 {
                return;
            }
            if self.map.len() >= self.capacity && let Some(victim) = self.order.pop_front() {
                self.map.remove(&victim);
            }
            self.order.push_back(key);
        }
        self.map.insert(key, value);
    }

    /// Drops the entry for `key`, returning its payload when present.
    pub fn remove(&mut self, key: &CellPos) -> Option<V> {
        let value = self.map.remove(key)?;
        self.order.retain(|queued| queued != key);
        Some(value)
    }

    /// Drops every entry.
    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }

    /// Moves `key` to the most-recently-used end. Caller guarantees presence.
    fn touch(&mut self, key: &CellPos) {
        self.order.retain(|queued| queued != key);
        self.order.push_back(*key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::Level;

    /// Builds a test cell at `x` on L10.
    fn cell(x: i64) -> CellPos {
        CellPos::new(Level::new(10).unwrap_or(Level::MIN), x, 0, 0)
    }

    #[test]
    fn same_sequence_replays_same_contents() {
        let replay = || {
            let mut cache = CellCache::new(4);
            for x in 0..4 {
                cache.insert(cell(x), x * 10);
            }
            cache.get(&cell(1));
            cache.insert(cell(9), 90);
            (0..10).filter(|x| cache.contains(&cell(*x))).collect::<Vec<_>>()
        };
        assert_eq!(replay(), replay());
        assert_eq!(replay(), vec![1, 2, 3, 9]);
    }

    #[test]
    fn capacity_bounds_live_entries() {
        let mut cache = CellCache::new(3);
        for x in 0..10 {
            cache.insert(cell(x), x);
            assert!(cache.len() <= cache.capacity(), "bound violated");
        }
        assert_eq!(cache.len(), 3);
        // Insertion-order eviction: the three newest survive.
        assert!(!cache.contains(&cell(6)));
        assert!(cache.contains(&cell(7)));
        assert!(cache.contains(&cell(8)));
        assert!(cache.contains(&cell(9)));
    }

    #[test]
    fn reads_protect_entries_from_eviction() {
        let mut cache = CellCache::new(2);
        cache.insert(cell(0), "old");
        cache.insert(cell(1), "mid");
        assert_eq!(cache.get(&cell(0)), Some(&"old"));
        cache.insert(cell(2), "new");
        // `cell(0)` was refreshed, so `cell(1)` is evicted instead.
        assert!(cache.contains(&cell(0)));
        assert!(!cache.contains(&cell(1)));
        assert!(cache.contains(&cell(2)));
    }

    #[test]
    fn insert_remove_and_peek_keep_order_consistent() {
        let mut cache = CellCache::new(2);
        cache.insert(cell(0), 0);
        cache.insert(cell(0), 1);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.peek(&cell(0)), Some(&1));
        assert_eq!(cache.remove(&cell(0)), Some(1));
        assert!(cache.is_empty());
        assert_eq!(cache.remove(&cell(0)), None);
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn zero_capacity_stores_nothing() {
        let mut cache = CellCache::new(0);
        cache.insert(cell(0), 0);
        assert!(cache.is_empty());
        assert_eq!(cache.get(&cell(0)), None);
    }
}
