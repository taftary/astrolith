//! Panel scrollbar thumbs: one visible bar per content area (#518).
//!
//! Child module of `panel`: only the panel shell uses it. Bevy reserves
//! scrollbar space and reports gutter/thumb rects but draws no bar, so this
//! module draws it: [`ScrollbarPlugin`] spawns one thumb per [`PanelContent`]
//! and keeps its size and position on the live layout. Dragging the thumb
//! writes the shared [`PanelScrolled`] offset (the single writer), which
//! [`sync_panel_scroll`](crate::panel::sync_panel_scroll) clamps and copies
//! into the content's `ScrollPosition`, so wheel and drag never fight.

use super::{PanelContent, PanelScrolled, scroll_clamp};
use crate::menu::Screen;
use crate::tokens::{SCROLLBAR_THUMB_MIN_PX, SCROLLBAR_THUMB_TINT, SCROLLBAR_WIDTH_PX};
use bevy::ecs::system::Single;
use bevy::prelude::*;
use bevy::ui::{Node, Val};

/// Marks the scrollbar thumb of one screen's content area.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PanelThumb(pub Screen);

/// Active thumb drag, if any (cleared on mouse release).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct ThumbDrag(pub Option<DragState>);

/// A thumb held from `grab_y` with the content at `start`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct DragState {
    /// Content offset in logical pixels when grabbed.
    pub start: f32,
    /// Cursor height in logical pixels when grabbed.
    pub grab_y: f32,
}

/// Thumb top and length in content-box pixels (pure).
///
/// Returns `None` when everything fits (no bar needed). Short content gets
/// at least [`SCROLLBAR_THUMB_MIN_PX`] of grabbable thumb.
#[must_use]
pub(crate) fn thumb_geometry(
    content_len: f32,
    viewport_len: f32,
    offset: f32,
) -> Option<(f32, f32)> {
    if content_len <= viewport_len {
        return None;
    }
    let len = (viewport_len * viewport_len / content_len)
        .max(SCROLLBAR_THUMB_MIN_PX)
        .min(viewport_len);
    let max_offset = (content_len - viewport_len).max(0.0);
    let travel = (viewport_len - len).max(0.0);
    let top = if max_offset > 0.0 {
        (offset.clamp(0.0, max_offset) / max_offset) * travel
    } else {
        0.0
    };
    Some((top, len))
}

/// Content offset for a thumb drag (pure: dragging down scrolls down).
#[must_use]
pub(crate) fn thumb_drag_offset(
    start: f32,
    grab_y: f32,
    cursor_y: f32,
    content_len: f32,
    viewport_len: f32,
) -> f32 {
    let per_px = if viewport_len > 0.0 {
        content_len / viewport_len
    } else {
        0.0
    };
    scroll_clamp(
        start + (cursor_y - grab_y) * per_px,
        content_len,
        viewport_len,
    )
}

/// Visible scrollbar thumbs, one per panel content area.
pub(crate) struct ScrollbarPlugin;

impl Plugin for ScrollbarPlugin {
    /// Registers the drag state and the thumb systems.
    fn build(&self, app: &mut App) {
        app.init_resource::<ThumbDrag>().add_systems(
            Update,
            (
                spawn_panel_thumbs,
                sync_panel_thumbs,
                thumb_drag_start,
                thumb_drag_move,
            ),
        );
    }
}

/// Spawns one hidden thumb per content area.
///
/// One-shot: runs once both panel shells exist, then parks. The thumb is an
/// absolute child of its content, so it overlays the reserved scrollbar
/// gutter; [`IgnoreScroll`] keeps it fixed while the content moves under it.
pub(crate) fn spawn_panel_thumbs(
    mut commands: Commands,
    contents: Query<(Entity, &PanelContent), Without<PanelThumb>>,
    mut spawned: Local<bool>,
) {
    if *spawned || contents.is_empty() {
        return;
    }
    *spawned = true;
    for (entity, content) in &contents {
        let screen = content.0;
        commands.entity(entity).with_children(|content| {
            content.spawn((
                PanelThumb(screen),
                Name::new(format!("panel-thumb-{screen:?}").to_lowercase()),
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(SCROLLBAR_WIDTH_PX),
                    height: Val::Px(SCROLLBAR_THUMB_MIN_PX),
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(SCROLLBAR_THUMB_TINT),
                IgnoreScroll(BVec2::new(false, true)),
                Visibility::Hidden,
            ));
        });
    }
}

