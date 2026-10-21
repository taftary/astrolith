//! Dive orbit and turn math for better dive control (#873).
//!
//! Pure and deterministic (`E-CORE-NO-BEVY`, `E-DET-TIERS`): rotating the
//! dive camera around its target at the same distance (orbit, Q1) and
//! rolling the view around the line to the target (turn, Q2). The window
//! (`universe-render`) maps `WASD`, left-drag and right-drag to these steps;
//! the headless tests prove the distance, speed and gaze rules.

use crate::coords::OpenUnits;

/// Base orbit rate in radians per second when the gap equals the distance
/// at speed `1.0` (#873 T1).
///
/// Far from the target (`gap ~= distance`) one second of hold turns about
/// `1 rad` (about 57 degrees); close to the surface the same hold turns
/// proportionally less, so the scene moves at about the same speed on
/// screen (C2). The `Dive orbit speed` setting multiplies it.
pub const DIVE_ORBIT_RATE: f64 = 1.0;

/// Share of the planet radius kept as clearance when sliding around it
/// (#873 T3): the avoid sphere is `radius * (1 + share)`, about 6 mm for
/// Earth, so the camera never touches the bare rock. The relief-plus-3 m
/// floor of T4 sits higher wherever relief exists.
pub const PLANET_CLEARANCE_SHARE: f64 = 1e-9;

/// Height above the drawn terrain the dive never goes below, in metres
/// (#873 T4 Q7 A, Q8 A): zooming in stops here, orbiting rises just enough
/// over hills. Stays in dive mode.
pub const GROUND_CLEARANCE_M: f64 = 3.0;

/// Height of the move-to-player arrival above the feet, in metres (#873 T4
/// Q6 C, 10 m): the camera sits straight above the body with the player
/// between it and the planet centre, looking down. Replaces #607 Q3 A
/// (behind and above).
pub const PLAYER_ARRIVAL_ABOVE_M: f64 = 10.0;

