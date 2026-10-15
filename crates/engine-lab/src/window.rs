//! The lab's window state, cameras, grid, axes and screenshot (#640).
//!
//! [`Lab`] holds what the panels edit (scene, selected node, view toggles,
//! camera). Each frame [`apply_lab`] turns it into the game's planet view
//! ([`PlanetViewPose`], [`SurfaceDebug`], the [`Universe`]); [`lab_input`]
//! reads the keys and the mouse; [`draw_grid_and_axes`] draws the grid on
//! the equator and the X/Y/Z axes at the planet centre.

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::render::error_handler::{ErrorType, RenderError, RenderErrorPolicy};
use bevy::render::view::window::screenshot::{Screenshot, save_to_disk};
use engine_lab::code::GAME_CODE;
use engine_lab::keys::{KEYS, LabAction, action_for};
use engine_lab::library::ShaderLibrary;
use engine_lab::nodes::{Node, ViewToggles};
use engine_lab::orbit::{CameraMode, OrbitCamera, Preset, free_fly_step, look_direction};
use engine_lab::scene::LabScene;
use engine_lab::select::{OUTLINE_RGB, PanelRects, Selection, outline, pick, pickables, tolerance};
use engine_lab::stats::FrameTimes;
use universe_render::{
    PLANET_SQUARES_SHADER_PATH, PlanetViewPose, SettingsStore, SquaresTuning, SurfaceDebug,
    Universe,
};

/// Mouse orbit speed, radians per pixel.
const ORBIT_PER_PX: f64 = 0.005;
/// Pan speed, distance shares per pixel.
const PAN_PER_PX: f64 = 0.0015;
/// Zoom factor per wheel line (in).
const ZOOM_PER_LINE: f64 = 0.85;
/// Free-fly look speed, radians per pixel.
const LOOK_PER_PX: f64 = 0.003;

/// Everything the lab's panels edit.
#[derive(Resource, Debug, Clone)]
pub(crate) struct Lab {
    /// The planet on show.
    pub(crate) scene: LabScene,
    /// The selection shared by the Nodes list and the 3D view (#654 D2).
    pub(crate) selection: Selection,
    /// Mouse travel since the left button went down, pixels (a click is
    /// a press and release that barely moves).
    pub(crate) press_travel: f32,
    /// View switches.
    pub(crate) toggles: ViewToggles,
    /// Which camera drives.
    pub(crate) mode: CameraMode,
    /// The orbit camera.
    pub(crate) orbit: OrbitCamera,
    /// Free-fly camera position, cell units.
    pub(crate) free_camera: [f64; 3],
    /// Free-fly yaw, radians.
    pub(crate) free_yaw: f64,
    /// Free-fly pitch, radians.
    pub(crate) free_pitch: f64,
    /// Free-fly speed step (Shift+1-9).
    pub(crate) speed: u32,
    /// The seed field's text.
    pub(crate) seed_text: String,
    /// The path field's text.
    pub(crate) path_text: String,
    /// Frame times for the stats overlay.
    pub(crate) times: FrameTimes,
    /// Last saved screenshot, for the status bar.
    pub(crate) last_shot: Option<String>,
    /// The planet's moons (radius km, orbit km, period days), read once
    /// per rebuild: the moon on show keeps its own rows (#654 D4).
    pub(crate) moons: Vec<(f64, f64, f64)>,
    /// The generator recipe of the planet on show, read once per rebuild
    /// for the Planet rows (T13).
    pub(crate) recipe: Option<universe_core::world::biome::RecipeSummary>,
    /// The About window is open.
    pub(crate) about: bool,
    /// The scene changed: rebuild the universe.
    pub(crate) rebuild: bool,
    /// A screenshot is asked for.
    pub(crate) screenshot: bool,
    /// The shader library: Planet squares plus created shaders (#667).
    /// Selecting an entry shows it on the body on show; the game never
    /// changes. Lab-only, reset on restart.
    pub(crate) library: ShaderLibrary,
    /// Screen rectangles the editor panels covered last frame, in egui
    /// points: with egui 0.36 `Panel`s are child `Ui`s, not `Area`s, so
    /// `is_pointer_over_egui()` stays false over them (#703).
    pub(crate) panels: PanelRects,
    /// The gallery shots draw the Generated look (T13, #685); every other
    /// shot draws Normal (Squares shots drive their own look).
    pub(crate) generated: bool,
    /// The last Copy values note, for the status bar.
    pub(crate) note: Option<String>,
}

