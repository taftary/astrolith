//! Player settings: values, ranges, defaults, and the text file format (#473).
//!
//! Pure data only (`E-CORE-NO-BEVY`): this module never touches the file
//! system. `universe-app` reads and writes the file; `universe-render`
//! applies the values. The file is plain `key = value` lines, written by
//! `Settings::format` and read back by the tolerant `Settings::parse`:
//! a missing, unknown, out-of-range, or unreadable line keeps its default,
//! so a damaged file can never stop the game from starting.

use crate::destination::Destination;
use crate::flight::FREE_SPEED_DEFAULT;
use crate::pick::LABEL_CAP;

/// Dimmest exposure the player can choose (two stops under the default).
pub const EXPOSURE_MIN: f32 = 0.25;
/// Brightest exposure the player can choose (two stops over the default).
pub const EXPOSURE_MAX: f32 = 4.0;
/// Exposure change per `E` / `Shift+E` tap and per slider step.
pub const EXPOSURE_STEP: f32 = 1.25;
/// Exposure at first launch and after Reset.
pub const EXPOSURE_DEFAULT: f32 = 1.0;
/// Number of saved-view slots (`Ctrl+1-8`).
pub const VIEW_SLOTS: usize = 8;
/// Slowest free-flight speed step.
pub const SPEED_MIN: u32 = 1;
/// Fastest free-flight speed step.
pub const SPEED_MAX: u32 = 9;
/// Slowest look sensitivity step (#557).
pub const LOOK_MIN: u32 = 1;
/// Fastest look sensitivity step.
pub const LOOK_MAX: u32 = 9;
/// Look sensitivity at first launch and after Reset (today's feel).
pub const LOOK_DEFAULT: u32 = 5;
/// Turning speed factor per look sensitivity step (about 1.4x a step).
pub const LOOK_STEP_FACTOR: f64 = std::f64::consts::SQRT_2;

/// Scene exposure multiplier, always inside `EXPOSURE_MIN..=EXPOSURE_MAX`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Exposure(f32);

impl Exposure {
    /// Exposure holding `value`, or `None` when it is not finite or out of range.
    #[must_use]
    pub fn new(value: f32) -> Option<Exposure> {
        (value.is_finite() && (EXPOSURE_MIN..=EXPOSURE_MAX).contains(&value))
            .then_some(Exposure(value))
    }

    /// The multiplier.
    #[must_use]
    pub fn get(self) -> f32 {
        self.0
    }

    /// One step brighter (`Shift+E`), clamped to the range.
    #[must_use]
    pub fn brighter(self) -> Exposure {
        Exposure((self.0 * EXPOSURE_STEP).clamp(EXPOSURE_MIN, EXPOSURE_MAX))
    }

    /// One step darker (`E`), clamped to the range.
    #[must_use]
    pub fn darker(self) -> Exposure {
        Exposure((self.0 / EXPOSURE_STEP).clamp(EXPOSURE_MIN, EXPOSURE_MAX))
    }
}

impl Default for Exposure {
    fn default() -> Exposure {
        Exposure(EXPOSURE_DEFAULT)
    }
}

/// Most portal labels on screen at once, `0..=LABEL_CAP`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LabelLimit(usize);

impl LabelLimit {
    /// Limit holding `count`, or `None` above [`LABEL_CAP`].
    #[must_use]
    pub fn new(count: usize) -> Option<LabelLimit> {
        (count <= LABEL_CAP).then_some(LabelLimit(count))
    }

    /// The count.
    #[must_use]
    pub fn get(self) -> usize {
        self.0
    }
}

impl Default for LabelLimit {
    fn default() -> LabelLimit {
        LabelLimit(LABEL_CAP)
    }
}

/// Free-flight speed step, `SPEED_MIN..=SPEED_MAX` (`Shift+1-9`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FlightSpeed(u32);

