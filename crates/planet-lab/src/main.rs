//! Planet lab window (#640, ADR 0029): a PC-only development program.

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
compile_error!("Planet lab is a desktop program: Windows, Linux or macOS only (#640, Q3).");

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{Backends, WgpuSettings};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};

/// Render plugin with Vulkan on Windows (ADR 0029): on the dev PC's Intel
/// GPU the egui pass loses the DirectX 12 device within seconds, while the
/// game (no egui) keeps DirectX 12 (ADR 0022). Vulkan logs the known
/// loader and validation lines there (#499). The `WGPU_BACKEND`
/// environment variable wins; other platforms keep Bevy's default.
fn render_plugin() -> RenderPlugin {
    let mut settings = WgpuSettings::default();
    settings.backends = Backends::from_env().or(if cfg!(windows) {
        Some(Backends::VULKAN)
    } else {
        settings.backends
    });
    RenderPlugin {
        render_creation: settings.into(),
        ..Default::default()
    }
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: planet_lab::TITLE.to_string(),
                        ..default()
                    }),
                    ..default()
                })
                .set(render_plugin()),
        )
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, spawn_camera)
        .add_systems(EguiPrimaryContextPass, menu_bar)
        .run();
}

/// The viewport camera. ACES Fitted needs no LUT feature (the game's
/// tonemapping, #157); Bevy's default would ask for one.
fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Tonemapping::AcesFitted,
        Name::new("lab-camera"),
    ));
}

/// The game's bundled Inter font (ADR 0023), egui's only font here.
const INTER_REGULAR: &[u8] = include_bytes!("../../universe-render/assets/fonts/Inter-Regular.ttf");

/// The menu bar (File, View, Camera, Help); filled in by later tasks.
fn menu_bar(mut contexts: EguiContexts, mut fonts_set: Local<bool>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if !*fonts_set {
        *fonts_set = true;
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
    // Panels lay out inside one root UI covering the window.
    let mut root = egui::Ui::new(
        ctx.clone(),
        "planet-lab".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    egui::Panel::top("menu").show(&mut root, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            for menu in ["File", "View", "Camera", "Help"] {
                ui.menu_button(menu, |_| {});
            }
        });
    });
    Ok(())
}
