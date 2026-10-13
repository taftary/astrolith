//! Which key or mouse button does each action (#560).
//!
//! Pure data (`E-CORE-NO-BEVY`): keys and buttons are names from one fixed
//! table ([`BINDING_NAMES`](crate::keymap::BINDING_NAMES)), so the file format and the settings stay
//! Bevy-free; `universe-render` maps the names to its input types. One key
//! per action (owner answer Q3 A); choosing a key another action holds
//! swaps the two (Q2 A), so no action is ever left without a key.

/// Every action a key or mouse button can do, in Keys-page order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Enter normal mode.
    NormalMode,
    /// Enter free mode.
    FreeMode,
    /// Enter dive mode.
    DiveMode,
    /// Switch between free and dive.
    SwitchFreeDive,
    /// Dive in.
    DiveIn,
    /// Rise out.
    RiseOut,
    /// Autopilot in dive mode; jump and fly in normal mode.
    SpaceAction,
    /// Make the player the destination again (#607, S1).
    GoToPlayer,
    /// Move forward.
    Forward,
    /// Move back.
    Back,
    /// Move left.
    Left,
    /// Move right.
    Right,
    /// Run (walking) or boost (flying); with digits, the speed step.
    Run,
    /// Descend while flying.
    Descend,
    /// Switch the camera (first or third person).
    Camera,
    /// Darker (with Run held: brighter).
    Exposure,
    /// Debug axes on or off.
    DebugAxes,
    /// Open the menu (and step back on screens).
    Menu,
    /// Choose a target.
    Pick,
    /// Look around.
    Look,
    /// Hold, then 1-8: save a view.
    SaveView,
    /// Hold, then 1-8: bring a view back.
    RecallView,
}

impl Action {
    /// Every action, in Keys-page order.
    pub const ALL: [Action; ACTIONS] = [
        Action::NormalMode,
        Action::FreeMode,
        Action::DiveMode,
        Action::SwitchFreeDive,
        Action::DiveIn,
        Action::RiseOut,
        Action::SpaceAction,
        Action::GoToPlayer,
        Action::Forward,
        Action::Back,
        Action::Left,
        Action::Right,
        Action::Run,
        Action::Descend,
        Action::Camera,
        Action::Exposure,
        Action::DebugAxes,
        Action::Menu,
        Action::Pick,
        Action::Look,
        Action::SaveView,
        Action::RecallView,
    ];

    /// Position in [`Action::ALL`].
    #[must_use]
    pub fn index(self) -> usize {
        Action::ALL
            .iter()
            .position(|action| *action == self)
            .unwrap_or(0)
    }

    /// Settings-file key (`key.<id>`).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Action::NormalMode => "normal_mode",
            Action::FreeMode => "free_mode",
            Action::DiveMode => "dive_mode",
            Action::SwitchFreeDive => "switch_free_dive",
            Action::DiveIn => "dive_in",
            Action::RiseOut => "rise_out",
            Action::SpaceAction => "autopilot_jump",
            Action::GoToPlayer => "go_to_player",
            Action::Forward => "forward",
            Action::Back => "back",
            Action::Left => "left",
            Action::Right => "right",
            Action::Run => "run",
            Action::Descend => "descend",
            Action::Camera => "camera",
            Action::Exposure => "exposure",
            Action::DebugAxes => "debug_axes",
            Action::Menu => "menu",
            Action::Pick => "pick",
            Action::Look => "look",
            Action::SaveView => "save_view",
            Action::RecallView => "recall_view",
        }
    }

    /// Keys-page label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Action::NormalMode => "Normal mode",
            Action::FreeMode => "Free mode",
            Action::DiveMode => "Dive mode",
            Action::SwitchFreeDive => "Switch free or dive",
            Action::DiveIn => "Dive in",
            Action::RiseOut => "Rise out",
            Action::SpaceAction => "Autopilot, jump and fly",
            Action::GoToPlayer => "Go to player",
            Action::Forward => "Forward",
            Action::Back => "Back",
            Action::Left => "Left",
            Action::Right => "Right",
            Action::Run => "Run or boost",
            Action::Descend => "Descend while flying",
            Action::Camera => "Switch camera",
            Action::Exposure => "Darker (with Run: brighter)",
            Action::DebugAxes => "Debug axes",
            Action::Menu => "Menu",
            Action::Pick => "Choose a target",
            Action::Look => "Look around",
            Action::SaveView => "Save a view (hold, then 1-8)",
            Action::RecallView => "Bring a view back (hold, then 1-8)",
        }
    }
}

