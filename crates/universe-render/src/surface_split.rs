//! Where the planet surface splits (#598, split from `surface_mesh`).
//!
//! The one icosphere surface (ADR 0024) starts as the 20 base faces split
//! once everywhere (the 80-triangle planet). Past that default a triangle
//! splits only when three things hold:
//!
//! - **In view** (Q1 A): its bounding sphere touches the camera's view cone,
//!   widened by [`VIEW_MARGIN`] so turning never shows coarse patches.
//! - **Splittable** (Q3 A): it lies over a place that leads somewhere, up
//!   to that place's depth. The cell you are in (and its opened ancestors)
//!   resolves to its depth, so its clusters and tones show, and enterable
//!   clusters reach the next level's depth, both while the camera is near
//!   or the triangle looks large ([`SPLIT_ANGLE`]). The target reaches its level depth
//!   whatever its size (Q2 A). Plain ground and closed populations stop at
//!   their level depth.
//! - **Not too deep** (Q5 A): never past [`max_depth`], the next level's
//!   depth, so the triangles you approach are the ones you enter.
//!
//! A priority queue spends the [`TRIANGLE_CAP`] budget in a fixed order
//! (Q4 A): the target first, then the centre of the view by angle, then the
//! rest of the view by size on screen; leftovers stay coarse. Everything
//! here is pure `f64` and deterministic.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use universe_core::icosphere::{TriAddr, base_corners, split_corners, tri_centre};
use universe_core::surface::{SurfaceFrame, WINDOW_RADIUS, surface_depth};

/// A triangle splits while its reach over its camera distance exceeds this.
pub(crate) const SPLIT_ANGLE: f64 = 0.5;

/// Most triangles one mesh holds (the split stops here, Q4 A).
pub(crate) const TRIANGLE_CAP: usize = 12_000;

/// A window stays resolved while the camera is within this many of its
/// cell's units (past the close distance of about ten, so entries and
/// exits never cross it).
const ZONE_TRIGGER: f64 = 12.0;

/// The view cone is this much wider than the screen (Q1 A: a 10% margin).
pub(crate) const VIEW_MARGIN: f64 = 1.1;

/// Share of the view cone counted as its centre for the budget order.
const CENTRE_SHARE: f64 = 0.5;

/// Default split depth everywhere: the 80-triangle planet.
const DEFAULT_DEPTH: u8 = 1;

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
    /// Half-angle of the cone through the screen corners, margin included
    /// (radians); `PI` or more sees all around.
    pub(crate) half_angle: f64,
}

impl SplitView {
    /// The view of a perspective camera with vertical field of view `fov`
    /// (radians) and `aspect` (width over height): the cone through the
    /// screen corners, widened by [`VIEW_MARGIN`].
    #[must_use]
    pub(crate) fn camera(
        position: [f64; 3],
        forward: [f64; 3],
        fov: f64,
        aspect: f64,
    ) -> SplitView {
        let tan_y = (fov * 0.5).tan();
        let corner = tan_y * (1.0 + aspect * aspect).sqrt() * VIEW_MARGIN;
        SplitView {
            position,
            forward: unit(forward),
            half_angle: corner.atan(),
        }
    }

