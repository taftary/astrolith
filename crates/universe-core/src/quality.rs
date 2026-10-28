//! Effective graphics quality: what Auto draws now (#1022 T5, Spec v4
//! points 1-5 and 8, R1-R3).
//!
//! Pure. The controller ([`crate::quality_controller`]) moves it one step
//! of one control at a time; the renderer applies it (T6-T9). It is never
//! saved: the player's saved choices live in [`crate::settings_quality`]
//! (point 10).

use crate::drawn_set::MarkerDetail;
use crate::settings_quality::{PlanetDetail, Tier};

/// One resolution step: 10 percentage points of native (point 5).
pub const RES_STEP: f64 = 0.1;

/// The picture floor: physical pixels on the viewport's shorter side, or
/// native when that is smaller (point 3).
pub const FLOOR_SHORT_SIDE: u32 = 720;

/// A control Auto can step (point 1, R2 loading work).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Control {
    /// The internal 3D resolution.
    Resolution,
    /// The terrain loading-work budget (5 / 10 / 15 % of the target
    /// interval, R2); a loading limit, not a graphics ceiling.
    TerrainWork,
    /// Terrain detail.
    Terrain,
    /// Marker detail.
    Markers,
    /// Atmosphere quality.
    Atmosphere,
    /// Near shadows.
    Shadows,
    /// Bloom.
    Bloom,
}

impl Control {
    /// Every control, in index order.
    pub const ALL: [Control; 7] = [
        Control::Resolution,
        Control::TerrainWork,
        Control::Terrain,
        Control::Markers,
        Control::Atmosphere,
        Control::Shadows,
        Control::Bloom,
    ];

    /// Recovery priority (point 8): resolution, terrain (its loading work,
    /// then its detail), markers, atmosphere, shadows, then bloom.
    pub const RECOVERY: [Control; 7] = Self::ALL;

    /// Reduction order for an unknown bottleneck (R1): resolution, the
    /// terrain-work budget, Marker detail, terrain detail, shadows, bloom,
    /// then atmosphere.
    pub const REDUCTION: [Control; 7] = [
        Control::Resolution,
        Control::TerrainWork,
        Control::Markers,
        Control::Terrain,
        Control::Shadows,
        Control::Bloom,
        Control::Atmosphere,
    ];

    /// Position in [`Control::ALL`].
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Name in logs and evidence.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Control::Resolution => "resolution",
            Control::TerrainWork => "terrain-work",
            Control::Terrain => "terrain",
            Control::Markers => "markers",
            Control::Atmosphere => "atmosphere",
            Control::Shadows => "shadows",
            Control::Bloom => "bloom",
        }
    }
}

/// Which way a step goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Lower quality, less work.
    Down,
    /// Higher quality, more work.
    Up,
}

impl Direction {
    /// The other way.
    #[must_use]
    pub const fn reverse(self) -> Direction {
        match self {
            Direction::Down => Direction::Up,
            Direction::Up => Direction::Down,
        }
    }
}

/// Where a frame's time goes, when reliable diagnostics say so (R1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Bottleneck {
    /// Not known: the plain reduction order.
    #[default]
    Unknown,
    /// Drawing pixels: resolution first.
    Pixels,
    /// Terrain processing: loading work before terrain detail.
    TerrainProcessing,
    /// Marker work: Marker detail first.
    Markers,
}

impl Bottleneck {
    /// The reduction order: the controls this bottleneck names first, then
    /// the rest in [`Control::REDUCTION`] order.
    #[must_use]
    pub fn reduction_order(self) -> [Control; 7] {
        let first: &[Control] = match self {
            Bottleneck::Unknown => &[],
            Bottleneck::Pixels => &[Control::Resolution],
            Bottleneck::TerrainProcessing => &[Control::TerrainWork, Control::Terrain],
            Bottleneck::Markers => &[Control::Markers],
        };
        let mut order = Control::REDUCTION;
        let rest = Control::REDUCTION
            .into_iter()
            .filter(|c| !first.contains(c));
        for (slot, c) in order.iter_mut().zip(first.iter().copied().chain(rest)) {
            *slot = c;
        }
        order
    }
}

/// What bounds the quality: the Auto ceiling, the picture floor and the
/// ground's memory pressure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    /// The Auto ceiling (point 2).
    pub ceiling: Tier,
    /// The lowest resolution scale, from [`floor_scale`].
    pub floor_scale: f64,
    /// The ground is short of memory: no terrain recovery and no more
    /// loading work (R2).
    pub memory_pressure: bool,
}

