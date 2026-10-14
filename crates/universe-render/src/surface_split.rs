//! Where the planet surface splits (#598, split from `surface_mesh`).
//!
//! The one icosphere surface (ADR 0024) starts as the 20 base faces split
//! once everywhere (the 80-triangle planet). Past that default a triangle
//! splits on one rule (Spec v2 of #598): while it is **in view** and its
//! edges look longer than about [`SPLIT_PX`] on screen. The view is the
//! camera's cone, widened by [`VIEW_MARGIN`] so turning never shows coarse
//! patches; faces turned away sit behind the horizon and stay coarse.
//! No level, place or target limits the depth, only the frame's own max
//! depth (24 on Earth, one more per doubling of radius, #685), so the
//! ground where you look keeps about the same density at every height,
//! through every shell level (#632).
//!
//! A priority queue spends the [`TRIANGLE_CAP`] budget in a fixed order:
//! the centre of the view by angle, then the rest of the view by size on
//! screen; leftovers stay coarse. Everything here is pure `f64` and
//! deterministic.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use universe_core::icosphere::{TriAddr, base_corners, split_corners};
use universe_core::surface::SurfaceFrame;

/// Typical edge of a triangle in view, in (logical) screen pixels (R2 A).
/// A split halves the edges, so a triangle splits while its longest edge
/// is over this times the square root of two, and the drawn edges end
/// between about 28 and 57 px, 40 on the geometric average. Tuned with
/// [`TRIANGLE_CAP`].
pub(crate) const SPLIT_PX: f64 = 40.0;

/// Longest on-screen edge a triangle in view keeps without splitting.
const SPLIT_LIMIT_PX: f64 = SPLIT_PX * std::f64::consts::SQRT_2;

/// Most triangles one mesh holds (R2 A: a phone budget).
pub(crate) const TRIANGLE_CAP: usize = 6_000;

/// The view cone is this much wider than the screen (a 10% margin).
pub(crate) const VIEW_MARGIN: f64 = 1.1;

/// Share of the view cone counted as its centre for the budget order.
const CENTRE_SHARE: f64 = 0.5;

/// Default split depth everywhere: the 80-triangle planet.
const DEFAULT_DEPTH: u8 = 1;

/// Window height assumed by a view without a camera (tests and tools).
const REFERENCE_HEIGHT_PX: f64 = 1080.0;

/// One drawn triangle: its address and its corners in cell units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DrawnTriangle {
    /// Icosphere address.
    pub(crate) addr: TriAddr,
    /// Corners on the flat-faced surface, in open-cell units.
    pub(crate) corners: [[f64; 3]; 3],
}

/// The camera as the split sees it, in open-cell units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SplitView {
    /// Camera position.
    pub(crate) position: [f64; 3],
    /// Unit view direction.
    pub(crate) forward: [f64; 3],
    /// Unit screen right.
    right: [f64; 3],
    /// Unit screen up.
    up: [f64; 3],
    /// Tangents of the half-width and half-height of the screen, margin
    /// included; infinite sees all around.
    tan_x: f64,
    /// See `tan_x`.
    tan_y: f64,
    /// Half-angle through the screen corners, margin included (radians),
    /// for the budget's view centre.
    pub(crate) half_angle: f64,
    /// Focal length in screen pixels.
    pub(crate) focal_px: f64,
}

impl SplitView {
    /// The view of a perspective camera looking along `forward` with `up`,
    /// vertical field of view `fov` (radians), `aspect` (width over height)
    /// and a window `height_px` pixels tall: the screen rectangle, widened
    /// by [`VIEW_MARGIN`].
    #[must_use]
    pub(crate) fn camera(
        position: [f64; 3],
        forward: [f64; 3],
        up: [f64; 3],
        fov: f64,
        aspect: f64,
        height_px: f64,
    ) -> SplitView {
        let forward = unit(forward);
        let mut right = cross(forward, up);
        if length(right) < 1e-9 {
            right = cross(forward, any_across(forward));
        }
        let right = unit(right);
        let up = cross(right, forward);
        let tan_y = (fov * 0.5).tan().max(1e-12);
        let tan_x = tan_y * aspect;
        let corner = tan_y * (1.0 + aspect * aspect).sqrt() * VIEW_MARGIN;
        SplitView {
            position,
            forward,
            right,
            up,
            tan_x: tan_x * VIEW_MARGIN,
            tan_y: tan_y * VIEW_MARGIN,
            half_angle: corner.atan(),
            focal_px: height_px.max(1.0) * 0.5 / tan_y,
        }
    }