impl FlightSpeed {
    /// Speed holding `step`, or `None` outside `1..=9`.
    #[must_use]
    pub fn new(step: u32) -> Option<FlightSpeed> {
        (SPEED_MIN..=SPEED_MAX)
            .contains(&step)
            .then_some(FlightSpeed(step))
    }

    /// The step.
    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }
}

impl Default for FlightSpeed {
    fn default() -> FlightSpeed {
        FlightSpeed(FREE_SPEED_DEFAULT)
    }
}

/// How fast looking around turns, step `LOOK_MIN..=LOOK_MAX` (#557).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LookSensitivity(u32);

impl LookSensitivity {
    /// Sensitivity holding `step`, or `None` outside `1..=9`.
    #[must_use]
    pub fn new(step: u32) -> Option<LookSensitivity> {
        (LOOK_MIN..=LOOK_MAX)
            .contains(&step)
            .then_some(LookSensitivity(step))
    }

    /// The step.
    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }

    /// Multiplier on the base turning speed: `1.0` at [`LOOK_DEFAULT`],
    /// [`LOOK_STEP_FACTOR`] more per step up, less per step down.
    #[must_use]
    pub fn scale(self) -> f64 {
        #[expect(
            clippy::cast_possible_wrap,
            reason = "E-CAST: steps are 1..=9, far inside i32"
        )]
        let offset = self.0 as i32 - LOOK_DEFAULT as i32;
        LOOK_STEP_FACTOR.powi(offset)
    }
}

impl Default for LookSensitivity {
    fn default() -> LookSensitivity {
        LookSensitivity(LOOK_DEFAULT)
    }
}

/// Darkest Background dimming, percent (#473 Spec v3, criterion 26).
pub const DIMMING_MAX: u8 = 95;
/// Lightest Background dimming, percent.
pub const DIMMING_MIN: u8 = 60;
/// Background dimming change per step, percent.
pub const DIMMING_STEP: u8 = 5;
/// Background dimming at first launch, after Reset, and when the file has
/// no value, percent.
pub const DIMMING_DEFAULT: u8 = 80;

/// How dark the veil behind the screens is, percent: a multiple of
/// [`DIMMING_STEP`] in `DIMMING_MIN..=DIMMING_MAX`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Dimming(u8);

impl Dimming {
    /// Dimming holding `percent`, or `None` when out of range or off-step.
    #[must_use]
    pub fn new(percent: u8) -> Option<Dimming> {
        ((DIMMING_MIN..=DIMMING_MAX).contains(&percent) && percent.is_multiple_of(DIMMING_STEP))
            .then_some(Dimming(percent))
    }

    /// The percent.
    #[must_use]
    pub fn get(self) -> u8 {
        self.0
    }

    /// Darkness as a 0..=1 fraction (the veil alpha).
    #[must_use]
    pub fn fraction(self) -> f32 {
        f32::from(self.0) / 100.0
    }

    /// One step darker, clamped at [`DIMMING_MAX`].
    #[must_use]
    pub fn darker(self) -> Dimming {
        Dimming((self.0 + DIMMING_STEP).min(DIMMING_MAX))
    }

    /// One step lighter, clamped at [`DIMMING_MIN`].
    #[must_use]
    pub fn lighter(self) -> Dimming {
        Dimming(self.0.saturating_sub(DIMMING_STEP).max(DIMMING_MIN))
    }
}

impl Default for Dimming {
    fn default() -> Dimming {
        Dimming(DIMMING_DEFAULT)
    }
}

/// Camera mode a view was saved in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ViewMode {
    /// Diving at the target.
    #[default]
    Dive,
    /// Free flight.
    Free,
    /// Normal mode: the body (#558).
    Normal,
}

/// One saved camera view (`Ctrl+1-8`), as kept between runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SavedPose {
    /// Camera offset in open-cell units.
    pub offset: [f64; 3],
    /// Destination stored with the view, if any.
    pub destination: Option<Destination>,
    /// Mode the view was saved in.
    pub mode: ViewMode,
    /// Free-flight look yaw.
    pub yaw: f64,
    /// Free-flight look pitch.
    pub pitch: f64,
    /// Free-flight speed step.
    pub speed: FlightSpeed,
}