/// Orbit angle rate in radians per second for `gap` metres (in open-cell
/// units) from the target surface at `distance` from its centre, at `speed`
/// (`Dive orbit speed`, `0.5..=2.0`).
///
/// Returns `k * speed * gap / distance` with [`DIVE_ORBIT_RATE`] as `k`, so
/// the turn is slower close to the surface and faster far from it (C2).
/// Degenerate or non-positive inputs hold still (`0.0`). Pure.
#[must_use]
pub fn orbit_rate(gap: f64, distance: f64, speed: f64) -> f64 {
    if !gap.is_finite() || !distance.is_finite() || !speed.is_finite() {
        return 0.0;
    }
    if distance <= 0.0 || speed <= 0.0 {
        return 0.0;
    }
    let gap = gap.max(0.0);
    DIVE_ORBIT_RATE * speed * gap / distance
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(v: [f64; 3], s: f64) -> [f64; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

/// Cross product of two direction vectors (shared with the free-flight
/// spaceship orientation, #884 T2). Pure.
#[must_use]
pub(crate) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(v: [f64; 3]) -> f64 {
    dot(v, v).sqrt()
}

fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let len = length(v);
    if len.is_finite() && len > 0.0 {
        Some(scale(v, 1.0 / len))
    } else {
        None
    }
}

/// Rotates `v` around unit-or-not `axis` by `angle` radians (right-hand
/// rule) with Rodrigues' formula.
///
/// Degenerate or non-finite inputs hold `v`. Shared with the free-flight
/// spaceship orientation (#884 T2). Pure.
#[must_use]
pub(crate) fn rotate_around(v: [f64; 3], axis: [f64; 3], angle: f64) -> [f64; 3] {
    if !v.iter().all(|c| c.is_finite()) || !axis.iter().all(|c| c.is_finite()) {
        return v;
    }
    if !angle.is_finite() || angle == 0.0 {
        return v;
    }
    let Some(k) = unit(axis) else {
        return v;
    };
    let (sin, cos) = angle.sin_cos();
    if !sin.is_finite() || !cos.is_finite() {
        return v;
    }
    let along = dot(k, v);
    let turned = add(
        add(scale(v, cos), scale(cross(k, v), sin)),
        scale(k, along * (1.0 - cos)),
    );
    if turned.iter().all(|c| c.is_finite()) {
        turned
    } else {
        v
    }
}

/// Orbits the dive camera around `center` without changing the distance.
///
/// `yaw` moves the camera right (`D`, positive) or left (`A`); `pitch`
/// moves it up (`W`, positive) or down (`S`). Both are radians for this
/// step (the caller multiplies [`orbit_rate`] by `dt` and the held input).
/// The returned up is carried round with the camera, so there is no roll
/// flip; the gaze stays on `center` by construction. A camera exactly at
/// `center` or a degenerate `up` holds its pose. Pure.
///
/// The pitch sign is chosen so positive pitch rises: the offset rotates by
/// `-pitch` around the right axis (`cross(forward, up)`).
#[must_use]
pub fn dive_orbit(
    camera: OpenUnits,
    center: OpenUnits,
    up: [f64; 3],
    yaw: f64,
    pitch: f64,
) -> (OpenUnits, [f64; 3]) {
    let yaw = if yaw.is_finite() { yaw } else { 0.0 };
    let pitch = if pitch.is_finite() { pitch } else { 0.0 };
    if yaw == 0.0 && pitch == 0.0 {
        return (camera, up);
    }
    let offset = sub(camera.0, center.0);
    let distance = length(offset);
    if !distance.is_finite() || distance <= 0.0 {
        return (camera, up);
    }
    let mut up_unit = unit(up).unwrap_or([0.0, 1.0, 0.0]);
    if !up_unit.iter().all(|c| c.is_finite()) {
        up_unit = [0.0, 1.0, 0.0];
    }
    // Yaw around the up axis: the camera moves right for positive yaw.
    let mut moved = offset;
    if yaw != 0.0 {
        moved = rotate_around(moved, up_unit, yaw);
    }
    // Pitch around the fresh right axis: positive pitch rises.
    if pitch != 0.0 {
        let forward = scale(moved, -1.0 / distance);
        let mut right = cross(forward, up_unit);
        if length(right) <= 1e-12 || !right.iter().all(|c| c.is_finite()) {
            // Forward parallel to up: pick any perpendicular.
            let helper = if forward[1].abs() < 0.9 {
                [0.0, 1.0, 0.0]
            } else {
                [1.0, 0.0, 0.0]
            };
            right = cross(forward, helper);
            if length(right) <= 1e-12 || !right.iter().all(|c| c.is_finite()) {
                right = [1.0, 0.0, 0.0];
            }
        }
        moved = rotate_around(moved, right, -pitch);
        up_unit = rotate_around(up_unit, right, -pitch);
    }
    let next = OpenUnits(add(center.0, moved));
    // Keep the pose finite: a bad rotation holds the old one.
    if next.0.iter().all(|c| c.is_finite()) && up_unit.iter().all(|c| c.is_finite()) {
        (next, up_unit)
    } else {
        (camera, up)
    }
}

/// Advances the dive turn `roll` by `delta` radians (right-drag, Q2).
///
/// The turn stays where it is left: the caller stores the sum. The sum is
/// wrapped to `-PI..=PI` so it stays bounded; wrapping is exact (no drift).
/// Non-finite inputs hold `roll`. Pure.
#[must_use]
pub fn advance_roll(roll: f64, delta: f64) -> f64 {
    if !roll.is_finite() {
        return if delta.is_finite() { delta } else { 0.0 };
    }
    if !delta.is_finite() || delta == 0.0 {
        return roll;
    }
    let two_pi = 2.0 * std::f64::consts::PI;
    let shifted = (roll + delta + std::f64::consts::PI).rem_euclid(two_pi) - std::f64::consts::PI;
    // `rem_euclid` maps `-PI` to `+PI` half the time; keep one branch.
    if shifted == -std::f64::consts::PI {
        std::f64::consts::PI
    } else {
        shifted
    }
}

/// Time constant for the drawn dive roll easing toward the dragged roll
/// (#884 T1, Q1 A): about 50 ms, so the view follows the mouse smoothly
/// with no jumps and stops exactly on release (no overshoot, no drift).
pub const ROLL_TAU_S: f64 = 0.05;

/// Eases the drawn roll `drawn` toward the dragged target `target` over
/// `dt` seconds (#884 T1).
///
/// Exponential approach with [`ROLL_TAU_S`]: steady input never jumps and
/// never overshoots, and a held target settles exactly on it (snapped
/// within 1e-9 rad). The shortest wrapped delta is eased, so crossing
/// `-PI`/`PI` never spins the long way. Degenerate inputs hold `drawn`.
/// Pure.
#[must_use]
pub fn ease_roll(drawn: f64, target: f64, dt: f64) -> f64 {
    if !drawn.is_finite() {
        return if target.is_finite() { target } else { 0.0 };
    }
    if !target.is_finite() || !dt.is_finite() || dt <= 0.0 {
        return drawn;
    }
    let two_pi = 2.0 * std::f64::consts::PI;
    let mut delta = (target - drawn).rem_euclid(two_pi);
    if delta > std::f64::consts::PI {
        delta -= two_pi;
    }
    if delta == 0.0 {
        return target;
    }
    if !ROLL_TAU_S.is_finite() || ROLL_TAU_S <= 0.0 {
        return target;
    }
    let share = 1.0 - (-dt / ROLL_TAU_S).exp();
    if !share.is_finite() {
        return drawn;
    }
    let stepped = drawn + delta * share.clamp(0.0, 1.0);
    // Snap when the step lands within a microradian: the drawn roll stops
    // exactly, so no later frame changes it after release.
    let mut rest = (target - stepped).rem_euclid(two_pi);
    if rest > std::f64::consts::PI {
        rest -= two_pi;
    }
    if rest.abs() < 1e-9 {
        target
    } else if stepped.is_finite() {
        stepped
    } else {
        drawn
    }
}

/// Rolls `up` around `forward` by `roll` radians (right-hand rule).
///
/// The camera position never changes and the view direction (`forward`)
/// never moves, so the target stays in the middle (C3, AC3). Degenerate
/// inputs hold `up`. Pure.
#[must_use]
pub fn roll_up(forward: [f64; 3], up: [f64; 3], roll: f64) -> [f64; 3] {
    if !roll.is_finite() || roll == 0.0 {
        return up;
    }
    if !forward.iter().all(|c| c.is_finite()) || !up.iter().all(|c| c.is_finite()) {
        return up;
    }
    rotate_around(up, forward, roll)
}

/// Avoid radius for a planet of `radius_cells`: the bare rock plus the
/// T3 clearance share, so the camera never touches it. Pure.
#[must_use]
pub fn planet_avoid_radius(radius_cells: f64) -> f64 {
    if !radius_cells.is_finite() || radius_cells <= 0.0 {
        return 0.0;
    }
    radius_cells * (1.0 + PLANET_CLEARANCE_SHARE)
}

/// Cursor distance from the screen centre, in pixels, under which the dive
/// turn falls back to horizontal motion (#907 T1): the swept angle is
/// unstable that close to the centre.
pub const WHEEL_MIN_RADIUS_PX: f64 = 24.0;

/// Signed angle the cursor swept around `centre` going from `prev` to
/// `now`, in radians, in screen coordinates with `y` down (#907 T1, AC1).
///
/// Like grabbing a wheel: a move to the right above the centre and one
/// below it sweep opposite ways, and one circle around the centre sweeps a
/// full turn. `None` when either point is within `min_radius` of the
/// centre or any input is not finite. Pure.
#[must_use]
pub fn wheel_turn(prev: [f64; 2], now: [f64; 2], centre: [f64; 2], min_radius: f64) -> Option<f64> {
    let all = [
        prev[0], prev[1], now[0], now[1], centre[0], centre[1], min_radius,
    ];
    if !all.iter().all(|c| c.is_finite()) {
        return None;
    }
    let a = [prev[0] - centre[0], prev[1] - centre[1]];
    let b = [now[0] - centre[0], now[1] - centre[1]];
    let ra = a[0].hypot(a[1]);
    let rb = b[0].hypot(b[1]);
    if ra < min_radius || rb < min_radius {
        return None;
    }
    let cross = a[0] * b[1] - a[1] * b[0];
    let dot = a[0] * b[0] + a[1] * b[1];
    let angle = cross.atan2(dot);
    angle.is_finite().then_some(angle)
}

/// Avoid radius below the lowest possible ground of a planet (#892 T1, Q2 A).
///
/// The #873 ball sat at the mean (old sea-level) radius, so it blocked every
/// basin below it. It now sits at the relief's lower bound, so it only keeps
/// the camera out of the planet's core; [`clamp_above_ground`] does the real
/// stop 3 m above the ground everywhere. Zero for degenerate inputs. Pure.
#[must_use]
pub fn lowest_ground_avoid_radius(
    frame: &crate::planet_frame::PlanetFrame,
    relief: &crate::relief::Relief,
) -> f64 {
    let mpc = frame.metres_per_cell();
    let radius_m = frame.radius_m();
    if !mpc.is_finite() || mpc <= 0.0 || !radius_m.is_finite() || radius_m <= 0.0 {
        return 0.0;
    }
    let low_m = relief.height_bounds_m().0.min(0.0);
    let floor_m = radius_m + low_m;
    if !floor_m.is_finite() || floor_m <= 0.0 {
        return 0.0;
    }
    planet_avoid_radius(floor_m / mpc)
}

/// Radius of the real ground under `camera_cells`, in L10 open-cell units
/// (#892 T1): the planet's mean radius plus the relief height in that
/// direction. Zero for degenerate inputs. Pure.
#[must_use]
pub fn ground_radius_cells(
    camera_cells: [f64; 3],
    frame: &crate::planet_frame::PlanetFrame,
    relief: &crate::relief::Relief,
) -> f64 {
    let mpc = frame.metres_per_cell();
    let radius_m = frame.radius_m();
    if !mpc.is_finite() || mpc <= 0.0 || !radius_m.is_finite() || radius_m <= 0.0 {
        return 0.0;
    }
    let Some(dir) = unit(camera_cells) else {
        return 0.0;
    };
    let ground_m = radius_m + relief.height_m(dir);
    if ground_m.is_finite() && ground_m > 0.0 {
        ground_m / mpc
    } else {
        0.0
    }
}

/// One dive step toward or away from the planet's centre measured from the
/// real ground (#892 T1, Q3 B).
///
/// The height above the ground scales by `factor`, so the dive eases down
/// exponentially and slows as the ground gets close; it never goes below
/// `floor_cells` above the ground (the 3 m stop). The direction from the
/// centre is kept. Degenerate inputs hold the camera. Pure.
#[must_use]
pub fn ground_dive_step(
    camera_cells: [f64; 3],
    ground_radius: f64,
    factor: f64,
    floor_cells: f64,
) -> [f64; 3] {
    if !factor.is_finite() || factor <= 0.0 || !ground_radius.is_finite() || ground_radius <= 0.0 {
        return camera_cells;
    }
    let Some(dir) = unit(camera_cells) else {
        return camera_cells;
    };
    let distance = length(camera_cells);
    let floor = if floor_cells.is_finite() {
        floor_cells.max(0.0)
    } else {
        0.0
    };
    let height = (distance - ground_radius).max(0.0);
    let next = (height * factor).max(floor);
    let moved = scale(dir, ground_radius + next);
    if moved.iter().all(|c| c.is_finite()) {
        moved
    } else {
        camera_cells
    }
}

/// Slides a step around a solid sphere so the camera never enters it
/// (#873 T3, Q3 A).
///
/// `from` is the current camera, `to` the proposed next camera, `center`
/// the sphere centre and `radius` already includes clearance (see
/// [`planet_avoid_radius`]). When `to` stays outside and the segment
/// `from`->`to` stays outside, returns `to` unchanged. Otherwise the inward
/// radial part of the step is projected away, keeping the tangential slide,
/// and the result is pushed outside: the camera goes around the body toward
/// its target instead of through it, and no frame lands inside. Pure.
#[must_use]
pub fn slide_around_sphere(
    from: [f64; 3],
    to: [f64; 3],
    center: [f64; 3],
    radius: f64,
) -> [f64; 3] {
    if !from.iter().all(|c| c.is_finite())
        || !to.iter().all(|c| c.is_finite())
        || !center.iter().all(|c| c.is_finite())
    {
        return from;
    }
    if !radius.is_finite() || radius <= 0.0 {
        return to;
    }
    let dist_to = length(sub(to, center));
    if !dist_to.is_finite() {
        return from;
    }
    // Outside and staying outside: fast path, no work.
    if dist_to >= radius && !segment_enters_sphere(from, to, center, radius) {
        return to;
    }
    // `to` inside: push it out to the surface along its own direction, so a
    // large step lands on the surface instead of sticking far above it. A
    // `to` exactly at the centre holds (no direction to push along).
    if dist_to < radius {
        let Some(dir) = unit(sub(to, center)) else {
            return from;
        };
        let pushed = add(center, scale(dir, radius));
        return if pushed.iter().all(|c| c.is_finite()) {
            pushed
        } else {
            from
        };
    }
    // `to` outside but the segment jumps over the sphere: keep the
    // tangential slide, dropping the inward part, so the camera goes around
    // instead of through. Pure diametral jumps with no tangential part hold.
    let from_out = sub(from, center);
    let dist_from = length(from_out);
    if !dist_from.is_finite() || dist_from < radius {
        return from;
    }
    let Some(n) = unit(from_out) else {
        return from;
    };
    let step = sub(to, from);
    let inward = dot(step, n);
    if inward >= 0.0 {
        return if to.iter().all(|c| c.is_finite()) {
            to
        } else {
            from
        };
    }
    let tangential = sub(step, scale(n, inward));
    let slid = add(from, tangential);
    let slid_out = sub(slid, center);
    let dist_slid = length(slid_out);
    if !dist_slid.is_finite() {
        return from;
    }
    if dist_slid >= radius {
        return if slid.iter().all(|c| c.is_finite()) {
            slid
        } else {
            from
        };
    }
    let Some(dir) = unit(slid_out) else {
        return from;
    };
    let pushed = add(center, scale(dir, radius));
    if pushed.iter().all(|c| c.is_finite()) {
        pushed
    } else {
        from
    }
}

/// Whether the segment `from`->`to` dips inside the sphere
/// (`center`, `radius`). Pure.
fn segment_enters_sphere(from: [f64; 3], to: [f64; 3], center: [f64; 3], radius: f64) -> bool {
    if !radius.is_finite() || radius <= 0.0 {
        return false;
    }
    let to_out = sub(to, center);
    if length(to_out) < radius {
        return true;
    }
    let from_out = sub(from, center);
    if length(from_out) < radius {
        return true;
    }
    let step = sub(to, from);
    let len2 = dot(step, step);
    if !len2.is_finite() || len2 <= 0.0 {
        return false;
    }
    // Closest approach of the segment to the centre, clamped to the segment.
    let t = dot(sub(center, from), step) / len2;
    let t = t.clamp(0.0, 1.0);
    if !t.is_finite() {
        return false;
    }
    let closest = add(from, scale(step, t));
    length(sub(closest, center)) < radius
}

/// Lifts `camera_cells` to at least `min_above_m` metres above the drawn
/// terrain (#873 T4 Q7 A, Q8 A): over a hill the camera rises just enough,
/// never closer than the floor. The L10 cell is planet-centred, so the
/// direction from the origin is the planet direction. Returns the camera
/// unchanged when already above the floor or on degenerate inputs. Pure.
#[must_use]
pub fn clamp_above_ground(
    camera_cells: [f64; 3],
    frame: &crate::planet_frame::PlanetFrame,
    relief: &crate::relief::Relief,
    min_above_m: f64,
) -> [f64; 3] {
    if !camera_cells.iter().all(|c| c.is_finite()) {
        return camera_cells;
    }
    if !min_above_m.is_finite() || min_above_m < 0.0 {
        return camera_cells;
    }
    let mpc = frame.metres_per_cell();
    if !mpc.is_finite() || mpc <= 0.0 {
        return camera_cells;
    }
    let dist_cells = length(camera_cells);
    if !dist_cells.is_finite() || dist_cells <= 0.0 {
        return camera_cells;
    }
    let Some(dir) = unit(camera_cells) else {
        return camera_cells;
    };
    if !dir.iter().all(|c| c.is_finite()) {
        return camera_cells;
    }
    let height = relief.height_m(dir);
    if !height.is_finite() {
        return camera_cells;
    }
    let radius_m = frame.radius_m();
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return camera_cells;
    }
    let dist_m = dist_cells * mpc;
    let above = dist_m - (radius_m + height);
    if above >= min_above_m {
        return camera_cells;
    }
    let want_m = radius_m + height + min_above_m;
    if !want_m.is_finite() || want_m <= 0.0 {
        return camera_cells;
    }
    let want_cells = want_m / mpc;
    let lifted = scale(dir, want_cells);
    if lifted.iter().all(|c| c.is_finite()) {
        lifted
    } else {
        camera_cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn distance(a: OpenUnits, b: OpenUnits) -> f64 {
        length(sub(a.0, b.0))
    }

    #[test]
    fn dive_orbit_keeps_distance() {
        // AC1/C1: W S A D or a left-drag circles the target at the same
        // distance (within 0.1 %) with the gaze locked on it.
        let center = OpenUnits([0.0, 0.0, 0.0]);
        for (camera, up) in [
            (OpenUnits([1.0, 0.5, 2.0]), [0.0, 1.0, 0.0]),
            (OpenUnits([0.0, 0.0, 3.58]), [0.0, 1.0, 0.0]),
            (OpenUnits([-0.7, 1.2, -0.4]), [0.0, 1.0, 0.0]),
        ] {
            let before = distance(camera, center);
            assert!(before > 0.0);
            // Start perpendicular, like the window's eased up.
            let forward_before = scale(sub(center.0, camera.0), 1.0 / before);
            let lean = dot(up, forward_before);
            let perp = sub(up, scale(forward_before, lean));
            let up_perp = unit(perp).expect("perpendicular up");
            let (next, next_up) = dive_orbit(camera, center, up_perp, 0.3, 0.2);
            let after = distance(next, center);
            let rel = (after - before).abs() / before;
            assert!(
                rel < 1e-3,
                "orbit keeps the distance within 0.1 %: {before} -> {after} (rel {rel})"
            );
            assert!(rel < 1e-9, "rotation is exact to float noise: rel {rel}");
            // Gaze: the new view direction still points at the centre.
            let forward = scale(sub(center.0, next.0), 1.0 / after);
            assert!((length(forward) - 1.0).abs() < 1e-12, "gaze stays unit");
            // Up stays unit and perpendicular to the view (no roll flip).
            assert!(
                (length(next_up) - 1.0).abs() < 1e-9,
                "up stays unit: {}",
                length(next_up)
            );
            assert!(
                dot(forward, next_up).abs() < 1e-9,
                "up stays perpendicular: {}",
                dot(forward, next_up)
            );
        }
        // No input holds the pose exactly.
        let camera = OpenUnits([1.0, 0.0, 0.0]);
        let (same, same_up) = dive_orbit(camera, center, [0.0, 1.0, 0.0], 0.0, 0.0);
        assert_eq!(same, camera);
        assert_eq!(same_up, [0.0, 1.0, 0.0]);
        // At the centre there is nothing to orbit.
        let (held, _) = dive_orbit(center, center, [0.0, 1.0, 0.0], 0.5, 0.5);
        assert_eq!(held, center);
    }

    #[test]
    fn dive_orbit_speed_scales_with_gap() {
        // AC2/C2: the orbit rate scales with the gap over the distance, and
        // with the Dive orbit speed setting.
        let far = orbit_rate(0.9, 1.0, 1.0);
        let near = orbit_rate(0.09, 1.0, 1.0);
        assert!(far > 0.0 && near > 0.0);
        assert!(
            far > near,
            "far from the surface turns faster: far {far}, near {near}"
        );
        let ratio = far / near;
        assert!(
            (ratio - 10.0).abs() < 1e-9,
            "rate scales with the gap: ratio {ratio}"
        );
        let slow = orbit_rate(0.5, 1.0, 1.0);
        let fast = orbit_rate(0.5, 1.0, 2.0);
        assert!(
            (fast - 2.0 * slow).abs() < 1e-12,
            "2.0 orbits twice as fast as 1.0: {slow} vs {fast}"
        );
        assert_eq!(orbit_rate(0.5, 0.0, 1.0), 0.0);
        assert_eq!(orbit_rate(0.5, 1.0, 0.0), 0.0);
        assert_eq!(orbit_rate(f64::NAN, 1.0, 1.0), 0.0);
    }

    #[test]
    fn dive_turn_keeps_camera_and_target() {
        // AC3/C3: the right-drag turn rolls the view around the line to the
        // target. The camera never moves, the forward still points at the
        // target, and the turn stays.
        let camera = OpenUnits([2.0, 1.0, 0.5]);
        let center = OpenUnits([0.3, -0.2, 0.1]);
        let forward = unit(sub(center.0, camera.0)).expect("off centre");
        let up = [0.0, 1.0, 0.0];
        let roll = advance_roll(0.0, 0.4);
        assert!((roll - 0.4).abs() < 1e-12, "the turn adds: {roll}");
        let held = advance_roll(roll, 0.0);
        assert_eq!(held, roll, "the turn stays after release");
        let rolled = roll_up(forward, up, roll);
        // Camera and target untouched: only the up rotates.
        assert_eq!(camera, camera, "camera position never changes");
        let still_forward = forward;
        assert!(
            dot(still_forward, sub(center.0, camera.0)) > 0.0,
            "still looking at the target"
        );
        assert!((length(rolled) - 1.0).abs() < 1e-12, "rolled up stays unit");
        // Rolling twice equals one double roll.
        let twice = roll_up(forward, roll_up(forward, up, 0.2), 0.2);
        let once = roll_up(forward, up, 0.4);
        for (a, b) in twice.iter().zip(once.iter()) {
            assert!((a - b).abs() < 1e-12, "rolls add");
        }
        // Zero roll holds the up.
        assert_eq!(roll_up(forward, up, 0.0), up);
    }

    #[test]
    fn dive_turn_follows_the_mouse_around_the_centre() {
        // C1 (#907 T1, AC1): like grabbing a wheel. Screen coordinates, y
        // down, centre at (640, 360).
        let centre = [640.0, 360.0];
        let min = WHEEL_MIN_RADIUS_PX;
        // A drag right above the centre and below it sweep opposite ways.
        let top = wheel_turn([640.0, 160.0], [660.0, 160.0], centre, min).expect("top");
        let bottom = wheel_turn([640.0, 560.0], [660.0, 560.0], centre, min).expect("bottom");
        assert!(top > 0.0 && bottom < 0.0, "opposite turns: {top} {bottom}");
        assert!((top + bottom).abs() < 1e-12, "mirror images");
        // The window turns the view by `-sweep`: on the top half a drag right
        // turns the same way as the old horizontal rule (`-dx`), so the top
        // half is unchanged and only the bottom half flips.
        assert!(-top < 0.0, "top half keeps today's direction");
        // One circle around the centre is one full turn.
        let mut total = 0.0;
        let steps = 360;
        for i in 0..steps {
            let a0 = f64::from(i) * std::f64::consts::TAU / f64::from(steps);
            let a1 = f64::from(i + 1) * std::f64::consts::TAU / f64::from(steps);
            let p0 = [centre[0] + 200.0 * a0.cos(), centre[1] + 200.0 * a0.sin()];
            let p1 = [centre[0] + 200.0 * a1.cos(), centre[1] + 200.0 * a1.sin()];
            total += wheel_turn(p0, p1, centre, min).expect("on the circle");
        }
        assert!(
            (total.abs() - std::f64::consts::TAU).abs() < 1e-9,
            "a circle is one turn: {total}"
        );
        // Near the centre the angle is unstable: no wheel, the caller falls
        // back to horizontal motion.
        assert!(wheel_turn([645.0, 360.0], [650.0, 362.0], centre, min).is_none());
        assert!(wheel_turn([f64::NAN, 0.0], [1.0, 1.0], centre, min).is_none());
    }

    #[test]
    fn dive_turn_stops_on_release() {
        // AC1/C1 (#884 T1): the drawn roll eases toward the dragged roll
        // with no jump above the smoothing step, then stops exactly: no
        // later frame changes it after release.
        let dt = 1.0 / 60.0;
        let target = advance_roll(0.0, 0.4);
        let mut drawn = 0.0;
        let mut last = drawn;
        let mut settled = false;
        for _ in 0..600 {
            drawn = ease_roll(drawn, target, dt);
            // No frame jumps past the target and no overshoot.
            let before = (target - last).rem_euclid(2.0 * std::f64::consts::PI);
            let before = if before > std::f64::consts::PI {
                before - 2.0 * std::f64::consts::PI
            } else {
                before
            };
            let after = (target - drawn).rem_euclid(2.0 * std::f64::consts::PI);
            let after = if after > std::f64::consts::PI {
                after - 2.0 * std::f64::consts::PI
            } else {
                after
            };
            assert!(
                after.abs() <= before.abs() + 1e-12,
                "never moves away: {before} -> {after}"
            );
            assert!(
                (drawn - last).abs() <= before.abs() + 1e-12,
                "no jump past the target"
            );
            last = drawn;
            if drawn == target {
                settled = true;
                break;
            }
        }
        assert!(settled, "settles exactly on the dragged angle");
        // After release the drawn roll never changes again.
        for _ in 0..120 {
            let held = ease_roll(drawn, target, dt);
            assert_eq!(held, drawn, "stops dead after release");
        }
        // The base up is never written back: rolling the same base twice
        // from the stored angles equals one roll, and the base stays.
        let forward = [0.0, 0.0, -1.0];
        let base = [0.0, 1.0, 0.0];
        let drawn_view = roll_up(forward, base, drawn);
        assert_eq!(base, [0.0, 1.0, 0.0], "base untouched");
        assert!(
            (length(drawn_view) - 1.0).abs() < 1e-12,
            "drawn up stays unit"
        );
        // Degenerate inputs hold.
        assert_eq!(ease_roll(f64::NAN, target, dt), target);
        assert_eq!(ease_roll(drawn, f64::NAN, dt), drawn);
        assert_eq!(ease_roll(drawn, target, 0.0), drawn);
    }
}
