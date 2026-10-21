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
pub struct Exposure(pub(crate) f32);

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
pub struct LabelLimit(pub(crate) usize);

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
pub struct FlightSpeed(pub(crate) u32);

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
pub struct LookSensitivity(pub(crate) u32);

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

/// Slowest dive orbit speed: half the base rate (#873 S1).
pub const DIVE_ORBIT_SPEED_MIN: f32 = 0.5;
/// Fastest dive orbit speed: twice the base rate (#873 S1).
pub const DIVE_ORBIT_SPEED_MAX: f32 = 2.0;
/// Dive orbit speed at first launch and after Reset (#873 S1).
pub const DIVE_ORBIT_SPEED_DEFAULT: f32 = 1.0;
/// Dive orbit speed change per slider step (#873 S1).
pub const DIVE_ORBIT_SPEED_STEP: f32 = 0.1;

/// How fast the dive orbits, `0.5..=2.0` (#873 S1).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct DiveOrbitSpeed(pub(crate) f32);

impl DiveOrbitSpeed {
    /// Speed holding `value`, or `None` when it is not finite or out of range.
    #[must_use]
    pub fn new(value: f32) -> Option<DiveOrbitSpeed> {
        (value.is_finite() && (DIVE_ORBIT_SPEED_MIN..=DIVE_ORBIT_SPEED_MAX).contains(&value))
            .then_some(DiveOrbitSpeed(value))
    }

    /// The multiplier.
    #[must_use]
    pub fn get(self) -> f32 {
        self.0
    }

    /// One step faster, clamped to the range.
    #[must_use]
    pub fn faster(self) -> DiveOrbitSpeed {
        DiveOrbitSpeed(
            (self.0 + DIVE_ORBIT_SPEED_STEP).clamp(DIVE_ORBIT_SPEED_MIN, DIVE_ORBIT_SPEED_MAX),
        )
    }

    /// One step slower, clamped to the range.
    #[must_use]
    pub fn slower(self) -> DiveOrbitSpeed {
        DiveOrbitSpeed(
            (self.0 - DIVE_ORBIT_SPEED_STEP).clamp(DIVE_ORBIT_SPEED_MIN, DIVE_ORBIT_SPEED_MAX),
        )
    }
}

impl Default for DiveOrbitSpeed {
    fn default() -> DiveOrbitSpeed {
        DiveOrbitSpeed(DIVE_ORBIT_SPEED_DEFAULT)
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
pub struct Dimming(pub(crate) u8);

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
    /// Free-flight look yaw (dive release and normal mode).
    pub yaw: f64,
    /// Free-flight look pitch.
    pub pitch: f64,
    /// Free-flight forward (spaceship, #884 T2).
    pub free_forward: [f64; 3],
    /// Free-flight up (spaceship, #884 T2).
    pub free_up: [f64; 3],
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

/// How the mouse look smooths in normal mode (#756, S1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CameraSmoothing {
    /// The raw mouse, as today.
    Off,
    /// Gentle smoothing (default): about 30 ms.
    #[default]
    Low,
    /// Stronger smoothing: about 60 ms.
    Medium,
    /// Strongest smoothing: about 120 ms.
    High,
}

impl CameraSmoothing {
    /// Every preset in cycle order.
    pub const ALL: [CameraSmoothing; 4] = [
        CameraSmoothing::Off,
        CameraSmoothing::Low,
        CameraSmoothing::Medium,
        CameraSmoothing::High,
    ];

    /// Next preset when the setting steps up.
    #[must_use]
    pub fn higher(self) -> CameraSmoothing {
        match self {
            CameraSmoothing::Off => CameraSmoothing::Low,
            CameraSmoothing::Low => CameraSmoothing::Medium,
            CameraSmoothing::Medium => CameraSmoothing::High,
            CameraSmoothing::High => CameraSmoothing::High,
        }
    }

