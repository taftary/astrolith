//! Shared two-column layout for the Guides and Settings screens (#473, T4).
//!
//! Everything here is `pub(crate)`: only this crate uses it. [`PanelPlugin`]
//! owns the shell both screens share: title bar (`Back`, title, close),
//! category list with expandable sub-categories, content area with
//! mouse-wheel scrolling, and the narrow collapse (list first, then content
//! full width with `Back`) under [`NARROW_BREAK_PX`](crate::tokens::NARROW_BREAK_PX).
//! The Guides (`guides_ui`, T6) and Settings (`settings_ui`, T5) modules
//! populate the lists with [`PanelItem`] buttons and the content areas; this
//! module only drives selection, expansion, scrolling, and visibility. The
//! visible scrollbar thumbs live in the `scrollbar` child module (#518).

mod scrollbar;

use crate::chrome;
use crate::fonts::{Icon, TextRole};
use crate::menu::Screen;
use crate::tokens::{
    BUTTON_RADIUS_PX, CARD_LINE_TINT, CHROME_CYAN, LAYER_SCREEN, MENU_BUTTON_DIAMETER_PX,
    MENU_DIM_TINT, MENU_MARGIN_PX, MENU_SELECT_TINT, MENU_TEXT_TINT, NARROW_BREAK_PX,
    PANEL_BAR_WIDTH_PX, PANEL_CARD_PERCENT, PANEL_LIST_WIDTH_PX, PANEL_NARROW_MARGIN_PX,
    PICK_TOUCH_DP, SCREEN_BUTTON_HEIGHT_PX, SCROLLBAR_WIDTH_PX,
};
use bevy::ecs::system::Single;
use bevy::prelude::*;
use bevy::ui::{Node, PositionType, RelativeCursorPosition, Val, widget::Text};

/// Shared panel state: selection, expansion, and the narrow detail view.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PanelState {
    /// Selected item id (`""` before the first selection).
    pub selected: String,
    /// Expanded category ids.
    pub expanded: Vec<String>,
    /// Narrow mode shows the detail view instead of the list.
    pub detail: bool,
}

impl PanelState {
    /// Selects `id` (clears the narrow detail view's list).
    pub(crate) fn select(&mut self, id: &str) {
        self.selected = id.to_string();
    }

    /// Expands `id` when collapsed, collapses it when expanded.
    pub(crate) fn toggle(&mut self, id: &str) {
        if self.is_expanded(id) {
            self.expanded.retain(|held| held != id);
        } else {
            self.expanded.push(id.to_string());
        }
    }

    /// Whether `id` is expanded.
    #[must_use]
    pub(crate) fn is_expanded(&self, id: &str) -> bool {
        self.expanded.iter().any(|held| held == id)
    }
}

/// Whether `window_width` counts as narrow (pure: the 720 px break).
#[must_use]
pub(crate) fn is_narrow(window_width: f32) -> bool {
    window_width < NARROW_BREAK_PX
}

/// Next scroll offset after a wheel delta (pure: 24 px per line, top-clamped).
#[must_use]
pub(crate) fn scroll_next(offset: f32, lines: f32) -> f32 {
    (offset - lines * 24.0).max(0.0)
}

/// Clamps a content scroll offset to its range (pure: top at zero, bottom at
/// the overflow past the viewport; content that fits stays at zero).
#[must_use]
pub(crate) fn scroll_clamp(offset: f32, content_len: f32, viewport_len: f32) -> f32 {
    offset.clamp(0.0, (content_len - viewport_len).max(0.0))
}

/// Row index of `selected` in `screen`'s list, categories and leaves in
/// tree order (pure: the spawn order follows the same walk).
#[must_use]
pub(crate) fn list_row_index(screen: Screen, selected: &str) -> Option<usize> {
    let tree = tree_of(screen)?;
    let mut index = 0;
    for node in tree {
        if node.id == selected {
            return Some(index);
        }
        index += 1;
        for child in node.children {
            if child.id == selected {
                return Some(index);
            }
            index += 1;
        }
    }
    None
}

