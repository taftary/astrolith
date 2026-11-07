//! Bounded actual UI text, physical layout, clipping and scroll records (#1238).
//! Node IDs are session-local; named aliases and exact text remain discoverable.

use crate::scenario_run::Snapshot;
use crate::scenario_values::{text, unavailable, value};
use bevy::prelude::*;
use bevy::ui::{CalculatedClip, OverrideClip, UiGlobalTransform};

const MAX_NODES: usize = 2048;
const MAX_TEXT_BYTES: usize = 262_144;

fn rect(snapshot: &mut Snapshot, key: &str, rect: Rect) {
    value(
        snapshot,
        key,
        format!(
            "{},{},{},{}",
            rect.min.x, rect.min.y, rect.max.x, rect.max.y
        ),
    );
    value(
        snapshot,
        &format!("{key}.units"),
        "camera-viewport-physical-pixels",
    );
}

fn geometry(
    world: &World,
    snapshot: &mut Snapshot,
    entity: Entity,
    key: &str,
    node: &ComputedNode,
    transform: &UiGlobalTransform,
) {
    let affine = transform.affine();
    let half = node.size() * 0.5;
    let points = [
        Vec2::new(-half.x, -half.y),
        Vec2::new(half.x, -half.y),
        half,
        Vec2::new(-half.x, half.y),
    ]
    .map(|p| affine.transform_point2(p));
    let bounds = points.iter().fold(
        Rect {
            min: Vec2::splat(f32::INFINITY),
            max: Vec2::splat(f32::NEG_INFINITY),
        },
        |r, p| Rect {
            min: r.min.min(*p),
            max: r.max.max(*p),
        },
    );
    rect(snapshot, &format!("{key}.bounds"), bounds);
    value(
        snapshot,
        &format!("{key}.size"),
        format!("{},{}", node.size().x, node.size().y),
    );
    value(
        snapshot,
        &format!("{key}.content_size"),
        format!("{},{}", node.content_size().x, node.content_size().y),
    );
    let clip = world.get::<CalculatedClip>(entity);
    if let Some(clip) = clip {
        rect(snapshot, &format!("{key}.clip"), clip.clip);
    } else {
        unavailable(
            snapshot,
            &format!("{key}.clip"),
            "not-applicable-no-inherited-clip",
        );
    }
    value(
        snapshot,
        &format!("{key}.center_unclipped"),
        clip.is_none_or(|c| c.clip.contains(affine.translation)),
    );
    value(
        snapshot,
        &format!("{key}.override_clip"),
        world.get::<OverrideClip>(entity).is_some(),
    );
    if let Some(scroll) = world.get::<ScrollPosition>(entity) {
        value(
            snapshot,
            &format!("{key}.scroll"),
            format!("{},{}", scroll.0.x, scroll.0.y),
        );
    }
}

fn descendant_text(world: &World, root: Entity) -> Result<String, String> {
    let mut stack = vec![root];
    let mut visited = 0usize;
    let mut result = String::new();
    while let Some(entity) = stack.pop() {
        visited += 1;
        if visited > MAX_NODES {
            return Err("UI text traversal node limit reached".into());
        }
        if world
            .get::<InheritedVisibility>(entity)
            .is_some_and(|v| !v.get())
        {
            continue;
        }
        if let Some(t) = world.get::<Text>(entity) {
            if result.len().saturating_add(t.0.len() + 1) > MAX_TEXT_BYTES {
                return Err("UI descendant text limit reached".into());
            }
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&t.0);
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter().rev());
        }
    }
    Ok(result)
}