    /// Next preset when the setting steps down.
    #[must_use]
    pub fn lower(self) -> CameraSmoothing {
        match self {
            CameraSmoothing::Off => CameraSmoothing::Off,
            CameraSmoothing::Low => CameraSmoothing::Off,
            CameraSmoothing::Medium => CameraSmoothing::Low,
            CameraSmoothing::High => CameraSmoothing::Medium,
        }
    }

    /// File id (`camera_smoothing = <id>`).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            CameraSmoothing::Off => "off",
            CameraSmoothing::Low => "low",
            CameraSmoothing::Medium => "medium",
            CameraSmoothing::High => "high",
        }
    }

    /// Display name shown in Settings.
    #[must_use]
    pub fn display(self) -> &'static str {
        match self {
            CameraSmoothing::Off => "Off",
            CameraSmoothing::Low => "Low",
            CameraSmoothing::Medium => "Medium",
            CameraSmoothing::High => "High",
        }
    }

    /// Preset from a file id, or `None` for anything else.
    #[must_use]
    pub fn from_id(id: &str) -> Option<CameraSmoothing> {
        match id {
            "off" => Some(CameraSmoothing::Off),
            "low" => Some(CameraSmoothing::Low),
            "medium" => Some(CameraSmoothing::Medium),
            "high" => Some(CameraSmoothing::High),
            _ => None,
        }
    }

    /// Smoothing time constant, seconds: 0 for Off (the raw mouse).
    #[must_use]
    pub fn tau(self) -> f64 {
        match self {
            CameraSmoothing::Off => crate::flight::LOOK_TAU_OFF,
            CameraSmoothing::Low => crate::flight::LOOK_TAU_LOW,
            CameraSmoothing::Medium => crate::flight::LOOK_TAU_MEDIUM,
            CameraSmoothing::High => crate::flight::LOOK_TAU_HIGH,
        }
    }
}

/// Body colour presets for the player figure (#755, S1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BodyColour {
    /// Light grey suit with an orange accent (default).
    #[default]
    GreyOrange,
    /// All white.
    White,
    /// All blue.
    Blue,
    /// All red.
    Red,
}

impl BodyColour {
    /// Every preset in cycle order.
    pub const ALL: [BodyColour; 4] = [
        BodyColour::GreyOrange,
        BodyColour::White,
        BodyColour::Blue,
        BodyColour::Red,
    ];

    /// Next preset when the setting is clicked.
    #[must_use]
    pub fn next(self) -> BodyColour {
        match self {
            BodyColour::GreyOrange => BodyColour::White,
            BodyColour::White => BodyColour::Blue,
            BodyColour::Blue => BodyColour::Red,
            BodyColour::Red => BodyColour::GreyOrange,
        }
    }

    /// File id (`body_colour = <id>`).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            BodyColour::GreyOrange => "grey-orange",
            BodyColour::White => "white",
            BodyColour::Blue => "blue",
            BodyColour::Red => "red",
        }
    }

    /// Display name shown in Settings.
    #[must_use]
    pub fn display(self) -> &'static str {
        match self {
            BodyColour::GreyOrange => "grey and orange",
            BodyColour::White => "white",
            BodyColour::Blue => "blue",
            BodyColour::Red => "red",
        }
    }

    /// Preset from a file id, or `None` for anything else.
    #[must_use]
    pub fn from_id(id: &str) -> Option<BodyColour> {
        match id {
            "grey-orange" => Some(BodyColour::GreyOrange),
            "white" => Some(BodyColour::White),
            "blue" => Some(BodyColour::Blue),
            "red" => Some(BodyColour::Red),
            _ => None,
        }
    }

    /// Suit and accent colours as sRGB `0..1`: the head and hands carry the
    /// accent, the torso and legs the suit.
    #[must_use]
    pub fn colours(self) -> ([f32; 3], [f32; 3]) {
        match self {
            BodyColour::GreyOrange => ([0.75, 0.75, 0.78], [1.0, 0.55, 0.1]),
            BodyColour::White => ([0.95, 0.95, 0.95], [0.95, 0.95, 0.95]),
            BodyColour::Blue => ([0.25, 0.45, 0.95], [0.25, 0.45, 0.95]),
            BodyColour::Red => ([0.85, 0.25, 0.25], [0.85, 0.25, 0.25]),
        }
    }
}

