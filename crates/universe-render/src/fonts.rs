//! Bundled UI fonts and text roles for the screens (#473, ADR 0023).
//!
//! Inter (SIL Open Font License 1.1) sets every screen text; Lucide (ISC)
//! draws the icons. Both TTF files and their licence files live in
//! `assets/fonts/` and are embedded at build time, so nothing is loaded at
//! run time. A screen text carries one [`TextRole`]; [`apply_text_roles`]
//! gives it its font and size from `tokens.rs` (`E-TOKENS`), so spawn sites
//! never thread font handles. Icons are named once in [`Icon`].

use crate::tokens::{TEXT_BODY_PX, TEXT_ICON_PX, TEXT_LIST_PX, TEXT_NOTE_PX, TEXT_TITLE_PX};
use bevy::prelude::*;
use bevy::text::{Font, FontSource, TextFont};

/// Inter Regular 4.1 (rsms/inter release v4.1, `extras/ttf/Inter-Regular.ttf`).
const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");

/// Inter Medium 4.1 (titles).
const INTER_MEDIUM: &[u8] = include_bytes!("../assets/fonts/Inter-Medium.ttf");

/// Lucide icon font (lucide-static 1.55.0, `font/lucide.ttf`).
const LUCIDE: &[u8] = include_bytes!("../assets/fonts/lucide.ttf");

/// Handles to the bundled fonts.
#[derive(Resource, Debug, Clone, Default)]
pub(crate) struct UiFonts {
    /// Inter Regular: body, list, and note text.
    pub text: Handle<Font>,
    /// Inter Medium: screen titles.
    pub title: Handle<Font>,
    /// Lucide: icons.
    pub icons: Handle<Font>,
}

/// What a screen text is, which decides its font and size.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextRole {
    /// Screen title (Inter Medium, [`TEXT_TITLE_PX`]).
    Title,
    /// Buttons and body text (Inter, [`TEXT_BODY_PX`]).
    Body,
    /// Category list items (Inter, [`TEXT_LIST_PX`]).
    ListItem,
    /// Small notes such as "Coming soon" (Inter, [`TEXT_NOTE_PX`]).
    Note,
    /// One Lucide icon glyph ([`TEXT_ICON_PX`]).
    Icon,
}

impl TextRole {
    /// Font size in logical pixels (pure).
    #[must_use]
    pub(crate) const fn size_px(self) -> f32 {
        match self {
            Self::Title => TEXT_TITLE_PX,
            Self::Body => TEXT_BODY_PX,
            Self::ListItem => TEXT_LIST_PX,
            Self::Note => TEXT_NOTE_PX,
            Self::Icon => TEXT_ICON_PX,
        }
    }
}

/// Lucide icons the screens use, by codepoint (lucide-static 1.55.0
/// `font/codepoints.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Icon {
    /// `menu` (☰): the menu button.
    Menu,
    /// `x` (✕): close.
    Close,
    /// `chevron-left` (‹): Back.
    Back,
    /// `play` (▶): Resume.
    Resume,
    /// `book-open`: Guides.
    Guides,
    /// `settings` (gear): Settings.
    Settings,
    /// `log-out` (exit arrow): Quit.
    Quit,
}

impl Icon {
    /// The icon's glyph in the Lucide font (pure).
    #[must_use]
    pub(crate) const fn glyph(self) -> char {
        match self {
            Self::Menu => '\u{e115}',
            Self::Close => '\u{e1b2}',
            Self::Back => '\u{e06e}',
            Self::Resume => '\u{e13c}',
            Self::Guides => '\u{e05f}',
            Self::Settings => '\u{e154}',
            Self::Quit => '\u{e10e}',
        }
    }

    /// Text bundle drawing this icon (font applied by [`apply_text_roles`]).
    #[must_use]
    pub(crate) fn text(self) -> (Text, TextRole) {
        (Text::new(self.glyph().to_string()), TextRole::Icon)
    }
}

/// Fonts plugin: embeds the fonts before any screen spawns and applies
/// text roles as texts appear.
pub(crate) struct FontsPlugin;

impl Plugin for FontsPlugin {
    /// Registers the font loader (startup) and the role system.
    fn build(&self, app: &mut App) {
        app.init_resource::<UiFonts>()
            .add_systems(PreStartup, load_ui_fonts)
            .add_systems(PostUpdate, apply_text_roles);
    }
}

/// Adds the embedded fonts to the font assets. Without a font store
/// (headless harness without text) the default handles stay in place.
pub(crate) fn load_ui_fonts(mut ui: ResMut<UiFonts>, fonts: Option<ResMut<Assets<Font>>>) {
    let Some(mut fonts) = fonts else {
        return;
    };
    ui.text = fonts.add(Font::from_bytes(INTER_REGULAR.to_vec()));
    ui.title = fonts.add(Font::from_bytes(INTER_MEDIUM.to_vec()));
    ui.icons = fonts.add(Font::from_bytes(LUCIDE.to_vec()));
}

/// Gives each newly tagged text its role's font and size.
pub(crate) fn apply_text_roles(
    ui: Res<UiFonts>,
    mut texts: Query<(&TextRole, &mut TextFont), Changed<TextRole>>,
) {
    for (role, mut font) in &mut texts {
        let handle = match role {
            TextRole::Title => ui.title.clone(),
            TextRole::Icon => ui.icons.clone(),
            TextRole::Body | TextRole::ListItem | TextRole::Note => ui.text.clone(),
        };
        font.font = FontSource::Handle(handle);
        font.font_size = role.size_px().into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_fonts_and_licences_ship_with_the_crate() {
        assert!(INTER_REGULAR.len() > 100_000);
        assert!(INTER_MEDIUM.len() > 100_000);
        assert!(LUCIDE.len() > 100_000);
        let inter = include_str!("../assets/fonts/Inter-LICENSE.txt");
        assert!(inter.contains("SIL Open Font License"));
        let lucide = include_str!("../assets/fonts/Lucide-LICENSE.txt");
        assert!(lucide.contains("ISC License"));
    }

    #[test]
    fn icons_are_distinct_private_use_glyphs() {
        let all = [
            Icon::Menu,
            Icon::Close,
            Icon::Back,
            Icon::Resume,
            Icon::Guides,
            Icon::Settings,
            Icon::Quit,
        ];
        for (index, icon) in all.iter().enumerate() {
            let glyph = u32::from(icon.glyph());
            assert!(
                (0xe000..=0xf8ff).contains(&glyph),
                "{icon:?} in the private use area"
            );
            assert!(
                all[index + 1..]
                    .iter()
                    .all(|other| other.glyph() != icon.glyph()),
                "{icon:?} unique"
            );
        }
    }

    #[test]
    fn roles_use_the_spec_sizes() {
        assert!((TextRole::Title.size_px() - 18.0).abs() < f32::EPSILON);
        assert!((TextRole::Body.size_px() - 14.0).abs() < f32::EPSILON);
        assert!((TextRole::ListItem.size_px() - 13.0).abs() < f32::EPSILON);
        assert!((TextRole::Note.size_px() - 11.0).abs() < f32::EPSILON);
    }
}
