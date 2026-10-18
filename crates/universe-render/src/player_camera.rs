//! One function for the player's camera (#754).
//!
//! [`player_camera`] gives the camera pose for every player case: gliding to
//! the body, first person, third person, the handover ease, and released
//! (dive and free keep their own rules). [`sync_camera`] only applies its
//! result; it no longer calls `normal_mode::camera_pose`.
//!
//! The handover keeps the view still: on takeover the look pitch starts at
//! the arrival tilt ([`arrival_tilt`], about -11.3 degrees), so the view
//! direction already matches the arrival, and the position eases from the
//! arrival point to the body's own target over [`HANDOVER_S`] with
//! smoothstep. The first body frame equals the last arrival frame (AC2), and
//! nothing cuts. Only the position eases; pitch and yaw stay under mouse
//! control from the first frame. Everything here is pure (`E-DET-TIERS`)
//! and `pub(crate)`.

use universe_core::player::{THIRD_BACK, THIRD_UP};

/// Handover ease time, seconds: the camera moves from the arrival point to
/// the body's own target in about 0.4 s.
pub(crate) const HANDOVER_S: f64 = 0.4;

/// Look pitch that matches the arrival view, radians.
///
/// The arrival looks from behind and above at the eye; tilting the body look
/// down by `atan(UP / BACK)` points the same way.
#[must_use]
pub(crate) fn arrival_tilt() -> f64 {
    -(THIRD_UP / THIRD_BACK).atan()
}

/// Smoothstep easing for the handover position: 0 at 0, 1 at 1, smooth in
/// between. Pure.
#[must_use]
pub(crate) fn smoothstep(t: f64) -> f64 {
    let clamped = t.clamp(0.0, 1.0);
    clamped * clamped * (3.0 - 2.0 * clamped)
}

/// The player's camera case (#754 AC3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CameraCase {
    /// Gliding to the body in dive mode: toward the arrival pose, looking at
    /// the eye.
    Gliding,
    /// First person in normal mode: from the eye.
    First,
    /// Third person in normal mode: behind and above the eye.
    Third,
    /// Handover ease after takeover: from the arrival point to the body's own
    /// target, direction already matches.
    Handover,
    /// Back to dive or free: this function returns `None`, the dive and free
    /// rules keep the camera.
    Released,
}

