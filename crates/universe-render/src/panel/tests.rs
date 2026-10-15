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
fn every_guides_page_has_a_tappable_list_row() {
    use universe_core::guides::GuidePage;
    let mut app = headless_app();
    app.update();
    app.update();
    let mut ids = Vec::new();
    let mut items = app.world_mut().query::<&PanelItem>();
    for item in items.iter(app.world()) {
        if item.screen == Screen::Guides {
            ids.push(item.id.clone());
        }
    }
    for page in GuidePage::ALL {
        assert!(ids.contains(&page.id().to_string()), "{page:?} has no row");
    }
    assert_eq!(ids.len(), 11, "two categories plus nine pages");
}

/// Opens the Guides screen in the headless app.
fn open_guides(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<Screen>>()
        .set(Screen::Guides);
    app.update();
    app.update();
}

/// Sends one wheel notch down over the panel.
fn wheel_down(app: &mut App) {
    use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
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
}

/// Marks the Guides list hovered (headless has no cursor).
fn hover_guides_list(app: &mut App, hovered: bool) {
    let mut lists = app
        .world_mut()
        .query::<(&PanelList, &mut RelativeCursorPosition)>();
    for (list, mut hover) in lists.iter_mut(app.world_mut()) {
        if list.0 == Screen::Guides {
            hover.cursor_over = hovered;
        }
    }
}

/// Gives the Guides list a live layout (headless measures zero).
fn layout_guides_list(app: &mut App, viewport: f32, stacked: f32) {
    let entity = app
        .world_mut()
        .query::<(Entity, &PanelList)>()
        .iter(app.world())
        .find(|(_, list)| list.0 == Screen::Guides)
        .map(|(entity, _)| entity)
        .expect("guides list exists");
    app.world_mut().entity_mut(entity).insert(ComputedNode {
        size: Vec2::new(260.0, viewport),
        content_size: Vec2::new(260.0, stacked),
        ..ComputedNode::DEFAULT
    });
}

#[test]
fn wheel_over_the_list_scrolls_rows_not_content() {
    let mut app = headless_app();
    open_guides(&mut app);
    hover_guides_list(&mut app, true);
    wheel_down(&mut app);
    assert_eq!(
        app.world().resource::<PanelListScrolled>().0,
        48.0,
        "two lines down move the list"
    );
    assert_eq!(
        app.world().resource::<PanelScrolled>().0,
        0.0,
        "content stays while the list is hovered"
    );
    hover_guides_list(&mut app, false);
    wheel_down(&mut app);
    assert_eq!(
        app.world().resource::<PanelScrolled>().0,
        48.0,
        "without hover the wheel keeps scrolling content"
    );
}

#[test]
fn selecting_a_row_reveals_it_in_the_list() {
    use bevy::ecs::system::RunSystemOnce;
    let mut app = headless_app();
    open_guides(&mut app);
    // Live Guides layout: eleven 56 px rows in a 481 px list.
    layout_guides_list(&mut app, 481.0, 616.0);
    app.world_mut()
        .resource_mut::<PanelState>()
        .select("planets");
    let _ = app.world_mut().run_system_once(sync_panel_list_scroll);
    assert_eq!(
        app.world().resource::<PanelListScrolled>().0,
        135.0,
        "the last row scrolls into view"
    );
    app.world_mut().resource_mut::<PanelState>().select("about");
    let _ = app.world_mut().run_system_once(sync_panel_list_scroll);
    assert_eq!(
        app.world().resource::<PanelListScrolled>().0,
        0.0,
        "the first row scrolls back to the top"
    );
}

#[test]
fn narrow_list_scrolls_like_the_wide_list() {
    let mut app = headless_app();
    app.world_mut().resource_mut::<PanelNarrow>().0 = true;
    open_guides(&mut app);
    hover_guides_list(&mut app, true);
    wheel_down(&mut app);
    assert_eq!(
        app.world().resource::<PanelListScrolled>().0,
        48.0,
        "narrow lists scroll the same way"
    );
}