    /// A view that sees all around (tests and tools without a camera).
    #[must_use]
    pub(crate) fn all_around(position: [f64; 3]) -> SplitView {
        SplitView {
            position,
            forward: [0.0, 0.0, -1.0],
            half_angle: std::f64::consts::PI,
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

    /// Whether the sphere at `centre` with `reach` touches the view cone.
    #[must_use]
    pub(crate) fn sees(&self, centre: [f64; 3], reach: f64) -> bool {
        self.angle_to(centre, reach) <= self.half_angle
    }
}

/// The open cell's target as the split sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SplitTarget {
    /// Target position in open-cell units.
    pub(crate) position: [f64; 3],
}

/// Deepest split near the camera: the next level's depth (L13: own + 2).
#[must_use]
pub(crate) fn max_depth(frame: &SurfaceFrame) -> u8 {
    frame
        .level()
        .deeper()
        .and_then(surface_depth)
        .unwrap_or(frame.depth().saturating_add(2))
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

fn unit(a: [f64; 3]) -> [f64; 3] {
    let l = length(a);
    if l.is_finite() && l > 0.0 {
        [a[0] / l, a[1] / l, a[2] / l]
    } else {
        [0.0, 0.0, -1.0]
    }
}

/// What a place lets a triangle over it split to.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Reach {
    /// The cell you are in (or an opened ancestor): its depth while the
    /// camera is near, or while the triangle looks large.
    Cell,
    /// An enterable cluster: the next level's depth while the camera is
    /// near, or while the triangle looks large.
    Enterable,
    /// The target: its depth, whatever its size.
    Target,
}

/// A window a triangle can split under.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Zone {
    /// Window centre in open-cell units.
    centre: [f64; 3],
    /// Window radius in open-cell units.
    radius: f64,
    /// Icosphere depth the window resolves to.
    depth: u8,
    /// Camera distance (open-cell units) within which the zone forces its
    /// depth.
    trigger: f64,
    /// What kind of place it is.
    reach: Reach,
}

/// Every window of the open cell a triangle can split under: the open
/// cell's and its opened ancestors' windows at their depth, each enterable
/// child at the child's depth, and the target at its depth. Triggers scale
/// with each window's own cell units, so a parent and its child agree on
/// the zones at an entry (scale-invariant, C3 of #458).
fn zones(frame: &SurfaceFrame, target: Option<SplitTarget>) -> Vec<Zone> {
    use universe_core::surface::cell_span_l10;
    let level = frame.level();
    let Some(span) = cell_span_l10(level) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (opened, centres) in frame.windows() {
        let (Some(depth), Some(opened_span)) = (surface_depth(*opened), cell_span_l10(*opened))
        else {
            continue;
        };
        let scale = opened_span / span;
        for centre in centres {
            out.push(Zone {
                centre: frame.to_cell(*centre),
                radius: WINDOW_RADIUS * scale,
                depth,
                trigger: ZONE_TRIGGER * scale,
                reach: Reach::Cell,
            });
        }
    }
    if let (Some(child), Some(ratio)) = (
        level.deeper().and_then(surface_depth),
        universe_core::nest::child_ratio(level),
    ) {
        for portal in frame.portal_positions() {
            out.push(Zone {
                centre: portal,
                radius: WINDOW_RADIUS * ratio,
                depth: child,
                trigger: ZONE_TRIGGER * ratio,
                reach: Reach::Enterable,
            });
        }
        if let Some(target) = target {
            out.push(Zone {
                centre: target.position,
                radius: WINDOW_RADIUS * ratio,
                depth: child,
                trigger: f64::INFINITY,
                reach: Reach::Target,
            });
        }
    }
    out
}

/// Budget class of a triangle that wants to split (Q4 A): lower first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Class {
    /// Under the target.
    Target = 0,
    /// In the centre of the view.
    Centre = 1,
    /// Elsewhere in the view.
    Rest = 2,
}