/// One function for the player's camera pose (#754 AC3).
///
/// Returns `(position, forward, up)` in open-cell units, or `None` for
/// [`CameraCase::Released`]. Pure and allocation-free (`E-HOT-NOALLOC`).
///
/// - `eye`, `ahead` (level forward from the yaw), `up`, `height` (one body
///   height in cell units), and `pitch` describe the body.
/// - `third_person` picks the handover target (third or first); `First` and
///   `Third` ignore it.
/// - `handover_from` is the camera point at takeover (the arrival point after
///   a glide) and `handover_t` seconds since (`0.0` on the first body frame).
/// - `arrival_camera` and `arrival_eye` are the arrival pose (glide target).
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "one pose's full inputs; a struct would only rename them"
)]
pub(crate) fn player_camera(
    case: CameraCase,
    eye: [f64; 3],
    ahead: [f64; 3],
    up: [f64; 3],
    height: f64,
    pitch: f64,
    third_person: bool,
    handover_from: [f64; 3],
    handover_t: f64,
    arrival_camera: [f64; 3],
    arrival_eye: [f64; 3],
) -> Option<([f64; 3], [f64; 3], [f64; 3])> {
    let (sin, cos) = pitch.sin_cos();
    let forward = [
        ahead[0] * cos + up[0] * sin,
        ahead[1] * cos + up[1] * sin,
        ahead[2] * cos + up[2] * sin,
    ];
    match case {
        CameraCase::Gliding => {
            let direction = [
                arrival_eye[0] - arrival_camera[0],
                arrival_eye[1] - arrival_camera[1],
                arrival_eye[2] - arrival_camera[2],
            ];
            let length = (direction[0] * direction[0]
                + direction[1] * direction[1]
                + direction[2] * direction[2])
                .sqrt();
            let look = if length.is_finite() && length > 0.0 {
                [
                    direction[0] / length,
                    direction[1] / length,
                    direction[2] / length,
                ]
            } else {
                forward
            };
            Some((arrival_camera, look, up))
        }
        CameraCase::First => Some((eye, forward, up)),
        CameraCase::Third => {
            let position = universe_core::player::third_person_camera(eye, forward, up, height);
            Some((position, forward, up))
        }
        CameraCase::Handover => {
            let target = if third_person {
                universe_core::player::third_person_camera(eye, forward, up, height)
            } else {
                eye
            };
            let share = smoothstep(handover_t / HANDOVER_S);
            let position = [
                handover_from[0] + (target[0] - handover_from[0]) * share,
                handover_from[1] + (target[1] - handover_from[1]) * share,
                handover_from[2] + (target[2] - handover_from[2]) * share,
            ];
            Some((position, forward, up))
        }
        CameraCase::Released => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ([f64; 3], [f64; 3], [f64; 3], f64) {
        let eye = [0.1, 0.05, -0.2];
        let ahead = [0.0, 0.0, -1.0];
        let up = [0.0, 1.0, 0.0];
        (eye, ahead, up, 0.001)
    }

    #[test]
    fn one_function_poses_the_player_camera() {
        // #754 AC3: one function covers gliding, first, third, handover, and
        // released.
        let (eye, ahead, up, height) = sample();
        let pitch = arrival_tilt();
        let arrival_camera = [0.1, 0.05 + 0.6 * height, -0.2 + 3.0 * height];
        let arrival_eye = eye;
        // Gliding: at the arrival pose, looking at the eye.
        let gliding = player_camera(
            CameraCase::Gliding,
            eye,
            ahead,
            up,
            height,
            pitch,
            true,
            arrival_camera,
            0.0,
            arrival_camera,
            arrival_eye,
        )
        .expect("gliding poses");
        assert_eq!(gliding.0, arrival_camera);
        assert_eq!(gliding.1, {
            let d = [
                arrival_eye[0] - arrival_camera[0],
                arrival_eye[1] - arrival_camera[1],
                arrival_eye[2] - arrival_camera[2],
            ];
            let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            [d[0] / l, d[1] / l, d[2] / l]
        });
        // First: from the eye.
        let first = player_camera(
            CameraCase::First,
            eye,
            ahead,
            up,
            height,
            pitch,
            false,
            arrival_camera,
            0.0,
            arrival_camera,
            arrival_eye,
        )
        .expect("first poses");
        assert_eq!(first.0, eye);
        // Third: behind and above the eye.
        let third = player_camera(
            CameraCase::Third,
            eye,
            ahead,
            up,
            height,
            pitch,
            true,
            arrival_camera,
            0.0,
            arrival_camera,
            arrival_eye,
        )
        .expect("third poses");
        let want = universe_core::player::third_person_camera(eye, first.1, up, height);
        assert_eq!(third.0, want);
        assert_eq!(third.1, first.1, "same pitch, same direction");
        // Handover at t=0 starts at the arrival point (third).
        let handover = player_camera(
            CameraCase::Handover,
            eye,
            ahead,
            up,
            height,
            pitch,
            true,
            arrival_camera,
            0.0,
            arrival_camera,
            arrival_eye,
        )
        .expect("handover poses");
        assert_eq!(handover.0, arrival_camera, "starts at the arrival");
        assert_eq!(handover.1, third.1, "direction already matches");
        // Handover at the end lands on the body's own target.
        let landed = player_camera(
            CameraCase::Handover,
            eye,
            ahead,
            up,
            height,
            pitch,
            true,
            arrival_camera,
            HANDOVER_S,
            arrival_camera,
            arrival_eye,
        )
        .expect("handover lands");
        assert_eq!(landed.0, third.0);
        // First-person handover eases to the eye.
        let first_handover = player_camera(
            CameraCase::Handover,
            eye,
            ahead,
            up,
            height,
            pitch,
            false,
            arrival_camera,
            0.0,
            arrival_camera,
            arrival_eye,
        )
        .expect("first handover poses");
        assert_eq!(first_handover.0, arrival_camera);
        let first_landed = player_camera(
            CameraCase::Handover,
            eye,
            ahead,
            up,
            height,
            pitch,
            false,
            arrival_camera,
            HANDOVER_S,
            arrival_camera,
            arrival_eye,
        )
        .expect("first handover lands");
        assert_eq!(first_landed.0, eye);
        // Released: back to dive or free, no pose.
        assert_eq!(
            player_camera(
                CameraCase::Released,
                eye,
                ahead,
                up,
                height,
                pitch,
                true,
                arrival_camera,
                0.0,
                arrival_camera,
                arrival_eye,
            ),
            None
        );
    }

    #[test]
    fn handover_keeps_the_camera_still() {
        // #754 AC2: the first body frame matches the last arrival frame
        // within a hundredth of a body height and a tenth of a degree, in
        // third person, and in first person after a smooth move into the eye.
        let (eye, ahead, up, height) = sample();
        let pitch = arrival_tilt();
        // The arrival pose for this eye and yaw (level ahead, +Y up).
        let arrival_camera = universe_core::player::third_person_camera(eye, ahead, up, height);
        let arrival_eye = eye;
        let arrival_dir = {
            let d = [
                arrival_eye[0] - arrival_camera[0],
                arrival_eye[1] - arrival_camera[1],
                arrival_eye[2] - arrival_camera[2],
            ];
            let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            [d[0] / l, d[1] / l, d[2] / l]
        };
        // Third person: the handover starts exactly at the arrival.
        let handover = player_camera(
            CameraCase::Handover,
            eye,
            ahead,
            up,
            height,
            pitch,
            true,
            arrival_camera,
            0.0,
            arrival_camera,
            arrival_eye,
        )
        .expect("handover poses");
        let gap = ((handover.0[0] - arrival_camera[0]).powi(2)
            + (handover.0[1] - arrival_camera[1]).powi(2)
            + (handover.0[2] - arrival_camera[2]).powi(2))
        .sqrt();
        assert!(
            gap <= 0.01 * height,
            "third-person position still: {gap} vs {}",
            0.01 * height
        );
        let cosine = (handover.1[0] * arrival_dir[0]
            + handover.1[1] * arrival_dir[1]
            + handover.1[2] * arrival_dir[2])
            .clamp(-1.0, 1.0);
        let angle_deg = cosine.acos().to_degrees();
        assert!(
            angle_deg <= 0.1,
            "third-person direction still: {angle_deg} deg"
        );
        // First person: the direction already matches; the position eases
        // from the arrival into the eye without a cut (mid-ease stays on the
        // segment).
        let first_start = player_camera(
            CameraCase::Handover,
            eye,
            ahead,
            up,
            height,
            pitch,
            false,
            arrival_camera,
            0.0,
            arrival_camera,
            arrival_eye,
        )
        .expect("first handover poses");
        assert_eq!(first_start.0, arrival_camera);
        let cosine = (first_start.1[0] * arrival_dir[0]
            + first_start.1[1] * arrival_dir[1]
            + first_start.1[2] * arrival_dir[2])
            .clamp(-1.0, 1.0);
        assert!(
            cosine.acos().to_degrees() <= 0.1,
            "first-person direction still"
        );
        let mid = player_camera(
            CameraCase::Handover,
            eye,
            ahead,
            up,
            height,
            pitch,
            false,
            arrival_camera,
            HANDOVER_S * 0.5,
            arrival_camera,
            arrival_eye,
        )
        .expect("mid handover poses");
        for axis in 0..3 {
            let low = arrival_camera[axis].min(eye[axis]) - 1e-12;
            let high = arrival_camera[axis].max(eye[axis]) + 1e-12;
            assert!(
                (low..=high).contains(&mid.0[axis]),
                "smooth move into the eye, axis {axis}"
            );
        }
    }
}
