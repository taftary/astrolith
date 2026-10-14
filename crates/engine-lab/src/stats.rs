//! The stats overlay (#640, AC11): FPS, frame time, triangles, meshes,
//! camera position, distance, height and the shell level of #632.

use crate::units::length_text;

/// Frames averaged for the FPS readout.
pub const WINDOW: usize = 60;

/// Rolling frame times and the overlay switch.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FrameTimes {
    seconds: Vec<f64>,
}

impl FrameTimes {
    /// Adds one frame's duration (seconds); keeps the last [`WINDOW`].
    pub fn push(&mut self, dt: f64) {
        if dt.is_finite() && dt > 0.0 {
            self.seconds.push(dt);
            if self.seconds.len() > WINDOW {
                self.seconds.remove(0);
            }
        }
    }

    /// Mean frame time in milliseconds (0 before any frame).
    #[must_use]
    pub fn frame_ms(&self) -> f64 {
        if self.seconds.is_empty() {
            return 0.0;
        }
        #[expect(clippy::cast_precision_loss, reason = "E-CAST: at most WINDOW frames")]
        let count = self.seconds.len() as f64;
        self.seconds.iter().sum::<f64>() / count * 1_000.0
    }

    /// Frames per second from the mean frame time.
    #[must_use]
    pub fn fps(&self) -> f64 {
        let ms = self.frame_ms();
        if ms > 0.0 { 1_000.0 / ms } else { 0.0 }
    }
}

/// What the overlay reads from the scene this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneReadout {
    /// Drawn surface triangles.
    pub triangles: usize,
    /// Drawn meshes.
    pub meshes: usize,
    /// Camera position, metres from the planet centre.
    pub position_m: [f64; 3],
    /// Height above the ground, metres.
    pub height_m: f64,
    /// Shell level shown (10-14).
    pub shell: u8,
}

/// The overlay lines; empty when the overlay is hidden.
#[must_use]
pub fn overlay_lines(shown: bool, times: &FrameTimes, scene: &SceneReadout) -> Vec<String> {
    if !shown {
        return Vec::new();
    }
    let [x, y, z] = scene.position_m;
    let distance = (x * x + y * y + z * z).sqrt();
    let level = universe_core::coords::Level::new(scene.shell)
        .map_or("-", universe_core::labels::level_label);
    vec![
        format!("FPS {:.0}  frame {:.1} ms", times.fps(), times.frame_ms()),
        format!("triangles {}  meshes {}", scene.triangles, scene.meshes),
        format!(
            "position {}, {}, {}",
            length_text(x),
            length_text(y),
            length_text(z)
        ),
        format!("distance {}", length_text(distance)),
        format!("height {}", length_text(scene.height_m)),
        format!("level {level}"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stats_overlay_reads_the_scene_and_hides() {
        // C12 / AC11.
        let mut times = FrameTimes::default();
        for _ in 0..(WINDOW + 10) {
            times.push(1.0 / 50.0);
        }
        times.push(f64::NAN);
        assert!((times.fps() - 50.0).abs() < 1e-9);
        assert!((times.frame_ms() - 20.0).abs() < 1e-9);
        let scene = SceneReadout {
            triangles: 6_000,
            meshes: 3,
            position_m: [0.0, 6_371_500.0, 0.0],
            height_m: 500.0,
            shell: 13,
        };
        let lines = overlay_lines(true, &times, &scene);
        assert_eq!(lines[0], "FPS 50  frame 20.0 ms");
        assert_eq!(lines[1], "triangles 6000  meshes 3");
        assert!(lines[2].starts_with("position 0 m, 6,372 km"));
        assert_eq!(lines[3], "distance 6,372 km");
        assert_eq!(lines[4], "height 500 m");
        assert_eq!(lines[5], "level L13 - Buildings");
        assert!(overlay_lines(false, &times, &scene).is_empty(), "hidden");
        assert_eq!(FrameTimes::default().fps(), 0.0);
    }
}