    /// A view that sees all around (tests and tools without a camera), at
    /// the pixel scale of a 1080 px window with a 45 degree field of view.
    #[must_use]
    pub(crate) fn all_around(position: [f64; 3]) -> SplitView {
        SplitView {
            position,
            forward: [0.0, 0.0, -1.0],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            tan_x: f64::INFINITY,
            tan_y: f64::INFINITY,
            half_angle: std::f64::consts::PI,
            focal_px: REFERENCE_HEIGHT_PX * 0.5 / std::f64::consts::FRAC_PI_8.tan(),
        }
    }

    /// Angle from the view direction to the nearest point of the sphere
    /// at `centre` with `reach` (zero when the camera is inside it).
    #[must_use]
    pub(crate) fn angle_to(&self, centre: [f64; 3], reach: f64) -> f64 {
        let to = sub(centre, self.position);
        let distance = length(to);
        if distance <= reach || distance <= 0.0 {
            return 0.0;
        }
        let cos = (dot(to, self.forward) / distance).clamp(-1.0, 1.0);
        (cos.acos() - (reach / distance).asin()).max(0.0)
    }

    /// Whether the sphere at `centre` with `reach` touches the view: the
    /// screen rectangle with its margin, in front of the camera.
    #[must_use]
    pub(crate) fn sees(&self, centre: [f64; 3], reach: f64) -> bool {
        if self.tan_x.is_infinite() {
            return true;
        }
        let to = sub(centre, self.position);
        let ahead = dot(to, self.forward);
        if ahead < -reach {
            return false;
        }
        // Each side plane passes through the camera; its inward normal
        // leans toward the view direction by the side's tangent.
        let (x, y) = (dot(to, self.right), dot(to, self.up));
        let side = |tan: f64, across: f64| (ahead * tan - across) / tan.hypot(1.0) >= -reach;
        side(self.tan_x, x) && side(self.tan_x, -x) && side(self.tan_y, y) && side(self.tan_y, -y)
    }

    /// How big `corners` look on screen, in pixels: the edge of the
    /// equal-sided triangle with the same on-screen area (pinhole
    /// projection). Ground seen edge-on looks thin, so it stops splitting
    /// sooner. A triangle reaching behind the camera counts as infinite.
    #[must_use]
    pub(crate) fn edge_px(&self, corners: [[f64; 3]; 3]) -> f64 {
        let mut screen = [[0.0; 2]; 3];
        for (point, corner) in screen.iter_mut().zip(corners) {
            let to = sub(corner, self.position);
            let ahead = dot(to, self.forward);
            if ahead <= 1e-12 * length(to).max(1e-300) {
                return f64::INFINITY;
            }
            *point = [dot(to, self.right) / ahead, dot(to, self.up) / ahead];
        }
        let [a, b, c] = screen;
        let area = ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() * 0.5;
        // An equal-sided triangle of edge e has area e^2 * sqrt(3) / 4.
        (area * 4.0 / 3f64.sqrt()).sqrt() * self.focal_px
    }
}

/// Centre and reach of three cell-space corners.
#[must_use]
pub(crate) fn centre_reach(corners: [[f64; 3]; 3]) -> ([f64; 3], f64) {
    let [a, b, c] = corners;
    let centre = [
        (a[0] + b[0] + c[0]) / 3.0,
        (a[1] + b[1] + c[1]) / 3.0,
        (a[2] + b[2] + c[2]) / 3.0,
    ];
    let reach = [a, b, c]
        .iter()
        .map(|corner| gap(*corner, centre))
        .fold(0.0, f64::max);
    (centre, reach)
}

