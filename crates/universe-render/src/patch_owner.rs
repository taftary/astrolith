//! The shared terrain stack's work interest and owner-centred conversions.
//! Visible ownership commits separately, only after coarse/fine cover is safe.

use crate::patches::{GroundSurface, PatchTree};
use universe_core::coords::OpenUnits;
use universe_core::planet_frame::PlanetPos;
use universe_core::surface_frame::SurfaceFrame;
use universe_core::surface_interest::{SurfaceCandidates, SurfaceSupport};
use universe_core::surface_observation::GroundObservation;

/// No navigation/input changes or per-frame catalog generation. Only the
/// bounded candidates are rebuilt when the actual containing identity changes.
pub(crate) fn update_interest(
    tree: &mut PatchTree,
    universe: &universe_core::nav::Universe,
    ground: &GroundSurface,
    player: Option<&crate::player_state::PlayerState>,
    camera: OpenUnits,
) {
    if universe.level().get() != 10 {
        tree.candidates = None;
        tree.interest = None;
        return;
    }
    if tree
        .candidates
        .as_ref()
        .is_none_or(|c| !c.matches(universe))
    {
        tree.candidates = universe_core::moon_catalog::PlanetIdentity::from_open_cell(universe)
            .ok()
            .and_then(SurfaceCandidates::for_host);
        tree.interest = None;
    }
    let player = player.filter(|p| p.active && p.root == universe.root.0);
    let support = player.and_then(|p| {
        let surface = ground.0.as_ref()?;
        let identity = ground.3?;
        if surface.owner_frame()?.owner() != identity {
            return None;
        }
        // Geometry comes from the actual held surface, not the requested owner.
        let observed = GroundObservation::from_ground(
            identity,
            surface.ground_at(p.body.position)?,
            OpenUnits(p.body.position),
            universe_core::planet_frame::metres_per_cell_l10(),
        )?;
        SurfaceSupport::from_contact(&p.body, observed)
    });
    tree.interest = tree.candidates.as_ref().and_then(|candidates| {
        candidates.choose(
            universe,
            tree.interest.map(|i| i.owner),
            support,
            player.map(|p| OpenUnits(p.body.position)),
            camera,
        )
    });
}

pub(crate) fn patch_at(frame: SurfaceFrame, cell: [f64; 3]) -> Option<PlanetPos> {
    frame.patch_position(frame.from_cell(OpenUnits(cell))?)
}

pub(crate) fn cell_at(frame: SurfaceFrame, patch: PlanetPos) -> Option<[f64; 3]> {
    Some(frame.to_cell(frame.position(patch.0)?)?.0)
}

#[cfg(test)]
#[path = "patch_owner_tests.rs"]
mod tests;