/// Number of actions.
pub const ACTIONS: usize = 22;

/// Every bindable key and mouse button, by name. Keys use the window
/// toolkit's key-code names; `Shift`, `Ctrl` and `Alt` count on either side
/// of the keyboard; `Mouse*` are mouse buttons.
pub const BINDING_NAMES: &[&str] = &[
    "KeyA",
    "KeyB",
    "KeyC",
    "KeyD",
    "KeyE",
    "KeyF",
    "KeyG",
    "KeyH",
    "KeyI",
    "KeyJ",
    "KeyK",
    "KeyL",
    "KeyM",
    "KeyN",
    "KeyO",
    "KeyP",
    "KeyQ",
    "KeyR",
    "KeyS",
    "KeyT",
    "KeyU",
    "KeyV",
    "KeyW",
    "KeyX",
    "KeyY",
    "KeyZ",
    "Digit0",
    "Digit1",
    "Digit2",
    "Digit3",
    "Digit4",
    "Digit5",
    "Digit6",
    "Digit7",
    "Digit8",
    "Digit9",
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "Space",
    "Tab",
    "Enter",
    "Backspace",
    "Escape",
    "Minus",
    "Equal",
    "BracketLeft",
    "BracketRight",
    "Semicolon",
    "Quote",
    "Comma",
    "Period",
    "Slash",
    "Backslash",
    "Backquote",
    "Insert",
    "Delete",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Numpad0",
    "Numpad1",
    "Numpad2",
    "Numpad3",
    "Numpad4",
    "Numpad5",
    "Numpad6",
    "Numpad7",
    "Numpad8",
    "Numpad9",
    "Shift",
    "Ctrl",
    "Alt",
    "MouseLeft",
    "MouseRight",
    "MouseMiddle",
    "MouseBack",
    "MouseForward",
];

/// One key or mouse button: an index into [`BINDING_NAMES`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Binding(u8);

impl Binding {
    /// The binding named `name`, if it is in [`BINDING_NAMES`].
    #[must_use]
    pub fn named(name: &str) -> Option<Binding> {
        let index = BINDING_NAMES.iter().position(|n| *n == name)?;
        u8::try_from(index).ok().map(Binding)
    }

    /// Its name in [`BINDING_NAMES`].
    #[must_use]
    pub fn name(self) -> &'static str {
        BINDING_NAMES
            .get(usize::from(self.0))
            .copied()
            .unwrap_or("Space")
    }

    /// Whether it is a mouse button.
    #[must_use]
    pub fn is_mouse(self) -> bool {
        self.name().starts_with("Mouse")
    }

    /// Keys-page text for it ("W", "Up arrow", "Left mouse button").
    #[must_use]
    pub fn label(self) -> String {
        let name = self.name();
        if let Some(letter) = name.strip_prefix("Key") {
            return letter.to_string();
        }
        if let Some(digit) = name.strip_prefix("Digit") {
            return digit.to_string();
        }
        if let Some(digit) = name.strip_prefix("Numpad") {
            return format!("Numpad {digit}");
        }
        if let Some(arrow) = name.strip_prefix("Arrow") {
            return format!("{arrow} arrow");
        }
        match name {
            "Escape" => "Esc".to_string(),
            "Minus" => "-".to_string(),
            "Equal" => "=".to_string(),
            "MouseLeft" => "Left mouse button".to_string(),
            "MouseRight" => "Right mouse button".to_string(),
            "MouseMiddle" => "Middle mouse button".to_string(),
            "MouseBack" => "Back mouse button".to_string(),
            "MouseForward" => "Forward mouse button".to_string(),
            other => other.to_string(),
        }
    }
}

/// One binding per action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keymap([Binding; ACTIONS]);

