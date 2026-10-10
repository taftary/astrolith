//! Quit, hover, click/wheel/keys, free flight, saved views, and the autopilot.
//!
//! Everything here is `pub(crate)`: only this crate uses it.

use crate::Universe;
use crate::camera::{ExposureLevel, step_exposure};
use crate::style::{PICK_PIXELS, to_vec3};
use bevy::camera::Camera;
use bevy::ecs::system::Single;
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use universe_core::coords::Level;
use universe_core::destination::{Destination, Way};
use universe_core::flight::{
    FREE_PITCH_LIMIT, FREE_SPEED_DEFAULT, FreeKeys, FreePose, ease_heading, free_flight_step_clamped,
    nearest_portal,
};
use universe_core::frame::IDENTITY_UP;
use universe_core::r#gen::MarkerKind;
use universe_core::nav::{
    AUTOPILOT_RATE, DiveEvent, DiveMode, KEY_RATE, START_OFFSET, WHEEL_FACTOR,
};
use universe_core::nest::Opened;

/// Hover, destination, target, and the carried camera heading.
///
/// `forward` and `up` are the camera's pose in open-cell units. They cross
/// every open and close through the same frame maps as the camera offset
/// (#403), so a transition is invisible: the world-space view is the same
/// the frame before and after. The destination is the final room (#423);
/// `target` is the marker in the open cell on the way to it, re-derived
/// every input frame, and the heading eases toward it. On arrival there
/// is no marker left and the camera looks at the room itself.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub(crate) struct Navigation {
    /// Marker under the cursor, if any.
    pub hover: Option<u32>,
    /// Final destination (#423); `None` only before the first input frame,
    /// which sets the Spacebar journey's room.
    pub destination: Option<Destination>,
    /// Marker in the open cell on the way to the destination, if any.
    pub target: Option<u32>,
    /// Unit view direction in open-cell units.
    pub forward: [f64; 3],
    /// Unit camera up in open-cell units (eased toward the local up: the
    /// frame's `+Y`, blending to the ground normal near the L10 planet).
    pub up: [f64; 3],
    /// Level the title was last written for.
    pub titled: Option<Level>,
    /// Debug axes visible (`X` toggles, off by default).
    pub show_axes: bool,
}

impl Default for Navigation {
    /// Starts looking from [`START_OFFSET`] at the root cell center, upright.
    fn default() -> Navigation {
        Navigation {
            hover: None,
            destination: None,
            target: None,
            forward: ease_heading(
                [0.0; 3],
                [-START_OFFSET[0], -START_OFFSET[1], -START_OFFSET[2]],
                1.0,
            ),
            up: IDENTITY_UP,
            titled: None,
            show_axes: false,
        }
    }
}

/// Spacebar autopilot: dives toward the destination until it arrives.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Autopilot {
    /// Manual control.
    #[default]
    Idle,
    /// Diving automatically until the destination is reached.
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

/// One saved camera pose: offset plus destination, or a free pose (#152).
///
/// Memory only; slots die with the session.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct SavedView {
    /// Camera offset in open-cell units.
    pub offset: [f64; 3],
    /// Stored destination (#423), if one was set.
    pub destination: Option<Destination>,
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

/// Carries the camera heading into an opened cell.
///
/// Directions turn with the frame exactly like the camera offset did
/// (#403). Pure.
pub(crate) fn carry_into_child(nav: &mut Navigation, opened: &Opened) {
    nav.forward = opened.to_child_direction(nav.forward);
    nav.up = opened.to_child_direction(nav.up);
}

/// Carries the camera heading back into the parent.
///
/// Exact inverse of [`carry_into_child`]. Pure.
pub(crate) fn carry_into_parent(nav: &mut Navigation, opened: &Opened) {
    nav.forward = opened.to_parent_direction(nav.forward);
    nav.up = opened.to_parent_direction(nav.up);
}

/// Dive mode for this frame's window dive (#430). Pure.
///
/// Manual window dives land: the step bends toward the surface normal near
/// surface portals and holds above the relief and the ground. The autopilot
/// keeps the straight replay line, so the headless journey and the goldens
/// stay byte-identical.
#[must_use]
pub(crate) fn dive_mode(autopilot: &Autopilot) -> DiveMode {
    if *autopilot == Autopilot::Flying {
        DiveMode::Targeted
    } else {
        DiveMode::Landing
    }
}