impl Limits {
    /// Limits for `ceiling` on a viewport whose shorter side is
    /// `native_short` physical pixels, with no memory pressure.
    #[must_use]
    pub fn new(ceiling: Tier, native_short: u32) -> Limits {
        Limits {
            ceiling,
            floor_scale: floor_scale(native_short),
            memory_pressure: false,
        }
    }

    /// Resolution steps from native down to the floor.
    #[must_use]
    pub fn res_steps_max(&self) -> u8 {
        let steps = ((1.0 - self.floor_scale) / RES_STEP - 1e-9).ceil().max(0.0);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "at most ten steps, never negative"
        )]
        let steps = steps.min(10.0) as u8;
        steps
    }
}

/// The lowest resolution scale for a viewport whose shorter side is
/// `native_short` physical pixels: 720 pixels, or native when smaller.
#[must_use]
pub fn floor_scale(native_short: u32) -> f64 {
    if native_short <= FLOOR_SHORT_SIDE {
        1.0
    } else {
        f64::from(FLOOR_SHORT_SIDE) / f64::from(native_short)
    }
}

/// The 3D picture size for a `native` viewport at `scale`: uniform and
/// rounded, never under the floor's short side nor above native (point 3).
#[must_use]
pub fn scaled_size(native: (u32, u32), scale: f64) -> (u32, u32) {
    let short = native.0.min(native.1);
    let floor_short = short.min(FLOOR_SHORT_SIDE);
    let scale = scale.clamp(0.0, 1.0);
    let at = |s: f64, side: u32| -> u32 {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a scaled pixel count within the native size"
        )]
        let px = (f64::from(side) * s).round().max(1.0) as u32;
        px.min(side)
    };
    let size = (at(scale, native.0), at(scale, native.1));
    if size.0.min(size.1) >= floor_short {
        return size;
    }
    let s = f64::from(floor_short) / f64::from(short.max(1));
    let up = |side: u32| -> u32 {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a scaled pixel count within the native size"
        )]
        let px = (f64::from(side) * s).ceil() as u32;
        px.min(side)
    };
    (up(native.0), up(native.1))
}

/// The terrain preset of a tier (R3: the existing Low / Normal / High).
#[must_use]
pub const fn planet_detail(t: Tier) -> PlanetDetail {
    match t {
        Tier::Low => PlanetDetail::Low,
        Tier::Normal => PlanetDetail::Normal,
        Tier::High => PlanetDetail::High,
    }
}

/// The tier of a terrain preset.
#[must_use]
pub const fn planet_tier(d: PlanetDetail) -> Tier {
    match d {
        PlanetDetail::Low => Tier::Low,
        PlanetDetail::Normal => Tier::Normal,
        PlanetDetail::High => Tier::High,
    }
}

/// The marker budget of a tier (R3: #998's 500 / 2,000 / 5,000).
#[must_use]
pub const fn marker_detail(t: Tier) -> MarkerDetail {
    match t {
        Tier::Low => MarkerDetail::Low,
        Tier::Normal => MarkerDetail::Normal,
        Tier::High => MarkerDetail::High,
    }
}

/// The tier of a marker budget.
#[must_use]
pub const fn marker_tier(d: MarkerDetail) -> Tier {
    match d {
        MarkerDetail::Low => Tier::Low,
        MarkerDetail::Normal => Tier::Normal,
        MarkerDetail::High => Tier::High,
    }
}

/// The atmosphere's sky-table and raymarch samples for a tier (R3): Low
/// 6 / 4, Normal 10 / 8 (today's), High 16 / 12. The table sizes stay the
/// same at every tier, so a change never reallocates them.
#[must_use]
pub const fn atmosphere_samples(t: Tier) -> (u32, u32) {
    match t {
        Tier::Low => (6, 4),
        Tier::Normal => (10, 8),
        Tier::High => (16, 12),
    }
}

/// The tier under `t`, if any.
const fn lower(t: Tier) -> Option<Tier> {
    match t {
        Tier::Low => None,
        Tier::Normal => Some(Tier::Low),
        Tier::High => Some(Tier::Normal),
    }
}

/// The tier above `t`, if any.
const fn higher(t: Tier) -> Option<Tier> {
    match t {
        Tier::Low => Some(Tier::Normal),
        Tier::Normal => Some(Tier::High),
        Tier::High => None,
    }
}

/// What is drawn now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectiveQuality {
    /// Resolution steps below native (0 is native).
    pub res_steps: u8,
    /// Terrain loading-work tier: Low 5 %, Normal 10 %, High 15 % of the
    /// target interval (R2).
    pub terrain_work: Tier,
    /// Terrain detail.
    pub terrain: Tier,
    /// Marker detail.
    pub markers: Tier,
    /// Atmosphere quality.
    pub atmosphere: Tier,
    /// Near shadows on.
    pub shadows: bool,
    /// Bloom on.
    pub bloom: bool,
}

