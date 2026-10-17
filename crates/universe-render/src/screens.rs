//! Screenshot mode sequencer: shows each UI screen and captures it (#492).
//!
//! [`ScreensPlugin`] is added only by `universe-app --screens <folder>`. It
//! walks the fixed shot list [`SHOTS`] (the single place to add a screen):
//! for each shot it sizes the window, opens the screen with its panel setup,
//! waits [`SETTLE_FRAMES`] frames for layout, then requests one screenshot of
//! the primary window. Each captured picture lands in [`ScreensQueue`]; the
//! app driver writes the files and owns the stdout protocol. This module does
//! no file I/O and prints nothing (`E-APP-THIN`, `E-STDOUT-PROTOCOL`).
//! A picture of one solid colour means the scene has not drawn yet (DirectX
//! 12 compiles shaders while the app runs, #492 correction round 1): the
//! shot settles again and is re-taken instead of being kept.
//! A shot that takes more than [`SHOT_FRAME_CAP`] frames ends the run as a
//! failure (`<file> is blank` when only blank pictures arrived), so the mode
//! never hangs.

use crate::menu::Screen;
use crate::panel::PanelState;
use crate::quit_popup::QuitPopup;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};

/// Frames to wait after a shot's setup before requesting the picture.
pub(crate) const SETTLE_FRAMES: u32 = 30;

/// Frames to wait before a surface shot (#458): the first surface mesh
/// compiles its pipeline while the app runs (DirectX 12 on Windows), and
/// a mesh whose pipeline is not ready is skipped for that frame.
pub(crate) const SURFACE_SETTLE_FRAMES: u32 = 240;

/// Settle frames for `shot`: surface shots wait for their pipeline.
#[must_use]
pub(crate) const fn settle_frames(shot: &Shot) -> u32 {
    if shot.dive > 0 {
        SURFACE_SETTLE_FRAMES
    } else {
        SETTLE_FRAMES
    }
}

/// Frames one shot may take (setup, settle, capture) before the run fails.
pub(crate) const SHOT_FRAME_CAP: u32 = 600;

/// Wide picture size in pixels.
pub(crate) const WIDE: (u32, u32) = (1280, 720);

/// Narrow picture size in pixels (below the 720 px panel break).
pub(crate) const NARROW: (u32, u32) = (640, 720);

/// Panel setup for one shot: expanded category and selected leaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PanelSetup {
    /// Category id to expand.
    pub expand: &'static str,
    /// Leaf id to select.
    pub select: &'static str,
}

/// One picture the mode takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Shot {
    /// File name of the picture (`<name>.png`).
    pub file: &'static str,
    /// Screen shown.
    pub screen: Screen,
    /// Window size in physical pixels.
    pub size: (u32, u32),
    /// Panel setup (`None` for screens without a panel).
    pub panel: Option<PanelSetup>,
    /// Quit pop-up open over the screen (#473 T6).
    pub popup: bool,
    /// Journey opens before the shot: `0` keeps the universe as it is; `n`
    /// restarts the demo universe and opens the first `n` autopilot
    /// markers, parked at the entry rest and aimed at the next one (#458).
    pub dive: usize,
    /// The touch controls show (Touch controls: always) for the shot
    /// (#561).
    pub touch: bool,
}

/// Settings with its first category open.
const SETTINGS_FIRST: PanelSetup = PanelSetup {
    expand: "display",
    select: "scene",
};

/// Settings on the editable Controls > Keys page (#560).
const SETTINGS_KEYS: PanelSetup = PanelSetup {
    expand: "controls",
    select: "keys",
};

/// Settings on the Controls > Touch page (#561).
const SETTINGS_TOUCH: PanelSetup = PanelSetup {
    expand: "controls",
    select: "touch",
};

/// Guides with its first page open.
const GUIDES_FIRST: PanelSetup = PanelSetup {
    expand: "about",
    select: "what",
};

