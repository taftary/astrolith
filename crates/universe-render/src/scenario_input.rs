//! Input boundary adapter. Never calls a gameplay handler or writes Interaction.

use crate::bindings::Source;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::{CursorMoved, PrimaryWindow};

fn primary(world: &mut World) -> Result<Entity, String> {
    let mut query = world.query_filtered::<Entity, With<PrimaryWindow>>();
    query
        .single(world)
        .map_err(|_| "scenario requires exactly one primary window".into())
}

pub(crate) fn input(world: &mut World, source: Source, down: bool) -> Result<(), String> {
    let window = primary(world)?;
    let state = if down {
        ButtonState::Pressed
    } else {
        ButtonState::Released
    };
    match source {
        Source::Keys(key_code, _) => {
            world.write_message(KeyboardInput {
                key_code,
                logical_key: Key::Unidentified(NativeKey::Unidentified),
                state,
                text: None,
                repeat: false,
                window,
            });
        }
        Source::Mouse(button) => {
            world.write_message(MouseButtonInput {
                button,
                state,
                window,
            });
        }
    }
    Ok(())
}

/// Logical window pixels, matching the player-facing cursor API.
pub(crate) fn pointer(world: &mut World, at: Vec2) -> Result<(), String> {
    let entity = primary(world)?;
    let mut window = world.get_mut::<Window>(entity).ok_or("window missing")?;
    if at.x < 0.0 || at.y < 0.0 || at.x >= window.width() || at.y >= window.height() {
        return Err("pointer lies outside the window".into());
    }
    let previous = window.cursor_position();
    window.set_cursor_position(Some(at));
    world.write_message(CursorMoved {
        window: entity,
        position: at,
        delta: previous.map(|p| at - p),
    });
    Ok(())
}

pub(crate) fn motion(world: &mut World, delta: Vec2) {
    world.write_message(MouseMotion { delta });
}

pub(crate) fn scroll(world: &mut World, x: f32, y: f32) -> Result<(), String> {
    let window = primary(world)?;
    world.write_message(MouseWheel {
        unit: MouseScrollUnit::Line,
        x,
        y,
        window,
        phase: bevy::input::touch::TouchPhase::Moved,
    });
    Ok(())
}

/// Release using the same event path, including on timeout. The next InputSystems
/// pass consumes releases before the app's terminal exit is emitted in Last.
pub(crate) fn release(world: &mut World) {
    let keys: Vec<_> = world
        .resource::<ButtonInput<KeyCode>>()
        .get_pressed()
        .copied()
        .collect();
    let buttons: Vec<_> = world
        .resource::<ButtonInput<MouseButton>>()
        .get_pressed()
        .copied()
        .collect();
    for key in keys {
        let _ = input(world, Source::Keys(key, None), false);
    }
    for button in buttons {
        let _ = input(world, Source::Mouse(button), false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_reach_normal_input_processing_and_edges_clear_in_order() {
        // E-TEST-LADDER: no window backend or GPU. Same InputPlugin as the
        // window; an entity supplies the destination window id only.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::input::InputPlugin));
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        let mut once = true;
        app.add_systems(
            PreUpdate,
            (move |world: &mut World| {
                if once {
                    input(world, Source::Keys(KeyCode::KeyW, None), true).unwrap();
                    once = false;
                }
            })
            .before(bevy::input::InputSystems),
        );
        app.update();
        assert!(
            app.world()
                .resource::<ButtonInput<KeyCode>>()
                .just_pressed(KeyCode::KeyW)
        );
        app.update();
        let keys = app.world().resource::<ButtonInput<KeyCode>>();
        assert!(keys.pressed(KeyCode::KeyW));
        assert!(!keys.just_pressed(KeyCode::KeyW));
        app.add_systems(PreUpdate, release.before(bevy::input::InputSystems));
        app.update();
        assert!(
            app.world()
                .resource::<ButtonInput<KeyCode>>()
                .just_released(KeyCode::KeyW)
        );
    }
}