/// Marker in the open cell on the way to `destination`, if any (#423).
///
/// `None` on arrival (the room is the open cell) and off the way. Pure and
/// allocation-free, so it runs every input frame.
#[must_use]
pub(crate) fn target_on_the_way(
    destination: Option<Destination>,
    path: &universe_core::nest::MarkerPath,
) -> Option<u32> {
    match destination.map(|held| held.on_the_way(path)) {
        Some(Way::Next(marker)) => Some(marker),
        Some(Way::Arrived | Way::Off) | None => None,
    }
}

/// Destination through the portal nearest the camera (#423).
///
/// Leaving free flight aims here: the nearest portal, then the seeded
/// room below it. Keeps `held` when the open cell offers no portal.
#[must_use]
fn nearest_destination(universe: &universe_core::nav::Universe, held: Destination) -> Destination {
    match nearest_portal(universe.path.offset(), &universe.open) {
        Some(marker) => held.clicked(universe.root.0, &universe.path, &universe.open, marker),
        None => held,
    }
}

/// Handles click, wheel, keys, flight, saves, and the autopilot.
///
/// There is always a destination, a room (#423): the Spacebar journey's
/// room at start, re-derived by a click on the hovered marker in dive mode
/// (the seeded room below it; the marker already on the way keeps it).
/// Wheel and arrows dive toward the marker on the way, in and out, and only
/// that marker opens: crossed markers off the way are ignored. On arrival
/// the dive is a center dive in the room. `F` toggles dive vs free flight;
/// leaving free flight aims at the room below the nearest portal. Free
/// flight steers with WASD plus right-drag look at a surface-distance speed
/// stepped by `Shift+1-9`. `Ctrl+1-8` saves a view with its destination,
/// `1-8` recalls it. `E` dims the manual exposure one step, `Shift+E`
/// brightens it (#157). Spacebar flies the dive-only autopilot toward the
/// destination and stops on arrival: in free flight it returns to the dive
/// (nearest portal) first. Any dive input cancels the autopilot.
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
    // There is always a destination (#423): the journey's room at start,
    // mended through the open cell should the path ever leave it.
    let root = universe.root.0;
    let mut destination = match nav.destination {
        None => Destination::seeded(root, &universe.path.indices()),
        Some(held) => held.reconciled(root, &universe.path),
    };
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
                // Entering free flight keeps the camera and the
                // destination; the autopilot never steers free flight.
                *autopilot = Autopilot::Idle;
                FlightMode::Free
            }
            FlightMode::Free => {
                // Leaving aims at the room below the nearest portal.
                destination = nearest_destination(&universe, destination);
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
                    destination: Some(destination),
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
            // A saved destination comes back when it runs through the
            // open cell; a saved offset only means something there.
            if let Some(saved) = view.destination
                && saved.on_the_way(&universe.path) != Way::Off
            {
                destination = saved;
            }
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
                destination = nearest_destination(&universe, destination);
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
        destination = destination.clicked(root, &universe.path, &universe.open, hover);
    }
    nav.destination = Some(destination);
    nav.target = match flight.mode {
        FlightMode::Dive => target_on_the_way(nav.destination, &universe.path),
        FlightMode::Free => None,
    };
    if *autopilot == Autopilot::Flying && flight.mode == FlightMode::Dive {
        // The autopilot stops on arrival (or at a marker that never opens).
        if nav.target.is_none() || !universe.path.can_open() {
            *autopilot = Autopilot::Idle;
        } else {
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
        let next = free_flight_step_clamped(
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
            universe.level(),
            universe.open_seed(),
        );
        universe.path.set_offset(next);
    }
    if flight.mode == FlightMode::Free {
        return;
    }
    if log_factor == 0.0 {
        return;
    }
    // Every dive is targeted (#423): only the marker on the way opens, so
    // crossed markers off the way never pull the camera into another
    // branch. Manual window dives land (#430): the step bends toward the
    // surface normal near surface portals and holds above the relief and
    // the ground. The autopilot keeps the straight replay line, so the
    // headless journey stays byte-identical. After a transition the heading
    // crosses the frame with the camera (#403) and the target is the next
    // marker on the way (on exit, the marker just left), so the view never
    // loses its aim.
    let mode = dive_mode(&autopilot);
    match universe.dive(nav.target, log_factor.exp(), mode) {
        DiveEvent::Moved => {}
        DiveEvent::Opened(_) => {
            // The new cell may arrive rotated (tail patch entries, #394).
            if let Some(opened) = universe.path.entered() {
                carry_into_child(&mut nav, &opened);
            }
            nav.target = target_on_the_way(nav.destination, &universe.path);
        }
        DiveEvent::Closed(opened) => {
            // `opened` is the frame just left; the path's own entry is now
            // one level up.
            carry_into_parent(&mut nav, &opened);
            nav.target = target_on_the_way(nav.destination, &universe.path);
        }
    }
}