/// Default binding names in [`Action::ALL`] order (the spec's table).
const DEFAULTS: [&str; ACTIONS] = [
    "Digit1",
    "Digit2",
    "Digit3",
    "KeyF",
    "ArrowUp",
    "ArrowDown",
    "Space",
    "KeyP",
    "KeyW",
    "KeyS",
    "KeyA",
    "KeyD",
    "Shift",
    "KeyC",
    "KeyV",
    "KeyE",
    "KeyX",
    "Escape",
    "MouseLeft",
    "MouseRight",
    "Ctrl",
    "Alt",
];

impl Default for Keymap {
    fn default() -> Keymap {
        Keymap(DEFAULTS.map(|name| Binding::named(name).unwrap_or(Binding(0))))
    }
}

impl Keymap {
    /// The binding for `action`.
    #[must_use]
    pub fn binding(&self, action: Action) -> Binding {
        self.0.get(action.index()).copied().unwrap_or(Binding(0))
    }

    /// The action bound to `binding`, if any.
    #[must_use]
    pub fn action_for(&self, binding: Binding) -> Option<Action> {
        Action::ALL
            .iter()
            .copied()
            .find(|action| self.binding(*action) == binding)
    }

    /// Binds `action` to `binding`; an action already holding `binding`
    /// takes `action`'s old binding instead (swap, Q2 A).
    pub fn bind(&mut self, action: Action, binding: Binding) {
        let old = self.binding(action);
        if let Some(other) = self.action_for(binding)
            && other != action
            && let Some(slot) = self.0.get_mut(other.index())
        {
            *slot = old;
        }
        if let Some(slot) = self.0.get_mut(action.index()) {
            *slot = binding;
        }
    }

    /// Applies one `key.<id> = <name>` file entry; unknown actions or
    /// names change nothing (the default stays).
    pub fn apply(&mut self, id: &str, name: &str) {
        let (Some(action), Some(binding)) = (
            Action::ALL.iter().copied().find(|action| action.id() == id),
            Binding::named(name),
        ) else {
            return;
        };
        if let Some(slot) = self.0.get_mut(action.index()) {
            *slot = binding;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_the_spec_table() {
        let map = Keymap::default();
        assert_eq!(map.binding(Action::Forward).name(), "KeyW");
        assert_eq!(map.binding(Action::DiveIn).label(), "Up arrow");
        assert_eq!(map.binding(Action::Look).label(), "Right mouse button");
        assert_eq!(map.binding(Action::Menu).label(), "Esc");
        assert_eq!(map.binding(Action::Run).name(), "Shift");
        for action in Action::ALL {
            assert_eq!(
                map.action_for(map.binding(action)),
                Some(action),
                "{action:?} is unique"
            );
        }
    }

    #[test]
    fn a_taken_key_swaps() {
        let mut map = Keymap::default();
        let w = map.binding(Action::Forward);
        let s = map.binding(Action::Back);
        map.bind(Action::Back, w);
        assert_eq!(map.binding(Action::Back), w);
        assert_eq!(
            map.binding(Action::Forward),
            s,
            "Forward takes Back's old key"
        );
        let i = Binding::named("KeyI").expect("I is bindable");
        map.bind(Action::Forward, i);
        assert_eq!(map.binding(Action::Forward), i);
        assert_eq!(map.action_for(s), None, "a free key is simply released");
    }

    #[test]
    fn file_entries_apply_and_bad_ones_keep_the_default() {
        let mut map = Keymap::default();
        map.apply("forward", "KeyI");
        assert_eq!(map.binding(Action::Forward).name(), "KeyI");
        map.apply("forward", "NotAKey");
        map.apply("teleport", "KeyQ");
        assert_eq!(map.binding(Action::Forward).name(), "KeyI");
        assert_eq!(map.binding(Action::Back).name(), "KeyS");
    }

    #[test]
    fn every_name_round_trips_and_labels_are_ascii() {
        for name in BINDING_NAMES {
            let binding = Binding::named(name).expect("table name binds");
            assert_eq!(binding.name(), *name);
            assert!(binding.label().is_ascii(), "the bundled font is ASCII-only");
        }
        assert_eq!(Action::ALL.len(), ACTIONS);
        for (i, action) in Action::ALL.iter().enumerate() {
            assert_eq!(action.index(), i);
        }
    }
}
