//! Menu button, Menu screen, the Esc rule, and pause/blocking (#473, T3).
//!
//! Everything here is `pub(crate)`: only this crate uses it. [`Screen`] is
//! the app screen; [`MenuPlugin`] owns the menu button, the Menu screen, and
//! the Esc rule. While the screen is not [`Screen::Universe`] the dive
//! pipeline does not run (gated in [`crate::DivePlugin`]), so the universe
//! pauses and no click or game key reaches it.

use crate::chrome;
use crate::fonts::{Icon, TextRole};
use crate::tokens::{
    BUTTON_INNER_INSET_PX, BUTTON_RADIUS_PX, CARD_LINE_TINT, CHROME_DANGER_TEXT, LAYER_SCREEN,
    LAYER_UNIVERSE, MENU_BUTTON_DIAMETER_PX, MENU_BUTTON_GAP_PX, MENU_CARD_WIDTH_PX,
    MENU_MARGIN_PX, MENU_TEXT_TINT, SCREEN_BUTTON_HEIGHT_PX,
};
use bevy::prelude::*;
use bevy::ui::{Node, PositionType, Val, widget::Text};

/// App screen: the universe or one full-window screen on top of it.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum Screen {
    /// Flying the universe; the menu button shows top-right.
    #[default]
    Universe,
    /// Menu screen with Resume, Guides, Settings, Quit.
    Menu,
    /// Guides screen (built by `guides_ui`, T6).
    Guides,
    /// Settings screen (built by `settings_ui`, T5).
    Settings,
}

/// Next screen for one Esc tap from `screen` (pure: the Spec v1 Esc rule).
///
/// Esc opens the Menu from the universe, steps back from Guides and Settings
/// to the Menu, and returns from the Menu to the universe. It never quits.
#[must_use]
pub(crate) fn esc_next(screen: Screen) -> Screen {
    match screen {
        Screen::Universe => Screen::Menu,
        Screen::Menu => Screen::Universe,
        Screen::Guides | Screen::Settings => Screen::Menu,
    }
}

/// Marks the top-right menu button (visible in the universe only).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MenuButton;

/// Marks the Menu screen root (visible in [`Screen::Menu`] only).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MenuRoot;

/// Menu-screen action carried by one button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuAction {
    /// Back to the universe.
    Resume,
    /// To the Guides screen.
    Guides,
    /// To the Settings screen.
    Settings,
    /// Close the app.
    Quit,
    /// Close button: back to the universe.
    Close,
}

impl MenuAction {
    /// Button label.
    #[must_use]
    pub(crate) fn label(self) -> &'static str {
        match self {
            MenuAction::Resume => "Resume",
            MenuAction::Guides => "Guides",
            MenuAction::Settings => "Settings",
            MenuAction::Quit => "Quit",
            MenuAction::Close => "X",
        }
    }

    /// Lucide icon drawn left of the label (alone for [`MenuAction::Close`]).
    #[must_use]
    pub(crate) fn icon(self) -> Icon {
        match self {
            MenuAction::Resume => Icon::Resume,
            MenuAction::Guides => Icon::Guides,
            MenuAction::Settings => Icon::Settings,
            MenuAction::Quit => Icon::Quit,
            MenuAction::Close => Icon::Close,
        }
    }

    /// Every Menu-screen button, top to bottom (Spec v1 order).
    pub(crate) const ORDER: [MenuAction; 5] = [
        MenuAction::Resume,
        MenuAction::Guides,
        MenuAction::Settings,
        MenuAction::Quit,
        MenuAction::Close,
    ];
}

/// Menu button, Menu screen, and the Esc rule.
pub(crate) struct MenuPlugin;

impl Plugin for MenuPlugin {
    /// Registers the [`Screen`] state, the menu UI, and the Esc rule.
    fn build(&self, app: &mut App) {
        use bevy::state::app::StatesPlugin;
        if !app.is_plugin_added::<StatesPlugin>() {
            app.add_plugins(StatesPlugin);
        }
        app.init_state::<Screen>()
            .add_systems(Startup, spawn_menu_ui)
            .add_systems(Update, (esc_rule, menu_button_click, menu_action_click))
            .add_systems(Update, sync_menu_visibility);
    }
}

