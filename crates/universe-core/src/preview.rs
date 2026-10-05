//! Pre-entry previews: interiors drawn before entry (R7, #63; #152 streaming).
//!
//! The preview cache holds the largest-on-screen markers' interiors with an
//! allocation-free steady state, plus a drain-in-place merge for background
//! generations built off the frame thread. The preview-set computation lives
//! here too. Everything here is pure and headless.

use crate::coords::{OpenUnits, ParentUnits};
use crate::r#gen::{Generated, MarkerKind, Point};
use crate::nav::{MarkerIndex, Universe};
use crate::nest::{
    Opened, PREVIEW_ANGLE, PREVIEW_CAP, angular_radius, child_ratio, child_world_position,
    generate_cell, length,
};

/// Interiors of the open cell's largest-on-screen markers, drawn before entry
/// (R7, #63).
///
/// Holds at most [`PREVIEW_CAP`] generations, keyed
/// by the open path and the marker index. [`PreviewCache::sync`] regenerates
/// only when the preview set changes, so the draw path never allocates per
/// frame (section 8 bounded memory: `2 + PREVIEW_CAP` generations alive at
/// most). A static camera neither regenerates nor reallocates.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PreviewCache {
    path: Vec<u32>,
    entries: Vec<(u32, Generated)>,
    /// Total cell generations performed by this cache (test/verify counter).
    pub regenerations: u64,
    /// Reused preview-set buffer: steady-state syncs allocate nothing.
    wanted: Vec<u32>,
    /// Camera offset of the last sync; `None` before the first sync.
    last_offset: Option<[f64; 3]>,
}

/// Compares stored marker indices against the live path without allocating.
///
/// Magnification milestones are skipped on both sides: only named opens
/// identify a cell, so milestones never invalidate the cache.
fn path_matches(stored: &[u32], chain: &[Opened]) -> bool {
    let mut stored_iter = stored.iter();
    for opened in chain.iter().filter(|opened| !opened.anonymous) {
        match stored_iter.next() {
            Some(marker) if *marker == opened.marker => {}
            _ => return false,
        }
    }
    stored_iter.next().is_none()
}

impl PreviewCache {
    /// Brings the cache in line with the camera: returns the current preview
    /// set and `true` when any cell was (re)generated.
    ///
    /// Allocation-free for a static camera: an unchanged path and offset
    /// return early, and the wanted set reuses a buffer otherwise.
    pub fn sync(&mut self, universe: &Universe) -> bool {
        let offset = universe.path.offset();
        if self.last_offset == Some(offset) && path_matches(&self.path, universe.path.chain()) {
            return false;
        }
        self.last_offset = Some(offset);
        if !path_matches(&self.path, universe.path.chain()) {
            self.path = universe.path.indices();
            self.entries.clear();
        }
        self.wanted.clear();
        if universe.path.can_open() {
            append_preview_set(
                &mut self.wanted,
                offset,
                &universe.open.points,
                universe.marker_radius(),
            );
        }
        let before = self.entries.len();
        self.entries
            .retain(|(marker, _)| self.wanted.contains(marker));
        let mut changed = self.entries.len() != before;
        for marker in self.wanted.iter().copied() {
            if self.entries.iter().any(|(m, _)| *m == marker) {
                continue;
            }
            let mut chain = self.path.clone();
            chain.push(marker);
            self.entries
                .push((marker, generate_cell(universe.root.0, &chain)));
            self.regenerations += 1;
            changed = true;
        }
        if changed {
            self.entries.sort_by_key(|(marker, _)| *marker);
        }
        changed
    }

