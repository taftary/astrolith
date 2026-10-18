//! The player's low-poly figure in core (#755).
//!
//! A simple lit person built in code: head, torso, two arms and two legs at
//! true size 1.8 m, facing the body's heading. No model files. The waiting
//! body and the active body share this one description (`E-CORE-API`); the
//! window draws it through one mesh (`figure_draw`). Shading lives in
//! [`crate::light`]; this module owns shape and placement only.
//!
//! Everything here is pure and headless (`E-CORE-NO-BEVY`, `E-DET-TIERS`)
//! and allocates nothing per frame (`E-HOT-NOALLOC`): fixed arrays by value.

/// Share of the body height where the head box starts.
pub const HEAD_BOTTOM: f64 = 0.83;
/// Share where the head box ends (the top of the figure).
pub const HEAD_TOP: f64 = 1.0;
/// Half width of the head box, as a share of the height.
pub const HEAD_HALF: f64 = 0.07;

/// Which part of the figure a triangle belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FigurePart {
    /// Head and nose (carry the accent colour).
    Head,
    /// Torso (suit colour).
    Torso,
    /// Upper arms and forearms (accent colour, hands included).
    Arm,
    /// Thighs and shins (suit colour).
    Leg,
}

/// One triangle of the figure in open-cell units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FigureTriangle {
    /// The three corners in open-cell units.
    pub vertices: [[f64; 3]; 3],
    /// Outward unit normal in open-cell units.
    pub normal: [f64; 3],
    /// Which part this triangle belongs to.
    pub part: FigurePart,
}

/// Boxes of the figure as `(min, max, part)` in local shares
/// `(side, rise, ahead)`: side along `right`, rise along `up`,
/// ahead along `forward`, all as shares of the body height.
const BOXES: [([f64; 3], [f64; 3], FigurePart); 11] = [
    ([-0.07, 0.83, -0.07], [0.07, 1.0, 0.07], FigurePart::Head),
    ([-0.02, 0.87, 0.07], [0.02, 0.92, 0.11], FigurePart::Head),
    ([-0.16, 0.50, -0.09], [0.16, 0.83, 0.09], FigurePart::Torso),
    ([-0.25, 0.64, -0.045], [-0.16, 0.80, 0.045], FigurePart::Arm),
    ([-0.25, 0.48, -0.045], [-0.16, 0.64, 0.045], FigurePart::Arm),
    ([0.16, 0.64, -0.045], [0.25, 0.80, 0.045], FigurePart::Arm),
    ([0.16, 0.48, -0.045], [0.25, 0.64, 0.045], FigurePart::Arm),
    ([-0.16, 0.25, -0.07], [-0.02, 0.50, 0.07], FigurePart::Leg),
    ([-0.16, 0.0, -0.07], [-0.02, 0.25, 0.07], FigurePart::Leg),
    ([0.02, 0.25, -0.07], [0.16, 0.50, 0.07], FigurePart::Leg),
    ([0.02, 0.0, -0.07], [0.16, 0.25, 0.07], FigurePart::Leg),
];

/// Triangles per box (two per face, six faces).
pub const TRIANGLES_PER_BOX: usize = 12;
/// Triangles of the full figure (11 boxes).
pub const FIGURE_TRIANGLES: usize = 132;
/// Triangles without the head (first person): head and nose boxes removed.
pub const FIRST_PERSON_TRIANGLES: usize = 108;
/// Head triangles at the start of the full array (two boxes).
const HEAD_TRIANGLES: usize = 24;

/// Near-plane share of the body height: the window camera keeps
/// `near = height * 0.05` in normal mode, so every first-person vertex must
/// sit farther than this from the eye.
pub const FIRST_PERSON_NEAR_SHARE: f64 = 0.05;

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// Local shares `(side, rise, ahead)` to open-cell units.
fn place(
    feet: [f64; 3],
    up: [f64; 3],
    forward: [f64; 3],
    right: [f64; 3],
    height: f64,
    local: [f64; 3],
) -> [f64; 3] {
    add(
        feet,
        add(
            scale(up, local[1] * height),
            add(
                scale(right, local[0] * height),
                scale(forward, local[2] * height),
            ),
        ),
    )
}

/// World normal for a local axis direction.
fn axis_normal(
    up: [f64; 3],
    forward: [f64; 3],
    right: [f64; 3],
    axis: usize,
    sign: f64,
) -> [f64; 3] {
    let base = match axis {
        0 => right,
        1 => up,
        _ => forward,
    };
    scale(base, sign)
}