impl EffectiveQuality {
    /// Where Auto starts (point 4): the resolution floor, Low detail,
    /// effects off, and the middle loading work (R2: start at 10 %).
    #[must_use]
    pub fn floor(limits: &Limits) -> EffectiveQuality {
        EffectiveQuality {
            res_steps: limits.res_steps_max(),
            terrain_work: Tier::Normal,
            terrain: Tier::Low,
            markers: Tier::Low,
            atmosphere: Tier::Low,
            shadows: false,
            bloom: false,
        }
    }

    /// The resolution scale under `limits`: 10 points a step, never under
    /// the floor.
    #[must_use]
    pub fn scale(&self, limits: &Limits) -> f64 {
        (1.0 - RES_STEP * f64::from(self.res_steps)).max(limits.floor_scale)
    }

    /// The loading-work budget as a share of the target interval (R2).
    #[must_use]
    pub const fn terrain_work_share(&self) -> f64 {
        match self.terrain_work {
            Tier::Low => 0.05,
            Tier::Normal => 0.10,
            Tier::High => 0.15,
        }
    }

    /// Whether `control` can take a step `dir` under `limits` (pure).
    #[must_use]
    pub fn can_step(&self, control: Control, dir: Direction, limits: &Limits) -> bool {
        let effects = limits.ceiling != Tier::Low;
        let tier_up = |t: Tier| t < limits.ceiling;
        match (control, dir) {
            (Control::Resolution, Direction::Down) => self.res_steps < limits.res_steps_max(),
            (Control::Resolution, Direction::Up) => self.res_steps > 0,
            (Control::TerrainWork, Direction::Down) => self.terrain_work > Tier::Low,
            (Control::TerrainWork, Direction::Up) => {
                self.terrain_work < Tier::High && !limits.memory_pressure
            }
            (Control::Terrain, Direction::Down) => self.terrain > Tier::Low,
            (Control::Terrain, Direction::Up) => tier_up(self.terrain) && !limits.memory_pressure,
            (Control::Markers, Direction::Down) => self.markers > Tier::Low,
            (Control::Markers, Direction::Up) => tier_up(self.markers),
            (Control::Atmosphere, Direction::Down) => self.atmosphere > Tier::Low,
            (Control::Atmosphere, Direction::Up) => tier_up(self.atmosphere),
            (Control::Shadows, Direction::Down) => self.shadows,
            (Control::Shadows, Direction::Up) => !self.shadows && effects,
            (Control::Bloom, Direction::Down) => self.bloom,
            (Control::Bloom, Direction::Up) => !self.bloom && effects,
        }
    }

    /// Takes one step of `control` in `dir`; a step at its end does
    /// nothing. Callers check [`EffectiveQuality::can_step`] first.
    pub fn step(&mut self, control: Control, dir: Direction) {
        let tier = |t: &mut Tier| {
            let next = match dir {
                Direction::Down => lower(*t),
                Direction::Up => higher(*t),
            };
            *t = next.unwrap_or(*t);
        };
        let on = dir == Direction::Up;
        match control {
            Control::Resolution => {
                self.res_steps = match dir {
                    Direction::Down => self.res_steps.saturating_add(1),
                    Direction::Up => self.res_steps.saturating_sub(1),
                };
            }
            Control::TerrainWork => tier(&mut self.terrain_work),
            Control::Terrain => tier(&mut self.terrain),
            Control::Markers => tier(&mut self.markers),
            Control::Atmosphere => tier(&mut self.atmosphere),
            Control::Shadows => self.shadows = on,
            Control::Bloom => self.bloom = on,
        }
    }

    /// The same quality inside `limits`: detail no higher than the
    /// ceiling, effects off under a Low ceiling, resolution no lower than
    /// the floor (point 9: a reduced ceiling applies at once).
    #[must_use]
    pub fn clamped(mut self, limits: &Limits) -> EffectiveQuality {
        self.terrain = self.terrain.min(limits.ceiling);
        self.markers = self.markers.min(limits.ceiling);
        self.atmosphere = self.atmosphere.min(limits.ceiling);
        if limits.ceiling == Tier::Low {
            self.shadows = false;
            self.bloom = false;
        }
        self.res_steps = self.res_steps.min(limits.res_steps_max());
        self
    }

    /// No control can step down: the picture floor.
    #[must_use]
    pub fn at_floor(&self, limits: &Limits) -> bool {
        Control::ALL
            .into_iter()
            .all(|c| !self.can_step(c, Direction::Down, limits))
    }

