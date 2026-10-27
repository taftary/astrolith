//! Continuous scaling study through L1-L7, crowds and budget (#998 T2,
//! study).
//!
//! A continuous zoom keeps the levels around the camera alive at once:
//! the open cell, its parent (the context you came from), and the previews
//! of the markers large enough to show their insides
//! ([`crate::preview::PreviewCache`], [`crate::stream::StreamLedger`] with
//! its cap of [`crate::stream::STREAM_CAP`] cells). Along the journey in,
//! this counts per frame the live cells, the points they hold, and the
//! points large enough to draw (one pixel or more at 1600 x 900 and the
//! game's field of view), and the cells newly generated that frame. The
//! window's fade-in by size (Q3 A) draws only the visible points; the
//! budget question is whether they stay under a few thousand and whether
//! generation stays within a frame. Timing is left to the caller (the
//! probe test), so this stays pure (`E-CORE-NO-BEVY`, `E-DET-TIERS`).

use crate::frame::angular_radius;
use crate::nav::{AUTOPILOT_RATE, DEMO_SEED, DiveEvent, DiveMode, Universe};
use crate::preview::{PreviewCache, preview_positions};
use crate::stream::{StreamLedger, working_chains};

/// Starting budget of live points (realism review, section 7).
pub const POINT_BUDGET: usize = 2_000;

/// One pixel's angle at 1600 x 900 with the game's vertical field of view,
/// radians.
#[must_use]
pub fn pixel_rad() -> f64 {
    crate::precision::FOV_DEG.to_radians() / 900.0
}

/// One level's worst frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Crowd {
    /// Level.
    pub level: u8,
    /// Most live cells in one frame.
    pub cells: usize,
    /// Most points held by the live cells in one frame.
    pub points: usize,
    /// Most points of one pixel or more in one frame.
    pub visible: usize,
    /// Most cells newly generated in one frame.
    pub loads: usize,
    /// Frames on this level.
    pub frames: usize,
}

fn length3(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Whether a point of `radius` at `distance` covers one pixel or more.
fn visible(radius: f64, distance: f64) -> bool {
    angular_radius(radius, distance) >= pixel_rad()
}

/// The study's state along the journey: the universe, its previews and the
/// live-cell ledger.
#[derive(Debug, Clone)]
pub struct CrowdWalk {
    /// The universe flown.
    pub universe: Universe,
    previews: PreviewCache,
    ledger: StreamLedger,
    target: Option<u32>,
    factor: f64,
}

impl Default for CrowdWalk {
    fn default() -> Self {
        Self::new()
    }
}

impl CrowdWalk {
    /// At L1, the journey not started.
    #[must_use]
    pub fn new() -> Self {
        Self {
            universe: Universe::new(DEMO_SEED),
            previews: PreviewCache::default(),
            ledger: StreamLedger::default(),
            target: None,
            factor: (-AUTOPILOT_RATE / 60.0).exp(),
        }
    }

    /// Streams this frame's cells (the generation work) and returns the
    /// level, the live cells, the cells newly loaded and the ledger's
    /// unloads. The caller times this call.
    pub fn stream(&mut self) -> (u8, usize, usize) {
        let before = self.previews.len();
        self.previews.sync(&self.universe);
        let chains = working_chains(
            &self.universe.path,
            self.universe.path.offset(),
            &self.universe.open.points,
            &self.previews,
        );
        let live_before = self.ledger.len();
        self.ledger.sync(DEMO_SEED, &chains);
        let loads = self.previews.len().saturating_sub(before)
            + self.ledger.len().saturating_sub(live_before);
        (self.universe.level().get(), self.ledger.len(), loads)
    }

    /// Counts the points the live cells hold and those of one pixel or
    /// more: the open cell's, its parent's (from the camera in parent
    /// units) and the previews'. The caller times this call too.
    #[must_use]
    pub fn count(&self) -> (usize, usize) {
        let universe = &self.universe;
        let camera = universe.path.offset();
        let radius = universe.marker_radius();
        let mut points = universe.open.points.len();
        let mut shown = universe
            .open
            .points
            .iter()
            .filter(|p| {
                visible(
                    p.radius.max(radius * 0.25),
                    length3(sub3(p.position, camera)),
                )
            })
            .count();
        if let (Some(parent), Some(opened)) = (universe.parent.as_ref(), universe.path.entered()) {
            let in_parent = opened.to_parent_point(camera);
            points += parent.points.len();
            shown += parent
                .points
                .iter()
                .filter(|p| {
                    let r = p.radius.max(radius * opened.ratio * 0.25);
                    visible(r, length3(sub3(p.position, in_parent)))
                })
                .count();
        }
        let child_ratio = crate::nest::child_ratio(universe.level()).unwrap_or(0.0);
        for (marker, content) in self.previews.entries() {
            points += content.points.len();
            let positions = preview_positions(universe, crate::nav::MarkerIndex(*marker), content);
            shown += content
                .points
                .iter()
                .zip(&positions)
                .filter(|(p, at)| {
                    let r = (p.radius * child_ratio).max(radius * child_ratio * 0.25);
                    visible(r, length3(sub3(at.0, camera)))
                })
                .count();
        }
        (points, shown)
    }

    /// One journey frame; `false` once L7 is reached or the journey stops.
    pub fn step(&mut self) -> bool {
        if self.universe.level().get() >= crate::headless_scale::STUDY_DEEPEST {
            return false;
        }
        if self.target.is_none() {
            self.target = self.universe.journey_target();
            if self.target.is_none() {
                return false;
            }
        }
        match self
            .universe
            .dive(self.target, self.factor, DiveMode::Journey)
        {
            DiveEvent::Opened(_) => self.target = None,
            DiveEvent::Closed(opened) => self.target = Some(opened.marker),
            DiveEvent::Moved => {}
        }
        true
    }
}

/// The crowds along the journey in, per level, worst frame each.
#[must_use]
pub fn crowds() -> Vec<Crowd> {
    let mut walk = CrowdWalk::new();
    let mut rows: Vec<Crowd> = Vec::new();
    let mut guard = 0;
    loop {
        guard += 1;
        if guard > 200_000 {
            break;
        }
        let (level, cells, loads) = walk.stream();
        let (points, shown) = walk.count();
        if !rows.iter().any(|r| r.level == level) {
            rows.push(Crowd {
                level,
                ..Crowd::default()
            });
        }
        let Some(row) = rows.iter_mut().find(|r| r.level == level) else {
            break;
        };
        row.cells = row.cells.max(cells);
        row.points = row.points.max(points);
        row.visible = row.visible.max(shown);
        row.loads = row.loads.max(loads);
        row.frames += 1;
        if !walk.step() {
            break;
        }
    }
    rows
}

/// The `SCALE-CROWD` lines.
#[must_use]
pub fn lines() -> Vec<String> {
    crowds()
        .iter()
        .map(|c| {
            format!(
                "SCALE-CROWD level=L{} cells_max={} points_max={} visible_max={} loads_max={} frames={}",
                c.level, c.cells, c.points, c.visible, c.loads, c.frames
            )
        })
        .collect()
}

#[cfg(test)]
#[path = "headless_scale_crowd_tests.rs"]
mod tests;
