//! Quit, hover, click/wheel/keys, and the autopilot.
//!
//! Everything here is `pub(crate)`: only this crate uses it.

use crate::Universe;
use crate::style::{PICK_PIXELS, to_vec3};
use bevy::camera::Camera;
use bevy::ecs::system::Single;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::math::Vec3;
use bevy::prelude::*;
use universe_core::coords::Level;
use universe_core::r#gen::MarkerKind;
use universe_core::nav::{AUTOPILOT_RATE, DiveEvent, KEY_RATE, WHEEL_FACTOR};

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

/// Handles click, wheel, keys, and the autopilot.
///
/// Click locks the hovered marker. Wheel and arrows dive with the target
/// (or the cell center). Spacebar toggles the autopilot; any manual input
/// cancels it. Open clears the target; close re-targets the closed marker.
pub(crate) fn handle_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    mut universe: ResMut<Universe>,
    mut nav: ResMut<Navigation>,
    mut autopilot: ResMut<Autopilot>,
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
    let clicked = buttons.just_pressed(MouseButton::Left);
    let manual = clicked || log_factor != 0.0;
    if keys.just_pressed(KeyCode::Space) {
        *autopilot = match *autopilot {
            Autopilot::Idle => Autopilot::Flying,
            Autopilot::Flying => Autopilot::Idle,
        };
    } else if manual {
        *autopilot = Autopilot::Idle;
    }
    if clicked && let Some(hover) = nav.hover {
        nav.target = Some(hover);
    }
    if *autopilot == Autopilot::Flying {
        if !universe.path.can_open() {
            *autopilot = Autopilot::Idle;
        } else {
            if nav.target.is_none() {
                nav.target = universe.autopilot_target();
            }
            log_factor = -AUTOPILOT_RATE * dt;
        }
    }
    if log_factor == 0.0 {
        return;
    }
    match universe.dive(nav.target, log_factor.exp()) {
        DiveEvent::Moved => {}
        DiveEvent::Opened(_) => {
            nav.target = None;
            nav.look = Vec3::ZERO;
        }
        DiveEvent::Closed(opened) => {
            nav.target = Some(opened.marker);
            nav.look = to_vec3(opened.position);
        }
    }
}
