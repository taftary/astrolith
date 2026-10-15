//! The Shaders panel: the game's shaders on the body on show (#665).
//!
//! The Nodes list carries a "Shaders" line with one line per game shader
//! under it (today, Planet squares), and the menu bar a Shaders menu. A
//! shader applies to the body on show (the planet, or a moon shown with F),
//! one at a time, the way the game uses it (Q3). Its settings change the
//! picture at once; they are lab-only and start from the game's values on
//! every run (Q4). At L1-L9 there is no surface, so all of it is greyed.

use crate::units::length_text;
use std::ops::RangeInclusive;
use universe_render::{DebugView, SquaresTuning};

/// A game shader the lab can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderId {
    /// The black-and-white squares on planets (#626).
    PlanetSquares,
}

impl ShaderId {
    /// Every game shader, in list order.
    pub const ALL: [ShaderId; 1] = [ShaderId::PlanetSquares];

    /// Its name in the Nodes list and the Shaders menu.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            ShaderId::PlanetSquares => "Planet squares",
        }
    }
}

/// Square size slider, metres.
pub const SQUARE_RANGE_M: RangeInclusive<f64> = 1.0..=10_000.0;

/// Sharp fade size slider, pixels.
pub const SHARP_RANGE_PX: RangeInclusive<f64> = 2.0..=64.0;

/// Even fade size slider, pixels.
pub const EVEN_RANGE_PX: RangeInclusive<f64> = 1.0..=16.0;

/// The distance view's range fields, metres.
pub const DISTANCE_RANGE_M: RangeInclusive<f64> = 0.0..=1.0e8;

/// The even size always stays this much below the sharp one.
const EVEN_GAP_PX: f64 = 0.5;

/// The shader on show and its settings. The default is the game's own
/// look with the shader off, as the game starts.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ShaderPanel {
    /// Planet squares is on.
    pub on: bool,
    /// What the shader draws with.
    pub tuning: SquaresTuning,
}

impl ShaderPanel {
    /// Keeps every setting inside its slider, the even fade below the
    /// sharp one, and the colours in 0..1.
    pub fn clamp(&mut self) {
        let t = &mut self.tuning;
        t.square_m = clamp_to(t.square_m, &SQUARE_RANGE_M);
        t.sharp_px = clamp_to(t.sharp_px, &SHARP_RANGE_PX);
        t.gray_px = clamp_to(t.gray_px, &EVEN_RANGE_PX).min(t.sharp_px - EVEN_GAP_PX);
        for channel in t.colour_a.iter_mut().chain(t.colour_b.iter_mut()) {
            *channel = clamp_to(*channel, &(0.0..=1.0));
        }
        let [near, far] = t.distance_m.map(|m| clamp_to(m, &DISTANCE_RANGE_M));
        t.distance_m = [near.min(far - 1.0).max(0.0), far.max(1.0)];
    }

    /// Copy values: the settings as Rust constants, written the way
    /// `universe_core::squares` writes them, to paste into the game.
    #[must_use]
    pub fn copy_text(&self) -> String {
        let t = &self.tuning;
        let number = |x: f64| format!("{:?}", (x * 1_000.0).round() / 1_000.0);
        let colour = |rgb: [f64; 3]| {
            let [r, g, b] = rgb.map(number);
            format!("[{r}, {g}, {b}]")
        };
        format!(
            "// Planet squares from the engine lab (#665), for crates/universe-core/src/squares.rs.\n\
             pub const SQUARE_M: f64 = {};\n\
             pub const SHARP_PX: f64 = {};\n\
             pub const GRAY_PX: f64 = {};\n\
             pub const COLOUR_A: [f64; 3] = {};\n\
             pub const COLOUR_B: [f64; 3] = {};\n",
            number(t.square_m),
            number(t.sharp_px),
            number(t.gray_px),
            colour(t.colour_a),
            colour(t.colour_b),
        )
    }

    /// Gives the settings back their game values; the on/off and the
    /// debug view stay.
    pub fn reset(&mut self) {
        self.tuning = SquaresTuning {
            debug: self.tuning.debug,
            distance_m: self.tuning.distance_m,
            ..SquaresTuning::default()
        };
    }