    /// Merges background generations into the cache (#152).
    ///
    /// The window streamer generates previews off the frame thread and
    /// merges completions here in marker order, so the merged set equals
    /// what [`PreviewCache::sync`] would have built inline. Completions from
    /// a moved-on path are dropped (the next frame re-spawns for the new
    /// path); each insert counts as a regeneration. Returns true when the
    /// set changed. `cells` drains in place, keeping its buffer.
    pub fn merge(
        &mut self,
        universe: &Universe,
        wanted: &[u32],
        cells: &mut Vec<(u32, Generated)>,
    ) -> bool {
        self.last_offset = Some(universe.path.offset());
        if !path_matches(&self.path, universe.path.chain()) {
            self.path = universe.path.indices();
            self.entries.clear();
        }
        let before = self.entries.len();
        self.entries.retain(|(marker, _)| wanted.contains(marker));
        let mut changed = self.entries.len() != before;
        cells.sort_by_key(|(marker, _)| *marker);
        for (marker, cell) in cells.drain(..) {
            if !wanted.contains(&marker) {
                continue;
            }
            if self.entries.iter().any(|(held, _)| *held == marker) {
                continue;
            }
            self.entries.push((marker, cell));
            self.regenerations += 1;
            changed = true;
        }
        if changed {
            self.entries.sort_by_key(|(marker, _)| *marker);
        }
        changed
    }

    /// Number of previewed markers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` when nothing is previewed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Previewed content of `marker`, if cached.
    #[must_use]
    pub fn get(&self, marker: u32) -> Option<&Generated> {
        self.entries
            .iter()
            .find(|(m, _)| *m == marker)
            .map(|(_, generated)| generated)
    }

    /// Previewed markers with their content, by marker index.
    #[must_use]
    pub fn entries(&self) -> &[(u32, Generated)] {
        &self.entries
    }
}

/// Markers large enough on screen to preview their interior.
///
/// Returns at most [`PREVIEW_CAP`] indices of markers whose angular radius
/// from `camera` exceeds [`PREVIEW_ANGLE`], largest first, ties by index.
/// Pure and deterministic.
#[must_use]
pub fn preview_set(camera: [f64; 3], markers: &[Point], radius: f64) -> Vec<u32> {
    let mut out = Vec::new();
    append_preview_set(&mut out, camera, markers, radius);
    out
}

/// Appends [`preview_set`] to `into`, reusing its buffer.
///
/// `PreviewCache::sync` keeps one buffer across frames so a static camera
/// allocates nothing per frame (E-HOT-NOALLOC); the window background
/// streamer (#152) reuses its own buffer the same way.
pub fn append_preview_set(into: &mut Vec<u32>, camera: [f64; 3], markers: &[Point], radius: f64) {
    let mut candidates: Vec<(f64, u32)> = markers
        .iter()
        .enumerate()
        .filter(|(_, point)| point.kind == MarkerKind::Portal)
        .filter_map(|(index, point)| {
            let distance = length([
                camera[0] - point.position[0],
                camera[1] - point.position[1],
                camera[2] - point.position[2],
            ]);
            let angular = angular_radius(radius, distance);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: marker index into a budgeted cell, always fits u32"
            )]
            let index_u32 = index as u32;
            (angular > PREVIEW_ANGLE).then_some((angular, index_u32))
        })
        .collect();
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    candidates.truncate(PREVIEW_CAP);
    into.extend(candidates.into_iter().map(|(_, index)| index));
}

/// World positions (open-cell units) of `marker`'s previewed children.
///
/// Empty when the marker is missing or the level cannot open.
#[must_use]
pub fn preview_positions(
    universe: &Universe,
    marker: MarkerIndex,
    content: &Generated,
) -> Vec<OpenUnits> {
    let (Some(marker_pos), Some(ratio)) = (universe.marker(marker), child_ratio(universe.level()))
    else {
        return Vec::new();
    };
    content
        .points
        .iter()
        .map(|point| {
            OpenUnits(child_world_position(ParentUnits(marker_pos), ratio, point.position).0)
        })
        .collect()
}

