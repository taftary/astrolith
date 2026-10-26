//! Moving the star system with the sky's clock (#981 T2).
//!
//! Planets go around their star in the planetary system (L8) and the star
//! close-up (L9), on their true orbits in the ecliptic (`XZ`) plane; moons
//! go around their planet in the open planet (L10), in the plane they were
//! generated in (`XY`). Each body's place is its generated angle plus one
//! turn per period ([`crate::clock::orbit_angle_at`]), rewritten in place.
//! An open planet's entry follows its planet, so a camera inside the
//! planet's cell rides along; a camera in the system stays still while the
//! bodies pass, and a gravity zone that reaches it opens the planet
//! ([`Universe::sync_zones`]).
//!
//! The open planet spins (#981 T3): its entry turns by the spin angle
//! ([`crate::clock::spin_angle_at`]) about the pole, so the ground stays
//! put in the planet's cell while the star, the planets and the moons
//! turn in its sky, and the light follows.
//!
//! Pure in time (`E-DET-TIERS`); the mover list is rebuilt only when a cell
//! regenerates (`E-HOT-NOALLOC`).

use crate::clock::{orbit_angle_at, spin_angle_at};
use crate::frame::turn_y;
use crate::r#gen::Point;
use crate::nav::{DiveEvent, Universe, l8_planet_zone, length3, sub3};

/// Which cell a mover lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cell {
    /// The open cell.
    Open,
    /// The parent cell.
    Parent,
}

/// One moving body: where it lives and the orbit it keeps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mover {
    cell: Cell,
    index: usize,
    /// Orbit radius in that cell's units.
    radius: f64,
    /// Angle at game time zero.
    base: f64,
    /// Orbital period in days.
    period_days: f64,
    /// Planets orbit in `XZ` around the star, moons in `XY` around the
    /// planet.
    moon: bool,
    /// The coordinate off the orbit plane, kept.
    height: f64,
}

/// The movers among `points` of a cell at `level`.
fn movers_of(cell: Cell, level: u8, points: &[Point], out: &mut Vec<Mover>) {
    for (index, point) in points.iter().enumerate() {
        let [x, y, z] = point.position;
        let mover = match (level, point.planet, point.moon) {
            (8 | 9, Some(planet), _) => Mover {
                cell,
                index,
                radius: x.hypot(z),
                base: z.atan2(x),
                period_days: planet.period_days,
                moon: false,
                height: y,
            },
            (10, _, Some(moon)) => Mover {
                cell,
                index,
                radius: x.hypot(y),
                base: y.atan2(x),
                period_days: moon.period_days,
                moon: true,
                height: z,
            },
            _ => continue,
        };
        out.push(mover);
    }
}

/// Rebuilds the movers of the open and parent cells (after a regenerate).
pub(crate) fn collect_movers(universe: &mut Universe) {
    let mut movers = std::mem::take(&mut universe.movers);
    movers.clear();
    let level = universe.level().get();
    movers_of(Cell::Open, level, &universe.open.points, &mut movers);
    if let Some(parent) = universe.parent.as_ref() {
        movers_of(
            Cell::Parent,
            level.saturating_sub(1),
            &parent.points,
            &mut movers,
        );
    }
    universe.movers = movers;
    universe.spin_hours = if level == 10 {
        universe
            .open
            .points
            .iter()
            .find_map(|p| p.surface)
            .map(|s| s.spin_hours)
    } else {
        None
    };
}

impl Universe {
    /// The sky's game time in seconds (#981).
    #[must_use]
    pub fn time_s(&self) -> f64 {
        self.time_s
    }

    /// Sets the sky's game time and moves every body to it (#981 T2).
    pub fn advance(&mut self, game_s: f64) {
        if game_s.is_finite() {
            self.time_s = game_s;
            self.apply_time(false);
        }
    }

    /// Moves the open and parent cells' bodies to [`Universe::time_s`],
    /// keeps an open planet's entry on its planet, so the camera inside
    /// rides along, and turns it by the planet's spin. `fresh` (after an
    /// open or close) keeps the camera's place in the system; otherwise the
    /// camera turns with the ground.
    pub(crate) fn apply_time(&mut self, fresh: bool) {
        let t = self.time_s;
        let spin = self.spin_hours.map_or(0.0, |h| spin_angle_at(h, t));
        for mover in &self.movers {
            // Planets go around their star the way their planet spins
            // (prograde, a right-hand turn about `+Y`: their angle in `XZ`
            // shrinks), so a solar day is a little longer than a turn.
            let turn = orbit_angle_at(0.0, mover.period_days, t);
            let angle = if mover.moon {
                mover.base + turn
            } else {
                mover.base - turn
            };
            let (s, c) = angle.sin_cos();
            let position = if mover.moon {
                // Moons go around in space; the planet's cell turns with
                // its ground, so they turn back by the spin.
                turn_y([mover.radius * c, mover.radius * s, mover.height], -spin)
            } else {
                [mover.radius * c, mover.height, mover.radius * s]
            };
            let cell = match mover.cell {
                Cell::Open => Some(&mut self.open),
                Cell::Parent => self.parent.as_mut(),
            };
            if let Some(point) = cell.and_then(|cell| cell.points.get_mut(mover.index)) {
                point.position = position;
            }
        }
        // An open planet rides on its planet in the star close-up.
        if self.level().get() == 10
            && let Some(entry) = self.path.entered()
            && !entry.anonymous
            && let Some(point) = self
                .parent
                .as_ref()
                .and_then(|parent| parent.points.get(entry.marker as usize))
            && point.planet.is_some()
        {
            let position = point.portal_position();
            self.path.move_entry(position);
            self.path.spin_entry(spin, fresh);
        }
    }

    /// Opens the planet whose gravity zone holds the camera, if any (#981:
    /// a moving zone reaches a still camera). Only in a planetary system.
    pub fn sync_zones(&mut self) -> Option<DiveEvent> {
        if self.level().get() != 8 {
            return None;
        }
        let camera = self.path.offset();
        let marker = (1u32..)
            .zip(self.open.points.iter().skip(1))
            .filter(|(_, point)| point.planet.is_some())
            .find_map(|(marker, point)| {
                let zone = l8_planet_zone(self, marker)?;
                (length3(sub3(camera, point.position)) < zone).then_some(marker)
            })?;
        self.open_planet(marker)
            .then_some(DiveEvent::Opened(marker))
    }
}

#[cfg(test)]
#[path = "sky_motion_tests.rs"]
mod tests;