/// Planet detail presets for the LOD screen error (#776, S1).
///
/// Low trades sharpness for GPU work on weaker PCs; Normal is the spec
/// default (1.5 px at 1600x900); High is sharper with more patches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlanetDetail {
    /// Coarse: splits above 2 px, merges below 1 px.
    Low,
    /// Spec default: splits above 1.5 px, merges below 0.75 px.
    #[default]
    Normal,
    /// Fine: splits above 1 px, merges below 0.5 px.
    High,
}

impl PlanetDetail {
    /// Every preset in cycle order.
    pub const ALL: [PlanetDetail; 3] =
        [PlanetDetail::Low, PlanetDetail::Normal, PlanetDetail::High];

    /// Next preset when the setting is clicked.
    #[must_use]
    pub fn next(self) -> PlanetDetail {
        match self {
            PlanetDetail::Low => PlanetDetail::Normal,
            PlanetDetail::Normal => PlanetDetail::High,
            PlanetDetail::High => PlanetDetail::Low,
        }
    }

    /// Split threshold in pixels at 1600x900 (Q2 A, S1).
    #[must_use]
    pub fn detail_px(self) -> f64 {
        match self {
            PlanetDetail::Low => 2.0,
            PlanetDetail::Normal => 1.5,
            PlanetDetail::High => 1.0,
        }
    }

    /// Pixels one patch quad covers at the end of its level's range (#817
    /// F1 A): fewer pixels per quad means finer detail.
    #[must_use]
    pub fn quad_px(self) -> f64 {
        match self {
            PlanetDetail::Low => 16.0,
            PlanetDetail::Normal => 8.0,
            PlanetDetail::High => 4.0,
        }
    }