    /// What the game's renderer gets: the squares on/off and the tuning,
    /// both off above the planet (L1-L9 have no surface).
    #[must_use]
    pub fn applied(&self, on_planet: bool) -> (bool, SquaresTuning) {
        if on_planet {
            (self.on, self.tuning)
        } else {
            (
                false,
                SquaresTuning {
                    debug: DebugView::Off,
                    ..self.tuning
                },
            )
        }
    }

    /// The rows the tools panel shows for `shader`.
    #[must_use]
    pub fn rows(&self, shader: ShaderId) -> Vec<(String, String)> {
        match shader {
            ShaderId::PlanetSquares => self.values(),
        }
    }

    /// The rows the tools panel shows for one library entry: on/off and
    /// the four settings from the game's values (#667 Q5).
    #[must_use]
    pub fn values(&self) -> Vec<(String, String)> {
        let t = &self.tuning;
        let row = |label: &str, value: String| (label.to_string(), value);
        vec![
            row("Shown", if self.on { "on" } else { "off" }.to_string()),
            row("Square size", length_text(t.square_m)),
            row("Colour A", hex(t.colour_a)),
            row("Colour B", hex(t.colour_b)),
            row(
                "Fade",
                format!("sharp {} px, even {} px", t.sharp_px, t.gray_px),
            ),
            row("Debug view", t.debug.label().to_string()),
        ]
    }

    /// The rows of the Shaders line: each shader and whether it is on.
    #[must_use]
    pub fn summary(&self) -> Vec<(String, String)> {
        ShaderId::ALL
            .iter()
            .map(|id| {
                let on = match id {
                    ShaderId::PlanetSquares => self.on,
                };
                (
                    id.label().to_string(),
                    if on { "on" } else { "off" }.to_string(),
                )
            })
            .collect()
    }
}

fn clamp_to(value: f64, range: &RangeInclusive<f64>) -> f64 {
    if value.is_nan() {
        *range.start()
    } else {
        value.clamp(*range.start(), *range.end())
    }
}

