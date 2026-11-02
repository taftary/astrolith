//! How a semantic part looks from the camera (#1128 T3).
//!
//! Pure display decisions for the developer layer: the representation stage
//! R0–R6 from the part's own apparent angular size (never raw distance or
//! the level), with separate enter/exit thresholds and a crossfade weight;
//! the view-angle and inside/local weights per shape family; the central
//! semantic palette with a non-hue cue per colour role; priority budgets;
//! and the collision-aware developer label layout. Rendering applies these
//! and decides nothing itself (`E-CORE-NO-BEVY`).

use crate::nest::{CLOSE_ANGLE, FORM_ANGLE, PREVIEW_ANGLE};
use crate::semantic::PartRecord;
use crate::semantic_catalog::{
    BudgetGroup, ColourRole, LocalRule, PartSpec, ShapeFamily, SizeBand, ViewRule,
};

/// Representation stage of one part (Spec v3 section 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Stage {
    /// R0: outside its context, not drawn.
    #[default]
    Hidden,
    /// R1: a point at the true anchor.
    Landmark,
    /// R2: a simple category glyph.
    Glyph,
    /// R3: true-oriented outer geometry.
    Outline,
    /// R4: secondary parts and a readable label.
    Composite,
    /// R5: the resolved child preview takes over where previews exist.
    Resolved,
    /// R6: inside or near, only useful local references.
    Local,
}

/// Angular radius (radians) at which each stage above R1 enters while the
/// part grows; R1 enters at the part's own band. From the existing ladder:
/// forms at 0.006, previews at 0.02, close at 0.05.
const GLYPH_ANGLE: f64 = 0.002;
const STAGE_ENTER: [(Stage, f64); 4] = [
    (Stage::Glyph, GLYPH_ANGLE),
    (Stage::Outline, FORM_ANGLE),
    (Stage::Composite, PREVIEW_ANGLE),
    (Stage::Resolved, CLOSE_ANGLE),
];

/// A stage leaves once the part shrinks below this share of its entry
/// angle, so slow zoom at a threshold never flickers.
pub const EXIT_SHARE: f64 = 0.8;

/// Angular radius (radians) of a sphere of `extent` seen from `distance`;
/// `pi/2` from inside. Window size and field of view never enter, so the
/// same view at any resolution selects the same stage.
#[must_use]
pub fn apparent_angle(extent: f64, distance: f64) -> f64 {
    crate::frame::angular_radius(extent.max(0.0), distance.max(0.0))
}

/// Entry angle of `stage` for a part with `band`.
fn enter_of(stage: Stage, band: SizeBand) -> f64 {
    match stage {
        Stage::Hidden => 0.0,
        Stage::Landmark => band.enter,
        _ => STAGE_ENTER
            .iter()
            .find(|(s, _)| *s == stage)
            .map_or(f64::INFINITY, |(_, angle)| angle.max(band.enter)),
    }
}

/// Exit angle of `stage` for a part with `band`.
fn exit_of(stage: Stage, band: SizeBand) -> f64 {
    match stage {
        Stage::Hidden => 0.0,
        Stage::Landmark => band.exit,
        _ => enter_of(stage, band) * EXIT_SHARE,
    }
}

/// Stage for a part of `spec` seen at angular radius `angle`, given the
/// stage it had last frame. Growing needs the next stage's entry angle;
/// shrinking needs to fall under the current stage's exit angle (hysteresis).
/// `inside` (the camera within the part's extent) selects R6 for parts with
/// a local rule. Parts without previews stop at R4.
#[must_use]
pub fn select_stage(spec: &PartSpec, angle: f64, previous: Stage, inside: bool) -> Stage {
    if inside && spec.local != LocalRule::Keep {
        return Stage::Local;
    }
    let top = if spec.shape == ShapeFamily::Point || spec.band.enter > 0.0 {
        Stage::Composite
    } else {
        Stage::Resolved
    };
    let ladder = [
        Stage::Landmark,
        Stage::Glyph,
        Stage::Outline,
        Stage::Composite,
        Stage::Resolved,
    ];
    let mut stage = Stage::Hidden;
    for candidate in ladder {
        if candidate > top {
            break;
        }
        // Stages at or below last frame's hold until their exit angle.
        let threshold = if candidate <= previous && previous != Stage::Local {
            exit_of(candidate, spec.band)
        } else {
            enter_of(candidate, spec.band)
        };
        let reached = if candidate == Stage::Landmark && spec.band.enter == 0.0 {
            true
        } else {
            angle >= threshold && angle > 0.0
        };
        if reached {
            stage = candidate;
        } else {
            break;
        }
    }
    stage
}

