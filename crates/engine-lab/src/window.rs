//! The lab's window state, cameras, grid, axes and screenshot (#640).
//!
//! [`Lab`] holds what the panels edit (scene, selected node, view toggles,
//! camera). Each frame [`apply_lab`] turns it into the game's planet view
//! ([`PlanetViewPose`], [`SurfaceDebug`], the [`Universe`]); [`lab_input`]
//! reads the keys and the mouse; [`draw_grid_and_axes`] draws the grid on
//! the equator and the X/Y/Z axes at the planet centre.

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::render::view::window::screenshot::{Screenshot, save_to_disk};
use engine_lab::keys::{KEYS, LabAction, action_for};
use engine_lab::nodes::{Node, ViewToggles};
use engine_lab::orbit::{CameraMode, OrbitCamera, Preset, free_fly_step, look_direction};
use engine_lab::scene::LabScene;
use engine_lab::select::{OUTLINE_RGB, Selection, outline, pick, pickables, tolerance};
use engine_lab::stats::FrameTimes;
use universe_render::{PlanetViewPose, SurfaceDebug, Universe};

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
    /// The About window is open.
    pub(crate) about: bool,
    /// The scene changed: rebuild the universe.
    pub(crate) rebuild: bool,
    /// A screenshot is asked for.
    pub(crate) screenshot: bool,
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
            about: false,
            rebuild: true,
            screenshot: false,
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
) {
    let (pointer_busy, keys_busy) = contexts.ctx_mut().map_or((false, false), |ctx| {
        (ctx.is_pointer_over_egui(), ctx.egui_wants_keyboard_input())
    });
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
    let busy = contexts
        .ctx_mut()
        .is_ok_and(|ctx| ctx.is_pointer_over_egui());
    let (Ok(window), Ok((camera, transform))) = (windows.single(), cameras.single()) else {
        return;
    };
    if busy {
        return;
    }
    let Some(ray) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(transform, cursor).ok())
    else {
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
}
