//! Black-and-white squares on planets (#626, Spec v2; ADR 0027).
//!
//! The planet surface and the moons draw with [`SurfaceMaterial`]: the
//! standard unlit material extended by one fragment shader
//! (`planet_squares.wgsl`, embedded in the binary). With the Planet look
//! set to Normal the shader is the standard unlit path, so the planets look
//! as before (AC1). With Squares it replaces the base colour by the 100 m
//! 3D chessboard of [`universe_core::squares`] in planet space, faded to
//! gray when small, without the place tones (AC2-AC4).
//!
//! The squares are glued to the planet: [`squares_view`] gives the shader
//! the render-to-planet rotation, metres per render unit and the render
//! origin's planet position reduced by the pattern's period, all from the
//! `f64` surface frame. Render positions stay camera-relative (ADR 0026),
//! so the GPU only sees small numbers at every level.
//!
//! The values the shader draws with (square size, the two colours, the
//! fade, and a debug view) come from [`SquaresTuning`]. The game never
//! changes it, so it always draws today's black-and-white 100 m squares; the
//! engine lab tunes it live (#665).

use crate::Universe;
use crate::planet_approach::{SurfaceHost, surface_host};
use crate::settings_ui::SettingsStore;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use universe_core::squares::{
    COLOUR_A, COLOUR_B, GRAY_PX, SHARP_PX, SQUARE_M, origin_in_period_for,
};

/// The surface and moon material: standard unlit plus the squares shader.
pub(crate) type SurfaceMaterial = ExtendedMaterial<StandardMaterial, PlanetSquares>;

/// Embedded path of the squares shader.
const SHADER: &str = "embedded://universe_render/planet_squares.wgsl";

/// Shader inputs (see `planet_squares.wgsl`).
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType, Reflect)]
pub(crate) struct SquaresUniform {
    /// Render-to-planet rotation, first row (w unused).
    pub(crate) row_x: Vec4,
    /// Second row.
    pub(crate) row_y: Vec4,
    /// Third row.
    pub(crate) row_z: Vec4,
    /// Render origin in planet metres reduced by the period (xyz), and
    /// metres per render unit (w).
    pub(crate) origin: Vec4,
    /// Squares on (x), square metres (y), sharp and gray sizes in pixels
    /// (z, w).
    pub(crate) look: Vec4,
    /// Colour of the parity-0 squares (rgb; w unused).
    pub(crate) colour_a: Vec4,
    /// Colour of the parity-1 squares (rgb; w unused).
    pub(crate) colour_b: Vec4,
    /// Debug view code (x, see [`DebugView::code`]) and the distance view's
    /// near and far metres (y, z).
    pub(crate) debug: Vec4,
}

/// What the surface shows instead of its colour, for the engine lab
/// (#665). The game always uses [`DebugView::Off`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DebugView {
    /// The normal picture.
    #[default]
    Off,
    /// Red, green and blue bands repeating every square along the planet's
    /// x, y and z.
    Position,
    /// The planet-frame direction the surface faces, as red, green, blue.
    Normal,
    /// Bright near the camera to dark far away, over the distance range.
    Distance,
}

impl DebugView {
    /// Every view, in menu order.
    pub const ALL: [Self; 4] = [Self::Off, Self::Position, Self::Normal, Self::Distance];

    /// Its name in menus.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Position => "Position",
            Self::Normal => "Normal",
            Self::Distance => "Distance",
        }
    }

    /// The code the shader reads.
    #[must_use]
    pub const fn code(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Position => 1.0,
            Self::Normal => 2.0,
            Self::Distance => 3.0,
        }
    }
}

/// The values the squares shader draws with. The default is the game's own
/// look from [`universe_core::squares`]; only the engine lab changes it.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct SquaresTuning {
    /// Edge of one square, in metres.
    pub square_m: f64,
    /// Colour of the parity-0 squares, linear RGB 0..1.
    pub colour_a: [f64; 3],
    /// Colour of the parity-1 squares, linear RGB 0..1.
    pub colour_b: [f64; 3],
    /// A square at least this many pixels wide is drawn pure.
    pub sharp_px: f64,
    /// A square at most this many pixels wide is the even mean tone.
    pub gray_px: f64,
    /// The debug view on show.
    pub debug: DebugView,
    /// The distance view's near and far metres.
    pub distance_m: [f64; 2],
}

impl Default for SquaresTuning {
    fn default() -> Self {
        Self {
            square_m: SQUARE_M,
            colour_a: COLOUR_A,
            colour_b: COLOUR_B,
            sharp_px: SHARP_PX,
            gray_px: GRAY_PX,
            debug: DebugView::Off,
            distance_m: [0.0, 10_000.0],
        }
    }
}

