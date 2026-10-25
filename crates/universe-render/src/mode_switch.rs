//! Mode toggle button and chooser (#955).
//!
//! The top-left corner carries one 48 px chrome button showing the current
//! mode's icon only. Clicking it opens a chooser with one row per mode
//! (icon plus label, current mode highlighted). The Display > Scene > Mode
//! reminder switch hides both. Everything here is `pub(crate)`.

use crate::Universe;
use crate::chrome;
use crate::fonts::{Icon, TextRole};
use crate::input::{Autopilot, Flight, FlightMode, Navigation};
use crate::tokens::{
    BUTTON_INNER_INSET_PX, BUTTON_RADIUS_PX, CARD_LINE_TINT, LAYER_UNIVERSE,
    MENU_BUTTON_DIAMETER_PX, MENU_BUTTON_GAP_PX, MENU_MARGIN_PX, MENU_SELECT_TINT, MENU_TEXT_TINT,
};
use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use bevy::ui::{Node, PositionType, Val, widget::Text};

/// Marks the top-left mode button (visible in the universe only).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModeButton;

/// Icon glyph inside the mode button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModeButtonIcon;

/// Marks the hidden mode chooser card under the button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModeChooser;

/// Full-window transparent veil behind the chooser: a click on it closes
/// the chooser, and it blocks game clicks while open.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModeVeil;

/// One chooser row, carrying the mode it picks.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModeRow(pub FlightMode);

/// Whether the chooser is open (closed by default).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ChooserOpen(pub bool);

/// This frame's press or tap was taken by the mode switch (the button, a
/// row, or an outside click while open): the game's own click handling and
/// the touch reader must not act on it too (#955 correction round 5: the
/// same click idled the Normal pick's takeover autopilot).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ModeSwitchClaim(pub bool);

/// Every mode in chooser order.
pub(crate) const MODES: [FlightMode; 3] = [FlightMode::Normal, FlightMode::Free, FlightMode::Dive];

/// Lucide icon for one mode (pure).
#[must_use]
pub(crate) const fn icon_for_mode(mode: FlightMode) -> Icon {
    match mode {
        FlightMode::Normal => Icon::ModeNormal,
        FlightMode::Free => Icon::ModeFree,
        FlightMode::Dive => Icon::ModeDive,
    }
}

/// Chooser label for one mode (pure).
#[must_use]
pub(crate) const fn label_for_mode(mode: FlightMode) -> &'static str {
    match mode {
        FlightMode::Normal => "Normal",
        FlightMode::Free => "Free",
        FlightMode::Dive => "Dive",
    }
}

/// Mode toggle button, chooser, and their sync.
pub(crate) struct ModeSwitchPlugin;

impl Plugin for ModeSwitchPlugin {
    /// Registers the chooser state, the button UI, and its sync.
    fn build(&self, app: &mut App) {
        use bevy::state::condition::in_state;
        app.init_resource::<ChooserOpen>()
            .init_resource::<ModeSwitchClaim>()
            .add_systems(Startup, spawn_mode_switch)
            .add_systems(Update, sync_mode_switch)
            .add_systems(
                Update,
                (
                    // Cursor math first: it must see the press while the
                    // chooser is still open (#955 correction round 3). The
                    // veil close runs after, so a row tap can no longer be
                    // consumed as an outside click in the same frame.
                    mode_chooser_click,
                    mode_button_click,
                    mode_row_click,
                    mode_veil_click,
                    touch_opens_chooser,
                )
                    .chain()
                    // Before the touch reader clears its tap flags: the
                    // opener and the tap path see this frame's taps next
                    // frame, exactly like the long-standing ambiguous order.
                    .before(crate::touch::read_touches)
                    .run_if(in_state(crate::menu::Screen::Universe)),
            );
    }
}