/// A Settings screen category (the left list, top level).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    /// Scene (exposure, labels) and Window (fullscreen, vsync).
    Display,
    /// Flight (speed) and Keys (read-only reference).
    Controls,
    /// The saved-view slots.
    Views,
    /// Empty for now ("Coming soon").
    Accessibility,
    /// Debug axes.
    Developer,
}

impl Category {
    /// Every category, in Settings-screen order.
    pub const ALL: [Category; 5] = [
        Category::Display,
        Category::Controls,
        Category::Views,
        Category::Accessibility,
        Category::Developer,
    ];
}

/// Every player setting plus the saved views.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// Display > Scene: exposure.
    pub exposure: Exposure,
    /// Display > Scene: portal labels shown.
    pub labels: bool,
    /// Display > Scene: most labels at once.
    pub label_limit: LabelLimit,
    /// Display > Scene: Background dimming behind the screens.
    pub dimming: Dimming,
    /// Display > Scene: Ground guides (horizon ticks, centre cross, ground
    /// ruler) at the region, city, and plot levels (#543).
    pub ground_guides: bool,
    /// Display > Scene: the on-screen mode reminder (#557).
    pub mode_reminder: bool,
    /// Display > Scene: the pull arrow and readout line in normal mode
    /// (#559, S1).
    pub show_pull: bool,
    /// Display > Scene: Planet look, black-and-white 100 m squares instead
    /// of the normal tones (#626).
    pub planet_squares: bool,
    /// Display > Window: fullscreen.
    pub fullscreen: bool,
    /// Display > Window: vsync.
    pub vsync: bool,
    /// Controls > Flight: free-flight speed.
    pub speed: FlightSpeed,
    /// Controls > Flight: how fast looking around turns (#557).
    pub look_sensitivity: LookSensitivity,
    /// Controls > Flight: up and down swapped when looking around (#557).
    pub invert_look: bool,
    /// Controls > Flight: dive keys toggle instead of hold (#557).
    pub dive_toggle: bool,
    /// Controls > Flight: the look button toggles instead of hold (#557).
    pub look_toggle: bool,
    /// Controls > Flight: flying turns like a plane (roll) instead of a
    /// drone (#558, S1).
    pub plane_turning: bool,
    /// Controls > Flight: normal mode starts in third person (#558, S2).
    pub start_third_person: bool,
    /// Controls > Keys: which key or mouse button does each action (#560).
    pub keymap: crate::keymap::Keymap,
    /// Controls > Touch: the touch controls always show, instead of only
    /// after a touch (#561, Q6).
    pub touch_always: bool,
    /// Controls > Touch: left hand (stick bottom right) instead of right
    /// (#561, Q7 A).
    pub left_hand: bool,
    /// Views: the eight slots.
    pub views: [Option<SavedPose>; VIEW_SLOTS],
    /// Developer: debug axes shown.
    pub debug_axes: bool,
    /// Developer: the mouse acts as one finger (#561, Q9 B).
    pub simulate_touch: bool,
    /// Where the player's body was left, with the universe seed it belongs
    /// to (#607, S2); `None` (or another seed) means the seeded spot.
    pub player: Option<(u64, crate::player::PlayerSpot)>,
}

impl Default for Settings {
    /// First-launch settings as documented in Spec v1 (#473).
    fn default() -> Settings {
        Settings {
            exposure: Exposure::default(),
            labels: true,
            label_limit: LabelLimit::default(),
            dimming: Dimming::default(),
            ground_guides: true,
            mode_reminder: true,
            show_pull: true,
            planet_squares: false,
            fullscreen: false,
            vsync: true,
            speed: FlightSpeed::default(),
            look_sensitivity: LookSensitivity::default(),
            invert_look: false,
            dive_toggle: false,
            look_toggle: false,
            plane_turning: false,
            start_third_person: true,
            keymap: crate::keymap::Keymap::default(),
            touch_always: false,
            left_hand: false,
            views: [None; VIEW_SLOTS],
            debug_axes: false,
            simulate_touch: false,
            player: None,
        }
    }
}

