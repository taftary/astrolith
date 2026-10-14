//! The lab's editor panels in egui (#640, AC1, AC6): the menu bar, the
//! nodes tree, the tools and options panel, the status bar and the stats
//! overlay over the viewport.

use crate::window::Lab;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use planet_lab::layout::{ABOUT, MENUS};
use planet_lab::nodes::{Node, NodeContext, SurfaceFigures, node_rows, tree};
use planet_lab::orbit::{CameraMode, FOV, Preset};
use planet_lab::scene::moons;
use planet_lab::stats::{SceneReadout, overlay_lines};
use planet_lab::units::{grid_spacing_m, metres_per_unit};
use universe_render::{SurfaceStats, Universe};

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
    let radius_earth = universe
        .open
        .points
        .iter()
        .find_map(|point| point.surface)
        .map_or(1.0, |surface| surface.radius_earth);
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
        moons: moons(universe)
            .iter()
            .filter_map(|point| point.moon)
            .map(|moon| (moon.radius_km, moon.orbit_km, moon.period_days))
            .collect(),
        camera_mode: lab.mode,
        camera: lab.pose().offset,
        fov: FOV,
        light: LIGHT,
        grid_m: grid_spacing_m(radius_m),
        axes_m: radius_m * 1.6,
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
        "planet-lab".into(),
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
            if let Some(shot) = &lab.last_shot {
                ui.separator();
                ui.label(format!("saved {shot}"));
            }
        });
    });
    let cx = node_context(lab, &universe, &stats);
    // Nodes tree.
    egui::Panel::left("nodes").show(&mut root, |ui| {
        ui.heading("Nodes");
        for node in tree(cx.moons.len()) {
            let indent = matches!(node, Node::Moon(_));
            ui.horizontal(|ui| {
                if indent {
                    ui.add_space(16.0);
                }
                if ui
                    .selectable_label(lab.node == node, node.label())
                    .clicked()
                {
                    lab.node = node;
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
        });
        ui.label(lab.scene.label());
        ui.separator();
        ui.heading(lab.node.label());
        egui::Grid::new("node values").striped(true).show(ui, |ui| {
            for (label, value) in node_rows(lab.node, &cx) {
                ui.label(label);
                ui.label(value);
                ui.end_row();
            }
        });
        match lab.node {
            Node::Planet => {
                ui.checkbox(&mut lab.toggles.wireframe, "Wireframe");
                ui.checkbox(&mut lab.toggles.normals, "Normals");
                ui.checkbox(&mut lab.toggles.lod_colors, "LOD depth view");
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
                if ui.checkbox(&mut free, "Free-fly (F)").changed() {
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
            Node::Moon(_) | Node::Light => {}
        }
    });
    // Stats overlay over the viewport.
    let readout = SceneReadout {
        triangles: stats.faces,
        meshes: meshes.iter().count(),
        position_m: lab
            .pose()
            .offset
            .map(|unit| unit * planet_lab::units::scene_metres(&universe)),
        height_m: universe.path.height_above_ground_m().unwrap_or(f64::NAN),
        shell: universe.shown_level().get(),
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
    let toggle = |ui: &mut egui::Ui, value: &mut bool| {
        ui.checkbox(value, item);
    };
    match (menu, item) {
        ("View", "Wireframe") => toggle(ui, &mut lab.toggles.wireframe),
        ("View", "Normals") => toggle(ui, &mut lab.toggles.normals),
        ("View", "LOD depth view") => toggle(ui, &mut lab.toggles.lod_colors),
        ("View", "Stats overlay") => toggle(ui, &mut lab.toggles.stats),
        ("View", "Grid") => toggle(ui, &mut lab.toggles.grid),
        ("View", "Axes") => toggle(ui, &mut lab.toggles.axes),
        _ => {
            if ui.button(item).clicked() {
                match item {
                    "Screenshot (F12)" => lab.screenshot = true,
                    "Quit" => {
                        exit.write(AppExit::Success);
                    }
                    "Orbit / Free-fly (F)" => lab.toggle_mode(),
                    "Home" => lab.home(),
                    "About" => lab.about = true,
                    _ => {}
                }
                ui.close();
            }
        }
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
