use serde_json::{Map as JsonMap, Value as JsonValue};

use crate::component::animation::Animator;
use crate::component::spatial::{Aabb, Position, Velocity};
use crate::ecs::entity::Entity;
use crate::ecs::world::World;
use crate::event::payload::SnapshotEvent;
use crate::tag::set::TagSet;

/// Build a JSONL-friendly snapshot stream for the current world state.
#[must_use]
pub fn snapshot_world(world: &World, tick: u64) -> Vec<SnapshotEvent> {
    let mut events = Vec::new();
    events.push(SnapshotEvent::BeginSnapshot { tick });

    for entity in world.entities() {
        events.push(SnapshotEvent::Entity {
            entity,
            components: snapshot_components(world, entity),
        });
    }

    events.push(SnapshotEvent::EndSnapshot);
    events
}

fn snapshot_components(world: &World, entity: Entity) -> JsonValue {
    let mut components = JsonMap::new();

    if let Some(tags) = world.get::<TagSet>(entity) {
        let tags = tags
            .iter()
            .filter_map(|tag| world.tag_registry().name(tag))
            .map(JsonValue::from)
            .collect::<Vec<_>>();
        components.insert("tags".into(), JsonValue::Array(tags));
    }
    if let Some(position) = world.get::<Position>(entity) {
        components.insert(
            "position".into(),
            serde_json::to_value(position).expect("Position should serialize"),
        );
    }
    if let Some(velocity) = world.get::<Velocity>(entity) {
        components.insert(
            "velocity".into(),
            serde_json::to_value(velocity).expect("Velocity should serialize"),
        );
    }
    if let Some(aabb) = world.get::<Aabb>(entity) {
        components.insert(
            "aabb".into(),
            serde_json::to_value(aabb).expect("Aabb should serialize"),
        );
    }
    if let Some(animator) = world.get::<Animator>(entity) {
        components.insert(
            "animator".into(),
            serde_json::to_value(animator).expect("Animator should serialize"),
        );
    }

    JsonValue::Object(components)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::Value as JsonValue;

    use crate::event::payload::SnapshotEvent;
    use crate::runtime::bootstrap::bootstrap;

    use super::*;

    #[test]
    fn snapshot_world_serializes_ball_collision_entities() {
        let state =
            bootstrap(Path::new("example/ball_collision")).expect("example should bootstrap");

        let events = snapshot_world(&state.world, 0);

        assert!(matches!(
            events.first(),
            Some(SnapshotEvent::BeginSnapshot { tick: 0 })
        ));
        assert!(matches!(events.last(), Some(SnapshotEvent::EndSnapshot)));

        let entity_components = events
            .iter()
            .filter_map(|event| match event {
                SnapshotEvent::Entity { components, .. } => Some(components),
                SnapshotEvent::BeginSnapshot { .. } | SnapshotEvent::EndSnapshot => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(entity_components.len(), 2);

        assert!(entity_components.iter().any(|components| {
            tags_include(components, "Ball")
                && components.pointer("/position/x") == Some(&JsonValue::from(0.0))
                && components.pointer("/velocity/x") == Some(&JsonValue::from(3.0))
        }));
        assert!(entity_components.iter().any(|components| {
            tags_include(components, "Wall")
                && components.pointer("/position/x") == Some(&JsonValue::from(2.0))
                && components.pointer("/aabb/half_extents/y") == Some(&JsonValue::from(1.5))
        }));
    }

    fn tags_include(components: &JsonValue, tag: &str) -> bool {
        components
            .get("tags")
            .and_then(JsonValue::as_array)
            .is_some_and(|tags| tags.iter().any(|value| value.as_str() == Some(tag)))
    }
}