impl Settings {
    /// First-launch settings; same as [`Settings::default`].
    #[must_use]
    pub fn defaults() -> Settings {
        Settings::default()
    }

    /// These settings with `category` put back to its defaults; the other
    /// categories are untouched. Views clears every slot.
    #[must_use]
    pub fn reset(self, category: Category) -> Settings {
        let base = Settings::defaults();
        match category {
            Category::Display => Settings {
                exposure: base.exposure,
                labels: base.labels,
                label_limit: base.label_limit,
                dimming: base.dimming,
                ground_guides: base.ground_guides,
                mode_reminder: base.mode_reminder,
                show_pull: base.show_pull,
                planet_squares: base.planet_squares,
                fullscreen: base.fullscreen,
                vsync: base.vsync,
                ..self
            },
            Category::Controls => Settings {
                speed: base.speed,
                look_sensitivity: base.look_sensitivity,
                invert_look: base.invert_look,
                dive_toggle: base.dive_toggle,
                look_toggle: base.look_toggle,
                plane_turning: base.plane_turning,
                start_third_person: base.start_third_person,
                keymap: base.keymap,
                touch_always: base.touch_always,
                left_hand: base.left_hand,
                ..self
            },
            Category::Views => Settings {
                views: base.views,
                player: base.player,
                ..self
            },
            Category::Accessibility => self,
            Category::Developer => Settings {
                debug_axes: base.debug_axes,
                simulate_touch: base.simulate_touch,
                ..self
            },
        }
    }

    /// The file text: one `key = value` line per setting, then one line per
    /// filled view slot.
    #[must_use]
    pub fn format(&self) -> String {
        let mut out = String::from("# Astrolith settings (#473). Edit in the game, not here.\n");
        push_line(&mut out, "exposure", &self.exposure.get().to_string());
        push_line(&mut out, "labels", on_off(self.labels));
        push_line(&mut out, "label_limit", &self.label_limit.get().to_string());
        push_line(&mut out, "dimming", &self.dimming.get().to_string());
        push_line(&mut out, "ground_guides", on_off(self.ground_guides));
        push_line(&mut out, "mode_reminder", on_off(self.mode_reminder));
        push_line(&mut out, "show_pull", on_off(self.show_pull));
        push_line(
            &mut out,
            "planet_look",
            if self.planet_squares {
                "squares"
            } else {
                "normal"
            },
        );
        push_line(&mut out, "fullscreen", on_off(self.fullscreen));
        push_line(&mut out, "vsync", on_off(self.vsync));
        push_line(&mut out, "speed", &self.speed.get().to_string());
        push_line(
            &mut out,
            "look_sensitivity",
            &self.look_sensitivity.get().to_string(),
        );
        push_line(&mut out, "invert_look", on_off(self.invert_look));
        push_line(&mut out, "dive_toggle", on_off(self.dive_toggle));
        push_line(&mut out, "look_toggle", on_off(self.look_toggle));
        push_line(
            &mut out,
            "flying_turns",
            if self.plane_turning { "plane" } else { "drone" },
        );
        push_line(
            &mut out,
            "starting_camera",
            if self.start_third_person {
                "third"
            } else {
                "first"
            },
        );
        push_line(
            &mut out,
            "touch_controls",
            if self.touch_always {
                "always"
            } else {
                "automatic"
            },
        );
        push_line(
            &mut out,
            "hand",
            if self.left_hand { "left" } else { "right" },
        );
        push_line(&mut out, "debug_axes", on_off(self.debug_axes));
        push_line(&mut out, "simulate_touch", on_off(self.simulate_touch));
        for action in crate::keymap::Action::ALL {
            push_line(
                &mut out,
                &format!("key.{}", action.id()),
                self.keymap.binding(action).name(),
            );
        }
        for (slot, view) in self.views.iter().enumerate() {
            if let Some(view) = view {
                push_line(&mut out, &format!("view{}", slot + 1), &format_view(view));
            }
        }
        if let Some((seed, spot)) = &self.player {
            push_line(&mut out, "player", &format_player(*seed, spot));
        }
        out
    }

