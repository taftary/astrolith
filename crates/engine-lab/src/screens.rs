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
            }
        }
        ShotView::ShadersGreyed => {
            while lab.scene.level().get() > 9 {
                lab.parent();
            }
            lab.selection = Selection::of(Node::Shader(ShaderId::PlanetSquares));
        }
    }
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
