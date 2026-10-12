//! Quit, hover, click/wheel/keys, free flight, saved views, and the autopilot.
//!
//! Everything here is `pub(crate)`: only this crate uses it.

use crate::Universe;
use crate::settings_ui::SettingsStore;
use crate::style::to_vec3;
use crate::tokens::{CYCLE_CAP, PICK_MOUSE_PX};
use bevy::camera::Camera;
use bevy::ecs::system::Single;
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use universe_core::coords::Level;
use universe_core::destination::{Destination, Way};
use universe_core::flight::{
    FREE_PITCH_LIMIT, FREE_SPEED_DEFAULT, FreeKeys, FreePose, ease_heading,
    free_flight_step_clamped, nearest_portal,
};
use universe_core::frame::IDENTITY_UP;
use universe_core::r#gen::MarkerKind;
use universe_core::nav::{
    AUTOPILOT_RATE, DiveEvent, DiveMode, KEY_RATE, START_OFFSET, WHEEL_FACTOR,
};
use universe_core::nest::Opened;
use universe_core::pick::{pick_order, resolve_click};
use universe_core::settings::FlightSpeed;

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

/// Numbered session save slots: `Ctrl+1-8` saves, `Alt+1-8` recalls
/// (#152, moved from plain `1-8` in #557).
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub(crate) struct SavedSlots {
    /// Slot per digit; empty slots recall to nothing.
    pub slots: [Option<SavedView>; 8],
}

/// Ranked portals under the cursor, refreshed every input frame (#446).
///
/// The order is destination-first, then nearest ([`pick_order`]); the hover
/// is its head. Fixed arrays, never allocated per frame (`E-HOT-NOALLOC`).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub(crate) struct CursorPortals {
    /// Cursor position the order was computed for, if any frame saw one.
    pub pos: [f32; 2],
    /// Ranked portal markers under the cursor.
    pub order: [u32; CYCLE_CAP],
    /// Markers held in `order`.
    pub len: usize,
    /// Whether the order is fresh (a cursor was seen this frame).
    pub valid: bool,
}

impl Default for CursorPortals {
    /// Empty, invalid order before the first cursor frame.
    fn default() -> CursorPortals {
        CursorPortals {
            pos: [0.0, 0.0],
            order: [0; CYCLE_CAP],
            len: 0,
            valid: false,
        }
    }
}

impl CursorPortals {
    /// Ranked markers as a slice (never longer than the array).
    #[must_use]
    pub(crate) fn ordered(&self) -> &[u32] {
        self.order.split_at(self.len.min(CYCLE_CAP)).0
    }
}

/// Last click for repeat-click cycling (#446 AC14).
///
/// `None` (unarmed) before the first click; moving the pointer away resets
/// through [`resolve_click`]. Copy, never allocated.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct ClickCycle {
    /// Cursor position of the last click.
    pub pos: [f32; 2],
    /// Marker the last click picked.
    pub picked: Option<u32>,
    /// Whether a previous click is held.
    pub armed: bool,
}

impl ClickCycle {
    /// Previous click as `(position, marker)`, if armed with a pick.
    #[must_use]
    pub(crate) fn last(&self) -> Option<([f32; 2], u32)> {
        if self.armed {
            self.picked.map(|marker| (self.pos, marker))
        } else {
            None
        }
    }
}

/// Drag-look sensitivity: radians of yaw/pitch per pixel at the default
/// look sensitivity step (#152; scaled by the setting since #557).
const LOOK_SENSITIVITY: f64 = 0.005;

/// When key 1 last asked for normal mode, which arrives in step 2 (#558).
///
/// The mode reminder shows [`universe_core::labels::NORMAL_MODE_NOTE`] for a
/// moment after it (#557).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct ModeNote {
    /// Elapsed seconds at the last key 1 press, if any.
    pub shown_at: Option<f64>,
}

/// Latched inputs for the toggle controls (#557).
///
/// With Dive set to toggle, a press of Up or Down keeps diving in or out
/// until the next press; with Look set to toggle, a right click keeps
/// looking until the next right click. Both rest when hold is chosen.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Latches {
    /// `1` diving in, `-1` rising out, `0` resting.
    pub dive: i8,
    /// Looking around without holding the button.
    pub look: bool,
}