    /// File id (`planet_detail = <id>`).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            PlanetDetail::Low => "low",
            PlanetDetail::Normal => "normal",
            PlanetDetail::High => "high",
        }
    }

    /// Display name shown in Settings.
    #[must_use]
    pub fn display(self) -> &'static str {
        match self {
            PlanetDetail::Low => "Low",
            PlanetDetail::Normal => "Normal",
            PlanetDetail::High => "High",
        }
    }

    /// Preset from a file id, or `None` for anything else.
    #[must_use]
    pub fn from_id(id: &str) -> Option<PlanetDetail> {
        match id {
            "low" => Some(PlanetDetail::Low),
            "normal" => Some(PlanetDetail::Normal),
            "high" => Some(PlanetDetail::High),
            _ => None,
        }
    }
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
    /// Display > Scene: the on-screen mode reminder (#557).
    pub mode_reminder: bool,
    /// Display > Scene: the pull arrow and readout line in normal mode
    /// (#559, S1).
    pub show_pull: bool,
    /// Display > Scene: the player figure colours (#755, S1).
    pub body_colour: BodyColour,
    /// Display > Scene: LOD sharpness (#776, S1).
    pub planet_detail: PlanetDetail,
    /// Display > Scene: near shadows from the planet's sun (#780, S1).
    pub shadows: bool,
    /// Display > Window: fullscreen.
    pub fullscreen: bool,
    /// Display > Window: vsync.
    pub vsync: bool,
    /// Controls > Flight: free-flight speed.
    pub speed: FlightSpeed,
    /// Controls > Flight: how fast looking around turns (#557).
    pub look_sensitivity: LookSensitivity,
    /// Controls > Flight: how fast the dive orbits (#873 S1).
    pub dive_orbit_speed: DiveOrbitSpeed,
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
    /// Controls > Flight: gradual starts and stops in normal mode (#756, S2).
    /// Off gives today's instant control.
    pub movement_inertia: bool,
    /// Controls > Flight: mouse-look smoothing in normal mode (#756, S1).
    /// Off gives today's raw mouse.
    pub camera_smoothing: CameraSmoothing,
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
    /// Developer: LOD debug overlay shown (#776, S2, Q6 A).
    pub debug_overlay: bool,
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
            mode_reminder: true,
            show_pull: true,
            body_colour: BodyColour::default(),
            planet_detail: PlanetDetail::default(),
            shadows: true,
            fullscreen: false,
            vsync: true,
            speed: FlightSpeed::default(),
            look_sensitivity: LookSensitivity::default(),
            dive_orbit_speed: DiveOrbitSpeed::default(),
            invert_look: false,
            dive_toggle: false,
            look_toggle: false,
            plane_turning: false,
            start_third_person: true,
            movement_inertia: true,
            camera_smoothing: CameraSmoothing::default(),
            keymap: crate::keymap::Keymap::default(),
            touch_always: false,
            left_hand: false,
            views: [None; VIEW_SLOTS],
            debug_axes: false,
            debug_overlay: false,
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
                mode_reminder: base.mode_reminder,
                show_pull: base.show_pull,
                body_colour: base.body_colour,
                planet_detail: base.planet_detail,
                shadows: base.shadows,
                fullscreen: base.fullscreen,
                vsync: base.vsync,
                ..self
            },
            Category::Controls => Settings {
                speed: base.speed,
                look_sensitivity: base.look_sensitivity,
                dive_orbit_speed: base.dive_orbit_speed,
                invert_look: base.invert_look,
                dive_toggle: base.dive_toggle,
                look_toggle: base.look_toggle,
                plane_turning: base.plane_turning,
                start_third_person: base.start_third_person,
                movement_inertia: base.movement_inertia,
                camera_smoothing: base.camera_smoothing,
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
                debug_overlay: base.debug_overlay,
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
        push_line(&mut out, "mode_reminder", on_off(self.mode_reminder));
        push_line(&mut out, "show_pull", on_off(self.show_pull));
        push_line(&mut out, "body_colour", self.body_colour.id());
        push_line(&mut out, "planet_detail", self.planet_detail.id());
        push_line(&mut out, "shadows", on_off(self.shadows));
        push_line(&mut out, "fullscreen", on_off(self.fullscreen));
        push_line(&mut out, "vsync", on_off(self.vsync));
        push_line(&mut out, "speed", &self.speed.get().to_string());
        push_line(
            &mut out,
            "look_sensitivity",
            &self.look_sensitivity.get().to_string(),
        );
        push_line(
            &mut out,
            "dive_orbit_speed",
            &self.dive_orbit_speed.get().to_string(),
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
        push_line(&mut out, "movement_inertia", on_off(self.movement_inertia));
        push_line(&mut out, "camera_smoothing", self.camera_smoothing.id());
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
        push_line(&mut out, "debug_overlay", on_off(self.debug_overlay));
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
            "ground_guides" => {} // #770: removed, old files ignore it.
            "mode_reminder" => set(&mut self.mode_reminder, parse_on_off(value)),
            "show_pull" => set(&mut self.show_pull, parse_on_off(value)),
            "body_colour" => set(&mut self.body_colour, BodyColour::from_id(value)),
            "planet_detail" => set(&mut self.planet_detail, PlanetDetail::from_id(value)),
            "shadows" => set(&mut self.shadows, parse_on_off(value)),
            "look_sensitivity" => set(
                &mut self.look_sensitivity,
                value.parse().ok().and_then(LookSensitivity::new),
            ),
            "dive_orbit_speed" => set(
                &mut self.dive_orbit_speed,
                value.parse().ok().and_then(DiveOrbitSpeed::new),
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
            "movement_inertia" => set(&mut self.movement_inertia, parse_on_off(value)),
            "camera_smoothing" => {
                set(&mut self.camera_smoothing, CameraSmoothing::from_id(value));
            }
            "fullscreen" => set(&mut self.fullscreen, parse_on_off(value)),
            "vsync" => set(&mut self.vsync, parse_on_off(value)),
            "speed" => set(
                &mut self.speed,
                value.parse().ok().and_then(FlightSpeed::new),
            ),
            "debug_axes" => set(&mut self.debug_axes, parse_on_off(value)),
            "debug_overlay" => set(&mut self.debug_overlay, parse_on_off(value)),
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

/// `mode, x, y, z, yaw, pitch, speed, destination` (dive/normal) or
/// `mode, x, y, z, yaw, pitch, fx, fy, fz, ux, uy, uz, speed, destination`
/// (free, #884 T2) with the destination as `-` (none), `*` (empty chain),
/// or markers joined by `/`. Old free files wrote only yaw/pitch and still
/// load (forward derived from the look direction).
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
    match view.mode {
        ViewMode::Free => {
            let [fx, fy, fz] = view.free_forward;
            let [ux, uy, uz] = view.free_up;
            format!(
                "{mode}, {x}, {y}, {z}, {}, {}, {fx}, {fy}, {fz}, {ux}, {uy}, {uz}, {}, {destination}",
                view.yaw,
                view.pitch,
                view.speed.get()
            )
        }
        ViewMode::Dive | ViewMode::Normal => format!(
            "{mode}, {x}, {y}, {z}, {}, {}, {}, {destination}",
            view.yaw,
            view.pitch,
            view.speed.get()
        ),
    }
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
    // Collect the remaining numbers before the speed/destination: free
    // writes yaw/pitch plus forward + up (eight), old free files wrote
    // yaw/pitch (two), dive/normal write yaw/pitch (two).
    let rest: Vec<String> = parts.map(str::to_string).collect();
    // The tail is `..., speed, destination`; speed is the second-to-last.
    if rest.len() < 3 {
        return None;
    }
    let destination_str = rest.last()?.clone();
    let speed_str = rest.get(rest.len().saturating_sub(2))?.clone();
    let speed = FlightSpeed::new(speed_str.parse().ok()?)?;
    let numbers_str = rest.get(..rest.len().saturating_sub(2))?;
    let numbers: Option<Vec<f64>> = numbers_str
        .iter()
        .map(|s| s.parse::<f64>().ok().filter(|v| v.is_finite()))
        .collect();
    let numbers = numbers?;
    let (yaw, pitch, free_forward, free_up) = match (mode, numbers.len()) {
        (ViewMode::Free, 8) => {
            let yaw = *numbers.first()?;
            let pitch = *numbers.get(1)?;
            let ff = [*numbers.get(2)?, *numbers.get(3)?, *numbers.get(4)?];
            let fu = [*numbers.get(5)?, *numbers.get(6)?, *numbers.get(7)?];
            if !ff.iter().all(|c| c.is_finite()) || !fu.iter().all(|c| c.is_finite()) {
                return None;
            }
            (yaw, pitch, ff, fu)
        }
        (_, 2) => {
            let yaw = *numbers.first()?;
            let pitch = *numbers.get(1)?;
            // Old free files (yaw/pitch): forward from the look direction,
            // up perpendicular to it.
            let ff = crate::flight::free_look_direction(yaw, pitch);
            let fu = crate::flight::orthogonal_up(ff, [0.0, 1.0, 0.0]);
            (yaw, pitch, ff, fu)
        }
        _ => return None,
    };
    let destination = match destination_str.as_str() {
        "-" => None,
        "*" => Some(Destination::from_chain(&[])?),
        chain => {
            let markers: Option<Vec<u32>> = chain.split('/').map(|m| m.parse().ok()).collect();
            Some(Destination::from_chain(&markers?)?)
        }
    };
    Some(SavedPose {
        offset,
        destination,
        mode,
        yaw,
        pitch,
        free_forward,
        free_up,
        speed,
    })
}