/// List offset that reveals `selected` with the least movement (pure: rows
/// share one fixed height, so the top of row `i` sits at `i` rows down).
#[must_use]
pub(crate) fn reveal_offset(selected: usize, row_h: f32, viewport: f32, current: f32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: list rows stay in the dozens, exactly representable"
    )]
    let index = selected as f32;
    let top = index * row_h;
    if top < current {
        top
    } else if top + row_h > current + viewport {
        top + row_h - viewport
    } else {
        current
    }
}

/// Whether the window is currently narrow (synced from the window when one
/// exists; tests set it directly).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct PanelNarrow(pub bool);

/// Marks the shell root of one screen's panel.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PanelRoot(pub Screen);

/// Marks the title-bar back button of one screen's panel.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PanelBack(pub Screen);

/// Marks the title-bar close button of one screen's panel.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PanelClose(pub Screen);

/// Marks the category-list container of one screen's panel.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PanelList(pub Screen);

/// Marks the content container of one screen's panel.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PanelContent(pub Screen);

/// One category or sub-category button in a panel list.
///
/// `settings_ui` (T5) and `guides_ui` (T6) spawn these into the [`PanelList`]
/// containers; `has_children` categories toggle, leaves select.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub(crate) struct PanelItem {
    /// Screen this item belongs to.
    pub screen: Screen,
    /// Stable id (also the [`PanelState`] key).
    pub id: String,
    /// Whether tapping toggles children instead of selecting.
    pub has_children: bool,
    /// Indent depth (0 for categories, 1 for sub-categories).
    pub depth: u32,
}

/// Marks the selection bar inside a [`PanelItem`] button.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub(crate) struct PanelBar {
    /// Item id this bar belongs to.
    pub item: String,
}

/// Marks the label text inside a [`PanelItem`] button.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub(crate) struct PanelLabel {
    /// Item id this label belongs to.
    pub item: String,
}

/// Shared two-column layout for Guides and Settings.
pub(crate) struct PanelPlugin;

impl Plugin for PanelPlugin {
    /// Registers the panel state and the shell systems.
    fn build(&self, app: &mut App) {
        app.init_resource::<PanelState>()
            .init_resource::<PanelNarrow>()
            .init_resource::<PanelScrolled>()
            .init_resource::<PanelListScrolled>()
            .add_plugins(scrollbar::ScrollbarPlugin)
            .add_systems(Startup, spawn_panel_shells)
            .add_systems(
                Update,
                (
                    sync_narrow_from_window,
                    panel_item_click,
                    panel_back_click,
                    panel_close_click,
                    panel_wheel_scroll,
                    sync_panel_scroll,
                    sync_panel_visibility,
                    sync_panel_selection,
                    sync_panel_frame,
                    sync_panel_list_scroll,
                ),
            );
    }
}

/// Panel title per screen.
fn title_of(screen: Screen) -> &'static str {
    match screen {
        Screen::Guides => "Guides",
        Screen::Settings => "Settings",
        Screen::Universe | Screen::Menu => "",
    }
}

/// One category or sub-category in a panel list (static tree data).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PanelNode {
    /// Stable id (also the [`PanelState`] key).
    pub id: &'static str,
    /// Button label.
    pub label: &'static str,
    /// Sub-categories (empty for leaves).
    pub children: &'static [PanelNode],
}

/// Settings list tree from Spec v1 (categories with sub-categories).
pub(crate) const SETTINGS_TREE: &[PanelNode] = &[
    PanelNode {
        id: "display",
        label: "Display",
        children: &[
            PanelNode {
                id: "scene",
                label: "Scene",
                children: &[],
            },
            PanelNode {
                id: "window",
                label: "Window",
                children: &[],
            },
        ],
    },
    PanelNode {
        id: "controls",
        label: "Controls",
        children: &[
            PanelNode {
                id: "flight",
                label: "Flight",
                children: &[],
            },
            PanelNode {
                id: "keys",
                label: "Keys",
                children: &[],
            },
            PanelNode {
                id: "touch",
                label: "Touch",
                children: &[],
            },
        ],
    },
    PanelNode {
        id: "views",
        label: "Views",
        children: &[],
    },
    PanelNode {
        id: "accessibility",
        label: "Accessibility",
        children: &[],
    },
    PanelNode {
        id: "developer",
        label: "Developer",
        children: &[],
    },
];