    /// Some control could still step up under `limits`: what holds it
    /// back is performance, not the ceiling or memory.
    #[must_use]
    pub fn below_ceiling(&self, limits: &Limits) -> bool {
        Control::ALL
            .into_iter()
            .any(|c| self.can_step(c, Direction::Up, limits))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_floor_is_720_pixels_on_the_shorter_side() {
        // 1600 x 900: the floor is 80 %, two steps below native.
        let limits = Limits::new(Tier::Normal, 900);
        assert!((limits.floor_scale - 0.8).abs() < 1e-12);
        assert_eq!(limits.res_steps_max(), 2);
        // 1920 x 1080: 66.7 %, four steps, the last one clamped.
        let limits = Limits::new(Tier::Normal, 1080);
        assert_eq!(limits.res_steps_max(), 4);
        let q = EffectiveQuality::floor(&limits);
        assert!((q.scale(&limits) - 720.0 / 1080.0).abs() < 1e-12);
        assert_eq!(scaled_size((1920, 1080), q.scale(&limits)), (1280, 720));
        // Under the floor: native only.
        assert_eq!(Limits::new(Tier::Normal, 600).res_steps_max(), 0);
        assert_eq!(scaled_size((1600, 900), 0.8), (1280, 720));
        assert_eq!(scaled_size((1600, 900), 0.5), (1280, 720), "never under");
        assert_eq!(scaled_size((1600, 900), 1.5), (1600, 900), "never above");
        assert_eq!(scaled_size((800, 600), 0.5), (800, 600));
    }

    #[test]
    fn ceilings_bound_detail_and_effects() {
        let low = Limits::new(Tier::Low, 900);
        let q = EffectiveQuality::floor(&low);
        // Auto starts at the middle loading work, which can still drop.
        assert!(!q.at_floor(&low));
        assert!(q.can_step(Control::TerrainWork, Direction::Down, &low));
        for c in [Control::Terrain, Control::Shadows, Control::Bloom] {
            assert!(!q.can_step(c, Direction::Up, &low), "{}", c.name());
        }
        assert!(q.can_step(Control::Resolution, Direction::Up, &low));
        let high = Limits::new(Tier::High, 900);
        let best = top(&high);
        assert_eq!(best.terrain, Tier::High);
        assert!(best.shadows && best.bloom && best.res_steps == 0);
        let normal = Limits::new(Tier::Normal, 900);
        let clamped = best.clamped(&normal);
        assert_eq!(
            (clamped.terrain, clamped.markers),
            (Tier::Normal, Tier::Normal)
        );
        assert!(clamped.shadows, "a Normal ceiling permits the effects");
        assert!(!best.clamped(&low).bloom);
        let pressed = Limits {
            memory_pressure: true,
            ..normal
        };
        let q = EffectiveQuality::floor(&normal);
        assert!(!q.can_step(Control::Terrain, Direction::Up, &pressed));
        assert!(!q.can_step(Control::TerrainWork, Direction::Up, &pressed));
        assert!(q.below_ceiling(&pressed), "resolution can still climb");
        assert!(!top(&normal).below_ceiling(&normal));
    }

    /// The best quality `limits` permit.
    fn top(limits: &Limits) -> EffectiveQuality {
        let mut q = EffectiveQuality::floor(limits);
        for c in Control::ALL {
            while q.can_step(c, Direction::Up, limits) {
                q.step(c, Direction::Up);
            }
        }
        q
    }

    #[test]
    fn tiers_map_to_the_existing_presets() {
        for t in Tier::ALL {
            assert_eq!(planet_tier(planet_detail(t)), t);
            assert_eq!(marker_tier(marker_detail(t)), t);
        }
        assert_eq!(marker_detail(Tier::Low).budget(), 500);
        assert_eq!(marker_detail(Tier::High).budget(), 5_000);
        assert_eq!(atmosphere_samples(Tier::Normal), (10, 8), "today's");
        assert_eq!(atmosphere_samples(Tier::Low), (6, 4));
        assert_eq!(atmosphere_samples(Tier::High), (16, 12));
    }

    #[test]
    fn bottleneck_hints_lead_the_reduction_order() {
        assert_eq!(Bottleneck::Unknown.reduction_order(), Control::REDUCTION);
        assert_eq!(Bottleneck::Pixels.reduction_order()[0], Control::Resolution);
        let terrain = Bottleneck::TerrainProcessing.reduction_order();
        assert_eq!(terrain[..2], [Control::TerrainWork, Control::Terrain]);
        assert_eq!(terrain[2], Control::Resolution);
        let markers = Bottleneck::Markers.reduction_order();
        assert_eq!(markers[..2], [Control::Markers, Control::Resolution]);
        for order in [terrain, markers] {
            for c in Control::ALL {
                assert_eq!(order.iter().filter(|o| **o == c).count(), 1);
            }
        }
    }
}
