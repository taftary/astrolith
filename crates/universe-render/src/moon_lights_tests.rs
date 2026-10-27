use super::*;
use crate::window_tests::headless_app;
use universe_core::moonlight::NightLight;
use universe_core::nav::MarkerIndex;

/// The moon lights' illuminances, by lamp index.
fn lamps(app: &mut App) -> Vec<f32> {
    let mut out = vec![0.0; MAX_MOON_LIGHTS];
    for (lamp, light) in app
        .world_mut()
        .query::<(&MoonLamp, &DirectionalLight)>()
        .iter(app.world())
    {
        out[lamp.0] = light.illuminance;
    }
    out
}

#[test]
fn the_moon_lights_follow_the_model_and_the_night_light_setting() {
    // #981 round 2: at the planet each lamp shows its moon's share of the
    // sunlight from the model; off the planet they are dark; Night light
    // 0% puts them out.
    let mut app = headless_app();
    app.update();
    assert!(
        lamps(&mut app).iter().all(|l| *l == 0.0),
        "dark off the planet"
    );
    {
        let mut universe = app.world_mut().resource_mut::<Universe>();
        for _ in 0..9 {
            let marker = universe.autopilot_target().expect("dive continues");
            assert!(universe.open(MarkerIndex(marker)), "marker opens");
        }
    }
    // Find an hour with a moon up at the camera.
    let mut lit = false;
    for hour in 0..(24 * 30) {
        // The sky follows its clock each frame (`sky::advance_sky`).
        app.world_mut()
            .resource_mut::<crate::sky::SkyClock>()
            .0
            .game_s = f64::from(hour) * 3_600.0;
        // A viewer on the planet's +Y side (headless apps draw no camera,
        // so the render origin is placed by hand).
        app.world_mut()
            .resource_mut::<crate::style::RenderOrigin>()
            .camera = [0.0, 6.4e6, 0.0];
        app.update();
        let camera = app.world().resource::<crate::style::RenderOrigin>().camera;
        let want = sky_now(
            app.world().resource::<Universe>(),
            camera,
            NightLight::default(),
        )
        .and_then(|sky| sky.moons[0])
        .map_or(0.0, |moon| {
            moon.share * universe_core::light::SUN_ILLUMINANCE
        });
        let got = f64::from(lamps(&mut app)[0]);
        assert!(
            (got - want).abs() < 1e-3,
            "hour {hour}: lamp {got}, model {want}"
        );
        if want > 0.0 {
            lit = true;
            break;
        }
    }
    assert!(lit, "a moon rose within 30 days");
    app.world_mut()
        .resource_mut::<crate::SettingsStore>()
        .0
        .night_light = NightLight::new(0).expect("0%");
    app.update();
    assert!(
        lamps(&mut app).iter().all(|l| *l == 0.0),
        "Night light 0% puts them out"
    );
}

#[test]
fn the_night_light_row_steps_within_its_range() {
    // S2 (#981): Display > Scene > Night light, 0% to 100% by tens,
    // default 50%.
    use crate::settings_ui::{SettingStep, StepField};
    let mut app = headless_app();
    app.update();
    app.world_mut()
        .resource_mut::<NextState<crate::Screen>>()
        .set(crate::Screen::Settings);
    app.update();
    app.update();
    let get = |app: &App| {
        app.world()
            .resource::<crate::SettingsStore>()
            .0
            .night_light
            .get()
    };
    assert_eq!(get(&app), 50);
    let mut press = |dir: i32| {
        let marker = SettingStep {
            field: StepField::NightLight,
            dir,
        };
        let entity = app
            .world_mut()
            .query::<(Entity, &SettingStep)>()
            .iter(app.world())
            .find(|(_, held)| **held == marker)
            .map(|(entity, _)| entity)
            .expect("the Night light row");
        *app.world_mut()
            .get_mut::<Interaction>(entity)
            .expect("a button") = Interaction::Pressed;
        app.update();
        *app.world_mut()
            .get_mut::<Interaction>(entity)
            .expect("a button") = Interaction::None;
        app.update();
        get(&app)
    };
    assert_eq!(press(1), 60);
    for _ in 0..10 {
        press(1);
    }
    assert_eq!(press(1), 100, "stops at 100%");
    for _ in 0..12 {
        press(-1);
    }
    assert_eq!(press(-1), 0, "stops at 0%");
}