/// Crossfade weight `0..=1` of `stage` at `angle`: it eases in between its
/// exit and entry angles, so a stage never pops in all at once.
#[must_use]
pub fn stage_weight(spec: &PartSpec, stage: Stage, angle: f64) -> f64 {
    match stage {
        Stage::Hidden => 0.0,
        Stage::Local => 1.0,
        Stage::Landmark if spec.band.enter == 0.0 => 1.0,
        _ => {
            let (lo, hi) = (exit_of(stage, spec.band), enter_of(stage, spec.band));
            if hi <= lo {
                return 1.0;
            }
            let t = ((angle - lo) / (hi - lo)).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        }
    }
}

/// Smallest drawn angular radius of any visible part: a findability floor.
pub const MIN_DRAWN_ANGLE: f64 = 0.000_5;

/// Drawn radius of a part: its true extent, never below the findability
/// floor at `distance`. The part's own extent is never changed.
#[must_use]
pub fn drawn_extent(extent: f64, distance: f64) -> f64 {
    extent.max(distance.max(0.0) * MIN_DRAWN_ANGLE)
}

/// Sine of 10°: a flat part this far or more out of its plane draws full.
const FACE_FULL_SIN: f64 = 0.173_648_18;
/// Sine of 2°: a flat part this close to its plane, or closer, is gone.
const FACE_GONE_SIN: f64 = 0.034_899_497;

fn smooth(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// How much of a part draws from `eye` (cell units), `0..=1`, from its view
/// rule and its local rule (Spec v3 section 5). `ground` is the near-ground
/// weight (1 high up, 0 on the ground). The part never moves: only its
/// weight changes.
#[must_use]
pub fn view_weight(record: &PartRecord, eye: [f64; 3], ground: f64) -> f64 {
    let spec = record.spec();
    let offset = sub(eye, record.anchor);
    let distance = dot(offset, offset).sqrt();
    let extent = record.extent.max(0.0);
    let facing = match (spec.view, record.orientation) {
        (ViewRule::FaceOn, Some(normal)) => {
            let n = dot(normal, normal).sqrt();
            if distance <= 0.0 || n <= 0.0 {
                0.0
            } else {
                let sin = (dot(offset, normal) / (distance * n)).abs();
                smooth((sin - FACE_GONE_SIN) / (FACE_FULL_SIN - FACE_GONE_SIN))
            }
        }
        _ => 1.0,
    };
    let local = match spec.local {
        LocalRule::Keep => 1.0,
        // Fades over the outer fifth of the volume as the eye goes in.
        LocalRule::FadeInside => {
            if extent <= 0.0 {
                1.0
            } else {
                smooth((distance / extent - 0.8) / 0.2)
            }
        }
        // Fades within a tenth of the extent of its plane.
        LocalRule::FadeNearPlane => match record.orientation {
            Some(normal) if extent > 0.0 => {
                let n = dot(normal, normal).sqrt().max(f64::MIN_POSITIVE);
                let height = (dot(offset, normal) / n).abs();
                smooth(height / (0.1 * extent))
            }
            _ => 1.0,
        },
        LocalRule::FadeNearGround => ground.clamp(0.0, 1.0),
    };
    facing * local
}

/// Whether `eye` is inside a part's extent.
#[must_use]
pub fn is_inside(record: &PartRecord, eye: [f64; 3]) -> bool {
    let offset = sub(eye, record.anchor);
    dot(offset, offset) < record.extent * record.extent
}

/// Line pattern: the non-hue cue that tells colour roles apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinePattern {
    /// Unbroken.
    Solid,
    /// Long dashes.
    Dashed,
    /// Short dots.
    Dotted,
    /// Dash, dot, dash.
    DashDot,
}

impl LinePattern {
    /// On/off run lengths in segments of a 48-segment circle; `None` solid.
    #[must_use]
    pub const fn runs(self) -> Option<(usize, usize)> {
        match self {
            LinePattern::Solid => None,
            LinePattern::Dashed => Some((4, 2)),
            LinePattern::Dotted => Some((1, 2)),
            LinePattern::DashDot => Some((3, 1)),
        }
    }
}

/// Central semantic palette (Spec v3 section 6): sRGB colour and line
/// pattern per role. `None` colour means the owner's own colour
/// (classification and era) or its muted variant (orbits and paths).
#[must_use]
pub const fn palette(role: ColourRole) -> (Option<[f32; 3]>, LinePattern) {
    match role {
        ColourRole::Classification | ColourRole::Era => (None, LinePattern::Solid),
        ColourRole::OwnerMuted => (None, LinePattern::Dashed),
        ColourRole::Void => (Some([0.56, 0.36, 0.85]), LinePattern::Dotted),
        ColourRole::Wall => (Some([0.80, 0.62, 0.22]), LinePattern::Solid),
        ColourRole::Filament => (Some([0.25, 0.80, 0.85]), LinePattern::Solid),
        ColourRole::Node => (Some([0.95, 0.65, 0.15]), LinePattern::Solid),
        ColourRole::CloudBoundary => (Some([0.20, 0.65, 0.58]), LinePattern::Dashed),
        ColourRole::Belt => (Some([0.78, 0.66, 0.50]), LinePattern::DashDot),
        ColourRole::Shell => (Some([0.35, 0.55, 0.95]), LinePattern::Dotted),
        ColourRole::Equator => (Some([0.30, 0.75, 0.35]), LinePattern::Solid),
        ColourRole::SpinAxis => (Some([0.90, 0.85, 0.25]), LinePattern::Solid),
        ColourRole::CellReference => (Some([0.55, 0.55, 0.55]), LinePattern::DashDot),
    }
}