/// Why and how urgently a triangle wants to split, or `None` to stay.
#[must_use]
fn wants_split(
    addr: TriAddr,
    corners: [[f64; 3]; 3],
    view: &SplitView,
    zones: &[Zone],
    deepest: u8,
) -> Option<(Class, f64)> {
    let depth = addr.depth();
    let (centre, reach) = centre_reach(corners);
    if depth < DEFAULT_DEPTH {
        return Some((Class::Target, 0.0));
    }
    if depth >= deepest || !view.sees(centre, reach) {
        return None;
    }
    let distance = gap(view.position, centre).max(1e-12);
    let large = reach / distance > SPLIT_ANGLE;
    let mut target = false;
    let mut split = false;
    for zone in zones {
        if depth >= zone.depth || gap(centre, zone.centre) > zone.radius * 1.25 + reach {
            continue;
        }
        let near = gap(view.position, zone.centre) <= zone.trigger;
        match zone.reach {
            Reach::Target => {
                target = true;
                split = true;
            }
            // A cell and the enterable clusters it came from share one
            // rule, so the parent and the child agree at an entry.
            Reach::Cell | Reach::Enterable => split |= near || large,
        }
    }
    if !split {
        return None;
    }
    let angle = view.angle_to(centre, reach);
    if target {
        Some((Class::Target, angle))
    } else if angle <= view.half_angle * CENTRE_SHARE {
        Some((Class::Centre, angle))
    } else {
        // Larger on screen first: the key grows as the triangle shrinks.
        Some((Class::Rest, distance / reach.max(1e-300)))
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

/// Whether the follow patch draws for `frame` at `fade` strength (#723,
/// step 1).
///
/// Only the L10 surface while visible: the far ball (fade zero) and the
/// deeper levels keep today's drawing.
#[must_use]
pub(crate) fn patch_active(frame: &SurfaceFrame, fade: f64) -> bool {
    frame.level().get() == 10 && fade > 0.0
}

/// Base faces of the follow patch around `centre` (planet axes, #723).
///
/// The depth-1 faces (the 80-triangle planet's own tiles) whose centre
/// direction lies inside the patch radius. Sampling stays planet-fixed:
/// the same centre returns the same faces whatever the camera does, so
/// terrain never slides.
#[must_use]
pub(crate) fn patch_faces(centre: [f64; 3]) -> Vec<TriAddr> {
    let mut faces = Vec::new();
    for face in 0..20u8 {
        let Some(root) = TriAddr::root(face) else {
            continue;
        };
        let Some(kids) = root.children() else {
            continue;
        };
        for addr in kids {
            if tri_centre(addr).is_some_and(|dir| SurfaceFrame::patch_contains(centre, dir)) {
                faces.push(addr);
            }
        }
    }
    faces
}

/// Partition of the drawn surface into ball and patch (#723).
///
/// Returns `(ball, patch)`: one coarse depth-1 triangle per base face
/// outside the patch, plus the refined triangles inside it. The two sets
/// tile the sphere exactly once: no hole, no double draw, no z-fighting
/// between ball and patch. Refinement inside the patch follows the same
/// view and target rule as [`surface_triangles`].
#[must_use]
pub(crate) fn patch_partition(
    frame: &SurfaceFrame,
    centre: [f64; 3],
    view: &SplitView,
    target: Option<SplitTarget>,
) -> (Vec<DrawnTriangle>, Vec<DrawnTriangle>) {
    let patch_set: HashSet<TriAddr> = patch_faces(centre).into_iter().collect();
    let mut ball = Vec::new();
    for face in 0..20u8 {
        let (Some(root), Some(units)) = (TriAddr::root(face), base_corners(face)) else {
            continue;
        };
        let Some(kids) = root.children() else {
            continue;
        };
        for (addr, kid_units) in kids.into_iter().zip(split_corners(units)) {
            if patch_set.contains(&addr) {
                continue;
            }
            ball.push(DrawnTriangle {
                addr,
                corners: [
                    frame.surface_cell(kid_units[0]),
                    frame.surface_cell(kid_units[1]),
                    frame.surface_cell(kid_units[2]),
                ],
            });
        }
    }
    let patch: Vec<DrawnTriangle> = surface_triangles(frame, view, target)
        .into_iter()
        .filter(|triangle| {
            triangle
                .addr
                .ancestor_at(1)
                .is_some_and(|ancestor| patch_set.contains(&ancestor))
        })
        .collect();
    (ball, patch)
}

/// Triangles of the open surface split for `view` and `target` (#598).
///
/// From the 20 base faces, the triangle that wants to split most (see
/// [`wants_split`] and the module docs) splits first, until [`TRIANGLE_CAP`]
/// triangles would be exceeded; everything left stays as it is. Corners are
/// memoised per unit-sphere direction, so shared corners read the surface
/// once and land on identical positions.
#[must_use]
pub(crate) fn surface_triangles(
    frame: &SurfaceFrame,
    view: &SplitView,
    target: Option<SplitTarget>,
) -> Vec<DrawnTriangle> {
    let deepest = max_depth(frame);
    let zones = zones(frame, target);
    let mut cache: HashMap<[u64; 3], [f64; 3]> = HashMap::new();
    let mut corner = |unit: [f64; 3]| -> [f64; 3] {
        let key = [unit[0].to_bits(), unit[1].to_bits(), unit[2].to_bits()];
        *cache.entry(key).or_insert_with(|| frame.surface_cell(unit))
    };
    let mut heap: BinaryHeap<Queued> = BinaryHeap::new();
    let mut out = Vec::new();
    let mut order = 0u64;
    let mut place = |addr: TriAddr,
                     units: [[f64; 3]; 3],
                     corners: [[f64; 3]; 3],
                     heap: &mut BinaryHeap<Queued>,
                     out: &mut Vec<DrawnTriangle>| {
        match wants_split(addr, corners, view, &zones, deepest) {
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
    use std::collections::HashSet;

    /// The demo journey's surface frame at level `n` (L10-L13).
    fn frame(n: u8) -> SurfaceFrame {
        let chain = universe_core::nest::autopilot_path(universe_core::nav::DEMO_SEED);
        let named = chain
            .get(..usize::from(n) - 1)
            .expect("journey reaches the surface");
        SurfaceFrame::for_path(universe_core::nav::DEMO_SEED, named).expect("surface frame")
    }

    /// A camera at `position` looking at `at` with a 45 degree field of view.
    fn looking(position: [f64; 3], at: [f64; 3]) -> SplitView {
        SplitView::camera(
            position,
            sub(at, position),
            std::f64::consts::FRAC_PI_4,
            16.0 / 9.0,
        )
    }

    /// Centre of the first depth-1 tile: always inside its own patch.
    fn root_first_child_centre() -> [f64; 3] {
        let root = TriAddr::root(0).expect("root");
        let kid = root.children().expect("children")[0];
        tri_centre(kid).expect("tile centre")
    }

    #[test]
    fn far_planet_is_the_icosphere_split_once() {
        let view = looking([0.0, 0.0, 50.0], [0.0; 3]);
        let triangles = surface_triangles(&frame(10), &view, None);
        assert_eq!(triangles.len(), 80, "Q2 of #458: 80 triangles by default");
        assert!(triangles.iter().all(|t| t.addr.depth() == 1));
    }

    #[test]
    fn only_triangles_in_view_split_past_the_default() {
        // Close over a region portal, looking at it: the side behind the
        // camera stays at the default split.
        let surface = frame(10);
        let portal = surface.portal_positions()[0];
        let camera = [portal[0] * 1.3, portal[1] * 1.3, portal[2] * 1.3];
        let view = looking(camera, portal);
        let triangles = surface_triangles(&surface, &view, None);
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
        // From farther out with the portal as the target: facing it, the
        // target splits; turned away, the whole planet stays at the default.
        let far = [portal[0] * 3.0, portal[1] * 3.0, portal[2] * 3.0];
        let target = Some(SplitTarget { position: portal });
        let facing = surface_triangles(&surface, &looking(far, portal), target);
        assert!(
            facing.iter().any(|t| t.addr.depth() > 1),
            "facing the target"
        );
        let away = SplitView::camera(
            far,
            sub(far, portal),
            std::f64::consts::FRAC_PI_4,
            16.0 / 9.0,
        );
        let behind = surface_triangles(&surface, &away, target);
        assert_eq!(
            behind.len(),
            80,
            "behind the camera nothing splits past the default"
        );
    }

    #[test]
    fn the_target_splits_to_its_depth_even_when_small() {
        // Far from the planet every triangle looks small; the target still
        // reaches the region depth.
        let surface = frame(10);
        let portal = surface.portal_positions()[0];
        let view = looking([portal[0] * 8.0, portal[1] * 8.0, portal[2] * 8.0], portal);
        let plain = surface_triangles(&surface, &view, None);
        assert!(plain.iter().all(|t| t.addr.depth() == 1), "small: no split");
        let aimed = surface_triangles(&surface, &view, Some(SplitTarget { position: portal }));
        let under = aimed
            .iter()
            .filter(|t| {
                let (c, r) = centre_reach(t.corners);
                gap(c, portal) <= r
            })
            .map(|t| t.addr.depth())
            .max()
            .expect("a triangle under the target");
        assert_eq!(
            under,
            max_depth(&surface),
            "the target is at its level depth"
        );
    }

    #[test]
    fn plain_ground_stays_at_its_level_depth_and_enterable_places_go_deeper() {
        // Standing low over the region cell, looking down at it.
        let surface = frame(11);
        let cell_depth = surface.depth();
        let portals = surface.portal_positions();
        let camera = [0.0, 0.05, 0.0];
        let view = looking(camera, [0.0, -1.0, 0.0]);
        let triangles = surface_triangles(&surface, &view, None);
        let mut deeper_over_portal = false;
        for triangle in &triangles {
            let (centre, reach) = centre_reach(triangle.corners);
            let over_portal = portals.iter().any(|p| {
                gap(centre, *p)
                    <= WINDOW_RADIUS
                        * universe_core::nest::child_ratio(surface.level()).expect("ratio")
                        * 1.25
                        // The parent that split was about twice as wide.
                        + reach * 3.0
            });
            if triangle.addr.depth() > cell_depth {
                assert!(
                    over_portal,
                    "only enterable places split past the cell depth"
                );
                deeper_over_portal = true;
            }
        }
        assert!(
            deeper_over_portal,
            "an enterable cluster nearby splits deeper"
        );
    }

    #[test]
    fn nothing_goes_past_the_next_level_depth_or_the_budget() {
        for n in 10..=13u8 {
            let surface = frame(n);
            let portal = surface
                .portal_positions()
                .first()
                .copied()
                .unwrap_or([0.0; 3]);
            let camera = [portal[0], portal[1] + 0.01, portal[2]];
            let view = looking(camera, portal);
            let triangles =
                surface_triangles(&surface, &view, Some(SplitTarget { position: portal }));
            assert!(triangles.len() <= TRIANGLE_CAP, "L{n} within the budget");
            assert!(
                triangles
                    .iter()
                    .all(|t| t.addr.depth() <= max_depth(&surface)),
                "L{n} never past the next level's depth"
            );
        }
    }

    #[test]
    fn patch_fades_in_on_approach_without_pop_or_hole() {
        // Close over a region portal, looking at it, the follow centre on
        // the portal: the patch refines around it while the ball keeps the
        // rest, tiling the sphere exactly once.
        let surface = frame(10);
        assert!(
            patch_active(&surface, 1.0),
            "visible surface draws the patch"
        );
        assert!(
            !patch_active(&surface, 0.0),
            "hidden surface draws no patch"
        );
        let portal = surface.portal_positions()[0];
        let centre = surface.patch_centre(portal);
        let faces: HashSet<TriAddr> = patch_faces(centre).into_iter().collect();
        assert!(!faces.is_empty(), "patch covers base faces");
        let camera = [portal[0] * 1.3, portal[1] * 1.3, portal[2] * 1.3];
        let view = looking(camera, portal);
        let (ball, patch) = patch_partition(&surface, centre, &view, None);
        assert!(!patch.is_empty(), "near field draws inside the patch");
        assert!(
            patch.iter().any(|t| t.addr.depth() > 1),
            "near field refines inside the patch"
        );
        assert!(
            ball.iter().all(|t| t.addr.depth() == 1),
            "ball stays coarse outside the patch"
        );
        let full: HashSet<TriAddr> = universe_core::icosphere::all_at_depth(1)
            .into_iter()
            .collect();
        assert_eq!(full.len(), 80, "ball is the 80-triangle planet");
        let ball_set: HashSet<TriAddr> = ball.iter().map(|triangle| triangle.addr).collect();
        assert_eq!(ball_set.len(), ball.len(), "each ball face drawn once");
        let mut covered = ball_set.clone();
        for triangle in &patch {
            let ancestor = triangle
                .addr
                .ancestor_at(1)
                .expect("patch refines base faces");
            assert!(faces.contains(&ancestor), "refined inside the patch");
            assert!(!ball_set.contains(&ancestor), "no double draw");
            covered.insert(ancestor);
        }
        assert_eq!(covered, full, "ball plus patch tile the sphere once");
        assert!(
            ball.len() + patch.len() <= TRIANGLE_CAP,
            "within the budget"
        );
        // A second follow centre tiles exactly once too.
        let other = surface.patch_centre(surface.surface_point(root_first_child_centre()));
        let (ball, patch) = patch_partition(&surface, other, &looking(camera, portal), None);
        let mut covered: HashSet<TriAddr> = ball.iter().map(|triangle| triangle.addr).collect();
        for triangle in &patch {
            covered.insert(triangle.addr.ancestor_at(1).expect("base ancestor"));
        }
        assert_eq!(covered, full, "roaming centre tiles the sphere once");
    }

    #[test]
    fn patch_fades_out_when_leaving() {
        let surface = frame(10);
        assert!(!patch_active(&surface, 0.0), "gone surface draws no patch");
        assert!(
            patch_active(&surface, 0.5),
            "fading surface keeps the patch"
        );
        assert!(patch_active(&surface, 1.0), "open surface keeps the patch");
    }

    #[test]
    fn far_away_planet_still_a_full_ball() {
        // Far away the fade is zero, so the gate stays off and the draw
        // path is exactly today's 80-triangle planet.
        let surface = frame(10);
        assert!(!patch_active(&surface, 0.0), "far ball draws no patch");
        let view = looking([0.0, 0.0, 50.0], [0.0; 3]);
        let triangles = surface_triangles(&surface, &view, None);
        assert_eq!(triangles.len(), 80, "far ball is whole");
        assert!(triangles.iter().all(|t| t.addr.depth() == 1));
    }

    #[test]
    fn deeper_levels_unchanged_in_step_one() {
        // Step 1 touches the L10 surface only.
        for n in 11..=13u8 {
            let surface = frame(n);
            for fade in [0.0, 0.5, 1.0] {
                assert!(!patch_active(&surface, fade), "L{n} draws no patch");
            }
        }
    }

    #[test]
    fn a_full_budget_keeps_the_target_then_the_view_centre() {
        let a = Queued {
            class: Class::Target,
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
                (Class::Target, 5.0),
                (Class::Centre, 0.0),
                (Class::Centre, 0.3),
                (Class::Rest, 0.0)
            ],
            "target, then nearest the centre, then the rest"
        );
    }

    #[test]
    fn entry_keeps_the_same_triangles_on_both_sides() {
        // C3 of #458: the split is angular and the view maps across the
        // entry, so the parent and child frames (one similarity apart) draw
        // the same triangles at the entry rest.
        for n in 10..=12u8 {
            let parent = frame(n);
            let child = frame(n + 1);
            let ground = child.ground_height(0.1, -0.05);
            let camera_child = [0.1, ground + 3.0, -0.05];
            let look_child = [0.0, -1.0, 0.2];
            let camera_parent = parent.to_cell(child.to_planet(camera_child));
            let look_parent = {
                let tip = parent.to_cell(child.to_planet([
                    camera_child[0] + look_child[0],
                    camera_child[1] + look_child[1],
                    camera_child[2] + look_child[2],
                ]));
                sub(tip, camera_parent)
            };
            let fov = std::f64::consts::FRAC_PI_4;
            let before: HashSet<TriAddr> = surface_triangles(
                &parent,
                &SplitView::camera(camera_parent, look_parent, fov, 16.0 / 9.0),
                None,
            )
            .iter()
            .map(|t| t.addr)
            .collect();
            let after: HashSet<TriAddr> = surface_triangles(
                &child,
                &SplitView::camera(camera_child, look_child, fov, 16.0 / 9.0),
                None,
            )
            .iter()
            .map(|t| t.addr)
            .collect();
            assert!(before.len() <= TRIANGLE_CAP && after.len() <= TRIANGLE_CAP);
            let differ = before.symmetric_difference(&after).count();
            assert!(
                differ * 100 <= before.len(),
                "L{n} -> L{}: {differ} of {} triangles change at entry",
                n + 1,
                before.len()
            );
        }
    }
}