/// `#rrggbb` of a 0..1 colour.
#[must_use]
pub fn hex(rgb: [f64; 3]) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "E-CAST: a 0..1 channel rounded to 0..255"
    )]
    let byte = |c: f64| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    let [r, g, b] = rgb.map(byte);
    format!("#{r:02x}{g:02x}{b:02x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(rows: &[(String, String)], label: &str) -> String {
        rows.iter()
            .find(|(l, _)| l == label)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    }

    #[test]
    fn the_shaders_line_and_menu_list_planet_squares() {
        // C1 / AC1: a "Shaders" line with "Planet squares" under it in the
        // Nodes list, after Camera, Light, Grid and Axes, at every level,
        // and a Shaders menu with the same shader.
        use crate::nodes::{Node, shader_nodes, tree};
        assert_eq!(
            shader_nodes(),
            vec![Node::Shaders, Node::Shader(ShaderId::PlanetSquares)]
        );
        assert_eq!(Node::Shaders.label(), "Shaders");
        assert_eq!(
            Node::Shader(ShaderId::PlanetSquares).label(),
            "Planet squares"
        );
        let nodes = tree(1);
        let axes = nodes.iter().position(|n| *n == Node::Axes).expect("Axes");
        assert_eq!(&nodes[axes + 1..], shader_nodes().as_slice());
        let scene = crate::scene::LabScene::default_home();
        let rows = scene.nodes(&scene.universe());
        assert!(
            rows.iter()
                .any(|(n, l)| *n == Node::Shaders && l == "Shaders")
        );
        let menu = crate::layout::MENUS
            .iter()
            .find(|(name, _)| *name == "Shaders")
            .expect("a Shaders menu");
        assert!(menu.1.contains(&"Planet squares"));
    }

    #[test]
    fn the_selected_shader_shows_its_controls() {
        // C2 / AC1: on/off and the four settings, from the game's values.
        let panel = ShaderPanel::default();
        let rows = panel.rows(ShaderId::PlanetSquares);
        assert_eq!(value(&rows, "Shown"), "off");
        assert_eq!(value(&rows, "Square size"), "100 m");
        assert_eq!(value(&rows, "Colour A"), "#000000");
        assert_eq!(value(&rows, "Colour B"), "#ffffff");
        assert_eq!(value(&rows, "Fade"), "sharp 8 px, even 2 px");
        assert_eq!(value(&rows, "Debug view"), "Off");
        assert_eq!(
            panel.summary(),
            vec![("Planet squares".to_string(), "off".to_string())]
        );
        // Reset and Copy values are the panel's two buttons (ui.rs).
        let ui = include_str!("ui.rs");
        for button in ["\"Reset\"", "\"Copy values\""] {
            assert!(ui.contains(button), "the tools panel has {button}");
        }
    }

    #[test]
    fn squares_follow_the_body_on_show_and_start_off() {
        // C3 / AC2: off at start; on gives the squares to the body on
        // show (the planet or a moon on show both read `on_planet`).
        let mut panel = ShaderPanel::default();
        assert_eq!(panel.applied(true), (false, SquaresTuning::default()));
        panel.on = true;
        assert_eq!(panel.applied(true), (true, SquaresTuning::default()));
        panel.on = false;
        assert!(!panel.applied(true).0, "off draws the Normal look");
    }

    #[test]
    fn live_settings_reach_the_shader_within_limits() {
        // C4 / AC3: every change goes straight to the renderer's tuning,
        // inside the sliders, the even fade below the sharp one.
        let mut panel = ShaderPanel {
            on: true,
            ..ShaderPanel::default()
        };
        panel.tuning.square_m = 1_000.0;
        panel.tuning.colour_a = [1.0, 0.0, 0.0];
        panel.clamp();
        let (_, tuning) = panel.applied(true);
        assert_eq!(tuning.square_m, 1_000.0);
        assert_eq!(tuning.colour_a, [1.0, 0.0, 0.0]);
        panel.tuning.square_m = 0.01;
        panel.tuning.sharp_px = 500.0;
        panel.clamp();
        assert_eq!(panel.tuning.square_m, 1.0);
        assert_eq!(panel.tuning.sharp_px, 64.0);
        panel.tuning.square_m = 1.0e9;
        panel.tuning.sharp_px = 2.0;
        panel.tuning.gray_px = 16.0;
        panel.tuning.colour_b = [2.0, -1.0, f64::NAN];
        panel.clamp();
        assert_eq!(panel.tuning.square_m, 10_000.0);
        assert!(
            panel.tuning.gray_px < panel.tuning.sharp_px,
            "even below sharp"
        );
        assert!(EVEN_RANGE_PX.contains(&panel.tuning.gray_px));
        assert_eq!(panel.tuning.colour_b, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn reset_and_restart_give_the_game_values() {
        // C5 / AC3, AC4: Reset gives back 100 m, black, white, 8 px and
        // 2 px; a new run starts there with the shader off, and the lab
        // never reads or writes a settings file.
        let mut panel = ShaderPanel {
            on: true,
            tuning: SquaresTuning {
                square_m: 7.0,
                colour_a: [0.2, 0.3, 0.4],
                colour_b: [0.5, 0.5, 0.0],
                sharp_px: 30.0,
                gray_px: 9.0,
                ..SquaresTuning::default()
            },
        };
        panel.reset();
        let game = SquaresTuning::default();
        assert_eq!(panel.tuning, game);
        assert_eq!(
            (
                game.square_m,
                game.colour_a,
                game.colour_b,
                game.sharp_px,
                game.gray_px
            ),
            (100.0, [0.0; 3], [1.0; 3], 8.0, 2.0)
        );
        assert!(panel.on, "Reset keeps the shader on");
        let start = ShaderPanel::default();
        assert!(!start.on && start.tuning == game, "a restart starts there");
        for source in [include_str!("main.rs"), include_str!("window.rs")] {
            assert!(
                !source.contains("settings::load") && !source.contains("settings::save"),
                "the lab keeps no settings file"
            );
        }
    }

    #[test]
    fn copy_values_writes_rust_constants() {
        // C6 / AC5: the current values as Rust constants named as in the
        // game's code; the status bar note comes from ui.rs.
        let mut panel = ShaderPanel::default();
        panel.tuning.square_m = 1_000.0;
        panel.tuning.colour_a = [1.0, 0.0, 0.0];
        let text = panel.copy_text();
        for line in [
            "pub const SQUARE_M: f64 = 1000.0;",
            "pub const SHARP_PX: f64 = 8.0;",
            "pub const GRAY_PX: f64 = 2.0;",
            "pub const COLOUR_A: [f64; 3] = [1.0, 0.0, 0.0];",
            "pub const COLOUR_B: [f64; 3] = [1.0, 1.0, 1.0];",
        ] {
            assert!(text.lines().any(|l| l == line), "{line} in {text}");
        }
        // The names are the game's own constants.
        let core = include_str!("../../universe-core/src/squares.rs");
        for name in [
            "SQUARE_M: f64",
            "SHARP_PX: f64",
            "GRAY_PX: f64",
            "COLOUR_A: [f64; 3]",
            "COLOUR_B: [f64; 3]",
        ] {
            assert!(core.contains(&format!("pub const {name} =")), "{name}");
        }
        panel.tuning.square_m = 123.456_789;
        assert!(panel.copy_text().contains("SQUARE_M: f64 = 123.457;"));
        let ui = include_str!("ui.rs");
        assert!(ui.contains("clipboard::copy(&panel.copy_text())"));
        assert!(ui.contains("\"Values copied\""));
    }

    #[test]
    fn debug_views_select_what_the_shader_shows() {
        // C7 / AC6: each view reaches the renderer on the body on show,
        // with the squares on or off; Off gives the normal picture back.
        for on in [false, true] {
            let mut panel = ShaderPanel {
                on,
                ..ShaderPanel::default()
            };
            for view in DebugView::ALL {
                panel.tuning.debug = view;
                let (shown, tuning) = panel.applied(true);
                assert_eq!(shown, on);
                assert_eq!(tuning.debug, view);
            }
            panel.tuning.debug = DebugView::Off;
            assert_eq!(panel.applied(true).1, SquaresTuning::default());
        }
        let labels: Vec<&str> = DebugView::ALL.iter().map(|v| v.label()).collect();
        assert_eq!(labels, ["Off", "Position", "Normal", "Distance"]);
        // The distance range stays ordered and non-negative.
        let mut panel = ShaderPanel::default();
        assert_eq!(
            panel.tuning.distance_m,
            [0.0, 10_000.0],
            "default 0 m to 10 km"
        );
        panel.tuning.distance_m = [500.0, 100.0];
        panel.clamp();
        let [near, far] = panel.tuning.distance_m;
        assert!(0.0 <= near && near < far);
        // Reset keeps the debug view; the panel and the menu offer it.
        panel.tuning.debug = DebugView::Normal;
        panel.reset();
        assert_eq!(panel.tuning.debug, DebugView::Normal);
        let ui = include_str!("ui.rs");
        assert!(ui.contains("debug_view_ui(ui, &mut panel.tuning)"));
        assert!(ui.contains("(\"Shaders\", \"Debug view\")"));
        let menu = crate::layout::MENUS
            .iter()
            .find(|(name, _)| *name == "Shaders")
            .expect("a Shaders menu");
        assert!(menu.1.contains(&"Debug view"));
    }

    #[test]
    fn shaders_are_greyed_above_the_planet() {
        // C8 / AC7: at L1-L9 nothing reaches the renderer, and the panel
        // greys its controls (ui.rs reads `on_planet`).
        let mut panel = ShaderPanel {
            on: true,
            ..ShaderPanel::default()
        };
        panel.tuning.debug = DebugView::Normal;
        let (on, tuning) = panel.applied(false);
        assert!(!on);
        assert_eq!(tuning.debug, DebugView::Off);
        assert!(include_str!("ui.rs").contains("add_enabled_ui(planet, |ui|"));
    }
}