/// Every shot, in order (Spec v1 of #492). Add a screen with one entry.
pub(crate) const SHOTS: &[Shot] = &[
    Shot {
        file: "universe.png",
        screen: Screen::Universe,
        size: WIDE,
        panel: None,
        popup: false,
        dive: 0,
        touch: false,
    },
    Shot {
        file: "menu.png",
        screen: Screen::Menu,
        size: WIDE,
        panel: None,
        popup: false,
        dive: 0,
        touch: false,
    },
    Shot {
        file: "quit.png",
        screen: Screen::Menu,
        size: WIDE,
        panel: None,
        popup: true,
        dive: 0,
        touch: false,
    },
    Shot {
        file: "settings-wide.png",
        screen: Screen::Settings,
        size: WIDE,
        panel: Some(SETTINGS_FIRST),
        popup: false,
        dive: 0,
        touch: false,
    },
    Shot {
        file: "settings-narrow.png",
        screen: Screen::Settings,
        size: NARROW,
        panel: Some(SETTINGS_FIRST),
        popup: false,
        dive: 0,
        touch: false,
    },
    Shot {
        file: "settings-keys.png",
        screen: Screen::Settings,
        size: WIDE,
        panel: Some(SETTINGS_KEYS),
        popup: false,
        dive: 0,
        touch: false,
    },
    Shot {
        file: "settings-touch.png",
        screen: Screen::Settings,
        size: WIDE,
        panel: Some(SETTINGS_TOUCH),
        popup: false,
        dive: 0,
        touch: false,
    },
    Shot {
        file: "guides-wide.png",
        screen: Screen::Guides,
        size: WIDE,
        panel: Some(GUIDES_FIRST),
        popup: false,
        dive: 0,
        touch: false,
    },
    Shot {
        file: "guides-narrow.png",
        screen: Screen::Guides,
        size: NARROW,
        panel: Some(GUIDES_FIRST),
        popup: false,
        dive: 0,
        touch: false,
    },
    surface_shot("approach-l9.png", 8),
    surface_shot("surface-l10.png", 9),
    touch_shot("touch-dive.png", 1),
];

/// A wide universe shot opened `dive` markers down the journey (#458).
const fn surface_shot(file: &'static str, dive: usize) -> Shot {
    Shot {
        file,
        screen: Screen::Universe,
        size: WIDE,
        panel: None,
        popup: false,
        dive,
        touch: false,
    }
}

/// A shot with the touch controls showing (Touch controls: always) in
/// dive mode (#561).
const fn touch_shot(file: &'static str, dive: usize) -> Shot {
    Shot {
        touch: true,
        ..surface_shot(file, dive)
    }
}

/// Number of shots the mode takes.
#[must_use]
pub fn shot_count() -> usize {
    SHOTS.len()
}

/// One captured picture, handed to the app driver.
#[derive(Debug, Clone)]
pub struct CapturedShot {
    /// File name from the shot list.
    pub file: &'static str,
    /// Window size the shot asked for, in physical pixels.
    pub size: (u32, u32),
    /// Captured window picture.
    pub image: Image,
}

/// Captured pictures plus the run's end state, drained by the app driver.
#[derive(Resource, Debug, Default)]
pub struct ScreensQueue {
    /// Pictures captured and not yet written.
    pub captured: Vec<CapturedShot>,
    /// Set when every shot has been captured.
    pub done: bool,
    /// Set when the run cannot finish (the reason, without reserved words).
    pub failure: Option<String>,
}

/// Where the sequencer is within the current shot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Phase {
    /// The shot's window size, screen, and panel are about to be applied.
    #[default]
    Setup,
    /// Waiting this many more frames for layout.
    Settle(u32),
    /// Screenshot requested; waiting for the picture.
    Capture,
    /// Every shot captured, or the run failed.
    Finished,
}

/// Sequencer state: current shot index, phase, and frames spent on it.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Sequencer {
    /// Index into [`SHOTS`].
    pub shot: usize,
    /// Phase within the shot.
    pub phase: Phase,
    /// Frames spent on the current shot.
    pub frames: u32,
    /// Pictures captured so far (advances the capture phase).
    pub captured: usize,
    /// Blank pictures received for the current shot.
    pub blanks: u32,
    /// The last picture of the current shot was blank: settle and re-take.
    pub retake: bool,
}