/// Colours kept for states, never for categories: hover white, target and
/// selection magenta, and the red, green and blue debug axes.
pub const RESERVED: [[f32; 3]; 5] = [
    [1.0, 1.0, 1.0],
    [1.0, 0.0, 1.0],
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.5, 1.0],
];

/// Relative luminance of an sRGB colour (approximate, linear weights).
#[must_use]
pub fn lightness(rgb: [f32; 3]) -> f32 {
    0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
}

/// One candidate for a budget: a part, its priority keys and its group.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidate {
    /// Place in the registry.
    pub slot: usize,
    /// Budget group.
    pub group: BudgetGroup,
    /// Catalog importance; higher wins.
    pub importance: u8,
    /// Apparent angle; larger wins.
    pub angle: f64,
    /// Distance from the eye; nearer wins.
    pub distance: f64,
    /// Pinned by navigation (target, hover, journey): always kept.
    pub pinned: bool,
}

/// Keeps, per group, the pinned candidates plus the best by importance,
/// apparent size and nearness, up to the group cap; populations take
/// `population_cap` (Marker detail). Sorts `candidates` in place and writes
/// the kept slots to `out`, reusing both buffers (`E-HOT-NOALLOC`). Floats
/// sort with `total_cmp` (`E-FLOAT-SORT`).
pub fn apply_budgets(candidates: &mut [Candidate], population_cap: usize, out: &mut Vec<usize>) {
    out.clear();
    candidates.sort_by(|a, b| {
        group_rank(a.group)
            .cmp(&group_rank(b.group))
            .then(b.pinned.cmp(&a.pinned))
            .then(b.importance.cmp(&a.importance))
            .then(b.angle.total_cmp(&a.angle))
            .then(a.distance.total_cmp(&b.distance))
            .then(a.slot.cmp(&b.slot))
    });
    let mut group = None;
    let mut kept = 0;
    for candidate in candidates.iter() {
        if group != Some(candidate.group) {
            group = Some(candidate.group);
            kept = 0;
        }
        let cap = candidate.group.cap().unwrap_or(population_cap);
        if candidate.pinned || kept < cap {
            out.push(candidate.slot);
            if !candidate.pinned {
                kept += 1;
            }
        }
    }
}

const fn group_rank(group: BudgetGroup) -> u8 {
    match group {
        BudgetGroup::Populations => 0,
        BudgetGroup::Bodies => 1,
        BudgetGroup::Structures => 2,
        BudgetGroup::Paths => 3,
        BudgetGroup::Boundaries => 4,
        BudgetGroup::References => 5,
    }
}

/// Most developer labels on screen at once.
pub const LABEL_CAP: usize = 24;

/// Smallest apparent angle (radians) at which a part is readable enough to
/// carry a developer label.
pub const LABEL_ANGLE: f64 = FORM_ANGLE;

/// One developer label wanting a place on screen, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelBox {
    /// Place in the registry.
    pub slot: usize,
    /// Top-left corner.
    pub at: [f32; 2],
    /// Width and height.
    pub size: [f32; 2],
    /// Priority; higher is placed first.
    pub priority: f64,
}

impl LabelBox {
    fn overlaps(&self, other: &LabelBox) -> bool {
        self.at[0] < other.at[0] + other.size[0]
            && other.at[0] < self.at[0] + self.size[0]
            && self.at[1] < other.at[1] + other.size[1]
            && other.at[1] < self.at[1] + self.size[1]
    }
}

/// Places developer labels highest priority first, skipping any that would
/// overlap a placed one or a `blocked` box (the navigation labels, which
/// always keep their place), up to `cap`. Reuses `boxes` and `out`.
pub fn place_labels(
    boxes: &mut [LabelBox],
    blocked: &[LabelBox],
    cap: usize,
    out: &mut Vec<LabelBox>,
) {
    out.clear();
    boxes.sort_by(|a, b| b.priority.total_cmp(&a.priority).then(a.slot.cmp(&b.slot)));
    for candidate in boxes.iter() {
        if out.len() >= cap {
            break;
        }
        let free = out
            .iter()
            .chain(blocked)
            .all(|placed| !placed.overlaps(candidate));
        if free {
            out.push(*candidate);
        }
    }
}

#[cfg(test)]
#[path = "semantic_display_tests.rs"]
mod tests;
