//! Quit confirmation pop-up (#473 Spec v3, criteria 24 and 26-29; T4).
//!
//! Quit on the Menu opens "Quit Astrolith?" with Cancel and Quit on the
//! pop-up layer, above the screens. A second dim veil under the card covers
//! the window and swallows every click outside the card. Esc closes the
//! pop-up only (the Esc row lives in [`crate::menu::esc_rule`]).

use crate::chrome;
use crate::fonts::TextRole;
use crate::menu::Screen;
use crate::tokens::{
    BUTTON_INNER_INSET_PX, BUTTON_RADIUS_PX, CHROME_DANGER_TEXT, LAYER_POPUP, LAYER_POPUP_DIM,
    MENU_BUTTON_GAP_PX, MENU_MARGIN_PX, MENU_TEXT_TINT, POPUP_CARD_WIDTH_PX, POPUP_VEIL_DARKNESS,
    SCREEN_BUTTON_HEIGHT_PX,
};
use bevy::prelude::*;
use bevy::ui::FocusPolicy;

/// Whether the Quit pop-up is open.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QuitPopup {
    /// The pop-up shows and blocks everything behind it.
    pub open: bool,
}

/// Marks the pop-up veil: dims the screen and blocks outside clicks.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QuitVeil;

/// Marks the full-window layer that centres the pop-up card.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QuitPopupRoot;

/// One pop-up button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuitChoice {
    /// Close the pop-up, stay on the Menu.
    Cancel,
    /// Close the app.
    Quit,
}

impl QuitChoice {
    /// Button label.
    #[must_use]
    pub(crate) fn label(self) -> &'static str {
        match self {
            QuitChoice::Cancel => "Cancel",
            QuitChoice::Quit => "Quit",
        }
    }
}

/// The pop-up title.
pub(crate) const QUIT_TITLE: &str = "Quit Astrolith?";

/// Quit pop-up plugin.
pub(crate) struct QuitPopupPlugin;

impl Plugin for QuitPopupPlugin {
    /// Registers the pop-up state, its UI, clicks, and visibility.
    fn build(&self, app: &mut App) {
        app.init_resource::<QuitPopup>()
            .add_systems(Startup, spawn_quit_popup)
            .add_systems(
                Update,
                (quit_choice_click, close_off_menu, sync_quit_popup).chain(),
            );
    }
}

/// Spawns the hidden veil and the hidden pop-up card.
pub(crate) fn spawn_quit_popup(mut commands: Commands) {
    commands.spawn((
        QuitVeil,
        Name::new("quit-veil"),
        GlobalZIndex(LAYER_POPUP_DIM),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(chrome::veil_color(POPUP_VEIL_DARKNESS)),
        FocusPolicy::Block,
        Interaction::default(),
        Visibility::Hidden,
    ));
    commands
        .spawn((
            QuitPopupRoot,
            Name::new("quit-popup"),
            GlobalZIndex(LAYER_POPUP),
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
        .with_children(|layer| {
            layer
                .spawn((
                    Name::new("quit-card"),
                    chrome::card(Val::Px(POPUP_CARD_WIDTH_PX), Val::Auto),
                ))
                .with_children(|card| {
                    card.spawn((
                        Name::new("quit-title"),
                        Text::new(QUIT_TITLE),
                        TextRole::Title,
                        TextColor(MENU_TEXT_TINT),
                        Node {
                            margin: UiRect::bottom(Val::Px(MENU_MARGIN_PX)),
                            ..default()
                        },
                    ));
                    card.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(MENU_BUTTON_GAP_PX),
                        ..default()
                    })
                    .with_children(|row| {
                        spawn_choice(row, QuitChoice::Cancel);
                        spawn_choice(row, QuitChoice::Quit);
                    });
                    chrome::spawn_card_trim(card);
                });
        });
}

/// One pop-up button; Quit carries the red danger look.
fn spawn_choice(row: &mut ChildSpawnerCommands, choice: QuitChoice) {
    let danger = choice == QuitChoice::Quit;
    row.spawn((
        choice,
        Name::new(format!("quit-{}", choice.label())),
        chrome::button(Val::Auto, Val::Px(SCREEN_BUTTON_HEIGHT_PX), danger),
    ))
    .insert(Node {
        flex_grow: 1.0,
        flex_basis: Val::Px(0.0),
        height: Val::Px(SCREEN_BUTTON_HEIGHT_PX),
        border: UiRect::all(Val::Px(1.0)),
        border_radius: BorderRadius::all(Val::Px(BUTTON_RADIUS_PX)),
        padding: UiRect::all(Val::Px(BUTTON_INNER_INSET_PX)),
        ..default()
    })
    .with_children(|button| {
        button
            .spawn(chrome::button_face(danger))
            .with_children(|face| {
                face.spawn((
                    Text::new(choice.label()),
                    TextRole::Body,
                    TextColor(if danger {
                        CHROME_DANGER_TEXT
                    } else {
                        MENU_TEXT_TINT
                    }),
                ));
            });
    });
}

