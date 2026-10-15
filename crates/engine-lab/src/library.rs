//! The shader library: Planet squares plus shaders created in the lab (#667).
//!
//! The library is lab-only: one in-memory list with Planet squares (the
//! game's shader, entry 0, never renamed or deleted) plus created shaders,
//! each with a name of its own. Every entry keeps its own on/off, settings
//! ([`ShaderPanel`]) and code ([`ShaderCode`]); selecting an entry makes it
//! the shader on the body on show at once. A new shader starts blank
//! ([`BLANK_CODE`], the Normal look) and is edited like #666. Created
//! shaders, their code and their settings are never saved: a restart starts
//! again with only Planet squares. The game never changes: reaching it is
//! Copy code / Copy values by hand.

use crate::code::{GAME_CODE, ShaderCode};
use crate::shaders::ShaderPanel;
use universe_render::SquaresTuning;

/// The Planet squares entry: always id 0, never renamed or deleted.
pub const PLANET_SQUARES_ENTRY: u32 = 0;

/// A new shader starts as this minimal working shader (#667 Q2): the exact
/// same imports, uniforms and bindings as the game's Planet squares shader
/// (so the #666 Save pre-check and Bevy's bind layout accept it), but the
/// fragment returns the base colour, the Normal look, until it is edited.
/// The #665 settings therefore reach it at once yet stay invisible until
/// its code uses them, as the spec says.
pub const BLANK_CODE: &str = concat!(
    "// New shader (#667): a blank library entry. The imports, uniforms and\n",
    "// bindings match the game's Planet squares shader, so Save accepts it;\n",
    "// the fragment returns the base colour (the Normal look) until edited.\n",
    "\n",
    "#import bevy_pbr::{\n",
    "    pbr_fragment::pbr_input_from_standard_material,\n",
    "    pbr_functions::{alpha_discard, main_pass_post_lighting_processing},\n",
    "    forward_io::{VertexOutput, FragmentOutput},\n",
    "    mesh_view_bindings::view,\n",
    "}\n",
    "\n",
    "struct PlanetSquares {\n",
    "    // Rows of the render-to-planet rotation (xyz; w unused).\n",
    "    row_x: vec4<f32>,\n",
    "    row_y: vec4<f32>,\n",
    "    row_z: vec4<f32>,\n",
    "    // xyz: the render origin's planet position in metres, reduced into\n",
    "    // one period; w: metres per render unit.\n",
    "    origin: vec4<f32>,\n",
    "    // x: 1 with Squares on; y: square edge in metres; z: sharp size in\n",
    "    // pixels; w: gray size in pixels.\n",
    "    look: vec4<f32>,\n",
    "    // rgb: colour of the parity-0 squares.\n",
    "    colour_a: vec4<f32>,\n",
    "    // rgb: colour of the parity-1 squares.\n",
    "    colour_b: vec4<f32>,\n",
    "    // x: debug view (0 off, 1 position, 2 normal, 3 distance); y, z: the\n",
    "    // distance view's near and far metres.\n",
    "    debug: vec4<f32>,\n",
    "}\n",
    "\n",
    "@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> squares: PlanetSquares;\n",
    "\n",
    "@fragment\n",
    "fn fragment(\n",
    "    in: VertexOutput,\n",
    "    @builtin(front_facing) is_front: bool,\n",
    ") -> FragmentOutput {\n",
    "    var pbr_input = pbr_input_from_standard_material(in, is_front);\n",
    "    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);\n",
    "    var out: FragmentOutput;\n",
    "    out.color = pbr_input.material.base_color;\n",
    "    out.color = main_pass_post_lighting_processing(pbr_input, out.color);\n",
    "    return out;\n",
    "}\n",
);

/// One library entry: a name with its own panel and code.
#[derive(Debug, Clone, PartialEq)]
pub struct LibraryEntry {
    /// Stable id: 0 is Planet squares, created shaders count up from 1.
    pub id: u32,
    /// Shown in the Nodes list and the Shaders menu; unique in the run.
    pub name: String,
    /// Its on/off and settings (#665).
    pub panel: ShaderPanel,
    /// Its edited, applied and error text (#666).
    pub code: ShaderCode,
}

/// The lab-only shader list of one run.
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderLibrary {
    entries: Vec<LibraryEntry>,
    /// The entry on the body on show.
    selected: u32,
    /// The number the next created shader takes ("Shader N").
    next: u32,
}