/// Guides list tree from Spec v1 (About and Help with their pages).
pub(crate) const GUIDES_TREE: &[PanelNode] = &[
    PanelNode {
        id: "about",
        label: "About",
        children: &[
            PanelNode {
                id: "what",
                label: "What is Astrolith",
                children: &[],
            },
            PanelNode {
                id: "ladder",
                label: "The scale ladder",
                children: &[],
            },
        ],
    },
    PanelNode {
        id: "help",
        label: "Help",
        children: &[
            PanelNode {
                id: "getting-started",
                label: "Getting started",
                children: &[],
            },
            PanelNode {
                id: "navigation",
                label: "Navigation",
                children: &[],
            },
            PanelNode {
                id: "normal-mode",
                label: "Normal mode",
                children: &[],
            },
            PanelNode {
                id: "free-flight",
                label: "Free flight",
                children: &[],
            },
            PanelNode {
                id: "saved-views",
                label: "Saved views",
                children: &[],
            },
            PanelNode {
                id: "touch",
                label: "Touch",
                children: &[],
            },
            PanelNode {
                id: "planets",
                label: "Planets",
                children: &[],
            },
        ],
    },
];

/// Tree per screen (`None` for screens without a panel).
#[must_use]
pub(crate) fn tree_of(screen: Screen) -> Option<&'static [PanelNode]> {
    match screen {
        Screen::Settings => Some(SETTINGS_TREE),
        Screen::Guides => Some(GUIDES_TREE),
        Screen::Universe | Screen::Menu => None,
    }
}

