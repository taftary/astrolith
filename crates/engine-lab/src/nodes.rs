//! The nodes tree and the values each node shows (#640, AC6).
//!
//! The left panel lists Planet, Moons (one line per moon), Camera, Light,
//! Grid and Axes; the right panel shows the selected node's rows. Rows are
//! plain label and value text built from the scene, in real metres.

use crate::orbit::CameraMode;
use crate::shaders::{ShaderId, ShaderPanel};
use crate::units::{group, group_count, length_text, metres_per_unit, radius_text};

/// One node of the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Node {
    /// The planet (selected by default).
    #[default]
    Planet,
    /// Moon `i` (0-based) of the planet.
    Moon(usize),
    /// Marker `i` of the cell on show above the planet (L1-L9, #654).
    Marker(u32),
    /// The viewport camera.
    Camera,
    /// The light.
    Light,
    /// The grid.
    Grid,
    /// The X/Y/Z axes gizmo.
    Axes,
    /// The game's shaders (#665).
    Shaders,
    /// One game shader, under the Shaders line.
    Shader(ShaderId),
}

impl Node {
    /// Tree label.
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Node::Planet => "Planet".to_string(),
            Node::Moon(index) => format!("Moon {}", index + 1),
            Node::Marker(index) => format!("Object {}", index + 1),
            Node::Camera => "Camera".to_string(),
            Node::Light => "Light".to_string(),
            Node::Grid => "Grid".to_string(),
            Node::Axes => "Axes".to_string(),
            Node::Shaders => "Shaders".to_string(),
            Node::Shader(id) => id.label().to_string(),
        }
    }
}

/// Planet surface figures from the last surface build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SurfaceFigures {
    /// Drawn triangles (faces).
    pub faces: usize,
    /// Distinct triangle edges.
    pub edges: usize,
    /// Distinct triangle corners (vertices).
    pub vertices: usize,
    /// Deepest icosphere split drawn (LOD depth).
    pub max_depth: u8,
}

/// The debug views and overlay switches (View menu, Planet node).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewToggles {
    /// Triangles as lines only.
    pub wireframe: bool,
    /// Face normals as short lines.
    pub normals: bool,
    /// Triangles coloured by LOD depth.
    pub lod_colors: bool,
    /// The stats overlay.
    pub stats: bool,
    /// The grid.
    pub grid: bool,
    /// The axes gizmo.
    pub axes: bool,
}

impl Default for ViewToggles {
    fn default() -> ViewToggles {
        ViewToggles {
            wireframe: false,
            normals: false,
            lod_colors: false,
            stats: true,
            grid: true,
            axes: true,
        }
    }
}

/// Everything the node rows read.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeContext {
    /// Planet name.
    pub planet_name: String,
    /// Planet radius in Earth radii.
    pub radius_earth: f64,
    /// Surface figures.
    pub surface: SurfaceFigures,
    /// View switches.
    pub toggles: ViewToggles,
    /// Moons: radius km, orbit km, period days.
    pub moons: Vec<(f64, f64, f64)>,
    /// Camera mode.
    pub camera_mode: CameraMode,
    /// Camera position, cell units.
    pub camera: [f64; 3],
    /// Field of view, radians.
    pub fov: f64,
    /// Light direction (unit) and strength (lux).
    pub light: ([f64; 3], f64),
    /// Grid spacing, metres.
    pub grid_m: f64,
    /// Axes length, metres.
    pub axes_m: f64,
    /// The Shaders panel (#665).
    pub shaders: ShaderPanel,
}

fn on_off(value: bool) -> String {
    if value { "on" } else { "off" }.to_string()
}

fn row(label: &str, value: String) -> (String, String) {
    (label.to_string(), value)
}

