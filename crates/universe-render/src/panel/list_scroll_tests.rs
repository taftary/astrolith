//! The category list scrolls on its own (#1128 R2): in a short window the
//! last Settings entry, Developer, fell below the card and could not be
//! reached. The wheel over the list moves the list, the wheel elsewhere
//! moves the content, and the list carries its own scrollbar thumb.

use super::scrollbar::{ListThumb, PanelThumb, sync_panel_thumbs};
use super::{PanelList, PanelListScrolled, PanelScrolled, PanelState};
use crate::lib_tests::headless_app;
use crate::menu::Screen;
use bevy::ecs::system::RunSystemOnce;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

/// Boots the app and opens Settings (transitions apply a frame later).
fn settings_app() -> App {
    let mut app = headless_app();
    app.update();
    app.world_mut()
        .resource_mut::<NextState<Screen>>()
        .set(Screen::Settings);
    app.update();
    app.update();
    app
}

/// The Settings list entity.
fn settings_list(app: &mut App) -> Entity {
    app.world_mut()
        .query::<(Entity, &PanelList)>()
        .iter(app.world())
        .find(|(_, list)| list.0 == Screen::Settings)
        .map(|(entity, _)| entity)
        .expect("the Settings list exists")
}

/// Puts the cursor over the Settings list, or off it.
fn hover_list(app: &mut App, over: bool) {
    let list = settings_list(app);
    app.world_mut()
        .entity_mut(list)
        .insert(RelativeCursorPosition {
            cursor_over: over,
            normalized: None,
        });
}

/// One wheel event of `lines` (negative scrolls down).
fn wheel(app: &mut App, lines: f32) {
    app.world_mut()
        .resource_mut::<Messages<MouseWheel>>()
        .write(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: lines,
            window: Entity::PLACEHOLDER,
            phase: bevy::input::touch::TouchPhase::Moved,
        });
}

#[test]
fn wheel_over_the_list_scrolls_the_list_not_the_content() {
    let mut app = settings_app();
    // One registered system keeps its message reader between runs.
    let scroll = app.world_mut().register_system(super::panel_wheel_scroll);
    hover_list(&mut app, true);
    wheel(&mut app, -2.0);
    // Run the wheel system alone: the headless layout measures nothing, so
    // the sync systems would clamp every offset back to zero.
    let _ = app.world_mut().run_system(scroll);
    assert_eq!(app.world().resource::<PanelListScrolled>().0, 48.0);
    assert_eq!(
        app.world().resource::<PanelScrolled>().0,
        0.0,
        "the content stays put while the list scrolls"
    );
    hover_list(&mut app, false);
    wheel(&mut app, -1.0);
    let _ = app.world_mut().run_system(scroll);
    assert_eq!(
        app.world().resource::<PanelScrolled>().0,
        24.0,
        "off the list, the wheel scrolls the content as before"
    );
    assert_eq!(app.world().resource::<PanelListScrolled>().0, 48.0);
}

#[test]
fn list_offset_clamps_to_the_live_list_in_logical_pixels() {
    let mut app = settings_app();
    let list = settings_list(&mut app);
    // A list 300 physical px tall holding 600, at scale 2 (inverse 0.5):
    // 150 logical px of travel.
    app.world_mut().entity_mut(list).insert(ComputedNode {
        size: Vec2::new(200.0, 300.0),
        content_size: Vec2::new(200.0, 600.0),
        inverse_scale_factor: 0.5,
        ..ComputedNode::DEFAULT
    });
    app.world_mut().resource_mut::<PanelListScrolled>().0 = 400.0;
    // A full frame: a one-off run would see the screen as just opened and
    // start the list at its top.
    app.update();
    assert_eq!(app.world().resource::<PanelListScrolled>().0, 150.0);
    let position = app
        .world()
        .get::<ScrollPosition>(list)
        .expect("the list scrolls")
        .y;
    assert_eq!(position, 150.0, "the bottom entry comes into view");
}

