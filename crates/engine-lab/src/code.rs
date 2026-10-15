//! The shader code editor state: the game's shader text, the edited text,
//! and the last applied text (#666).
//!
//! The window shows a lab-only copy of the game's Planet squares shader.
//! Save validates the edited text and publishes it for the renderer;
//! an error keeps the last good text and records the message. The game's
//! file on disk is never written. Reset gives back the game's text, and a
//! restart starts there with the window closed.

/// The game's Planet squares shader text, embedded at compile time.
///
/// This is the lab's copy to show and edit; the file on disk is never
/// written, so the game never changes.
pub const GAME_CODE: &str = include_str!("../../universe-render/src/planet_squares.wgsl");

/// The shader code on show in the lab's separate code window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderCode {
    /// Text in the editor box.
    pub edited: String,
    /// Last text that passed validation and reached the renderer.
    pub applied: String,
    /// Last validation error, shown on the window's error line.
    pub error: Option<String>,
    /// The code window is open.
    pub open: bool,
}

impl Default for ShaderCode {
    fn default() -> Self {
        Self {
            edited: GAME_CODE.to_string(),
            applied: GAME_CODE.to_string(),
            error: None,
            open: false,
        }
    }
}

impl ShaderCode {
    /// Validate `source` for Save.
    ///
    /// Two gates, both fail-closed (an error keeps the last good text):
    /// imports first, then syntax. `#import` lines are Bevy-resolved and
    /// cannot be validated here, so edited text whose preprocessor lines
    /// differ from the game's is rejected with a plain message. The rest
    /// is syntax-checked with [`checkable`] plus [`PRELUDE`] stubs for
    /// the imported names; Bevy does the real type checking when the
    /// applied text reaches the renderer (see T2, #709).
    ///
    /// # Errors
    ///
    /// Returns the message for the window's error line when `source`
    /// changes the imports or does not parse.
    pub fn check(source: &str) -> Result<(), String> {
        if import_signature(source) != import_signature(GAME_CODE) {
            return Err(
                "Imports are Bevy-resolved and stay as in the game's shader; only the code below them is editable."
                    .to_string(),
            );
        }
        let plain = checkable(source);
        let mut stubs = String::new();
        for (name, declaration) in PRELUDE {
            if !declares(&plain, name) {
                stubs.push_str(declaration);
                stubs.push('\n');
            }
        }
        let checked = format!("{plain}\n{stubs}");
        // `plain` keeps every byte length of `source`, so spans point at
        // the same characters; show the real text with the same stubs.
        let shown = format!("{source}\n{stubs}");
        naga::front::wgsl::parse_str(&checked)
            .map(|_| ())
            .map_err(|error| error.emit_to_string(&shown))
    }

    /// Save: validate the edited text; on success it reaches the renderer
    /// and the error clears, on failure the last good text stays and the
    /// message is recorded.
    pub fn save(&mut self) {
        match Self::check(&self.edited) {
            Ok(()) => {
                self.applied.clone_from(&self.edited);
                self.error = None;
            }
            Err(message) => {
                self.error = Some(message);
            }
        }
    }

    /// Give back the game's code at once; the error clears, the window
    /// stays open.
    pub fn reset(&mut self) {
        self.edited = GAME_CODE.to_string();
        self.applied = GAME_CODE.to_string();
        self.error = None;
    }

    /// Copy code: the exact edited text, to paste into the game's code.
    #[must_use]
    pub fn copy_text(&self) -> String {
        self.edited.clone()
    }

    /// Text the renderer gets: the applied text on the planet, the game's
    /// text above it (L1-L9 have no surface).
    #[must_use]
    pub fn applied_text(&self, on_planet: bool) -> &str {
        if on_planet { &self.applied } else { GAME_CODE }
    }
}

