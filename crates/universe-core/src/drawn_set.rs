//! What a continuous zoom draws, and the fade by size (#998 build T2, B2).
//!
//! The window draws the open cell, its parent's markers around it (the
//! level you came from), the grandparent's (two up, faded in and out over
//! half a second at a handover, [`LayerFades`](crate::drawn_set::LayerFades)),
//! and the previews of the markers large enough to show their insides.
//! This module lists that set in open-cell units, the
//! same mapping the window uses (`crate::nav::sibling_in_open_units` for
//! the parent, [`crate::frame::child_world_position_oriented`] for
//! previews), and gives each marker its fade by size on screen (Q3 A): 0
//! below half a pixel, 1 from two pixels, smooth between. A handover must
//! not change the set, so nothing pops; the tests walk the journey and
//! check exactly that. Pure (`E-CORE-NO-BEVY`, `E-DET-TIERS`).

use crate::coords::ParentUnits;
use crate::frame::{IDENTITY_UP, Opened, angular_radius, child_world_position_oriented};
use crate::nav::{MarkerIndex, Universe, sibling_in_open_units};
use crate::preview::PreviewCache;

/// Where a drawn marker comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    /// The open cell.
    Open,
    /// The parent cell around it.
    Parent,
    /// The grandparent cell.
    Grandparent,
    /// A preview inside an open-cell marker.
    Preview,
}

/// One drawn marker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drawn {
    /// Layer.
    pub layer: Layer,
    /// Position in open-cell units.
    pub at: [f64; 3],
    /// Radius in open-cell units.
    pub radius: f64,
    /// Whether it is a portal (portals keep their actionable floor).
    pub portal: bool,
    /// The layer's own weight: the open cell's dimming as you rise out, 1
    /// for the parent, the grandparent's fade, or the preview's interior
    /// brightness.
    pub weight: f64,
    /// For a preview, the open-cell marker it sits inside.
    pub inside: Option<u32>,
}

/// How many markers the continuous zoom draws at once (#998 S1): Display
/// > Scene > Marker detail.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MarkerDetail {
    /// 500 markers, for phones.
    Low,
    /// 2,000 markers.
    #[default]
    Normal,
    /// 5,000 markers.
    High,
}

impl MarkerDetail {
    /// Every choice, in Settings order.
    pub const ALL: [MarkerDetail; 3] =
        [MarkerDetail::Low, MarkerDetail::Normal, MarkerDetail::High];

    /// Markers drawn at once.
    #[must_use]
    pub const fn budget(self) -> usize {
        match self {
            MarkerDetail::Low => 500,
            MarkerDetail::Normal => 2_000,
            MarkerDetail::High => 5_000,
        }
    }

    /// Name shown in Settings.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            MarkerDetail::Low => "Low",
            MarkerDetail::Normal => "Normal",
            MarkerDetail::High => "High",
        }
    }

    /// Id in the settings file.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            MarkerDetail::Low => "low",
            MarkerDetail::Normal => "normal",
            MarkerDetail::High => "high",
        }
    }

    /// The choice with settings-file `id`.
    #[must_use]
    pub fn from_id(id: &str) -> Option<MarkerDetail> {
        Self::ALL.into_iter().find(|d| d.id() == id)
    }

    /// The next choice, wrapping (one press of the Settings row).
    #[must_use]
    pub fn next(self) -> MarkerDetail {
        match self {
            MarkerDetail::Low => MarkerDetail::Normal,
            MarkerDetail::Normal => MarkerDetail::High,
            MarkerDetail::High => MarkerDetail::Low,
        }
    }
}

