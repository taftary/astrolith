//! The lab's editor panels in egui (#640, AC1, AC6): the menu bar, the
//! nodes tree, the tools and options panel, the status bar and the stats
//! overlay over the viewport.

use crate::window::Lab;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use engine_lab::clipboard;
use engine_lab::layout::{ABOUT, MENUS};
use engine_lab::nodes::{Node, NodeContext, SurfaceFigures, node_rows};
use engine_lab::orbit::{CameraMode, FOV, Preset};
use engine_lab::shaders::{
    DISTANCE_RANGE_M, EVEN_RANGE_PX, SHARP_RANGE_PX, SQUARE_RANGE_M, ShaderId, ShaderPanel,
};
use engine_lab::stats::{SceneReadout, overlay_lines};
use engine_lab::units::{grid_spacing_m, metres_per_unit};
use universe_render::{DebugView, SquaresTuning, SurfaceStats, Universe};

/// The game's bundled Inter font (ADR 0023), egui's only font here.
const INTER_REGULAR: &[u8] = include_bytes!("../../universe-render/assets/fonts/Inter-Regular.ttf");

/// Light shown in the Light node: straight down, daylight strength.
pub(crate) const LIGHT: ([f64; 3], f64) = ([0.0, -1.0, 0.0], 10_000.0);

/// Installs the Inter font once.
fn install_fonts(ctx: &egui::Context, done: &mut bool) {
    if *done {
        return;
    }
    *done = true;
    let mut fonts = egui::FontDefinitions::empty();
    fonts.font_data.insert(
        "inter".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(INTER_REGULAR)),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("inter".to_owned());
    }
    ctx.set_fonts(fonts);
}

/// What the node rows need from the scene this frame.
fn node_context(lab: &Lab, universe: &Universe, stats: &SurfaceStats) -> NodeContext {
    // The body on show: the planet, or the moon on its own (#654 D4).
    let radius_earth = universe
        .frame
        .as_ref()
        .map_or(1.0, universe_core::surface::SurfaceFrame::radius_earth);
    let radius_m = universe_core::terrain::PLANET_RADIUS_CELL * metres_per_unit(radius_earth);
    NodeContext {
        planet_name: lab.scene.planet_name(),
        radius_earth,
        surface: SurfaceFigures {
            faces: stats.faces,
            edges: stats.edges,
            vertices: stats.vertices,
            max_depth: stats.max_depth(),
        },
        toggles: lab.toggles,
        moons: lab.moons.clone(),
        camera_mode: lab.mode,
        camera: lab.pose().offset,
        fov: FOV,
        light: LIGHT,
        grid_m: grid_spacing_m(radius_m),
        axes_m: radius_m * 1.6,
        shaders: lab.shaders,
    }
}