    /// Settings read from file `text`. Never fails: every line that is
    /// missing, unknown, out of range, or unreadable keeps its default.
    #[must_use]
    pub fn parse(text: &str) -> Settings {
        let mut s = Settings::defaults();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            s.apply(key.trim(), value.trim());
        }
        s
    }

    fn apply(&mut self, key: &str, value: &str) {
        match key {
            "exposure" => set(
                &mut self.exposure,
                value.parse().ok().and_then(Exposure::new),
            ),
            "labels" => set(&mut self.labels, parse_on_off(value)),
            "label_limit" => set(
                &mut self.label_limit,
                value.parse().ok().and_then(LabelLimit::new),
            ),
            "dimming" => set(&mut self.dimming, value.parse().ok().and_then(Dimming::new)),
            "ground_guides" => set(&mut self.ground_guides, parse_on_off(value)),
            "mode_reminder" => set(&mut self.mode_reminder, parse_on_off(value)),
            "show_pull" => set(&mut self.show_pull, parse_on_off(value)),
            "planet_look" => set(
                &mut self.planet_squares,
                match value {
                    "squares" => Some(true),
                    "normal" => Some(false),
                    _ => None,
                },
            ),
            "look_sensitivity" => set(
                &mut self.look_sensitivity,
                value.parse().ok().and_then(LookSensitivity::new),
            ),
            "invert_look" => set(&mut self.invert_look, parse_on_off(value)),
            "dive_toggle" => set(&mut self.dive_toggle, parse_on_off(value)),
            "look_toggle" => set(&mut self.look_toggle, parse_on_off(value)),
            "flying_turns" => set(
                &mut self.plane_turning,
                match value {
                    "plane" => Some(true),
                    "drone" => Some(false),
                    _ => None,
                },
            ),
            "starting_camera" => set(
                &mut self.start_third_person,
                match value {
                    "third" => Some(true),
                    "first" => Some(false),
                    _ => None,
                },
            ),
            "fullscreen" => set(&mut self.fullscreen, parse_on_off(value)),
            "vsync" => set(&mut self.vsync, parse_on_off(value)),
            "speed" => set(
                &mut self.speed,
                value.parse().ok().and_then(FlightSpeed::new),
            ),
            "debug_axes" => set(&mut self.debug_axes, parse_on_off(value)),
            "simulate_touch" => set(&mut self.simulate_touch, parse_on_off(value)),
            "touch_controls" => set(
                &mut self.touch_always,
                match value {
                    "always" => Some(true),
                    "automatic" => Some(false),
                    _ => None,
                },
            ),
            "hand" => set(
                &mut self.left_hand,
                match value {
                    "left" => Some(true),
                    "right" => Some(false),
                    _ => None,
                },
            ),
            "player" => {
                if let Some(player) = parse_player(value) {
                    self.player = Some(player);
                }
            }
            _ if key.starts_with("key.") => {
                self.keymap.apply(key.trim_start_matches("key."), value);
            }
            _ => {
                if let Some(slot) = view_slot(key)
                    && let Some(view) = parse_view(value)
                    && let Some(entry) = self.views.get_mut(slot)
                {
                    *entry = Some(view);
                }
            }
        }
    }
}

fn set<T>(field: &mut T, value: Option<T>) {
    if let Some(value) = value {
        *field = value;
    }
}

fn push_line(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push_str(" = ");
    out.push_str(value);
    out.push('\n');
}

fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

fn parse_on_off(value: &str) -> Option<bool> {
    match value {
        "on" => Some(true),
        "off" => Some(false),
        _ => None,
    }
}