/// Twelve triangles of one box, written at `out` from `cursor`.
#[allow(
    clippy::too_many_arguments,
    reason = "one box's full placement; a struct would only rename it"
)]
#[expect(
    clippy::indexing_slicing,
    reason = "E-NO-PANIC: corners 0..8, faces 0..5, all fixed"
)]
fn box_triangles(
    min: [f64; 3],
    max: [f64; 3],
    part: FigurePart,
    feet: [f64; 3],
    up: [f64; 3],
    forward: [f64; 3],
    right: [f64; 3],
    height: f64,
    out: &mut [FigureTriangle],
    cursor: &mut usize,
) {
    // Eight corners in local shares.
    let mut corner_local = [[0.0; 3]; 8];
    for (i, corner) in corner_local.iter_mut().enumerate() {
        corner[0] = if i & 1 == 0 { min[0] } else { max[0] };
        corner[1] = if i & 2 == 0 { min[1] } else { max[1] };
        corner[2] = if i & 4 == 0 { min[2] } else { max[2] };
    }
    let mut corner_world = [[0.0; 3]; 8];
    for (i, local) in corner_local.iter().enumerate() {
        corner_world[i] = place(feet, up, forward, right, height, *local);
    }
    // Six faces as (axis, sign, four corner indices in fan order).
    // Corner bit order: 0 = side, 1 = rise, 2 = ahead.
    const FACES: [(usize, f64, [usize; 4]); 6] = [
        (0, -1.0, [0, 2, 6, 4]),
        (0, 1.0, [1, 5, 7, 3]),
        (1, -1.0, [0, 4, 5, 1]),
        (1, 1.0, [2, 3, 7, 6]),
        (2, -1.0, [0, 1, 3, 2]),
        (2, 1.0, [4, 6, 7, 5]),
    ];
    for (axis, sign, quad) in FACES {
        let normal = axis_normal(up, forward, right, axis, sign);
        let tris = [[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]];
        for tri in tris {
            let vertices = [
                corner_world[tri[0]],
                corner_world[tri[1]],
                corner_world[tri[2]],
            ];
            if let Some(slot) = out.get_mut(*cursor) {
                *slot = FigureTriangle {
                    vertices,
                    normal,
                    part,
                };
            }
            *cursor += 1;
        }
    }
}

/// Full low-poly figure at `feet` with `up`, facing `forward` (`right`
/// completes the frame), `height` tall in open-cell units.
///
/// Fixed array by value, no allocation (`E-HOT-NOALLOC`). The top sits one
/// height above the feet; the nose protrudes along `forward` so the heading
/// reads. Pure (`E-DET-TIERS`).
#[must_use]
pub fn figure_triangles(
    feet: [f64; 3],
    up: [f64; 3],
    forward: [f64; 3],
    right: [f64; 3],
    height: f64,
) -> [FigureTriangle; FIGURE_TRIANGLES] {
    let mut out = [FigureTriangle {
        vertices: [[0.0; 3]; 3],
        normal: [0.0, 1.0, 0.0],
        part: FigurePart::Torso,
    }; FIGURE_TRIANGLES];
    let mut cursor = 0usize;
    for (min, max, part) in BOXES {
        box_triangles(
            min,
            max,
            part,
            feet,
            up,
            forward,
            right,
            height,
            &mut out,
            &mut cursor,
        );
    }
    out
}

/// First-person figure: the same figure without the head.
///
/// Looking down shows the torso, arms and legs; the head is never drawn so
/// it can never block the view (Q4 A). Same order as the tail of
/// [`figure_triangles`]. Pure, no allocation.
#[must_use]
pub fn first_person_triangles(
    feet: [f64; 3],
    up: [f64; 3],
    forward: [f64; 3],
    right: [f64; 3],
    height: f64,
) -> [FigureTriangle; FIRST_PERSON_TRIANGLES] {
    let full = figure_triangles(feet, up, forward, right, height);
    let mut out = [FigureTriangle {
        vertices: [[0.0; 3]; 3],
        normal: [0.0, 1.0, 0.0],
        part: FigurePart::Torso,
    }; FIRST_PERSON_TRIANGLES];
    for (cursor, tri) in full.iter().skip(HEAD_TRIANGLES).enumerate() {
        if let Some(slot) = out.get_mut(cursor) {
            *slot = *tri;
        }
    }
    out
}