/// Enters free flight: the camera and the destination stay, the autopilot
/// stops (it never steers free flight).
fn enter_free(flight: &mut Flight, autopilot: &mut Autopilot) {
    if flight.mode == FlightMode::Dive {
        *autopilot = Autopilot::Idle;
        flight.mode = FlightMode::Free;
    }
}

/// Returns to the dive, aimed at the room below the nearest portal.
fn enter_dive(
    flight: &mut Flight,
    universe: &universe_core::nav::Universe,
    destination: &mut Destination,
) {
    if flight.mode == FlightMode::Free {
        *destination = nearest_destination(universe, *destination);
        flight.mode = FlightMode::Dive;
    }
}

/// Look change in radians for a mouse drag of `(dx, dy)` pixels at look
/// sensitivity `scale`, with up and down swapped when `invert` (#557).
#[must_use]
pub(crate) fn look_delta(dx: f64, dy: f64, scale: f64, invert: bool) -> (f64, f64) {
    let yaw = -dx * LOOK_SENSITIVITY * scale;
    let pitch = -dy * LOOK_SENSITIVITY * scale;
    (yaw, if invert { -pitch } else { pitch })
}

/// Next dive latch after this frame's presses (#557): pressing the
/// direction already latched stops it, the other direction takes over.
#[must_use]
pub(crate) fn next_dive_latch(latch: i8, pressed_in: bool, pressed_out: bool) -> i8 {
    if pressed_in {
        if latch == 1 { 0 } else { 1 }
    } else if pressed_out {
        if latch == -1 { 0 } else { -1 }
    } else {
        latch
    }
}

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

/// Former quit-on-`Esc` handler, kept as a no-op pipeline stage (#473).
///
/// `Esc` no longer quits: it follows the menu Esc rule in
/// [`crate::menu`] (universe opens the Menu, screens step back, and the Menu
/// returns to the universe). Quitting happens through the Menu screen's Quit
/// button (which writes [`AppExit`]) or the window's own close button. The
/// system stays in the `Input` chain so the pipeline order test still pins
/// every gap.
pub(crate) fn handle_quit(_keys: Res<ButtonInput<KeyCode>>, _exit: MessageWriter<AppExit>) {}

/// Clears the hover and the cursor ranking when the universe view is left:
/// [`pick_hover`] only runs there, so without this the last hovered portal
/// stays "hovered" behind every screen (#473 Spec v4, criterion 19).
pub(crate) fn clear_hover(mut nav: ResMut<Navigation>, mut cursor: ResMut<CursorPortals>) {
    nav.hover = None;
    cursor.valid = false;
    cursor.len = 0;
}

