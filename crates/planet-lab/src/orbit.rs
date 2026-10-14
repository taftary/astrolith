//! The lab's two cameras: orbit with preset views, and free-fly (#640, Q6).
//!
//! The orbit camera turns around a target (the planet centre by default)
//! at a distance, with the seven Blender-style preset views and Home (the
//! whole planet and its moons in view). Free-fly is the game's free flight
//! (WASD, look, Shift+1-9 speed steps) whose speed follows the height above
//! the ground, so you can go down to the ground as in the game. Everything
//! is in the scene's L10 cell units; pure and deterministic.

use universe_core::flight::{free_look_direction, free_speed_multiplier};

/// Look direction for `yaw` and `pitch` with no pitch limit (the preset
/// views look straight down and up): yaw zero faces `-Z`, as in the game.
#[must_use]
pub fn look_direction(yaw: f64, pitch: f64) -> [f64; 3] {
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    [-sin_yaw * cos_pitch, sin_pitch, -cos_yaw * cos_pitch]
}

/// The lab's field of view, radians (the game's 45 degrees).
pub const FOV: f64 = std::f64::consts::FRAC_PI_4;

/// Home framing: the scene radius kept in view, cell units (the moons'
/// display shells reach about 0.49).
pub const HOME_RADIUS: f64 = 0.5;

/// Home yaw and pitch, radians: a three-quarter view from above.
pub const HOME_YAW: f64 = 0.5;
/// Home pitch, radians (looking slightly down).
pub const HOME_PITCH: f64 = -0.35;

/// Closest the orbit camera comes to its target, cell units.
pub const MIN_DISTANCE: f64 = 1e-9;

/// Which camera drives the view (switched with F).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CameraMode {
    /// Orbit around the target.
    #[default]
    Orbit,
    /// The game's free flight.
    Free,
}

/// The seven preset views (Numpad 7/1/3 with Ctrl, and 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// Looking down the -Y axis (Numpad 7).
    Top,
    /// Looking up the +Y axis (Ctrl+Numpad 7).
    Bottom,
    /// Looking along -Z (Numpad 1).
    Front,
    /// Looking along +Z (Ctrl+Numpad 1).
    Back,
    /// Looking along -X (Numpad 3).
    Right,
    /// Looking along +X (Ctrl+Numpad 3).
    Left,
    /// The Home three-quarter view (Numpad 0).
    Perspective,
}

impl Preset {
    /// Every preset, in menu order.
    pub const ALL: [Preset; 7] = [
        Preset::Top,
        Preset::Bottom,
        Preset::Front,
        Preset::Back,
        Preset::Right,
        Preset::Left,
        Preset::Perspective,
    ];

    /// Menu label with its key.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Preset::Top => "Top (Numpad 7)",
            Preset::Bottom => "Bottom (Ctrl+Numpad 7)",
            Preset::Front => "Front (Numpad 1)",
            Preset::Back => "Back (Ctrl+Numpad 1)",
            Preset::Right => "Right (Numpad 3)",
            Preset::Left => "Left (Ctrl+Numpad 3)",
            Preset::Perspective => "Perspective (Numpad 0)",
        }
    }

    /// Yaw and pitch of the preset, radians.
    #[must_use]
    pub const fn angles(self) -> (f64, f64) {
        use std::f64::consts::{FRAC_PI_2, PI};
        // Just shy of straight up or down keeps the up vector defined.
        let pole = FRAC_PI_2 - 1e-6;
        match self {
            Preset::Top => (0.0, -pole),
            Preset::Bottom => (0.0, pole),
            Preset::Front => (0.0, 0.0),
            Preset::Back => (PI, 0.0),
            Preset::Right => (FRAC_PI_2, 0.0),
            Preset::Left => (-FRAC_PI_2, 0.0),
            Preset::Perspective => (HOME_YAW, HOME_PITCH),
        }
    }
}

