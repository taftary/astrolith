//! The nodes tree and the values each node shows (#640, AC6).
//!
//! The left panel lists Planet, Moons (one line per moon), Camera, Light,
//! Grid and Axes; the right panel shows the selected node's rows. Rows are
//! plain label and value text built from the scene, in real metres.

use crate::library::ShaderLibrary;
use crate::orbit::CameraMode;
use crate::shaders::{ShaderId, ShaderPanel};
use crate::units::{group, group_count, length_text, metres_per_unit, radius_text};
use universe_core::world::biome::RecipeSummary;

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
    /// One created library shader, under the Shaders line (#667).
    LibraryShader(u32),
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
            Node::LibraryShader(id) => format!("Shader {id}"),
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
    /// Generator recipe of the planet on show, if framed with an orbit.
    pub recipe: Option<RecipeSummary>,
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
    /// The Shaders panel (#665): Planet squares' rows (created shaders
    /// read their own entry in `library`).
    pub shaders: ShaderPanel,
    /// The shader library: every entry's name, panel and code (#667).
    pub library: ShaderLibrary,
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
        Node::Planet => {
            let mut rows = vec![
                row("Name", cx.planet_name.clone()),
                row("Radius", radius_text(cx.radius_earth)),
                row("LOD depth", cx.surface.max_depth.to_string()),
                row("Vertices", group_count(cx.surface.vertices)),
                row("Edges", group_count(cx.surface.edges)),
                row("Faces", group_count(cx.surface.faces)),
                row("Wireframe", on_off(cx.toggles.wireframe)),
                row("Normals", on_off(cx.toggles.normals)),
                row("LOD depth view", on_off(cx.toggles.lod_colors)),
            ];
            if let Some(recipe) = &cx.recipe {
                rows.extend(recipe_rows(recipe));
            }
            rows
        }
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
        Node::LibraryShader(id) => cx
            .library
            .entry(id)
            .map_or_else(Vec::new, |entry| entry.panel.values()),
    }
}