/// The smallest angular radius that still draws when `set` (weighted and
/// faded, from `camera`) holds more than `budget` markers: the largest
/// `budget` stay, `0.0` when everything fits. `angles` is the caller's
/// buffer (`E-HOT-NOALLOC`).
#[must_use]
pub fn budget_cutoff(set: &[Drawn], camera: [f64; 3], budget: usize, angles: &mut Vec<f64>) -> f64 {
    angles.clear();
    for d in set {
        let v = [
            d.at[0] - camera[0],
            d.at[1] - camera[1],
            d.at[2] - camera[2],
        ];
        let distance = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if d.weight * size_fade(d.radius, distance) > 0.0 {
            angles.push(angular_radius(d.radius, distance));
        }
    }
    if angles.len() <= budget {
        return 0.0;
    }
    angles.sort_by(|a, b| b.total_cmp(a));
    angles.get(budget.saturating_sub(1)).copied().unwrap_or(0.0)
}

/// Seconds over which a layer fades in or out after a handover (#998 B2).
pub const LAYER_FADE_S: f64 = 0.5;

/// The fades that keep a handover from popping (#998 B2): each ancestor
/// level of siblings stays about a pixel wide at every depth (their radii
/// and distances both grow by the ratio), so a level leaving or joining the
/// drawn set would pop. On an open, the level that drops (three up) fades
/// out over [`LAYER_FADE_S`]; on a close, the level that joins (two up)
/// fades in over the same time.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayerFades {
    /// The grandparent's weight, `0..=1`.
    pub grand: f64,
    /// The level fading out after an open, in open-cell units.
    pub leaving: Vec<Drawn>,
    /// Its weight, `0..=1`.
    pub leaving_weight: f64,
}

impl LayerFades {
    /// Fully in, nothing leaving.
    #[must_use]
    pub fn settled() -> Self {
        Self {
            grand: 1.0,
            leaving: Vec::new(),
            leaving_weight: 0.0,
        }
    }

    /// After an open through `entered`: `dropped` (what the frame before
    /// drew that the new level does not: the old grandparent and the
    /// previews inside the markers not entered, old open-cell units) fades
    /// out from where it was; the new grandparent was the parent, fully
    /// drawn. See [`leaving_on_open`].
    pub fn on_open(&mut self, dropped: &[Drawn], entered: Opened) {
        self.leaving.clear();
        if entered.ratio > 0.0 {
            self.leaving.extend(dropped.iter().map(|d| Drawn {
                at: entered.to_child_point(d.at),
                radius: d.radius / entered.ratio,
                ..*d
            }));
        }
        // Each leaving marker keeps its own weight and fades from there.
        self.leaving_weight = 1.0;
        self.grand = 1.0;
    }

    /// After a close: the new grandparent fades in; nothing leaves.
    pub fn on_close(&mut self) {
        self.leaving.clear();
        self.leaving_weight = 0.0;
        self.grand = 0.0;
    }

    /// Moves the fades on by `dt` seconds.
    pub fn tick(&mut self, dt: f64) {
        let step = if dt.is_finite() && dt > 0.0 {
            dt / LAYER_FADE_S
        } else {
            0.0
        };
        self.grand = (self.grand + step).min(1.0);
        self.leaving_weight = (self.leaving_weight - step).max(0.0);
        if self.leaving_weight <= 0.0 {
            self.leaving.clear();
        }
    }
}

/// One pixel's angle at 1600 x 900 with the game's vertical field of view.
#[must_use]
pub fn pixel_rad() -> f64 {
    crate::precision::FOV_DEG.to_radians() / 900.0
}