impl Lab {
    /// The lab on the game's home planet, Home view.
    #[must_use]
    pub(crate) fn new(scene: LabScene) -> Lab {
        let seed_text = scene.seed().to_string();
        let path_text = scene.path_text();
        Lab {
            scene,
            selection: Selection::of(Node::Planet),
            press_travel: 0.0,
            toggles: ViewToggles::default(),
            mode: CameraMode::Orbit,
            orbit: OrbitCamera::home(),
            free_camera: OrbitCamera::home().position(),
            free_yaw: 0.0,
            free_pitch: 0.0,
            speed: 5,
            seed_text,
            path_text,
            times: FrameTimes::default(),
            last_shot: None,
            moons: Vec::new(),
            recipe: None,
            about: false,
            rebuild: true,
            screenshot: false,
            library: ShaderLibrary::default(),
            panels: PanelRects::default(),
            generated: false,
            note: None,
        }
    }

    /// Camera position, forward and up for the current mode.
    #[must_use]
    pub(crate) fn pose(&self) -> PlanetViewPose {
        let (offset, forward) = match self.mode {
            CameraMode::Orbit => (self.orbit.position(), self.orbit.forward()),
            CameraMode::Free => (
                self.free_camera,
                look_direction(self.free_yaw, self.free_pitch),
            ),
        };
        PlanetViewPose {
            offset,
            forward,
            up: up_for(forward),
        }
    }

    /// Switches orbit and free-fly (G), keeping the view where it is.
    pub(crate) fn toggle_mode(&mut self) {
        match self.mode {
            CameraMode::Orbit => {
                self.free_camera = self.orbit.position();
                self.free_yaw = self.orbit.yaw;
                self.free_pitch = self.orbit.pitch;
                self.mode = CameraMode::Free;
            }
            CameraMode::Free => {
                let [x, y, z] = self.free_camera;
                self.orbit.target = [0.0; 3];
                self.orbit.distance = (x * x + y * y + z * z).sqrt().max(1e-9);
                // Looking at the centre from here: forward = -position.
                self.orbit.pitch = (-y / self.orbit.distance).clamp(-1.0, 1.0).asin();
                self.orbit.yaw = x.atan2(z);
                self.mode = CameraMode::Orbit;
            }
        }
    }

    /// Home (and Numpad 0 in free-fly): back to the orbit Home view of
    /// the planet, or of the whole cell above it.
    pub(crate) fn home(&mut self) {
        self.mode = CameraMode::Orbit;
        self.orbit = if self.scene.on_planet() {
            OrbitCamera::home()
        } else {
            OrbitCamera::home_cell()
        };
    }

    /// Parent (Page Up): one level up, Home view there (#654 D5).
    pub(crate) fn parent(&mut self) {
        match self.scene.parent() {
            Ok(()) => self.moved(),
            Err(message) => self.scene.refuse(message),
        }
    }

    /// F or "Show selected": the selected moon on its own, or the
    /// selected object opened (#654 D3-D5).
    pub(crate) fn show_selected(&mut self) {
        match self.scene.show_selected(self.selection.node()) {
            Ok(()) => {
                let moon = self.scene.moon_on_show().map(Node::Moon);
                self.moved();
                if let Some(node) = moon {
                    self.selection = Selection::of(node);
                }
            }
            Err(message) => self.scene.refuse(message),
        }
    }

    /// Child (Page Down): opens the selected object, Home view there.
    pub(crate) fn child(&mut self) {
        let result = match self.selection.node() {
            Some(Node::Marker(marker)) => self.scene.child(marker),
            _ => Err("select an object of this level first".to_string()),
        };
        match result {
            Ok(()) => self.moved(),
            Err(message) => self.scene.refuse(message),
        }
    }

    /// The view moved to another cell: rebuild, select the planet (or
    /// nothing above it), frame it.
    fn moved(&mut self) {
        self.selection = if self.scene.on_planet() {
            Selection::of(Node::Planet)
        } else {
            Selection::default()
        };
        self.rebuild = true;
        self.home();
    }

    /// A preset view, in orbit mode.
    pub(crate) fn preset(&mut self, preset: Preset) {
        self.mode = CameraMode::Orbit;
        self.orbit.preset(preset);
    }
}

/// An up vector perpendicular to `forward`, leaning toward `+Y` (or `-Z`
/// when looking straight up or down).
#[must_use]
pub(crate) fn up_for(forward: [f64; 3]) -> [f64; 3] {
    let reference = if forward[1].abs() > 0.999 {
        [0.0, 0.0, -forward[1].signum()]
    } else {
        [0.0, 1.0, 0.0]
    };
    let along = forward[0] * reference[0] + forward[1] * reference[1] + forward[2] * reference[2];
    let up = [
        reference[0] - forward[0] * along,
        reference[1] - forward[1] * along,
        reference[2] - forward[2] * along,
    ];
    let length = (up[0] * up[0] + up[1] * up[1] + up[2] * up[2])
        .sqrt()
        .max(1e-300);
    [up[0] / length, up[1] / length, up[2] / length]
}

