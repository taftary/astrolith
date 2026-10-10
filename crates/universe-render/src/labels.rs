//! Portal labels: pooled text nodes reading "kind · name" (#446).
//!
//! Labels appear only on portals that are large enough on screen, selected,
//! or hovered ([`universe_core::pick::should_label`]); populations never earn
//! one, so the black-and-white test (ring plus label on portals only) holds.
//! A fixed pool of [`LABEL_CAP`] text entities is spawned once and reused:
//! no spawning per frame, and the entity count never exceeds the pool.
//! Names resolve through [`universe_core::names`] when the open cell changes
//! and are cached, never per frame (`E-HOT-NOALLOC`).

use crate::Universe;
use crate::input::Navigation;
use crate::style::to_vec3;
use crate::tokens::{LABEL_CAP, LABEL_HALO, LABEL_MIN_PX, LABEL_OFFSET_PX, LABEL_TINT, PX_RAD};
use bevy::camera::Camera;
use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::ui::{Node, PositionType, Val, widget::Text};
use universe_core::coords::Level;
use universe_core::r#gen::MarkerKind;
use universe_core::names::{label_text, marker_kind_name, marker_name};
use universe_core::nav::{MarkerIndex, drawn_radius};

/// Marks entities owned by [`sync_labels`].
#[derive(Component)]
pub(crate) struct PortalLabel {
    /// Pool slot, `0..LABEL_CAP`.
    slot: usize,
}

/// Cached names for the open cell's portals, rebuilt on cell change only.
#[derive(Resource, Default)]
pub(crate) struct LabelCache {
    /// Open-cell seed the entries were built for, if any.
    seed: Option<u64>,
    /// One entry per portal marker: marker plus its `"kind · name"` line.
    entries: Vec<LabelEntry>,
}

/// One cached portal name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LabelEntry {
    /// Marker index in the open cell.
    marker: u32,
    /// Display line (`"star · Alpha Centauri"`, kind alone when nameless).
    text: String,
}

/// Label text for one marker, or `None` for populations.
///
/// Pure: `"kind · name"`, the kind alone when nameless. Called on cache
/// rebuild, never per frame.
#[must_use]
pub(crate) fn portal_label_text(
    root: u64,
    chain: &[u32],
    level: Level,
    marker: u32,
    point: &universe_core::r#gen::Point,
) -> Option<String> {
    if point.kind != MarkerKind::Portal {
        return None;
    }
    let kind = marker_kind_name(level, point);
    let name = marker_name(root, chain, level, marker, point);
    Some(label_text(kind, &name))
}

/// Largest-first capped pick of labelled markers (Spec v1 AC7).
///
/// `diameters` holds `(marker, on-screen diameter in pixels)` for portals
/// already passing [`should_label`]; returns at most `cap` markers,
/// largest first. Pure.
#[must_use]
pub(crate) fn top_labels(diameters: &[(u32, f32)], cap: usize) -> Vec<u32> {
    let mut ranked: Vec<(u32, f32)> = diameters.to_vec();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked
        .into_iter()
        .take(cap)
        .map(|(marker, _)| marker)
        .collect()
}

/// On-screen diameter in pixels of a drawn world radius at `distance`.
fn diameter_px(world_radius: f32, distance: f64) -> f32 {
    if !world_radius.is_finite() || world_radius <= 0.0 {
        return 0.0;
    }
    let angle = f64::from(world_radius) / distance.max(1e-6);
    let pixels = angle.atan() / PX_RAD * 2.0;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: pixel diameter narrowed for layout, intended"
    )]
    let narrowed = pixels as f32;
    if narrowed.is_finite() { narrowed } else { 0.0 }
}

/// Spawns the fixed label pool (one text node per slot, hidden).
pub(crate) fn spawn_labels(mut commands: Commands) {
    for slot in 0..LABEL_CAP {
        commands.spawn((
            PortalLabel { slot },
            Name::new("portal-label"),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            Text::new(String::new()),
            TextColor(LABEL_TINT),
            BackgroundColor(Color::NONE),
            Visibility::Hidden,
        ));
    }
}