/// The whole editor UI, once per frame.
#[allow(
    clippy::too_many_lines,
    reason = "one egui pass lays out every panel in order; splitting would thread the same borrows through helpers"
)]
pub(crate) fn lab_ui(
    mut contexts: EguiContexts,
    mut lab: ResMut<Lab>,
    universe: Res<Universe>,
    stats: Res<SurfaceStats>,
    meshes: Query<(), With<Mesh3d>>,
    mut exit: MessageWriter<AppExit>,
    mut fonts_done: Local<bool>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    install_fonts(ctx, &mut fonts_done);
    let mut root = egui::Ui::new(
        ctx.clone(),
        "engine-lab".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    let lab = &mut *lab;
    // Menu bar.
    egui::Panel::top("menu bar").show(&mut root, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            for (menu, items) in MENUS {
                ui.menu_button(menu, |ui| {
                    for item in items {
                        menu_item(ui, lab, menu, item, &mut exit);
                    }
                    if menu == "Camera" {
                        ui.separator();
                        for preset in Preset::ALL {
                            if ui.button(preset.label()).clicked() {
                                lab.preset(preset);
                                ui.close();
                            }
                        }
                    }
                });
            }
        });
    });
    // Status bar.
    egui::Panel::bottom("status bar").show(&mut root, |ui| {
        ui.horizontal(|ui| {
            ui.label(format!("seed {}", lab.scene.seed()));
            ui.separator();
            ui.label(format!("path {}", lab.scene.path_text()));
            ui.separator();
            ui.label(lab.scene.label());
            ui.separator();
            ui.label(lab.scene.level_text());
            if let Some(shot) = &lab.last_shot {
                ui.separator();
                ui.label(format!("saved {shot}"));
            }
            if let Some(note) = &lab.note {
                ui.separator();
                ui.label(note);
            }
        });
    });
    let cx = node_context(lab, &universe, &stats);
    // Nodes tree.
    egui::Panel::left("nodes").show(&mut root, |ui| {
        ui.heading("Nodes");
        let rows = lab.scene.nodes(&universe);
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (node, label) in rows {
                let indent = matches!(node, Node::Moon(_) | Node::Shader(_));
                ui.horizontal(|ui| {
                    if indent {
                        ui.add_space(16.0);
                    }
                    if ui.selectable_label(lab.selection.is(node), label).clicked() {
                        lab.selection.click_list(node);
                    }
                });
                if node == Node::Planet && cx.moons.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.weak("Moons: none");
                    });
                }
            }
        });
    });
    // Tools and options.
    egui::Panel::right("tools and options").show(&mut root, |ui| {
        ui.heading("Planet");
        ui.horizontal(|ui| {
            ui.label("Seed");
            let field = ui.text_edit_singleline(&mut lab.seed_text);
            if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                regenerate(lab);
            }
        });
        ui.horizontal(|ui| {
            ui.label("Path");
            let field = ui.text_edit_singleline(&mut lab.path_text);
            if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                regenerate(lab);
            }
        });
        if ui.button("Regenerate").clicked() {
            regenerate(lab);
        }
        if let Some(error) = lab.scene.error() {
            ui.colored_label(egui::Color32::from_rgb(230, 70, 70), error);
        }
        ui.horizontal(|ui| {
            if ui.button("Parent").clicked() {
                lab.parent();
            }
            if ui.button("Child").clicked() {
                lab.child();
            }
            if ui.button("Show selected (F)").clicked() {
                lab.show_selected();
            }
        });
        ui.label(lab.scene.level_text());
        let planet = lab.scene.on_planet();
        ui.add_enabled_ui(planet, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Previous").clicked() {
                    lab.scene.previous();
                    lab.rebuild = true;
                }
                let count = lab.scene.planet_count().max(1);
                let mut index = lab.scene.selected() + 1;
                if ui.add(egui::Slider::new(&mut index, 1..=count)).changed() {
                    lab.scene.select(index - 1);
                    lab.rebuild = true;
                }
                if ui.button("Next").clicked() {
                    lab.scene.next();
                    lab.rebuild = true;
                }
            })
        });
        ui.label(lab.scene.label());
        ui.separator();
        let Some(shown) = lab.selection.node() else {
            ui.weak("Nothing selected");
            return;
        };
        ui.heading(shown.label());
        egui::Grid::new("node values").striped(true).show(ui, |ui| {
            for (label, value) in node_rows(shown, &cx) {
                ui.label(label);
                ui.label(value);
                ui.end_row();
            }
        });
        match shown {
            Node::Planet => {
                ui.add_enabled(
                    planet,
                    egui::Checkbox::new(&mut lab.toggles.wireframe, "Wireframe"),
                );
                ui.add_enabled(
                    planet,
                    egui::Checkbox::new(&mut lab.toggles.normals, "Normals"),
                );
                ui.add_enabled(
                    planet,
                    egui::Checkbox::new(&mut lab.toggles.lod_colors, "LOD depth view"),
                );
                if lab.toggles.lod_colors {
                    ui.label("Splits per depth");
                    for (depth, count) in stats.per_depth.iter().enumerate() {
                        if *count > 0 {
                            ui.label(format!("depth {depth}: {count}"));
                        }
                    }
                }
            }
            Node::Camera => {
                let mut free = lab.mode == CameraMode::Free;
                if ui.checkbox(&mut free, "Free-fly (G)").changed() {
                    lab.toggle_mode();
                }
                if ui.button("Home").clicked() {
                    lab.home();
                }
            }
            Node::Grid => {
                ui.checkbox(&mut lab.toggles.grid, "Grid");
            }
            Node::Axes => {
                ui.checkbox(&mut lab.toggles.axes, "Axes");
            }
            Node::Moon(_) if lab.scene.moon_on_show().is_some() => {
                ui.checkbox(&mut lab.toggles.wireframe, "Wireframe");
                ui.checkbox(&mut lab.toggles.normals, "Normals");
                ui.checkbox(&mut lab.toggles.lod_colors, "LOD depth view");
            }
            Node::Shader(id) => {
                let note = ui
                    .add_enabled_ui(planet, |ui| shaders_ui(ui, &mut lab.shaders, id))
                    .inner;
                if note.is_some() {
                    lab.note = note;
                }
            }
            Node::Moon(_) | Node::Marker(_) | Node::Light | Node::Shaders => {}
        }
    });
    // Stats overlay over the viewport.
    let readout = SceneReadout {
        // The surface counts belong to a body on show (a planet or moon).
        triangles: if lab.scene.on_planet() {
            stats.faces
        } else {
            0
        },
        meshes: meshes.iter().count(),
        position_m: lab
            .pose()
            .offset
            .map(|unit| unit * engine_lab::units::scene_metres(&universe)),
        height_m: universe.path.height_above_ground_m().unwrap_or(f64::NAN),
        shell: if lab.scene.on_planet() {
            universe.shown_level().get()
        } else {
            lab.scene.level().get()
        },
        size: lab.scene.level_size(),
    };
    let lines = overlay_lines(lab.toggles.stats, &lab.times, &readout);
    if !lines.is_empty() {
        egui::Window::new("Stats")
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(200.0, 40.0))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                for line in lines {
                    ui.monospace(line);
                }
            });
    }
    if lab.about {
        let mut open = true;
        egui::Window::new("About").open(&mut open).show(ctx, |ui| {
            ui.label(ABOUT);
        });
        lab.about = open;
    }
    Ok(())
}