/// The squares extension of the standard material.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub(crate) struct PlanetSquares {
    /// Everything the shader reads.
    #[uniform(100)]
    pub(crate) squares: SquaresUniform,
}

impl MaterialExtension for PlanetSquares {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
}

/// A surface material from its standard base, Normal look until the
/// squares view says otherwise.
#[must_use]
pub(crate) fn surface_material(base: StandardMaterial) -> SurfaceMaterial {
    ExtendedMaterial {
        base,
        extension: PlanetSquares::default(),
    }
}

/// Planet-space view of the render space this frame: the shader uniform
/// for `host` with the render origin at `camera` (open-cell units), the
/// squares on when `on`, drawn with `tuning`.
#[must_use]
pub(crate) fn squares_view(
    host: &SurfaceHost,
    camera: [f64; 3],
    on: bool,
    tuning: &SquaresTuning,
) -> SquaresUniform {
    // The planet's own metre (#648): its ground at its real radius.
    let per_unit = host.frame.metres_per_unit();
    let planet = |open: [f64; 3]| host.to_surface(open);
    let here = planet(camera);
    // The scale is measured at the surface cell's centre, where its numbers
    // are smallest, whatever the camera's distance.
    let [cx, cy, cz] = host.offset;
    let metres_per_unit =
        crate::surface_split::gap(planet(host.offset), planet([cx + 1.0, cy, cz])) * per_unit;
    let origin = origin_in_period_for(here.map(|unit| unit * per_unit), 2.0 * tuning.square_m);
    let columns = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: small rotation, period-reduced metres and a scale narrowed for the GPU"
    )]
    let narrow = |x: f64| x as f32;
    let [[ax, ay, az], [bx, by, bz], [cx, cy, cz]] = columns;
    let row = |a: f64, b: f64, c: f64| Vec4::new(narrow(a), narrow(b), narrow(c), 0.0);
    let [ox, oy, oz] = origin;
    let colour = |[r, g, b]: [f64; 3]| Vec4::new(narrow(r), narrow(g), narrow(b), 1.0);
    let [near, far] = tuning.distance_m;
    SquaresUniform {
        row_x: row(ax, bx, cx),
        row_y: row(ay, by, cy),
        row_z: row(az, bz, cz),
        origin: Vec4::new(narrow(ox), narrow(oy), narrow(oz), narrow(metres_per_unit)),
        look: Vec4::new(
            if on { 1.0 } else { 0.0 },
            narrow(tuning.square_m),
            narrow(tuning.sharp_px),
            narrow(tuning.gray_px),
        ),
        colour_a: colour(tuning.colour_a),
        colour_b: colour(tuning.colour_b),
        debug: Vec4::new(tuning.debug.code(), narrow(near), narrow(far), 0.0),
    }
}

/// Planet-space metres of render point `render` under `view` (the
/// shader's mapping, in `f64` for tests).
#[cfg(test)]
#[must_use]
pub(crate) fn planet_metres(view: &SquaresUniform, render: [f64; 3]) -> [f64; 3] {
    let scale = f64::from(view.origin.w);
    let local = render.map(|r| r * scale);
    let dot = |row: Vec4| {
        f64::from(row.x) * local[0] + f64::from(row.y) * local[1] + f64::from(row.z) * local[2]
    };
    [
        dot(view.row_x) + f64::from(view.origin.x),
        dot(view.row_y) + f64::from(view.origin.y),
        dot(view.row_z) + f64::from(view.origin.z),
    ]
}

/// Whether the uniform must follow the camera every frame: the squares or
/// a debug view read the planet position (the Normal look alone does not).
#[must_use]
pub(crate) fn follows_camera(on: bool, tuning: &SquaresTuning) -> bool {
    on || tuning.debug != DebugView::Off
}

/// Writes this frame's squares view into every surface material while a
/// surface is drawn; with Normal and no debug view it writes once, on a
/// change of the look or of the tuning.
pub(crate) fn update_squares(
    universe: Res<Universe>,
    store: Res<SettingsStore>,
    tuning: Option<Res<SquaresTuning>>,
    mut materials: ResMut<Assets<SurfaceMaterial>>,
    mut last: Local<Option<(bool, SquaresTuning)>>,
) {
    let on = store.0.planet_look.squares();
    let tuning = tuning.map_or_else(SquaresTuning::default, |t| *t);
    if !follows_camera(on, &tuning) && *last == Some((on, tuning)) {
        return;
    }
    let camera = universe.path.offset();
    let view = surface_host(&universe).map_or_else(SquaresUniform::default, |host| {
        squares_view(&host, camera, on, &tuning)
    });
    for (_, material) in materials.iter_mut() {
        if material.extension.squares != view {
            material.extension.squares = view;
        }
    }
    *last = Some((on, tuning));
}

