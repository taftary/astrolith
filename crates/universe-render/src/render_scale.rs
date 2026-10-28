//! The internal 3D resolution (#1022 T6, ADR 0054).
//!
//! At native size the 3D camera draws straight into the window, exactly as
//! before. Below it, the camera draws into an image of the scaled size
//! ([`universe_core::quality::scaled_size`]: 10-point steps, the 720-pixel
//! floor, never above native, the aspect kept). The image target's scale
//! factor makes its logical size the window's, so labels, picking and every
//! logical-pixel measure read the same coordinates at every scale. A second
//! camera stretches that image over the window and carries the whole UI at
//! native size. When the size changes the old image is removed from the
//! assets, which releases its texture.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{CameraUpdateSystems, ImageRenderTarget, RenderTarget};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::ui::IsDefaultUiCamera;
use bevy::window::PrimaryWindow;
use universe_core::quality::scaled_size;
use universe_core::settings::{QualityMode, Resolution3d};

use crate::SettingsStore;
use crate::tokens::LINE_MIN_PX;

/// The render layer of the present camera: nothing of the 3D scene is on
/// it, so it draws only its UI.
const PRESENT_LAYER: usize = 31;

/// The effective linear scale of the 3D picture (1 is native). Manual
/// takes it from the saved 3D resolution; Auto's controller writes it
/// (T10). Never saved (Spec v4 point 10).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct RenderScale(pub f64);

impl Default for RenderScale {
    fn default() -> Self {
        RenderScale(1.0)
    }
}

/// A fixed scale for evidence runs, over the settings
/// (`ASTROLITH_RENDER_SCALE`, #1022 T6).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct RenderScalePin(pub f64);

/// The camera that draws the scaled 3D picture and the UI.
#[derive(Component, Debug)]
pub(crate) struct PresentCamera;

/// The full-window node showing the scaled 3D picture.
#[derive(Component, Debug)]
pub(crate) struct PresentImage;

/// The scaled picture in use: its size and image, `None` at native.
#[derive(Resource, Debug, Default)]
pub(crate) struct ScaledTarget {
    /// The image size in physical pixels.
    pub size: Option<UVec2>,
    /// The image.
    pub image: Option<Handle<Image>>,
    /// The window's physical size and scale factor it was made for.
    window: Option<(u32, u32, u32)>,
}

/// The Manual scale for a saved `choice` on a window whose shorter side is
/// `native_short` physical pixels: a preset as tall as the window or
/// taller is Native (Q25). Pure.
#[must_use]
pub(crate) fn manual_scale(native_short: u32, choice: Resolution3d) -> f64 {
    choice
        .short_side()
        .filter(|s| *s < native_short)
        .map_or(1.0, |s| f64::from(s) / f64::from(native_short))
}

/// Spawns the present camera (inactive) and its picture node (hidden).
fn spawn_present(mut commands: Commands) {
    let camera = commands
        .spawn((
            Name::new("present camera"),
            Camera2d,
            Camera {
                order: 1,
                is_active: false,
                ..default()
            },
            RenderLayers::layer(PRESENT_LAYER),
            Msaa::Off,
            Tonemapping::None,
            PresentCamera,
        ))
        .id();
    commands.spawn((
        Name::new("3D picture"),
        PresentImage,
        ImageNode::default(),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        GlobalZIndex(i32::MIN),
        Visibility::Hidden,
        UiTargetCamera(camera),
    ));
}

/// Sets [`RenderScale`] from the pin, or in Manual from the saved 3D
/// resolution. Auto stays native until its controller is connected (T10).
fn choose_render_scale(
    store: Option<Res<SettingsStore>>,
    pin: Option<Res<RenderScalePin>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut scale: ResMut<RenderScale>,
) {
    let mode = store
        .as_ref()
        .map_or(QualityMode::Auto, |s| s.0.quality_mode);
    let want = match (pin, mode) {
        (Some(pin), _) => pin.0,
        (None, QualityMode::Manual) => {
            let Ok(window) = window.single() else {
                return;
            };
            let short = window.physical_width().min(window.physical_height());
            let choice = store.map_or(Resolution3d::Native, |s| s.0.resolution_3d);
            manual_scale(short, choice)
        }
        (None, QualityMode::Auto) => 1.0,
    };
    if scale.0 != want {
        scale.0 = want;
    }
}

/// The 3D camera, not the present one.
type MainCamera = (With<Camera3d>, Without<PresentCamera>);