/// `seed, x, y, z, yaw, chain` with the chain as markers joined by `/`
/// (#607).
fn format_player(seed: u64, spot: &crate::player::PlayerSpot) -> String {
    let [x, y, z] = spot.position;
    let parts: Vec<String> = spot.chain().iter().map(u32::to_string).collect();
    format!("{seed}, {x}, {y}, {z}, {}, {}", spot.yaw, parts.join("/"))
}

/// The inverse of [`format_player`]; `None` for anything unreadable.
fn parse_player(value: &str) -> Option<(u64, crate::player::PlayerSpot)> {
    let mut fields = value.split(',').map(str::trim);
    let seed: u64 = fields.next()?.parse().ok()?;
    let mut number = || -> Option<f64> {
        let n: f64 = fields.next()?.parse().ok()?;
        n.is_finite().then_some(n)
    };
    let position = [number()?, number()?, number()?];
    let yaw = number()?;
    let chain: Vec<u32> = fields
        .next()?
        .split('/')
        .map(|part| part.parse().ok())
        .collect::<Option<Vec<u32>>>()?;
    let spot = crate::player::PlayerSpot::new(&chain, position, yaw)?;
    Some((seed, spot))
}

/// `view1`..`view8` to slot `0..8`.
fn view_slot(key: &str) -> Option<usize> {
    let n: usize = key.strip_prefix("view")?.parse().ok()?;
    (1..=VIEW_SLOTS).contains(&n).then(|| n - 1)
}

/// `mode, x, y, z, yaw, pitch, speed, destination` with the destination as
/// `-` (none), `*` (empty chain), or markers joined by `/`.
fn format_view(view: &SavedPose) -> String {
    let mode = match view.mode {
        ViewMode::Dive => "dive",
        ViewMode::Free => "free",
        ViewMode::Normal => "normal",
    };
    let destination = match view.destination {
        None => String::from("-"),
        Some(d) if d.markers().is_empty() => String::from("*"),
        Some(d) => {
            let parts: Vec<String> = d.markers().iter().map(u32::to_string).collect();
            parts.join("/")
        }
    };
    let [x, y, z] = view.offset;
    format!(
        "{mode}, {x}, {y}, {z}, {}, {}, {}, {destination}",
        view.yaw,
        view.pitch,
        view.speed.get()
    )
}

