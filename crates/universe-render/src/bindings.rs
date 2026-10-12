//! Reads actions through the saved key map (#560).
//!
//! The core [`Keymap`] names one key or mouse button per action; this maps
//! the names to Bevy's [`KeyCode`] and [`MouseButton`] (`Shift`, `Ctrl` and
//! `Alt` count on either side) so input systems ask "is Forward held?"
//! instead of naming a key. Everything here is `pub(crate)`.

use bevy::prelude::*;
use universe_core::keymap::{Action, Binding, Keymap};

/// What one binding listens to: up to two keys (left and right modifier),
/// or one mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    /// One or two keys; either counts.
    Keys(KeyCode, Option<KeyCode>),
    /// One mouse button.
    Mouse(MouseButton),
}

/// The Bevy input a binding name stands for, or `None` for an unknown name.
#[must_use]
pub(crate) fn source(binding: Binding) -> Option<Source> {
    let one = |key| Some(Source::Keys(key, None));
    match binding.name() {
        "Shift" => Some(Source::Keys(KeyCode::ShiftLeft, Some(KeyCode::ShiftRight))),
        "Ctrl" => Some(Source::Keys(
            KeyCode::ControlLeft,
            Some(KeyCode::ControlRight),
        )),
        "Alt" => Some(Source::Keys(KeyCode::AltLeft, Some(KeyCode::AltRight))),
        "MouseLeft" => Some(Source::Mouse(MouseButton::Left)),
        "MouseRight" => Some(Source::Mouse(MouseButton::Right)),
        "MouseMiddle" => Some(Source::Mouse(MouseButton::Middle)),
        "MouseBack" => Some(Source::Mouse(MouseButton::Back)),
        "MouseForward" => Some(Source::Mouse(MouseButton::Forward)),
        name => key_code(name).and_then(one),
    }
}

/// The key code for a plain key name in the core table.
fn key_code(name: &str) -> Option<KeyCode> {
    Some(match name {
        "KeyA" => KeyCode::KeyA,
        "KeyB" => KeyCode::KeyB,
        "KeyC" => KeyCode::KeyC,
        "KeyD" => KeyCode::KeyD,
        "KeyE" => KeyCode::KeyE,
        "KeyF" => KeyCode::KeyF,
        "KeyG" => KeyCode::KeyG,
        "KeyH" => KeyCode::KeyH,
        "KeyI" => KeyCode::KeyI,
        "KeyJ" => KeyCode::KeyJ,
        "KeyK" => KeyCode::KeyK,
        "KeyL" => KeyCode::KeyL,
        "KeyM" => KeyCode::KeyM,
        "KeyN" => KeyCode::KeyN,
        "KeyO" => KeyCode::KeyO,
        "KeyP" => KeyCode::KeyP,
        "KeyQ" => KeyCode::KeyQ,
        "KeyR" => KeyCode::KeyR,
        "KeyS" => KeyCode::KeyS,
        "KeyT" => KeyCode::KeyT,
        "KeyU" => KeyCode::KeyU,
        "KeyV" => KeyCode::KeyV,
        "KeyW" => KeyCode::KeyW,
        "KeyX" => KeyCode::KeyX,
        "KeyY" => KeyCode::KeyY,
        "KeyZ" => KeyCode::KeyZ,
        "Digit0" => KeyCode::Digit0,
        "Digit1" => KeyCode::Digit1,
        "Digit2" => KeyCode::Digit2,
        "Digit3" => KeyCode::Digit3,
        "Digit4" => KeyCode::Digit4,
        "Digit5" => KeyCode::Digit5,
        "Digit6" => KeyCode::Digit6,
        "Digit7" => KeyCode::Digit7,
        "Digit8" => KeyCode::Digit8,
        "Digit9" => KeyCode::Digit9,
        "ArrowUp" => KeyCode::ArrowUp,
        "ArrowDown" => KeyCode::ArrowDown,
        "ArrowLeft" => KeyCode::ArrowLeft,
        "ArrowRight" => KeyCode::ArrowRight,
        "F1" => KeyCode::F1,
        "F2" => KeyCode::F2,
        "F3" => KeyCode::F3,
        "F4" => KeyCode::F4,
        "F5" => KeyCode::F5,
        "F6" => KeyCode::F6,
        "F7" => KeyCode::F7,
        "F8" => KeyCode::F8,
        "F9" => KeyCode::F9,
        "F10" => KeyCode::F10,
        "F11" => KeyCode::F11,
        "F12" => KeyCode::F12,
        "Space" => KeyCode::Space,
        "Tab" => KeyCode::Tab,
        "Enter" => KeyCode::Enter,
        "Backspace" => KeyCode::Backspace,
        "Escape" => KeyCode::Escape,
        "Minus" => KeyCode::Minus,
        "Equal" => KeyCode::Equal,
        "BracketLeft" => KeyCode::BracketLeft,
        "BracketRight" => KeyCode::BracketRight,
        "Semicolon" => KeyCode::Semicolon,
        "Quote" => KeyCode::Quote,
        "Comma" => KeyCode::Comma,
        "Period" => KeyCode::Period,
        "Slash" => KeyCode::Slash,
        "Backslash" => KeyCode::Backslash,
        "Backquote" => KeyCode::Backquote,
        "Insert" => KeyCode::Insert,
        "Delete" => KeyCode::Delete,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "Numpad0" => KeyCode::Numpad0,
        "Numpad1" => KeyCode::Numpad1,
        "Numpad2" => KeyCode::Numpad2,
        "Numpad3" => KeyCode::Numpad3,
        "Numpad4" => KeyCode::Numpad4,
        "Numpad5" => KeyCode::Numpad5,
        "Numpad6" => KeyCode::Numpad6,
        "Numpad7" => KeyCode::Numpad7,
        "Numpad8" => KeyCode::Numpad8,
        "Numpad9" => KeyCode::Numpad9,
        _ => return None,
    })
}

