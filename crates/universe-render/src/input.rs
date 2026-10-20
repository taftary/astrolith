//! Quit, hover, click/wheel/keys, free flight, saved views, and the autopilot.
//!
//! Everything here is `pub(crate)`: only this crate uses it.

use crate::Universe;
use crate::bindings::Actions;
use crate::settings_ui::SettingsStore;
use crate::tokens::{CYCLE_CAP, PICK_MOUSE_PX, PICK_TOUCH_DP};
use bevy::camera::Camera;
use bevy::ecs::system::Single;
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use universe_core::coords::Level;
use universe_core::coords::OpenUnits;
use universe_core::destination::{Destination, Way};
use universe_core::dive_orbit::{advance_roll, dive_orbit, orbit_rate};
use universe_core::flight::{
    FREE_PITCH_LIMIT, FREE_SPEED_DEFAULT, FreeKeys, FreePose, ease_heading, free_flight_step,
    nearest_portal,
};
use universe_core::frame::IDENTITY_UP;
use universe_core::r#gen::MarkerKind;
use universe_core::keymap::Action;
use universe_core::nav::{
    AUTOPILOT_RATE, DiveEvent, DiveMode, KEY_RATE, MarkerIndex, START_OFFSET, WHEEL_FACTOR,
};
use universe_core::nest::Opened;
use universe_core::pick::{pick_order, resolve_click};
use universe_core::settings::FlightSpeed;

/// Hover, destination, target, and the carried camera heading.
///
/// `forward` and `up` are the camera's pose in open-cell units. They cross
/// every open and close through the same frame maps as the camera offset
/// (#403), so a transition is invisible: the world-space view is the same
/// the frame before and after. The destination is the final surface (#423);
/// `target` is the marker in the open cell on the way to it, re-derived
/// every input frame, and the heading eases toward it. On arrival there
/// is no marker left and the camera looks at the surface itself.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub(crate) struct Navigation {
    /// Marker under the cursor, if any.
    pub hover: Option<u32>,
    /// Final destination (#423); `None` only before the first input frame,
    /// which sets the Spacebar journey's surface.
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
    /// LOD debug overlay visible (`F3` toggles, off by default, #776 Q6 A).
    pub show_overlay: bool,
    /// Point the dive camera looks at instead of the marker on the way:
    /// the player's body once the dive reaches its cell (#607, Q3 A).
    pub focus: Option<[f64; 3]>,
    /// Turn around the line to the target in dive mode (right-drag, #873 Q2):
    /// rolled onto the camera up, stays where left. Radians.
    pub roll: f64,
    /// Accumulated left-drag travel since the press, logical px (#873 T2):
    /// orbit starts past [`DIVE_DRAG_THRESHOLD_PX`], so clicks still pick.
    pub orbit_acc: f32,
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
            show_overlay: false,
            focus: None,
            roll: 0.0,
            orbit_acc: 0.0,
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
    /// Walk and fly in a body (#558, [`crate::normal_mode`]).
    Normal,
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
    /// Raw look yaw target: the mouse integrates here, and the camera
    /// eases after it in normal mode (#756 C3).
    pub target_yaw: f64,
    /// Raw look pitch target, clamped to the core limit.
    pub target_pitch: f64,
    /// Speed step `1..=9` (`Shift+1-9`); 5 is the surface-distance rate.
    pub step: u32,
}

