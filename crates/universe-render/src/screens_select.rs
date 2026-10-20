//! Shot groups, the smoke set and the ready-early rule for `--screens`
//! (#847 Q1 A), split from `screens` under `E-FILE-SIZE`.
//!
//! Every shot belongs to one group by what it shows: walking (a player
//! shot), sky (a sky view), planet (a dive to L9 or L10), menus
//! (everything else). A run takes the whole list by default, or only the
//! chosen groups, or the smoke set: one shot from each group. Planet, sky
//! and walking shots are taken as soon as the ground is built (and the
//! body has landed) after [`READY_MIN_FRAMES`], not after their full
//! settle time, which stays the cap.

use crate::screens::{SHOTS, Shot};
use bevy::prelude::*;

/// Frames a ground shot always waits before it may be taken early: the
/// atmosphere and the first patches appear within a few frames.
pub(crate) const READY_MIN_FRAMES: u32 = 60;

/// One shot from each group, in list order: the smoke set.
pub const SMOKE: [&str; 4] = [
    "menu.png",
    "planet-l10.png",
    "player-third.png",
    "sky-ground.png",
];

/// What a shot shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotGroup {
    /// Menus, settings, guides and the touch overlay.
    Menus,
    /// The dive's planet views (L9 approach, L10).
    Planet,
    /// The sky shots under the real sun.
    Sky,
    /// The player's figure on foot.
    Walking,
}

impl ShotGroup {
    /// Every group, in the order of the list.
    pub const ALL: [ShotGroup; 4] = [
        ShotGroup::Menus,
        ShotGroup::Planet,
        ShotGroup::Sky,
        ShotGroup::Walking,
    ];

    /// The group named `id` on the command line, if any.
    #[must_use]
    pub fn from_id(id: &str) -> Option<ShotGroup> {
        match id {
            "menus" => Some(ShotGroup::Menus),
            "planet" => Some(ShotGroup::Planet),
            "sky" => Some(ShotGroup::Sky),
            "walking" => Some(ShotGroup::Walking),
            _ => None,
        }
    }
}

/// The group of `shot`.
#[must_use]
pub(crate) const fn group_of(shot: &Shot) -> ShotGroup {
    if shot.player.is_some() {
        ShotGroup::Walking
    } else if shot.sky.is_some() {
        ShotGroup::Sky
    } else if shot.dive >= 8 {
        ShotGroup::Planet
    } else {
        ShotGroup::Menus
    }
}

/// Which shots a run takes: every shot when empty, else the chosen groups
/// and, with `smoke`, the smoke set.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct ShotSelection {
    /// Chosen groups.
    pub groups: Vec<ShotGroup>,
    /// Whether the smoke set is chosen.
    pub smoke: bool,
}

impl ShotSelection {
    /// Reads a comma list of `smoke`, `menus`, `planet`, `sky` and
    /// `walking`; `None` on any other word or an empty list.
    #[must_use]
    pub fn parse(spec: &str) -> Option<ShotSelection> {
        let mut selection = ShotSelection::default();
        for word in spec.split(',').map(str::trim) {
            if word == "smoke" {
                selection.smoke = true;
            } else {
                let group = ShotGroup::from_id(word)?;
                if !selection.groups.contains(&group) {
                    selection.groups.push(group);
                }
            }
        }
        (selection.smoke || !selection.groups.is_empty()).then_some(selection)
    }

    /// Whether the run takes `shot`.
    #[must_use]
    pub(crate) fn takes(&self, shot: &Shot) -> bool {
        if self.groups.is_empty() && !self.smoke {
            return true;
        }
        self.groups.contains(&group_of(shot)) || (self.smoke && SMOKE.contains(&shot.file))
    }
}

/// Number of shots a run with `selection` takes.
#[must_use]
pub fn shot_count_for(selection: &ShotSelection) -> usize {
    SHOTS.iter().filter(|shot| selection.takes(shot)).count()
}

/// Whether a ground shot may be taken now: past [`READY_MIN_FRAMES`], the
/// drawn ground built, and (for a player shot) the body landed on it.
#[must_use]
pub(crate) const fn ready_early(
    shot: &Shot,
    waited: u32,
    ground_idle: bool,
    body_pending: bool,
) -> bool {
    let ground_shot = shot.dive >= 9 || shot.player.is_some();
    ground_shot
        && waited >= READY_MIN_FRAMES
        && ground_idle
        && !(shot.player.is_some() && body_pending)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screens_run_the_chosen_groups() {
        // #847 AC1: every group has shots; the default takes them all; a
        // group choice takes only its shots; the smoke set takes one shot
        // from each group; unknown words are refused.
        for group in ShotGroup::ALL {
            assert!(
                SHOTS.iter().any(|shot| group_of(shot) == group),
                "{group:?} has shots"
            );
        }
        assert_eq!(shot_count_for(&ShotSelection::default()), SHOTS.len());
        let sky = ShotSelection::parse("sky").expect("sky");
        assert!(
            SHOTS
                .iter()
                .filter(|s| sky.takes(s))
                .all(|s| s.sky.is_some())
        );
        assert_eq!(shot_count_for(&sky), 7);
        let smoke = ShotSelection::parse("smoke").expect("smoke");
        assert_eq!(shot_count_for(&smoke), SMOKE.len());
        let groups: Vec<ShotGroup> = SHOTS
            .iter()
            .filter(|s| smoke.takes(s))
            .map(group_of)
            .collect();
        for group in ShotGroup::ALL {
            assert!(groups.contains(&group), "smoke covers {group:?}");
        }
        for file in SMOKE {
            assert!(SHOTS.iter().any(|s| s.file == file), "{file} is a shot");
        }
        let both = ShotSelection::parse("planet, walking").expect("two groups");
        assert_eq!(both.groups, vec![ShotGroup::Planet, ShotGroup::Walking]);
        assert_eq!(ShotSelection::parse("ground"), None);
        assert_eq!(ShotSelection::parse(""), None);
    }

    #[test]
    fn ground_shots_wait_for_the_ground() {
        // A ground shot is taken early only past the minimum wait, with the
        // ground built and, on foot, the body landed; menus never early.
        let planet = SHOTS
            .iter()
            .find(|s| s.file == "planet-l10.png")
            .expect("shot");
        let walking = SHOTS
            .iter()
            .find(|s| s.file == "player-third.png")
            .expect("shot");
        let menu = SHOTS.iter().find(|s| s.file == "menu.png").expect("shot");
        assert!(ready_early(planet, READY_MIN_FRAMES, true, false));
        assert!(!ready_early(planet, READY_MIN_FRAMES - 1, true, false));
        assert!(!ready_early(planet, READY_MIN_FRAMES, false, false));
        assert!(!ready_early(walking, READY_MIN_FRAMES, true, true));
        assert!(ready_early(walking, READY_MIN_FRAMES, true, false));
        assert!(!ready_early(menu, 1_000, true, false));
    }
}