impl Default for ShaderLibrary {
    fn default() -> Self {
        Self {
            entries: vec![LibraryEntry {
                id: PLANET_SQUARES_ENTRY,
                name: "Planet squares".to_string(),
                panel: ShaderPanel::default(),
                code: ShaderCode::default(),
            }],
            selected: PLANET_SQUARES_ENTRY,
            next: 1,
        }
    }
}

impl ShaderLibrary {
    /// Every entry, in list order.
    #[must_use]
    pub fn entries(&self) -> &[LibraryEntry] {
        &self.entries
    }

    /// The id on the body on show.
    #[must_use]
    pub fn selected(&self) -> u32 {
        self.selected
    }

    /// The entry on the body on show, if it still exists.
    #[must_use]
    pub fn selected_entry(&self) -> Option<&LibraryEntry> {
        self.entry(self.selected)
    }

    /// The entry on the body on show, to edit, if it still exists.
    pub fn selected_entry_mut(&mut self) -> Option<&mut LibraryEntry> {
        let selected = self.selected;
        self.entry_mut(selected)
    }

    /// The entry `id`, if it exists.
    #[must_use]
    pub fn entry(&self, id: u32) -> Option<&LibraryEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    /// The entry `id`, to edit, if it exists.
    pub fn entry_mut(&mut self, id: u32) -> Option<&mut LibraryEntry> {
        self.entries.iter_mut().find(|entry| entry.id == id)
    }

    /// Whether `id` is the game's shader (never renamed or deleted).
    #[must_use]
    pub const fn is_planet_squares(id: u32) -> bool {
        id == PLANET_SQUARES_ENTRY
    }

    /// The Planet squares entry (id 0 always exists unless deleted, which
    /// it cannot be), if it is still listed.
    #[must_use]
    pub fn planet_squares(&self) -> Option<&LibraryEntry> {
        self.entry(PLANET_SQUARES_ENTRY)
    }

    /// The Planet squares entry, to edit, if it is still listed.
    pub fn planet_squares_mut(&mut self) -> Option<&mut LibraryEntry> {
        self.entry_mut(PLANET_SQUARES_ENTRY)
    }

    /// Whether the shown entry's applied text differs from its start:
    /// while true, a render validation error is its doing (T2).
    #[must_use]
    pub fn selected_is_custom(&self) -> bool {
        let id = self.selected;
        self.entry(id)
            .is_some_and(|entry| entry.code.applied != Self::default_code(id))
    }