/// Spawns the shell (title, list, content) for Guides and Settings.
pub(crate) fn spawn_panel_shells(mut commands: Commands) {
    for screen in [Screen::Guides, Screen::Settings] {
        let mut list_entity = Entity::PLACEHOLDER;
        commands
            .spawn((
                PanelRoot(screen),
                Name::new(format!("panel-{}", title_of(screen))),
                GlobalZIndex(LAYER_SCREEN),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    padding: frame_padding(false),
                    ..default()
                },
                Visibility::Hidden,
            ))
            .with_children(|frame| {
                frame
                    .spawn((
                        Name::new("panel-card"),
                        chrome::card(Val::Percent(100.0), Val::Percent(100.0)),
                    ))
                    .with_children(|panel| {
                        panel
                            .spawn((
                                Name::new("panel-titlebar"),
                                Node {
                                    width: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Row,
                                    justify_content: JustifyContent::SpaceBetween,
                                    align_items: AlignItems::Center,
                                    border: UiRect::bottom(Val::Px(1.0)),
                                    padding: UiRect::bottom(Val::Px(MENU_MARGIN_PX)),
                                    margin: UiRect::bottom(Val::Px(MENU_MARGIN_PX)),
                                    ..default()
                                },
                                BorderColor::all(CARD_LINE_TINT),
                            ))
                            .with_children(|bar| {
                                bar.spawn((
                                    PanelBack(screen),
                                    Name::new("panel-back"),
                                    chrome::button(
                                        Val::Px(MENU_BUTTON_DIAMETER_PX),
                                        Val::Px(MENU_BUTTON_DIAMETER_PX),
                                        false,
                                    ),
                                ))
                                .with_children(|back| {
                                    back.spawn(chrome::button_face(false))
                                        .with_children(|face| {
                                            face.spawn((
                                                Icon::Back.text(),
                                                TextColor(MENU_TEXT_TINT),
                                            ));
                                        });
                                });
                                bar.spawn((
                                    Text::new(title_of(screen)),
                                    TextRole::Title,
                                    TextColor(MENU_TEXT_TINT),
                                ));
                                bar.spawn((
                                    PanelClose(screen),
                                    Name::new("panel-close"),
                                    chrome::button(
                                        Val::Px(MENU_BUTTON_DIAMETER_PX),
                                        Val::Px(MENU_BUTTON_DIAMETER_PX),
                                        false,
                                    ),
                                ))
                                .with_children(|close| {
                                    close
                                        .spawn(chrome::button_face(false))
                                        .with_children(|face| {
                                            face.spawn((
                                                Icon::Close.text(),
                                                TextColor(MENU_TEXT_TINT),
                                            ));
                                        });
                                });
                            });
                        panel
                            .spawn((
                                Name::new("panel-body"),
                                Node {
                                    width: Val::Percent(100.0),
                                    flex_grow: 1.0,
                                    // Stays inside the card: children
                                    // stretch to it instead of a percent
                                    // height that resolved against the card.
                                    min_height: Val::Px(0.0),
                                    flex_direction: FlexDirection::Row,
                                    ..default()
                                },
                            ))
                            .with_children(|body| {
                                list_entity = body
                                    .spawn((
                                        PanelList(screen),
                                        Name::new("panel-list"),
                                        RelativeCursorPosition::default(),
                                        Node {
                                            width: Val::Px(PANEL_LIST_WIDTH_PX),
                                            overflow: Overflow::scroll_y(),
                                            flex_direction: FlexDirection::Column,
                                            border: UiRect::right(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BorderColor::all(CARD_LINE_TINT),
                                    ))
                                    .id();
                                body.spawn((
                                    PanelContent(screen),
                                    Name::new("panel-content"),
                                    Node {
                                        flex_grow: 1.0,
                                        flex_direction: FlexDirection::Column,
                                        // Clipped, scrollable content: the wheel
                                        // offset (`PanelScrolled`) is copied
                                        // into `ScrollPosition` below (#518).
                                        overflow: Overflow::scroll_y(),
                                        // Gutter the scrollbar thumb overlays.
                                        scrollbar_width: SCROLLBAR_WIDTH_PX,
                                        ..default()
                                    },
                                ));
                            });
                        chrome::spawn_card_trim(panel);
                    });
            });
        if let Some(tree) = tree_of(screen) {
            commands.entity(list_entity).with_children(|items| {
                let mut spawn_item =
                    |id: &'static str, label: &'static str, has_children: bool, depth: u32| {
                        #[expect(
                            clippy::cast_precision_loss,
                            reason = "E-CAST: indent depth 0-1, exactly representable"
                        )]
                        let indent =
                            MENU_MARGIN_PX + PANEL_BAR_WIDTH_PX * 2.0 + depth as f32 * 16.0;
                        items
                            .spawn((
                                PanelItem {
                                    screen,
                                    id: id.to_string(),
                                    has_children,
                                    depth,
                                },
                                Name::new(format!("panel-item-{id}")),
                                Button,
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(SCREEN_BUTTON_HEIGHT_PX),
                                    // Shrinks to fit the card, never under
                                    // the 48 px touch size.
                                    min_height: Val::Px(PICK_TOUCH_DP),
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::left(Val::Px(indent)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    border_radius: BorderRadius::all(Val::Px(BUTTON_RADIUS_PX)),
                                    ..default()
                                },
                                BorderColor::all(Color::NONE),
                            ))
                            .with_children(|item| {
                                item.spawn((
                                    PanelBar {
                                        item: id.to_string(),
                                    },
                                    Name::new("panel-select-bar"),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(MENU_MARGIN_PX),
                                        width: Val::Px(PANEL_BAR_WIDTH_PX),
                                        height: Val::Percent(60.0),
                                        ..default()
                                    },
                                    BackgroundColor(MENU_SELECT_TINT),
                                    Visibility::Hidden,
                                ));
                                item.spawn((
                                    PanelLabel {
                                        item: id.to_string(),
                                    },
                                    Text::new(label),
                                    TextRole::ListItem,
                                    TextColor(MENU_DIM_TINT),
                                ));
                            });
                    };
                for node in tree {
                    spawn_item(node.id, node.label, !node.children.is_empty(), 0);
                    for child in node.children {
                        spawn_item(child.id, child.label, false, 1);
                    }
                }
            });
        }
    }
}

/// Space around the Guides/Settings card: 5 % per side when wide (a 90 %
/// card), a small margin when narrow (pure).
#[must_use]
pub(crate) fn frame_padding(narrow: bool) -> UiRect {
    if narrow {
        UiRect::all(Val::Px(PANEL_NARROW_MARGIN_PX))
    } else {
        UiRect::all(Val::Percent((100.0 - PANEL_CARD_PERCENT) / 2.0))
    }
}