/// Spawns the mode button and the hidden chooser.
pub(crate) fn spawn_mode_switch(mut commands: Commands) {
    // Click-catching veil first (below the button and chooser): a click on
    // it closes the chooser, and it blocks game clicks while open.
    commands.spawn((
        ModeVeil,
        Name::new("mode-veil"),
        GlobalZIndex(LAYER_UNIVERSE),
        FocusPolicy::Block,
        // Interaction: without it the focus system never presses the veil,
        // so outside clicks would do nothing (#955 correction round 1).
        Interaction::None,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        Visibility::Hidden,
    ));
    commands
        .spawn((
            ModeButton,
            Name::new("mode-button"),
            GlobalZIndex(LAYER_UNIVERSE),
            chrome::button(
                Val::Px(MENU_BUTTON_DIAMETER_PX),
                Val::Px(MENU_BUTTON_DIAMETER_PX),
                false,
            ),
        ))
        .insert(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(MENU_MARGIN_PX),
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
                    let (text, role) = icon_for_mode(FlightMode::Dive).text();
                    face.spawn((ModeButtonIcon, text, role, TextColor(MENU_TEXT_TINT)));
                });
        });
    commands
        .spawn((
            ModeChooser,
            Name::new("mode-chooser"),
            GlobalZIndex(LAYER_UNIVERSE),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(MENU_MARGIN_PX),
                top: Val::Px(MENU_MARGIN_PX + MENU_BUTTON_DIAMETER_PX + MENU_BUTTON_GAP_PX),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(MENU_BUTTON_GAP_PX),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|chooser| {
            for mode in MODES {
                chooser
                    .spawn((
                        ModeRow(mode),
                        Name::new("mode-row"),
                        chrome::button(Val::Px(200.0), Val::Px(56.0), false),
                    ))
                    .with_children(|row| {
                        row.spawn(chrome::button_face(false)).with_children(|face| {
                            let (icon, role) = icon_for_mode(mode).text();
                            face.spawn((icon, role, TextColor(MENU_TEXT_TINT)));
                            face.spawn((
                                Text::new(label_for_mode(mode)),
                                TextRole::Body,
                                TextColor(MENU_TEXT_TINT),
                            ));
                        });
                    });
            }
        });
}

/// Visibility of one mode-switch node: each query contrasts its own
/// marker against the other two so Bevy sees them disjoint.
type ButtonVisibility<'w, 's> = Query<
    'w,
    's,
    &'static mut Visibility,
    (With<ModeButton>, Without<ModeChooser>, Without<ModeVeil>),
>;
/// Visibility of one mode-switch node: each query contrasts its own
/// marker against the other two so Bevy sees them disjoint.
type ChooserVisibility<'w, 's> = Query<
    'w,
    's,
    &'static mut Visibility,
    (With<ModeChooser>, Without<ModeButton>, Without<ModeVeil>),
>;
/// Visibility of one mode-switch node: each query contrasts its own
/// marker against the other two so Bevy sees them disjoint.
type VeilVisibility<'w, 's> = Query<
    'w,
    's,
    &'static mut Visibility,
    (With<ModeVeil>, Without<ModeButton>, Without<ModeChooser>),
>;

/// Refreshes the button and chooser: hidden when the Mode reminder switch
/// is off, the chooser shown only while open.
///
/// Writes only on a change; missing nodes are a no-op.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input; the button, icon, chooser, veil and rows sync in one place"
)]
pub(crate) fn sync_mode_switch(
    store: Res<crate::SettingsStore>,
    flight: Res<Flight>,
    open: Res<ChooserOpen>,
    mut button: ButtonVisibility<'_, '_>,
    mut icon: Query<&mut Text, With<ModeButtonIcon>>,
    mut chooser: ChooserVisibility<'_, '_>,
    mut veil: VeilVisibility<'_, '_>,
    mut rows: Query<(&ModeRow, &mut BorderColor)>,
) {
    let Ok(mut button_visibility) = button.single_mut() else {
        return;
    };
    let want_button = if store.0.mode_reminder {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *button_visibility != want_button {
        *button_visibility = want_button;
    }
    if let Ok(mut text) = icon.single_mut() {
        let want = icon_for_mode(flight.mode).glyph().to_string();
        if text.0 != want {
            text.0 = want;
        }
    }
    if let Ok(mut chooser_visibility) = chooser.single_mut() {
        let want_chooser = if store.0.mode_reminder && open.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *chooser_visibility != want_chooser {
            *chooser_visibility = want_chooser;
        }
    }
    if let Ok(mut veil_visibility) = veil.single_mut() {
        let want_veil = if store.0.mode_reminder && open.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *veil_visibility != want_veil {
            *veil_visibility = want_veil;
        }
    }
    for (row, mut border) in &mut rows {
        let want = BorderColor::all(if row.0 == flight.mode {
            MENU_SELECT_TINT
        } else {
            CARD_LINE_TINT
        });
        if *border != want {
            *border = want;
        }
    }
}

/// Opens and closes the chooser from the mode button.
pub(crate) fn mode_button_click(
    buttons: Query<&Interaction, (Changed<Interaction>, With<ModeButton>)>,
    mut open: ResMut<ChooserOpen>,
) {
    for interaction in &buttons {
        if *interaction == Interaction::Pressed {
            open.0 = !open.0;
        }
    }
}

/// Picks the row's mode with the shared enter rule, then closes.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input; picking a mode touches flight, nav, destination and the chooser at once"
)]
pub(crate) fn mode_row_click(
    buttons: Query<(&Interaction, &ModeRow), Changed<Interaction>>,
    mut flight: ResMut<Flight>,
    mut nav: ResMut<Navigation>,
    universe: Res<Universe>,
    mut autopilot: ResMut<Autopilot>,
    mut open: ResMut<ChooserOpen>,
    mut player: Option<ResMut<crate::player_state::PlayerState>>,
) {
    for (interaction, row) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        apply_pick(
            row.0,
            &mut flight,
            &mut nav,
            &universe,
            &mut autopilot,
            &mut open,
            player.as_deref_mut(),
        );
    }
}

