//! Shared two-column layout for the Guides and Settings screens (#473, T4).
//!
//! Everything here is `pub(crate)`: only this crate uses it. [`PanelPlugin`]
//! owns the shell both screens share: title bar (`Back`, title, close),
//! category list with expandable sub-categories, content area with
//! mouse-wheel scrolling, and the narrow collapse (list first, then content
//! full width with `Back`) under [`NARROW_BREAK_PX`](crate::tokens::NARROW_BREAK_PX).
//! The Guides (`guides_ui`, T6) and Settings (`settings_ui`, T5) modules
//! populate the lists with [`PanelItem`] buttons and the content areas; this
//! module only drives selection, expansion, scrolling, and visibility.

use crate::menu::Screen;
use crate::tokens::{
    MENU_DIM_TINT, MENU_MARGIN_PX, MENU_PANEL_TINT, MENU_SELECT_TINT, MENU_TEXT_TINT,
    NARROW_BREAK_PX, PANEL_BAR_WIDTH_PX, PANEL_LIST_WIDTH_PX, SCREEN_BUTTON_HEIGHT_PX,
};
use bevy::ecs::system::Single;
use bevy::prelude::*;
use bevy::ui::{Node, PositionType, Val, widget::Text};

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
            .add_systems(Startup, spawn_panel_shells)
            .add_systems(
                Update,
                (
                    sync_narrow_from_window,
                    panel_item_click,
                    panel_back_click,
                    panel_close_click,
                    panel_wheel_scroll,
                    sync_panel_visibility,
                    sync_panel_selection,
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
                id: "free-flight",
                label: "Free flight",
                children: &[],
            },
            PanelNode {
                id: "saved-views",
                label: "Saved views",
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
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                BackgroundColor(MENU_PANEL_TINT),
                Visibility::Hidden,
            ))
            .with_children(|panel| {
                panel
                    .spawn((
                        Name::new("panel-titlebar"),
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(SCREEN_BUTTON_HEIGHT_PX),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(MENU_MARGIN_PX)),
                            ..default()
                        },
                    ))
                    .with_children(|bar| {
                        bar.spawn((
                            PanelBack(screen),
                            Name::new("panel-back"),
                            Button,
                            Node {
                                width: Val::Px(SCREEN_BUTTON_HEIGHT_PX),
                                height: Val::Px(SCREEN_BUTTON_HEIGHT_PX),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                        ))
                        .with_children(|back| {
                            back.spawn((Text::new("<"), TextColor(MENU_TEXT_TINT)));
                        });
                        bar.spawn((Text::new(title_of(screen)), TextColor(MENU_TEXT_TINT)));
                        bar.spawn((
                            PanelClose(screen),
                            Name::new("panel-close"),
                            Button,
                            Node {
                                width: Val::Px(SCREEN_BUTTON_HEIGHT_PX),
                                height: Val::Px(SCREEN_BUTTON_HEIGHT_PX),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                        ))
                        .with_children(|close| {
                            close.spawn((Text::new("X"), TextColor(MENU_TEXT_TINT)));
                        });
                    });
                panel
                    .spawn((
                        Name::new("panel-body"),
                        Node {
                            width: Val::Percent(100.0),
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Row,
                            ..default()
                        },
                    ))
                    .with_children(|body| {
                        list_entity = body
                            .spawn((
                                PanelList(screen),
                                Name::new("panel-list"),
                                Node {
                                    width: Val::Px(PANEL_LIST_WIDTH_PX),
                                    height: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    ..default()
                                },
                            ))
                            .id();
                        body.spawn((
                            PanelContent(screen),
                            Name::new("panel-content"),
                            Node {
                                flex_grow: 1.0,
                                height: Val::Percent(100.0),
                                flex_direction: FlexDirection::Column,
                                ..default()
                            },
                        ));
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
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::left(Val::Px(indent)),
                                    ..default()
                                },
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

/// Mouse-wheel lines scroll the open screen's content.
pub(crate) fn panel_wheel_scroll(
    mut wheels: MessageReader<bevy::input::mouse::MouseWheel>,
    screen: Res<State<Screen>>,
    mut scrolled: ResMut<PanelScrolled>,
) {
    use bevy::input::mouse::MouseScrollUnit;
    let in_panel = matches!(*screen.get(), Screen::Guides | Screen::Settings);
    for event in wheels.read() {
        if !in_panel {
            continue;
        }
        let lines = match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / 24.0,
        };
        scrolled.0 = scroll_next(scrolled.0, lines);
    }
}

/// Accumulated content scroll offset in logical pixels.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct PanelScrolled(pub f32);

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
) {
    if !state.is_changed() {
        return;
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
mod tests {
    use super::*;
    use crate::tokens::MENU_BUTTON_DIAMETER_PX;
    use bevy::asset::AssetPlugin;
    use bevy::ecs::message::Messages;
    use bevy::gizmos::{
        GizmoAsset, GizmoHandles,
        config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    };
    use bevy::input::mouse::{MouseMotion, MouseWheel};
    use bevy::mesh::Mesh;
    use bevy::pbr::StandardMaterial;

    /// Headless app with the dive pipeline plus menu and panel.
    ///
    /// Lists come pre-populated from the trees ([`SETTINGS_TREE`],
    /// [`GUIDES_TREE`]); `PanelScrolled` is owned by [`PanelPlugin`].
    fn headless_app() -> App {
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

    /// Presses the [`PanelItem`] button holding `id` for one update.
    fn press_item(app: &mut App, id: &str) {
        let entity = app
            .world_mut()
            .query::<(Entity, &PanelItem)>()
            .iter(app.world())
            .find(|(_, item)| item.id == id)
            .map(|(entity, _)| entity)
            .expect("item exists");
        app.world_mut()
            .entity_mut(entity)
            .insert(Interaction::Pressed);
        app.update();
        app.world_mut().entity_mut(entity).insert(Interaction::None);
        app.update();
    }

    #[test]
    fn categories_expand_and_collapse_leaves_select() {
        let mut app = headless_app();
        app.update();
        press_item(&mut app, "display");
        let state = app.world().resource::<PanelState>();
        assert!(state.is_expanded("display"), "first tap expands");
        assert_eq!(state.selected, "", "toggling selects nothing");
        press_item(&mut app, "display");
        assert!(
            !app.world().resource::<PanelState>().is_expanded("display"),
            "second tap collapses"
        );
        press_item(&mut app, "scene");
        assert_eq!(
            app.world().resource::<PanelState>().selected,
            "scene",
            "leaf tap selects"
        );
    }

    #[test]
    fn narrow_flow_lists_first_then_details_with_back() {
        use bevy::input::mouse::MouseScrollUnit;
        let mut app = headless_app();
        app.update();
        app.world_mut().resource_mut::<PanelNarrow>().0 = true;
        app.world_mut()
            .resource_mut::<NextState<Screen>>()
            .set(Screen::Settings);
        app.update();
        assert!(
            !app.world().resource::<PanelState>().detail,
            "narrow opens on the list"
        );
        press_item(&mut app, "scene");
        assert!(
            app.world().resource::<PanelState>().detail,
            "tapping a leaf opens the detail view"
        );
        // The wheel scrolls the detail content.
        app.world_mut()
            .resource_mut::<Messages<MouseWheel>>()
            .write(MouseWheel {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y: -2.0,
                window: Entity::PLACEHOLDER,
                phase: bevy::input::touch::TouchPhase::Moved,
            });
        app.update();
        assert!(
            app.world().resource::<PanelScrolled>().0 > 0.0,
            "wheel lines scroll the content"
        );
        // Back returns to the list, Back again leaves for the menu.
        let back = app
            .world_mut()
            .query_filtered::<Entity, With<PanelBack>>()
            .iter(app.world())
            .next()
            .expect("back button exists");
        app.world_mut()
            .entity_mut(back)
            .insert(Interaction::Pressed);
        app.update();
        assert!(
            !app.world().resource::<PanelState>().detail,
            "back returns to the list"
        );
        app.world_mut()
            .entity_mut(back)
            .insert(Interaction::Pressed);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<Screen>>().get(),
            Screen::Menu,
            "back from the list returns to the menu"
        );
    }

    #[test]
    fn screen_tokens_meet_the_spec_sizes_and_contrast() {
        use universe_core::contrast::relative_luminance;
        const {
            assert!(SCREEN_BUTTON_HEIGHT_PX >= 48.0);
            assert!(MENU_BUTTON_DIAMETER_PX >= 48.0);
            assert!(NARROW_BREAK_PX == 720.0);
        }
        // WCAG contrast of the text tint over the nearly opaque panel tint.
        let text = relative_luminance([1.0, 1.0, 1.0]);
        let panel = relative_luminance([0.05, 0.06, 0.09]);
        let (lighter, darker) = if text > panel {
            (text, panel)
        } else {
            (panel, text)
        };
        let ratio = (lighter + 0.05) / (darker + 0.05);
        assert!(ratio >= 4.5, "text contrast {ratio} meets the 4.5:1 floor");
    }

    #[test]
    fn narrow_break_is_below_wide_and_above_zero() {
        assert!(!is_narrow(1280.0), "desktop width is wide");
        assert!(!is_narrow(720.0), "the break itself is wide");
        assert!(is_narrow(719.0), "below the break is narrow");
        assert_eq!(scroll_next(0.0, -2.0), 48.0, "two lines down scroll 48 px");
        assert_eq!(scroll_next(48.0, 1.0), 24.0, "one line up scrolls back");
        assert_eq!(scroll_next(0.0, 5.0), 0.0, "scrolling clamps at the top");
    }
}