/// Syncs portal labels from the open cell.
///
/// Rebuilds the name cache when the open cell changes; per frame it projects
/// cached portals, keeps the largest-first [`LABEL_CAP`] passing
/// [`should_label`], and rewrites only changed text. Missing camera or window
/// is a no-op return, never a panic. Allocates only on cell change and on
/// first text write per slot.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per engine input; the label pool, cache, camera, and navigation share one frame"
)]
pub(crate) fn sync_labels(
    universe: Res<Universe>,
    nav: Res<Navigation>,
    mut cache: ResMut<LabelCache>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut labels: Query<(
        &PortalLabel,
        &mut Text,
        &mut Node,
        &mut Visibility,
        &mut BackgroundColor,
    )>,
) {
    let seed = universe.open_seed();
    if cache.seed != Some(seed) {
        cache.entries.clear();
        let chain = universe.path.indices();
        let level = universe.level();
        for (index, point) in universe.open.points.iter().enumerate() {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "E-CAST: cached marker index into a budgeted cell, always fits u32"
            )]
            let marker = index as u32;
            if let Some(text) = portal_label_text(universe.root.0, &chain, level, marker, point) {
                cache.entries.push(LabelEntry { marker, text });
            }
        }
        cache.seed = Some(seed);
    }
    let Some((camera, camera_transform)) = cameras.iter().next() else {
        return;
    };
    let marker_radius = universe.marker_radius();
    // Largest-first top set: one scratch vector per frame is accepted here
    // (UI text assignment dominates anyway; gizmo paths stay allocation-free).
    // The size threshold aliases the core label rule through tokens (#446).
    let mut candidates: Vec<(u32, f32, Vec2)> = Vec::new();
    for entry in &cache.entries {
        let Some(world) = universe.marker(MarkerIndex(entry.marker)) else {
            continue;
        };
        let distance = (to_vec3(universe.path.offset()) - to_vec3(world)).length();
        if distance < 1e-6 {
            continue;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: render-domain narrowing of a radius, intended"
        )]
        let drawn = drawn_radius(marker_radius, f64::from(distance)) as f32;
        let wide = diameter_px(drawn, f64::from(distance));
        let selected = nav.target == Some(entry.marker);
        let hovered = nav.hover == Some(entry.marker);
        if !(wide >= LABEL_MIN_PX || selected || hovered) {
            continue;
        }
        // Portal screen anchor: the body position (labels name the marker;
        // the ring marks the child offset).
        let body = universe
            .open
            .points
            .get(entry.marker as usize)
            .map_or(world, |point| point.position);
        let Ok(screen) = camera.world_to_viewport(camera_transform, to_vec3(body)) else {
            continue;
        };
        if !screen.x.is_finite() || !screen.y.is_finite() {
            continue;
        }
        candidates.push((entry.marker, wide, screen));
    }
    let diameters: Vec<(u32, f32)> = candidates
        .iter()
        .map(|&(marker, wide, _)| (marker, wide))
        .collect();
    let top = top_labels(&diameters, LABEL_CAP);
    let mut shown: Vec<(u32, Vec2)> = Vec::with_capacity(top.len());
    for marker in &top {
        if let Some(&(_, _, screen)) = candidates.iter().find(|&&(held, _, _)| held == *marker) {
            shown.push((*marker, screen));
        }
    }
    for (label, mut text, mut node, mut visibility, mut halo) in &mut labels {
        if let Some(&(marker, screen)) = shown.get(label.slot) {
            let entry = cache.entries.iter().find(|entry| entry.marker == marker);
            let Some(entry) = entry else {
                *visibility = Visibility::Hidden;
                continue;
            };
            if text.0 != entry.text {
                text.0.clone_from(&entry.text);
            }
            node.left = Val::Px(screen.x + LABEL_OFFSET_PX);
            node.top = Val::Px(screen.y - LABEL_OFFSET_PX);
            *visibility = Visibility::Visible;
            let selected = nav.target == Some(marker);
            halo.0 = if selected { LABEL_HALO } else { Color::NONE };
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use universe_core::r#gen::Point;

    #[test]
    fn labels_belong_to_qualifying_portals_and_read_kind_name() {
        use universe_core::coords::Level;
        let level = Level::new(6).expect("L6");
        let portal = Point::bare([0.0; 3], 0.01, MarkerKind::Portal);
        let text = portal_label_text(42, &[], level, 0, &portal).expect("portal label");
        assert!(text.contains('·'), "label reads kind · name: {text}");
        let population = Point::bare([0.0; 3], 0.01, MarkerKind::Population);
        assert_eq!(portal_label_text(42, &[], level, 0, &population), None);
    }

    #[test]
    fn largest_first_cap_picks_the_biggest_portals() {
        let diameters = [(1u32, 10.0f32), (2, 64.0), (3, 32.0)];
        assert_eq!(top_labels(&diameters, 2), vec![2, 3]);
        assert_eq!(top_labels(&[], LABEL_CAP), Vec::<u32>::new());
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: test ranks below 40, exactly representable"
        )]
        let many: Vec<(u32, f32)> = (0..40u32).map(|m| (m, m as f32)).collect();
        assert_eq!(top_labels(&many, LABEL_CAP).len(), LABEL_CAP);
    }

    #[test]
    fn pool_never_exceeds_its_cap() {
        use bevy::asset::AssetPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_resource::<LabelCache>();
        app.add_systems(Startup, spawn_labels);
        app.update();
        app.update();
        let count = app
            .world_mut()
            .query_filtered::<Entity, With<PortalLabel>>()
            .iter(app.world())
            .count();
        assert_eq!(count, LABEL_CAP, "the pool spawns exactly once");
    }
}