/// Sizes and places every thumb on the live layout; hides it when the page
/// fits its content area.
pub(crate) fn sync_panel_thumbs(
    contents: Query<(&ComputedNode, &Children), With<PanelContent>>,
    mut thumbs: Query<(&PanelThumb, &mut Node, &mut Visibility)>,
) {
    for (computed, children) in &contents {
        let geometry = thumb_geometry(
            computed.content_size.y,
            computed.size.y,
            computed.scroll_position.y,
        );
        for child in children.iter() {
            let Ok((_thumb, mut node, mut visibility)) = thumbs.get_mut(child) else {
                continue;
            };
            match geometry {
                Some((top, len)) => {
                    node.top = Val::Px(top);
                    node.height = Val::Px(len);
                    if node.display != Display::Flex {
                        node.display = Display::Flex;
                    }
                    if *visibility != Visibility::Visible {
                        *visibility = Visibility::Visible;
                    }
                }
                None => {
                    if node.display != Display::None {
                        node.display = Display::None;
                    }
                    if *visibility != Visibility::Hidden {
                        *visibility = Visibility::Hidden;
                    }
                }
            }
        }
    }
}

/// Grabs the thumb: records the content offset and cursor height a drag
/// measures from (skipped without a window or cursor).
pub(crate) fn thumb_drag_start(
    mut drag: ResMut<ThumbDrag>,
    thumbs: Query<(&Interaction, &PanelThumb), Changed<Interaction>>,
    contents: Query<(&PanelContent, &ScrollPosition)>,
    window: Single<&Window>,
) {
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    for (interaction, thumb) in &thumbs {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let start = contents
            .iter()
            .find(|(content, _)| content.0 == thumb.0)
            .map_or(0.0, |(_, position)| position.y);
        drag.0 = Some(DragState {
            start,
            grab_y: cursor.y,
        });
    }
}

