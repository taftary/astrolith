//! Engine lab window (#640, ADR 0029): a PC-only development program.
//!
//! `engine-lab` opens the editor window on the game's home planet;
//! `engine-lab --screens <dir>` takes the listed pictures and exits.

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
compile_error!("Engine lab is a desktop program: Windows, Linux or macOS only (#640, Q3).");

mod screens;
mod ui;
mod window;

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{Backends, WgpuSettings};
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use engine_lab::scene::LabScene;
use universe_render::{PlanetSquaresPlugin, PlanetViewPlugin, PlanetViewSystems};

/// Exit code for a wrong command line (the game's protocol, ADR 0021).
const EXIT_USAGE: i32 = 2;

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

/// The lab app, with the `--screens` sequencer when `screens_dir` is set.
fn lab_app(screens_dir: Option<String>) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: engine_lab::TITLE.to_string(),
                    resolution: (1600, 900).into(),
                    ..default()
                }),
                ..default()
            })
            .set(render_plugin()),
    )
    .add_plugins((EguiPlugin::default(), PlanetSquaresPlugin))
    .insert_resource(window::Lab::new(LabScene::default_home()))
    .add_plugins(PlanetViewPlugin)
    .add_systems(Startup, spawn_scene)
    .add_systems(
        Update,
        (window::lab_input, window::pick_click, window::apply_lab)
            .chain()
            .before(PlanetViewSystems::Pose),
    )
    .add_systems(
        Update,
        (
            window::draw_grid_and_axes,
            window::draw_selection,
            window::take_screenshot,
        )
            .after(PlanetViewSystems::Draw),
    )
    .add_systems(EguiPrimaryContextPass, ui::lab_ui);
    if let Some(dir) = screens_dir {
        app.add_plugins(screens::LabScreensPlugin { dir });
    }
    app
}

/// The viewport camera and the light. ACES Fitted needs no LUT feature
/// (the game's tonemapping, #157); Bevy's default would ask for one.
fn spawn_scene(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Tonemapping::AcesFitted,
        Name::new("lab-camera"),
    ));
    let ([x, y, z], lux) = ui::LIGHT;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: a unit direction and a lux value narrowed for Bevy"
    )]
    let (direction, illuminance) = (Vec3::new(x as f32, y as f32, z as f32), lux as f32);
    commands.spawn((
        DirectionalLight {
            illuminance,
            ..default()
        },
        Transform::from_translation(Vec3::ZERO).looking_to(direction, Vec3::Z),
        Name::new("lab-light"),
    ));
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let screens_dir = match engine_lab::shots::parse_args(&args) {
        Some(engine_lab::shots::Launch::Window) => None,
        Some(engine_lab::shots::Launch::Screens(dir)) => Some(dir),
        None => {
            eprintln!("{}", engine_lab::shots::USAGE);
            std::process::exit(EXIT_USAGE);
        }
    };
    if let Some(dir) = &screens_dir
        && !std::path::Path::new(dir).is_dir()
    {
        eprintln!("engine-lab --screens: no such folder: {dir}");
        std::process::exit(EXIT_USAGE);
    }
    lab_app(screens_dir).run();
}