/// Spawns the menu button and the hidden Menu screen.
pub(crate) fn spawn_menu_ui(mut commands: Commands) {
    // The menu button is a double-border chrome button carrying the Lucide
    // menu icon (#473 Spec v3: no placeholder "M").
    commands
        .spawn((
            MenuButton,
            Name::new("menu-button"),
            GlobalZIndex(LAYER_UNIVERSE),
            chrome::button(
                Val::Px(MENU_BUTTON_DIAMETER_PX),
                Val::Px(MENU_BUTTON_DIAMETER_PX),
                false,
            ),
        ))
        .insert(Node {
            position_type: PositionType::Absolute,
            right: Val::Px(MENU_MARGIN_PX),
            top: Val::Px(MENU_MARGIN_PX),
            width: Val::Px(MENU_BUTTON_DIAMETER_PX),
            height: Val::Px(MENU_BUTTON_DIAMETER_PX),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(BUTTON_RADIUS_PX)),
            padding: UiRect::all(Val::Px(BUTTON_INNER_INSET_PX)),
            ..default()
        })
        .with_children(|button| {
            button
                .spawn(chrome::button_face(false))
                .with_children(|face| {
                    face.spawn((Icon::Menu.text(), TextColor(MENU_TEXT_TINT)));
                });
        });
    // Transparent full-window layer centring the framed Menu card; the dim
    // veil (chrome) sits one layer below.
    commands
        .spawn((
            MenuRoot,
            Name::new("menu-screen"),
            GlobalZIndex(LAYER_SCREEN),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|screen| {
            screen
                .spawn((
                    Name::new("menu-card"),
                    chrome::card(Val::Px(MENU_CARD_WIDTH_PX), Val::Auto),
                ))
                .with_children(|card| {
                    spawn_menu_header(card);
                    card.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(MENU_BUTTON_GAP_PX),
                        ..default()
                    })
                    .with_children(|list| {
                        for action in MenuAction::ORDER {
                            if action != MenuAction::Close {
                                spawn_menu_entry(list, action);
                            }
                        }
                    });
                    chrome::spawn_card_trim(card);
                });
        });
}

/// Card header: ASTROLITH title, the close icon, and the line under them.
fn spawn_menu_header(card: &mut ChildSpawnerCommands) {
    card.spawn((
        Name::new("menu-header"),
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            border: UiRect::bottom(Val::Px(1.0)),
            padding: UiRect::bottom(Val::Px(MENU_MARGIN_PX)),
            margin: UiRect::bottom(Val::Px(MENU_BUTTON_GAP_PX)),
            ..default()
        },
        BorderColor::all(CARD_LINE_TINT),
    ))
    .with_children(|header| {
        header.spawn((
            Name::new("menu-title"),
            Text::new("ASTROLITH"),
            TextRole::Title,
            TextColor(MENU_TEXT_TINT),
        ));
        header
            .spawn((
                MenuAction::Close,
                Name::new(format!("menu-{}", MenuAction::Close.label())),
                chrome::button(
                    Val::Px(MENU_BUTTON_DIAMETER_PX),
                    Val::Px(MENU_BUTTON_DIAMETER_PX),
                    false,
                ),
            ))
            .with_children(|button| {
                button
                    .spawn(chrome::button_face(false))
                    .with_children(|face| {
                        face.spawn((MenuAction::Close.icon().text(), TextColor(MENU_TEXT_TINT)));
                    });
            });
    });
}

/// One Menu entry: double-border button with its icon and label.
fn spawn_menu_entry(list: &mut ChildSpawnerCommands, action: MenuAction) {
    let danger = action == MenuAction::Quit;
    let tint = if danger {
        CHROME_DANGER_TEXT
    } else {
        MENU_TEXT_TINT
    };
    list.spawn((
        action,
        Name::new(format!("menu-{}", action.label())),
        chrome::button(
            Val::Percent(100.0),
            Val::Px(SCREEN_BUTTON_HEIGHT_PX),
            danger,
        ),
    ))
    .with_children(|button| {
        button
            .spawn(chrome::button_face(danger))
            .with_children(|face| {
                face.spawn((action.icon().text(), TextColor(tint)));
                face.spawn((
                    Text::new(action.label()),
                    TextRole::Body,
                    TextColor(MENU_TEXT_TINT),
                ));
            });
    });
}

