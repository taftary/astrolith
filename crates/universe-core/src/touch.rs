//! Touch layout and gesture math (#561).
//!
//! Pure (`E-CORE-NO-BEVY`): positions are logical pixels with the origin at
//! the window's top left, as the window toolkit reports touches. The
//! on-screen stick sits in one bottom corner and the round buttons in the
//! other; Hand left swaps them (owner answer Q7 A). Pushing the stick to its
//! edge runs (Q4 A). Pinching apart dives in, together rises out (Q1 C).

/// Gap between the controls and the window edge, logical px.
pub const EDGE_GAP_PX: f32 = 32.0;

/// Gap between the controls and the window bottom, logical px: clears the
/// three-line scale readout at the bottom left.
pub const BOTTOM_GAP_PX: f32 = 96.0;

/// Stick radius (its round base), logical px.
pub const STICK_RADIUS_PX: f32 = 80.0;

/// Round button radius, logical px (twice the finger size across).
pub const BUTTON_RADIUS_PX: f32 = 40.0;

/// Gap between two round buttons, logical px.
pub const BUTTON_GAP_PX: f32 = 20.0;

/// Stick push (share of its radius) below which nothing moves.
pub const STICK_DEAD_ZONE: f32 = 0.25;

/// Stick push (share of the radius, along one axis) that presses that
/// direction; under 0.5 so diagonals press both.
pub const STICK_AXIS: f32 = 0.38;

/// Stick push (share of its radius) that runs or boosts (the edge).
pub const STICK_RUN: f32 = 0.9;

/// A finger that moves at most this far (logical px) and lifts within
/// [`TAP_MAX_S`] is a tap.
pub const TAP_SLOP_PX: f32 = 12.0;

/// Longest press that still counts as a tap, seconds.
pub const TAP_MAX_S: f64 = 0.35;

/// Dive factor per natural-log unit of pinch spread: spreading the fingers
/// to twice their distance dives in like this many wheel notches' worth.
pub const PINCH_GAIN: f64 = 3.0;

const _: () = assert!(2.0 * BUTTON_RADIUS_PX >= crate::pick::PICK_TOUCH_DP);

/// One on-screen round button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TouchButton {
    /// Jump (tap), fly (double-tap), climb (hold while flying); normal mode.
    Jump,
    /// Descend while flying; normal mode, only while flying.
    Down,
    /// First or third person; normal mode.
    Camera,
    /// Autopilot on or off; dive mode.
    Autopilot,
}

impl TouchButton {
    /// Every button.
    pub const ALL: [TouchButton; 4] = [
        TouchButton::Jump,
        TouchButton::Down,
        TouchButton::Camera,
        TouchButton::Autopilot,
    ];

    /// Text on the button.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            TouchButton::Jump => "Jump",
            TouchButton::Down => "Down",
            TouchButton::Camera => "Camera",
            TouchButton::Autopilot => "Auto",
        }
    }
}

/// Where the touch controls sit for one window size and hand.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    /// Centre of the stick base.
    pub stick: [f32; 2],
    /// Centres of the round buttons, in [`TouchButton::ALL`] order.
    pub buttons: [[f32; 2]; 4],
}

impl Layout {
    /// The layout for a `width` x `height` window (logical px). Right hand:
    /// stick bottom left, buttons bottom right; left hand mirrors it.
    #[must_use]
    pub fn new(width: f32, height: f32, left_hand: bool) -> Layout {
        let side = |x: f32| if left_hand { width - x } else { x };
        let stick = [
            side(EDGE_GAP_PX + STICK_RADIUS_PX),
            height - BOTTOM_GAP_PX - STICK_RADIUS_PX,
        ];
        let step = 2.0 * BUTTON_RADIUS_PX + BUTTON_GAP_PX;
        // Jump in the corner, Down beside it, Camera above it; Autopilot
        // (dive mode, when the others hide) takes Jump's place.
        let corner_x = width - EDGE_GAP_PX - BUTTON_RADIUS_PX;
        let corner_y = height - BOTTOM_GAP_PX - BUTTON_RADIUS_PX;
        let at = |x: f32, y: f32| [if left_hand { width - x } else { x }, y];
        Layout {
            stick,
            buttons: [
                at(corner_x, corner_y),
                at(corner_x - step, corner_y),
                at(corner_x, corner_y - step),
                at(corner_x, corner_y),
            ],
        }
    }

    /// Centre of `button`.
    #[must_use]
    pub fn button(&self, button: TouchButton) -> [f32; 2] {
        let index = TouchButton::ALL
            .iter()
            .position(|b| *b == button)
            .unwrap_or(0);
        self.buttons.get(index).copied().unwrap_or(self.stick)
    }

