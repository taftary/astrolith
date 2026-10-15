//! Selection, 3D picking and the selection outline (#654 D2).
//!
//! One selection is shared by the Nodes list and the 3D view: a click on a
//! list line or on an object selects the same [`Node`], a click on empty
//! space clears it. The selected object wears a bright orange outline.
//! Everything here is pure: a pick is a ray against the objects' drawn
//! spheres, with a few pixels of screen tolerance so even a tiny marker
//! far away can be clicked.

use crate::nodes::Node;
use crate::scene::{LabScene, moons};
use universe_core::nav::Universe;

/// Outline colour: the bright orange of the squares look's edges (#626).
pub const OUTLINE_RGB: [f32; 3] = [1.0, 0.55, 0.1];

/// Moon body radius in cell units, as the game draws moons (#640).
pub const MOON_RADIUS_CELL: f64 = 0.008;

/// Screen tolerance of a click, in pixels.
pub const PICK_PX: f64 = 6.0;

/// The outline circle's radius over the object's radius.
const OUTLINE_SHARE: f64 = 1.15;

/// The shared selection of the list and the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Selection(Option<Node>);
impl Selection {
    /// A selection of `node`.
    #[must_use]
    pub const fn of(node: Node) -> Selection {
        Selection(Some(node))
    }

    /// The selected node, if any.
    #[must_use]
    pub const fn node(self) -> Option<Node> {
        self.0
    }

    /// Whether `node`'s list line is highlighted.
    #[must_use]
    pub fn is(self, node: Node) -> bool {
        self.0 == Some(node)
    }

    /// A click on a list line selects its node.
    pub fn click_list(&mut self, node: Node) {
        self.0 = Some(node);
    }

    /// A click in the 3D view: the picked object, or none (empty space
    /// clears the selection).
    pub fn click_view(&mut self, picked: Option<Node>) {
        self.0 = picked;
    }
}

/// Screen rectangles covered by the lab's editor panels, in egui points
/// (logical pixels, the same units as the window cursor position).
///
/// The 3D pick and the camera controls must ignore a click that lands on
/// a panel: with egui 0.36 `Panel`s are child `Ui`s, not `Area`s, so
/// `is_pointer_over_egui()` stays false over them (#703).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PanelRects {
    /// Covered rectangles as `[min_x, min_y, max_x, max_y]`.
    pub rects: Vec<[f32; 4]>,
}

impl PanelRects {
    /// Whether `point` (`[x, y]`) lies on any covered rectangle.
    #[must_use]
    pub fn covers(&self, point: [f32; 2]) -> bool {
        let [x, y] = point;
        self.rects.iter().any(|[min_x, min_y, max_x, max_y]| {
            *min_x <= x && x <= *max_x && *min_y <= y && y <= *max_y
        })
    }
}

/// One object a click can hit: its node, centre (cell units) and drawn
/// radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pickable {
    /// The object's node.
    pub node: Node,
    /// Centre, open-cell units.
    pub centre: [f64; 3],
    /// Drawn radius, open-cell units.
    pub radius: f64,
}

