//! Developer labels: a separate, capped, collision-aware pool (#1128 T3).
//!
//! The navigation labels (`labels`) keep their own pool and are never
//! touched here; their boxes only block developer labels, so the Labels
//! filter never removes a navigation label (AC10). A developer label names
//! a part's category and stable ID once the part is readable (stage R4 at
//! or above [`LABEL_ANGLE`]), and the layout itself is decided in core
//! ([`place_labels`]).

use bevy::prelude::*;
use bevy::ui::{Node, PositionType, Val, widget::Text};
use universe_core::semantic::{LabelRule, Parent, PartId, PartRecord};
use universe_core::semantic_display::{LABEL_ANGLE, LABEL_CAP, LabelBox, Stage, place_labels};
use universe_core::settings::DeveloperPart;

use crate::developer::DeveloperLayer;
use crate::labels::PortalLabel;
use crate::menu::Screen;
use crate::style::RenderOrigin;
use crate::tokens::LAYER_UNIVERSE;

/// Approximate glyph width and line height of a label, in pixels.
const CHAR_PX: f32 = 12.0;
const LINE_PX: f32 = 24.0;

/// Offset of a developer label from its anchor, below-right (navigation
/// labels sit above-right).
const OFFSET_PX: f32 = 6.0;

/// Developer label tint: a muted grey, distinct from the white navigation
/// labels and from every reserved state colour.
const DEVELOPER_LABEL_TINT: Color = Color::srgb(0.62, 0.66, 0.70);

/// One slot of the developer label pool.
#[derive(Component)]
pub(crate) struct DeveloperLabel {
    slot: usize,
}

/// Layout scratch, reused every frame (`E-HOT-NOALLOC`).
#[derive(Default)]
pub(crate) struct LabelScratch {
    boxes: Vec<LabelBox>,
    blocked: Vec<LabelBox>,
    placed: Vec<LabelBox>,
    /// Part each slot's text names, so text is rewritten only on change.
    written: Vec<Option<PartId>>,
}

/// Spawns the fixed developer label pool (hidden).
pub(crate) fn spawn_developer_labels(mut commands: Commands) {
    for slot in 0..LABEL_CAP {
        commands.spawn((
            DeveloperLabel { slot },
            Name::new("developer-label"),
            GlobalZIndex(LAYER_UNIVERSE),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            Text::new(String::new()),
            TextColor(DEVELOPER_LABEL_TINT),
            Visibility::Hidden,
        ));
    }
}

/// Developer label slots.
type DeveloperSlots<'w, 's> = Query<
    'w,
    's,
    (
        &'static DeveloperLabel,
        &'static mut Text,
        &'static mut Node,
        &'static mut Visibility,
    ),
    Without<PortalLabel>,
>;

/// Navigation label slots, read only to keep developer labels off them.
type NavigationSlots<'w, 's> =
    Query<'w, 's, (&'static Text, &'static Node, &'static Visibility), With<PortalLabel>>;

/// Places and writes developer labels for the planned parts.
pub(crate) fn sync_developer_labels(
    layer: Res<DeveloperLayer>,
    screen: Option<Res<State<Screen>>>,
    origin: Res<RenderOrigin>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    navigation: NavigationSlots,
    mut slots: DeveloperSlots,
    mut scratch: Local<LabelScratch>,
) {
    let on = layer.effective().allows(DeveloperPart::Labels)
        && screen.is_none_or(|screen| *screen.get() == Screen::Universe);
    let scratch = &mut *scratch;
    scratch.boxes.clear();
    scratch.placed.clear();
    if on && let Some((camera, transform)) = cameras.iter().next() {
        for (record, plan) in layer.planned() {
            let spec = record.spec();
            if spec.label != LabelRule::WhenReadable
                || plan.stage < Stage::Composite
                || plan.angle < LABEL_ANGLE
            {
                continue;
            }
            let Ok(screen) = camera.world_to_viewport(transform, origin.at(record.anchor)) else {
                continue;
            };
            if !screen.x.is_finite() || !screen.y.is_finite() {
                continue;
            }
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: a label length in characters, exact in f32"
            )]
            let chars = label_len(record) as f32;
            scratch.boxes.push(LabelBox {
                slot: plan.slot,
                at: [screen.x + OFFSET_PX, screen.y + OFFSET_PX],
                size: [chars * CHAR_PX, LINE_PX],
                priority: f64::from(spec.importance) + plan.angle,
            });
        }
        scratch.blocked.clear();
        for (text, node, visibility) in &navigation {
            if *visibility == Visibility::Hidden {
                continue;
            }
            let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
                continue;
            };
            #[expect(
                clippy::cast_precision_loss,
                reason = "E-CAST: a label length in characters, exact in f32"
            )]
            let width = text.0.chars().count() as f32 * CHAR_PX;
            scratch.blocked.push(LabelBox {
                slot: usize::MAX,
                at: [left, top],
                size: [width, LINE_PX],
                priority: f64::INFINITY,
            });
        }
        place_labels(
            &mut scratch.boxes,
            &scratch.blocked,
            LABEL_CAP,
            &mut scratch.placed,
        );
    }
    for (label, mut text, mut node, mut visibility) in &mut slots {
        let Some(placed) = scratch.placed.get(label.slot) else {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            continue;
        };
        let Some((record, _)) = layer.planned().find(|(_, plan)| plan.slot == placed.slot) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        if scratch.written.len() < LABEL_CAP {
            scratch.written.resize(LABEL_CAP, None);
        }
        if let Some(written) = scratch.written.get_mut(label.slot)
            && *written != Some(record.id)
        {
            text.0 = label_text(record);
            *written = Some(record.id);
        }
        node.left = Val::Px(placed.at[0]);
        node.top = Val::Px(placed.at[1]);
        *visibility = Visibility::Visible;
    }
}

/// A developer label: the part's category, and the stable ID of the marker
/// that owns it (shared parts belong to the cell and carry none).
fn label_text(record: &PartRecord) -> String {
    match record.parent {
        Parent::Marker(id) => format!("{} #{id}", record.id.kind.token()),
        Parent::Cell => record.id.kind.token().to_string(),
    }
}

/// Characters of [`label_text`], without building it.
fn label_len(record: &PartRecord) -> usize {
    let token = record.id.kind.token().len();
    match record.parent {
        Parent::Marker(id) => {
            token + 2 + id.checked_ilog10().map_or(1, |digits| digits as usize + 1)
        }
        Parent::Cell => token,
    }
}