/// Distance between two points.
#[must_use]
pub(crate) fn gap(a: [f64; 3], b: [f64; 3]) -> f64 {
    length(sub(a, b))
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// A direction not along `a`.
fn any_across(a: [f64; 3]) -> [f64; 3] {
    if a[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    }
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let l = length(a);
    if l.is_finite() && l > 0.0 {
        [a[0] / l, a[1] / l, a[2] / l]
    } else {
        [0.0, 0.0, -1.0]
    }
}

/// Budget class of a triangle that wants to split: lower first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Class {
    /// Below the default split: always first.
    Default = 0,
    /// In the centre of the view.
    Centre = 1,
    /// Elsewhere in the view.
    Rest = 2,
}

/// Whether the flat triangle's outer side faces the camera. Past the
/// default every triangle lies on one face of the convex surface, so one
/// facing away sits behind the horizon and is never seen.
fn faces(corners: [[f64; 3]; 3], centre: [f64; 3], planet: [f64; 3], view: &SplitView) -> bool {
    let [a, b, c] = corners;
    let mut normal = cross(sub(b, a), sub(c, a));
    if dot(normal, sub(centre, planet)) < 0.0 {
        normal = [-normal[0], -normal[1], -normal[2]];
    }
    view.tan_x.is_infinite() || dot(normal, sub(view.position, a)) > 0.0
}

/// Why and how urgently a triangle wants to split, or `None` to stay.
#[must_use]
fn wants_split(
    addr: TriAddr,
    corners: [[f64; 3]; 3],
    view: &SplitView,
    planet: [f64; 3],
    max_depth: u8,
) -> Option<(Class, f64)> {
    let depth = addr.depth();
    if depth < DEFAULT_DEPTH {
        return Some((Class::Default, 0.0));
    }
    let (centre, reach) = centre_reach(corners);
    if depth >= max_depth || !view.sees(centre, reach) || !faces(corners, centre, planet, view) {
        return None;
    }
    let pixels = view.edge_px(corners);
    if pixels <= SPLIT_LIMIT_PX {
        return None;
    }
    let angle = view.angle_to(centre, reach);
    if angle <= view.half_angle * CENTRE_SHARE {
        Some((Class::Centre, angle))
    } else {
        // Larger on screen first: the key grows as the triangle shrinks.
        Some((Class::Rest, 1.0 / pixels))
    }
}

/// One queued triangle with its budget order.
#[derive(Clone, Copy, Debug)]
struct Queued {
    class: Class,
    key: f64,
    order: u64,
    addr: TriAddr,
    units: [[f64; 3]; 3],
    corners: [[f64; 3]; 3],
}

