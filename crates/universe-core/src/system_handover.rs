//! Frame handovers inside one star system (#995 T3, study).
//!
//! A position lives in one frame at a time, as `f64` metres from the
//! frame's origin:
//!
//! - the star frame (origin at the star, ecliptic axes);
//! - a planet's or moon's orbit frame, from inside its gravity zone
//!   ([`crate::system_space::influence_radius_m`]): origin at the body,
//!   the star frame's axes, so you follow its orbit;
//! - a planet's spin frame, below [`SPIN_HEIGHT_RADII`] planet radii above
//!   its surface: origin at the planet, axes tilted with it, so you turn
//!   with it.
//!
//! You enter a frame at its radius and leave it only past
//! [`LEAVE_FACTOR`] times that radius, so a path along a boundary never
//! flickers. A handover re-expresses the same point in the new frame; the
//! study measures what that costs.
//!
//! Pure and deterministic (`E-CORE-NO-BEVY`, `E-DET-TIERS`).

use crate::system_space::{BodyKind, SystemSpace, distance_m, influence_radius_m};

/// A frame is left only past this multiple of the radius it is entered at.
pub const LEAVE_FACTOR: f64 = 1.2;

/// Height above a planet's surface, in planet radii, below which you turn
/// with it (Kerbal Space Program's rotating frame starts 100 km over a
/// 600 km planet, a sixth of a radius; one radius is a rounder,
/// generous choice for the study).
pub const SPIN_HEIGHT_RADII: f64 = 1.0;

/// Which frame a position is held in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frame {
    /// The star frame.
    Star,
    /// The orbit frame of body `index` (a planet or a moon).
    Orbit(usize),
    /// The spin frame of planet `index`.
    Spin(usize),
}

/// A point held in one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Located {
    /// The frame.
    pub frame: Frame,
    /// Metres from the frame's origin, along the frame's axes.
    pub local_m: [f64; 3],
}

/// The rotation of planet `index`'s spin frame: its tilt about the star
/// frame's `+Z` axis, as an orthonormal basis (rows are the frame's axes
/// in star-frame terms).
#[must_use]
pub fn spin_basis(space: &SystemSpace, index: usize) -> [[f64; 3]; 3] {
    let tilt = space
        .bodies
        .get(index)
        .map_or(0.0, |body| body.tilt_deg.to_radians());
    let (s, c) = tilt.sin_cos();
    [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]]
}

fn origin_of(space: &SystemSpace, frame: Frame) -> [f64; 3] {
    match frame {
        Frame::Star => [0.0; 3],
        Frame::Orbit(i) | Frame::Spin(i) => space.bodies.get(i).map_or([0.0; 3], |b| b.position_m),
    }
}

/// A frame-relative vector turned into star-frame axes.
#[must_use]
pub fn to_star_axes(space: &SystemSpace, frame: Frame, v: [f64; 3]) -> [f64; 3] {
    let Frame::Spin(i) = frame else {
        return v;
    };
    let [a, b, c] = spin_basis(space, i);
    [
        a[0] * v[0] + b[0] * v[1] + c[0] * v[2],
        a[1] * v[0] + b[1] * v[1] + c[1] * v[2],
        a[2] * v[0] + b[2] * v[1] + c[2] * v[2],
    ]
}

/// A star-frame-axes vector turned into `frame`'s axes.
#[must_use]
pub fn from_star_axes(space: &SystemSpace, frame: Frame, v: [f64; 3]) -> [f64; 3] {
    let Frame::Spin(i) = frame else {
        return v;
    };
    let [a, b, c] = spin_basis(space, i);
    let dot = |r: [f64; 3]| r[0] * v[0] + r[1] * v[1] + r[2] * v[2];
    [dot(a), dot(b), dot(c)]
}

/// The point in the star frame.
#[must_use]
pub fn to_star(space: &SystemSpace, at: Located) -> [f64; 3] {
    let o = origin_of(space, at.frame);
    let v = to_star_axes(space, at.frame, at.local_m);
    [o[0] + v[0], o[1] + v[1], o[2] + v[2]]
}

/// The star-frame point `star_m` held in `frame`.
#[must_use]
pub fn locate(space: &SystemSpace, frame: Frame, star_m: [f64; 3]) -> Located {
    let o = origin_of(space, frame);
    let v = [star_m[0] - o[0], star_m[1] - o[1], star_m[2] - o[2]];
    Located {
        frame,
        local_m: from_star_axes(space, frame, v),
    }
}

/// The frame a point at `star_m` belongs in, given the frame it is held in
/// now (`current`): the innermost zone containing it (a moon's orbit frame,
/// then a planet's spin frame, then a planet's orbit frame, then the star
/// frame), each entered at its radius and left past [`LEAVE_FACTOR`] times
/// it.
#[must_use]
pub fn frame_for(space: &SystemSpace, star_m: [f64; 3], current: Frame) -> Frame {
    let mut candidates: Vec<(Frame, f64)> = Vec::new();
    for (index, body) in space.bodies.iter().enumerate() {
        if let BodyKind::Moon { .. } = body.kind {
            candidates.push((Frame::Orbit(index), influence_radius_m(space, body)));
        }
    }
    for (index, body) in space.bodies.iter().enumerate() {
        if let BodyKind::Planet { .. } = body.kind {
            candidates.push((
                Frame::Spin(index),
                body.radius_m * (1.0 + SPIN_HEIGHT_RADII),
            ));
        }
    }
    for (index, body) in space.bodies.iter().enumerate() {
        if let BodyKind::Planet { .. } = body.kind {
            candidates.push((Frame::Orbit(index), influence_radius_m(space, body)));
        }
    }
    for (frame, radius) in candidates {
        let Some(body) = frame_body(frame).and_then(|i| space.bodies.get(i)) else {
            continue;
        };
        let reach = if holds(space, current, frame) {
            radius * LEAVE_FACTOR
        } else {
            radius
        };
        if distance_m(star_m, body.position_m) < reach {
            return frame;
        }
    }
    Frame::Star
}

/// Whether holding `current` means `frame` is already entered: the same
/// frame, or a planet's orbit frame while in its spin frame or in one of its
/// moons' frames.
fn holds(space: &SystemSpace, current: Frame, frame: Frame) -> bool {
    if current == frame {
        return true;
    }
    let Frame::Orbit(planet) = frame else {
        return false;
    };
    match current {
        Frame::Spin(i) => i == planet,
        Frame::Orbit(i) => matches!(
            space.bodies.get(i).map(|b| b.kind),
            Some(BodyKind::Moon { planet: host, .. }) if host == planet
        ),
        Frame::Star => false,
    }
}

/// The body a frame is attached to.
#[must_use]
pub fn frame_body(frame: Frame) -> Option<usize> {
    match frame {
        Frame::Star => None,
        Frame::Orbit(i) | Frame::Spin(i) => Some(i),
    }
}

#[cfg(test)]
#[path = "system_handover_tests.rs"]
mod tests;