/// Largest relative error between `previewed` (pre-open, parent units,
/// re-expressed through `opened`) and the open cell's marker positions.
#[must_use]
pub fn preview_error(previewed: &[OpenUnits], opened: Opened, open: &Generated) -> f64 {
    let mut worst = 0.0f64;
    if previewed.len() != open.points.len() {
        return f64::INFINITY;
    }
    for (world, point) in previewed.iter().zip(&open.points) {
        for ((w, anchor), want) in world.0.iter().zip(opened.position).zip(point.position) {
            let back = (w - anchor) / opened.ratio;
            worst = worst.max((back - want).abs() / want.abs().max(1.0));
        }
    }
    worst
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::ParentUnits;
    use crate::nav::{DEMO_SEED, DiveEvent, DiveMode, MarkerIndex, Universe, WHEEL_FACTOR};
    use crate::nest::{
        MarkerPath, autopilot_path, child_ratio, child_world_position, generate_cell,
        marker_position,
    };

    #[test]
    fn preview_cache_regenerates_only_when_the_set_changes() {
        let mut universe = Universe::new(DEMO_SEED);
        let mut cache = PreviewCache::default();
        assert!(!cache.sync(&universe), "far away nothing previews");
        assert!(cache.is_empty());
        let marker = universe.autopilot_target().expect("marker");
        let target = Some(marker);
        let mut synced_once = false;
        for _ in 0..500 {
            cache.sync(&universe);
            if !cache.is_empty() && !synced_once {
                synced_once = true;
                let before = cache.regenerations;
                assert!(!cache.sync(&universe), "same camera must not regenerate");
                assert_eq!(cache.regenerations, before);
                assert!(cache.len() <= PREVIEW_CAP);
            }
            if universe.dive(target, WHEEL_FACTOR, 0.0, DiveMode::Targeted) != DiveEvent::Moved {
                break;
            }
        }
        assert!(synced_once, "target never entered the preview set");
        assert_eq!(universe.level().get(), 2);
        cache.sync(&universe);
        assert!(
            cache
                .entries()
                .iter()
                .all(|(m, _)| universe.marker(MarkerIndex(*m)).is_some())
        );
    }

    #[test]
    fn milestones_never_invalidate_previews() {
        let mut universe = Universe::new(DEMO_SEED);
        let mut cache = PreviewCache::default();
        cache.sync(&universe);
        let before = cache.regenerations;
        // Milestones pushed and popped change no named path: no regeneration.
        universe.path.open_anonymous(0);
        universe.path.open_anonymous(0);
        assert!(!cache.sync(&universe), "milestones must not resync");
        assert_eq!(cache.regenerations, before);
        universe.path.close();
        universe.path.close();
        assert!(!cache.sync(&universe), "unwind must not resync");
        assert_eq!(cache.regenerations, before);
        assert!(universe.path.is_at_root());
    }

    #[test]
    fn sync_is_free_for_a_static_camera() {
        let mut universe = Universe::new(DEMO_SEED);
        let mut cache = PreviewCache::default();
        let marker = universe.autopilot_target().expect("marker");
        let target = Some(marker);
        for _ in 0..500 {
            cache.sync(&universe);
            if !cache.is_empty() {
                break;
            }
            if universe.dive(target, WHEEL_FACTOR, 0.0, DiveMode::Targeted) != DiveEvent::Moved {
                break;
            }
        }
        assert!(!cache.is_empty(), "nothing ever previewed");
        assert!(!cache.wanted.is_empty(), "wanted buffer never filled");
        let regens = cache.regenerations;
        let entries_ptr = cache.entries.as_ptr();
        let entries_cap = cache.entries.capacity();
        let wanted_ptr = cache.wanted.as_ptr();
        let wanted_cap = cache.wanted.capacity();
        for _ in 0..10 {
            assert!(!cache.sync(&universe), "static camera must not change");
        }
        assert_eq!(cache.regenerations, regens, "static camera regenerated");
        assert_eq!(cache.entries.as_ptr(), entries_ptr, "entries reallocated");
        assert_eq!(cache.entries.capacity(), entries_cap);
        assert_eq!(
            cache.wanted.as_ptr(),
            wanted_ptr,
            "wanted buffer reallocated"
        );
        assert_eq!(cache.wanted.capacity(), wanted_cap);
    }

    #[test]
    fn preview_positions_match_open_cell_exactly() {
        let chain = autopilot_path(42);
        let mut path = MarkerPath::root([1.4, 1.0, 1.4]);
        for &marker in &chain {
            let parent = generate_cell(42, &path.indices());
            let marker_pos = marker_position(&parent, marker).expect("marker");
            let ratio = child_ratio(path.level()).expect("ratio");
            let mut child_chain = path.indices();
            child_chain.push(marker);
            let child = generate_cell(42, &child_chain);
            let previewed: Vec<ParentUnits> = child
                .points
                .iter()
                .map(|p| child_world_position(ParentUnits(marker_pos), ratio, p.position))
                .collect();
            assert!(path.open(marker, marker_pos));
            let open = generate_cell(42, &path.indices());
            assert_eq!(open, child, "open cell must be the previewed content");
            for (world, point) in previewed.iter().zip(&open.points) {
                for ((w, anchor), want) in world.0.iter().zip(marker_pos).zip(point.position) {
                    let back = (*w - anchor) / ratio;
                    assert!(
                        (back - want).abs() <= 1e-9 * want.abs().max(1.0),
                        "preview drifted at L{}: {back} vs {want}",
                        path.level().get()
                    );
                }
            }
        }
    }

    #[test]
    fn preview_set_is_capped_sorted_and_thresholded() {
        let portal = |x: f64, y: f64, z: f64| Point {
            position: [x, y, z],
            radius: 0.02,
            kind: MarkerKind::Portal,
            galaxy: None, star: None, planet: None, cloud: None,
        };
        let population = |x: f64, y: f64, z: f64| Point {
            position: [x, y, z],
            radius: 0.008,
            kind: MarkerKind::Population,
            galaxy: None, star: None, planet: None, cloud: None,
        };
        // Camera at z=0.6; eight portals above the angle, one
        // nearer population (kind-excluded), one far portal (angle-excluded).
        let points = vec![
            portal(0.0, 0.0, 0.5),
            population(0.0, 0.0, 0.55),
            portal(0.0, 0.0, 0.45),
            portal(0.0, 0.0, 0.4),
            portal(0.0, 0.0, 0.35),
            portal(0.0, 0.0, 0.3),
            portal(0.0, 0.0, 0.2),
            portal(0.0, 0.0, 0.1),
            portal(0.0, 0.0, 0.0),
            population(0.0, 0.0, -0.5),
            portal(0.3, 0.0, -0.5),
        ];
        let near = preview_set([0.0, 0.0, 0.6], &points, 0.02);
        assert_eq!(
            near,
            vec![0, 2, 3, 4, 5, 6],
            "largest-first portals to the cap"
        );
        let far = preview_set([0.0, 0.0, 50.0], &points, 0.02);
        assert!(far.is_empty(), "nothing previews from far away");
        let mut previous = f64::INFINITY;
        for &index in &near {
            let p = points[index as usize].position;
            let d = ((p[0]).powi(2) + (p[1]).powi(2) + (p[2] - 0.6).powi(2)).sqrt();
            let angular = angular_radius(0.02, d);
            assert!(angular > PREVIEW_ANGLE);
            assert!(angular <= previous, "preview set not largest-first");
            previous = angular;
        }
        assert_eq!(near, preview_set([0.0, 0.0, 0.6], &points, 0.02));
    }

    #[test]
    fn preview_set_excludes_populations() {
        let points = vec![
            Point {
                position: [0.0, 0.0, 0.55],
                radius: 0.008,
                kind: MarkerKind::Population,
                galaxy: None, star: None, planet: None, cloud: None,
            },
            Point {
                position: [0.0, 0.0, 0.45],
                radius: 0.02,
                kind: MarkerKind::Portal,
                galaxy: None, star: None, planet: None, cloud: None,
            },
        ];
        let near = preview_set([0.0, 0.0, 0.6], &points, 0.02);
        assert_eq!(near, vec![1], "only the portal previews");
    }
}