/// Turns the lab into the game's planet view each frame.
pub(crate) fn apply_lab(
    mut lab: ResMut<Lab>,
    mut universe: ResMut<Universe>,
    mut pose: ResMut<PlanetViewPose>,
    mut debug: ResMut<SurfaceDebug>,
    (mut store, mut tuning): (ResMut<SettingsStore>, ResMut<SquaresTuning>),
    time: Res<Time>,
) {
    lab.times.push(f64::from(time.delta_secs()));
    if lab.rebuild {
        lab.rebuild = false;
        universe.0 = lab.scene.universe();
        lab.moons = lab
            .scene
            .moon_infos()
            .iter()
            .map(|moon| (moon.radius_km, moon.orbit_km, moon.period_days))
            .collect();
        lab.recipe = universe
            .frame
            .as_ref()
            .and_then(universe_core::world::biome::recipe_summary);
        lab.seed_text = lab.scene.seed().to_string();
        lab.path_text = lab.scene.path_text();
    }
    let wanted = lab.pose();
    if *pose != wanted {
        *pose = wanted;
    }
    // The planet's views apply on a planet only (#654 D7).
    let toggles = lab.toggles;
    let planet = lab.scene.on_planet();
    let next = SurfaceDebug {
        wireframe: planet && toggles.wireframe,
        normals: planet && toggles.normals,
        lod_colors: planet && toggles.lod_colors,
    };
    if *debug != next {
        *debug = next;
    }
    // The library's shown entry drives the game's squares on the body on
    // show; the lab's settings store is in memory only (#665 Q4, #667).
    let (on, wanted, _) = lab.library.applied(planet);
    // The squares look is on or off; the gallery shots draw Generated, and
    // anything else draws Normal (#685 T13).
    let squares = universe_core::look::PlanetLook::Squares;
    let generated = universe_core::look::PlanetLook::Generated;
    let normal = universe_core::look::PlanetLook::Normal;
    if on && store.0.planet_look != squares {
        store.0.planet_look = squares;
    } else if !on && store.0.planet_look == squares {
        store.0.planet_look = normal;
    }
    if !on {
        if lab.generated && store.0.planet_look != generated {
            store.0.planet_look = generated;
        } else if !lab.generated && store.0.planet_look == generated {
            store.0.planet_look = normal;
        }
    }
    if *tuning != wanted {
        *tuning = wanted;
    }
}

/// Mirrors the library's shown entry into the in-memory squares shader
/// asset (#666, #667): the hot-reload path, so Bevy rebuilds the
/// pipelines and the body on show redraws. Lab-only; the game keeps the
/// embedded default, and the file on disk is never written. Save
/// publishes only validated text, so the asset never receives a parse
/// error; above the planet the game text stays (L1-L9 have no surface).
pub(crate) fn apply_shader_code(
    lab: Res<Lab>,
    asset_server: Res<AssetServer>,
    mut shaders: ResMut<Assets<Shader>>,
    mut guard: ResMut<ShaderSwapGuard>,
    mut handle: Local<Option<Handle<Shader>>>,
) {
    let wanted = lab.library.applied(lab.scene.on_planet()).2;
    if guard.committed.as_deref() == Some(wanted) {
        return;
    }
    if guard.committed.is_none() && wanted == GAME_CODE {
        // Startup: the asset already holds the embedded default.
        guard.committed = Some(wanted.to_string());
        return;
    }
    let handle = handle.get_or_insert_with(|| asset_server.load(PLANET_SQUARES_SHADER_PATH));
    if swap_shader_to(&mut shaders, handle, wanted) {
        guard.committed = Some(wanted.to_string());
        // A new swap answers for itself: later errors are fresh.
        guard.reverted_since_swap = false;
    }
}

/// Write `wanted` into the shader asset at `handle` (the hot-reload
/// path: [`Assets::get_mut`] emits `Modified`, so Bevy rebuilds the
/// pipelines). Returns whether the asset exists yet; the caller retries
/// next frame when it does not.
fn swap_shader_to(shaders: &mut Assets<Shader>, handle: &Handle<Shader>, wanted: &str) -> bool {
    if let Some(mut shader) = shaders.get_mut(handle) {
        *shader = Shader::from_wgsl(wanted.to_string(), PLANET_SQUARES_SHADER_PATH);
        true
    } else {
        false
    }
}