/// One menu item's action.
fn menu_item(
    ui: &mut egui::Ui,
    lab: &mut Lab,
    menu: &str,
    item: &str,
    exit: &mut MessageWriter<AppExit>,
) {
    // The planet's views are greyed above the planet (#654 D7).
    let planet = lab.scene.on_planet();
    let toggle = |ui: &mut egui::Ui, value: &mut bool| {
        ui.checkbox(value, item);
    };
    let planet_toggle = |ui: &mut egui::Ui, value: &mut bool| {
        ui.add_enabled(planet, egui::Checkbox::new(value, item));
    };
    match (menu, item) {
        ("View", "Wireframe") => planet_toggle(ui, &mut lab.toggles.wireframe),
        ("View", "Normals") => planet_toggle(ui, &mut lab.toggles.normals),
        ("View", "LOD depth view") => planet_toggle(ui, &mut lab.toggles.lod_colors),
        ("View", "Stats overlay") => toggle(ui, &mut lab.toggles.stats),
        ("View", "Grid") => toggle(ui, &mut lab.toggles.grid),
        ("View", "Axes") => toggle(ui, &mut lab.toggles.axes),
        ("Shaders", "Planet squares") => {
            let shown = Node::Shader(ShaderId::PlanetSquares);
            let response = ui.add_enabled(planet, egui::Checkbox::new(&mut lab.shaders.on, item));
            if response.clicked() {
                lab.selection.click_list(shown);
            }
        }
        ("Shaders", "Debug view") => {
            ui.add_enabled_ui(planet, |ui| {
                ui.menu_button(item, |ui| {
                    for view in DebugView::ALL {
                        ui.radio_value(&mut lab.shaders.tuning.debug, view, view.label());
                    }
                });
            });
        }
        _ => {
            if ui.button(item).clicked() {
                match item {
                    "Screenshot (F12)" => lab.screenshot = true,
                    "Quit" => {
                        exit.write(AppExit::Success);
                    }
                    "Orbit / Free-fly (G)" => lab.toggle_mode(),
                    "Home" => lab.home(),
                    "About" => lab.about = true,
                    _ => {}
                }
                ui.close();
            }
        }
    }
}