/// Generator recipe rows for the Planet node (T13): type, mass,
/// gravity, age, surface temperature, air, plate regime and moon kinds.
/// Vegetation tints stay speculative (see the guides page).
fn recipe_rows(recipe: &RecipeSummary) -> Vec<(String, String)> {
    use universe_core::world::{air::AirMix, moons::MoonKind, plates::Regime};
    let air = if !recipe.air_present {
        "none".to_string()
    } else {
        let mix = match recipe.air_mix {
            AirMix::Vacuum => "vacuum",
            AirMix::CarbonDioxide => "carbon dioxide",
            AirMix::NitrogenCarbonDioxide => "nitrogen/carbon dioxide",
            AirMix::NitrogenMethane => "nitrogen/methane",
            AirMix::HydrogenHelium => "hydrogen/helium",
            AirMix::RockVapour => "rock vapour",
            AirMix::WaterMix => "water mix",
        };
        match recipe.pressure_bar {
            Some(bar) => format!("{mix}, {bar:.2} bar"),
            None => format!("{mix} envelope"),
        }
    };
    let regime = match recipe.regime {
        Regime::Mobile => "Mobile",
        Regime::Episodic => "Episodic",
        Regime::Stagnant => "Stagnant",
        Regime::HeatPipe => "Heat pipe",
        Regime::Ice => "Ice",
    }
    .to_string();
    let mut moons = String::new();
    let mut total = 0usize;
    for kind in MoonKind::ALL {
        let count = recipe.moon_kinds.iter().filter(|k| **k == kind).count();
        total += count;
        if count > 0 {
            if !moons.is_empty() {
                moons.push_str(", ");
            }
            moons.push_str(&format!("{} {count}", kind.label()));
        }
    }
    vec![
        row("Type", recipe.kind.label().to_string()),
        row("Mass", format!("{:.2} Earth", recipe.mass_earth)),
        row("Gravity", format!("{:.1} m/s²", recipe.gravity)),
        row("Age", format!("{:.2} Gyr", recipe.age_gyr)),
        row("Temperature", format!("{:.0} °C", recipe.temp_c)),
        row("Air", air),
        row("Plates", regime),
        row(
            "Moons",
            if total == 0 {
                "none".to_string()
            } else {
                format!("{total} ({moons})")
            },
        ),
    ]
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

/// One Nodes list line per library entry, in order, after the static
/// shader lines: Planet squares keeps its game-shader line, created
/// shaders follow it (#667).
#[must_use]
pub fn library_nodes(library: &ShaderLibrary) -> Vec<Node> {
    library
        .entries()
        .iter()
        .filter(|entry| !ShaderLibrary::is_planet_squares(entry.id))
        .map(|entry| Node::LibraryShader(entry.id))
        .collect()
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
            recipe: None,
            camera_mode: CameraMode::Orbit,
            camera: [0.0, 0.0, 1.0],
            fov: crate::orbit::FOV,
            light: ([0.0, -1.0, 0.0], 10_000.0),
            grid_m: 1.0e6,
            axes_m: 1.0e7,
            shaders: ShaderPanel::default(),
            library: ShaderLibrary::default(),
        }
    }

    fn value(rows: &[(String, String)], label: &str) -> String {
        rows.iter()
            .find(|(l, _)| l == label)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    }

    #[test]
    fn the_lab_shows_generated_values_and_gallery() {
        // C15 / AC15 (T13): the Planet node carries the generator recipe,
        // and the gallery holds one seed per planet kind and per moon
        // kind with a navigating lab shot each.
        use crate::scene::LabScene;
        use universe_core::nav::DEMO_SEED;
        use universe_core::world::gallery::{GALLERY_ROOT, MOON_GALLERY, PLANET_GALLERY};
        // Recipe rows read the live summary, self-consistently.
        let scene = LabScene::default_home();
        assert_eq!(scene.seed(), DEMO_SEED);
        assert_eq!(DEMO_SEED, GALLERY_ROOT, "gallery rooted elsewhere");
        let universe = scene.universe();
        let summary = universe
            .frame
            .as_ref()
            .and_then(universe_core::world::biome::recipe_summary)
            .expect("home recipe");
        let mut cx = context();
        cx.recipe = Some(summary.clone());
        let planet = node_rows(Node::Planet, &cx);
        for label in [
            "Type",
            "Mass",
            "Gravity",
            "Age",
            "Temperature",
            "Air",
            "Plates",
            "Moons",
        ] {
            assert!(!value(&planet, label).is_empty(), "{label} row missing");
        }
        assert_eq!(value(&planet, "Type"), summary.kind.label());
        assert!(value(&planet, "Mass").ends_with("Earth"));
        assert!(value(&planet, "Gravity").ends_with("m/s²"));
        assert!(value(&planet, "Age").ends_with("Gyr"));
        assert!(value(&planet, "Temperature").ends_with("°C"));
        if summary.moon_kinds.is_empty() {
            assert_eq!(value(&planet, "Moons"), "none");
        } else {
            let total = summary.moon_kinds.len().to_string();
            assert!(value(&planet, "Moons").starts_with(total.as_str()));
        }
        // Gallery coverage: every star-bound kind and every moon kind.
        assert_eq!(PLANET_GALLERY.len(), 13, "rogue starless");
        assert_eq!(MOON_GALLERY.len(), 7);
        for entry in PLANET_GALLERY {
            assert_ne!(entry.kind, universe_core::world::classify::WorldKind::Rogue);
        }
        // Every gallery shot navigates there with no error.
        let mut scene = LabScene::default_home();
        for entry in PLANET_GALLERY {
            scene.show_gallery(&entry.star, entry.planet);
            assert_eq!(scene.error(), None, "{:?}", entry.kind);
            assert!(scene.on_planet());
            assert_eq!(scene.selected(), entry.planet);
            let shot = crate::shots::SHOTS
                .iter()
                .find(|shot| shot.view == crate::shots::ShotView::GalleryPlanet(entry.kind));
            assert!(shot.is_some(), "{:?} shot", entry.kind);
        }
        for entry in MOON_GALLERY {
            scene.show_gallery(&entry.star, entry.planet);
            assert_eq!(scene.error(), None, "{:?}", entry.kind);
            assert!(scene.on_planet());
            scene
                .show_selected(Some(Node::Moon(entry.moon_index)))
                .expect("gallery moon");
            assert_eq!(scene.moon_on_show(), Some(entry.moon_index));
            let shot = crate::shots::SHOTS
                .iter()
                .find(|shot| shot.view == crate::shots::ShotView::GalleryMoon(entry.kind));
            assert!(shot.is_some(), "{:?} shot", entry.kind);
            scene.parent().expect("back to the planet");
        }
    }

    #[test]
    fn library_entries_appear_under_shaders() {
        // C1 / AC1 of #667: one Nodes line per created shader after the
        // game shader lines, in creation order; Planet squares keeps its
        // own line and is never a library line.
        use crate::library::ShaderLibrary;
        let mut library = ShaderLibrary::default();
        assert_eq!(library_nodes(&library), Vec::new());
        let first = library.new_shader();
        let second = library.new_shader();
        assert_eq!(
            library_nodes(&library),
            vec![Node::LibraryShader(first), Node::LibraryShader(second)]
        );
        let mut cx = context();
        cx.library = library;
        for node in [Node::LibraryShader(first), Node::LibraryShader(second)] {
            let rows = node_rows(node, &cx);
            assert_eq!(value(&rows, "Shown"), "off");
            assert_eq!(value(&rows, "Square size"), "100 m");
        }
        assert_eq!(
            node_rows(Node::LibraryShader(999), &cx),
            Vec::new(),
            "a deleted shader shows nothing"
        );
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