    /// The control under `point` among the `shown` ones: a button, else
    /// the stick, else `None` (free screen: look, tap, or pinch).
    #[must_use]
    pub fn hit(&self, point: [f32; 2], shown: Shown) -> Option<Control> {
        for (index, button) in TouchButton::ALL.into_iter().enumerate() {
            let on = shown.buttons.get(index).copied().unwrap_or(false);
            if on && distance(point, self.button(button)) <= BUTTON_RADIUS_PX {
                return Some(Control::Button(button));
            }
        }
        if shown.stick && distance(point, self.stick) <= STICK_RADIUS_PX {
            return Some(Control::Stick);
        }
        None
    }
}

/// Which touch controls a mode shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Shown {
    /// The stick (normal and free mode).
    pub stick: bool,
    /// Each button, in [`TouchButton::ALL`] order.
    pub buttons: [bool; 4],
}

impl Shown {
    /// Normal mode: stick, Jump, Camera, and Down while `flying`.
    #[must_use]
    pub fn normal(flying: bool) -> Shown {
        Shown {
            stick: true,
            buttons: [true, flying, true, false],
        }
    }

    /// Free mode: the stick alone.
    #[must_use]
    pub fn free() -> Shown {
        Shown {
            stick: true,
            buttons: [false; 4],
        }
    }

    /// Dive mode: the Autopilot button alone.
    #[must_use]
    pub fn dive() -> Shown {
        Shown {
            stick: false,
            buttons: [false, false, false, true],
        }
    }

    /// Whether `button` shows.
    #[must_use]
    pub fn has(self, button: TouchButton) -> bool {
        let index = TouchButton::ALL
            .iter()
            .position(|b| *b == button)
            .unwrap_or(0);
        self.buttons.get(index).copied().unwrap_or(false)
    }
}

/// A touch control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// The on-screen stick.
    Stick,
    /// One round button.
    Button(TouchButton),
}

/// Distance between two points.
#[must_use]
pub fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    let (dx, dy) = (a[0] - b[0], a[1] - b[1]);
    (dx * dx + dy * dy).sqrt()
}

/// Directions the stick presses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StickInput {
    /// Up on screen: forward.
    pub forward: bool,
    /// Down on screen: back.
    pub back: bool,
    /// Left.
    pub left: bool,
    /// Right.
    pub right: bool,
    /// Pushed to the edge: run or boost.
    pub run: bool,
}

/// The directions for a finger `offset` (logical px) from the stick centre.
#[must_use]
pub fn stick(offset: [f32; 2]) -> StickInput {
    let x = offset[0] / STICK_RADIUS_PX;
    let y = offset[1] / STICK_RADIUS_PX;
    let push = (x * x + y * y).sqrt();
    if !push.is_finite() || push < STICK_DEAD_ZONE {
        return StickInput::default();
    }
    // Past the edge the finger still pushes in its direction.
    let scale = push.max(1.0);
    let (x, y) = (x / scale, y / scale);
    StickInput {
        forward: y <= -STICK_AXIS,
        back: y >= STICK_AXIS,
        left: x <= -STICK_AXIS,
        right: x >= STICK_AXIS,
        run: push >= STICK_RUN,
    }
}

/// The thumb position drawn for a finger `offset`: held inside the base.
#[must_use]
pub fn thumb(offset: [f32; 2]) -> [f32; 2] {
    let push = (offset[0] * offset[0] + offset[1] * offset[1]).sqrt();
    if push <= STICK_RADIUS_PX || !push.is_finite() {
        return offset;
    }
    let scale = STICK_RADIUS_PX / push;
    [offset[0] * scale, offset[1] * scale]
}

/// Dive log factor for a pinch whose finger distance went from `before` to
/// `after` (logical px): apart dives in (negative, like the wheel up),
/// together rises out.
#[must_use]
pub fn pinch_log_factor(before: f32, after: f32) -> f64 {
    if !(before.is_finite() && after.is_finite()) || before <= 0.0 || after <= 0.0 {
        return 0.0;
    }
    -(f64::from(after) / f64::from(before)).ln() * PINCH_GAIN
}