#[test]
fn guides_planets_shot_reveals_its_row() {
    use bevy::ecs::system::RunSystemOnce;
    // The guides-planets shot opens Guides with "planets" selected:
    // the list must end scrolled to the last row.
    let mut app = headless_app();
    open_guides(&mut app);
    layout_guides_list(&mut app, 481.0, 616.0);
    app.world_mut()
        .resource_mut::<PanelState>()
        .select("planets");
    let _ = app.world_mut().run_system_once(sync_panel_list_scroll);
    assert_eq!(app.world().resource::<PanelState>().selected, "planets");
    assert_eq!(
        app.world().resource::<PanelListScrolled>().0,
        135.0,
        "the shot reveals the Planets row"
    );
}

#[test]
fn list_rows_keep_the_shared_button_height() {
    let mut app = headless_app();
    app.update();
    app.update();
    let mut rows = 0;
    let mut items = app.world_mut().query::<(&PanelItem, &Node)>();
    for (_, node) in items.iter(app.world()) {
        rows += 1;
        assert_eq!(
            node.height,
            Val::Px(SCREEN_BUTTON_HEIGHT_PX),
            "rows keep the shared height"
        );
    }
    assert!(rows > 0, "lists spawn rows");
}

#[test]
fn list_gutter_opens_only_on_overflow() {
    use bevy::ecs::system::RunSystemOnce;
    let mut app = headless_app();
    open_guides(&mut app);
    layout_guides_list(&mut app, 481.0, 616.0);
    let _ = app.world_mut().run_system_once(sync_panel_list_scroll);
    let gutter = app
        .world_mut()
        .query::<(&PanelList, &Node)>()
        .iter(app.world())
        .find(|(list, _)| list.0 == Screen::Guides)
        .map(|(_, node)| node.scrollbar_width)
        .expect("guides list exists");
    assert_eq!(
        gutter, SCROLLBAR_WIDTH_PX,
        "the overflowing list reserves the thumb gutter"
    );
    layout_guides_list(&mut app, 481.0, 400.0);
    let _ = app.world_mut().run_system_once(sync_panel_list_scroll);
    let gutter = app
        .world_mut()
        .query::<(&PanelList, &Node)>()
        .iter(app.world())
        .find(|(list, _)| list.0 == Screen::Guides)
        .map(|(_, node)| node.scrollbar_width)
        .expect("guides list exists");
    assert_eq!(gutter, 0.0, "a fitting list keeps the gutter shut");
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

#[test]
fn scroll_clamp_holds_the_content_range() {
    assert_eq!(scroll_clamp(24.0, 400.0, 200.0), 24.0, "inside stays");
    assert_eq!(scroll_clamp(-10.0, 400.0, 200.0), 0.0, "top clamps");
    assert_eq!(
        scroll_clamp(500.0, 400.0, 200.0),
        200.0,
        "bottom clamps to the overflow"
    );
    assert_eq!(
        scroll_clamp(50.0, 100.0, 200.0),
        0.0,
        "content that fits stays at top"
    );
}

/// Opens `screen` headlessly (state transitions apply a frame later).
fn open_screen(app: &mut App, screen: Screen) {
    app.world_mut()
        .resource_mut::<NextState<Screen>>()
        .set(screen);
    app.update();
    app.update();
}

#[test]
fn new_selection_resets_the_scroll_to_top() {
    let mut app = headless_app();
    app.update();
    open_screen(&mut app, Screen::Settings);
    press_item(&mut app, "scene");
    // Simulate a scrolled-down page, then switch pages.
    app.world_mut().resource_mut::<PanelScrolled>().0 = 96.0;
    press_item(&mut app, "window");
    assert_eq!(
        app.world().resource::<PanelState>().selected,
        "window",
        "the new page is selected"
    );
    assert_eq!(
        app.world().resource::<PanelScrolled>().0,
        0.0,
        "switching pages resets to the top"
    );
    // The open content carries a scroll position at the top (headless
    // layout measures nothing, so the range clamps to zero).
    let at_top = app
        .world_mut()
        .query::<(&PanelContent, &ScrollPosition)>()
        .iter(app.world())
        .all(|(_, position)| position.y == 0.0);
    assert!(at_top, "every content sits at the top");
}