/// Applies one picked mode with the shared enter rule, then closes.
/// Normal far from the body arms the key-1 takeover instead of placing a
/// body under the camera (#955: picks do exactly what the keys do).
fn apply_pick(
    mode: FlightMode,
    flight: &mut Flight,
    nav: &mut Navigation,
    universe: &Universe,
    autopilot: &mut Autopilot,
    open: &mut ChooserOpen,
    player: Option<&mut crate::player_state::PlayerState>,
) {
    use universe_core::destination::Destination;
    let root = universe.root.0;
    let mut destination = match nav.destination {
        None => Destination::seeded(root, &universe.path.indices()),
        Some(held) => held.reconciled(root, &universe.path),
    };
    match mode {
        FlightMode::Normal => {
            let far = flight.mode != FlightMode::Normal
                && player
                    .as_ref()
                    .is_some_and(|p| p.root == root && !p.spot.is_near(&universe.path));
            match player {
                Some(body) if far => {
                    crate::player::arm_takeover(body, nav, autopilot, flight);
                    open.0 = false;
                    return;
                }
                _ => crate::input::enter_normal(flight, autopilot),
            }
        }
        FlightMode::Free => crate::input::enter_free(flight, nav, autopilot),
        FlightMode::Dive => {
            crate::input::enter_dive(flight, nav, universe, &mut destination);
        }
    }
    nav.destination = Some(destination);
    open.0 = false;
}

/// Closes the chooser on a click on the veil (anywhere outside it).
pub(crate) fn mode_veil_click(
    buttons: Query<&Interaction, (Changed<Interaction>, With<ModeVeil>)>,
    mut open: ResMut<ChooserOpen>,
) {
    for interaction in &buttons {
        if *interaction == Interaction::Pressed {
            open.0 = false;
        }
    }
}

/// Picks rows by cursor math as a second path beside per-row dispatch
/// (#955 correction round 2: row presses never arrive, while veil and
/// button presses do). The button owns its region; a row hit picks that
/// mode and closes; anything else closes. Uses the same hit test and
/// logical-pixel convention as touch controls.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input; one press resolves button, rows and outside at once"
)]
pub(crate) fn mode_chooser_click(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    touch: Res<crate::touch::TouchState>,
    button: Query<(&ComputedNode, &UiGlobalTransform), With<ModeButton>>,
    rows: Query<(&ModeRow, &ComputedNode, &UiGlobalTransform)>,
    mut flight: ResMut<Flight>,
    mut nav: ResMut<Navigation>,
    universe: Res<Universe>,
    mut autopilot: ResMut<Autopilot>,
    mut open: ResMut<ChooserOpen>,
    mut player: Option<ResMut<crate::player_state::PlayerState>>,
    mut claim: ResMut<ModeSwitchClaim>,
) {
    claim.0 = false;
    let window = windows.iter().next();
    let scale = window.map_or(1.0, Window::scale_factor);
    let mut at: Option<[f32; 2]> = None;
    if buttons.just_pressed(MouseButton::Left) {
        at = window
            .and_then(Window::cursor_position)
            .map(|cursor| [cursor.x, cursor.y]);
    } else if let Some(tap) = touch.tap {
        at = Some(tap);
    }
    let Some(at) = at else {
        return;
    };
    // The button's own press toggles through its `Interaction`; it is
    // still the mode switch's click, never the game's.
    if crate::touch::contains(&button, at, scale) {
        claim.0 = true;
        return;
    }
    if !open.0 {
        return;
    }
    // Open: the veil covers the window, so every press is ours.
    claim.0 = true;
    let point = Vec2::new(at[0], at[1]) * scale;
    for (row, node, transform) in &rows {
        if node.contains_point(*transform, point) {
            apply_pick(
                row.0,
                &mut flight,
                &mut nav,
                &universe,
                &mut autopilot,
                &mut open,
                player.as_deref_mut(),
            );
            return;
        }
    }
    open.0 = false;
}

