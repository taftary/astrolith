//! Final destination of the camera: one room the view always aims at (#423).
//!
//! A [`Destination`](crate::destination::Destination) is the named marker chain from L1 down to the final
//! place: a room (L14 cell) through the seeded journey picks, or the
//! deepest marker that branch reaches. The open cell's named chain is
//! always a prefix of it, so in every dimension exactly one marker is "on
//! the way" ([`Destination::on_the_way`](crate::destination::Destination::on_the_way)), and the camera looks at it and
//! dives toward it, from the cosmic web down to the room and back out.
//!
//! The value is a fixed array plus a length, so it is `Copy`, stored in
//! saved views, and read per frame without allocating (`E-HOT-NOALLOC`).
//! Everything here is pure and deterministic (`E-DET-TIERS`).

use crate::coords::MAX_LEVEL;
use crate::r#gen::{Generated, MarkerKind};
use crate::nest::{
    MAX_OPEN_LEVEL, MarkerPath, autopilot_candidates, autopilot_marker, generate_cell, path_level,
    path_seed,
};

/// Most markers a destination holds: one per level, L1 to L14.
///
/// Thirteen open cells take the camera to an L14 room; the fourteenth slot
/// holds a final marker that never opens (a room's content), so a click
/// anywhere on the ladder has a place to aim.
pub const DESTINATION_SLOTS: usize = MAX_LEVEL as usize;

/// The camera's final destination (#423): a named marker chain from L1.
///
/// Markers before the last are cells the dive opens on the way down. The
/// last one is the final target: a room portal at L13, or the deepest
/// marker the branch reaches (a portal whose cell offers no portal, or a
/// marker that never opens, such as a population or a room's content).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Destination {
    markers: [u32; DESTINATION_SLOTS],
    len: u8,
}

/// Where the open cell stands relative to a [`Destination`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    /// The open cell is on the way; this marker in it leads on.
    Next(u32),
    /// The open cell is the destination itself: the room is reached.
    Arrived,
    /// The open cell is not on the way (the path left the chain).
    Off,
}

impl Destination {
    /// Destination holding exactly `chain`, or `None` when it is longer
    /// than [`DESTINATION_SLOTS`].
    #[must_use]
    pub fn from_chain(chain: &[u32]) -> Option<Destination> {
        if chain.len() > DESTINATION_SLOTS {
            return None;
        }
        let mut markers = [0u32; DESTINATION_SLOTS];
        for (slot, &marker) in markers.iter_mut().zip(chain) {
            *slot = marker;
        }
        let len = u8::try_from(chain.len()).ok()?;
        Some(Destination { markers, len })
    }

    /// The marker chain from L1 to the final target.
    #[must_use]
    pub fn markers(&self) -> &[u32] {
        self.markers
            .get(..usize::from(self.len))
            .unwrap_or(&self.markers)
    }

    /// The final target marker, or `None` for the root cell itself.
    #[must_use]
    pub fn final_marker(&self) -> Option<u32> {
        self.markers().last().copied()
    }

    /// Seeded destination below `prefix`: the Spacebar journey's picks.
    ///
    /// Follows [`autopilot_candidates`] and [`autopilot_marker`] from the
    /// cell at `prefix` down to L14, stopping early at a cell that offers
    /// no portal (the deepest marker reached is then the final target).
    /// With an empty prefix this is `autopilot_path(root)`, the start
    /// destination. A prefix longer than the slots is cut to fit.
    #[must_use]
    pub fn seeded(root: u64, prefix: &[u32]) -> Destination {
        descend(prefix, |chain| {
            let level = path_level(chain);
            let seed = path_seed(root, chain);
            let cell = generate_cell(root, chain);
            autopilot_marker(seed, &autopilot_candidates(level, seed, &cell.points))
        })
    }

    /// Where the open cell of `path` stands on the way to this destination.
    ///
    /// Reads the named chain in place (milestones skipped), so it runs
    /// every frame without allocating.
    #[must_use]
    pub fn on_the_way(&self, path: &MarkerPath) -> Way {
        let markers = self.markers();
        let mut depth = 0usize;
        for opened in path.chain().iter().filter(|opened| !opened.anonymous) {
            if markers.get(depth) != Some(&opened.marker) {
                return Way::Off;
            }
            depth += 1;
        }
        markers
            .get(depth)
            .map_or(Way::Arrived, |&marker| Way::Next(marker))
    }