/// Whether the last shader swap is still awaiting its verdict (#666).
/// The apply system and the error handler share it: the system records
/// every swap, the handler records every revert, so a redundant swap
/// after a revert cannot disarm it. A failing pipeline compilation
/// queues several device errors (creation and submit time); after the
/// first reverts, the rest are stale echoes.
#[derive(Resource, Debug, Default)]
pub(crate) struct ShaderSwapGuard {
    /// Text the shader asset is believed to hold.
    pub(crate) committed: Option<String>,
    /// A revert answered the last swap; later echoes are ignored.
    pub(crate) reverted_since_swap: bool,
}

/// The lab's render-error handler (#666, T2 spike, #667 T2): Bevy quits on
/// any render validation error by default, but edited shader text the
/// syntax pre-check accepted can still fail pipeline creation (a wrong
/// bind group, a renamed entry point). While a library entry's custom text
/// is applied, a validation error reverts that entry to its start (the
/// game's code, or the blank shader), records Bevy's message on the error
/// line, and keeps rendering; stale echoes after a revert are ignored;
/// anything else keeps the default policy.
pub(crate) fn lab_error_handler(
    error: &RenderError,
    main_world: &mut World,
    _render_world: &mut World,
) -> RenderErrorPolicy {
    if error.ty == ErrorType::Validation
        && main_world
            .get_resource::<ShaderSwapGuard>()
            .is_some_and(|guard| guard.reverted_since_swap)
    {
        // Stale echo of the failure the last revert already answered.
        return RenderErrorPolicy::Ignore;
    }
    let custom = main_world
        .get_resource::<Lab>()
        .is_some_and(|lab| lab.library.selected_is_custom());
    if custom && error.ty == ErrorType::Validation {
        let fallback = main_world
            .get_resource::<Lab>()
            .map(|lab| ShaderLibrary::default_code(lab.library.selected()).to_string());
        if let Some(mut lab) = main_world.get_resource_mut::<Lab>() {
            lab.library.revert_selected(error.description.clone());
        }
        let handle: Option<Handle<Shader>> = main_world
            .get_resource::<AssetServer>()
            .map(|server| server.load(PLANET_SQUARES_SHADER_PATH));
        if let (Some(handle), Some(mut shaders)) =
            (handle, main_world.get_resource_mut::<Assets<Shader>>())
        {
            if let Some(fallback) = fallback.as_deref() {
                swap_shader_to(&mut shaders, &handle, fallback);
            }
        }
        if let Some(mut guard) = main_world.get_resource_mut::<ShaderSwapGuard>() {
            // The asset is back to the entry's start: no catch-up swap follows.
            guard.committed = fallback;
            guard.reverted_since_swap = true;
        }
        return RenderErrorPolicy::Ignore;
    }
    bevy::log::error!("Quitting the application due to {:?} RenderError", error.ty);
    main_world.write_message(AppExit::error());
    RenderErrorPolicy::StopRendering
}