/// Points the 3D camera at the window or at a scaled image, and the UI at
/// the camera that reaches the window (see the module notes).
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input"
)]
fn sync_render_scale(
    scale: Res<RenderScale>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut target: ResMut<ScaledTarget>,
    mut images: ResMut<Assets<Image>>,
    mut main: Query<(Entity, &mut RenderTarget), MainCamera>,
    mut present: Query<(Entity, &mut Camera), With<PresentCamera>>,
    mut picture: Query<(&mut ImageNode, &mut Visibility), With<PresentImage>>,
    gizmos: Option<ResMut<GizmoConfigStore>>,
    mut commands: Commands,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let native = (window.physical_width(), window.physical_height());
    if native.0 == 0 || native.1 == 0 {
        return;
    }
    let size = scaled_size(native, scale.0);
    let want = (size != native).then(|| UVec2::new(size.0, size.1));
    let key = Some((native.0, native.1, window.scale_factor().to_bits()));
    if want == target.size && key == target.window {
        return;
    }
    let (Ok((main, mut main_target)), Ok((present, mut present_camera))) =
        (main.single_mut(), present.single_mut())
    else {
        return;
    };
    let reuse = want.is_some() && want == target.size;
    if !reuse && let Some(old) = target.image.take() {
        images.remove(&old);
    }
    target.size = want;
    target.window = key;
    // Gizmo widths are picture pixels: thinner in a smaller picture, the
    // same width once stretched over the window.
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: pixel counts well within f32"
    )]
    let ratio = want.map_or(1.0, |s| s.x as f32 / native.0 as f32);
    if let Some(mut store) = gizmos
        && let Some((config, _)) = store.get_config_mut::<DefaultGizmoConfigGroup>()
    {
        config.line.width = LINE_MIN_PX * ratio;
    }
    let Some(size) = want else {
        *main_target = RenderTarget::Window(bevy::window::WindowRef::Primary);
        present_camera.is_active = false;
        commands.entity(present).remove::<IsDefaultUiCamera>();
        commands.entity(main).insert(IsDefaultUiCamera);
        if let Ok((_, mut visibility)) = picture.single_mut() {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    let handle = match target.image.clone() {
        Some(handle) => handle,
        None => {
            let mut image =
                Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None);
            // The GPU holds the picture; no CPU copy.
            image.asset_usage = RenderAssetUsages::RENDER_WORLD;
            let handle = images.add(image);
            target.image = Some(handle.clone());
            handle
        }
    };
    // Logical size = physical / scale factor: the window's logical width.
    #[expect(
        clippy::cast_precision_loss,
        reason = "E-CAST: a pixel count well within f32"
    )]
    let scale_factor = size.x as f32 / window.width().max(1.0);
    *main_target = RenderTarget::Image(ImageRenderTarget {
        handle: handle.clone(),
        scale_factor,
    });
    present_camera.is_active = true;
    commands.entity(main).remove::<IsDefaultUiCamera>();
    commands.entity(present).insert(IsDefaultUiCamera);
    if let Ok((mut node, mut visibility)) = picture.single_mut() {
        node.image = handle;
        *visibility = Visibility::Inherited;
    }
}

/// The internal 3D resolution: [`RenderScale`], the present camera and the
/// switch between the two targets.
pub(crate) struct RenderScalePlugin;