/// Strip Bevy's shader preprocessing for the syntax pre-check, keeping
/// every byte length so error spans point at the real lines: `#...`
/// preprocessor lines (imports, conditionals) become blanks, a multi-line
/// `#import ... { ... }` block is blanked through its closing brace, and
/// each `#{DEF}` value becomes `0` padded with blanks. Bevy resolves both
/// when the applied text reaches the renderer.
fn checkable(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut in_import = false;
    for line in source.split_inclusive('\n') {
        let (body, term) = match line.strip_suffix("\r\n") {
            Some(body) => (body, "\r\n"),
            None => match line.strip_suffix('\n') {
                Some(body) => (body, "\n"),
                None => (line, ""),
            },
        };
        let blank = " ".repeat(body.len());
        let trimmed = body.trim_start();
        if in_import {
            out.push_str(&blank);
            if trimmed.starts_with('}') {
                in_import = false;
            }
        } else if trimmed.starts_with("#import") {
            out.push_str(&blank);
            in_import = body.contains('{') && !body.contains('}');
        } else if trimmed.starts_with('#') && !trimmed.starts_with("#{") {
            out.push_str(&blank);
        } else {
            let mut rest = body;
            while let Some(start) = rest.find("#{") {
                out.push_str(&rest[..start]);
                let after = &rest[start + 2..];
                match after.find('}') {
                    Some(rel) => {
                        out.push('0');
                        for _ in 0..rel + 2 {
                            out.push(' ');
                        }
                        rest = &after[rel + 1..];
                    }
                    None => {
                        out.push_str(rest);
                        rest = "";
                        break;
                    }
                }
            }
            out.push_str(rest);
        }
        out.push_str(term);
    }
    out
}

/// Stub declarations for the names Bevy's `#import` lines inject, so the
/// syntax pre-check resolves them. Shapes mirror Bevy 0.19's `bevy_pbr`
/// (`forward_io`, `mesh_view_bindings`, `pbr_fragment`, `pbr_functions`)
/// for the names the game's shader imports; Bevy does the real type
/// checking when the applied text reaches the renderer. A stub is left
/// out when the checked text declares the name itself.
const PRELUDE: [(&str, &str); 9] = [
    (
        "StandardMaterial",
        "struct StandardMaterial { base_color: vec4<f32>, };",
    ),
    (
        "PbrInput",
        "struct PbrInput { material: StandardMaterial, };",
    ),
    (
        "VertexOutput",
        "struct VertexOutput { world_position: vec4<f32>, world_normal: vec3<f32>, };",
    ),
    (
        "FragmentOutput",
        "struct FragmentOutput { color: vec4<f32>, };",
    ),
    ("View", "struct View { world_position: vec3<f32>, };"),
    ("view", "var<private> view: View;"),
    (
        "pbr_input_from_standard_material",
        "fn pbr_input_from_standard_material(_v: VertexOutput, _front: bool) -> PbrInput { return PbrInput(StandardMaterial(vec4<f32>(0.0))); }",
    ),
    (
        "alpha_discard",
        "fn alpha_discard(_material: StandardMaterial, _color: vec4<f32>) -> vec4<f32> { return _color; }",
    ),
    (
        "main_pass_post_lighting_processing",
        "fn main_pass_post_lighting_processing(_input: PbrInput, _color: vec4<f32>) -> vec4<f32> { return _color; }",
    ),
];

/// Whether `text` declares `name` itself (`struct`, `fn`, `var`, `const`,
/// `override` or `alias`); its stub is then left out of the prelude.
fn declares(text: &str, name: &str) -> bool {
    for prefix in [
        "struct ",
        "fn ",
        "var ",
        "var<",
        "const ",
        "override ",
        "alias ",
    ] {
        if text.contains(&format!("{prefix}{name}")) {
            return true;
        }
    }
    false
}

