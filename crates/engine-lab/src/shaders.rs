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
        let t = &self.tuning;
        let row = |label: &str, value: String| (label.to_string(), value);
        match shader {
            ShaderId::PlanetSquares => vec![
                row("Shown", if self.on { "on" } else { "off" }.to_string()),
                row("Square size", length_text(t.square_m)),
                row("Colour A", hex(t.colour_a)),
                row("Colour B", hex(t.colour_b)),
                row(
                    "Fade",
                    format!("sharp {} px, even {} px", t.sharp_px, t.gray_px),
                ),
                row("Debug view", t.debug.label().to_string()),
            ],
        }
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
        // Reset is the panel's button (ui.rs).
        assert!(include_str!("ui.rs").contains("\"Reset\""));
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
        assert!(include_str!("ui.rs").contains("add_enabled_ui(planet, |ui| shaders_ui("));
    }
}