/// Moves a grabbed thumb: the cursor travel, scaled to content pixels,
/// becomes the shared scroll offset (skipped without a window or cursor;
/// the drag clears on mouse release).
pub(crate) fn thumb_drag_move(
    mut drag: ResMut<ThumbDrag>,
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    contents: Query<(&PanelContent, &ComputedNode)>,
    screen: Res<State<Screen>>,
    mut scrolled: ResMut<PanelScrolled>,
) {
    let Some(held) = drag.0 else {
        return;
    };
    if !buttons.pressed(MouseButton::Left) {
        drag.0 = None;
        return;
    }
    let (Some(cursor), Some((_, computed))) = (
        window.cursor_position(),
        contents
            .iter()
            .find(|(content, _)| content.0 == *screen.get()),
    ) else {
        return;
    };
    scrolled.0 = thumb_drag_offset(
        held.start,
        held.grab_y,
        cursor.y,
        computed.content_size.y,
        computed.size.y,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_geometry_needs_overflow_and_fills_the_gutter() {
        assert_eq!(thumb_geometry(100.0, 200.0, 0.0), None, "fits: no bar");
        assert_eq!(thumb_geometry(200.0, 200.0, 0.0), None, "exact: no bar");
        // Viewport 200 in content 400: thumb 100 long at the top.
        assert_eq!(thumb_geometry(400.0, 200.0, 0.0), Some((0.0, 100.0)));
        // Bottom offset parks the thumb at the gutter end.
        assert_eq!(thumb_geometry(400.0, 200.0, 200.0), Some((100.0, 100.0)));
        // Middle offset centres the thumb.
        assert_eq!(thumb_geometry(400.0, 200.0, 100.0), Some((50.0, 100.0)));
    }

    #[test]
    fn short_content_keeps_a_grabbable_thumb() {
        // Viewport 200 in content 4000: proportional thumb would be 10 px,
        // floored to the minimum and parked at the gutter top.
        assert_eq!(
            thumb_geometry(4000.0, 200.0, 0.0),
            Some((0.0, SCROLLBAR_THUMB_MIN_PX))
        );
        const {
            assert!(SCROLLBAR_THUMB_MIN_PX >= 16.0);
        }
    }

    #[test]
    fn thumb_drag_scales_cursor_travel_to_content() {
        // Viewport 200 in content 400: one cursor pixel moves two content px.
        assert_eq!(thumb_drag_offset(0.0, 100.0, 150.0, 400.0, 200.0), 100.0);
        assert_eq!(thumb_drag_offset(100.0, 150.0, 100.0, 400.0, 200.0), 0.0);
        assert_eq!(
            thumb_drag_offset(0.0, 100.0, 900.0, 400.0, 200.0),
            200.0,
            "drag clamps at the bottom"
        );
        assert_eq!(
            thumb_drag_offset(0.0, 100.0, 150.0, 100.0, 200.0),
            0.0,
            "content that fits never moves"
        );
    }

    /// Headless app with the full dive pipeline (thumbs included).
    fn headless_app() -> App {
        use bevy::asset::AssetPlugin;
        use bevy::gizmos::{
            GizmoAsset, GizmoHandles,
            config::{DefaultGizmoConfigGroup, GizmoConfigStore},
        };
        use bevy::input::mouse::{MouseMotion, MouseWheel};
        use bevy::mesh::Mesh;
        use bevy::pbr::StandardMaterial;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.add_message::<AppExit>();
        app.add_message::<MouseWheel>();
        app.add_message::<MouseMotion>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.init_resource::<GizmoConfigStore>();
        app.init_gizmo_group::<DefaultGizmoConfigGroup>();
        app.init_asset::<GizmoAsset>();
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.init_resource::<GizmoHandles>();
        app.add_plugins(crate::DivePlugin);
        app
    }

    #[test]
    fn every_content_area_gets_one_hidden_thumb() {
        let mut app = headless_app();
        app.update();
        let mut screens: Vec<Screen> = app
            .world_mut()
            .query::<&PanelThumb>()
            .iter(app.world())
            .map(|thumb| thumb.0)
            .collect();
        screens.sort_by_key(|screen| *screen as u8);
        assert_eq!(
            screens,
            vec![Screen::Guides, Screen::Settings],
            "one thumb per content area"
        );
        // Headless layout measures nothing, so every thumb stays parked.
        let parked = app
            .world_mut()
            .query::<(&PanelThumb, &Node)>()
            .iter(app.world())
            .all(|(_, node)| node.display == Display::None);
        assert!(parked, "thumbs hide until pages overflow");
    }

    #[test]
    fn sync_places_the_thumb_on_live_layout() {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = headless_app();
        app.update();
        // Fake a tall Settings page: viewport 200, content 400, at top.
        let content = app
            .world_mut()
            .query_filtered::<Entity, With<PanelContent>>()
            .iter(app.world())
            .next()
            .expect("content exists");
        app.world_mut().entity_mut(content).insert(ComputedNode {
            size: Vec2::new(300.0, 200.0),
            content_size: Vec2::new(300.0, 400.0),
            ..ComputedNode::DEFAULT
        });
        let _ = app.world_mut().run_system_once(sync_panel_thumbs);
        let shown: Vec<(Val, Val)> = app
            .world_mut()
            .query::<(&PanelThumb, &Node)>()
            .iter(app.world())
            .map(|(_, node)| (node.top, node.height))
            .collect();
        assert!(
            shown.contains(&(Val::Px(0.0), Val::Px(100.0))),
            "one thumb sits at the gutter top, half the gutter long: {shown:?}"
        );
    }
}