/// The rows the tools panel shows for `node`.
#[must_use]
pub fn node_rows(node: Node, cx: &NodeContext) -> Vec<(String, String)> {
    match node {
        Node::Marker(_) => vec![row("Open", "F, Child or Page Down".to_string())],
        Node::Planet => vec![
            row("Name", cx.planet_name.clone()),
            row("Radius", radius_text(cx.radius_earth)),
            row("LOD depth", cx.surface.max_depth.to_string()),
            row("Vertices", group_count(cx.surface.vertices)),
            row("Edges", group_count(cx.surface.edges)),
            row("Faces", group_count(cx.surface.faces)),
            row("Wireframe", on_off(cx.toggles.wireframe)),
            row("Normals", on_off(cx.toggles.normals)),
            row("LOD depth view", on_off(cx.toggles.lod_colors)),
        ],
        Node::Moon(index) => match cx.moons.get(index) {
            Some(&(radius_km, orbit_km, period_days)) => vec![
                row("Radius", length_text(radius_km * 1_000.0)),
                row("Orbit", length_text(orbit_km * 1_000.0)),
                row("Period", format!("{period_days:.1} days")),
            ],
            None => vec![row("Moon", "none".to_string())],
        },
        Node::Camera => {
            let [x, y, z] = cx
                .camera
                .map(|unit| unit * metres_per_unit(cx.radius_earth));
            let distance = (x * x + y * y + z * z).sqrt();
            vec![
                row(
                    "Mode",
                    match cx.camera_mode {
                        CameraMode::Orbit => "Orbit",
                        CameraMode::Free => "Free-fly",
                    }
                    .to_string(),
                ),
                row(
                    "Position",
                    format!("{}, {}, {}", length_text(x), length_text(y), length_text(z)),
                ),
                row("Distance", length_text(distance)),
                row("Field of view", format!("{:.0} deg", cx.fov.to_degrees())),
            ]
        }
        Node::Light => {
            let [x, y, z] = cx.light.0;
            vec![
                row("Direction", format!("{x:.2}, {y:.2}, {z:.2}")),
                row("Strength", format!("{} lux", group(cx.light.1))),
            ]
        }
        Node::Grid => vec![
            row("Shown", on_off(cx.toggles.grid)),
            row("Spacing", length_text(cx.grid_m)),
        ],
        Node::Axes => vec![
            row("Shown", on_off(cx.toggles.axes)),
            row("Length", length_text(cx.axes_m)),
        ],
        Node::Shaders => cx.shaders.summary(),
        Node::Shader(id) => cx.shaders.rows(id),
    }
}

/// The tree's nodes for a planet with `moons` moons, in panel order.
#[must_use]
pub fn tree(moons: usize) -> Vec<Node> {
    let mut nodes = vec![Node::Planet];
    nodes.extend((0..moons).map(Node::Moon));
    nodes.extend([Node::Camera, Node::Light, Node::Grid, Node::Axes]);
    nodes.extend(shader_nodes());
    nodes
}

/// The Shaders line and one line per game shader under it (#665 Q2).
#[must_use]
pub fn shader_nodes() -> Vec<Node> {
    let mut nodes = vec![Node::Shaders];
    nodes.extend(ShaderId::ALL.map(Node::Shader));
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> NodeContext {
        NodeContext {
            planet_name: "Earth".to_string(),
            radius_earth: 1.0,
            surface: SurfaceFigures {
                faces: 80,
                edges: 120,
                vertices: 42,
                max_depth: 1,
            },
            toggles: ViewToggles::default(),
            moons: vec![(1_737.4, 384_400.0, 27.3)],
            camera_mode: CameraMode::Orbit,
            camera: [0.0, 0.0, 1.0],
            fov: crate::orbit::FOV,
            light: ([0.0, -1.0, 0.0], 10_000.0),
            grid_m: 1.0e6,
            axes_m: 1.0e7,
            shaders: ShaderPanel::default(),
        }
    }

    fn value(rows: &[(String, String)], label: &str) -> String {
        rows.iter()
            .find(|(l, _)| l == label)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    }

    #[test]
    fn every_node_shows_its_values() {
        // C7 / AC6.
        let cx = context();
        let nodes = tree(1);
        assert_eq!(
            nodes,
            vec![
                Node::Planet,
                Node::Moon(0),
                Node::Camera,
                Node::Light,
                Node::Grid,
                Node::Axes,
                Node::Shaders,
                Node::Shader(ShaderId::PlanetSquares),
            ]
        );
        let planet = node_rows(Node::Planet, &cx);
        assert_eq!(value(&planet, "Radius"), "6,371 km (1.00 Earth)");
        assert_eq!(value(&planet, "Faces"), "80");
        assert_eq!(value(&planet, "Edges"), "120");
        assert_eq!(value(&planet, "Vertices"), "42");
        assert_eq!(value(&planet, "LOD depth"), "1");
        for toggle in ["Wireframe", "Normals", "LOD depth view"] {
            assert_eq!(value(&planet, toggle), "off");
        }
        let moon = node_rows(Node::Moon(0), &cx);
        assert_eq!(value(&moon, "Radius"), "1,737 km");
        assert_eq!(value(&moon, "Orbit"), "384,400 km");
        assert_eq!(value(&moon, "Period"), "27.3 days");
        let camera = node_rows(Node::Camera, &cx);
        assert_eq!(value(&camera, "Mode"), "Orbit");
        assert_eq!(value(&camera, "Field of view"), "45 deg");
        assert!(value(&camera, "Distance").ends_with("km"));
        assert_eq!(
            value(&node_rows(Node::Light, &cx), "Strength"),
            "10,000 lux"
        );
        assert_eq!(value(&node_rows(Node::Grid, &cx), "Spacing"), "1,000 km");
        assert_eq!(value(&node_rows(Node::Axes, &cx), "Shown"), "on");
        assert_eq!(Node::Moon(0).label(), "Moon 1");
    }
}
