//! Guides content: About and Help pages with the approved text (#473, T6).
//!
//! Everything here is `pub(crate)`: only this crate uses it. [`GuidesUiPlugin`]
//! fills the Guides content area with one section per page; bodies come from
//! `universe_core::guides` (the ladder is built from the game labels, so it
//! cannot drift). Visibility follows the panel selection (`about` shows the
//! first About page, `help` the first Help page).

use crate::fonts::TextRole;
use crate::menu::Screen;
use crate::panel::{PanelContent, PanelState};
use crate::tokens::{MENU_DIM_TINT, MENU_MARGIN_PX, MENU_PANEL_TINT, MENU_TEXT_TINT};
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use bevy::ui::{Node, Val, widget::Text};
use universe_core::guides::{GuidePage, scale_ladder, static_text};

/// Marks one guides content section (shown for its leaf ids).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuideSection {
    /// Leaf ids showing this section.
    pub leaves: &'static [&'static str],
}

/// Guides content.
pub(crate) struct GuidesUiPlugin;

impl Plugin for GuidesUiPlugin {
    /// Registers the content spawn and the section sync.
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_guides_content, sync_guide_sections));
    }
}

/// Spawns every guides section into the Guides content area.
///
/// One-shot: runs until the panel shell exists (Startup order across plugins
/// is unspecified), then parks.
pub(crate) fn spawn_guides_content(
    mut commands: Commands,
    content: Query<Entity, With<PanelContent>>,
    panel: Query<&PanelContent>,
    mut spawned: Local<bool>,
) {
    if *spawned {
        return;
    }
    let Some(content) = content.iter().find(|entity| {
        panel
            .get(*entity)
            .is_ok_and(|content| content.0 == Screen::Guides)
    }) else {
        return;
    };
    *spawned = true;
    commands.entity(content).with_children(|sections| {
        for page in GuidePage::ALL {
            spawn_guide_section(sections, page);
        }
    });
}

/// Spawns one page section with its title and body.
fn spawn_guide_section(sections: &mut ChildSpawnerCommands, page: GuidePage) {
    let body: String = match static_text(page) {
        Some(text) => text.to_string(),
        None => scale_ladder(),
    };
    sections
        .spawn((
            GuideSection {
                leaves: leaves_of(page),
            },
            Name::new(format!("guides-{}", page.id())),
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(MENU_MARGIN_PX)),
                ..default()
            },
            BackgroundColor(MENU_PANEL_TINT),
            Visibility::Hidden,
        ))
        .with_children(|section| {
            section.spawn((
                Text::new(page.title()),
                TextRole::Body,
                TextColor(MENU_TEXT_TINT),
            ));
            // One text entity per line: long bodies wrap poorly in a single
            // node, and per-line entities scroll evenly.
            for line in body.lines() {
                section.spawn((Text::new(line), TextRole::Body, TextColor(MENU_DIM_TINT)));
            }
        });
}

/// Leaf ids showing one page's section.
fn leaves_of(page: GuidePage) -> &'static [&'static str] {
    match page {
        GuidePage::What => &["what"],
        GuidePage::Ladder => &["ladder"],
        GuidePage::GettingStarted => &["getting-started"],
        GuidePage::Navigation => &["navigation"],
        GuidePage::FreeFlight => &["free-flight"],
        GuidePage::SavedViews => &["saved-views"],
    }
}

/// Page id shown for a panel selection (`about`/`help` open their first page).
#[must_use]
pub(crate) fn guide_leaf_for(selected: &str) -> &str {
    match selected {
        "about" => GuidePage::What.id(),
        "help" => GuidePage::GettingStarted.id(),
        other => other,
    }
}

/// Shows the section matching the panel selection.
pub(crate) fn sync_guide_sections(
    selection: Res<PanelState>,
    screen: Res<State<Screen>>,
    mut sections: Query<(&GuideSection, &mut Visibility), Without<PanelContent>>,
) {
    if !selection.is_changed() && !screen.is_changed() {
        return;
    }
    let open = *screen.get() == Screen::Guides;
    let want_id = guide_leaf_for(selection.selected.as_str());
    for (section, mut visibility) in &mut sections {
        let want = if open && section.leaves.contains(&want_id) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::AssetPlugin;
    use bevy::gizmos::{
        GizmoAsset, GizmoHandles,
        config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    };
    use bevy::input::mouse::{MouseMotion, MouseWheel};
    use bevy::mesh::Mesh;
    use bevy::pbr::StandardMaterial;
    use universe_core::guides::WHAT;

    /// Headless app with the full dive pipeline (guides included).
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

    /// Full text of one guides section by leaf id.
    fn section_text(app: &mut App, id: &str) -> String {
        let section = app
            .world_mut()
            .query::<(Entity, &GuideSection)>()
            .iter(app.world())
            .find(|(_, section)| section.leaves.contains(&id))
            .map(|(entity, _)| entity)
            .expect("section exists");
        let mut out = String::new();
        let mut stack = vec![section];
        while let Some(entity) = stack.pop() {
            if let Some(text) = app.world().get::<Text>(entity) {
                out.push_str(&text.0);
                out.push('\n');
            }
            if let Some(children) = app.world().get::<Children>(entity) {
                for child in children.iter() {
                    stack.push(child);
                }
            }
        }
        out
    }

    #[test]
    fn every_spec_page_exists_with_its_approved_text() {
        let mut app = headless_app();
        app.update();
        for page in GuidePage::ALL {
            let text = section_text(&mut app, page.id());
            assert!(text.contains(page.title()), "{page:?} shows its title");
            match static_text(page) {
                Some(approved) => assert!(
                    text.contains(approved),
                    "{page:?} carries the approved Spec v1 text"
                ),
                None => {
                    for level in 1..=14u8 {
                        assert!(
                            text.contains(&format!("L{level}")),
                            "the ladder names L{level}"
                        );
                    }
                }
            }
        }
        assert!(
            section_text(&mut app, GuidePage::What.id()).contains(WHAT),
            "What is Astrolith matches the notion verbatim"
        );
    }

    #[test]
    fn category_selections_open_their_first_pages() {
        assert_eq!(guide_leaf_for("about"), "what");
        assert_eq!(guide_leaf_for("help"), "getting-started");
        assert_eq!(guide_leaf_for("ladder"), "ladder");
    }
}