/// The orbit camera: angles around the target and the distance to it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitCamera {
    /// Turn around the vertical, radians.
    pub yaw: f64,
    /// Tilt above (+) or below (-) the horizon, radians.
    pub pitch: f64,
    /// Distance to the target, cell units.
    pub distance: f64,
    /// The point turned around, cell units.
    pub target: [f64; 3],
}

impl Default for OrbitCamera {
    fn default() -> OrbitCamera {
        OrbitCamera::home()
    }
}

impl OrbitCamera {
    /// The Home view: the whole planet and its moons in view (AC2, AC7).
    #[must_use]
    pub fn home() -> OrbitCamera {
        OrbitCamera {
            yaw: HOME_YAW,
            pitch: HOME_PITCH,
            distance: HOME_RADIUS / (FOV * 0.5).sin() * 1.1,
            target: [0.0; 3],
        }
    }

    /// The camera's looking direction (unit).
    #[must_use]
    pub fn forward(&self) -> [f64; 3] {
        look_direction(self.yaw, self.pitch)
    }

    /// The camera position, cell units.
    #[must_use]
    pub fn position(&self) -> [f64; 3] {
        let f = self.forward();
        [
            self.target[0] - f[0] * self.distance,
            self.target[1] - f[1] * self.distance,
            self.target[2] - f[2] * self.distance,
        ]
    }

    /// Left-drag: turn around the target by `dx`, `dy` radians; the pitch
    /// stops just short of the poles.
    pub fn orbit(&mut self, dx: f64, dy: f64) {
        let pole = std::f64::consts::FRAC_PI_2 - 1e-6;
        self.yaw = (self.yaw + dx).rem_euclid(std::f64::consts::TAU);
        self.pitch = (self.pitch + dy).clamp(-pole, pole);
    }

    /// Wheel: the distance shrinks by `factor` (< 1 in, > 1 out).
    pub fn zoom(&mut self, factor: f64) {
        if factor.is_finite() && factor > 0.0 {
            self.distance = (self.distance * factor).max(MIN_DISTANCE);
        }
    }

    /// Middle-drag: moves the target across the view by `dx`, `dy` shares
    /// of the distance.
    pub fn pan(&mut self, dx: f64, dy: f64) {
        let f = self.forward();
        let right = unit(cross(f, [0.0, 1.0, 0.0])).unwrap_or([1.0, 0.0, 0.0]);
        let up = cross(right, f);
        for ((target, r), u) in self.target.iter_mut().zip(right).zip(up) {
            *target += (r * dx + u * dy) * self.distance;
        }
    }

    /// Jumps to `preset`, keeping the distance, the target back on the
    /// planet centre.
    pub fn preset(&mut self, preset: Preset) {
        let (yaw, pitch) = preset.angles();
        self.yaw = yaw;
        self.pitch = pitch;
        self.target = [0.0; 3];
    }
}

