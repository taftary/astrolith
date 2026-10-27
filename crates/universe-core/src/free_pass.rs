//! Free flight passes through levels (#998 build T4, B3; Q4 A).
//!
//! Free flight used to move inside the open cell only. Now, after a free
//! step, flying close enough to a portal opens it (the same open gate as
//! the dive) and flying far enough out of the cell closes it (the same
//! close gate), with the exact frames the #998 study measured, so a
//! straight flight stays straight across both. From L1 to the star-system
//! space; the planet and the ground keep their own rules. Pure
//! (`E-CORE-NO-BEVY`, `E-DET-TIERS`).

use crate::r#gen::MarkerKind;
use crate::nav::{DiveEvent, MarkerIndex, Universe, length3, should_close, should_open};

/// Deepest level a free flight opens from: below the star-system space
/// (L8) bodies open by their gravity zones, not by angle.
pub const FREE_OPEN_DEEPEST: u8 = 7;

/// Deepest level a free flight closes from by the cell's close gate.
pub const FREE_CLOSE_DEEPEST: u8 = 8;

impl Universe {
    /// Opens or closes after a free-flight step (#998 B3): the portal with
    /// the largest angle past the open gate opens; otherwise the open cell
    /// closes past its close gate. At most one handover per step.
    pub fn free_pass(&mut self) -> DiveEvent {
        let level = self.level().get();
        if level <= FREE_OPEN_DEEPEST && self.path.can_open() {
            let camera = self.path.offset();
            let radius = self.marker_radius();
            let best = self
                .open
                .points
                .iter()
                .enumerate()
                .filter(|(_, p)| p.kind == MarkerKind::Portal)
                .map(|(i, p)| {
                    let at = p.portal_position();
                    (
                        i,
                        length3([at[0] - camera[0], at[1] - camera[1], at[2] - camera[2]]),
                    )
                })
                .filter(|(_, distance)| should_open(radius, *distance))
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((index, _)) = best
                && let Ok(marker) = u32::try_from(index)
                && self.open(MarkerIndex(marker))
            {
                return DiveEvent::Opened(marker);
            }
        }
        if level <= FREE_CLOSE_DEEPEST
            && !self.path.is_at_root()
            && should_close(self.path.distance_to_center())
            && let Some(opened) = self.close()
        {
            return DiveEvent::Closed(opened);
        }
        DiveEvent::Moved
    }
}

#[cfg(test)]
#[path = "free_pass_tests.rs"]
mod tests;