/// Keys and mouse for the cameras, presets, F, Home and F12; ignored while
/// the pointer is over a panel or a text field has the keyboard.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn lab_input(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut motions: MessageReader<MouseMotion>,
    mut wheels: MessageReader<MouseWheel>,
    mut lab: ResMut<Lab>,
    universe: Res<Universe>,
    time: Res<Time>,
    mut contexts: bevy_egui::EguiContexts,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let (area_busy, keys_busy) = contexts.ctx_mut().map_or((false, false), |ctx| {
        (ctx.is_pointer_over_egui(), ctx.egui_wants_keyboard_input())
    });
    // Panels are child `Ui`s, invisible to `is_pointer_over_egui()` (#703).
    let cursor = windows
        .single()
        .ok()
        .and_then(|window| window.cursor_position());
    let pointer_busy = pointer_over_ui(&lab.panels, cursor, area_busy);
    let drag: Vec2 = motions.read().map(|motion| motion.delta).sum();
    let lines: f32 = wheels.read().map(|wheel| wheel.y).sum();
    if !keys_busy {
        let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
        let presets = [
            (KeyCode::Numpad7, Preset::Top, Preset::Bottom),
            (KeyCode::Numpad1, Preset::Front, Preset::Back),
            (KeyCode::Numpad3, Preset::Right, Preset::Left),
        ];
        for (key, plain, with_ctrl) in presets {
            if keys.just_pressed(key) {
                lab.preset(if ctrl { with_ctrl } else { plain });
            }
        }
        if keys.just_pressed(KeyCode::Numpad0) {
            lab.preset(Preset::Perspective);
        }
        for key in KEYS {
            if !keys.just_pressed(key) {
                continue;
            }
            match action_for(key) {
                Some(LabAction::ShowSelected) => lab.show_selected(),
                Some(LabAction::ToggleFreeFly) => lab.toggle_mode(),
                Some(LabAction::Parent) => lab.parent(),
                Some(LabAction::Child) => lab.child(),
                Some(LabAction::Home) => lab.home(),
                Some(LabAction::Screenshot) => lab.screenshot = true,
                None => {}
            }
        }
    }
    match lab.mode {
        CameraMode::Orbit => {
            if !pointer_busy {
                if buttons.pressed(MouseButton::Left) {
                    lab.orbit.orbit(
                        -f64::from(drag.x) * ORBIT_PER_PX,
                        -f64::from(drag.y) * ORBIT_PER_PX,
                    );
                }
                if buttons.pressed(MouseButton::Middle) {
                    lab.orbit.pan(
                        -f64::from(drag.x) * PAN_PER_PX,
                        f64::from(drag.y) * PAN_PER_PX,
                    );
                }
                if lines != 0.0 {
                    lab.orbit.zoom(ZOOM_PER_LINE.powf(f64::from(lines)));
                }
            }
        }
        CameraMode::Free => {
            if !pointer_busy && buttons.pressed(MouseButton::Right) {
                lab.free_yaw -= f64::from(drag.x) * LOOK_PER_PX;
                lab.free_pitch =
                    (lab.free_pitch - f64::from(drag.y) * LOOK_PER_PX).clamp(-1.55, 1.55);
            }
            if !keys_busy {
                let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
                let digits = [
                    KeyCode::Digit1,
                    KeyCode::Digit2,
                    KeyCode::Digit3,
                    KeyCode::Digit4,
                    KeyCode::Digit5,
                    KeyCode::Digit6,
                    KeyCode::Digit7,
                    KeyCode::Digit8,
                    KeyCode::Digit9,
                ];
                for (step, key) in (1u32..).zip(digits) {
                    if shift && keys.just_pressed(key) {
                        lab.speed = step;
                    }
                }
                let held = [
                    keys.pressed(KeyCode::KeyW),
                    keys.pressed(KeyCode::KeyS),
                    keys.pressed(KeyCode::KeyA),
                    keys.pressed(KeyCode::KeyD),
                ];
                let height = universe
                    .path
                    .height_above_ground_m()
                    .zip(universe.path.planet_metres_per_unit())
                    .map_or(1.0, |(metres, per_unit)| metres / per_unit);
                let next = free_fly_step(
                    lab.free_camera,
                    lab.free_yaw,
                    lab.free_pitch,
                    held,
                    lab.speed,
                    height,
                    f64::from(time.delta_secs()),
                );
                lab.free_camera = next;
            }
        }
    }
}

/// Saves a screenshot when asked (F12, File > Screenshot): a PNG of the
/// window in `screenshots/`, named with the date and time (AC10).
pub(crate) fn take_screenshot(mut commands: Commands, mut lab: ResMut<Lab>) {
    if !lab.screenshot {
        return;
    }
    lab.screenshot = false;
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let path = engine_lab::shots::screenshot_path(secs);
    if std::fs::create_dir_all(engine_lab::shots::SCREENSHOT_DIR).is_err() {
        lab.last_shot = Some(format!(
            "cannot create {}",
            engine_lab::shots::SCREENSHOT_DIR
        ));
        return;
    }
    commands
        .spawn((Name::new("lab-screenshot"), Screenshot::primary_window()))
        .observe(save_to_disk(path.clone()));
    lab.last_shot = Some(path);
}

/// Grid tint (faint) and axis tints (X red, Y green, Z blue).
const GRID_TINT: Color = Color::srgba(0.5, 0.5, 0.55, 0.35);