/// The `#`-preprocessor lines of `text`: an `#import` block through its
/// closing brace counts as one unit. Imports are Bevy-resolved, so Save
/// only accepts text with the game's signature.
fn import_signature(text: &str) -> String {
    let mut out = String::new();
    let mut in_import = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if in_import {
            out.push_str(line);
            out.push('\n');
            if trimmed.starts_with('}') {
                in_import = false;
            }
        } else if trimmed.starts_with("#import") {
            out.push_str(line);
            out.push('\n');
            in_import = line.contains('{') && !line.contains('}');
        } else if trimmed.starts_with('#') {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_code_window_shows_the_game_code_with_save_reset_and_copy() {
        // C1 / AC1: a fresh window shows the game's Planet squares code,
        // with Save, Reset, Copy code and an empty error line (ui.rs).
        let code = ShaderCode::default();
        assert!(!code.open, "a restart starts with the window closed");
        assert_eq!(code.edited, GAME_CODE);
        assert_eq!(code.applied, GAME_CODE);
        assert_eq!(code.error, None);
        assert!(
            GAME_CODE.contains("PlanetSquares"),
            "the game shader text is shown"
        );
        // The game's text parses, so Save applies it.
        assert_eq!(ShaderCode::check(GAME_CODE), Ok(()));
        // Save, Reset and Copy code become the window's three buttons (T3).
    }

    #[test]
    fn save_applies_edited_code_to_the_body_on_show() {
        // C2 / AC2: Save publishes valid edited text for the body on show.
        let mut code = ShaderCode {
            open: true,
            edited: GAME_CODE.replace("0.5", "0.0"),
            ..Default::default()
        };
        let tuned = code.edited.clone();
        assert_ne!(tuned, GAME_CODE, "the edit changes the text");
        code.save();
        assert_eq!(code.error, None);
        assert_eq!(code.applied, tuned);
        assert_eq!(code.applied_text(true), tuned);
    }

    #[test]
    fn shader_errors_keep_the_last_good_picture() {
        // C3 / AC3: an error Save keeps the last good text and records the
        // message; fixing the text and saving again recovers.
        let mut code = ShaderCode {
            open: true,
            edited: "this is not wgsl;".to_string(),
            ..Default::default()
        };
        code.save();
        assert_eq!(code.applied, GAME_CODE, "the last good text stays");
        let message = code.error.clone().expect("an error message");
        assert!(!message.is_empty(), "the error line is shown");
        code.edited = GAME_CODE.to_string();
        code.save();
        assert_eq!(code.error, None, "the error line clears");
        assert_eq!(code.applied, GAME_CODE);
    }

    #[test]
    fn reset_and_restart_give_the_game_code() {
        // C4 / AC4: Reset gives back the game's code at once; a restart
        // starts there with the window closed, and the lab writes no
        // shader file.
        let mut code = ShaderCode {
            open: true,
            edited: "this is not wgsl;".to_string(),
            ..Default::default()
        };
        code.save();
        code.reset();
        assert_eq!(code.edited, GAME_CODE);
        assert_eq!(code.applied, GAME_CODE);
        assert_eq!(code.error, None);
        assert!(code.open, "Reset keeps the window open");
        let start = ShaderCode::default();
        assert!(
            !start.open && start.edited == GAME_CODE && start.applied == GAME_CODE,
            "a restart starts from the game code"
        );
        for source in [include_str!("main.rs"), include_str!("window.rs")] {
            assert!(
                !source.contains("universe-render/src/"),
                "the lab writes no game file"
            );
        }
    }

    #[test]
    fn bevy_imports_and_defs_pass_the_precheck() {
        // The game text is Bevy-flavoured (`#import`, `#{DEF}`): the
        // pre-check neuters those lines byte-exactly and parses the rest.
        let plain = checkable(GAME_CODE);
        assert_eq!(plain.len(), GAME_CODE.len(), "spans stay aligned");
        assert!(
            !plain.lines().any(|line| {
                let trimmed = line.trim_start();
                trimmed.starts_with('#') && !trimmed.starts_with("//")
            }),
            "no preprocessor line is parsed"
        );
        assert!(!plain.contains("#{"), "no def substitution is parsed");
        // A changed import block stays Bevy's domain: rejected outright,
        // with a plain message, so only validated text reaches Bevy.
        let tweaked = GAME_CODE.replace("#import bevy_pbr::{", "#import bevy_nope::{");
        let Err(message) = ShaderCode::check(&tweaked) else {
            panic!("changed imports pass the pre-check")
        };
        assert!(
            message.contains("Imports are Bevy-resolved"),
            "the message names the rule: {message}"
        );
        // A broken code line fails, pointing at the real line.
        let broken = GAME_CODE.replace("let alpha = ", "let alpha = = ");
        let Err(message) = ShaderCode::check(&broken) else {
            panic!("broken code passes the pre-check")
        };
        assert!(
            message.contains("alpha"),
            "the message names the real line: {message}"
        );
    }

    #[test]
    fn copy_code_puts_the_edited_text_on_the_clipboard() {
        // C5 / AC5: the exact edited text, confirmed on the status bar
        // (ui.rs).
        let code = ShaderCode {
            edited: format!("{GAME_CODE}\n// lab note\n"),
            ..Default::default()
        };
        assert_eq!(code.copy_text(), code.edited);
        // Copy code reaches the platform clipboard through `clipboard` (T3).
    }

    #[test]
    fn save_is_greyed_above_the_planet() {
        // C6 / AC6: at L1-L9 nothing edited reaches the renderer, and the
        // window greys Save (ui.rs reads `on_planet`).
        let mut code = ShaderCode {
            edited: GAME_CODE.replace("0.5", "0.0"),
            ..Default::default()
        };
        code.save();
        assert_eq!(
            code.applied_text(false),
            GAME_CODE,
            "above the planet the game text stays"
        );
        assert_ne!(
            code.applied_text(true),
            GAME_CODE,
            "on the planet the edit applies"
        );
        // The window greys Save above the planet (T3 wires `on_planet`).
    }
}