/// Whether `part` carries the accent colour (head and hands).
#[must_use]
pub fn is_accent(part: FigurePart) -> bool {
    matches!(part, FigurePart::Head | FigurePart::Arm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::body_height;
    use crate::coords::Level;

    fn level(n: u8) -> Level {
        Level::new(n).expect("level")
    }

    fn basis(yaw: f64) -> ([f64; 3], [f64; 3]) {
        crate::body::heading_basis([0.0, 1.0, 0.0], yaw)
    }

    #[test]
    fn the_figure_is_a_true_size_low_poly_person() {
        // Built in code: fixed triangle budget, no model files.
        assert_eq!(FIGURE_TRIANGLES, 132, "11 boxes of 12 triangles");
        assert_eq!(BOXES.len(), 11);
        let mut kinds = std::collections::HashSet::new();
        for (_, _, part) in BOXES {
            kinds.insert(part);
        }
        for want in [
            FigurePart::Head,
            FigurePart::Torso,
            FigurePart::Arm,
            FigurePart::Leg,
        ] {
            assert!(kinds.contains(&want), "the figure has {want:?}");
        }
        // True size at every level: one body height tall, feet on the ground.
        for n in [1u8, 5, 9, 10] {
            let height = body_height(level(n));
            let feet = [0.0; 3];
            let (forward, right) = basis(0.7);
            let tris = figure_triangles(feet, [0.0, 1.0, 0.0], forward, right, height);
            let mut lowest = f64::INFINITY;
            let mut highest = f64::NEG_INFINITY;
            for tri in &tris {
                for v in tri.vertices {
                    let rise = (v[1] - feet[1]) / height;
                    lowest = lowest.min(rise);
                    highest = highest.max(rise);
                }
            }
            assert!((lowest - 0.0).abs() < 1e-12, "feet on the ground: L{n}");
            assert!(
                (highest - HEAD_TOP).abs() < 1e-12,
                "one body height tall: L{n}"
            );
        }
        // Facing: the nose protrudes along the heading and turns with it.
        let height = body_height(level(10));
        for yaw in [0.0, 0.7, -2.0] {
            let (forward, right) = basis(yaw);
            let tris = figure_triangles([0.0; 3], [0.0, 1.0, 0.0], forward, right, height);
            let mut nose_ahead = f64::NEG_INFINITY;
            let mut other_ahead = f64::NEG_INFINITY;
            for tri in &tris {
                for v in tri.vertices {
                    let along = v[0] * forward[0] + v[1] * forward[1] + v[2] * forward[2];
                    if tri.part == FigurePart::Head && along > nose_ahead {
                        // Head box also reaches forward; the nose reaches
                        // farthest, so the max over head triangles is the nose.
                        nose_ahead = along;
                    }
                    if tri.part != FigurePart::Head {
                        other_ahead = other_ahead.max(along);
                    }
                }
            }
            assert!(
                nose_ahead > other_ahead,
                "the nose leads the figure (yaw {yaw})"
            );
            // The nose tip sits ahead of the head front by construction.
            assert!(
                (nose_ahead - 0.11 * height).abs() < 1e-12,
                "nose tip share 0.11 (yaw {yaw})"
            );
        }
        // Normals stay unit length.
        let (forward, right) = basis(0.3);
        for tri in figure_triangles([0.0; 3], [0.0, 1.0, 0.0], forward, right, 1.0) {
            let n = (tri.normal[0] * tri.normal[0]
                + tri.normal[1] * tri.normal[1]
                + tri.normal[2] * tri.normal[2])
                .sqrt();
            assert!((n - 1.0).abs() < 1e-12, "unit normals");
        }
    }

    #[test]
    fn first_person_shows_the_body_not_the_head() {
        let height = body_height(level(10));
        let (forward, right) = basis(0.4);
        let feet = [0.1, 0.2, -0.3];
        let up = [0.0, 1.0, 0.0];
        let full = figure_triangles(feet, up, forward, right, height);
        let first = first_person_triangles(feet, up, forward, right, height);
        assert_eq!(FIRST_PERSON_TRIANGLES, 108);
        assert!(
            first.iter().all(|tri| tri.part != FigurePart::Head),
            "never the head"
        );
        let mut kinds = std::collections::HashSet::new();
        for tri in &first {
            kinds.insert(tri.part);
        }
        for want in [FigurePart::Torso, FigurePart::Arm, FigurePart::Leg] {
            assert!(kinds.contains(&want), "looking down shows {want:?}");
        }
        // Same geometry as the full figure without the head boxes.
        for (a, b) in first.iter().zip(full.iter().skip(HEAD_TRIANGLES)) {
            assert_eq!(a.vertices, b.vertices);
            assert_eq!(a.normal, b.normal);
            assert_eq!(a.part, b.part);
        }
        // Every first-person vertex sits past the near plane from the eye.
        let eye = [
            feet[0] + up[0] * height * crate::body::EYE_SHARE,
            feet[1] + up[1] * height * crate::body::EYE_SHARE,
            feet[2] + up[2] * height * crate::body::EYE_SHARE,
        ];
        let near = height * FIRST_PERSON_NEAR_SHARE;
        assert_eq!(FIRST_PERSON_NEAR_SHARE, 0.05, "shares the camera near");
        for tri in &first {
            for v in tri.vertices {
                let gap =
                    ((v[0] - eye[0]).powi(2) + (v[1] - eye[1]).powi(2) + (v[2] - eye[2]).powi(2))
                        .sqrt();
                assert!(
                    gap > near,
                    "the torso and arms sit outside the near plane: {gap} vs {near}"
                );
            }
        }
        // The accent rule: head and arms carry it.
        assert!(is_accent(FigurePart::Head));
        assert!(is_accent(FigurePart::Arm));
        assert!(!is_accent(FigurePart::Torso));
        assert!(!is_accent(FigurePart::Leg));
    }
}