fn parse_view(value: &str) -> Option<SavedPose> {
    let mut parts = value.split(',').map(str::trim);
    let mode = match parts.next()? {
        "dive" => ViewMode::Dive,
        "free" => ViewMode::Free,
        "normal" => ViewMode::Normal,
        _ => return None,
    };
    let mut num = || -> Option<f64> {
        let v: f64 = parts.next()?.parse().ok()?;
        v.is_finite().then_some(v)
    };
    let offset = [num()?, num()?, num()?];
    let yaw = num()?;
    let pitch = num()?;
    let speed = FlightSpeed::new(parts.next()?.parse().ok()?)?;
    let destination = match parts.next()? {
        "-" => None,
        "*" => Some(Destination::from_chain(&[])?),
        chain => {
            let markers: Option<Vec<u32>> = chain.split('/').map(|m| m.parse().ok()).collect();
            Some(Destination::from_chain(&markers?)?)
        }
    };
    if parts.next().is_some() {
        return None;
    }
    Some(SavedPose {
        offset,
        destination,
        mode,
        yaw,
        pitch,
        speed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose() -> SavedPose {
        SavedPose {
            offset: [0.25, -1.5, 3.0e-7],
            destination: Destination::from_chain(&[3, 1, 4]),
            mode: ViewMode::Free,
            yaw: 0.1,
            pitch: -0.2,
            speed: FlightSpeed(7),
        }
    }

    #[test]
    fn defaults_match_spec_v1() {
        let s = Settings::defaults();
        assert_eq!(s.exposure.get(), 1.0);
        assert!(s.labels);
        assert_eq!(s.label_limit.get(), 24);
        assert!(!s.fullscreen);
        assert!(s.vsync);
        assert!(s.ground_guides, "ground guides start on (#543)");
        assert!(s.mode_reminder, "the mode reminder starts on (#557)");
        assert!(s.show_pull, "the pull shows by default (#559)");
        assert!(
            !s.planet_squares,
            "the normal planet look by default (#626)"
        );
        assert_eq!(s.look_sensitivity.get(), LOOK_DEFAULT);
        assert!(
            !s.invert_look && !s.dive_toggle && !s.look_toggle,
            "today's controls by default"
        );
        assert!(
            !s.plane_turning,
            "flying turns like a drone by default (#558)"
        );
        assert!(
            s.start_third_person,
            "normal mode starts in third person (#558)"
        );
        assert_eq!(s.speed.get(), 5);
        assert!(s.views.iter().all(Option::is_none));
        assert!(!s.debug_axes);
        assert!(
            !s.touch_always && !s.left_hand && !s.simulate_touch,
            "touch: automatic, right hand, no simulation (#561)"
        );
    }

    #[test]
    fn ranges_reject_out_of_range_values() {
        assert!(Exposure::new(0.2).is_none());
        assert!(Exposure::new(4.1).is_none());
        assert!(Exposure::new(f32::NAN).is_none());
        assert!(Exposure::new(0.25).is_some());
        assert!(LabelLimit::new(25).is_none());
        assert!(LabelLimit::new(0).is_some());
        assert!(FlightSpeed::new(0).is_none());
        assert!(FlightSpeed::new(10).is_none());
        assert!(FlightSpeed::new(9).is_some());
    }

    #[test]
    fn exposure_steps_clamp_at_both_ends() {
        let mut e = Exposure::default();
        for _ in 0..20 {
            e = e.brighter();
        }
        assert_eq!(e.get(), EXPOSURE_MAX);
        for _ in 0..40 {
            e = e.darker();
        }
        assert_eq!(e.get(), EXPOSURE_MIN);
        assert_eq!(Exposure::default().brighter().get(), EXPOSURE_STEP);
    }

    #[test]
    fn look_sensitivity_scales_around_today() {
        assert_eq!(LookSensitivity::default().scale(), 1.0);
        let slow = LookSensitivity::new(LOOK_MIN).expect("min").scale();
        let fast = LookSensitivity::new(LOOK_MAX).expect("max").scale();
        assert!(slow < 1.0 && fast > 1.0, "{slow} {fast}");
        assert!(
            (fast * slow - 1.0).abs() < 1e-12,
            "symmetric around the default"
        );
        assert!(LookSensitivity::new(0).is_none() && LookSensitivity::new(10).is_none());
    }

    #[test]
    fn controls_reset_restores_the_new_controls() {
        let mut s = Settings::defaults();
        s.look_sensitivity = LookSensitivity(2);
        s.invert_look = true;
        s.dive_toggle = true;
        s.look_toggle = true;
        s.mode_reminder = false;
        let controls = s.reset(Category::Controls);
        assert_eq!(controls.look_sensitivity, LookSensitivity::default());
        assert!(!controls.invert_look && !controls.dive_toggle && !controls.look_toggle);
        assert!(!controls.mode_reminder, "Display keeps its own values");
        assert!(s.reset(Category::Display).mode_reminder);
    }

    #[test]
    fn round_trip_keeps_every_value() {
        let mut s = Settings::defaults();
        s.exposure = Exposure(2.4414062);
        s.labels = false;
        s.label_limit = LabelLimit(3);
        s.ground_guides = false;
        s.mode_reminder = false;
        s.show_pull = false;
        s.planet_squares = true;
        s.look_sensitivity = LookSensitivity(8);
        s.invert_look = true;
        s.dive_toggle = true;
        s.look_toggle = true;
        s.plane_turning = true;
        s.start_third_person = false;
        s.keymap.bind(
            crate::keymap::Action::Forward,
            crate::keymap::Binding::named("KeyI").expect("bindable"),
        );
        s.fullscreen = true;
        s.vsync = false;
        s.speed = FlightSpeed(9);
        s.debug_axes = true;
        s.touch_always = true;
        s.left_hand = true;
        s.simulate_touch = true;
        s.player = Some((
            42,
            crate::player::PlayerSpot::new(&[1, 2, 3], [0.1, -0.2, 0.3], 1.5).expect("spot"),
        ));
        s.views[2] = Some(pose());
        s.views[7] = Some(SavedPose {
            destination: None,
            mode: ViewMode::Dive,
            ..pose()
        });
        assert_eq!(Settings::parse(&s.format()), s);
    }

    #[test]
    fn empty_or_garbage_text_gives_defaults() {
        assert_eq!(Settings::parse(""), Settings::defaults());
        assert_eq!(
            Settings::parse("\u{0}\u{1}random ### text = = =\nexposure\n= 3"),
            Settings::defaults()
        );
    }

    #[test]
    fn bad_lines_keep_defaults_and_good_lines_still_apply() {
        let s = Settings::parse(
            "exposure = 99\nlabels = maybe\nlabel_limit = -1\nspeed = 12\n\
             view9 = dive, 0, 0, 0, 0, 0, 5, -\nview1 = dive, x, 0, 0, 0, 0, 5, -\n\
             view2 = dive, 0, 0, 0, 0, 0, 5, -, extra\nunknown = 1\nvsync = off\n",
        );
        let mut want = Settings::defaults();
        want.vsync = false;
        assert_eq!(s, want);
    }

    #[test]
    fn reset_touches_only_its_category() {
        let mut s = Settings::defaults();
        s.exposure = Exposure(2.0);
        s.speed = FlightSpeed(2);
        s.views[0] = Some(pose());
        s.debug_axes = true;
        let d = s.reset(Category::Display);
        assert_eq!(d.exposure, Exposure::default());
        assert_eq!(d.speed.get(), 2);
        let v = s.reset(Category::Views);
        assert!(v.views.iter().all(Option::is_none));
        assert!(v.debug_axes);
        assert_eq!(s.reset(Category::Accessibility), s);
        assert!(!s.reset(Category::Developer).debug_axes);
        assert_eq!(s.reset(Category::Controls).speed, FlightSpeed::default());
    }

    #[test]
    fn dimming_is_60_to_95_in_fives_default_80() {
        assert_eq!(Settings::defaults().dimming.get(), 80);
        assert!(Dimming::new(55).is_none());
        assert!(Dimming::new(100).is_none());
        assert!(Dimming::new(72).is_none(), "off-step");
        assert!(Dimming::new(60).is_some() && Dimming::new(95).is_some());
        let mut d = Dimming::default();
        for _ in 0..20 {
            d = d.darker();
        }
        assert_eq!(d.get(), DIMMING_MAX);
        for _ in 0..20 {
            d = d.lighter();
        }
        assert_eq!(d.get(), DIMMING_MIN);
        assert_eq!(Dimming::default().darker().get(), 85);
        assert!((Dimming(70).fraction() - 0.70).abs() < 1e-6);
    }

    #[test]
    fn dimming_saves_loads_and_falls_back_to_80() {
        let mut s = Settings::defaults();
        s.dimming = Dimming(70);
        assert!(s.format().contains("dimming = 70"));
        assert_eq!(Settings::parse(&s.format()).dimming.get(), 70);
        // A file written before this setting existed has no dimming line.
        let old = "exposure = 2\nlabels = off\nspeed = 3\n";
        let loaded = Settings::parse(old);
        assert_eq!(loaded.dimming.get(), DIMMING_DEFAULT);
        assert!(!loaded.labels, "the rest of the old file still loads");
        assert_eq!(Settings::parse("dimming = 99").dimming.get(), 80);
    }

    #[test]
    fn display_reset_puts_dimming_back_to_80() {
        let mut s = Settings::defaults();
        s.dimming = Dimming(60);
        assert_eq!(s.reset(Category::Display).dimming.get(), 80);
        assert_eq!(s.reset(Category::Controls).dimming.get(), 60);
    }
}