impl Default for Flight {
    fn default() -> Flight {
        Flight {
            mode: FlightMode::Dive,
            yaw: 0.0,
            pitch: 0.0,
            target_yaw: 0.0,
            target_pitch: 0.0,
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

/// Left-drag orbit starts only after this much travel (logical px), so a
/// plain click without dragging still chooses a target (#873 T2).
const DIVE_DRAG_THRESHOLD_PX: f64 = 4.0;

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
    if flight.mode != FlightMode::Free {
        *autopilot = Autopilot::Idle;
        flight.mode = FlightMode::Free;
    }
}

/// Returns to the dive, aimed at the surface below the nearest portal.
fn enter_dive(
    flight: &mut Flight,
    universe: &universe_core::nav::Universe,
    destination: &mut Destination,
) {
    if flight.mode != FlightMode::Dive {
        *destination = nearest_destination(universe, *destination);
        flight.mode = FlightMode::Dive;
    }
}

/// Enters normal mode: the body is placed by [`crate::normal_mode`] on its
/// next step; the autopilot stops (#558).
fn enter_normal(flight: &mut Flight, autopilot: &mut Autopilot) {
    if flight.mode != FlightMode::Normal {
        *autopilot = Autopilot::Idle;
        flight.mode = FlightMode::Normal;
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
    origin: Res<crate::style::RenderOrigin>,
    touch: Res<crate::touch::TouchState>,
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
    // A tap picks at the finger with the finger-sized area (#561).
    let (pointer, reach) = match touch.tap {
        Some(tap) => (Vec2::new(tap[0], tap[1]), PICK_TOUCH_DP),
        None => match window.cursor_position() {
            Some(cursor) => (cursor, PICK_MOUSE_PX),
            None => return,
        },
    };
    // Nearest-first insertion into a fixed stack order (no allocation):
    // portals past the cycle cap keep only the nearest ones.
    let mut nearest: [(u32, f32); CYCLE_CAP] = [(0, f32::INFINITY); CYCLE_CAP];
    let mut count = 0usize;
    for (index, point) in universe.open.points.iter().enumerate() {
        if point.kind != MarkerKind::Portal {
            continue;
        }
        let Ok(screen) = camera.world_to_viewport(camera_transform, origin.at(point.position))
        else {
            continue;
        };
        let distance = screen.distance(pointer);
        if distance > reach || !distance.is_finite() {
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

/// Marker in the open cell on the way to `destination`, if any (#423).
///
/// `None` on arrival (the surface is the open cell) and off the way. Pure and
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
/// surface below it. Keeps `held` when the open cell offers no portal.
#[must_use]
fn nearest_destination(universe: &universe_core::nav::Universe, held: Destination) -> Destination {
    match nearest_portal(universe.path.offset(), &universe.open) {
        Some(marker) => held.clicked(universe.root.0, &universe.path, &universe.open, marker),
        None => held,
    }
}

/// Handles click, wheel, keys, flight, saves, and the autopilot.
///
/// There is always a destination, a surface (#423): the Spacebar journey's
/// surface at start, re-derived by a ranked click in dive mode (the portal on
/// the way wins, repeats cycle through portals only, #446; the seeded surface
/// below the pick; the marker already on the way keeps it).
/// Wheel and arrows dive toward the marker on the way, in and out, and only
/// that marker opens: crossed markers off the way are ignored. On arrival
/// the dive is a center dive on the surface. `F` toggles dive vs free flight;
/// leaving free flight aims at the surface below the nearest portal. Free
/// flight steers with WASD plus right-drag look at a surface-distance speed
/// stepped by `Shift+1-9`. Keys `2` and `3` enter free flight and the dive,
/// `1` enters normal mode (#558). `Ctrl+1-8` saves a view
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
    mut latches: ResMut<Latches>,
    touch: Res<crate::touch::TouchState>,
) {
    let mut notches = 0.0f64;
    for event in wheel.read() {
        notches += match event.unit {
            MouseScrollUnit::Line => f64::from(event.y),
            MouseScrollUnit::Pixel => f64::from(event.y) / 24.0,
        };
    }
    let dt = f64::from(time.delta_secs());
    // There is always a destination (#423): the journey's surface at start,
    // mended through the open cell should the path ever leave it.
    let root = universe.root.0;
    let mut destination = match nav.destination {
        None => Destination::seeded(root, &universe.path.indices()),
        Some(held) => held.reconciled(root, &universe.path),
    };
    // Every key and button comes from the saved key map (#560).
    let actions = Actions {
        map: store.0.keymap,
        keys: &keys,
        buttons: &buttons,
    };
    // A pinch dives like the wheel (#561).
    let mut log_factor = notches * WHEEL_FACTOR.ln() + touch.pinch;
    if store.0.dive_toggle {
        // Toggle (#557): a press latches diving in or out until the next.
        let pressed_in = actions.just_pressed(Action::DiveIn);
        let pressed_out = actions.just_pressed(Action::RiseOut);
        latches.dive = next_dive_latch(latches.dive, pressed_in, pressed_out);
        log_factor -= f64::from(latches.dive) * KEY_RATE * dt;
    } else {
        latches.dive = 0;
        if actions.pressed(Action::DiveIn) {
            log_factor -= KEY_RATE * dt;
        }
        if actions.pressed(Action::RiseOut) {
            log_factor += KEY_RATE * dt;
        }
    }
    // Switch free or dive (`F` by default) toggles target-dive vs free flight.
    if actions.just_pressed(Action::SwitchFreeDive) {
        match flight.mode {
            FlightMode::Dive | FlightMode::Normal => enter_free(&mut flight, &mut autopilot),
            FlightMode::Free => enter_dive(&mut flight, &universe, &mut destination),
        }
    }
    // Digits: Run+1-9 steps speed, Save view+1-8 saves, Bring a view
    // back+1-8 recalls (Shift, Ctrl and Alt by default); the mode keys
    // (`1`/`2`/`3` by default) switch to normal mode / free flight / the
    // dive (#557, #558, #560).
    let ctrl = actions.pressed(Action::SaveView);
    let shift = actions.pressed(Action::Run);
    let alt = actions.pressed(Action::RecallView);
    // Darker (`E`) dims the store exposure one step, with Run held it
    // brightens (#157): the camera level follows through the store (#473).
    if actions.just_pressed(Action::Exposure) {
        store.0.exposure = if shift {
            store.0.exposure.brighter()
        } else {
            store.0.exposure.darker()
        };
    }
    // Debug axes (`X`) toggles the store debug axes, off by default (#384).
    if actions.just_pressed(Action::DebugAxes) {
        store.0.debug_axes = !store.0.debug_axes;
    }
    // LOD debug overlay (`F3`) toggles the store overlay, off by default
    // (#776 Q6 A).
    if actions.just_pressed(Action::DebugOverlay) {
        store.0.debug_overlay = !store.0.debug_overlay;
    }
    // Tapping the mode reminder cycles normal, free, dive (#561, Q2 B).
    if touch.reminder_tapped {
        match flight.mode {
            FlightMode::Normal => enter_free(&mut flight, &mut autopilot),
            FlightMode::Free => enter_dive(&mut flight, &universe, &mut destination),
            FlightMode::Dive => enter_normal(&mut flight, &mut autopilot),
        }
    }
    let modified = shift || ctrl || alt;
    if !modified {
        if actions.just_pressed(Action::NormalMode) {
            enter_normal(&mut flight, &mut autopilot);
        } else if actions.just_pressed(Action::FreeMode) {
            enter_free(&mut flight, &mut autopilot);
        } else if actions.just_pressed(Action::DiveMode) {
            enter_dive(&mut flight, &universe, &mut destination);
        }
    }
    if let Some(digit) = pressed_digit(&keys) {
        if shift {
            store.0.speed = FlightSpeed::new(digit).unwrap_or_default();
        } else if ctrl {
            if let Some(slot) = slot_index(digit) {
                crate::views_ui::save_view(
                    slot,
                    &universe,
                    destination,
                    &flight,
                    &mut slots,
                    &mut store,
                );
            }
        } else if alt && let Some(slot) = slot_index(digit) {
            crate::views_ui::recall_view(
                slot,
                &mut universe,
                &mut destination,
                &mut flight,
                &slots,
            );
        }
    }
    // Look (right-drag by default) looks around in free flight; with Look
    // set to toggle a press latches looking until the next (#557). In dive
    // mode the same hold turns in place instead (#873 T2).
    let looking = if store.0.look_toggle {
        if actions.just_pressed(Action::Look) {
            latches.look = !latches.look;
        }
        latches.look
    } else {
        latches.look = false;
        actions.pressed(Action::Look)
    };
    let is_dive = flight.mode == FlightMode::Dive;
    // With Simulate touch on the left button is the finger, so the mouse
    // left-drag is ignored there and the finger drag orbits via `touch.look`
    // (#561, #873 T2).
    let simulated_pick =
        store.0.simulate_touch && store.0.keymap.binding(Action::Pick).name() == "MouseLeft";
    let left_held = buttons.pressed(MouseButton::Left) && !simulated_pick;
    if buttons.just_pressed(MouseButton::Left) {
        nav.orbit_acc = 0.0;
    }
    // A free finger drags the look without a button (#561). In dive mode the
    // same finger drag orbits instead (#873 T2).
    let (mut drag_x, mut drag_y) = (f64::from(touch.look[0]), f64::from(touch.look[1]));
    let (mut orbit_drag_x, mut orbit_drag_y) = (drag_x, drag_y);
    let mut turn_dx = 0.0f64;
    for motion in motions.read() {
        let dx = f64::from(motion.delta.x);
        let dy = f64::from(motion.delta.y);
        if looking {
            if is_dive {
                turn_dx += dx;
            } else {
                drag_x += dx;
                drag_y += dy;
            }
        }
        if is_dive && left_held {
            orbit_drag_x += dx;
            orbit_drag_y += dy;
            let step = (dx * dx + dy * dy).sqrt();
            if step.is_finite() {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "E-CAST: accumulated drag pixels, saturating narrow is intended"
                )]
                let step_px = step as f32;
                if step_px.is_finite() {
                    nav.orbit_acc += step_px;
                }
            }
        }
    }
    // Touch orbit shares the look finger in dive mode; the mouse left-drag
    // below the threshold holds still so clicks still pick (#873 T2).
    if is_dive && !left_held {
        orbit_drag_x = drag_x;
        orbit_drag_y = drag_y;
    } else if is_dive && left_held && f64::from(nav.orbit_acc) < DIVE_DRAG_THRESHOLD_PX {
        // Below the click threshold the mouse contributes nothing; a touch
        // finger still orbits through `touch.look` already counted above.
        // When only the mouse drags, this holds the pose for clicks.
        let touch_only_x = f64::from(touch.look[0]);
        let touch_only_y = f64::from(touch.look[1]);
        orbit_drag_x = touch_only_x;
        orbit_drag_y = touch_only_y;
    }
    if flight.mode != FlightMode::Dive && (drag_x != 0.0 || drag_y != 0.0) {
        let (yaw, pitch) = look_delta(
            drag_x,
            drag_y,
            store.0.look_sensitivity.scale(),
            store.0.invert_look,
        );
        if flight.mode == FlightMode::Normal {
            // The raw target moves at once; the camera eases after it
            // in `normal_mode_step`, so the view never jerks (#756 C3).
            flight.target_yaw += yaw;
            flight.target_pitch =
                (flight.target_pitch + pitch).clamp(-FREE_PITCH_LIMIT, FREE_PITCH_LIMIT);
        } else {
            flight.yaw += yaw;
            flight.pitch = (flight.pitch + pitch).clamp(-FREE_PITCH_LIMIT, FREE_PITCH_LIMIT);
            flight.target_yaw = flight.yaw;
            flight.target_pitch = flight.pitch;
        }
    }
    if flight.mode == FlightMode::Normal {
        // Normal mode moves the body in `normal_mode_step`: no dive, no
        // autopilot, no target (#558). Space jumps there.
        nav.destination = Some(destination);
        nav.target = None;
        return;
    }
    // With Simulate touch on the left button is the finger, so it picks
    // through the tap only (#561).
    let simulated =
        store.0.simulate_touch && store.0.keymap.binding(Action::Pick).name() == "MouseLeft";
    let clicked = (actions.just_pressed(Action::Pick) && !simulated) || touch.tap.is_some();
    let manual = clicked || log_factor != 0.0;
    if actions.just_pressed(Action::SpaceAction)
        || touch.pressed(universe_core::touch::TouchButton::Autopilot)
    {
        match flight.mode {
            FlightMode::Dive => {
                *autopilot = match *autopilot {
                    Autopilot::Idle => Autopilot::Flying,
                    Autopilot::Flying => Autopilot::Idle,
                };
            }
            FlightMode::Free | FlightMode::Normal => {
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
        FlightMode::Free | FlightMode::Normal => None,
    };
    // Dive orbit and turn (#873 T2, Q1-Q2): WASD plus the left-drag orbit at
    // the same distance with the gaze locked, right-drag turns in place.
    // Touch one-finger drag orbits through `touch.look`; the turn stays.
    if is_dive {
        let look_scale = store.0.look_sensitivity.scale();
        // T5 wires the Dive orbit speed setting; T2 holds 1.0.
        let orbit_speed = 1.0f64;
        let (center_arr, radius) = match nav.target.and_then(|m| universe.marker(MarkerIndex(m))) {
            Some(at) => (at, universe.marker_radius()),
            None => ([0.0; 3], 0.0),
        };
        let camera_arr = universe.path.offset();
        let dx = camera_arr[0] - center_arr[0];
        let dy = camera_arr[1] - center_arr[1];
        let dz = camera_arr[2] - center_arr[2];
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();
        if distance.is_finite() && distance > 0.0 {
            let gap = (distance - radius).max(0.0);
            let gap_share = if gap.is_finite() { gap / distance } else { 0.0 };
            let key_yaw = f64::from(u8::from(actions.pressed(Action::Right)))
                - f64::from(u8::from(actions.pressed(Action::Left)));
            let key_pitch = f64::from(u8::from(actions.pressed(Action::Forward)))
                - f64::from(u8::from(actions.pressed(Action::Back)));
            let rate = orbit_rate(gap, distance, orbit_speed);
            let mut yaw_angle = key_yaw * rate * dt;
            let mut pitch_angle = key_pitch * rate * dt;
            if orbit_drag_x != 0.0 || orbit_drag_y != 0.0 {
                // Drag follows the mouse: right moves right, up moves up;
                // invert flips the vertical drag like the look does.
                let (drag_yaw, drag_pitch) = (
                    -orbit_drag_x * LOOK_SENSITIVITY * look_scale * orbit_speed * gap_share,
                    -orbit_drag_y * LOOK_SENSITIVITY * look_scale * orbit_speed * gap_share,
                );
                let drag_pitch = if store.0.invert_look {
                    -drag_pitch
                } else {
                    drag_pitch
                };
                yaw_angle += drag_yaw;
                pitch_angle += drag_pitch;
            }
            if yaw_angle != 0.0 || pitch_angle != 0.0 {
                let (next, next_up) = dive_orbit(
                    OpenUnits(camera_arr),
                    OpenUnits(center_arr),
                    nav.up,
                    yaw_angle,
                    pitch_angle,
                );
                universe.path.set_offset(next.0);
                nav.up = next_up;
                *autopilot = Autopilot::Idle;
            }
        }
        if turn_dx != 0.0 {
            let roll_delta = -turn_dx * LOOK_SENSITIVITY * look_scale * orbit_speed;
            nav.roll = advance_roll(nav.roll, roll_delta);
            *autopilot = Autopilot::Idle;
        }
    }
    let off_the_way = nav
        .destination
        .is_some_and(|held| held.on_the_way(&universe.path) == Way::Off);
    if *autopilot == Autopilot::Flying && flight.mode == FlightMode::Dive && off_the_way {
        // Only an anchored destination (the player's, #607) stays off the
        // way: the autopilot rises back out until the way leads on.
        log_factor = AUTOPILOT_RATE * dt;
    } else if *autopilot == Autopilot::Flying && flight.mode == FlightMode::Dive {
        // The autopilot stops on arrival (or at a marker that never opens).
        // At the capped surface it instead lands: it flies to the target's
        // ground point and stops at the landing rest (#742).
        if nav.target.is_none() || (!universe.path.can_open() && universe.level().get() != 10) {
            *autopilot = Autopilot::Idle;
        } else {
            log_factor = -AUTOPILOT_RATE * dt;
        }
    }
    if flight.mode == FlightMode::Free {
        let steering = FreeKeys {
            forward: actions.pressed(Action::Forward) || touch.stick.forward,
            back: actions.pressed(Action::Back) || touch.stick.back,
            left: actions.pressed(Action::Left) || touch.stick.left,
            right: actions.pressed(Action::Right) || touch.stick.right,
        };
        // Free mode passes through everything, the ground included
        // (#558, Q7): the raw step, no ground standoff.
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
    if flight.mode == FlightMode::Free {
        return;
    }
    if log_factor == 0.0 {
        return;
    }
    // Every dive is targeted (#423, #770: landing bend removed): only the
    // marker on the way opens, so crossed markers off the way never pull the
    // camera into another branch. The autopilot keeps the straight replay
    // line, so the headless journey stays byte-identical. After a transition
    // the heading crosses the frame with the camera (#403) and the target is
    // the next marker on the way (on exit, the marker just left), so the view
    // never loses its aim.
    match universe.dive(nav.target, log_factor.exp(), DiveMode::Targeted) {
        DiveEvent::Moved => {}
        DiveEvent::Opened(_) => {
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
    // Arrived: the autopilot stops within one marker radius of the target
    // instead of hovering past it (#770: no landing bend; targeted stop).
    if *autopilot == Autopilot::Flying
        && flight.mode == FlightMode::Dive
        && universe.level().get() == 10
        && let Some(marker) = nav.target
        && let Some(at) = universe.marker(MarkerIndex(marker))
    {
        // #770: landing removed; stop within one marker radius.
        let rest = universe.marker_radius();
        let offset = universe.path.offset();
        let gap = ((offset[0] - at[0]).powi(2)
            + (offset[1] - at[1]).powi(2)
            + (offset[2] - at[2]).powi(2))
        .sqrt();
        if gap <= rest {
            *autopilot = Autopilot::Idle;
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