    /// The code an entry starts from: the game's text for Planet squares,
    /// the blank shader for a created one.
    #[must_use]
    pub const fn default_code(id: u32) -> &'static str {
        if Self::is_planet_squares(id) {
            GAME_CODE
        } else {
            BLANK_CODE
        }
    }

    /// Add "Shader N" and select it at once. Returns its id.
    pub fn new_shader(&mut self) -> u32 {
        let id = self.next;
        self.next += 1;
        let blank = BLANK_CODE.to_string();
        self.entries.push(LibraryEntry {
            id,
            name: format!("Shader {id}"),
            panel: ShaderPanel::default(),
            code: ShaderCode {
                edited: blank.clone(),
                applied: blank,
                error: None,
                open: false,
            },
        });
        self.selected = id;
        id
    }

    /// Make `id` the shader on the body on show. Returns whether it exists.
    pub fn select(&mut self, id: u32) -> bool {
        if self.entry(id).is_some() {
            self.selected = id;
            true
        } else {
            false
        }
    }

    /// Rename a created shader; names stay unique (a taken name gains a
    /// number). Planet squares cannot be renamed.
    ///
    /// # Errors
    ///
    /// Returns the message for the tools panel when `id` is Planet squares,
    /// missing, or the name is empty.
    pub fn rename(&mut self, id: u32, name: &str) -> Result<(), String> {
        if Self::is_planet_squares(id) {
            return Err("Planet squares cannot be renamed".to_string());
        }
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("A shader needs a name".to_string());
        }
        if self.entry(id).is_none() {
            return Err("That shader is gone".to_string());
        }
        let mut unique = trimmed.to_string();
        let mut suffix = 2;
        while self
            .entries
            .iter()
            .any(|entry| entry.id != id && entry.name == unique)
        {
            unique = format!("{trimmed} ({suffix})");
            suffix += 1;
        }
        if let Some(entry) = self.entry_mut(id) {
            entry.name = unique;
        }
        Ok(())
    }

    /// Delete a created shader at once; deleting the shown one falls back
    /// to the Normal look on Planet squares. Planet squares cannot be
    /// deleted.
    ///
    /// # Errors
    ///
    /// Returns the message for the tools panel when `id` is Planet squares
    /// or missing.
    pub fn delete(&mut self, id: u32) -> Result<(), String> {
        if Self::is_planet_squares(id) {
            return Err("Planet squares cannot be deleted".to_string());
        }
        let Some(at) = self.entries.iter().position(|entry| entry.id == id) else {
            return Err("That shader is gone".to_string());
        };
        self.entries.remove(at);
        if self.selected == id {
            self.selected = PLANET_SQUARES_ENTRY;
        }
        Ok(())
    }

    /// Give back an entry's start at once: the game's code for Planet
    /// squares, the blank shader for a created one. The error clears.
    pub fn reset_entry(&mut self, id: u32) {
        let Some(entry) = self.entry_mut(id) else {
            return;
        };
        if Self::is_planet_squares(id) {
            entry.code.reset();
        } else {
            entry.code.edited = BLANK_CODE.to_string();
            entry.code.applied = BLANK_CODE.to_string();
            entry.code.error = None;
        }
    }

    /// Revert the shown entry to its start after Bevy rejects its applied
    /// text (T2): the asset and the state agree again, and Bevy's message
    /// stays on the error line.
    pub fn revert_selected(&mut self, message: String) {
        let selected = self.selected;
        self.reset_entry(selected);
        if let Some(entry) = self.entry_mut(selected) {
            entry.code.error = Some(message);
        }
    }

    /// What the renderer gets for the body on show: the shown entry's
    /// on/off, its tuning, and its applied text. Above the planet (L1-L9
    /// have no surface) everything stays off on the game's text. A missing
    /// selection falls back to Planet squares off.
    #[must_use]
    pub fn applied(&self, on_planet: bool) -> (bool, SquaresTuning, &str) {
        let Some(entry) = self.selected_entry() else {
            return (false, SquaresTuning::default(), GAME_CODE);
        };
        let (on, tuning) = entry.panel.applied(on_planet);
        (on, tuning, entry.code.applied_text(on_planet))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_code_passes_the_save_precheck() {
        // The blank template keeps the game's imports (Save refuses
        // changed imports), so a new shader is applicable from the start.
        assert_eq!(ShaderCode::check(BLANK_CODE), Ok(()));
        assert_ne!(BLANK_CODE, GAME_CODE, "blank is not a copy");
    }

    #[test]
    fn new_shader_appears_selected_with_a_unique_name() {
        // C1 / AC1: "New shader" adds and selects "Shader 1", "Shader 2".
        let mut library = ShaderLibrary::default();
        assert_eq!(library.entries().len(), 1, "only Planet squares at start");
        let first = library.new_shader();
        let second = library.new_shader();
        assert_ne!(first, second);
        assert_eq!(library.selected(), second);
        let names: Vec<&str> = library
            .entries()
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert_eq!(names, ["Planet squares", "Shader 1", "Shader 2"]);
    }

    #[test]
    fn selecting_an_entry_shows_it_on_the_body_on_show() {
        // C2 / AC2: selecting a library entry shows it at once; only one
        // draws at a time; a missing id selects nothing.
        let mut library = ShaderLibrary::default();
        let created = library.new_shader();
        assert!(library.select(PLANET_SQUARES_ENTRY));
        assert_eq!(library.selected(), PLANET_SQUARES_ENTRY);
        assert!(library.select(created));
        assert_eq!(library.selected(), created);
        assert!(!library.select(999), "a missing shader selects nothing");
        assert_eq!(library.selected(), created);
    }

    #[test]
    fn blank_shader_draws_normal_until_edited() {
        // C3 / AC3: a new shader is the Normal look (no squares) until its
        // code is edited and saved; Save then redraws at once.
        let mut library = ShaderLibrary::default();
        let id = library.new_shader();
        let entry = library.entry(id).expect("the new shader");
        assert_eq!(entry.code.applied, BLANK_CODE);
        let (_, _, text) = library.applied(true);
        assert_eq!(text, BLANK_CODE);
        {
            let edited = library.selected_entry_mut().expect("selected");
            edited.code.edited = BLANK_CODE.replace(
                "out.color = pbr_input.material.base_color;",
                "out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0);",
            );
            edited.code.save();
            assert_eq!(edited.code.error, None);
        }
        let (_, _, text) = library.applied(true);
        assert_ne!(text, BLANK_CODE, "the edit applies at once");
        assert!(text.contains("1.0, 0.0, 0.0"), "the planet turns red");
    }

    #[test]
    fn each_shader_keeps_its_own_settings_and_code() {
        // C4 / AC4-5: on/off, settings, code and error stay per entry;
        // switching keeps them.
        let mut library = ShaderLibrary::default();
        let created = library.new_shader();
        {
            let shown = library.selected_entry_mut().expect("selected");
            shown.panel.on = true;
            shown.panel.tuning.square_m = 1_000.0;
            shown.code.edited = "this is not wgsl;".to_string();
            shown.code.save();
        }
        assert!(
            library
                .selected_entry()
                .expect("selected")
                .code
                .error
                .is_some(),
            "its error stays"
        );
        assert!(library.select(PLANET_SQUARES_ENTRY));
        let (on, tuning, text) = library.applied(true);
        assert!(!on, "Planet squares is still off");
        assert_eq!(tuning, SquaresTuning::default());
        assert_eq!(text, GAME_CODE);
        assert!(library.select(created));
        let (on, tuning, _) = library.applied(true);
        assert!(on, "its on/off stays");
        assert_eq!(tuning.square_m, 1_000.0, "its settings stay");
        assert!(
            library
                .selected_entry()
                .expect("selected")
                .code
                .error
                .is_some(),
            "its error stays"
        );
    }

    #[test]
    fn rename_stays_unique_and_planet_squares_is_protected() {
        // C4 / AC4: rename keeps names unique; Planet squares is protected.
        let mut library = ShaderLibrary::default();
        let first = library.new_shader();
        let second = library.new_shader();
        assert!(library.rename(first, "Lava test").is_ok());
        assert_eq!(library.entry(first).expect("renamed").name, "Lava test");
        assert!(library.rename(second, "Lava test").is_ok());
        assert_eq!(library.entry(second).expect("unique").name, "Lava test (2)");
        assert!(library.rename(second, "  ").is_err(), "a name is needed");
        assert!(
            library.rename(PLANET_SQUARES_ENTRY, "Squares").is_err(),
            "Planet squares cannot be renamed"
        );
        assert_eq!(
            library.entry(PLANET_SQUARES_ENTRY).expect("game").name,
            "Planet squares"
        );
    }

    #[test]
    fn delete_removes_a_created_shader_at_once() {
        // C4 / AC4: Delete leaves the list at once; deleting the shown one
        // falls back to Planet squares; Planet squares cannot be deleted.
        let mut library = ShaderLibrary::default();
        let created = library.new_shader();
        assert!(library.delete(created).is_ok());
        assert_eq!(library.entries().len(), 1);
        assert_eq!(library.selected(), PLANET_SQUARES_ENTRY);
        assert!(
            library.delete(PLANET_SQUARES_ENTRY).is_err(),
            "Planet squares cannot be deleted"
        );
        assert!(library.delete(999).is_err(), "a missing shader is gone");
    }

    #[test]
    fn reset_entry_gives_back_each_start() {
        // C3 / AC3: Reset gives a created shader its blank start;
        // Planet squares keeps its #666 Reset to the game's code.
        let mut library = ShaderLibrary::default();
        let created = library.new_shader();
        {
            let shown = library.selected_entry_mut().expect("selected");
            shown.code.edited = BLANK_CODE.replace("base_color", "base_color_");
        }
        library.reset_entry(created);
        let entry = library.entry(created).expect("reset");
        assert_eq!(entry.code.edited, BLANK_CODE);
        assert_eq!(entry.code.applied, BLANK_CODE);
        assert_eq!(entry.code.error, None);
        library.reset_entry(PLANET_SQUARES_ENTRY);
        let game = library.entry(PLANET_SQUARES_ENTRY).expect("game");
        assert_eq!(game.code.edited, GAME_CODE);
    }

    #[test]
    fn created_shader_errors_keep_the_last_good_text() {
        // C3 / AC3: an error Save on a created shader keeps its last good
        // text and records the message; fixing and saving again recovers.
        let mut library = ShaderLibrary::default();
        library.new_shader();
        {
            let shown = library.selected_entry_mut().expect("selected");
            shown.code.edited = "this is not wgsl;".to_string();
            shown.code.save();
            assert_eq!(shown.code.applied, BLANK_CODE, "the last good text stays");
            assert!(shown.code.error.is_some(), "the error line is shown");
            shown.code.edited = BLANK_CODE.to_string();
            shown.code.save();
            assert_eq!(shown.code.error, None, "the error line clears");
            assert_eq!(shown.code.applied, BLANK_CODE);
        }
    }

    #[test]
    fn copy_texts_follow_the_selected_entry() {
        // C5 / AC5: Copy code puts the shown entry's exact edited text and
        // Copy values its settings as Rust constants; the status bar
        // confirms each (ui.rs).
        let mut library = ShaderLibrary::default();
        library.new_shader();
        {
            let shown = library.selected_entry_mut().expect("selected");
            shown.panel.tuning.square_m = 1_000.0;
            shown.code.edited = format!("{}\n// lab note\n", shown.code.edited);
        }
        let shown = library.selected_entry().expect("selected");
        assert_eq!(shown.code.copy_text(), shown.code.edited);
        assert!(
            shown
                .panel
                .copy_text()
                .contains("pub const SQUARE_M: f64 = 1000.0;"),
            "its settings as Rust constants"
        );
        let ui = include_str!("ui.rs");
        for button in ["\"Copy code\"", "\"Copy values\""] {
            assert!(ui.contains(button), "the panel has {button}");
        }
    }

    #[test]
    fn new_shader_and_save_are_greyed_above_the_planet() {
        // C7 / AC7: at L1-L9 nothing created reaches the renderer, and New
        // shader plus Save are greyed (ui.rs reads `on_planet`).
        let mut library = ShaderLibrary::default();
        library.new_shader();
        let (on, _, text) = library.applied(false);
        assert!(!on, "off above the planet");
        assert_eq!(text, GAME_CODE, "the game text stays above the planet");
        let ui = include_str!("ui.rs");
        assert!(ui.contains("add_enabled(planet, egui::Button::new(\"New shader\"))"));
        assert!(ui.contains("egui::Button::new(\"Save\")"));
    }

    #[test]
    fn the_library_ui_carries_new_rename_and_delete() {
        // C1/C4/C5 of #667: the Shaders menu and the tools panel carry
        // New shader, Rename and Delete; the code window follows the
        // selected entry; New shader is greyed above the planet.
        let ui = include_str!("ui.rs");
        for item in ["\"New shader\"", "\"Rename\"", "\"Delete\""] {
            assert!(ui.contains(item), "the library UI has {item}");
        }
        assert!(
            ui.contains("selected_entry_mut()"),
            "the code window follows the selection"
        );
        assert!(
            ui.contains("add_enabled(planet, egui::Button::new(\"New shader\"))"),
            "New shader is greyed above the planet"
        );
        let shaders = crate::layout::MENUS
            .iter()
            .find(|(name, _)| *name == "Shaders")
            .expect("a Shaders menu");
        assert!(shaders.1.contains(&"New shader"));
        assert!(
            shaders.1.contains(&"Planet squares"),
            "the game shader stays"
        );
    }

    #[test]
    fn restart_gives_only_planet_squares() {
        // C6 / AC6: a restart starts with only Planet squares (off, game
        // values, window closed); created shaders are never saved.
        let mut library = ShaderLibrary::default();
        library.new_shader();
        library.new_shader();
        let start = ShaderLibrary::default();
        assert_eq!(start.entries().len(), 1);
        assert_eq!(start.selected(), PLANET_SQUARES_ENTRY);
        let entry = start.selected_entry().expect("game");
        assert_eq!(entry.name, "Planet squares");
        assert!(!entry.panel.on, "off at start");
        assert_eq!(entry.panel.tuning, SquaresTuning::default());
        assert!(!entry.code.open, "the window starts closed");
        for source in [include_str!("main.rs"), include_str!("window.rs")] {
            assert!(
                !source.contains("library.save") && !source.contains("shader_library.toml"),
                "created shaders are never saved to a file"
            );
        }
    }
}