/// One free-fly step (the game's free flight, #152): the held keys move
/// the camera in the look frame at the height above the ground times the
/// speed step, so the speed slows near the ground and you can land.
#[must_use]
pub fn free_fly_step(
    camera: [f64; 3],
    yaw: f64,
    pitch: f64,
    keys: [bool; 4],
    speed_step: u32,
    height_units: f64,
    dt: f64,
) -> [f64; 3] {
    let [forward_key, back, left, right_key] = keys;
    let forward = free_look_direction(yaw, pitch);
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let right = [cos_yaw, 0.0, -sin_yaw];
    let mut wish = [0.0; 3];
    let mut add = |v: [f64; 3], s: f64| {
        for (w, x) in wish.iter_mut().zip(v) {
            *w += x * s;
        }
    };
    if forward_key {
        add(forward, 1.0);
    }
    if back {
        add(forward, -1.0);
    }
    if right_key {
        add(right, 1.0);
    }
    if left {
        add(right, -1.0);
    }
    let Some(direction) = unit(wish) else {
        return camera;
    };
    let gap = height_units.abs().max(1e-10);
    let travelled = gap * free_speed_multiplier(speed_step) * dt;
    if !travelled.is_finite() || travelled <= 0.0 {
        return camera;
    }
    [
        camera[0] + direction[0] * travelled,
        camera[1] + direction[1] * travelled,
        camera[2] + direction[2] * travelled,
    ]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let length = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    (length.is_finite() && length > 1e-300).then(|| [a[0] / length, a[1] / length, a[2] / length])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn length(a: [f64; 3]) -> f64 {
        (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
    }

    #[test]
    fn the_orbit_camera_and_preset_views_frame_the_planet() {
        // C8 / AC7: Home frames the planet and its moons; orbit, zoom and
        // pan move as expected; every preset looks along its axis.
        let home = OrbitCamera::home();
        let seen = (FOV * 0.5).sin() * length(home.position());
        assert!(seen >= HOME_RADIUS, "Home keeps the moons in view");
        let mut camera = home;
        camera.orbit(0.3, 0.1);
        assert!((camera.yaw - (HOME_YAW + 0.3)).abs() < 1e-12);
        assert!(
            (length(camera.position()) - home.distance).abs() < 1e-12,
            "orbit keeps the distance"
        );
        camera.zoom(0.5);
        assert!((camera.distance - home.distance * 0.5).abs() < 1e-12);
        camera.pan(0.1, 0.0);
        assert!(length(camera.target) > 0.0, "pan moves the target");
        let looks = [
            (Preset::Top, [0.0, -1.0, 0.0]),
            (Preset::Bottom, [0.0, 1.0, 0.0]),
            (Preset::Front, [0.0, 0.0, -1.0]),
            (Preset::Back, [0.0, 0.0, 1.0]),
            (Preset::Right, [-1.0, 0.0, 0.0]),
            (Preset::Left, [1.0, 0.0, 0.0]),
        ];
        for (preset, want) in looks {
            camera.preset(preset);
            let f = camera.forward();
            let along = f[0] * want[0] + f[1] * want[1] + f[2] * want[2];
            assert!(along > 0.999, "{preset:?} looks along {want:?}: {f:?}");
            assert_eq!(camera.target, [0.0; 3], "{preset:?} recentres");
        }
        camera.preset(Preset::Perspective);
        assert_eq!((camera.yaw, camera.pitch), (HOME_YAW, HOME_PITCH));
        assert_eq!(Preset::ALL.len(), 7);
        camera.zoom(0.0);
        camera.zoom(f64::NAN);
        assert!(camera.distance > 0.0, "bad zooms are ignored");
    }

    #[test]
    fn free_fly_reaches_the_ground_through_every_shell() {
        // C9 / AC8: flying down with W from orbit, the speed follows the
        // height, the camera never crosses the ground, and the shell level
        // reads L11, L12, L13 and L14 on the way.
        let mut universe = crate::scene::LabScene::default_home().universe();
        let ground = universe_core::terrain::surface_radius([0.0, 1.0, 0.0]);
        let start = 2.0e6 / crate::units::metres_per_unit();
        universe.path.set_offset([0.0, ground + start, 0.0]);
        let down = -std::f64::consts::FRAC_PI_2;
        let mut seen = vec![universe.shown_level().get()];
        for _ in 0..20_000 {
            let height_m = universe
                .path
                .height_above_ground_m()
                .expect("on the planet");
            assert!(height_m > 0.0, "never under the ground: {height_m} m");
            if height_m < 5.0 {
                break;
            }
            let height = height_m / crate::units::metres_per_unit();
            let next = free_fly_step(
                universe.path.offset(),
                0.0,
                down,
                [true, false, false, false],
                5,
                height,
                1.0 / 60.0,
            );
            universe.path.set_offset(next);
            let now = universe.shown_level().get();
            if seen.last() != Some(&now) {
                seen.push(now);
            }
        }
        assert_eq!(
            seen,
            vec![10, 11, 12, 13, 14],
            "every shell on the way down"
        );
        let still = free_fly_step([0.0, 1.0, 0.0], 0.0, 0.0, [false; 4], 5, 1.0, 0.1);
        assert_eq!(still, [0.0, 1.0, 0.0], "no keys, no move");
    }
}