/// Ranks the portals under the cursor within [`PICK_MOUSE_PX`](crate::tokens::PICK_MOUSE_PX).
///
/// The hover is the ranked head: the portal on the way to the destination
/// first, then the nearest ([`pick_order`], Spec v1 AC13). Populations never
/// qualify. The ranked order lands in [`CursorPortals`] for click cycling.
pub(crate) fn pick_hover(
    universe: Res<Universe>,
    mut nav: ResMut<Navigation>,
    mut cursor: ResMut<CursorPortals>,
    window: Single<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    nav.hover = None;
    cursor.valid = false;
    cursor.len = 0;
    if !universe.path.can_open() {
        return;
    }
    let window = window.into_inner();
    let Some((camera, camera_transform)) = cameras.iter().next() else {
        return;
    };
    let Some(pointer) = window.cursor_position() else {
        return;
    };
    // Nearest-first insertion into a fixed stack order (no allocation):
    // portals past the cycle cap keep only the nearest ones.
    let mut nearest: [(u32, f32); CYCLE_CAP] = [(0, f32::INFINITY); CYCLE_CAP];
    let mut count = 0usize;
    for (index, point) in universe.open.points.iter().enumerate() {
        if point.kind != MarkerKind::Portal {
            continue;
        }
        let Ok(screen) = camera.world_to_viewport(camera_transform, to_vec3(point.position)) else {
            continue;
        };
        let distance = screen.distance(pointer);
        if distance > PICK_MOUSE_PX || !distance.is_finite() {
            continue;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: hovered marker index into a budgeted cell, always fits u32"
        )]
        let picked = index as u32;
        // Insertion rank by screen distance.
        let mut slot = count.min(CYCLE_CAP);
        while slot > 0 {
            let held = nearest.get(slot - 1).map(|&(_, distance)| distance);
            let Some(held) = held else {
                break;
            };
            if held <= distance {
                break;
            }
            let moved = nearest.get(slot - 1).copied();
            if let (Some(value), Some(dst)) = (moved, nearest.get_mut(slot)) {
                *dst = value;
            }
            slot -= 1;
        }
        if slot < CYCLE_CAP {
            if let Some(dst) = nearest.get_mut(slot) {
                *dst = (picked, distance);
            }
            count = (count + 1).min(CYCLE_CAP);
        }
    }
    let ranked = pick_order(nearest.split_at(count.min(CYCLE_CAP)).0, nav.target);
    cursor.pos = pointer.to_array();
    cursor.len = 0;
    for marker in ranked {
        let at = cursor.len;
        if at < CYCLE_CAP
            && let Some(dst) = cursor.order.get_mut(at)
        {
            *dst = marker;
            cursor.len = at + 1;
        }
    }
    cursor.valid = true;
    nav.hover = cursor.ordered().first().copied();
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
/// room at start, re-derived by a ranked click in dive mode (the portal on
/// the way wins, repeats cycle through portals only, #446; the seeded room
/// below the pick; the marker already on the way keeps it).
/// Wheel and arrows dive toward the marker on the way, in and out, and only
/// that marker opens: crossed markers off the way are ignored. On arrival
/// the dive is a center dive in the room. `F` toggles dive vs free flight;
/// leaving free flight aims at the room below the nearest portal. Free
/// flight steers with WASD plus right-drag look at a surface-distance speed
/// stepped by `Shift+1-9`. Keys `2` and `3` enter free flight and the dive,
/// `1` notes that normal mode comes later (#557). `Ctrl+1-8` saves a view
/// with its destination, `Alt+1-8` recalls it. Look sensitivity, invert
/// look, and hold-or-toggle for the dive and look inputs follow the
/// settings store (#557). `E` dims the store exposure one step, `Shift+E`
/// brightens it (#157, applied through the store in #473). Spacebar flies the dive-only autopilot toward the
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
    mut store: ResMut<SettingsStore>,
    cursor: Res<CursorPortals>,
    mut cycle: ResMut<ClickCycle>,
    mut note: ResMut<ModeNote>,
    mut latches: ResMut<Latches>,
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
    if store.0.dive_toggle {
        // Toggle (#557): a press latches diving in or out until the next.
        let pressed_in = keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::Equal);
        let pressed_out =
            keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::Minus);
        latches.dive = next_dive_latch(latches.dive, pressed_in, pressed_out);
        log_factor -= f64::from(latches.dive) * KEY_RATE * dt;
    } else {
        latches.dive = 0;
        if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::Equal) {
            log_factor -= KEY_RATE * dt;
        }
        if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::Minus) {
            log_factor += KEY_RATE * dt;
        }
    }
    // `F` toggles target-dive vs free flight.
    if keys.just_pressed(KeyCode::KeyF) {
        match flight.mode {
            FlightMode::Dive => enter_free(&mut flight, &mut autopilot),
            FlightMode::Free => enter_dive(&mut flight, &universe, &mut destination),
        }
    }
    // Digits: `Shift+1-9` steps speed, `Ctrl+1-8` saves, `Alt+1-8` recalls,
    // plain `2`/`3` switch to free flight / the dive, plain `1` notes that
    // normal mode comes later (#557).
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    // `E` dims the store exposure one step, `Shift+E` brightens it (#157):
    // the camera level follows through the store (#473).
    if keys.just_pressed(KeyCode::KeyE) {
        store.0.exposure = if shift {
            store.0.exposure.brighter()
        } else {
            store.0.exposure.darker()
        };
    }
    // `X` toggles the store debug axes, off by default (#384).
    if keys.just_pressed(KeyCode::KeyX) {
        store.0.debug_axes = !store.0.debug_axes;
    }
    if let Some(digit) = pressed_digit(&keys) {
        if shift {
            store.0.speed = FlightSpeed::new(digit).unwrap_or_default();
        } else if ctrl {
            if let Some(slot) = slot_index(digit)
                && let Some(entry) = slots.slots.get_mut(slot)
                && let Some(stored) = store.0.views.get_mut(slot)
            {
                let saved = SavedView {
                    offset: universe.path.offset(),
                    destination: Some(destination),
                    mode: flight.mode,
                    yaw: flight.yaw,
                    pitch: flight.pitch,
                    step: flight.step,
                };
                *stored = Some(SettingsStore::view_pose(&saved));
                *entry = Some(saved);
            }
        } else if !alt {
            match digit {
                1 => note.shown_at = Some(time.elapsed_secs_f64()),
                2 => enter_free(&mut flight, &mut autopilot),
                3 => enter_dive(&mut flight, &universe, &mut destination),
                _ => {}
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
    // Right-drag looks around in free flight; with Look set to toggle a
    // right click latches looking until the next click (#557).
    let looking = if store.0.look_toggle {
        if buttons.just_pressed(MouseButton::Right) {
            latches.look = !latches.look;
        }
        latches.look
    } else {
        latches.look = false;
        buttons.pressed(MouseButton::Right)
    };
    let (mut drag_x, mut drag_y) = (0.0f64, 0.0f64);
    for motion in motions.read() {
        if looking {
            drag_x += f64::from(motion.delta.x);
            drag_y += f64::from(motion.delta.y);
        }
    }
    if flight.mode == FlightMode::Free && (drag_x != 0.0 || drag_y != 0.0) {
        let (yaw, pitch) = look_delta(
            drag_x,
            drag_y,
            store.0.look_sensitivity.scale(),
            store.0.invert_look,
        );
        flight.yaw += yaw;
        flight.pitch = (flight.pitch + pitch).clamp(-FREE_PITCH_LIMIT, FREE_PITCH_LIMIT);
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
                enter_dive(&mut flight, &universe, &mut destination);
                *autopilot = Autopilot::Flying;
            }
        }
    } else if manual {
        *autopilot = Autopilot::Idle;
    }
    if clicked && flight.mode == FlightMode::Dive {
        // Ranked click (#446 AC13-AC14): the cursor order resolves repeats
        // into cycles through portals only; without a fresh order the hover
        // still picks, so headless clicks keep working.
        let picked = if cursor.valid {
            resolve_click(cursor.ordered(), cycle.last(), cursor.pos)
        } else {
            nav.hover
        };
        if let Some(marker) = picked {
            destination = destination.clicked(root, &universe.path, &universe.open, marker);
            cycle.pos = cursor.pos;
            cycle.picked = Some(marker);
            cycle.armed = cursor.valid;
        }
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
            &universe,
            dt,
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

#[cfg(test)]
mod control_tests {
    use super::*;

    #[test]
    fn look_follows_sensitivity_and_invert() {
        let (yaw, pitch) = look_delta(10.0, 4.0, 1.0, false);
        assert!((yaw + 10.0 * LOOK_SENSITIVITY).abs() < 1e-12);
        assert!((pitch + 4.0 * LOOK_SENSITIVITY).abs() < 1e-12);
        let (_, inverted) = look_delta(10.0, 4.0, 1.0, true);
        assert_eq!(inverted, -pitch, "invert flips up and down only");
        let (fast, _) = look_delta(10.0, 4.0, 2.0, false);
        assert!(
            (fast - 2.0 * yaw).abs() < 1e-12,
            "sensitivity scales the turn"
        );
    }

    #[test]
    fn dive_latch_toggles_and_switches_direction() {
        assert_eq!(next_dive_latch(0, true, false), 1);
        assert_eq!(next_dive_latch(1, true, false), 0, "same key stops");
        assert_eq!(next_dive_latch(1, false, true), -1, "other key reverses");
        assert_eq!(next_dive_latch(-1, false, true), 0);
        assert_eq!(next_dive_latch(-1, false, false), -1, "no press keeps it");
    }
}