pub(crate) fn observe(world: &mut World, snapshot: &mut Snapshot) -> Result<(), String> {
    let mut query = world.query::<(
        Entity,
        &ComputedNode,
        &UiGlobalTransform,
        &InheritedVisibility,
    )>();
    let nodes: Vec<_> = query
        .iter(world)
        .filter(|(_, node, _, visible)| visible.get() && node.size().min_element() > 0.0)
        .take(MAX_NODES + 1)
        .map(|(e, node, transform, _)| (e, *node, *transform))
        .collect();
    if nodes.len() > MAX_NODES {
        return Err("UI observation node limit 2048 reached".into());
    }
    let mut counts = std::collections::BTreeMap::new();
    for (entity, _, _) in &nodes {
        if let Some(name) = world.get::<Name>(*entity) {
            *counts
                .entry(crate::scenario_ui::target_id(name.as_str()))
                .or_insert(0usize) += 1;
        }
    }
    let mut text_bytes = 0usize;
    for (entity, node, transform) in &nodes {
        let key = format!("ui.node.{}", entity.to_bits());
        value(snapshot, &format!("{key}.entity"), entity.to_bits());
        if let Some(parent) = world.get::<ChildOf>(*entity) {
            value(
                snapshot,
                &format!("{key}.parent"),
                parent.parent().to_bits(),
            );
        }
        if let Some(name) = world.get::<Name>(*entity) {
            text(snapshot, &format!("{key}.name"), name.as_str());
            let id = crate::scenario_ui::target_id(name.as_str());
            value(
                snapshot,
                &format!("ui.{id}.name_count"),
                counts.get(&id).copied().unwrap_or(0),
            );
            if counts.get(&id) == Some(&1) {
                geometry(
                    world,
                    snapshot,
                    *entity,
                    &format!("ui.{id}"),
                    node,
                    transform,
                );
                let content = descendant_text(world, *entity)?;
                text_bytes = text_bytes.saturating_add(content.len());
                if text_bytes > MAX_TEXT_BYTES {
                    return Err("UI observation text limit 262144 bytes reached".into());
                }
                if !content.is_empty() {
                    text(snapshot, &format!("ui.{id}.text"), &content);
                }
            }
        }
        geometry(world, snapshot, *entity, &key, node, transform);
        if let Some(t) = world.get::<Text>(*entity) {
            text_bytes = text_bytes.saturating_add(t.0.len());
            if text_bytes > MAX_TEXT_BYTES {
                return Err("UI observation text limit 262144 bytes reached".into());
            }
            text(snapshot, &format!("{key}.text"), &t.0);
        }
    }
    value(
        snapshot,
        "ui.layout.source",
        "actual-computed-nodes-inherited-visibility-and-calculated-clips-not-pixel-occlusion",
    );
    value(snapshot, "ui.layout.node_count", nodes.len());
    value(snapshot, "ui.layout.text_bytes", text_bytes);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_geometry_and_actual_clip_are_retained_without_layout_writes() {
        let mut world = World::new();
        let entity = world
            .spawn((
                Name::new("read-only"),
                Text::new("Body\ttext\nline"),
                ComputedNode {
                    size: Vec2::new(200.0, 40.0),
                    content_size: Vec2::new(200.0, 400.0),
                    ..default()
                },
                UiGlobalTransform::default(),
                InheritedVisibility::VISIBLE,
                CalculatedClip {
                    clip: Rect::from_center_size(Vec2::ZERO, Vec2::splat(20.0)),
                },
                ScrollPosition(Vec2::new(0.0, 100.0)),
            ))
            .id();
        let mut snapshot = Snapshot::new();
        observe(&mut world, &mut snapshot).unwrap();
        assert_eq!(snapshot["ui.read-only.bounds"], "-100,-20,100,20");
        assert_eq!(snapshot["ui.read-only.clip"], "-10,-10,10,10");
        assert_eq!(snapshot["ui.read-only.scroll"], "0,100");
        assert_eq!(
            snapshot[&format!("ui.node.{}.text", entity.to_bits())],
            "Body\\ttext\\nline"
        );
        assert_eq!(world.get::<ScrollPosition>(entity).unwrap().0.y, 100.0);
    }

    #[test]
    fn layout_limits_refuse_partial_required_evidence() {
        let mut world = World::new();
        for _ in 0..=MAX_NODES {
            world.spawn((
                ComputedNode {
                    size: Vec2::ONE,
                    ..default()
                },
                UiGlobalTransform::default(),
                InheritedVisibility::VISIBLE,
            ));
        }
        assert!(
            observe(&mut world, &mut Snapshot::new())
                .unwrap_err()
                .contains("node limit")
        );
        let mut world = World::new();
        world.spawn((
            Text::new("x".repeat(MAX_TEXT_BYTES + 1)),
            ComputedNode {
                size: Vec2::ONE,
                ..default()
            },
            UiGlobalTransform::default(),
            InheritedVisibility::VISIBLE,
        ));
        assert!(
            observe(&mut world, &mut Snapshot::new())
                .unwrap_err()
                .contains("text limit")
        );
    }
}