impl Plugin for RenderScalePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderScale>()
            .init_resource::<ScaledTarget>()
            .add_systems(Startup, spawn_present)
            .add_systems(
                PostUpdate,
                (choose_render_scale, sync_render_scale)
                    .chain()
                    .before(CameraUpdateSystems),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::camera::camera_system;
    use bevy::render::texture::ManualTextureViews;
    use bevy::window::WindowResolution;

    /// A headless app with a 1920 x 1080 primary window, Bevy's camera
    /// system, one 3D camera looking down -Z and the render scale plugin.
    fn scaled_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            TransformPlugin,
            WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            },
        ));
        app.init_asset::<Image>();
        app.init_asset::<Mesh>();
        app.init_resource::<ManualTextureViews>();
        app.add_systems(PostUpdate, camera_system.in_set(CameraUpdateSystems));
        app.add_plugins(RenderScalePlugin);
        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0).looking_to(Vec3::NEG_Z, Vec3::Y),
        ));
        app.update();
        app
    }

    /// Where the camera puts `point` on the window, in logical pixels.
    fn on_screen(app: &mut App, point: Vec3) -> Vec2 {
        let (camera, transform) = app
            .world_mut()
            .query_filtered::<(&Camera, &GlobalTransform), With<Camera3d>>()
            .single(app.world())
            .expect("one camera");
        camera.world_to_viewport(transform, point).expect("in view")
    }

    fn set_scale(app: &mut App, scale: f64) {
        app.insert_resource(RenderScalePin(scale));
        app.update();
        app.update();
    }

    #[test]
    fn a_marker_projects_to_the_same_window_point_at_every_scale() {
        // Picking and labels compare `world_to_viewport` with the cursor in
        // window logical pixels: the image target's scale factor keeps that
        // coordinate the same at 100 % and 70 % (#1022 T6).
        let mut app = scaled_app();
        let points = [
            Vec3::new(3.0, 1.0, -10.0),
            Vec3::new(-6.0, -3.0, -8.0),
            Vec3::new(0.0, 0.0, -50.0),
        ];
        let native: Vec<Vec2> = points.iter().map(|p| on_screen(&mut app, *p)).collect();
        assert!(app.world().resource::<ScaledTarget>().size.is_none());
        set_scale(&mut app, 0.7);
        assert_eq!(
            app.world().resource::<ScaledTarget>().size,
            Some(UVec2::new(1344, 756))
        );
        for (point, want) in points.iter().zip(&native) {
            let got = on_screen(&mut app, *point);
            assert!(got.distance(*want) < 1.0, "{point}: {got} vs {want}");
        }
        // The UI moved to the present camera, which is now drawing.
        let present = app
            .world_mut()
            .query_filtered::<(&Camera, Has<IsDefaultUiCamera>), With<PresentCamera>>()
            .single(app.world())
            .map(|(c, ui)| (c.is_active, ui))
            .expect("present camera");
        assert_eq!(present, (true, true));
        set_scale(&mut app, 1.0);
        assert!(app.world().resource::<ScaledTarget>().size.is_none());
        for (point, want) in points.iter().zip(&native) {
            assert!(on_screen(&mut app, *point).distance(*want) < 1e-3);
        }
    }

    /// Times the settings store changed after it was inserted.
    #[derive(Resource, Default)]
    struct StoreChanges(u32);

    fn count_store_changes(store: Res<SettingsStore>, mut n: ResMut<StoreChanges>) {
        if store.is_changed() && !store.is_added() {
            n.0 += 1;
        }
    }

    #[test]
    fn effective_quality_changes_never_touch_the_saved_settings() {
        // Spec v4 point 10 (#1022 T4, tested here with the first effective
        // control): the effective scale moves while the saved settings, which
        // the app writes to the file only when they change, stay untouched.
        let mut app = scaled_app();
        app.insert_resource(SettingsStore(universe_core::settings::Settings {
            quality_mode: QualityMode::Manual,
            resolution_3d: Resolution3d::Short720,
            ..Default::default()
        }));
        app.init_resource::<StoreChanges>();
        app.add_systems(Last, count_store_changes);
        app.update();
        assert!((app.world().resource::<RenderScale>().0 - 720.0 / 1080.0).abs() < 1e-12);
        for scale in [0.7, 0.9, 1.0, 0.8] {
            set_scale(&mut app, scale);
        }
        assert_eq!(app.world().resource::<StoreChanges>().0, 0);
        let saved = &app.world().resource::<SettingsStore>().0;
        assert_eq!(saved.resolution_3d, Resolution3d::Short720);
    }

    #[test]
    fn a_resize_loop_keeps_one_picture() {
        // Old pictures are released: across repeated resizes and scale
        // changes at most one scaled image exists.
        let mut app = scaled_app();
        set_scale(&mut app, 0.7);
        for i in 0..20u32 {
            let mut window = app
                .world_mut()
                .query::<&mut Window>()
                .single_mut(app.world_mut())
                .expect("window");
            window
                .resolution
                .set_physical_resolution(1600 + 16 * (i % 5), 900 + 9 * (i % 5));
            set_scale(
                &mut app,
                if i % 3 == 0 {
                    1.0
                } else {
                    0.7 + 0.1 * f64::from(i % 2)
                },
            );
            let images = app.world().resource::<Assets<Image>>().len();
            assert!(images <= 1, "{images} pictures after resize {i}");
        }
    }

    #[test]
    fn manual_presets_never_exceed_native() {
        // Q25: on 1600 x 900, 720 is 80 %; 900 and 1080 are Native.
        assert!((manual_scale(900, Resolution3d::Short720) - 0.8).abs() < 1e-12);
        assert_eq!(manual_scale(900, Resolution3d::Short900), 1.0);
        assert_eq!(manual_scale(900, Resolution3d::Short1080), 1.0);
        assert_eq!(manual_scale(900, Resolution3d::Native), 1.0);
        assert!((manual_scale(1080, Resolution3d::Short900) - 900.0 / 1080.0).abs() < 1e-12);
        assert_eq!(manual_scale(600, Resolution3d::Short720), 1.0);
    }
}
