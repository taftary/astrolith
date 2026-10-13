//! Black-and-white squares on planets (#626, Spec v2).
//!
//! The Planet look setting can draw every planet and moon as a 3D
//! chessboard: space is cut into [`SQUARE_M`] cubes, alternating black and
//! white, and the surface shows where it crosses them (R2 A). Once a square
//! is only a few pixels on screen it fades to an even gray (R4 A), so far
//! surfaces never flicker.
//!
//! The renderer's fragment shader (`planet_squares.wgsl`) computes the same
//! pattern on the GPU; this module is its pure `f64` reference, pinned by
//! tests. The pattern repeats every [`PERIOD_M`] along each axis, so the
//! CPU reduces the render origin's planet position by that period
//! ([`origin_in_period`]) and the GPU only ever sees small numbers.

/// Edge of one square, in metres (R3: "100m/100m").
pub const SQUARE_M: f64 = 100.0;

/// The pattern repeats every two squares along each axis.
pub const PERIOD_M: f64 = 2.0 * SQUARE_M;

/// A square at least this many pixels wide on screen is drawn pure.
pub const SHARP_PX: f64 = 8.0;

/// A square at most this many pixels wide on screen is drawn even gray.
pub const GRAY_PX: f64 = 2.0;

/// The even gray far squares fade to: the average of black and white.
pub const GRAY: f64 = 0.5;

/// Colour of the cube holding planet-space point `metres`: `0.0` black or
/// `1.0` white, alternating like a chessboard along every axis.
#[must_use]
pub fn square_parity(metres: [f64; 3]) -> f64 {
    let sum: f64 = metres.iter().map(|m| (m / SQUARE_M).floor()).sum();
    if sum.rem_euclid(2.0) < 1.0 { 0.0 } else { 1.0 }
}

/// How far a square `square_px` pixels wide has faded to gray: `0.0` pure
/// at [`SHARP_PX`] or more, `1.0` gray at [`GRAY_PX`] or less, smooth
/// between (the shader's `smoothstep`).
#[must_use]
pub fn gray_share(square_px: f64) -> f64 {
    if square_px.is_nan() {
        return 1.0;
    }
    // In squares per pixel, as the shader's `fwidth` gives it.
    let per_px = 1.0 / square_px.max(1e-300);
    let t = ((per_px - 1.0 / SHARP_PX) / (1.0 / GRAY_PX - 1.0 / SHARP_PX)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Tone at planet-space point `metres` for a square `square_px` pixels
/// wide on screen: black or white, faded toward [`GRAY`] when small.
#[must_use]
pub fn square_tone(metres: [f64; 3], square_px: f64) -> f64 {
    let share = gray_share(square_px);
    square_parity(metres) * (1.0 - share) + GRAY * share
}

/// `metres` reduced into `0..PERIOD_M` along each axis: the same squares,
/// with numbers small enough for the GPU's `f32` at any planet size.
#[must_use]
pub fn origin_in_period(metres: [f64; 3]) -> [f64; 3] {
    metres.map(|m| {
        if m.is_finite() {
            m.rem_euclid(PERIOD_M)
        } else {
            0.0
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neighbouring_squares_alternate_along_every_axis() {
        let here = square_parity([50.0, 50.0, 50.0]);
        for axis in 0..3 {
            let mut next = [50.0, 50.0, 50.0];
            next[axis] += SQUARE_M;
            assert_ne!(square_parity(next), here, "axis {axis} alternates");
            next[axis] += SQUARE_M;
            assert_eq!(square_parity(next), here, "two squares on, the same colour");
        }
        // Negative coordinates keep alternating across zero.
        assert_ne!(
            square_parity([-50.0, 50.0, 50.0]),
            square_parity([50.0, 50.0, 50.0])
        );
    }

    #[test]
    fn a_square_is_one_hundred_metres_and_only_black_or_white() {
        let colour = square_parity([0.5, 0.5, 0.5]);
        for step in [1.0, 25.0, 50.0, 99.0] {
            assert_eq!(square_parity([step, 0.5, 0.5]), colour, "inside one square");
        }
        assert_ne!(square_parity([100.5, 0.5, 0.5]), colour, "the next square");
        for x in 0..40 {
            let tone = square_parity([f64::from(x) * 7.3, 13.0, -41.0]);
            assert!(tone == 0.0 || tone == 1.0, "pure black or white");
        }
    }

    #[test]
    fn small_squares_fade_to_an_even_gray() {
        let p = [10.0, 10.0, 10.0];
        let pure = square_parity(p);
        assert_eq!(square_tone(p, 40.0), pure, "big squares are pure");
        assert_eq!(
            square_tone(p, SHARP_PX),
            pure,
            "pure down to the sharp size"
        );
        assert_eq!(square_tone(p, GRAY_PX), GRAY, "gray at the gray size");
        assert_eq!(square_tone(p, 0.01), GRAY, "a far planet is gray");
        let between = square_tone(p, 4.0);
        assert!(
            between > 0.0 && between < 1.0 && between != GRAY,
            "smooth between"
        );
        let mut last = 0.0;
        for px in [16.0, 8.0, 6.0, 4.0, 3.0, 2.0, 1.0] {
            let share = gray_share(px);
            assert!(share >= last, "the fade only grows as squares shrink");
            last = share;
        }
    }

    #[test]
    fn the_origin_reduced_by_the_period_gives_the_same_squares() {
        let far = [4_215_717.3, -1_234_567.8, 987_654.1];
        let reduced = origin_in_period(far);
        assert!(reduced.iter().all(|m| (0.0..PERIOD_M).contains(m)));
        for offset in [[0.0, 0.0, 0.0], [37.0, -120.0, 260.0], [-5.0, 5.0, -300.0]] {
            let a = [far[0] + offset[0], far[1] + offset[1], far[2] + offset[2]];
            let b = [
                reduced[0] + offset[0],
                reduced[1] + offset[1],
                reduced[2] + offset[2],
            ];
            assert_eq!(
                square_parity(a),
                square_parity(b),
                "same square at {offset:?}"
            );
        }
    }
}