    /// Destination after clicking `marker` in the open cell of `path`.
    ///
    /// Clicking the marker already on the way keeps the destination. A
    /// portal that can open leads to the seeded room below it; anything
    /// else (a population, a room's content, a marker past the deepest
    /// open level) becomes the final target itself.
    #[must_use]
    pub fn clicked(&self, root: u64, path: &MarkerPath, open: &Generated, marker: u32) -> Self {
        if self.on_the_way(path) == Way::Next(marker) {
            return *self;
        }
        let mut chain = path.indices();
        chain.push(marker);
        let opens = path.can_open()
            && open
                .points
                .get(marker as usize)
                .is_some_and(|point| point.kind == MarkerKind::Portal);
        if opens {
            return Destination::seeded(root, &chain);
        }
        Destination::from_chain(&chain).unwrap_or(*self)
    }

    /// This destination when the open cell is on its way, else the seeded
    /// one through the open cell (keeps the invariant if it ever broke).
    #[must_use]
    pub fn reconciled(&self, root: u64, path: &MarkerPath) -> Destination {
        if self.on_the_way(path) == Way::Off {
            Destination::seeded(root, &path.indices())
        } else {
            *self
        }
    }
}

/// Chain from `prefix` extended by `pick` until L14 or no pick (#423).
///
/// `pick` names the next marker inside the cell at the chain so far, or
/// `None` when that cell offers nothing to enter. Pure: the deepest-marker
/// rule is the `None` stop, so it is testable without a generator.
fn descend(prefix: &[u32], mut pick: impl FnMut(&[u32]) -> Option<u32>) -> Destination {
    let deepest = usize::from(MAX_OPEN_LEVEL - 1);
    let mut markers = [0u32; DESTINATION_SLOTS];
    let mut len = 0usize;
    for (slot, &marker) in markers.iter_mut().zip(prefix) {
        *slot = marker;
        len += 1;
    }
    while len < deepest {
        let Some(chain) = markers.get(..len) else {
            break;
        };
        let Some(next) = pick(chain) else {
            break;
        };
        let Some(slot) = markers.get_mut(len) else {
            break;
        };
        *slot = next;
        len += 1;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: len is at most DESTINATION_SLOTS (14), fits u8"
    )]
    let len = len as u8;
    Destination { markers, len }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::{DEMO_SEED, MarkerIndex, Universe};
    use crate::nest::autopilot_path;

    #[test]
    fn the_start_destination_is_the_spacebar_journeys_room() {
        let start = Destination::seeded(DEMO_SEED, &[]);
        assert_eq!(start.markers(), autopilot_path(DEMO_SEED).as_slice());
        assert_eq!(
            start.markers().len(),
            usize::from(MAX_OPEN_LEVEL - 1),
            "the journey reaches a room at L14"
        );
    }

    #[test]
    fn the_root_is_on_the_way_and_names_the_first_marker() {
        let universe = Universe::new(DEMO_SEED);
        let start = Destination::seeded(DEMO_SEED, &[]);
        assert_eq!(
            start.on_the_way(&universe.path),
            Way::Next(autopilot_path(DEMO_SEED)[0])
        );
    }

    #[test]
    fn opening_the_way_walks_it_to_arrival_and_closing_walks_back() {
        let mut universe = Universe::new(DEMO_SEED);
        let start = Destination::seeded(DEMO_SEED, &[]);
        for &marker in start.markers() {
            assert_eq!(start.on_the_way(&universe.path), Way::Next(marker));
            assert!(universe.open(MarkerIndex(marker)), "way marker opens");
        }
        assert_eq!(start.on_the_way(&universe.path), Way::Arrived);
        for &marker in start.markers().iter().rev() {
            let closed = universe.close().expect("the way closes back");
            assert_eq!(closed.marker, marker);
            assert_eq!(
                start.on_the_way(&universe.path),
                Way::Next(marker),
                "after a close the marker on the way is the one just left"
            );
        }
    }

    #[test]
    fn milestones_do_not_count_as_steps_on_the_way() {
        let mut universe = Universe::new(DEMO_SEED);
        let start = Destination::seeded(DEMO_SEED, &[]);
        let first = start.markers()[0];
        universe.path.open_anonymous(first);
        assert_eq!(start.on_the_way(&universe.path), Way::Next(first));
    }

    #[test]
    fn a_cell_off_the_way_reads_off_and_reconciles_through_it() {
        let mut universe = Universe::new(DEMO_SEED);
        let start = Destination::seeded(DEMO_SEED, &[]);
        let other = universe
            .open
            .points
            .iter()
            .enumerate()
            .filter(|(_, point)| point.kind == MarkerKind::Portal)
            .map(|(index, _)| u32::try_from(index).expect("budgeted cell"))
            .find(|&marker| marker != start.markers()[0])
            .expect("root offers another portal");
        assert!(universe.open(MarkerIndex(other)));
        assert_eq!(start.on_the_way(&universe.path), Way::Off);
        let mended = start.reconciled(DEMO_SEED, &universe.path);
        assert_eq!(mended, Destination::seeded(DEMO_SEED, &[other]));
        assert_eq!(
            mended.on_the_way(&universe.path),
            Way::Next(mended.markers()[1])
        );
    }

    #[test]
    fn clicking_the_marker_on_the_way_keeps_the_destination() {
        let universe = Universe::new(DEMO_SEED);
        let start = Destination::seeded(DEMO_SEED, &[]);
        let Way::Next(on_way) = start.on_the_way(&universe.path) else {
            panic!("root is on the way");
        };
        let after = start.clicked(DEMO_SEED, &universe.path, &universe.open, on_way);
        assert_eq!(after, start);
    }

    #[test]
    fn clicking_another_portal_gives_its_seeded_room_every_time() {
        let universe = Universe::new(DEMO_SEED);
        let start = Destination::seeded(DEMO_SEED, &[]);
        let other = universe
            .open
            .points
            .iter()
            .enumerate()
            .filter(|(_, point)| point.kind == MarkerKind::Portal)
            .map(|(index, _)| u32::try_from(index).expect("budgeted cell"))
            .find(|&marker| marker != start.markers()[0])
            .expect("root offers another portal");
        let once = start.clicked(DEMO_SEED, &universe.path, &universe.open, other);
        let twice = start.clicked(DEMO_SEED, &universe.path, &universe.open, other);
        assert_eq!(once, twice, "the same click gives the same room");
        assert_eq!(once.markers()[0], other, "the way runs through the click");
        assert_eq!(once, Destination::seeded(DEMO_SEED, &[other]));
        assert_eq!(once.on_the_way(&universe.path), Way::Next(other));
    }

    #[test]
    fn clicking_a_marker_that_never_opens_makes_it_the_final_target() {
        let mut universe = Universe::new(DEMO_SEED);
        let start = Destination::seeded(DEMO_SEED, &[]);
        // Walk the journey to the room: its content never opens.
        for &marker in start.markers() {
            assert!(universe.open(MarkerIndex(marker)));
        }
        assert!(
            !universe.path.can_open(),
            "test premise: the room is terminal"
        );
        let furniture = 0u32;
        assert!(
            !universe.open.points.is_empty(),
            "test premise: the room has content"
        );
        let after = start.clicked(DEMO_SEED, &universe.path, &universe.open, furniture);
        assert_eq!(after.final_marker(), Some(furniture));
        assert_eq!(&after.markers()[..start.markers().len()], start.markers());
        assert_eq!(after.on_the_way(&universe.path), Way::Next(furniture));
    }

    #[test]
    fn a_branch_without_portals_ends_at_its_deepest_marker() {
        // Three levels offer a pick, the fourth offers none: the chain
        // stops there, so the deepest marker reached is the final target.
        let ended = descend(&[7], |chain| (chain.len() < 4).then_some(2));
        assert_eq!(ended.markers(), &[7, 2, 2, 2]);
        assert_eq!(ended.final_marker(), Some(2));
    }

    #[test]
    fn a_descent_never_runs_past_the_room() {
        let ended = descend(&[], |_| Some(1));
        assert_eq!(ended.markers().len(), usize::from(MAX_OPEN_LEVEL - 1));
    }

    #[test]
    fn a_chain_longer_than_the_slots_is_refused() {
        assert!(Destination::from_chain(&[0; DESTINATION_SLOTS]).is_some());
        assert!(Destination::from_chain(&[0; DESTINATION_SLOTS + 1]).is_none());
    }
}
