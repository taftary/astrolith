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

use crate::Universe;
use crate::planet_approach::{SurfaceHost, surface_host};
use crate::settings_ui::SettingsStore;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use universe_core::coords::Level;
use universe_core::squares::{GRAY_PX, SHARP_PX, SQUARE_M, origin_in_period};

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
/// squares on when `on`.
#[must_use]
pub(crate) fn squares_view(host: &SurfaceHost, camera: [f64; 3], on: bool) -> SquaresUniform {
    let frame = &host.frame;
    let metre_l10 = universe_core::body::metre(Level::new(10).unwrap_or(Level::MIN));
    let planet = |open: [f64; 3]| frame.to_planet(host.to_surface(open));
    let here = planet(camera);
    // The scale is measured at the surface cell's centre, where its numbers
    // are smallest, whatever the camera's distance.
    let [cx, cy, cz] = host.offset;
    let metres_per_unit =
        crate::surface_split::gap(planet(host.offset), planet([cx + 1.0, cy, cz])) / metre_l10;
    let origin = origin_in_period(here.map(|unit| unit / metre_l10));
    let columns = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        .map(|axis| frame.to_planet_direction(axis));
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: small rotation, period-reduced metres and a scale narrowed for the GPU"
    )]
    let narrow = |x: f64| x as f32;
    let [[ax, ay, az], [bx, by, bz], [cx, cy, cz]] = columns;
    let row = |a: f64, b: f64, c: f64| Vec4::new(narrow(a), narrow(b), narrow(c), 0.0);
    let [ox, oy, oz] = origin;
    SquaresUniform {
        row_x: row(ax, bx, cx),
        row_y: row(ay, by, cy),
        row_z: row(az, bz, cz),
        origin: Vec4::new(narrow(ox), narrow(oy), narrow(oz), narrow(metres_per_unit)),
        look: Vec4::new(
            if on { 1.0 } else { 0.0 },
            narrow(SQUARE_M),
            narrow(SHARP_PX),
            narrow(GRAY_PX),
        ),
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

/// Writes this frame's squares view into every surface material while a
/// surface is drawn; with Normal it writes once, on the change.
pub(crate) fn update_squares(
    universe: Res<Universe>,
    store: Res<SettingsStore>,
    mut materials: ResMut<Assets<SurfaceMaterial>>,
    mut last_on: Local<Option<bool>>,
) {
    let on = store.0.planet_squares;
    if !on && *last_on == Some(false) {
        return;
    }
    let camera = universe.path.offset();
    let view = surface_host(&universe).map_or_else(SquaresUniform::default, |host| {
        squares_view(&host, camera, on)
    });
    for (_, material) in materials.iter_mut() {
        if material.extension.squares != view {
            material.extension.squares = view;
        }
    }
    *last_on = Some(on);
}

/// Registers the surface material and its embedded shader (window app).
pub struct PlanetSquaresPlugin;

impl Plugin for PlanetSquaresPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "planet_squares.wgsl");
        app.add_plugins(MaterialPlugin::<SurfaceMaterial>::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use universe_core::squares::{PERIOD_M, square_parity};

    /// The demo journey's universe opened down to level `n`.
    fn opened(n: u8) -> Universe {
        let mut universe = Universe(universe_core::nav::Universe::new(
            universe_core::nav::DEMO_SEED,
        ));
        while universe.level().get() < n {
            let marker = universe.autopilot_target().expect("dive continues");
            assert!(universe.open(universe_core::nav::MarkerIndex(marker)));
        }
        // A camera a little above the ground at the cell's centre.
        let ground = surface_host(&universe)
            .expect("a surface")
            .frame
            .ground_height(0.0, 0.0);
        universe.path.set_offset([0.0, ground + 0.01, 0.0]);
        universe
    }

    #[test]
    fn the_view_maps_render_space_to_planet_metres() {
        for n in 10..=13u8 {
            let universe = opened(n);
            let camera = universe.path.offset();
            let host = surface_host(&universe).expect("a surface");
            let view = squares_view(&host, camera, true);
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
                "L{n}: {moved} m for one unit, want {want}"
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
        let universe = opened(12);
        let host = surface_host(&universe).expect("a surface");
        let camera = universe.path.offset();
        let ground = [0.01, host.frame.ground_height(0.01, -0.02), -0.02];
        let moved = [camera[0] + 0.003, camera[1] - 0.002, camera[2] + 0.004];
        for spot in [camera, moved] {
            let view = squares_view(&host, spot, true);
            let render = [
                ground[0] - spot[0],
                ground[1] - spot[1],
                ground[2] - spot[2],
            ];
            let metres = planet_metres(&view, render);
            let truth = host.frame.to_planet(host.to_surface(ground));
            let metre_l10 = universe_core::body::metre(Level::new(10).expect("L10"));
            let exact = origin_in_period(truth.map(|u| u / metre_l10));
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
    fn normal_look_turns_the_squares_off() {
        let universe = opened(11);
        let host = surface_host(&universe).expect("a surface");
        let view = squares_view(&host, universe.path.offset(), false);
        assert_eq!(view.look.x, 0.0);
        assert_eq!(view.look.y, 100.0, "100 m squares (R3)");
    }
}
