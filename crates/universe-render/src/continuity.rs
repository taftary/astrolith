//! A seamless zoom in the window (#998 build T2, B2).
//!
//! [`universe_core::drawn_set`] lists what the window draws and how much:
//! the open cell dimmed as you rise out of it, the parent, the grandparent,
//! the previews, and for half a second after a handover the level that
//! left or joins ([`LayerFades`]). This keeps that list each frame, feeds
//! the handovers to the fades, and draws the two layers the older drawers
//! do not: the grandparent and what is fading out. The older drawers read
//! the same fade by size and the open cell's dimming from here, so the
//! picture across a handover changes no more than on an ordinary frame.

use crate::Universe;
use crate::style::{RenderOrigin, scaled, sibling_color_for_level};
use crate::tokens::dot_floor_world;
use bevy::prelude::*;
use universe_core::coords::Level;
use universe_core::drawn_set::{
    Drawn, Layer, LayerFades, budget_cutoff, drawn_set, leaving_on_open, size_fade,
};

/// The drawn set and its fades, kept between frames.
#[derive(Resource, Debug)]
pub(crate) struct Continuity {
    /// The layer fades.
    pub(crate) fades: LayerFades,
    /// This frame's drawn set (the buffer is kept, `E-HOT-NOALLOC`).
    pub(crate) set: Vec<Drawn>,
    /// Named opens on the path last frame.
    depth: Option<usize>,
    /// The smallest angular radius drawn this frame under the Marker detail
    /// budget (#998 S1); 0 when everything fits.
    pub(crate) cutoff: f64,
    /// Buffer for the budget's angles (`E-HOT-NOALLOC`).
    angles: Vec<f64>,
}

impl Default for Continuity {
    fn default() -> Self {
        Self {
            fades: LayerFades::settled(),
            set: Vec::new(),
            depth: None,
            cutoff: 0.0,
            angles: Vec::new(),
        }
    }
}

/// Named opens on `universe`'s path (no allocation).
fn named_depth(universe: &universe_core::nav::Universe) -> usize {
    universe
        .path
        .chain()
        .iter()
        .filter(|opened| !opened.anonymous)
        .count()
}

/// Feeds this frame's handover to the fades, moves them on and rebuilds
/// the drawn set. Runs after the previews sync.
pub(crate) fn sync_continuity(
    time: Res<Time>,
    universe: Res<Universe>,
    previews: Res<crate::PreviewCache>,
    store: Option<Res<crate::SettingsStore>>,
    mut state: ResMut<Continuity>,
) {
    let depth = named_depth(&universe);
    let state = &mut *state;
    match state.depth {
        Some(last) if depth > last => {
            if let Some(entered) = universe.path.entered() {
                let leaving = leaving_on_open(&state.set, entered);
                state.fades.on_open(&leaving, entered);
            }
        }
        Some(last) if depth < last => state.fades.on_close(),
        _ => {}
    }
    state.depth = Some(depth);
    state.fades.tick(f64::from(time.delta_secs()));
    drawn_set(&universe, &previews.0, &state.fades, &mut state.set);
    let budget = store.map_or(
        universe_core::drawn_set::MarkerDetail::default().budget(),
        |store| store.0.marker_detail.budget(),
    );
    state.cutoff = budget_cutoff(
        &state.set,
        universe.path.offset(),
        budget,
        &mut state.angles,
    );
}

/// The open cell's markers' weight this frame (1 at the root).
#[must_use]
pub(crate) fn open_weight(state: &Continuity) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: render-domain narrowing of a brightness, intended"
    )]
    let weight = state
        .set
        .iter()
        .find(|d| d.layer == Layer::Open)
        .map_or(1.0, |d| d.weight) as f32;
    weight
}

/// The fade by size of a marker of `radius` at `distance` (cell units),
/// zero below the Marker detail budget's `cutoff` angle (#998 S1).
#[must_use]
pub(crate) fn fade_by_size(radius: f64, distance: f64, cutoff: f64) -> f32 {
    if cutoff > 0.0 && universe_core::frame::angular_radius(radius, distance) < cutoff {
        return 0.0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: render-domain narrowing of a brightness, intended"
    )]
    let fade = size_fade(radius, distance) as f32;
    fade
}

/// The budget's cutoff angle, 0 without the resource.
#[must_use]
pub(crate) fn cutoff(state: Option<&Continuity>) -> f64 {
    state.map_or(0.0, |s| s.cutoff)
}

/// Draws the grandparent's markers and whatever is fading out, as dots in
/// the sibling tint two levels up, weighted and faded by size.
pub(crate) fn draw_continuity(
    mut gizmos: Gizmos,
    universe: Res<Universe>,
    origin: Res<RenderOrigin>,
    state: Res<Continuity>,
) {
    let camera = universe.path.offset();
    let two_up = universe
        .level()
        .shallower()
        .and_then(Level::shallower)
        .unwrap_or(Level::MIN);
    let color = sibling_color_for_level(two_up);
    let leaving = if state.fades.leaving_weight > 0.0 {
        state.fades.leaving.len()
    } else {
        0
    };
    for (index, drawn) in state.set.iter().enumerate() {
        if index >= leaving && drawn.layer != Layer::Grandparent {
            continue;
        }
        let v = [
            drawn.at[0] - camera[0],
            drawn.at[1] - camera[1],
            drawn.at[2] - camera[2],
        ];
        let distance = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a brightness, intended"
        )]
        let weight = drawn.weight as f32;
        let brightness = weight * fade_by_size(drawn.radius, distance, state.cutoff);
        if brightness <= 0.0 {
            continue;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a radius, intended"
        )]
        let own = drawn.radius as f32;
        let dot = own.max(dot_floor_world(distance));
        gizmos.sphere(
            Isometry3d::from_translation(origin.at(drawn.at)),
            origin.len(f64::from(dot)),
            scaled(color, brightness),
        );
    }
}
