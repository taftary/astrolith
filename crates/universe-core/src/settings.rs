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
    /// Display > Window: fullscreen.
    pub fullscreen: bool,
    /// Display > Window: vsync.
    pub vsync: bool,
    /// Controls > Flight: free-flight speed.
    pub speed: FlightSpeed,
    /// Views: the eight slots.
    pub views: [Option<SavedPose>; VIEW_SLOTS],
    /// Developer: debug axes shown.
    pub debug_axes: bool,
}

impl Default for Settings {
    /// First-launch settings as documented in Spec v1 (#473).
    fn default() -> Settings {
        Settings {
            exposure: Exposure::default(),
            labels: true,
            label_limit: LabelLimit::default(),
            dimming: Dimming::default(),
            fullscreen: false,
            vsync: true,
            speed: FlightSpeed::default(),
            views: [None; VIEW_SLOTS],
            debug_axes: false,
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
                fullscreen: base.fullscreen,
                vsync: base.vsync,
                ..self
            },
            Category::Controls => Settings {
                speed: base.speed,
                ..self
            },
            Category::Views => Settings {
                views: base.views,
                ..self
            },
            Category::Accessibility => self,
            Category::Developer => Settings {
                debug_axes: base.debug_axes,
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
        push_line(&mut out, "fullscreen", on_off(self.fullscreen));
        push_line(&mut out, "vsync", on_off(self.vsync));
        push_line(&mut out, "speed", &self.speed.get().to_string());
        push_line(&mut out, "debug_axes", on_off(self.debug_axes));
        for (slot, view) in self.views.iter().enumerate() {
            if let Some(view) = view {
                push_line(&mut out, &format!("view{}", slot + 1), &format_view(view));
            }
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
            "fullscreen" => set(&mut self.fullscreen, parse_on_off(value)),
            "vsync" => set(&mut self.vsync, parse_on_off(value)),
            "speed" => set(
                &mut self.speed,
                value.parse().ok().and_then(FlightSpeed::new),
            ),
            "debug_axes" => set(&mut self.debug_axes, parse_on_off(value)),
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
        assert_eq!(s.speed.get(), 5);
        assert!(s.views.iter().all(Option::is_none));
        assert!(!s.debug_axes);
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
    fn round_trip_keeps_every_value() {
        let mut s = Settings::defaults();
        s.exposure = Exposure(2.4414062);
        s.labels = false;
        s.label_limit = LabelLimit(3);
        s.fullscreen = true;
        s.vsync = false;
        s.speed = FlightSpeed(9);
        s.debug_axes = true;
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