/// Screenshot mode: steps through the shot list and fills [`ScreensQueue`].
pub struct ScreensPlugin;

impl Plugin for ScreensPlugin {
    /// Registers the queue, the sequencer, and its step system.
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreensQueue>()
            .init_resource::<Sequencer>()
            .add_systems(
                Update,
                (step_sequencer, dive_for_shot, touch_for_shot).chain(),
            );
    }
}

/// Advances the sequencer by one frame.
pub(crate) fn step_sequencer(
    mut commands: Commands,
    mut seq: ResMut<Sequencer>,
    mut queue: ResMut<ScreensQueue>,
    mut next: ResMut<NextState<Screen>>,
    mut panel: ResMut<PanelState>,
    mut popup: ResMut<QuitPopup>,
    mut window: Single<&mut Window>,
) {
    if seq.phase == Phase::Finished {
        return;
    }
    let Some(shot) = SHOTS.get(seq.shot).copied() else {
        seq.phase = Phase::Finished;
        queue.done = true;
        return;
    };
    seq.frames += 1;
    // Held every frame of the shot: leaving a screen closes the pop-up, so a
    // one-off set could be undone by the screen change it rides with.
    if popup.open != shot.popup {
        popup.open = shot.popup;
    }
    if seq.frames > SHOT_FRAME_CAP {
        seq.phase = Phase::Finished;
        queue.failure = Some(if seq.blanks > 0 {
            format!("{} is blank", shot.file)
        } else {
            format!("shot {} timed out", shot.file)
        });
        return;
    }
    match seq.phase {
        Phase::Setup => {
            window
                .resolution
                .set_physical_resolution(shot.size.0, shot.size.1);
            next.set(shot.screen);
            *panel = panel_for(shot.panel);
            seq.phase = Phase::Settle(settle_frames(&shot));
        }
        Phase::Settle(0) => {
            let (file, size) = (shot.file, shot.size);
            commands
                .spawn((Name::new("screens-shot"), Screenshot::primary_window()))
                .observe(
                    move |captured: On<ScreenshotCaptured>,
                          mut queue: ResMut<ScreensQueue>,
                          mut seq: ResMut<Sequencer>| {
                        receive_picture(&mut seq, &mut queue, file, size, &captured.image);
                    },
                );
            seq.phase = Phase::Capture;
        }
        Phase::Settle(left) => seq.phase = Phase::Settle(left - 1),
        Phase::Capture => {
            if seq.retake {
                seq.retake = false;
                seq.phase = Phase::Settle(SETTLE_FRAMES);
            } else if seq.captured > seq.shot {
                seq.shot += 1;
                seq.frames = 0;
                seq.blanks = 0;
                seq.phase = Phase::Setup;
                if seq.shot >= SHOTS.len() {
                    seq.phase = Phase::Finished;
                    queue.done = true;
                }
            }
        }
        Phase::Finished => {}
    }
}

/// Opens the current shot's journey markers once per shot (#458).
///
/// Runs after the sequencer; a shot with `dive > 0` restarts the demo
/// universe and dives once, then leaves it alone while the shot settles.
/// Without a universe (UI-only test apps) it does nothing.
pub(crate) fn dive_for_shot(
    seq: Res<Sequencer>,
    universe: Option<ResMut<crate::Universe>>,
    nav: Option<ResMut<crate::input::Navigation>>,
    flight: Option<ResMut<crate::input::Flight>>,
    normal: Option<ResMut<crate::normal_mode::NormalState>>,
    mut dived: Local<Option<usize>>,
) {
    let Some(shot) = SHOTS.get(seq.shot) else {
        return;
    };
    if shot.dive == 0 || *dived == Some(seq.shot) {
        return;
    }
    if let (Some(mut universe), Some(mut nav)) = (universe, nav) {
        dive_to(&mut universe, &mut nav, shot.dive);
        if let Some(mut normal) = normal {
            // A fresh body state for every shot.
            normal.active = false;
        }
        if let Some(mut flight) = flight {
            // Every shot runs in dive mode.
            flight.mode = crate::input::FlightMode::Dive;
        }
        *dived = Some(seq.shot);
    }
}

