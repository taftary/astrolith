//! `engine-lab --screens <dir>`: takes every listed picture and exits
//! (#640, AC12), the way the game's screens mode does (ADR 0021).
//!
//! For each shot of [`engine_lab::shots::SHOTS`] the sequencer sets the
//! lab up (view, toggles, selected node), lets it settle, saves one PNG of
//! the window into the folder, and moves on. At the end it writes
//! `screens.txt`, prints `SCREENS-OK files=<n> dir=<dir>` and exits 0. A
//! shot that never arrives ends the run with `SCREENS-FAIL` and exit 1.

use crate::window::Lab;
use bevy::prelude::*;
use bevy::render::view::window::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};
use engine_lab::code::ShaderCode;
use engine_lab::nodes::Node;
use engine_lab::nodes::ViewToggles;
use engine_lab::orbit::CameraMode;
use engine_lab::select::Selection;
use engine_lab::shaders::{ShaderId, ShaderPanel};
use engine_lab::shots::{SHOTS, ShotView, SquaresShot, manifest};

/// Frames a shot settles before its picture (the surface rebuilds and the
/// shaders compile on the first ones).
const SETTLE_FRAMES: u32 = 30;

/// Frames a picture may take to arrive before the run fails.
const CAPTURE_CAP: u32 = 600;

/// Height of the free-fly ground shot, metres.
const GROUND_SHOT_M: f64 = 3.0;

/// Height of the Planet squares pictures, metres: 100 m squares are big
/// near the camera and fade toward the horizon (#665).
const SQUARES_SHOT_M: f64 = 400.0;

/// Adds the screens sequencer for `dir`.
pub(crate) struct LabScreensPlugin {
    /// Folder the pictures go to (it exists).
    pub(crate) dir: String,
}

#[derive(Resource, Debug)]
struct Sequencer {
    dir: String,
    shot: usize,
    phase: Phase,
    saved: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Setup,
    Settle(u32),
    Capture(u32),
}

/// Set by the screenshot observer when a picture was saved.
#[derive(Resource, Debug, Default)]
struct Arrived(bool);

impl Plugin for LabScreensPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Sequencer {
            dir: self.dir.clone(),
            shot: 0,
            phase: Phase::Setup,
            saved: 0,
        })
        .init_resource::<Arrived>()
        .add_systems(Update, sequence.before(crate::window::apply_lab));
    }
}

/// Puts the lab into the state `view` shows.
fn set_up(lab: &mut Lab, view: ShotView) {
    lab.toggles = ViewToggles::default();
    lab.shaders = ShaderPanel::default();
    lab.code = ShaderCode::default();
    lab.generated = false;
    // Every shot starts on the home planet (#654: some leave it).
    let home = engine_lab::scene::LabScene::default_home();
    if lab.scene != home {
        lab.scene = home;
        lab.rebuild = true;
    }
    lab.home();
    lab.selection = Selection::of(Node::Planet);
    match view {
        ShotView::Home => {}
        ShotView::Preset(preset) => lab.preset(preset),
        ShotView::FreeFlyGround => {
            let ground = universe_core::terrain::surface_radius([0.0, 1.0, 0.0]);
            let lift = GROUND_SHOT_M / engine_lab::units::metres_per_unit(1.0);
            lab.mode = CameraMode::Free;
            lab.free_camera = [0.0, ground + lift, 0.0];
            lab.free_yaw = 0.0;
            lab.free_pitch = -0.3;
        }
        ShotView::Wireframe => lab.toggles.wireframe = true,
        ShotView::Normals => lab.toggles.normals = true,
        ShotView::LodDepth => lab.toggles.lod_colors = true,
        ShotView::Node(node) => lab.selection = Selection::of(node),
        ShotView::Selected => {
            lab.selection = Selection::of(Node::Moon(0));
        }
        ShotView::MoonOnShow => {
            lab.selection = Selection::of(Node::Moon(0));
            lab.show_selected();
        }
        ShotView::Level(level) => {
            while lab.scene.level().get() > level {
                lab.parent();
            }
        }
        ShotView::Squares(shot) => {
            let ground = universe_core::terrain::surface_radius([0.0, 1.0, 0.0]);
            let lift = SQUARES_SHOT_M / engine_lab::units::metres_per_unit(1.0);
            lab.mode = CameraMode::Free;
            lab.free_camera = [0.0, ground + lift, 0.0];
            lab.free_yaw = 0.0;
            lab.free_pitch = -0.5;
            lab.selection = Selection::of(Node::Shader(ShaderId::PlanetSquares));
            match shot {
                SquaresShot::On => lab.shaders.on = true,
                SquaresShot::Tuned => {
                    lab.shaders.on = true;
                    lab.shaders.tuning.square_m = 1_000.0;
                    lab.shaders.tuning.colour_a = [1.0, 0.0, 0.0];
                }
                SquaresShot::Debug(view) => lab.shaders.tuning.debug = view,
                SquaresShot::Edited => {
                    lab.shaders.on = true;
                    lab.code.open = true;
                    lab.code.edited = engine_lab::code::GAME_CODE.replace("* 0.5;", "* 0.0;");
                    lab.code.save();
                }
                SquaresShot::CodeError => {
                    lab.shaders.on = true;
                    lab.code.open = true;
                    lab.code.edited =
                        engine_lab::code::GAME_CODE.replace("let alpha = ", "let alpha = = ");
                    lab.code.save();
                }
            }
        }
        ShotView::ShadersGreyed => {
            while lab.scene.level().get() > 9 {
                lab.parent();
            }
            lab.selection = Selection::of(Node::Shader(ShaderId::PlanetSquares));
        }
        ShotView::GalleryPlanet(kind) => {
            let Some(entry) = universe_core::world::gallery::PLANET_GALLERY
                .iter()
                .find(|entry| entry.kind == kind)
            else {
                return;
            };
            navigate_gallery(lab, &entry.star, entry.planet);
            lab.generated = true;
            lab.selection = Selection::of(Node::Planet);
        }
        ShotView::GalleryMoon(kind) => {
            let Some(entry) = universe_core::world::gallery::MOON_GALLERY
                .iter()
                .find(|entry| entry.kind == kind)
            else {
                return;
            };
            navigate_gallery(lab, &entry.star, entry.planet);
            lab.generated = true;
            lab.selection = Selection::of(Node::Moon(entry.moon_index));
            lab.show_selected();
        }
    }
}

