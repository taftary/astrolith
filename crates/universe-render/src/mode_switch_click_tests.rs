//! Real-press chooser tests (#955 correction round 5): the press goes
//! through `ButtonInput` and the window cursor like a real click, so the
//! game's own click handling sees it too. Split from `mode_switch` under
//! `E-FILE-SIZE`.

use super::*;
use crate::input::{Autopilot, Flight, FlightMode};
use crate::player_state::PlayerState;
use crate::window_tests::headless_app;
use bevy::math::Affine2;

/// Row centre in logical pixels (rows are 200 x 56, scale factor 1).
fn row_centre(index: usize) -> Vec2 {
    let top = MENU_MARGIN_PX + MENU_BUTTON_DIAMETER_PX + MENU_BUTTON_GAP_PX;
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: row index 0..3 is exact in f32"
    )]
    let y = top + (56.0 + MENU_BUTTON_GAP_PX) * index as f32 + 28.0;
    Vec2::new(MENU_MARGIN_PX + 100.0, y)
}

/// A universe app with a laid-out chooser (no UI layout runs headless, so
/// the rows get their box by hand) and a window whose cursor sits on the
/// `mode` row.
fn app_with_cursor_on(mode: FlightMode, simulate: bool) -> App {
    let mut app = headless_app();
    app.world_mut()
        .resource_mut::<crate::SettingsStore>()
        .0
        .simulate_touch = simulate;
    app.update();
    let rows: Vec<(Entity, FlightMode)> = app
        .world_mut()
        .query::<(Entity, &ModeRow)>()
        .iter(app.world())
        .map(|(entity, row)| (entity, row.0))
        .collect();
    let mut cursor = Vec2::ZERO;
    for (index, (entity, row)) in rows.into_iter().enumerate() {
        let centre = row_centre(index);
        if row == mode {
            cursor = centre;
        }
        app.world_mut().entity_mut(entity).insert((
            ComputedNode {
                size: Vec2::new(200.0, 56.0),
                ..ComputedNode::DEFAULT
            },
            UiGlobalTransform::from(Affine2::from_translation(centre)),
        ));
    }
    let mut window = Window::default();
    window.resolution.set(1280.0, 720.0);
    window.set_cursor_position(Some(cursor));
    app.world_mut().spawn(window);
    app.world_mut().resource_mut::<ChooserOpen>().0 = true;
    app.update();
    app
}

/// One left click: press for a frame, then release.
fn click(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    buttons.release(MouseButton::Left);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
    app.update();
}

#[test]
fn a_far_normal_pick_keeps_the_takeover_through_the_click() {
    // The owner's failure: far from the body, a real click on Normal armed
    // the key-1 takeover, then the same click reached the game's click
    // handling, which set the autopilot idle. Both touch settings.
    for simulate in [false, true] {
        let mut app = app_with_cursor_on(FlightMode::Normal, simulate);
        click(&mut app);
        assert!(
            !app.world().resource::<ChooserOpen>().0,
            "simulate {simulate}: the pick closes the chooser"
        );
        assert_eq!(
            app.world().resource::<Flight>().mode,
            FlightMode::Dive,
            "simulate {simulate}: far Normal dives to the player first, like key 1"
        );
        assert_eq!(
            *app.world().resource::<Autopilot>(),
            Autopilot::Flying,
            "simulate {simulate}: the takeover keeps flying after the click"
        );
        let player = app.world().resource::<PlayerState>();
        assert!(
            player.takeover && player.selected,
            "simulate {simulate}: the handover stays armed"
        );
    }
}

#[test]
fn a_real_click_picks_free_and_dive() {
    // Free and Dive through the same real press, both touch settings.
    for simulate in [false, true] {
        let mut app = app_with_cursor_on(FlightMode::Free, simulate);
        click(&mut app);
        assert_eq!(
            app.world().resource::<Flight>().mode,
            FlightMode::Free,
            "simulate {simulate}: Free picked"
        );
        let mut app = app_with_cursor_on(FlightMode::Dive, simulate);
        app.world_mut().resource_mut::<Flight>().mode = FlightMode::Free;
        click(&mut app);
        assert_eq!(
            app.world().resource::<Flight>().mode,
            FlightMode::Dive,
            "simulate {simulate}: Dive picked"
        );
    }
}

#[test]
fn an_outside_click_does_not_stop_the_autopilot() {
    // The veil blocks game clicks while open: closing the chooser by a
    // click elsewhere leaves a running autopilot alone.
    for simulate in [false, true] {
        let mut app = app_with_cursor_on(FlightMode::Free, simulate);
        let mut window = app
            .world_mut()
            .query::<&mut Window>()
            .single_mut(app.world_mut())
            .expect("one window");
        window.set_cursor_position(Some(Vec2::new(700.0, 400.0)));
        *app.world_mut().resource_mut::<Autopilot>() = Autopilot::Flying;
        click(&mut app);
        assert!(
            !app.world().resource::<ChooserOpen>().0,
            "simulate {simulate}: outside click closes"
        );
        assert_eq!(
            *app.world().resource::<Autopilot>(),
            Autopilot::Flying,
            "simulate {simulate}: the closing click is not a game click"
        );
    }
}
