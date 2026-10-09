//! Quit, hover, click/wheel/keys, free flight, saved views, and the autopilot.
//!
//! Everything here is `pub(crate)`: only this crate uses it.

use crate::Universe;
use crate::camera::{ExposureLevel, step_exposure};
use crate::style::{PICK_PIXELS, to_vec3};
use bevy::camera::Camera;
use bevy::ecs::system::Single;
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::math::Vec3;
use bevy::prelude::*;
use universe_core::coords::Level;
use universe_core::flight::{
    FREE_PITCH_LIMIT, FREE_SPEED_DEFAULT, FreeKeys, FreePose, free_flight_step, nearest_portal,
};
use universe_core::r#gen::MarkerKind;
use universe_core::nav::{AUTOPILOT_RATE, DiveEvent, DiveMode, KEY_RATE, WHEEL_FACTOR};

/// Hover, target, and smoothed look point.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Navigation {
    /// Marker under the cursor, if any.
    pub hover: Option<u32>,
    /// Locked target marker, if any.
    pub target: Option<u32>,
    /// Current look point in open-cell units (eased toward the target).
    pub look: Vec3,
    /// Level the title was last written for.
    pub titled: Option<Level>,
    /// Camera offset at the previous input sample, for horizon speed.
    pub last: Option<[f64; 3]>,
    /// Debug axes visible (`X` toggles, off by default).
    pub show_axes: bool,
}

/// Spacebar autopilot: targets the seeded marker per level and dives.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Autopilot {
    /// Manual control.
    #[default]
    Idle,
    /// Diving automatically until L10.
    Flying,
}

/// Target-dive versus free flight (#152).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum FlightMode {
    /// Dive at the stored target (wheel, arrows, autopilot).
    #[default]
    Dive,
    /// Steer freely (WASD plus mouse-drag look).
    Free,
}

/// Free-flight pose and speed (#152).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub(crate) struct Flight {
    /// Current mode.
    pub mode: FlightMode,
    /// Look yaw around Y; zero faces `-Z`.
    pub yaw: f64,
    /// Look pitch toward `+Y`, clamped to the core limit.
    pub pitch: f64,
    /// Speed step `1..=9` (`Shift+1-9`); 5 is the surface-distance rate.
    pub step: u32,
}

impl Default for Flight {
    fn default() -> Flight {
        Flight {
            mode: FlightMode::Dive,
            yaw: 0.0,
            pitch: 0.0,
            step: FREE_SPEED_DEFAULT,
        }
    }
}

/// One saved camera pose: offset plus target, or a free pose (#152).
///
/// Memory only; slots die with the session.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct SavedView {
    /// Camera offset in open-cell units.
    pub offset: [f64; 3],
    /// Stored target, if any.
    pub target: Option<u32>,
    /// Mode the pose was saved in.
    pub mode: FlightMode,
    /// Free-flight look yaw.
    pub yaw: f64,
    /// Free-flight look pitch.
    pub pitch: f64,
    /// Free-flight speed step.
    pub step: u32,
}

/// Numbered session save slots: `Ctrl+1-8` saves, `1-8` recalls (#152).
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub(crate) struct SavedSlots {
    /// Slot per digit; empty slots recall to nothing.
    pub slots: [Option<SavedView>; 8],
}

/// Drag-look sensitivity: radians of yaw/pitch per pixel (#152).
const LOOK_SENSITIVITY: f64 = 0.005;

/// Digit just pressed this frame (`1-9`), if any.
fn pressed_digit(keys: &ButtonInput<KeyCode>) -> Option<u32> {
    if keys.just_pressed(KeyCode::Digit1) {
        Some(1)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(2)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(3)
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some(4)
    } else if keys.just_pressed(KeyCode::Digit5) {
        Some(5)
    } else if keys.just_pressed(KeyCode::Digit6) {
        Some(6)
    } else if keys.just_pressed(KeyCode::Digit7) {
        Some(7)
    } else if keys.just_pressed(KeyCode::Digit8) {
        Some(8)
    } else if keys.just_pressed(KeyCode::Digit9) {
        Some(9)
    } else {
        None
    }
}

/// Save-slot index for digit `1-8`; `9` steps speed only and has no slot.
fn slot_index(digit: u32) -> Option<usize> {
    match digit {
        1 => Some(0),
        2 => Some(1),
        3 => Some(2),
        4 => Some(3),
        5 => Some(4),
        6 => Some(5),
        7 => Some(6),
        8 => Some(7),
        _ => None,
    }
}