/// Runs the pop-up button that was pressed (only while open).
pub(crate) fn quit_choice_click(
    mut popup: ResMut<QuitPopup>,
    mut exit: MessageWriter<AppExit>,
    buttons: Query<(&Interaction, &QuitChoice), Changed<Interaction>>,
) {
    if !popup.open {
        return;
    }
    for (interaction, choice) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match choice {
            QuitChoice::Cancel => popup.open = false,
            QuitChoice::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

/// The pop-up belongs to the Menu: leaving the Menu closes it.
pub(crate) fn close_off_menu(screen: Res<State<Screen>>, mut popup: ResMut<QuitPopup>) {
    if popup.open && *screen.get() != Screen::Menu {
        popup.open = false;
    }
}

/// The pop-up's two layers: the veil and the card root.
type PopupLayers = Or<(With<QuitVeil>, With<QuitPopupRoot>)>;

/// Shows the veil and the card while the pop-up is open.
pub(crate) fn sync_quit_popup(
    popup: Res<QuitPopup>,
    mut layers: Query<&mut Visibility, PopupLayers>,
) {
    let want = if popup.open {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut layers {
        if *visibility != want {
            *visibility = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::MenuAction;
    use crate::tokens::LAYER_SCREEN;
    use crate::window_tests::headless_app;
    use bevy::ecs::message::Messages;

    /// App on the Menu screen.
    fn on_menu() -> App {
        let mut app = headless_app();
        app.update();
        app.world_mut()
            .resource_mut::<NextState<Screen>>()
            .set(Screen::Menu);
        app.update();
        app.update();
        app
    }

    /// The entity carrying component value `want`.
    fn find<C: Component + PartialEq>(app: &mut App, want: C) -> Entity {
        app.world_mut()
            .query::<(Entity, &C)>()
            .iter(app.world())
            .find(|(_, held)| **held == want)
            .map(|(entity, _)| entity)
            .expect("entity exists")
    }

    /// Presses `entity` for one update, then releases it.
    fn press(app: &mut App, entity: Entity) {
        *app.world_mut().get_mut::<Interaction>(entity).unwrap() = Interaction::Pressed;
        app.update();
        *app.world_mut().get_mut::<Interaction>(entity).unwrap() = Interaction::None;
        app.update();
    }

    /// Presses the pop-up button `choice`.
    fn press_choice(app: &mut App, choice: QuitChoice) {
        let entity = find(app, choice);
        press(app, entity);
    }

    /// Presses the Menu's Quit entry.
    fn press_menu_quit(app: &mut App) {
        let entity = find(app, MenuAction::Quit);
        press(app, entity);
    }

    fn is_open(app: &App) -> bool {
        app.world().resource::<QuitPopup>().open
    }

    fn screen(app: &App) -> Screen {
        *app.world().resource::<State<Screen>>().get()
    }

    fn exited(app: &App) -> bool {
        !app.world().resource::<Messages<AppExit>>().is_empty()
    }

    /// Whether the pop-up card layer shows.
    fn card_shown(app: &mut App) -> bool {
        app.world_mut()
            .query_filtered::<&Visibility, With<QuitPopupRoot>>()
            .iter(app.world())
            .all(|v| *v == Visibility::Visible)
    }

    #[test]
    fn menu_quit_opens_the_popup_and_keeps_the_app_open() {
        let mut app = on_menu();
        assert!(!is_open(&app));
        assert!(!card_shown(&mut app), "hidden until Quit");
        press_menu_quit(&mut app);
        assert!(is_open(&app), "Quit opens the pop-up");
        assert!(card_shown(&mut app), "pop-up card shows");
        assert!(!exited(&app), "the app is still open");
        assert_eq!(screen(&app), Screen::Menu);
        let title: Vec<String> = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|t| t.0.clone())
            .filter(|t| t == QUIT_TITLE)
            .collect();
        assert_eq!(title, [QUIT_TITLE], "title says Quit Astrolith?");
    }

    #[test]
    fn cancel_closes_the_popup_and_stays_on_the_menu() {
        let mut app = on_menu();
        press_menu_quit(&mut app);
        press_choice(&mut app, QuitChoice::Cancel);
        assert!(!is_open(&app));
        assert!(!card_shown(&mut app));
        assert_eq!(screen(&app), Screen::Menu);
        assert!(!exited(&app));
    }

    #[test]
    fn esc_closes_only_the_popup() {
        let mut app = on_menu();
        press_menu_quit(&mut app);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::Escape);
            keys.clear();
        }
        app.update();
        assert!(!is_open(&app), "Esc closes the pop-up");
        assert_eq!(screen(&app), Screen::Menu, "and stays on the Menu");
        assert!(!exited(&app));
    }

    #[test]
    fn quit_in_the_popup_closes_the_app() {
        let mut app = on_menu();
        press_menu_quit(&mut app);
        press_choice(&mut app, QuitChoice::Quit);
        assert!(exited(&app), "Quit in the pop-up closes the app");
    }

    #[test]
    fn outside_clicks_do_nothing_while_open() {
        let mut app = on_menu();
        press_menu_quit(&mut app);
        // The veil covers the window above the screens and blocks focus.
        let (z, policy) = app
            .world_mut()
            .query_filtered::<(&GlobalZIndex, &FocusPolicy), With<QuitVeil>>()
            .single(app.world())
            .map(|(z, p)| (z.0, *p))
            .expect("one veil");
        assert!(
            z > LAYER_SCREEN && z < LAYER_POPUP,
            "veil between screen and card"
        );
        assert_eq!(policy, FocusPolicy::Block, "veil swallows clicks");
        // A click landing on the veil changes nothing.
        let veil = find(&mut app, QuitVeil);
        press(&mut app, veil);
        assert!(is_open(&app));
        assert!(!exited(&app));
        // Even if a Menu button behind were pressed, it does not react.
        let resume = find(&mut app, MenuAction::Resume);
        press(&mut app, resume);
        assert_eq!(screen(&app), Screen::Menu, "the Menu behind does not react");
        assert!(is_open(&app));
    }
}