/// Draws the grid on the planet's equator plane and the X/Y/Z axes at
/// its centre, relative to the camera (camera-relative drawing, ADR 0026).
pub(crate) fn draw_grid_and_axes(
    mut gizmos: Gizmos,
    lab: Res<Lab>,
    pose: Res<PlanetViewPose>,
    universe: Res<Universe>,
) {
    let at = |p: [f64; 3]| {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: camera-relative point narrowed for the GPU, intended"
        )]
        let v = Vec3::new(
            (p[0] - pose.offset[0]) as f32,
            (p[1] - pose.offset[1]) as f32,
            (p[2] - pose.offset[2]) as f32,
        );
        v
    };
    let radius = universe_core::terrain::PLANET_RADIUS_CELL;
    if lab.toggles.grid {
        let per_unit = universe.path.planet_metres_per_unit().unwrap_or(1.0);
        let spacing = engine_lab::units::grid_spacing_m(radius * per_unit) / per_unit;
        let reach = radius * 2.0;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: a small line count"
        )]
        let count = (reach / spacing).ceil() as i32;
        // The centre lines are left to the X and Z axes, so they read.
        for i in (-count..=count).filter(|&i| i != 0 || !lab.toggles.axes) {
            let d = f64::from(i) * spacing;
            gizmos.line(at([d, 0.0, -reach]), at([d, 0.0, reach]), GRID_TINT);
            gizmos.line(at([-reach, 0.0, d]), at([reach, 0.0, d]), GRID_TINT);
        }
    }
    if lab.toggles.axes {
        let length = radius * 1.6;
        for (axis, tint) in [
            ([length, 0.0, 0.0], Color::srgb(0.9, 0.25, 0.25)),
            ([0.0, length, 0.0], Color::srgb(0.3, 0.85, 0.3)),
            ([0.0, 0.0, length], Color::srgb(0.3, 0.45, 0.95)),
        ] {
            gizmos.line(at([0.0; 3]), at(axis), tint);
        }
    }
}

/// A press and release that moves less than this many pixels is a click.
const CLICK_PX: f32 = 4.0;

/// Whether the pointer is over the editor UI: a recorded panel rectangle
/// or a real egui area (popups, the Stats and About windows). Panels are
/// child `Ui`s, not `Area`s, so `is_pointer_over_egui()` stays false over
/// them and the recorded rectangles carry the check (#703).
fn pointer_over_ui(panels: &PanelRects, cursor: Option<Vec2>, over_area: bool) -> bool {
    over_area || cursor.is_some_and(|at| panels.covers([at.x, at.y]))
}

/// Picks the object under a left click in the 3D view (#654 D2): an
/// object selects it, empty space clears the selection. A drag (orbit) is
/// not a click; clicks on a panel belong to the panel.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
pub(crate) fn pick_click(
    buttons: Res<ButtonInput<MouseButton>>,
    mut motions: MessageReader<MouseMotion>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut lab: ResMut<Lab>,
    universe: Res<Universe>,
    pose: Res<PlanetViewPose>,
    mut contexts: bevy_egui::EguiContexts,
) {
    let travel: f32 = motions.read().map(|motion| motion.delta.length()).sum();
    if buttons.just_pressed(MouseButton::Left) {
        lab.press_travel = 0.0;
    } else if buttons.pressed(MouseButton::Left) {
        lab.press_travel += travel;
    }
    if !buttons.just_released(MouseButton::Left) || lab.press_travel >= CLICK_PX {
        return;
    }
    let (Ok(window), Ok((camera, transform))) = (windows.single(), cameras.single()) else {
        return;
    };
    let cursor = window.cursor_position();
    let over_area = contexts
        .ctx_mut()
        .is_ok_and(|ctx| ctx.is_pointer_over_egui());
    if pointer_over_ui(&lab.panels, cursor, over_area) {
        return;
    }
    let Some(ray) = cursor.and_then(|at| camera.viewport_to_world(transform, at).ok()) else {
        return;
    };
    let origin = [
        pose.offset[0] + f64::from(ray.origin.x),
        pose.offset[1] + f64::from(ray.origin.y),
        pose.offset[2] + f64::from(ray.origin.z),
    ];
    let direction = [
        f64::from(ray.direction.x),
        f64::from(ray.direction.y),
        f64::from(ray.direction.z),
    ];
    let tol = tolerance(engine_lab::orbit::FOV, f64::from(window.height()));
    let objects = pickables(&lab.scene, &universe);
    let picked = pick(&objects, origin, direction, tol);
    lab.selection.click_view(picked);
}