/// Opens the chooser from a touch tap on the mode button (bridges the tap
/// flag that `touch::read_touches` raises on the button node).
pub(crate) fn touch_opens_chooser(
    touch: Res<crate::touch::TouchState>,
    mut open: ResMut<ChooserOpen>,
) {
    if touch.reminder_tapped {
        open.0 = true;
    }
}

#[cfg(test)]
#[path = "mode_switch_click_tests.rs"]
mod click_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_tests::headless_app;

    #[test]
    fn mode_button_and_chooser_spawn_with_the_off_state() {
        let mut app = headless_app();
        app.update();
        let button = app
            .world_mut()
            .query_filtered::<&Visibility, With<ModeButton>>()
            .single(app.world())
            .copied()
            .expect("one mode button");
        assert_eq!(button, Visibility::Inherited, "button shows by default");
        let rows: Vec<FlightMode> = app
            .world_mut()
            .query::<&ModeRow>()
            .iter(app.world())
            .map(|row| row.0)
            .collect();
        assert_eq!(rows, MODES, "one row per mode in order");
        let chooser = app
            .world_mut()
            .query_filtered::<&Visibility, With<ModeChooser>>()
            .single(app.world())
            .copied()
            .expect("one chooser");
        assert_eq!(chooser, Visibility::Hidden, "chooser starts closed");
        app.world_mut()
            .resource_mut::<crate::SettingsStore>()
            .0
            .mode_reminder = false;
        app.update();
        let button = app
            .world_mut()
            .query_filtered::<&Visibility, With<ModeButton>>()
            .single(app.world())
            .copied()
            .expect("one mode button");
        assert_eq!(button, Visibility::Hidden, "the switch hides the button");
        let veil_interaction = app
            .world_mut()
            .query_filtered::<&Interaction, With<ModeVeil>>()
            .single(app.world())
            .is_ok();
        assert!(
            veil_interaction,
            "the veil carries Interaction so outside clicks reach it"
        );
    }

    /// Presses the entity carrying `marker` for one update.
    fn press_marker<T: Component>(app: &mut App) {
        let entity = app
            .world_mut()
            .query_filtered::<Entity, With<T>>()
            .single(app.world())
            .expect("one marked node");
        app.world_mut()
            .entity_mut(entity)
            .insert(Interaction::Pressed);
        app.update();
        app.world_mut().entity_mut(entity).insert(Interaction::None);
        app.update();
    }

    #[test]
    fn mode_chooser_lists_three_modes_with_highlight() {
        use crate::input::FlightMode;
        let mut app = headless_app();
        app.update();
        assert!(
            !app.world().resource::<ChooserOpen>().0,
            "chooser starts closed"
        );
        press_marker::<ModeButton>(&mut app);
        assert!(
            app.world().resource::<ChooserOpen>().0,
            "the button opens the chooser"
        );
        let chooser = app
            .world_mut()
            .query_filtered::<&Visibility, With<ModeChooser>>()
            .single(app.world())
            .copied()
            .expect("one chooser");
        assert_eq!(chooser, Visibility::Inherited, "chooser shows");
        let rows: Vec<FlightMode> = app
            .world_mut()
            .query::<&ModeRow>()
            .iter(app.world())
            .map(|row| row.0)
            .collect();
        assert_eq!(rows, MODES, "three modes with the current highlighted");
    }

    #[test]
    fn choosing_a_mode_switches_and_closes() {
        use crate::input::{Autopilot, Flight, FlightMode};
        use crate::player_state::PlayerState;
        let mut app = headless_app();
        app.update();
        let pick = |app: &mut App, mode: FlightMode| {
            press_marker::<ModeButton>(app);
            let row = app
                .world_mut()
                .query::<(Entity, &ModeRow)>()
                .iter(app.world())
                .find(|(_, row)| row.0 == mode)
                .map(|(entity, _)| entity)
                .expect("row exists");
            app.world_mut().entity_mut(row).insert(Interaction::Pressed);
            app.update();
        };
        // Far from the body, Normal arms the key-1 takeover: the autopilot
        // dives to the player first instead of placing a body in place.
        pick(&mut app, FlightMode::Normal);
        assert_eq!(
            app.world().resource::<Flight>().mode,
            FlightMode::Dive,
            "far pick dives to the player first, like key 1"
        );
        assert_eq!(
            *app.world().resource::<Autopilot>(),
            Autopilot::Flying,
            "takeover autopilot runs"
        );
        assert!(
            app.world().resource::<PlayerState>().takeover,
            "handover armed"
        );
        assert!(
            !app.world().resource::<ChooserOpen>().0,
            "picking closes the chooser"
        );
        // Near the body, Normal enters at once, like key 1.
        crate::player::player_here(&mut app);
        pick(&mut app, FlightMode::Normal);
        assert_eq!(
            app.world().resource::<Flight>().mode,
            FlightMode::Normal,
            "near pick switches the mode"
        );
    }

    #[test]
    fn outside_click_closes_without_switching() {
        use crate::input::{Flight, FlightMode};
        let mut app = headless_app();
        app.update();
        press_marker::<ModeButton>(&mut app);
        assert!(app.world().resource::<ChooserOpen>().0, "chooser open");
        press_marker::<ModeVeil>(&mut app);
        assert!(
            !app.world().resource::<ChooserOpen>().0,
            "veil click closes"
        );
        assert_eq!(
            app.world().resource::<Flight>().mode,
            FlightMode::Dive,
            "no switch on outside click"
        );
    }

    #[test]
    fn touch_tap_opens_the_chooser() {
        let mut app = headless_app();
        app.update();
        app.world_mut()
            .resource_mut::<crate::touch::TouchState>()
            .reminder_tapped = true;
        app.update();
        assert!(
            app.world().resource::<ChooserOpen>().0,
            "touch tap opens like a click"
        );
    }

    /// Presses `key` for one update.
    fn tap(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release(key);
        input.clear();
    }

    #[test]
    fn mode_toggle_button_shows_current_icon_only() {
        use crate::fonts::Icon;
        let mut app = headless_app();
        app.update();
        let shown = |app: &mut App| {
            let mut texts: Vec<String> = app
                .world_mut()
                .query_filtered::<&Text, With<ModeButtonIcon>>()
                .iter(app.world())
                .map(|text| text.0.clone())
                .collect();
            assert_eq!(texts.len(), 1, "one icon and no label");
            texts.pop().expect("icon text")
        };
        assert_eq!(
            shown(&mut app),
            Icon::ModeDive.glyph().to_string(),
            "starts on the dive icon"
        );
    }

    #[test]
    fn keys_and_f_switch_modes_and_icon_follows() {
        use crate::fonts::Icon;
        use crate::input::{Flight, FlightMode};
        use crate::tokens::{CARD_LINE_TINT, MENU_SELECT_TINT};
        let mut app = headless_app();
        app.update();
        crate::player::player_here(&mut app);
        let shown = |app: &mut App| {
            app.world_mut()
                .query_filtered::<&Text, With<ModeButtonIcon>>()
                .single(app.world())
                .map(|text| text.0.clone())
                .expect("one icon")
        };
        let highlight = |app: &mut App, mode: FlightMode| {
            app.world_mut()
                .query::<(&ModeRow, &BorderColor)>()
                .iter(app.world())
                .find(|(row, _)| row.0 == mode)
                .map(|(_, border)| *border)
                .expect("row exists")
        };
        tap(&mut app, KeyCode::Digit1);
        assert_eq!(
            app.world().resource::<Flight>().mode,
            FlightMode::Normal,
            "key 1 enters normal mode"
        );
        assert_eq!(shown(&mut app), Icon::ModeNormal.glyph().to_string());
        assert_eq!(
            highlight(&mut app, FlightMode::Normal),
            BorderColor::all(MENU_SELECT_TINT),
            "picked row highlighted"
        );
        assert_eq!(
            highlight(&mut app, FlightMode::Dive),
            BorderColor::all(CARD_LINE_TINT),
            "old row back to rest"
        );
        tap(&mut app, KeyCode::KeyF);
        assert_eq!(app.world().resource::<Flight>().mode, FlightMode::Free);
        assert_eq!(shown(&mut app), Icon::ModeFree.glyph().to_string());
        tap(&mut app, KeyCode::Digit3);
        assert_eq!(app.world().resource::<Flight>().mode, FlightMode::Dive);
        assert_eq!(shown(&mut app), Icon::ModeDive.glyph().to_string());
    }

    #[test]
    fn apply_pick_switches_and_closes() {
        use crate::input::{Autopilot, Flight, FlightMode, Navigation};
        use universe_core::nav::{DEMO_SEED, Universe as CoreUniverse};
        let mut flight = Flight::default();
        let mut nav = Navigation::default();
        let universe = Universe(CoreUniverse::new(DEMO_SEED));
        let mut autopilot = Autopilot::default();
        let mut open = ChooserOpen(true);
        apply_pick(
            FlightMode::Free,
            &mut flight,
            &mut nav,
            &universe,
            &mut autopilot,
            &mut open,
            None,
        );
        assert_eq!(flight.mode, FlightMode::Free, "pick switches");
        assert!(!open.0, "pick closes");
        assert!(nav.destination.is_some(), "destination kept");
    }
}