impl PartialEq for Queued {
    fn eq(&self, other: &Queued) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Queued {}

impl PartialOrd for Queued {
    fn partial_cmp(&self, other: &Queued) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Queued {
    /// Max-heap order: the lowest class, then the lowest key, then the
    /// earliest queued pops first (deterministic ties).
    fn cmp(&self, other: &Queued) -> Ordering {
        other
            .class
            .cmp(&self.class)
            .then(other.key.total_cmp(&self.key))
            .then(other.order.cmp(&self.order))
    }
}

/// Triangles of the open surface split for `view` (#598).
///
/// From the 20 base faces, the triangle that wants to split most (see
/// [`wants_split`] and the module docs) splits first, until [`TRIANGLE_CAP`]
/// triangles would be exceeded; everything left stays as it is. Corners are
/// memoised per unit-sphere direction, so shared corners read the surface
/// once and land on identical positions.
#[must_use]
pub(crate) fn surface_triangles(frame: &SurfaceFrame, view: &SplitView) -> Vec<DrawnTriangle> {
    let planet = [0.0; 3];
    let max_depth = frame.max_depth();
    let mut cache: HashMap<[u64; 3], [f64; 3]> = HashMap::new();
    let mut corner = |unit: [f64; 3]| -> [f64; 3] {
        let key = [unit[0].to_bits(), unit[1].to_bits(), unit[2].to_bits()];
        *cache
            .entry(key)
            .or_insert_with(|| frame.surface_point(unit))
    };
    let mut heap: BinaryHeap<Queued> = BinaryHeap::new();
    let mut out = Vec::new();
    let mut order = 0u64;
    let mut place = |addr: TriAddr,
                     units: [[f64; 3]; 3],
                     corners: [[f64; 3]; 3],
                     heap: &mut BinaryHeap<Queued>,
                     out: &mut Vec<DrawnTriangle>| {
        match wants_split(addr, corners, view, planet, max_depth) {
            Some((class, key)) => {
                order += 1;
                heap.push(Queued {
                    class,
                    key,
                    order,
                    addr,
                    units,
                    corners,
                });
            }
            None => out.push(DrawnTriangle { addr, corners }),
        }
    };
    for face in 0..20u8 {
        if let (Some(addr), Some(units)) = (TriAddr::root(face), base_corners(face)) {
            let corners = [corner(units[0]), corner(units[1]), corner(units[2])];
            place(addr, units, corners, &mut heap, &mut out);
        }
    }
    while let Some(next) = heap.pop() {
        let room = out.len() + heap.len() + 4 <= TRIANGLE_CAP;
        match next.addr.children() {
            Some(kids) if room => {
                for (kid, kid_units) in kids.into_iter().zip(split_corners(next.units)) {
                    let corners = [
                        corner(kid_units[0]),
                        corner(kid_units[1]),
                        corner(kid_units[2]),
                    ];
                    place(kid, kid_units, corners, &mut heap, &mut out);
                }
            }
            _ => out.push(DrawnTriangle {
                addr: next.addr,
                corners: next.corners,
            }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The demo journey's planet surface (L10).
    fn frame() -> SurfaceFrame {
        let chain = universe_core::nest::autopilot_path(universe_core::nav::DEMO_SEED);
        let planet = chain.get(..9).expect("journey reaches the planet");
        SurfaceFrame::for_path(universe_core::nav::DEMO_SEED, planet).expect("planet surface")
    }

    /// A spot on the planet's ground, off the face corners.
    fn spot(surface: &SurfaceFrame) -> [f64; 3] {
        surface.surface_point([0.3, 0.8, 0.5])
    }

    /// A 1080 px camera at `position` looking at `at`, 45 degree field of
    /// view.
    fn looking(position: [f64; 3], at: [f64; 3]) -> SplitView {
        SplitView::camera(
            position,
            sub(at, position),
            [0.0, 1.0, 0.0],
            std::f64::consts::FRAC_PI_4,
            16.0 / 9.0,
            1080.0,
        )
    }

    /// A camera `height_px` tall `metres` above the planet's north pole,
    /// looking down at 45 degrees.
    fn over_ground(surface: &SurfaceFrame, metres: f64, height_px: f64) -> SplitView {
        let ground = surface.ground_height(0.0, 0.0);
        let height = metres / surface.metres_per_unit();
        SplitView::camera(
            [0.0, ground + height, 0.0],
            [0.0, -1.0, -1.0],
            [0.0, 1.0, 0.0],
            std::f64::consts::FRAC_PI_4,
            16.0 / 9.0,
            height_px,
        )
    }

    #[test]
    fn far_planet_is_the_icosphere_split_once() {
        let view = looking([0.0, 0.0, 50.0], [0.0; 3]);
        let triangles = surface_triangles(&frame(), &view);
        assert_eq!(triangles.len(), 80, "Q2 of #458: 80 triangles by default");
        assert!(triangles.iter().all(|t| t.addr.depth() == 1));
    }

    #[test]
    fn only_triangles_in_view_split_past_the_default() {
        // Close over a spot, looking at it: the side behind the camera
        // stays at the default split.
        let surface = frame();
        let portal = spot(&surface);
        let camera = [portal[0] * 1.3, portal[1] * 1.3, portal[2] * 1.3];
        let view = looking(camera, portal);
        let triangles = surface_triangles(&surface, &view);
        assert!(
            triangles.iter().any(|t| t.addr.depth() > 1),
            "what you see splits"
        );
        for triangle in &triangles {
            let (centre, reach) = centre_reach(triangle.corners);
            // A split triangle came from a parent in view; the parent's
            // sphere lies inside four of the child's reaches.
            if triangle.addr.depth() > 1 {
                assert!(
                    view.sees(centre, reach * 4.0),
                    "split from a parent in view"
                );
            }
        }
        // Turned away, the planet stays at the default (a big triangle
        // whose bounding sphere holds the camera may split once).
        let away = looking(camera, [camera[0] * 2.0, camera[1] * 2.0, camera[2] * 2.0]);
        let behind = surface_triangles(&surface, &away);
        assert!(
            behind.len() < 100 && behind.iter().all(|t| t.addr.depth() <= 2),
            "behind the camera nothing splits past the default"
        );
    }

    #[test]
    fn triangles_in_view_stop_near_forty_pixels_on_screen() {
        // R1 A / AC2: whatever is under them, triangles in view end at or
        // below the split limit (about 40 px), and most are past half of it.
        for metres in [200.0, 20_000.0] {
            let surface = frame();
            let view = over_ground(&surface, metres, 720.0);
            let triangles = surface_triangles(&surface, &view);
            assert!(
                triangles.len() < TRIANGLE_CAP,
                "{metres} m: the budget is not the limit here"
            );
            let planet = [0.0; 3];
            let mut seen = 0;
            let mut near_limit = 0;
            for triangle in &triangles {
                let (centre, reach) = centre_reach(triangle.corners);
                if view.sees(centre, reach) && faces(triangle.corners, centre, planet, &view) {
                    let pixels = view.edge_px(triangle.corners);
                    assert!(
                        pixels <= SPLIT_LIMIT_PX,
                        "{metres} m: a seen triangle at {pixels} px"
                    );
                    seen += 1;
                    if pixels > SPLIT_LIMIT_PX * 0.25 {
                        near_limit += 1;
                    }
                }
            }
            assert!(seen > 1000, "{metres} m: plenty of ground in view ({seen})");
            assert!(
                near_limit * 10 >= seen * 9,
                "{metres} m: {near_limit} of {seen} near the limit"
            );
        }
    }

    #[test]
    fn plain_ground_close_by_splits_deep() {
        // R1 A: no cap by level or place. Standing 20 m over the ground (a
        // shell level, #632) splits it far past the old building depth
        // (19), down toward the addressable limit.
        let surface = frame();
        let triangles = surface_triangles(&surface, &over_ground(&surface, 20.0, 720.0));
        let deepest = triangles
            .iter()
            .map(|t| t.addr.depth())
            .max()
            .expect("triangles");
        assert!(deepest > 19, "ground reaches depth {deepest}");
        assert!(
            triangles
                .iter()
                .all(|t| t.addr.depth() <= frame().max_depth()),
            "splits stop at the frame's max depth"
        );
    }

    #[test]
    fn the_budget_holds_even_on_the_ground() {
        let surface = frame();
        for metres in [0.01, 1.7, 50.0] {
            let triangles = surface_triangles(&surface, &over_ground(&surface, metres, 2160.0));
            assert!(
                triangles.len() <= TRIANGLE_CAP,
                "{metres} m within the budget"
            );
        }
    }

    #[test]
    fn a_full_budget_keeps_the_view_centre_then_the_largest() {
        let a = Queued {
            class: Class::Default,
            key: 5.0,
            order: 9,
            addr: TriAddr::root(0).expect("root"),
            units: [[0.0; 3]; 3],
            corners: [[0.0; 3]; 3],
        };
        let b = Queued {
            class: Class::Centre,
            key: 0.0,
            order: 1,
            ..a
        };
        let c = Queued {
            class: Class::Centre,
            key: 0.3,
            order: 0,
            ..a
        };
        let d = Queued {
            class: Class::Rest,
            key: 0.0,
            order: 0,
            ..a
        };
        let mut heap: BinaryHeap<Queued> = [d, c, b, a].into_iter().collect();
        let popped: Vec<(Class, f64)> = std::iter::from_fn(|| heap.pop())
            .map(|q| (q.class, q.key))
            .collect();
        assert_eq!(
            popped,
            vec![
                (Class::Default, 5.0),
                (Class::Centre, 0.0),
                (Class::Centre, 0.3),
                (Class::Rest, 0.0)
            ],
            "the default split, then nearest the centre, then the rest"
        );
    }
}