/// The fade by size of a marker of `radius` at `distance`: 0 under half a
/// pixel of angular radius, 1 from two pixels, smoothstep between.
#[must_use]
pub fn size_fade(radius: f64, distance: f64) -> f64 {
    let px = angular_radius(radius, distance) / pixel_rad();
    let t = ((px - 0.5) / 1.5).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// What a frame drew that the level opened through `entered` no longer
/// draws: the grandparent, and the previews inside other markers.
#[must_use]
pub fn leaving_on_open(before: &[Drawn], entered: Opened) -> Vec<Drawn> {
    before
        .iter()
        .filter(|d| {
            d.layer == Layer::Grandparent
                || (d.layer == Layer::Preview && d.inside != Some(entered.marker))
        })
        .copied()
        .collect()
}

/// Lists what the window draws for `universe`, `previews` and `fades` into
/// `out` (cleared first; the caller keeps the buffer, `E-HOT-NOALLOC`).
pub fn drawn_set(
    universe: &Universe,
    previews: &PreviewCache,
    fades: &LayerFades,
    out: &mut Vec<Drawn>,
) {
    out.clear();
    if fades.leaving_weight > 0.0 {
        out.extend(fades.leaving.iter().map(|d| Drawn {
            weight: d.weight * fades.leaving_weight,
            ..*d
        }));
    }
    // The open cell's markers dim as you rise out of it exactly as its
    // marker's interior dims from the parent (`children_brightness` of the
    // cell's own angular radius), so a close changes nothing (#998 B2).
    let open_weight = if universe.path.is_at_root() {
        1.0
    } else {
        crate::frame::children_brightness(angular_radius(
            0.5,
            crate::nav::length3(universe.path.offset()),
        ))
    };
    for point in &universe.open.points {
        if point.surface.is_some() {
            continue;
        }
        out.push(Drawn {
            layer: Layer::Open,
            at: point.position,
            radius: point.radius,
            portal: point.kind == crate::r#gen::MarkerKind::Portal,
            weight: open_weight,
            inside: None,
        });
    }
    if let (Some(parent), Some(entered)) = (universe.parent.as_ref(), universe.path.entered())
        && !entered.anonymous
        && entered.ratio > 0.0
    {
        for (index, point) in parent.points.iter().enumerate() {
            if u32::try_from(index).ok() == Some(entered.marker) {
                continue;
            }
            out.push(Drawn {
                layer: Layer::Parent,
                at: sibling_in_open_units(entered, ParentUnits(point.position))
                    .0
                    .0,
                radius: point.radius / entered.ratio,
                portal: false,
                weight: 1.0,
                inside: None,
            });
        }
        if let (Some(grand), Some(parent_entry)) =
            (universe.grandparent.as_ref(), universe.parent_entry())
            && fades.grand > 0.0
            && parent_entry.ratio > 0.0
        {
            for (index, point) in grand.points.iter().enumerate() {
                if u32::try_from(index).ok() == Some(parent_entry.marker) {
                    continue;
                }
                let in_parent = parent_entry.to_child_point(point.position);
                out.push(Drawn {
                    layer: Layer::Grandparent,
                    at: entered.to_child_point(in_parent),
                    radius: point.radius / parent_entry.ratio / entered.ratio,
                    portal: false,
                    weight: fades.grand,
                    inside: None,
                });
            }
        }
    }
    let Some(ratio) = crate::nest::child_ratio(universe.level()) else {
        return;
    };
    let marker_radius = universe.marker_radius();
    let camera = universe.path.offset();
    for (marker, content) in previews.entries() {
        let Some(marker_pos) = universe.marker(MarkerIndex(*marker)) else {
            continue;
        };
        let inside = crate::frame::children_brightness(angular_radius(
            marker_radius,
            crate::nav::length3([
                marker_pos[0] - camera[0],
                marker_pos[1] - camera[1],
                marker_pos[2] - camera[2],
            ]),
        ));
        for point in &content.points {
            if point.surface.is_some() {
                continue;
            }
            out.push(Drawn {
                layer: Layer::Preview,
                at: child_world_position_oriented(
                    ParentUnits(marker_pos),
                    ratio,
                    point.position,
                    IDENTITY_UP,
                )
                .0,
                radius: point.radius * ratio,
                portal: false,
                weight: inside,
                inside: Some(*marker),
            });
        }
    }
}

/// The summed fade of `set` seen from `camera`: how much is on screen.
#[must_use]
pub fn shown(set: &[Drawn], camera: [f64; 3]) -> f64 {
    set.iter()
        .map(|d| {
            let v = [
                d.at[0] - camera[0],
                d.at[1] - camera[1],
                d.at[2] - camera[2],
            ];
            d.weight * size_fade(d.radius, (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())
        })
        .sum()
}

#[cfg(test)]
#[path = "drawn_set_tests.rs"]
mod tests;