/// Whether a press that moved `moved_px` at most and lasted `held_s` is a
/// tap.
#[must_use]
pub fn is_tap(moved_px: f32, held_s: f64) -> bool {
    moved_px <= TAP_SLOP_PX && held_s <= TAP_MAX_S
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: f32 = 1280.0;
    const H: f32 = 720.0;

    #[test]
    fn right_hand_puts_the_stick_left_and_left_hand_swaps() {
        let right = Layout::new(W, H, false);
        let left = Layout::new(W, H, true);
        assert!(right.stick[0] < W / 2.0, "stick bottom left");
        assert!(right.stick[1] > H / 2.0);
        assert!(
            right.button(TouchButton::Jump)[0] > W / 2.0,
            "buttons right"
        );
        assert!(left.stick[0] > W / 2.0, "left hand: stick bottom right");
        assert!(left.button(TouchButton::Jump)[0] < W / 2.0);
        assert!(
            (left.stick[0] - (W - right.stick[0])).abs() < 1e-3,
            "mirrored"
        );
        // Everything sits on screen.
        for layout in [right, left] {
            for centre in layout.buttons {
                assert!(centre[0] >= BUTTON_RADIUS_PX && centre[0] <= W - BUTTON_RADIUS_PX);
                assert!(centre[1] >= BUTTON_RADIUS_PX && centre[1] <= H - BUTTON_RADIUS_PX);
            }
        }
    }

    #[test]
    fn buttons_do_not_overlap_and_are_finger_sized() {
        let layout = Layout::new(W, H, false);
        let normal = [TouchButton::Jump, TouchButton::Down, TouchButton::Camera];
        for a in normal {
            for b in normal {
                if a != b {
                    assert!(distance(layout.button(a), layout.button(b)) >= 2.0 * BUTTON_RADIUS_PX);
                }
            }
        }
    }

    #[test]
    fn hit_finds_shown_controls_only() {
        let layout = Layout::new(W, H, false);
        let normal = Shown::normal(true);
        let dive = Shown::dive();
        let jump = layout.button(TouchButton::Jump);
        assert_eq!(
            layout.hit(jump, normal),
            Some(Control::Button(TouchButton::Jump))
        );
        assert_eq!(
            layout.hit(jump, dive),
            Some(Control::Button(TouchButton::Autopilot)),
            "dive mode: Autopilot in Jump's place"
        );
        assert_eq!(layout.hit(layout.stick, normal), Some(Control::Stick));
        assert_eq!(
            layout.hit(layout.stick, dive),
            None,
            "no stick in dive mode"
        );
        assert_eq!(layout.hit([W / 2.0, H / 2.0], normal), None, "free screen");
        let down = layout.button(TouchButton::Down);
        assert_eq!(
            layout.hit(down, Shown::normal(false)),
            None,
            "Down only while flying"
        );
        assert_eq!(
            layout.hit(layout.stick, Shown::free()),
            Some(Control::Stick)
        );
        assert!(Shown::dive().has(TouchButton::Autopilot) && !Shown::free().has(TouchButton::Jump));
    }

    #[test]
    fn the_stick_walks_then_runs_at_its_edge() {
        assert_eq!(stick([0.0, -10.0]), StickInput::default(), "dead zone");
        let walk = stick([0.0, -0.5 * STICK_RADIUS_PX]);
        assert!(walk.forward && !walk.run && !walk.left && !walk.right);
        let run = stick([0.0, -STICK_RADIUS_PX]);
        assert!(run.forward && run.run);
        let beyond = stick([0.0, 3.0 * STICK_RADIUS_PX]);
        assert!(beyond.back && beyond.run, "past the edge still runs");
        let diagonal = stick([0.6 * STICK_RADIUS_PX, -0.6 * STICK_RADIUS_PX]);
        assert!(diagonal.forward && diagonal.right);
        assert_eq!(thumb([0.0, 3.0 * STICK_RADIUS_PX]), [0.0, STICK_RADIUS_PX]);
    }

    #[test]
    fn pinching_apart_dives_in_and_together_rises_out() {
        assert!(pinch_log_factor(100.0, 200.0) < 0.0, "apart dives in");
        assert!(pinch_log_factor(200.0, 100.0) > 0.0, "together rises out");
        assert!(
            (pinch_log_factor(100.0, 200.0) + pinch_log_factor(200.0, 100.0)).abs() < 1e-12,
            "symmetric"
        );
        assert_eq!(pinch_log_factor(0.0, 100.0), 0.0);
    }

    #[test]
    fn short_still_presses_are_taps() {
        assert!(is_tap(3.0, 0.1));
        assert!(!is_tap(TAP_SLOP_PX + 1.0, 0.1), "moved: a drag");
        assert!(!is_tap(0.0, TAP_MAX_S + 0.1), "held: a hold");
    }
}