/// Resizes the card frame when the window crosses the narrow break.
pub(crate) fn sync_panel_frame(
    narrow: Res<PanelNarrow>,
    mut roots: Query<&mut Node, With<PanelRoot>>,
) {
    if !narrow.is_changed() {
        return;
    }
    let want = frame_padding(narrow.0);
    for mut node in &mut roots {
        if node.padding != want {
            node.padding = want;
        }
    }
}

/// Reads the window width into [`PanelNarrow`] (skipped without a window).
pub(crate) fn sync_narrow_from_window(window: Single<&Window>, mut narrow: ResMut<PanelNarrow>) {
    narrow.0 = is_narrow(window.width());
}

/// Toggles categories and selects leaves; narrow taps open the detail view.
pub(crate) fn panel_item_click(
    mut state: ResMut<PanelState>,
    narrow: Res<PanelNarrow>,
    items: Query<(&Interaction, &PanelItem), Changed<Interaction>>,
) {
    for (interaction, item) in &items {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if item.has_children {
            state.toggle(&item.id);
        } else {
            state.select(&item.id);
            if narrow.0 {
                state.detail = true;
            }
        }
    }
}

/// Back goes to the list in narrow detail, else to the Menu screen.
pub(crate) fn panel_back_click(
    mut state: ResMut<PanelState>,
    narrow: Res<PanelNarrow>,
    mut next: ResMut<NextState<Screen>>,
    buttons: Query<&Interaction, (Changed<Interaction>, With<PanelBack>)>,
) {
    for interaction in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if narrow.0 && state.detail {
            state.detail = false;
        } else {
            next.set(Screen::Menu);
        }
    }
}

/// Close always returns to the universe.
pub(crate) fn panel_close_click(
    mut next: ResMut<NextState<Screen>>,
    buttons: Query<&Interaction, (Changed<Interaction>, With<PanelClose>)>,
) {
    for interaction in &buttons {
        if *interaction == Interaction::Pressed {
            next.set(Screen::Universe);
        }
    }
}

/// Mouse-wheel lines scroll the open screen's list when hovered, else its
/// content (#701: wheel only, no touch-drag).
pub(crate) fn panel_wheel_scroll(
    mut wheels: MessageReader<bevy::input::mouse::MouseWheel>,
    screen: Res<State<Screen>>,
    mut scrolled: ResMut<PanelScrolled>,
    mut list_scrolled: ResMut<PanelListScrolled>,
    lists: Query<(&PanelList, &RelativeCursorPosition)>,
) {
    use bevy::input::mouse::MouseScrollUnit;
    let in_panel = matches!(*screen.get(), Screen::Guides | Screen::Settings);
    let over_list = lists
        .iter()
        .any(|(list, hover)| list.0 == *screen.get() && hover.cursor_over());
    for event in wheels.read() {
        if !in_panel {
            continue;
        }
        let lines = match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / 24.0,
        };
        if over_list {
            list_scrolled.0 = scroll_next(list_scrolled.0, lines);
        } else {
            scrolled.0 = scroll_next(scrolled.0, lines);
        }
    }
}

/// Accumulated content scroll offset in logical pixels.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct PanelScrolled(pub f32);

/// Accumulated list scroll offset in logical pixels (#701).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct PanelListScrolled(pub f32);

/// Copies the wheel offset into the open screen's `ScrollPosition`.
///
/// A new selection (or screen) resets to the top first, so every page starts
/// at its top (#518). The offset clamps to the live content range; without a
/// layout run yet (headless tests) everything measures zero and stays at top.
pub(crate) fn sync_panel_scroll(
    state: Res<PanelState>,
    screen: Res<State<Screen>>,
    mut scrolled: ResMut<PanelScrolled>,
    mut contents: Query<(&PanelContent, &ComputedNode, &mut ScrollPosition)>,
    mut previous: Local<String>,
) {
    if screen.is_changed() || state.selected != *previous {
        *previous = state.selected.clone();
        scrolled.0 = 0.0;
    }
    let open = *screen.get();
    for (content, computed, mut position) in &mut contents {
        if content.0 != open {
            continue;
        }
        scrolled.0 = scroll_clamp(scrolled.0, computed.content_size.y, computed.size.y);
        if position.y != scrolled.0 {
            position.y = scrolled.0;
        }
    }
}