#[test]
fn choosing_a_page_keeps_the_list_where_it_is() {
    let mut app = settings_app();
    let list = settings_list(&mut app);
    let tall = ComputedNode {
        size: Vec2::new(200.0, 300.0),
        content_size: Vec2::new(200.0, 600.0),
        ..ComputedNode::DEFAULT
    };
    app.world_mut().entity_mut(list).insert(tall);
    app.world_mut().resource_mut::<PanelListScrolled>().0 = 120.0;
    app.world_mut()
        .resource_mut::<PanelState>()
        .select("developer");
    app.update();
    // The layout run is headless, so re-insert the measured list each frame.
    app.world_mut().entity_mut(list).insert(tall);
    app.update();
    assert_eq!(
        app.world().resource::<PanelListScrolled>().0,
        120.0,
        "the chosen entry stays in view"
    );
    // Leaving and reopening Settings starts the list at its top.
    app.world_mut()
        .resource_mut::<NextState<Screen>>()
        .set(Screen::Menu);
    app.update();
    app.update();
    assert_eq!(app.world().resource::<PanelListScrolled>().0, 0.0);
}

#[test]
fn every_list_gets_its_own_thumb_placed_on_live_layout() {
    let mut app = settings_app();
    let mut screens: Vec<Screen> = app
        .world_mut()
        .query_filtered::<&PanelThumb, With<ListThumb>>()
        .iter(app.world())
        .map(|thumb| thumb.0)
        .collect();
    screens.sort_by_key(|screen| *screen as u8);
    assert_eq!(screens, vec![Screen::Guides, Screen::Settings]);
    let content_thumbs = app
        .world_mut()
        .query_filtered::<&PanelThumb, Without<ListThumb>>()
        .iter(app.world())
        .count();
    assert_eq!(content_thumbs, 2, "content thumbs are unchanged");
    // An overflowing list at scale 2: the thumb is half the gutter, in
    // logical pixels.
    let list = settings_list(&mut app);
    app.world_mut().entity_mut(list).insert(ComputedNode {
        size: Vec2::new(200.0, 400.0),
        content_size: Vec2::new(200.0, 800.0),
        inverse_scale_factor: 0.5,
        ..ComputedNode::DEFAULT
    });
    let _ = app.world_mut().run_system_once(sync_panel_thumbs);
    let shown: Vec<(Val, Val, Display)> = app
        .world_mut()
        .query_filtered::<(&Node, &ChildOf), With<ListThumb>>()
        .iter(app.world())
        .filter(|(_, parent)| parent.parent() == list)
        .map(|(node, _)| (node.top, node.height, node.display))
        .collect();
    assert_eq!(
        shown,
        vec![(Val::Px(0.0), Val::Px(100.0), Display::Flex)],
        "the list thumb shows at the top, half of 200 logical px"
    );
    let content_shown = app
        .world_mut()
        .query_filtered::<&Node, (With<PanelThumb>, Without<ListThumb>)>()
        .iter(app.world())
        .all(|node| node.display == Display::None);
    assert!(content_shown, "content that fits keeps its thumb hidden");
}

#[test]
fn closing_the_panel_leaves_no_bar_or_thumb_behind() {
    use super::PanelBar;
    let mut app = settings_app();
    let list = settings_list(&mut app);
    app.world_mut().entity_mut(list).insert(ComputedNode {
        size: Vec2::new(200.0, 300.0),
        content_size: Vec2::new(200.0, 600.0),
        ..ComputedNode::DEFAULT
    });
    app.world_mut()
        .resource_mut::<PanelState>()
        .select("developer");
    app.update();
    // An explicit `Visible` outlives the hidden panel: after closing
    // Settings, the selection bar and the list thumb stayed on the universe.
    let explicit = |app: &mut App| {
        let bars = app
            .world_mut()
            .query::<(&PanelBar, &Visibility)>()
            .iter(app.world())
            .filter(|(_, visibility)| **visibility == Visibility::Visible)
            .count();
        let thumbs = app
            .world_mut()
            .query::<(&PanelThumb, &Visibility)>()
            .iter(app.world())
            .filter(|(_, visibility)| **visibility == Visibility::Visible)
            .count();
        bars + thumbs
    };
    assert_eq!(
        explicit(&mut app),
        0,
        "shown parts inherit the panel's state"
    );
    let shown_bars = app
        .world_mut()
        .query::<(&PanelBar, &Visibility)>()
        .iter(app.world())
        .filter(|(bar, visibility)| {
            bar.item == "developer" && **visibility == Visibility::Inherited
        })
        .count();
    assert_eq!(shown_bars, 1, "the chosen entry's bar shows");
}