/// Registers the surface material and its embedded shader (window app).
pub struct PlanetSquaresPlugin;

impl Plugin for PlanetSquaresPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "planet_squares.wgsl");
        app.init_resource::<SquaresTuning>()
            .add_plugins(MaterialPlugin::<SurfaceMaterial>::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use universe_core::squares::{PERIOD_M, origin_in_period, square_parity};

    /// The demo journey's planet with the camera `metres` above the
    /// ground at the north pole (a shell level below 1,000 km, #632).
    fn on_planet(metres: f64) -> Universe {
        let mut universe = Universe(universe_core::nav::Universe::new(
            universe_core::nav::DEMO_SEED,
        ));
        while universe.level().get() < 10 {
            let marker = universe.autopilot_target().expect("dive continues");
            assert!(universe.open(universe_core::nav::MarkerIndex(marker)));
        }
        let ground = surface_host(&universe)
            .expect("a surface")
            .frame
            .ground_height(0.0, 0.0);
        let lift = metres / universe.path.planet_metres_per_unit().expect("on a planet");
        universe.path.set_offset([0.0, ground + lift, 0.0]);
        universe
    }

    #[test]
    fn the_view_maps_render_space_to_planet_metres() {
        for metres in [5.0e5, 5.0e4, 500.0, 5.0] {
            let universe = on_planet(metres);
            let camera = universe.path.offset();
            let host = surface_host(&universe).expect("a surface");
            let view = squares_view(&host, camera, true, &SquaresTuning::default());
            assert_eq!(view.look.x, 1.0, "squares on");
            let origin = planet_metres(&view, [0.0; 3]);
            assert!(origin.iter().all(|m| (0.0..PERIOD_M + 1e-3).contains(m)));
            // One render unit along x is metres_per_unit metres in planet
            // space, in some direction (a pure rotation).
            let step = planet_metres(&view, [1.0, 0.0, 0.0]);
            let moved = crate::surface_split::gap(origin, step);
            let want = f64::from(view.origin.w);
            assert!(
                (moved - want).abs() <= want * 1e-4,
                "{metres} m up: {moved} m for one unit, want {want}"
            );
            // The rotation keeps lengths.
            let diagonal = planet_metres(&view, [1.0, 1.0, 1.0]);
            let length = crate::surface_split::gap(origin, diagonal);
            assert!((length - want * 3f64.sqrt()).abs() <= want * 1e-3);
        }
    }

    #[test]
    fn the_squares_stay_glued_to_the_planet_when_the_camera_moves() {
        // AC3: the same ground point keeps its square from two camera spots.
        let universe = on_planet(5.0e4);
        let host = surface_host(&universe).expect("a surface");
        let camera = universe.path.offset();
        let ground = [0.01, host.frame.ground_height(0.01, -0.02), -0.02];
        let moved = [camera[0] + 0.003, camera[1] - 0.002, camera[2] + 0.004];
        for spot in [camera, moved] {
            let view = squares_view(&host, spot, true, &SquaresTuning::default());
            let render = [
                ground[0] - spot[0],
                ground[1] - spot[1],
                ground[2] - spot[2],
            ];
            let metres = planet_metres(&view, render);
            let truth = host.to_surface(ground);
            let per_unit = host.frame.metres_per_unit();
            let exact = origin_in_period(truth.map(|u| u * per_unit));
            let near = origin_in_period(metres);
            for axis in 0..3 {
                let d = (near[axis] - exact[axis]).abs();
                assert!(
                    d < 0.5 || (PERIOD_M - d) < 0.5,
                    "axis {axis}: {near:?} vs {exact:?}"
                );
            }
            assert_eq!(square_parity(near), square_parity(exact));
        }
    }

    #[test]
    fn body_and_squares_use_the_planet_metre() {
        // C4 / AC4 of #648: on Earth one scene unit is Earth's own metre
        // (6,371 km at the ground corners), the body is 1.8 m in it and a
        // square is 100 m in it.
        let universe = on_planet(50.0);
        let per_unit = universe_core::shell::metres_per_unit(1.0);
        let host = surface_host(&universe).expect("a surface");
        assert_eq!(host.frame.metres_per_unit(), per_unit, "Earth's metre");
        let metre = universe_core::body::metre_at(&universe);
        let tall = universe_core::body::body_height(metre) * per_unit;
        assert!((tall - 1.8).abs() < 1e-9, "the body is {tall} m");
        let view = squares_view(
            &host,
            universe.path.offset(),
            true,
            &SquaresTuning::default(),
        );
        let scale = f64::from(view.origin.w);
        assert!((scale / per_unit - 1.0).abs() < 1e-6, "{scale} m per unit");
        assert_eq!(view.look.y, 100.0, "100 m squares");
        let a = planet_metres(&view, [0.0; 3]);
        let b = planet_metres(&view, [100.0 / per_unit, 0.0, 0.0]);
        let across = (0..3).map(|i| (b[i] - a[i]).powi(2)).sum::<f64>().sqrt();
        assert!((across - 100.0).abs() < 0.05, "one square spans {across} m");
    }

    #[test]
    fn normal_look_turns_the_squares_off() {
        let universe = on_planet(5.0e5);
        let host = surface_host(&universe).expect("a surface");
        let view = squares_view(
            &host,
            universe.path.offset(),
            false,
            &SquaresTuning::default(),
        );
        assert_eq!(view.look.x, 0.0);
        assert_eq!(view.look.y, 100.0, "100 m squares (R3)");
    }

    #[test]
    fn the_game_squares_values_are_unchanged() {
        // C9 / AC8 of #665: the game draws 100 m black-and-white squares,
        // pure from 8 px and even gray at 2 px, and never a debug view.
        let tuning = SquaresTuning::default();
        assert_eq!(tuning.square_m, 100.0);
        assert_eq!(tuning.colour_a, [0.0; 3], "black");
        assert_eq!(tuning.colour_b, [1.0; 3], "white");
        assert_eq!((tuning.sharp_px, tuning.gray_px), (8.0, 2.0));
        assert_eq!(tuning.debug, DebugView::Off);
        let universe = on_planet(5.0e4);
        let host = surface_host(&universe).expect("a surface");
        let view = squares_view(&host, universe.path.offset(), true, &tuning);
        assert_eq!(view.look, Vec4::new(1.0, 100.0, 8.0, 2.0));
        assert_eq!(view.colour_a, Vec4::new(0.0, 0.0, 0.0, 1.0));
        assert_eq!(view.colour_b, Vec4::new(1.0, 1.0, 1.0, 1.0));
        assert_eq!(view.debug.x, 0.0, "no debug view");
        // The even tone between the two colours is the game's gray.
        let mean = (view.colour_a + view.colour_b) * 0.5;
        assert_eq!(f64::from(mean.x), universe_core::squares::GRAY);
    }

    #[test]
    fn a_tuned_view_reaches_the_uniform() {
        let universe = on_planet(5.0e4);
        let host = surface_host(&universe).expect("a surface");
        let tuning = SquaresTuning {
            square_m: 1_000.0,
            colour_a: [1.0, 0.0, 0.0],
            colour_b: [0.0, 0.0, 1.0],
            sharp_px: 32.0,
            gray_px: 4.0,
            debug: DebugView::Distance,
            distance_m: [10.0, 500.0],
        };
        let view = squares_view(&host, universe.path.offset(), false, &tuning);
        assert_eq!(view.look, Vec4::new(0.0, 1_000.0, 32.0, 4.0));
        assert_eq!(view.colour_a, Vec4::new(1.0, 0.0, 0.0, 1.0));
        assert_eq!(view.colour_b, Vec4::new(0.0, 0.0, 1.0, 1.0));
        assert_eq!(view.debug, Vec4::new(3.0, 10.0, 500.0, 0.0));
        // The origin is reduced by twice the tuned square, at 1 m and 10 km.
        for square_m in [1.0, 1_000.0, 10_000.0] {
            let sized = SquaresTuning { square_m, ..tuning };
            let view = squares_view(&host, universe.path.offset(), true, &sized);
            let origin = planet_metres(&view, [0.0; 3]);
            assert!(
                origin
                    .iter()
                    .all(|m| (0.0..2.0 * square_m + 1e-3).contains(m)),
                "{square_m} m: {origin:?}"
            );
        }
    }

    #[test]
    fn a_debug_view_follows_the_camera_with_the_squares_off() {
        // Off, then a debug view, then off again: the uniform is rewritten
        // every frame only while something reads the planet position.
        let off = SquaresTuning::default();
        assert!(!follows_camera(false, &off), "the Normal look writes once");
        for debug in [DebugView::Position, DebugView::Normal, DebugView::Distance] {
            assert!(follows_camera(false, &SquaresTuning { debug, ..off }));
        }
        assert!(follows_camera(true, &off));
        let codes: Vec<f32> = DebugView::ALL.iter().map(|v| v.code()).collect();
        assert_eq!(codes, [0.0, 1.0, 2.0, 3.0]);
    }
}