/// Copies the wheel offset into the open screen's list `ScrollPosition`.
///
/// A new screen resets to the top; a new selection reveals its row with
/// the least movement (#701), so the selected page is always visible. The
/// reveal also re-runs when the live layout arrives or changes: the shot
/// setup selects before the first layout pass, when rows still measure
/// zero. Manual wheel scrolls never retrigger it, so the two never fight.
/// The thumb gutter opens only while rows overflow, keeping fitting lists
/// pixel-identical to the unscrollable shell.
pub(crate) fn sync_panel_list_scroll(
    state: Res<PanelState>,
    screen: Res<State<Screen>>,
    mut scrolled: ResMut<PanelListScrolled>,
    mut lists: Query<(&PanelList, &ComputedNode, &mut ScrollPosition, &mut Node)>,
    mut previous: Local<Option<(Screen, String, f32, f32)>>,
) {
    let open = *screen.get();
    for (list, computed, mut position, mut node) in &mut lists {
        if list.0 != open {
            continue;
        }
        let viewport = computed.size.y;
        let stacked = computed.content_size.y;
        let changed = previous.as_ref().is_none_or(|held| {
            held.0 != open || held.1 != state.selected || held.2 != viewport || held.3 != stacked
        });
        *previous = Some((open, state.selected.clone(), viewport, stacked));
        let overflows = stacked > viewport;
        let gutter = if overflows { SCROLLBAR_WIDTH_PX } else { 0.0 };
        if node.scrollbar_width != gutter {
            node.scrollbar_width = gutter;
        }
        if !changed {
            continue;
        }
        let base = if screen.is_changed() { 0.0 } else { scrolled.0 };
        let mut offset = base;
        if let Some(index) = list_row_index(open, &state.selected) {
            offset = reveal_offset(index, SCREEN_BUTTON_HEIGHT_PX, viewport, base);
        }
        offset = scroll_clamp(offset, stacked, viewport);
        scrolled.0 = offset;
        if position.y != offset {
            position.y = offset;
        }
    }
}

/// Shows the shell of the open screen; narrow detail hides the list.
pub(crate) fn sync_panel_visibility(
    screen: Res<State<Screen>>,
    narrow: Res<PanelNarrow>,
    state: Res<PanelState>,
    mut roots: Query<(&PanelRoot, &mut Visibility), Without<PanelList>>,
    mut lists: Query<(&PanelList, &mut Visibility), Without<PanelRoot>>,
) {
    let showing = *screen.get();
    for (root, mut visibility) in &mut roots {
        let want = if root.0 == showing {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
    }
    for (list, mut visibility) in &mut lists {
        let want = if list.0 == showing && !(narrow.0 && state.detail) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
    }
}

/// Shows the selection bar (plus brighter text) on the selected item only.
pub(crate) fn sync_panel_selection(
    state: Res<PanelState>,
    mut bars: Query<(&PanelBar, &mut Visibility), Without<PanelLabel>>,
    mut labels: Query<(&PanelLabel, &mut TextColor), Without<PanelBar>>,
    mut items: Query<(&PanelItem, &mut BorderColor)>,
) {
    if !state.is_changed() {
        return;
    }
    for (item, mut border) in &mut items {
        let want = if item.id == state.selected {
            BorderColor::all(CHROME_CYAN)
        } else {
            BorderColor::all(Color::NONE)
        };
        if *border != want {
            *border = want;
        }
    }
    for (bar, mut visibility) in &mut bars {
        let want = if bar.item == state.selected {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
    }
    for (label, mut color) in &mut labels {
        let want = if label.item == state.selected {
            TextColor(Color::WHITE)
        } else {
            TextColor(MENU_DIM_TINT)
        };
        if *color != want {
            *color = want;
        }
    }
}

#[cfg(test)]
mod tests;