/// The selected shader's controls (#665 Q1, Q4, Q5): on/off, the settings
/// with their sliders, Reset, Copy values and the debug view. Returns the
/// status bar note of a copy.
fn shaders_ui(ui: &mut egui::Ui, panel: &mut ShaderPanel, id: ShaderId) -> Option<String> {
    let mut note = None;
    match id {
        ShaderId::PlanetSquares => {
            ui.checkbox(&mut panel.on, "Planet squares on");
            let t = &mut panel.tuning;
            ui.add(
                egui::Slider::new(&mut t.square_m, SQUARE_RANGE_M)
                    .logarithmic(true)
                    .suffix(" m")
                    .text("Square size"),
            );
            ui.horizontal(|ui| {
                colour_edit(ui, &mut t.colour_a);
                ui.label("Colour A");
            });
            ui.horizontal(|ui| {
                colour_edit(ui, &mut t.colour_b);
                ui.label("Colour B");
            });
            ui.add(
                egui::Slider::new(&mut t.sharp_px, SHARP_RANGE_PX)
                    .suffix(" px")
                    .text("Fade: sharp"),
            );
            ui.add(
                egui::Slider::new(&mut t.gray_px, EVEN_RANGE_PX)
                    .suffix(" px")
                    .text("Fade: even"),
            );
            ui.horizontal(|ui| {
                if ui.button("Reset").clicked() {
                    panel.reset();
                }
                if ui.button("Copy values").clicked() {
                    note = Some(match clipboard::copy(&panel.copy_text()) {
                        Ok(()) => "Values copied".to_string(),
                        Err(why) => why,
                    });
                }
            });
        }
    }
    ui.separator();
    debug_view_ui(ui, &mut panel.tuning);
    panel.clamp();
    note
}

/// The Debug view choice, with the distance range under Distance.
fn debug_view_ui(ui: &mut egui::Ui, tuning: &mut SquaresTuning) {
    egui::ComboBox::from_label("Debug view")
        .selected_text(tuning.debug.label())
        .show_ui(ui, |ui| {
            for view in DebugView::ALL {
                ui.selectable_value(&mut tuning.debug, view, view.label());
            }
        });
    if tuning.debug == DebugView::Distance {
        let [near, far] = &mut tuning.distance_m;
        ui.horizontal(|ui| {
            ui.label("From");
            ui.add(
                egui::DragValue::new(near)
                    .range(DISTANCE_RANGE_M)
                    .suffix(" m"),
            );
            ui.label("to");
            ui.add(
                egui::DragValue::new(far)
                    .range(DISTANCE_RANGE_M)
                    .suffix(" m"),
            );
        });
    }
}

/// An egui colour picker over a 0..1 linear RGB colour.
fn colour_edit(ui: &mut egui::Ui, rgb: &mut [f64; 3]) {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: a 0..1 colour channel narrowed for egui"
    )]
    let mut narrow = rgb.map(|c| c as f32);
    if egui::color_picker::color_edit_button_rgb(ui, &mut narrow).changed() {
        *rgb = narrow.map(f64::from);
    }
}

/// Applies the seed and path fields (Enter or Regenerate): a new seed
/// rebuilds its universe on its home star; the same seed jumps to the
/// path. A refused value keeps the planet and shows its message.
fn regenerate(lab: &mut Lab) {
    match lab.seed_text.trim().parse::<u64>() {
        Ok(seed) if seed != lab.scene.seed() => {
            lab.scene.set_seed(seed);
        }
        Ok(_) => {
            let path = lab.path_text.clone();
            lab.scene.set_path_text(&path);
        }
        Err(_) => {
            let message = format!("\"{}\" is not a seed number", lab.seed_text.trim());
            lab.scene.refuse(message);
            return;
        }
    }
    if lab.scene.error().is_none() {
        lab.rebuild = true;
    }
}