/// The binding a pressed key or mouse button stands for (the Keys page's
/// capture, #560 T3): either side of a modifier is the one modifier.
#[must_use]
pub(crate) fn binding_of_key(key: KeyCode) -> Option<Binding> {
    let name = match key {
        KeyCode::ShiftLeft | KeyCode::ShiftRight => "Shift",
        KeyCode::ControlLeft | KeyCode::ControlRight => "Ctrl",
        KeyCode::AltLeft | KeyCode::AltRight => "Alt",
        other => universe_core::keymap::BINDING_NAMES
            .iter()
            .copied()
            .find(|name| key_code(name) == Some(other))?,
    };
    Binding::named(name)
}

/// The binding a mouse button stands for, if it is bindable.
#[must_use]
pub(crate) fn binding_of_button(button: MouseButton) -> Option<Binding> {
    Binding::named(match button {
        MouseButton::Left => "MouseLeft",
        MouseButton::Right => "MouseRight",
        MouseButton::Middle => "MouseMiddle",
        MouseButton::Back => "MouseBack",
        MouseButton::Forward => "MouseForward",
        MouseButton::Other(_) => return None,
    })
}

/// This frame's keys and mouse buttons read through a key map.
#[derive(Clone, Copy)]
pub(crate) struct Actions<'a> {
    /// The key map in force.
    pub map: Keymap,
    /// Keyboard state.
    pub keys: &'a ButtonInput<KeyCode>,
    /// Mouse button state.
    pub buttons: &'a ButtonInput<MouseButton>,
}

impl Actions<'_> {
    fn any(
        &self,
        action: Action,
        key: impl Fn(KeyCode) -> bool,
        button: impl Fn(MouseButton) -> bool,
    ) -> bool {
        match source(self.map.binding(action)) {
            Some(Source::Keys(first, second)) => key(first) || second.is_some_and(&key),
            Some(Source::Mouse(mouse)) => button(mouse),
            None => false,
        }
    }

    /// Whether the action's key or button is held.
    #[must_use]
    pub(crate) fn pressed(&self, action: Action) -> bool {
        self.any(
            action,
            |key| self.keys.pressed(key),
            |button| self.buttons.pressed(button),
        )
    }

    /// Whether the action's key or button went down this frame.
    #[must_use]
    pub(crate) fn just_pressed(&self, action: Action) -> bool {
        self.any(
            action,
            |key| self.keys.just_pressed(key),
            |button| self.buttons.just_pressed(button),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_binding_name_maps_to_an_input_and_back() {
        for name in universe_core::keymap::BINDING_NAMES {
            let binding = Binding::named(name).expect("a table name");
            let mapped = source(binding).unwrap_or_else(|| panic!("{name} maps"));
            let back = match mapped {
                Source::Keys(key, _) => binding_of_key(key),
                Source::Mouse(button) => binding_of_button(button),
            };
            assert_eq!(back, Some(binding), "{name} round-trips");
        }
    }

    #[test]
    fn a_rebound_key_works_and_the_old_one_does_not() {
        let mut map = Keymap::default();
        map.bind(Action::Forward, Binding::named("KeyI").expect("bindable"));
        let mut keys = ButtonInput::<KeyCode>::default();
        let buttons = ButtonInput::<MouseButton>::default();
        keys.press(KeyCode::KeyW);
        let actions = Actions {
            map,
            keys: &keys,
            buttons: &buttons,
        };
        assert!(
            !actions.pressed(Action::Forward),
            "W no longer moves forward"
        );
        keys.press(KeyCode::KeyI);
        let actions = Actions {
            map,
            keys: &keys,
            buttons: &buttons,
        };
        assert!(actions.pressed(Action::Forward), "I moves forward");
        assert!(actions.just_pressed(Action::Forward));
    }

    #[test]
    fn modifiers_count_on_either_side_and_buttons_bind() {
        let map = Keymap::default();
        let mut keys = ButtonInput::<KeyCode>::default();
        let mut buttons = ButtonInput::<MouseButton>::default();
        keys.press(KeyCode::ShiftRight);
        buttons.press(MouseButton::Right);
        let actions = Actions {
            map,
            keys: &keys,
            buttons: &buttons,
        };
        assert!(actions.pressed(Action::Run), "right Shift runs");
        assert!(actions.pressed(Action::Look), "right button looks");
        assert!(!actions.pressed(Action::Pick));
    }
}