/// The objects of `scene` that a click can hit in `universe` (the scene's
/// universe): the planet and its moons on a planet, the level's markers
/// above it.
#[must_use]
pub fn pickables(scene: &LabScene, universe: &Universe) -> Vec<Pickable> {
    if let Some(index) = scene.moon_on_show() {
        return vec![Pickable {
            node: Node::Moon(index),
            centre: [0.0; 3],
            radius: universe_core::terrain::PLANET_RADIUS_CELL,
        }];
    }
    if scene.on_planet() {
        let mut out = vec![Pickable {
            node: Node::Planet,
            centre: [0.0; 3],
            radius: universe_core::terrain::PLANET_RADIUS_CELL,
        }];
        out.extend(
            moons(universe)
                .iter()
                .enumerate()
                .map(|(i, point)| Pickable {
                    node: Node::Moon(i),
                    centre: point.position,
                    radius: MOON_RADIUS_CELL,
                }),
        );
        return out;
    }
    let radius = universe.marker_radius();
    universe
        .open
        .points
        .iter()
        .enumerate()
        .filter_map(|(index, point)| {
            Some(Pickable {
                node: Node::Marker(u32::try_from(index).ok()?),
                centre: point.position,
                radius,
            })
        })
        .collect()
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The object a click ray hits: from `origin` along unit `direction`,
/// each object counts as hit when the ray passes within its radius or
/// within `tolerance` radians of its centre; the nearest hit along the ray
/// wins. `None` for empty space or a bad ray.
#[must_use]
pub fn pick(
    objects: &[Pickable],
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance: f64,
) -> Option<Node> {
    let length = dot(direction, direction).sqrt();
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    let unit = direction.map(|d| d / length);
    objects
        .iter()
        .filter_map(|object| {
            let to = sub(object.centre, origin);
            let along = dot(to, unit);
            if along <= 0.0 {
                return None;
            }
            let miss = (dot(to, to) - along * along).max(0.0).sqrt();
            (miss <= object.radius || miss <= along * tolerance.tan())
                .then_some((along, object.node))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, node)| node)
}

/// Click tolerance in radians for a view `fov` radians tall over
/// `height_px` pixels.
#[must_use]
pub fn tolerance(fov: f64, height_px: f64) -> f64 {
    if height_px > 0.0 {
        fov / height_px * PICK_PX
    } else {
        0.0
    }
}

/// The selection outline: centre (cell units) and radius of an orange
/// circle just outside the selected object, never smaller than the click
/// tolerance seen from `camera`. `None` when nothing (or a tool node) is
/// selected.
#[must_use]
pub fn outline(
    objects: &[Pickable],
    selection: Selection,
    camera: [f64; 3],
    tolerance: f64,
) -> Option<([f64; 3], f64)> {
    let node = selection.node()?;
    let object = objects.iter().find(|object| object.node == node)?;
    let to = sub(object.centre, camera);
    let distance = dot(to, to).sqrt();
    // At least twice the click tolerance, so a far, tiny object's outline
    // still reads on screen.
    let least = distance * (2.0 * tolerance).tan();
    Some((object.centre, (object.radius * OUTLINE_SHARE).max(least)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_click_does_not_reach_the_view() {
        // #703: clicking the shader Colour boxes (right panel) cleared the
        // selection because the 3D pick fired too. Covered points belong
        // to the panel; only the bare view reaches the pick.
        let panels = PanelRects {
            rects: vec![[1300.0, 0.0, 1600.0, 900.0]],
        };
        assert!(
            panels.covers([1400.0, 450.0]),
            "a Colour box point is on the panel"
        );
        assert!(
            !panels.covers([800.0, 450.0]),
            "a central view point reaches the view"
        );
        assert!(
            !PanelRects::default().covers([1400.0, 450.0]),
            "nothing recorded yet reaches the view"
        );
    }

    #[test]
    fn selection_is_shared_by_the_list_and_the_view() {
        // C3 / AC2 of #654: a list click and a 3D click select the same
        // node; the selected object has the orange outline; empty space
        // clears.
        let scene = LabScene::default_home();
        let universe = scene.universe();
        let objects = pickables(&scene, &universe);
        let moon = objects
            .iter()
            .find(|object| object.node == Node::Moon(0))
            .copied()
            .expect("Earth's Moon");
        let camera = [0.0, 0.3, 3.0];
        let tol = tolerance(crate::orbit::FOV, 900.0);
        // A 3D click straight at the Moon picks it.
        let mut selection = Selection::default();
        selection.click_view(pick(&objects, camera, sub(moon.centre, camera), tol));
        assert_eq!(selection.node(), Some(Node::Moon(0)));
        assert!(selection.is(Node::Moon(0)), "its list line is highlighted");
        assert!(!selection.is(Node::Planet));
        // The outline sits on the Moon, just outside it, in orange.
        let (centre, radius) = outline(&objects, selection, camera, tol).expect("an outline");
        assert_eq!(centre, moon.centre);
        assert!(radius > moon.radius);
        assert_eq!(OUTLINE_RGB, [1.0, 0.55, 0.1]);
        // A list click selects the planet; a 3D click at the planet agrees.
        selection.click_list(Node::Planet);
        let mut from_view = Selection::default();
        from_view.click_view(pick(&objects, camera, sub([0.0; 3], camera), tol));
        assert_eq!(from_view, selection, "list and view select the same node");
        // Empty space (straight away from everything) clears.
        selection.click_view(pick(&objects, camera, [0.0, 1.0, 0.0], tol));
        assert_eq!(selection.node(), None);
        assert_eq!(outline(&objects, selection, camera, tol), None);
        // Above the planet the level's markers are picked the same way.
        let mut up = scene.clone();
        up.parent().expect("L9");
        let universe = up.universe();
        let markers = pickables(&up, &universe);
        let first = markers.first().copied().expect("a marker");
        let eye = [0.0, 0.0, 2.0];
        let hit = pick(&markers, eye, sub(first.centre, eye), tol);
        assert!(matches!(hit, Some(Node::Marker(_))), "{hit:?}");
        assert_eq!(pick(&[], eye, [0.0, 0.0, -1.0], tol), None);
        assert_eq!(pick(&markers, eye, [0.0; 3], tol), None, "bad ray");
    }
}