/// Quits on `Esc`.
pub(crate) fn handle_quit(keys: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

/// Finds the marker nearest the cursor within [`PICK_PIXELS`](crate::style::PICK_PIXELS).
pub(crate) fn pick_hover(
    universe: Res<Universe>,
    mut nav: ResMut<Navigation>,
    window: Single<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    nav.hover = None;
    if !universe.path.can_open() {
        return;
    }
    let window = window.into_inner();
    let Some((camera, camera_transform)) = cameras.iter().next() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let mut best: Option<(u32, f32)> = None;
    for (index, point) in universe.open.points.iter().enumerate() {
        if point.kind != MarkerKind::Portal {
            continue;
        }
        let Ok(screen) = camera.world_to_viewport(camera_transform, to_vec3(point.position)) else {
            continue;
        };
        let distance = screen.distance(cursor);
        if distance <= PICK_PIXELS && best.is_none_or(|(_, d)| distance < d) {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: hovered marker index into a budgeted cell, always fits u32"
            )]
            let picked = index as u32;
            best = Some((picked, distance));
        }
    }
    nav.hover = best.map(|(index, _)| index);
}

/// Handles click, wheel, keys, flight, saves, and the autopilot.
///
/// Click locks the hovered marker in dive mode. Wheel and arrows dive with
/// the target (or the cell center). `F` toggles target-dive vs free flight;
/// free flight steers with WASD plus right-drag look at a surface-distance
/// speed stepped by `Shift+1-9`. `Ctrl+1-8` saves a view, `1-8` recalls it.
/// `E` dims the manual exposure one step, `Shift+E` brightens it (#157).
/// Spacebar flies the dive-only autopilot: in free flight it returns to the
/// dive (nearest portal) first, then flies. Any dive input cancels the
/// autopilot; a manual stored target survives pass-through entry while the
/// autopilot re-picks per cell.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input; the mode toggle, saves, steering, and dive share one frame so a split would manufacture ordering hazards"
)]
pub(crate) fn handle_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    mut motions: MessageReader<MouseMotion>,
    mut universe: ResMut<Universe>,
    mut nav: ResMut<Navigation>,
    mut autopilot: ResMut<Autopilot>,
    mut flight: ResMut<Flight>,
    mut slots: ResMut<SavedSlots>,
    mut exposure: ResMut<ExposureLevel>,
) {
    let mut notches = 0.0f64;
    for event in wheel.read() {
        notches += match event.unit {
            MouseScrollUnit::Line => f64::from(event.y),
            MouseScrollUnit::Pixel => f64::from(event.y) / 24.0,
        };
    }
    let dt = f64::from(time.delta_secs());
    let mut log_factor = notches * WHEEL_FACTOR.ln();
    if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::Equal) {
        log_factor -= KEY_RATE * dt;
    }
    if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::Minus) {
        log_factor += KEY_RATE * dt;
    }
    // `F` toggles target-dive vs free flight.
    if keys.just_pressed(KeyCode::KeyF) {
        flight.mode = match flight.mode {
            FlightMode::Dive => {
                // Entering free flight keeps the camera and drops the
                // target; the autopilot never steers free flight.
                nav.target = None;
                *autopilot = Autopilot::Idle;
                FlightMode::Free
            }
            FlightMode::Free => {
                // Leaving targets the nearest portal.
                nav.target = nearest_portal(universe.path.offset(), &universe.open);
                FlightMode::Dive
            }
        };
    }
    // Digits: `Shift+1-9` steps speed, `Ctrl+1-8` saves, `1-8` recalls.
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    // `E` dims the manual exposure one step, `Shift+E` brightens it (#157).
    if keys.just_pressed(KeyCode::KeyE) {
        exposure.0 = step_exposure(exposure.0, shift);
    }
    // `X` toggles the debug axes, off by default (#384).
    if keys.just_pressed(KeyCode::KeyX) {
        nav.show_axes = !nav.show_axes;
    }
    if let Some(digit) = pressed_digit(&keys) {
        if shift {
            flight.step = digit;
        } else if ctrl {
            if let Some(slot) = slot_index(digit)
                && let Some(entry) = slots.slots.get_mut(slot)
            {
                *entry = Some(SavedView {
                    offset: universe.path.offset(),
                    target: nav.target,
                    mode: flight.mode,
                    yaw: flight.yaw,
                    pitch: flight.pitch,
                    step: flight.step,
                });
            }
        } else if let Some(slot) = slot_index(digit)
            && let Some(view) = slots.slots.get(slot).copied().flatten()
        {
            universe.path.set_offset(view.offset);
            nav.target = view.target;
            flight.mode = view.mode;
            flight.yaw = view.yaw;
            flight.pitch = view.pitch;
            flight.step = view.step;
        }
    }
    // Right-drag looks around in free flight.
    let (mut drag_x, mut drag_y) = (0.0f64, 0.0f64);
    for motion in motions.read() {
        if buttons.pressed(MouseButton::Right) {
            drag_x += f64::from(motion.delta.x);
            drag_y += f64::from(motion.delta.y);
        }
    }
    if flight.mode == FlightMode::Free && (drag_x != 0.0 || drag_y != 0.0) {
        flight.yaw -= drag_x * LOOK_SENSITIVITY;
        flight.pitch =
            (flight.pitch - drag_y * LOOK_SENSITIVITY).clamp(-FREE_PITCH_LIMIT, FREE_PITCH_LIMIT);
    }
    let clicked = buttons.just_pressed(MouseButton::Left);
    let manual = clicked || log_factor != 0.0;
    if keys.just_pressed(KeyCode::Space) {
        match flight.mode {
            FlightMode::Dive => {
                *autopilot = match *autopilot {
                    Autopilot::Idle => Autopilot::Flying,
                    Autopilot::Flying => Autopilot::Idle,
                };
            }
            FlightMode::Free => {
                // Returns to the dive first (nearest portal), then flies.
                flight.mode = FlightMode::Dive;
                nav.target = nearest_portal(universe.path.offset(), &universe.open);
                *autopilot = Autopilot::Flying;
            }
        }
    } else if manual {
        *autopilot = Autopilot::Idle;
    }
    if clicked
        && flight.mode == FlightMode::Dive
        && let Some(hover) = nav.hover
    {
        nav.target = Some(hover);
    }
    if *autopilot == Autopilot::Flying && flight.mode == FlightMode::Dive {
        if !universe.path.can_open() {
            *autopilot = Autopilot::Idle;
        } else {
            if nav.target.is_none() {
                nav.target = universe.autopilot_target();
            }
            log_factor = -AUTOPILOT_RATE * dt;
        }
    }
    if flight.mode == FlightMode::Free {
        let steering = FreeKeys {
            forward: keys.pressed(KeyCode::KeyW),
            back: keys.pressed(KeyCode::KeyS),
            left: keys.pressed(KeyCode::KeyA),
            right: keys.pressed(KeyCode::KeyD),
        };
        let next = free_flight_step(
            FreePose {
                camera: universe.path.offset(),
                yaw: flight.yaw,
                pitch: flight.pitch,
            },
            steering,
            flight.step,
            &universe.open,
            universe.marker_radius(),
            dt,
        );
        universe.path.set_offset(next);
    }
    // Horizon speed from the actual camera velocity (#152).
    let before = universe.path.offset();
    let speed = match (nav.last, dt > 0.0) {
        (Some(last), true) => {
            let dx = before[0] - last[0];
            let dy = before[1] - last[1];
            let dz = before[2] - last[2];
            (dx * dx + dy * dy + dz * dz).sqrt() / dt
        }
        _ => 0.0,
    };
    nav.last = Some(before);
    if flight.mode == FlightMode::Free {
        return;
    }
    if log_factor == 0.0 {
        return;
    }
    // The autopilot flies the fixed journey (target-only opens); a manual
    // dive passes through crossed portals with the stored target kept.
    let mode = match *autopilot {
        Autopilot::Flying => DiveMode::Targeted,
        Autopilot::Idle => DiveMode::Passing,
    };
    match universe.dive(nav.target, log_factor.exp(), speed, mode) {
        DiveEvent::Moved => {}
        DiveEvent::Opened(_) => {
            // Pass-through (#152): a manual stored target survives entry.
            // The autopilot re-picks per cell, so it clears back to `None`.
            if *autopilot == Autopilot::Flying {
                nav.target = None;
            }
            nav.look = Vec3::ZERO;
        }
        DiveEvent::Closed(opened) => {
            nav.target = Some(opened.marker);
            nav.look = to_vec3(opened.position);
        }
    }
}
