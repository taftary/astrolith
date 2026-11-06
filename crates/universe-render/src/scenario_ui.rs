//! Read-only UI hit targets from real layout, visibility and clipping (#1227).

use crate::scenario_run::Snapshot;
use bevy::prelude::*;
use bevy::ui::{ComputedUiTargetCamera, OverrideClip, UiGlobalTransform};

#[derive(Debug, Clone)]
pub(crate) struct Target {
    pub id: String,
    pub center: Vec2,
    pub interaction: Interaction,
    pub actionable: bool,
}

fn unclipped(world: &World, mut entity: Entity, point: Vec2) -> bool {
    // Mirror the engine's ancestor clipping test, with a defensive depth cap.
    for _ in 0..128 {
        if world.get::<OverrideClip>(entity).is_some() {
            return true;
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            return true;
        };
        entity = parent.parent();
        if let (Some(node), Some(computed), Some(transform)) = (
            world.get::<Node>(entity),
            world.get::<ComputedNode>(entity),
            world.get::<UiGlobalTransform>(entity),
        ) {
            let Some(inverse) = transform.try_inverse() else {
                return false;
            };
            if !computed
                .resolve_clip_rect(node.overflow, node.overflow_clip_margin)
                .contains(inverse.transform_point2(point))
            {
                return false;
            }
        }
    }
    false
}

pub(crate) fn targets(world: &mut World) -> Vec<Target> {
    let mut windows = world.query_filtered::<&Window, With<bevy::window::PrimaryWindow>>();
    let Ok(window) = windows.single(world) else {
        return Vec::new();
    };
    let (width, height, scale) = (window.width(), window.height(), window.scale_factor());
    let mut query = world.query::<(
        Entity,
        &Name,
        &ComputedNode,
        &UiGlobalTransform,
        &InheritedVisibility,
        &ComputedUiTargetCamera,
        Option<&Interaction>,
        Option<&Button>,
    )>();
    query
        .iter(world)
        .filter_map(
            |(entity, name, node, transform, visibility, camera, interaction, button)| {
                if !visibility.get() || node.size().min_element() <= 0.0 {
                    return None;
                }
                let point = transform.affine().translation;
                if !unclipped(world, entity, point) {
                    return None;
                }
                let camera = world.get::<Camera>(camera.get()?)?;
                let viewport = camera.physical_viewport_rect()?;
                let physical = point + viewport.min.as_vec2();
                // Scenario mode owns one primary window. Camera target geometry still
                // determines the point; only convert physical pixels at the boundary.
                let center = physical / scale;
                if center.x < 0.0 || center.y < 0.0 || center.x >= width || center.y >= height {
                    return None;
                }
                Some(Target {
                    id: name.as_str().to_owned(),
                    center,
                    interaction: interaction.copied().unwrap_or(Interaction::None),
                    actionable: button.is_some() && interaction.is_some(),
                })
            },
        )
        .collect()
}

pub(crate) fn click_point(world: &mut World, id: &str) -> Result<Vec2, String> {
    let found: Vec<_> = targets(world)
        .into_iter()
        .filter(|t| t.id == id && t.actionable)
        .collect();
    match found.as_slice() {
        [target] => Ok(target.center),
        [] => Err(format!("missing or invisible button {id}")),
        _ => Err(format!("ambiguous button {id}")),
    }
}

pub(crate) fn observe(world: &mut World, snapshot: &mut Snapshot) {
    let targets = targets(world);
    let mut counts = std::collections::BTreeMap::<&str, usize>::new();
    for t in &targets {
        *counts.entry(&t.id).or_default() += 1;
    }
    for target in &targets {
        if counts.get(target.id.as_str()) != Some(&1) {
            continue;
        }
        snapshot.insert(format!("ui.{}.visible", target.id), "true".into());
        snapshot.insert(
            format!("ui.{}.actionable", target.id),
            target.actionable.to_string(),
        );
        snapshot.insert(
            format!("ui.{}.interaction", target.id),
            format!("{:?}", target.interaction),
        );
        snapshot.insert(
            format!("ui.{}.center", target.id),
            format!("{},{}", target.center.x, target.center.y),
        );
    }
}
