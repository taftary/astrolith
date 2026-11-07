//! Bounded surface-interest decisions, distinct from ground contact (#1251).
//!
//! Preparation resolves permanent owners once per containing cell. Decisions
//! read actual current frames without regeneration, allocation or clock changes.
//! Reference-sphere distance chooses work; it never asserts landability/contact.

use crate::coords::OpenUnits;
use crate::moon_catalog::{PlanetIdentity, moons_for_planet};
use crate::nav::Universe;
use crate::surface_frame::SurfaceFrame;
use crate::surface_observation::{GroundObservation, SurfaceIdentity};

/// One planet plus the generator's existing fifteen-moon ceiling (ADR 0058).
pub const LOCAL_SURFACE_LIMIT: usize = 16;

/// A contact-qualified interest lock, not a proximity or strongest-pull guess.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceSupport(GroundObservation);

impl SurfaceSupport {
    /// Requires the actual body contact flag and its matching drawn-ground query.
    /// A hovering/flying body or a readiness flag alone cannot construct support.
    #[must_use]
    pub fn from_contact(body: &crate::body::Body, ground: GroundObservation) -> Option<Self> {
        (body.on_ground
            && !body.flying
            && ground.feet.0 == body.position
            && ground.clearance.0.is_finite()
            && ground.clearance.0.abs() <= 0.01)
            .then_some(Self(ground))
    }
}

/// Why the single requested owner takes priority. None of these is a travel action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceInterestKind {
    /// A contact-qualified player cannot lose ground to camera interest.
    Support,
    /// The active body's approach takes precedence over a detached camera.
    PlayerApproach,
    /// No active body approach/contact; prepare what the camera is near.
    Camera,
}

/// Work request only; the renderer must separately commit a safe visible handover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceInterest {
    /// Requested permanent geometry owner, not a new selectable travel target.
    pub owner: SurfaceIdentity,
    /// Priority evidence used to make this request, not a synthetic contact flag.
    pub kind: SurfaceInterestKind,
}

/// Fixed-size permanent candidates prepared once for one existing L10 host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceCandidates {
    host: PlanetIdentity,
    owners: [Option<SurfaceIdentity>; LOCAL_SURFACE_LIMIT],
}

impl SurfaceCandidates {
    /// Reads existing records without inventing owners or excluding small moons.
    /// Refuses invalid records or a future generator exceeding the declared cap.
    #[must_use]
    pub fn for_host(host: PlanetIdentity) -> Option<Self> {
        let mut owners = [None; LOCAL_SURFACE_LIMIT];
        *owners.first_mut()? = Some(SurfaceIdentity::Planet(host));
        for (index, moon) in moons_for_planet(host).ok()?.into_iter().enumerate() {
            *owners.get_mut(index + 1)? = Some(SurfaceIdentity::Moon(moon.identity));
        }
        Some(Self { host, owners })
    }

    /// Allocation-free check that the candidate set still names this exact cell.
    #[must_use]
    pub fn matches(&self, universe: &Universe) -> bool {
        universe.root == self.host.seed()
            && universe.path.matches_indices(self.host.chain())
            && crate::nav::planet_frame(universe).is_some()
    }

    /// Permanent candidates in stable planet/marker order (ties are repeatable).
    pub fn owners(&self) -> impl Iterator<Item = SurfaceIdentity> + '_ {
        self.owners.iter().flatten().copied()
    }

    /// Support, then actual player approach, then camera. A challenger must be
    /// at least 20% nearer than the current request to change it; ties retain it.
    /// This hysteresis uses no radius cutoff or accumulated per-visited-body state.
    #[must_use]
    pub fn choose(
        &self,
        universe: &Universe,
        current: Option<SurfaceIdentity>,
        support: Option<SurfaceSupport>,
        player: Option<OpenUnits>,
        camera: OpenUnits,
    ) -> Option<SurfaceInterest> {
        if !self.matches(universe) {
            return None;
        }
        if let Some(SurfaceSupport(ground)) = support
            && self.owners().any(|owner| owner == ground.identity)
            && SurfaceFrame::for_owner(universe, ground.identity).is_some()
        {
            return Some(SurfaceInterest {
                owner: ground.identity,
                kind: SurfaceInterestKind::Support,
            });
        }
        // An invalid active-body position is not silently replaced by a camera.
        let (at, kind) = player.map_or((camera, SurfaceInterestKind::Camera), |at| {
            (at, SurfaceInterestKind::PlayerApproach)
        });
        let distance = |owner| -> Option<f64> {
            let frame = SurfaceFrame::for_owner(universe, owner)?;
            let [x, y, z] = frame.from_cell(at)?.metres();
            let gap = (x.hypot(y).hypot(z) - frame.radius().get()).abs();
            gap.is_finite().then_some(gap)
        };
        let mut best = None;
        for owner in self.owners() {
            let Some(gap) = distance(owner) else {
                continue;
            };
            if best.is_none_or(|(_, held)| gap < held) {
                best = Some((owner, gap));
            }
        }
        let (mut owner, gap) = best?;
        if let Some(held) = current.filter(|held| self.owners().any(|owner| owner == *held))
            && let Some(held_gap) = distance(held)
            && gap >= held_gap * 0.8
        {
            owner = held;
        }
        Some(SurfaceInterest { owner, kind })
    }
}

#[cfg(test)]
#[path = "surface_interest_tests.rs"]
mod tests;