/// Shows the menu button in the universe and the Menu screen in the menu.
pub(crate) fn sync_menu_visibility(
    screen: Res<State<Screen>>,
    mut buttons: Query<&mut Visibility, (With<MenuButton>, Without<MenuRoot>)>,
    mut roots: Query<&mut Visibility, With<MenuRoot>>,
) {
    let showing = *screen.get();
    for mut visibility in &mut buttons {
        let want = if showing == Screen::Universe {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
    }
    for mut visibility in &mut roots {
        let want = if showing == Screen::Menu {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
    }
}

/// Applies the Esc rule: one tap steps per [`esc_next`], never quits.
pub(crate) fn esc_rule(
    keys: Res<ButtonInput<KeyCode>>,
    screen: Res<State<Screen>>,
    mut next: ResMut<NextState<Screen>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next.set(esc_next(*screen.get()));
    }
}

/// Opens the Menu screen when the menu button is pressed.
pub(crate) fn menu_button_click(
    screen: Res<State<Screen>>,
    mut next: ResMut<NextState<Screen>>,
    buttons: Query<&Interaction, (Changed<Interaction>, With<MenuButton>)>,
) {
    if *screen.get() != Screen::Universe {
        return;
    }
    for interaction in &buttons {
        if *interaction == Interaction::Pressed {
            next.set(Screen::Menu);
        }
    }
}

/// Runs the Menu-screen button that was pressed.
pub(crate) fn menu_action_click(
    screen: Res<State<Screen>>,
    mut next: ResMut<NextState<Screen>>,
    mut exit: MessageWriter<AppExit>,
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
) {
    if *screen.get() != Screen::Menu {
        return;
    }
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            MenuAction::Resume | MenuAction::Close => next.set(Screen::Universe),
            MenuAction::Guides => next.set(Screen::Guides),
            MenuAction::Settings => next.set(Screen::Settings),
            MenuAction::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Universe;
    use bevy::asset::AssetPlugin;
    use bevy::ecs::message::Messages;
    use bevy::gizmos::{
        GizmoAsset, GizmoHandles,
        config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    };
    use bevy::input::mouse::{MouseMotion, MouseWheel};
    use bevy::mesh::Mesh;
    use bevy::pbr::StandardMaterial;

    /// Headless app with the dive pipeline plus the menu (no window or GPU).
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

    /// Taps `key` for exactly one update.
    fn tap(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release(key);
        input.clear();
        // State transitions apply on the following schedule run.
        app.update();
    }

    /// Current screen.
    fn screen(app: &App) -> Screen {
        *app.world().resource::<State<Screen>>().get()
    }

    /// Sets the screen directly (headless stand-in for a button press).
    fn set_screen(app: &mut App, to: Screen) {
        app.world_mut().resource_mut::<NextState<Screen>>().set(to);
        app.update();
    }

    #[test]
    fn esc_follows_the_spec_rule_and_never_quits() {
        let mut app = headless_app();
        assert_eq!(screen(&app), Screen::Universe);
        tap(&mut app, KeyCode::Escape);
        assert_eq!(screen(&app), Screen::Menu);
        tap(&mut app, KeyCode::Escape);
        assert_eq!(screen(&app), Screen::Universe);
        set_screen(&mut app, Screen::Settings);
        tap(&mut app, KeyCode::Escape);
        assert_eq!(screen(&app), Screen::Menu);
        set_screen(&mut app, Screen::Guides);
        tap(&mut app, KeyCode::Escape);
        assert_eq!(screen(&app), Screen::Menu);
        let exits = app.world().resource::<Messages<AppExit>>();
        assert!(exits.is_empty(), "Esc never quits the app");
    }

    #[test]
    fn esc_next_is_the_spec_table() {
        assert_eq!(esc_next(Screen::Universe), Screen::Menu);
        assert_eq!(esc_next(Screen::Menu), Screen::Universe);
        assert_eq!(esc_next(Screen::Guides), Screen::Menu);
        assert_eq!(esc_next(Screen::Settings), Screen::Menu);
    }

    /// Content entities of a chrome button: the children of its face.
    fn face_children(world: &World, children: &Children) -> Vec<Entity> {
        children
            .iter()
            .filter_map(|face| world.get::<Children>(face))
            .flat_map(|inner| inner.iter().collect::<Vec<_>>())
            .collect()
    }

    #[test]
    fn menu_screen_lists_resume_guides_settings_quit_in_order() {
        let mut app = headless_app();
        app.update();
        // One button per action: its icon, then its label text (#473 Spec v3);
        // Close carries the close icon alone.
        let mut pairs: Vec<(MenuAction, String)> = Vec::new();
        for (action, children) in app
            .world_mut()
            .query::<(&MenuAction, &Children)>()
            .iter(app.world())
        {
            let texts: Vec<(String, bool)> = face_children(app.world(), children)
                .into_iter()
                .filter_map(|child| {
                    let text = app.world().get::<Text>(child)?;
                    let icon = app.world().get::<TextRole>(child) == Some(&TextRole::Icon);
                    Some((text.0.clone(), icon))
                })
                .collect();
            assert_eq!(
                texts.first().map(|(text, icon)| (text.as_str(), *icon)),
                Some((action.icon().glyph().to_string().as_str(), true)),
                "button {action:?} starts with its icon"
            );
            let label = if *action == MenuAction::Close {
                "X".to_string()
            } else {
                texts
                    .get(1)
                    .map(|(text, _)| text.clone())
                    .unwrap_or_default()
            };
            pairs.push((*action, label));
        }
        assert_eq!(
            pairs.len(),
            MenuAction::ORDER.len(),
            "one button per action"
        );
        for action in MenuAction::ORDER {
            let found = pairs.iter().find(|(held, _)| *held == action);
            assert_eq!(
                found.map(|(_, text)| text.as_str()),
                Some(action.label()),
                "button {action:?} carries its label"
            );
        }
        assert_eq!(
            MenuAction::ORDER.map(MenuAction::label),
            ["Resume", "Guides", "Settings", "Quit", "X"]
        );
    }

    #[test]
    fn menu_button_carries_a_white_glyph() {
        let mut app = headless_app();
        app.update();
        // The button is a dark square on a near-black sky: without a text
        // child it reads as missing. One non-empty white glyph is required.
        let mut glyphs = 0;
        for (_button, children) in app
            .world_mut()
            .query::<(&MenuButton, &Children)>()
            .iter(app.world())
        {
            for child in face_children(app.world(), children) {
                let Some(text) = app.world().get::<Text>(child).map(|t| t.0.clone()) else {
                    continue;
                };
                assert!(!text.is_empty(), "menu glyph text is not empty");
                assert_eq!(
                    app.world().get::<TextColor>(child),
                    Some(&TextColor(MENU_TEXT_TINT)),
                    "menu glyph reads white on the dark button"
                );
                glyphs += 1;
            }
        }
        // Querying (&MenuButton, &Children) pairs each button with its own
        // children, so exactly one button carries exactly one glyph.
        assert_eq!(glyphs, 1, "one menu button with one glyph");
    }

    #[test]
    fn menu_button_shows_in_the_universe_and_hides_behind_screens() {
        let mut app = headless_app();
        app.update();
        let visible = |app: &mut App| {
            app.world_mut()
                .query_filtered::<&Visibility, With<MenuButton>>()
                .iter(app.world())
                .all(|v| *v == Visibility::Visible)
        };
        let root_shown = |app: &mut App| {
            app.world_mut()
                .query_filtered::<&Visibility, With<MenuRoot>>()
                .iter(app.world())
                .all(|v| *v == Visibility::Visible)
        };
        assert!(visible(&mut app), "button shows in the universe");
        assert!(!root_shown(&mut app), "menu hidden in the universe");
        set_screen(&mut app, Screen::Menu);
        assert!(!visible(&mut app), "button hides behind the menu");
        assert!(root_shown(&mut app), "menu shows");
        set_screen(&mut app, Screen::Universe);
        assert!(visible(&mut app), "button returns with the universe");
    }

    #[test]
    fn keys_and_clicks_do_not_move_the_universe_behind_a_screen() {
        let mut app = headless_app();
        app.update();
        let before = app.world().resource::<Universe>().path.offset();
        // Universe: holding Up dives.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowUp);
        for _ in 0..30 {
            app.update();
        }
        {
            let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            input.release(KeyCode::ArrowUp);
            input.clear();
        }
        let flown = app.world().resource::<Universe>().path.offset();
        assert_ne!(flown, before, "the dive moves in the universe");
        // Menu: the same hold is blocked.
        set_screen(&mut app, Screen::Menu);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowUp);
        for _ in 0..30 {
            app.update();
        }
        {
            let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            input.release(KeyCode::ArrowUp);
            input.clear();
        }
        assert_eq!(
            app.world().resource::<Universe>().path.offset(),
            flown,
            "the universe pauses behind the menu"
        );
        // A click behind the menu targets nothing.
        let destination = app
            .world()
            .resource::<crate::input::Navigation>()
            .destination;
        app.world_mut()
            .resource_mut::<crate::input::Navigation>()
            .hover = Some(1);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        {
            let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            buttons.release(MouseButton::Left);
            buttons.clear();
        }
        assert_eq!(
            app.world()
                .resource::<crate::input::Navigation>()
                .destination,
            destination,
            "clicks behind the menu target nothing"
        );
    }
}