/// Shows the touch controls (Touch controls: always) for a touch shot and
/// puts the setting back for the others (#561). Without a settings store
/// (UI-only test apps) it does nothing.
pub(crate) fn touch_for_shot(seq: Res<Sequencer>, store: Option<ResMut<crate::SettingsStore>>) {
    let (Some(shot), Some(mut store)) = (SHOTS.get(seq.shot), store) else {
        return;
    };
    if store.0.touch_always != shot.touch {
        store.0.touch_always = shot.touch;
    }
}

/// Restarts the demo universe and opens `dive` journey markers (#458).
///
/// The camera parks at a context view of the opened cell (three cell
/// units out at the planet, a little above the ground in a tail cell),
/// aimed at the next journey marker. At the level above the planet (L9)
/// it parks mid-way through the planet's fade-in, facing it (#603).
fn dive_to(universe: &mut crate::Universe, nav: &mut crate::input::Navigation, dive: usize) {
    use universe_core::nav::{DEMO_SEED, MarkerIndex};
    let chain = universe_core::nest::autopilot_path(DEMO_SEED);
    universe.0 = universe_core::nav::Universe::new(DEMO_SEED);
    for &marker in chain.iter().take(dive) {
        if !universe.0.open(MarkerIndex(marker)) {
            break;
        }
    }
    // A context view three cell units out; on the planet (L10) just
    // above the surface, looking down at the dome; at the level above
    // the planet (L9) the offset below parks mid-way through the
    // planet's fade-in, facing it.
    if universe.0.level().get() == 10
        && let Some(frame) = universe.0.frame.as_ref()
    {
        // Just above the surface, looking down at the dome: inside every
        // shell so none draws, but outside the camera near plane (about
        // 5% of the distance) so the ground is not clipped.
        let ground = frame.surface_point([0.0, 1.0, 0.0]);
        let offset = [ground[0] * 1.08, ground[1] * 1.08, ground[2] * 1.08];
        universe.0.path.set_offset(offset);
        let down = [
            ground[0] - offset[0],
            ground[1] - offset[1],
            ground[2] - offset[2],
        ];
        let length = (down[0] * down[0] + down[1] * down[1] + down[2] * down[2])
            .sqrt()
            .max(1e-12);
        nav.forward = [down[0] / length, down[1] / length, down[2] / length];
    } else {
        universe.0.path.set_offset([0.0, 0.0, 3.0]);
    }
    nav.target = chain.get(dive).copied();
    if universe.0.level().get() == 9
        && let Some(point) = nav
            .target
            .and_then(|marker| universe.0.open.points.get(marker as usize))
        && let Some(ratio) = universe_core::nest::child_ratio(universe.0.level())
    {
        // Out along the planet's line from the cell centre, where its shell
        // looks 80% of its opening size: mid-way through the fade.
        let at = point.portal_position();
        let distance = 0.5 * ratio / (universe_core::nest::OPEN_ANGLE * 0.8).sin();
        let length = (at[0] * at[0] + at[1] * at[1] + at[2] * at[2])
            .sqrt()
            .max(1e-12);
        let out = [at[0] / length, at[1] / length, at[2] / length];
        universe.0.path.set_offset([
            at[0] + out[0] * distance,
            at[1] + out[1] * distance,
            at[2] + out[2] * distance,
        ]);
        nav.forward = [-out[0], -out[1], -out[2]];
    }
}