/// Navigates the lab to a gallery entry's planet (T13): the gallery root
/// seed, its star path, then the planet, framed at Home.
fn navigate_gallery(lab: &mut Lab, star: &[u32; 8], planet: usize) {
    use universe_core::world::gallery::GALLERY_ROOT;
    if lab.scene.seed() != GALLERY_ROOT {
        lab.scene.set_seed(GALLERY_ROOT);
    }
    lab.scene.show_gallery(star, planet);
    lab.home();
}

fn sequence(
    mut commands: Commands,
    mut seq: ResMut<Sequencer>,
    mut arrived: ResMut<Arrived>,
    mut lab: ResMut<Lab>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(shot) = SHOTS.get(seq.shot).copied() else {
        let manifest_path = std::path::Path::new(&seq.dir).join("screens.txt");
        let files = seq.saved + usize::from(std::fs::write(&manifest_path, manifest()).is_ok());
        println!("SCREENS-OK files={files} dir={}", seq.dir);
        exit.write(AppExit::Success);
        return;
    };
    match seq.phase {
        Phase::Setup => {
            set_up(&mut lab, shot.view);
            seq.phase = Phase::Settle(SETTLE_FRAMES);
        }
        Phase::Settle(0) => {
            arrived.0 = false;
            let path = std::path::Path::new(&seq.dir).join(shot.file);
            let mut save = save_to_disk(path);
            commands
                .spawn((Name::new("lab-screens-shot"), Screenshot::primary_window()))
                .observe(
                    move |captured: On<ScreenshotCaptured>, mut arrived: ResMut<Arrived>| {
                        arrived.0 = true;
                        save(captured);
                    },
                );
            seq.phase = Phase::Capture(0);
        }
        Phase::Settle(left) => seq.phase = Phase::Settle(left - 1),
        Phase::Capture(waited) => {
            if arrived.0 {
                seq.saved += 1;
                seq.shot += 1;
                seq.phase = Phase::Setup;
            } else if waited >= CAPTURE_CAP {
                println!("SCREENS-FAIL {} never arrived dir={}", shot.file, seq.dir);
                exit.write(AppExit::from_code(1));
            } else {
                seq.phase = Phase::Capture(waited + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_lab::nodes::Node;
    use engine_lab::scene::LabScene;

    #[test]
    fn gallery_shots_draw_generated_and_navigate() {
        // T13: gallery shots switch the Generated look on and land on
        // their planet (or moon); every other shot draws Normal.
        let mut lab = Lab::new(LabScene::default_home());
        set_up(&mut lab, ShotView::Home);
        assert!(!lab.generated);
        for kind in universe_core::world::classify::WorldKind::ALL {
            if kind == universe_core::world::classify::WorldKind::Rogue {
                continue;
            }
            set_up(&mut lab, ShotView::GalleryPlanet(kind));
            assert!(lab.generated, "{kind:?} draws Normal");
            assert_eq!(lab.scene.error(), None, "{kind:?} explains itself");
            assert!(lab.scene.on_planet());
        }
        for kind in universe_core::world::moons::MoonKind::ALL {
            set_up(&mut lab, ShotView::GalleryMoon(kind));
            assert!(lab.generated, "{kind:?} draws Normal");
            assert_eq!(lab.scene.error(), None, "{kind:?} explains itself");
            assert!(lab.scene.moon_on_show().is_some(), "{kind:?} shows no moon");
        }
        set_up(&mut lab, ShotView::Home);
        assert!(!lab.generated, "Generated leaks past the gallery");
        assert_eq!(lab.selection, Selection::of(Node::Planet));
    }
}