/// Draws the orange outline around the selected object: a camera-facing
/// circle just outside it (indicators only, `E-RENDER-NO-MESH`).
pub(crate) fn draw_selection(
    mut gizmos: Gizmos,
    lab: Res<Lab>,
    universe: Res<Universe>,
    pose: Res<PlanetViewPose>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let height = windows
        .single()
        .map_or(900.0, |window| f64::from(window.height()));
    let tol = tolerance(engine_lab::orbit::FOV, height);
    let objects = pickables(&lab.scene, &universe);
    let Some((centre, radius)) = outline(&objects, lab.selection, pose.offset, tol) else {
        return;
    };
    let to = [
        centre[0] - pose.offset[0],
        centre[1] - pose.offset[1],
        centre[2] - pose.offset[2],
    ];
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: camera-relative point and radius narrowed for the GPU, intended"
    )]
    let (at, size) = (
        Vec3::new(to[0] as f32, to[1] as f32, to[2] as f32),
        radius as f32,
    );
    let facing = at.try_normalize().unwrap_or(Vec3::Z);
    let rotation = Quat::from_rotation_arc(Vec3::Z, facing);
    let [r, g, b] = OUTLINE_RGB;
    gizmos.circle(Isometry3d::new(at, rotation), size, Color::srgb(r, g, b));
    gizmos.circle(
        Isometry3d::new(at, rotation),
        size * 1.04,
        Color::srgb(r, g, b),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_switches_keep_the_view() {
        let mut lab = Lab::new(LabScene::default_home());
        let before = lab.pose();
        lab.toggle_mode();
        assert_eq!(lab.mode, CameraMode::Free);
        let free = lab.pose();
        for axis in 0..3 {
            assert!((free.offset[axis] - before.offset[axis]).abs() < 1e-12);
            assert!((free.forward[axis] - before.forward[axis]).abs() < 1e-9);
        }
        lab.toggle_mode();
        let back = lab.pose();
        for axis in 0..3 {
            assert!((back.offset[axis] - before.offset[axis]).abs() < 1e-9);
        }
        let up = up_for([0.0, -1.0, 0.0]);
        assert!(up[1].abs() < 1e-12, "a defined up looking straight down");
    }

    #[test]
    fn shader_failure_reverts_to_the_game_code() {
        // T2 of #666: a render validation error while edited text is
        // applied reverts the state to the game's code, records Bevy's
        // message, and keeps rendering. No window, no GPU.
        let mut main_world = World::new();
        main_world.init_resource::<Assets<Shader>>();
        main_world.init_resource::<ShaderSwapGuard>();
        let mut lab = Lab::new(LabScene::default_home());
        {
            let game = lab
                .library
                .selected_entry_mut()
                .expect("Planet squares selected");
            game.code.edited = GAME_CODE.replace("* 0.5;", "* 0.0;");
            game.code.save();
        }
        main_world.insert_resource(lab);
        let mut render_world = World::new();
        let error = RenderError {
            ty: ErrorType::Validation,
            description: "binding missing".to_string(),
            source: None,
        };
        let policy = lab_error_handler(&error, &mut main_world, &mut render_world);
        assert!(matches!(policy, RenderErrorPolicy::Ignore));
        let lab = main_world.get_resource::<Lab>().expect("the lab resource");
        let game = lab.library.selected_entry().expect("Planet squares shown");
        assert_eq!(game.code.applied, GAME_CODE);
        assert_eq!(game.code.error.as_deref(), Some("binding missing"));
        let guard = main_world
            .get_resource::<ShaderSwapGuard>()
            .expect("the swap guard");
        assert_eq!(
            guard.committed.as_deref(),
            Some(GAME_CODE),
            "no catch-up swap follows the revert"
        );
        assert!(guard.reverted_since_swap);
    }

    #[test]
    fn shader_failure_reverts_a_created_entry_to_blank() {
        // T2 of #667: a validation error while a created shader's custom
        // text is applied reverts that entry to its blank start (not the
        // game's code), records Bevy's message, and keeps rendering.
        use engine_lab::library::BLANK_CODE;
        let mut main_world = World::new();
        main_world.init_resource::<Assets<Shader>>();
        main_world.init_resource::<ShaderSwapGuard>();
        let mut lab = Lab::new(LabScene::default_home());
        let created = lab.library.new_shader();
        {
            let shown = lab
                .library
                .selected_entry_mut()
                .expect("the new shader selected");
            shown.code.edited = BLANK_CODE.replace("base_color", "base_color_");
            // Bypass Save's parse (this text is broken): publish directly,
            // as a Bevy-level failure the pre-check accepted would.
            shown.code.applied.clone_from(&shown.code.edited);
        }
        main_world.insert_resource(lab);
        let mut render_world = World::new();
        let error = RenderError {
            ty: ErrorType::Validation,
            description: "binding missing".to_string(),
            source: None,
        };
        let policy = lab_error_handler(&error, &mut main_world, &mut render_world);
        assert!(matches!(policy, RenderErrorPolicy::Ignore));
        let lab = main_world.get_resource::<Lab>().expect("the lab resource");
        assert_eq!(lab.library.selected(), created);
        let shown = lab.library.selected_entry().expect("the new shader");
        assert_eq!(shown.code.applied, BLANK_CODE);
        assert_eq!(shown.code.error.as_deref(), Some("binding missing"));
        let guard = main_world
            .get_resource::<ShaderSwapGuard>()
            .expect("the swap guard");
        assert_eq!(guard.committed.as_deref(), Some(BLANK_CODE));
        assert!(guard.reverted_since_swap);
    }

    #[test]
    fn stale_error_echoes_after_a_revert_are_ignored() {
        // T2 of #666: a failing pipeline queues several device errors;
        // after the first reverts, the echoes are ignored instead of
        // quitting, even though the state is back to the game code.
        let mut main_world = World::new();
        main_world.init_resource::<Assets<Shader>>();
        main_world.insert_resource(Lab::new(LabScene::default_home()));
        main_world.insert_resource(ShaderSwapGuard {
            committed: Some(engine_lab::code::GAME_CODE.to_string()),
            reverted_since_swap: true,
        });
        let mut render_world = World::new();
        let error = RenderError {
            ty: ErrorType::Validation,
            description: "binding missing".to_string(),
            source: None,
        };
        let policy = lab_error_handler(&error, &mut main_world, &mut render_world);
        assert!(matches!(policy, RenderErrorPolicy::Ignore));
    }

    #[test]
    fn other_render_errors_keep_the_default_policy() {
        // Anything else (game text applied, or a non-validation error)
        // quits like Bevy's default handler.
        let mut main_world = World::new();
        main_world.init_resource::<Messages<AppExit>>();
        main_world.insert_resource(Lab::new(LabScene::default_home()));
        let mut render_world = World::new();
        let error = RenderError {
            ty: ErrorType::Validation,
            description: "broken".to_string(),
            source: None,
        };
        let policy = lab_error_handler(&error, &mut main_world, &mut render_world);
        assert!(matches!(policy, RenderErrorPolicy::StopRendering));
    }

    #[test]
    fn lab_swap_reaches_the_shader_asset() {
        // T2 of #666: the swap writes the applied text into the shader
        // asset (the hot-reload path `get_mut` notifies); a missing
        // asset reports false so the system retries. No window, no GPU.
        // The full path (handle resolution, pipeline rebuild, redraw) is
        // proved on the real window by the T2 spike.
        let mut shaders = Assets::<Shader>::default();
        let handle = shaders.add(Shader::from_wgsl(engine_lab::code::GAME_CODE, "spike"));
        let edited = engine_lab::code::GAME_CODE.replace("0.5", "0.0");
        assert!(swap_shader_to(&mut shaders, &handle, &edited));
        assert_eq!(
            shaders
                .get(&handle)
                .expect("the shader asset")
                .source
                .as_str(),
            edited.as_str(),
            "the applied text reaches the asset"
        );
        shaders.remove(&handle);
        assert!(
            !swap_shader_to(&mut shaders, &handle, &edited),
            "a missing asset retries later"
        );
    }

    #[test]
    fn selected_entry_text_reaches_the_shader_asset() {
        // T2 of #667: what `apply_shader_code` swaps is the shown entry's
        // applied text: a saved red edit on a created shader reaches the
        // single asset on select, and going back restores the game code.
        // No window, no GPU (handle resolution and the redraw run live).
        let mut lab = Lab::new(LabScene::default_home());
        lab.library.new_shader();
        {
            let shown = lab
                .library
                .selected_entry_mut()
                .expect("the new shader selected");
            shown.code.edited = engine_lab::library::BLANK_CODE.replace(
                "out.color = pbr_input.material.base_color;",
                "out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0);",
            );
            shown.code.save();
        }
        let mut shaders = Assets::<Shader>::default();
        let handle = shaders.add(Shader::from_wgsl(GAME_CODE, "spike"));
        let wanted = lab.library.applied(true).2.to_string();
        assert!(wanted.contains("1.0, 0.0, 0.0"), "the red edit is wanted");
        assert!(swap_shader_to(&mut shaders, &handle, &wanted));
        assert_eq!(
            shaders
                .get(&handle)
                .expect("the shader asset")
                .source
                .as_str(),
            wanted.as_str(),
            "the shown entry reaches the asset"
        );
        assert!(
            lab.library
                .select(engine_lab::library::PLANET_SQUARES_ENTRY)
        );
        let wanted = lab.library.applied(true).2.to_string();
        assert_eq!(wanted, GAME_CODE, "going back restores the game code");
        assert!(swap_shader_to(&mut shaders, &handle, &wanted));
    }
}