/// Takes one delivered picture: a blank one asks for a re-take, any other is
/// queued for the app driver and advances the capture count.
pub(crate) fn receive_picture(
    seq: &mut Sequencer,
    queue: &mut ScreensQueue,
    file: &'static str,
    size: (u32, u32),
    image: &Image,
) {
    if is_one_colour(image.data.as_deref().unwrap_or_default()) {
        seq.blanks += 1;
        seq.retake = true;
        return;
    }
    queue.captured.push(CapturedShot {
        file,
        size,
        image: image.clone(),
    });
    seq.captured += 1;
}

/// Whether 4-byte pixels all share one colour (pure; empty counts as blank).
#[must_use]
pub(crate) fn is_one_colour(pixels: &[u8]) -> bool {
    let mut chunks = pixels.chunks_exact(4);
    chunks
        .next()
        .is_none_or(|first| chunks.all(|pixel| pixel == first))
}

/// Panel state for one shot: first category expanded, first leaf selected,
/// narrow mode on the list (pure).
#[must_use]
pub(crate) fn panel_for(setup: Option<PanelSetup>) -> PanelState {
    match setup {
        Some(setup) => PanelState {
            selected: setup.select.to_string(),
            expanded: vec![setup.expand.to_string()],
            detail: false,
        },
        None => PanelState::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panel::{GUIDES_TREE, SETTINGS_TREE, is_narrow};
    use bevy::state::app::StatesPlugin;

    /// Headless app with only the state, panel state, a window component
    /// (no OS window, `E-TEST-NO-WINDOW`), and the sequencer.
    fn headless_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin));
        app.init_state::<Screen>();
        app.init_resource::<PanelState>();
        app.init_resource::<QuitPopup>();
        app.add_plugins(ScreensPlugin);
        app.world_mut().spawn(Window::default());
        app
    }

    /// Steps until the sequencer reaches the capture phase (capped).
    fn run_to_capture(app: &mut App) {
        for _ in 0..(SURFACE_SETTLE_FRAMES + 5) {
            app.update();
            if app.world().resource::<Sequencer>().phase == Phase::Capture {
                return;
            }
        }
        panic!("capture phase not reached");
    }

    /// A picture with the given 4-byte pixels (no GPU).
    fn picture(pixels: &[u8]) -> Image {
        Image {
            data: Some(pixels.to_vec()),
            ..Image::default()
        }
    }

    /// Two different pixels: a drawn scene.
    const DRAWN: [u8; 8] = [0, 0, 0, 255, 200, 100, 50, 255];

    /// One colour twice: a scene not drawn yet.
    const BLANK: [u8; 8] = [0, 0, 0, 255, 0, 0, 0, 255];

    /// Simulates the GPU delivering `image` for the current shot.
    fn deliver_image(app: &mut App, image: &Image) {
        let shot = SHOTS[app.world().resource::<Sequencer>().shot];
        app.world_mut()
            .resource_scope(|world, mut queue: Mut<ScreensQueue>| {
                let mut seq = world.resource_mut::<Sequencer>();
                receive_picture(&mut seq, &mut queue, shot.file, shot.size, image);
            });
        app.update();
    }

    /// Simulates the GPU delivering a drawn picture for the current shot.
    fn deliver(app: &mut App) {
        deliver_image(app, &picture(&DRAWN));
    }

    #[test]
    fn one_colour_is_blank_and_two_colours_are_not() {
        assert!(is_one_colour(&BLANK));
        assert!(!is_one_colour(&DRAWN));
        assert!(is_one_colour(&[]));
    }

    #[test]
    fn a_blank_picture_is_retaken_not_kept() {
        let mut app = headless_app();
        run_to_capture(&mut app);
        deliver_image(&mut app, &picture(&BLANK));
        let seq = *app.world().resource::<Sequencer>();
        assert_eq!(seq.shot, 0, "still on the first shot");
        assert_eq!(seq.blanks, 1);
        assert!(matches!(seq.phase, Phase::Settle(_)), "settles again");
        assert!(app.world().resource::<ScreensQueue>().captured.is_empty());
        run_to_capture(&mut app);
        deliver(&mut app);
        let seq = *app.world().resource::<Sequencer>();
        assert_eq!((seq.shot, seq.blanks), (1, 0), "next shot, count reset");
        let queue = app.world().resource::<ScreensQueue>();
        assert_eq!(queue.captured.len(), 1);
        assert!(queue.failure.is_none());
    }

    #[test]
    fn a_shot_that_stays_blank_fails_at_the_frame_cap() {
        let mut app = headless_app();
        for _ in 0..(SHOT_FRAME_CAP + 5) {
            if app.world().resource::<Sequencer>().phase == Phase::Capture {
                deliver_image(&mut app, &picture(&BLANK));
            } else {
                app.update();
            }
        }
        let queue = app.world().resource::<ScreensQueue>();
        assert!(!queue.done);
        assert!(queue.captured.is_empty());
        assert_eq!(queue.failure.as_deref(), Some("universe.png is blank"));
    }

    #[test]
    fn shot_list_matches_the_spec_names_and_sizes() {
        let names: Vec<_> = SHOTS.iter().map(|shot| shot.file).collect();
        assert_eq!(
            names,
            [
                "universe.png",
                "menu.png",
                "quit.png",
                "settings-wide.png",
                "settings-narrow.png",
                "settings-keys.png",
                "settings-touch.png",
                "guides-wide.png",
                "guides-narrow.png",
                "approach-l9.png",
                "surface-l10.png",
                "touch-dive.png",
            ]
        );
        for shot in SHOTS {
            let want = if shot.file.contains("narrow") {
                NARROW
            } else {
                WIDE
            };
            assert_eq!(shot.size, want, "{}", shot.file);
        }
        assert_eq!(WIDE, (1280, 720));
        assert_eq!(NARROW, (640, 720));
        let px = |width: u32| f32::from(u16::try_from(width).expect("small width"));
        assert!(is_narrow(px(NARROW.0)) && !is_narrow(px(WIDE.0)));
    }

    #[test]
    fn panel_setups_name_the_first_category_and_leaf() {
        for (setup, tree) in [(SETTINGS_FIRST, SETTINGS_TREE), (GUIDES_FIRST, GUIDES_TREE)] {
            let first = &tree[0];
            assert_eq!(setup.expand, first.id);
            assert_eq!(setup.select, first.children[0].id);
        }
    }

    #[test]
    fn sequencer_shows_every_shot_in_order_then_finishes() {
        let mut app = headless_app();
        for (index, shot) in SHOTS.iter().enumerate() {
            run_to_capture(&mut app);
            assert_eq!(app.world().resource::<Sequencer>().shot, index);
            assert_eq!(
                *app.world().resource::<State<Screen>>().get(),
                shot.screen,
                "{} shows its screen",
                shot.file
            );
            assert_eq!(
                *app.world().resource::<PanelState>(),
                panel_for(shot.panel),
                "{} panel setup",
                shot.file
            );
            assert_eq!(
                app.world().resource::<QuitPopup>().open,
                shot.popup,
                "{} pop-up",
                shot.file
            );
            let mut windows = app.world_mut().query::<&Window>();
            let window = windows.single(app.world()).expect("one window");
            assert_eq!(
                (window.physical_width(), window.physical_height()),
                shot.size,
                "{} window size",
                shot.file
            );
            deliver(&mut app);
        }
        let queue = app.world().resource::<ScreensQueue>();
        assert!(queue.done, "run finishes after the last shot");
        assert!(queue.failure.is_none());
        assert_eq!(queue.captured.len(), SHOTS.len());
    }

    #[test]
    fn a_shot_that_never_arrives_trips_the_frame_cap() {
        let mut app = headless_app();
        for _ in 0..(SHOT_FRAME_CAP + 5) {
            app.update();
        }
        let queue = app.world().resource::<ScreensQueue>();
        assert!(!queue.done);
        assert_eq!(
            queue.failure.as_deref(),
            Some("shot universe.png timed out")
        );
        assert_eq!(app.world().resource::<Sequencer>().phase, Phase::Finished);
    }
}
