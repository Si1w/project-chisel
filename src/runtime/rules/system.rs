use serde_json::{Map as JsonMap, Value as JsonValue};

use crate::component::animation::{Animator, Clip};
use crate::component::spatial::{Position, Velocity};
use crate::ecs::entity::Entity;
use crate::ecs::schedule::TickContext;
use crate::ecs::world::World;
use crate::event::bus::Bus;
use crate::event::payload::{DomainEvent, MarkerEvent, PresentationCommand};
use crate::event::queue::EventQueue;
use crate::math::vec2::Vec2;
use crate::runtime::rules::model::{Action, EntityMatch, ParamId, ReverseAxis, Rule, RuleSet};
use crate::runtime::template::{TemplateStore, spawn_template};
use crate::tag::set::TagSet;

/// Processes events from the `EventQueue` between ticks. **Not** a
/// `System` — runs outside `Schedule::tick` so it can publish drained
/// events to `Bus` (which systems aren't allowed to touch) and so
/// cascading rule actions can push more events back into the queue
/// without violating invariant 1.
pub struct RuleProcessor {
    rules: RuleSet,
}

impl RuleProcessor {
    #[must_use]
    pub fn new(rules: RuleSet) -> Self {
        Self { rules }
    }

    #[must_use]
    pub fn rules(&self) -> &RuleSet {
        &self.rules
    }

    /// Drains the `EventQueue` resource. Domain events are forwarded to
    /// `bus`, matched against loaded rules, and used to run the
    /// currently-supported rule actions (`spawn`, `set_velocity`,
    /// `reverse_velocity`, `emit`, `despawn`, and `play_animation`).
    /// Marker events are forwarded to the marker bus, then matched
    /// against rules through their marker type and payload.
    ///
    /// At most `max_iterations` domain events are processed per call.
    /// Remaining domain events stay in the queue for the next tick; a
    /// `rule_iteration_limit_reached` diagnostic is published directly
    /// to the domain bus when the limit is reached with queued domain
    /// work still pending.
    ///
    /// # Panics
    ///
    /// Panics if the world's `EventQueue` resource has been removed.
    pub fn process(&self, world: &mut World, bus: &Bus, _ctx: &TickContext, max_iterations: u32) {
        let mut processed = 0;
        while processed < max_iterations {
            let Some(event) = world
                .resource_mut::<EventQueue>()
                .expect("EventQueue is inserted by World::new")
                .next_domain()
            else {
                break;
            };
            processed += 1;
            let _ = bus.domain.emit(event.clone());
            self.process_rule_event(world, bus, &event);
        }
        let remaining_domain_len = world
            .resource::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .domain_len();
        if remaining_domain_len > 0 {
            let remaining_domain_len = u64::try_from(remaining_domain_len).unwrap_or(u64::MAX);
            let _ = bus.domain.emit(DomainEvent::rule_iteration_limit_reached(
                max_iterations,
                remaining_domain_len,
            ));
        }

        while let Some(event) = world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .next_marker()
        {
            let _ = bus.marker.emit(event.clone());
            let rule_event = rule_event_from_marker(&event);
            self.process_rule_event(world, bus, &rule_event);
        }
    }

    fn process_rule_event(&self, world: &mut World, bus: &Bus, event: &DomainEvent) {
        for rule in self.rules.rules_for(&event.name) {
            let Some(bindings) = bind_event(world, rule, event) else {
                continue;
            };
            run_actions(world, bus, rule, &bindings, event);
        }
    }
}

fn rule_event_from_marker(event: &MarkerEvent) -> DomainEvent {
    match event {
        MarkerEvent::Reached { entity, marker } => {
            let mut payload = JsonMap::new();
            payload.insert("entity".into(), entity_value(*entity));
            payload.insert("marker".into(), JsonValue::String(marker.clone()));
            DomainEvent::custom("reached", payload)
        }
    }
}

fn entity_value(entity: Entity) -> JsonValue {
    let mut value = JsonMap::new();
    value.insert("index".into(), JsonValue::from(entity.index));
    value.insert("generation".into(), JsonValue::from(entity.generation));
    JsonValue::Object(value)
}

struct RuleBindings {
    params: Vec<String>,
    entities: Vec<Entity>,
}

impl RuleBindings {
    fn entity(&self, param: ParamId) -> Option<Entity> {
        self.entities.get(param.0 as usize).copied()
    }

    fn entity_by_name(&self, name: &str) -> Option<Entity> {
        let index = self.params.iter().position(|param| param == name)?;
        self.entities.get(index).copied()
    }
}

fn bind_event(world: &World, rule: &Rule, event: &DomainEvent) -> Option<RuleBindings> {
    let mut params = Vec::with_capacity(rule.match_spec.params.len());
    let mut entities = Vec::with_capacity(rule.match_spec.params.len());

    for (index, param) in rule.match_spec.params.iter().enumerate() {
        let entity = parse_entity(event.payload.get(param)?)?;
        if !world.is_alive(entity) {
            return None;
        }
        if let Some(filter) = rule.match_spec.filters.get(index)
            && !entity_matches(world, entity, filter)
        {
            return None;
        }
        params.push(param.clone());
        entities.push(entity);
    }

    Some(RuleBindings { params, entities })
}

fn entity_matches(world: &World, entity: Entity, filter: &EntityMatch) -> bool {
    let Some(tags) = world.get::<TagSet>(entity) else {
        return filter.required.is_empty();
    };

    filter.required.iter().all(|id| tags.contains(*id))
        && filter.forbidden.iter().all(|id| !tags.contains(*id))
}

fn parse_entity(value: &JsonValue) -> Option<Entity> {
    serde_json::from_value(value.clone()).ok()
}

fn run_actions(
    world: &mut World,
    bus: &Bus,
    rule: &Rule,
    bindings: &RuleBindings,
    event: &DomainEvent,
) {
    for (action_index, action) in rule.actions.iter().enumerate() {
        if let Err(reason) = run_action(world, bus, action, bindings, event) {
            emit_rule_action_failed(world, rule, action_index, &reason);
            break;
        }
    }
}

fn run_action(
    world: &mut World,
    bus: &Bus,
    action: &Action,
    bindings: &RuleBindings,
    event: &DomainEvent,
) -> Result<(), String> {
    match action {
        Action::SetVelocity { target, velocity } => {
            let entity = bound_entity(bindings, *target)?;
            let current = world
                .get_mut::<Velocity>(entity)
                .ok_or_else(|| format!("velocity component missing on entity {entity:?}"))?;
            current.0 = *velocity;
            Ok(())
        }
        Action::ReverseVelocity { target, axis } => {
            let entity = bound_entity(bindings, *target)?;
            let velocity = world
                .get_mut::<Velocity>(entity)
                .ok_or_else(|| format!("velocity component missing on entity {entity:?}"))?;
            reverse_velocity(velocity, *axis, event)
        }
        Action::Emit { event, payload } => {
            let payload = substitute_payload(payload, bindings)?;
            let JsonValue::Object(payload) = payload else {
                return Err("emit payload must be an object".into());
            };
            world
                .resource_mut::<EventQueue>()
                .expect("EventQueue is inserted by World::new")
                .emit_domain(DomainEvent::custom(event.clone(), payload));
            Ok(())
        }
        Action::Despawn { target } => {
            let entity = bound_entity(bindings, *target)?;
            world.despawn(entity).map_err(|error| error.to_string())
        }
        Action::Spawn { template, position } => {
            let mut instance = world
                .resource::<TemplateStore>()
                .ok_or("template store missing")?
                .get(template)
                .ok_or_else(|| format!("unknown template {template:?}"))?
                .clone();
            instance.position = Some(Position(*position));
            spawn_template(world, instance)
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
        Action::PlayAnimation {
            target,
            clip,
            priority,
        } => {
            let entity = bound_entity(bindings, *target)?;
            world
                .insert(
                    entity,
                    Animator {
                        clip: Clip { name: clip.clone() },
                        elapsed: 0.0,
                        speed: 1.0,
                        looping: false,
                    },
                )
                .map_err(|error| error.to_string())?;
            let _ = bus.presentation.emit(PresentationCommand::PlayAnimation {
                entity,
                clip: clip.clone(),
                priority: *priority,
            });
            Ok(())
        }
    }
}

fn bound_entity(bindings: &RuleBindings, target: ParamId) -> Result<Entity, String> {
    bindings
        .entity(target)
        .ok_or_else(|| format!("param {} is not bound", target.0))
}

fn substitute_payload(value: &JsonValue, bindings: &RuleBindings) -> Result<JsonValue, String> {
    match value {
        JsonValue::String(text) => {
            if let Some(param) = text.strip_prefix('$') {
                let entity = bindings
                    .entity_by_name(param)
                    .ok_or_else(|| format!("unknown emit binding {text:?}"))?;
                serde_json::to_value(entity).map_err(|error| error.to_string())
            } else {
                Ok(value.clone())
            }
        }
        JsonValue::Array(items) => {
            let mut substituted = Vec::with_capacity(items.len());
            for item in items {
                substituted.push(substitute_payload(item, bindings)?);
            }
            Ok(JsonValue::Array(substituted))
        }
        JsonValue::Object(fields) => {
            let mut substituted = serde_json::Map::new();
            for (key, item) in fields {
                substituted.insert(key.clone(), substitute_payload(item, bindings)?);
            }
            Ok(JsonValue::Object(substituted))
        }
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => Ok(value.clone()),
    }
}

fn emit_rule_action_failed(world: &mut World, rule: &Rule, action_index: usize, reason: &str) {
    let action_index = u32::try_from(action_index).unwrap_or(u32::MAX);
    world
        .resource_mut::<EventQueue>()
        .expect("EventQueue is inserted by World::new")
        .emit_domain(DomainEvent::rule_action_failed(
            &rule.id.0,
            action_index,
            reason,
        ));
}

fn reverse_velocity(
    velocity: &mut Velocity,
    axis: ReverseAxis,
    event: &DomainEvent,
) -> Result<(), String> {
    match axis {
        ReverseAxis::X => velocity.x = -velocity.x,
        ReverseAxis::Y => velocity.y = -velocity.y,
        ReverseAxis::Both => {
            velocity.x = -velocity.x;
            velocity.y = -velocity.y;
        }
        ReverseAxis::FromNormal => {
            let normal = event_normal(event).ok_or("normal missing or malformed")?;
            if normal.x != 0.0 {
                velocity.x = -velocity.x;
            }
            if normal.y != 0.0 {
                velocity.y = -velocity.y;
            }
        }
    }
    Ok(())
}

fn event_normal(event: &DomainEvent) -> Option<Vec2> {
    serde_json::from_value(event.payload.get("normal")?.clone()).ok()
}

#[cfg(test)]
mod tests {
    use serde_json::{Map as JsonMap, json};

    use crate::component::animation::Animator;
    use crate::component::spatial::Aabb;
    use crate::event::payload::{DomainEvent, MarkerEvent, PresentationCommand};
    use crate::math::vec2::Vec2;
    use crate::runtime::rules::model::{
        Action, EntityMatch, MatchSpec, ParamId, ReverseAxis, Rule, RuleId,
    };
    use crate::runtime::template::{Template, TemplateStore};

    use super::*;

    #[tokio::test]
    async fn process_drains_and_forwards_domain_events() {
        let mut world = World::new();
        world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .emit_domain(DomainEvent::tick());
        let (bus, _endpoints) = Bus::new(4, 4);
        let mut domain_rx = bus.domain.subscribe();
        let processor = RuleProcessor::new(RuleSet::new());

        processor.process(&mut world, &bus, &TickContext { tick: 1, dt: 0.0 }, 16);

        assert!(
            world
                .resource::<EventQueue>()
                .expect("EventQueue is inserted by World::new")
                .is_empty()
        );
        let event = domain_rx.recv().await.expect("domain event should arrive");
        assert_eq!(event.name, "tick");
    }

    #[tokio::test]
    async fn process_drains_and_forwards_marker_events() {
        let mut world = World::new();
        let entity = world.spawn().finish();
        world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .emit_marker(MarkerEvent::Reached {
                entity,
                marker: "landed".into(),
            });
        let (bus, _endpoints) = Bus::new(4, 4);
        let mut marker_rx = bus.marker.subscribe();
        let processor = RuleProcessor::new(RuleSet::new());

        processor.process(&mut world, &bus, &TickContext { tick: 1, dt: 0.0 }, 16);

        assert!(
            world
                .resource::<EventQueue>()
                .expect("EventQueue is inserted by World::new")
                .is_empty()
        );
        let event = marker_rx.recv().await.expect("marker event should arrive");
        match event {
            MarkerEvent::Reached {
                entity: actual,
                marker,
            } => {
                assert_eq!(actual, entity);
                assert_eq!(marker, "landed");
            }
        }
    }

    #[tokio::test]
    async fn process_runs_matching_marker_rule() {
        let mut world = World::new();
        let entity = world.spawn().finish();
        world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .emit_marker(MarkerEvent::Reached {
                entity,
                marker: "landed".into(),
            });

        let mut rules = RuleSet::new();
        rules.add(Rule {
            id: RuleId("marker-landed".into()),
            event_name: "reached".into(),
            match_spec: MatchSpec {
                params: vec!["entity".into()],
                filters: vec![EntityMatch::default()],
            },
            actions: vec![Action::Emit {
                event: "landed".into(),
                payload: json!({
                    "who": "$entity",
                    "marker": "landed"
                }),
            }],
        });
        let processor = RuleProcessor::new(rules);
        let (bus, _endpoints) = Bus::new(4, 4);
        let mut marker_rx = bus.marker.subscribe();
        let mut domain_rx = bus.domain.subscribe();

        processor.process(&mut world, &bus, &TickContext { tick: 1, dt: 0.0 }, 16);

        let marker_event = marker_rx.recv().await.expect("marker event should arrive");
        match marker_event {
            MarkerEvent::Reached {
                entity: actual,
                marker,
            } => {
                assert_eq!(actual, entity);
                assert_eq!(marker, "landed");
            }
        }
        assert!(
            domain_rx
                .try_recv()
                .expect("domain channel should stay open")
                .is_none()
        );
        assert_eq!(
            world
                .resource::<EventQueue>()
                .expect("EventQueue is inserted by World::new")
                .len(),
            1
        );

        processor.process(&mut world, &bus, &TickContext { tick: 2, dt: 0.0 }, 16);

        let emitted = domain_rx
            .recv()
            .await
            .expect("marker rule domain event should arrive");
        assert_eq!(emitted.name, "landed");
        assert_eq!(
            emitted.payload.get("who"),
            Some(&json!({ "index": entity.index, "generation": entity.generation }))
        );
        assert_eq!(emitted.payload.get("marker"), Some(&json!("landed")));
        assert!(
            world
                .resource::<EventQueue>()
                .expect("EventQueue is inserted by World::new")
                .is_empty()
        );
    }

    #[test]
    fn process_drains_events_without_subscribers() {
        let mut world = World::new();
        let entity = world.spawn().finish();
        {
            let queue = world
                .resource_mut::<EventQueue>()
                .expect("EventQueue is inserted by World::new");
            queue.emit_domain(DomainEvent::tick());
            queue.emit_marker(MarkerEvent::Reached {
                entity,
                marker: "landed".into(),
            });
        }
        let (bus, _endpoints) = Bus::new(4, 4);
        let processor = RuleProcessor::new(RuleSet::new());

        processor.process(&mut world, &bus, &TickContext { tick: 1, dt: 0.0 }, 16);

        assert!(
            world
                .resource::<EventQueue>()
                .expect("EventQueue is inserted by World::new")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn process_preserves_domain_fifo_order() {
        let mut world = World::new();
        {
            let queue = world
                .resource_mut::<EventQueue>()
                .expect("EventQueue is inserted by World::new");
            queue.emit_domain(DomainEvent::custom("first", JsonMap::default()));
            queue.emit_domain(DomainEvent::custom("second", JsonMap::default()));
        }
        let (bus, _endpoints) = Bus::new(4, 4);
        let mut domain_rx = bus.domain.subscribe();
        let processor = RuleProcessor::new(RuleSet::new());

        processor.process(&mut world, &bus, &TickContext { tick: 1, dt: 0.0 }, 16);

        let first = domain_rx
            .recv()
            .await
            .expect("first domain event should arrive");
        let second = domain_rx
            .recv()
            .await
            .expect("second domain event should arrive");
        assert_eq!(first.name, "first");
        assert_eq!(second.name, "second");
    }

    #[test]
    fn process_stops_at_max_iterations_and_keeps_remaining_domain_events() {
        let mut world = World::new();
        world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .emit_domain(DomainEvent::tick());

        let mut rules = RuleSet::new();
        rules.add(Rule {
            id: RuleId("tick-to-next".into()),
            event_name: "tick".into(),
            match_spec: MatchSpec::default(),
            actions: vec![Action::Emit {
                event: "next".into(),
                payload: json!({}),
            }],
        });
        let processor = RuleProcessor::new(rules);
        let (bus, _endpoints) = Bus::new(4, 4);
        let mut domain_rx = bus.domain.subscribe();

        processor.process(&mut world, &bus, &TickContext { tick: 1, dt: 0.0 }, 1);

        assert_eq!(
            world
                .resource::<EventQueue>()
                .expect("EventQueue is inserted by World::new")
                .len(),
            1
        );
        let forwarded = domain_rx
            .try_recv()
            .expect("processed domain event should arrive");
        let limit = domain_rx
            .try_recv()
            .expect("limit diagnostic should arrive");

        let forwarded = forwarded.expect("processed domain event should arrive");
        let limit = limit.expect("limit diagnostic should arrive");

        assert_eq!(forwarded.name, "tick");
        assert_eq!(limit.name, "rule_iteration_limit_reached");
        assert_eq!(limit.payload.get("limit"), Some(&json!(1)));
        assert_eq!(limit.payload.get("remaining_domain_len"), Some(&json!(1)));
    }

    #[test]
    fn process_runs_matching_set_velocity_rule() {
        let mut world = World::new();
        let target = world.spawn().with(Velocity(Vec2::ZERO)).finish();
        let other = world.spawn().finish();
        world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .emit_domain(DomainEvent::collision(target, other, Vec2::new(1.0, 0.0)));

        let mut rules = RuleSet::new();
        rules.add(Rule {
            id: RuleId("set-speed".into()),
            event_name: "collision".into(),
            match_spec: MatchSpec {
                params: vec!["a".into()],
                filters: vec![EntityMatch::default()],
            },
            actions: vec![Action::SetVelocity {
                target: ParamId(0),
                velocity: Vec2::new(4.0, -2.0),
            }],
        });
        let processor = RuleProcessor::new(rules);
        let (bus, _endpoints) = Bus::new(4, 4);

        processor.process(&mut world, &bus, &TickContext { tick: 1, dt: 0.0 }, 16);

        assert_eq!(
            world.get::<Velocity>(target).map(|velocity| velocity.0),
            Some(Vec2::new(4.0, -2.0))
        );
    }

    #[test]
    fn process_skips_rule_when_required_tag_is_missing() {
        let mut world = World::new();
        let ball = world
            .tag_registry_mut()
            .intern("Ball")
            .expect("tag should intern");
        let target = world.spawn().with(Velocity(Vec2::ZERO)).finish();
        let other = world.spawn().finish();
        world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .emit_domain(DomainEvent::collision(target, other, Vec2::new(1.0, 0.0)));

        let mut rules = RuleSet::new();
        rules.add(Rule {
            id: RuleId("require-ball".into()),
            event_name: "collision".into(),
            match_spec: MatchSpec {
                params: vec!["a".into()],
                filters: vec![EntityMatch {
                    required: vec![ball],
                    forbidden: Vec::new(),
                }],
            },
            actions: vec![Action::SetVelocity {
                target: ParamId(0),
                velocity: Vec2::new(4.0, -2.0),
            }],
        });
        let processor = RuleProcessor::new(rules);
        let (bus, _endpoints) = Bus::new(4, 4);

        processor.process(&mut world, &bus, &TickContext { tick: 1, dt: 0.0 }, 16);

        assert_eq!(
            world.get::<Velocity>(target).map(|velocity| velocity.0),
            Some(Vec2::ZERO)
        );
    }

    #[test]
    fn process_reverses_velocity_from_collision_normal() {
        let mut world = World::new();
        let target = world.spawn().with(Velocity(Vec2::new(3.0, 4.0))).finish();
        let other = world.spawn().finish();
        world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .emit_domain(DomainEvent::collision(target, other, Vec2::new(1.0, 0.0)));

        let mut rules = RuleSet::new();
        rules.add(Rule {
            id: RuleId("bounce".into()),
            event_name: "collision".into(),
            match_spec: MatchSpec {
                params: vec!["a".into()],
                filters: vec![EntityMatch::default()],
            },
            actions: vec![Action::ReverseVelocity {
                target: ParamId(0),
                axis: ReverseAxis::FromNormal,
            }],
        });
        let processor = RuleProcessor::new(rules);
        let (bus, _endpoints) = Bus::new(4, 4);

        processor.process(&mut world, &bus, &TickContext { tick: 1, dt: 0.0 }, 16);

        assert_eq!(
            world.get::<Velocity>(target).map(|velocity| velocity.0),
            Some(Vec2::new(-3.0, 4.0))
        );
    }

    #[test]
    fn process_despawns_bound_entity() {
        let mut world = World::new();
        let target = world.spawn().finish();
        let other = world.spawn().finish();
        world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .emit_domain(DomainEvent::collision(target, other, Vec2::new(1.0, 0.0)));

        let mut rules = RuleSet::new();
        rules.add(Rule {
            id: RuleId("despawn-target".into()),
            event_name: "collision".into(),
            match_spec: MatchSpec {
                params: vec!["a".into(), "b".into()],
                filters: vec![EntityMatch::default(), EntityMatch::default()],
            },
            actions: vec![Action::Despawn { target: ParamId(0) }],
        });
        let processor = RuleProcessor::new(rules);
        let (bus, _endpoints) = Bus::new(4, 4);

        processor.process(&mut world, &bus, &TickContext { tick: 1, dt: 0.0 }, 16);

        assert!(!world.is_alive(target));
        assert!(world.is_alive(other));
    }

    #[test]
    fn process_spawns_template_at_action_position() {
        let mut world = World::new();
        let projectile = world
            .tag_registry_mut()
            .intern("Projectile")
            .expect("tag should intern");
        let mut tags = TagSet::new();
        tags.insert(projectile);
        let mut templates = TemplateStore::new();
        templates.insert(Template {
            name: "Projectile".into(),
            tags,
            position: Some(Position(Vec2::new(-1.0, -1.0))),
            velocity: Some(Velocity(Vec2::new(0.0, 9.0))),
            aabb: Some(Aabb {
                half_extents: Vec2::new(0.5, 0.25),
            }),
            animator: None,
        });
        world.insert_resource(templates);
        world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .emit_domain(DomainEvent::tick());

        let mut rules = RuleSet::new();
        rules.add(Rule {
            id: RuleId("spawn-projectile".into()),
            event_name: "tick".into(),
            match_spec: MatchSpec::default(),
            actions: vec![Action::Spawn {
                template: "Projectile".into(),
                position: Vec2::new(5.0, 6.0),
            }],
        });
        let processor = RuleProcessor::new(rules);
        let (bus, _endpoints) = Bus::new(4, 4);

        processor.process(&mut world, &bus, &TickContext { tick: 1, dt: 0.0 }, 16);

        let spawned = world
            .entities()
            .find(|entity| {
                world
                    .get::<TagSet>(*entity)
                    .is_some_and(|tags| tags.contains(projectile))
            })
            .expect("spawn action should create tagged entity");
        assert_eq!(
            world.get::<Position>(spawned).map(|position| position.0),
            Some(Vec2::new(5.0, 6.0))
        );
        assert_eq!(
            world.get::<Velocity>(spawned).map(|velocity| velocity.0),
            Some(Vec2::new(0.0, 9.0))
        );
        assert_eq!(
            world.get::<Aabb>(spawned).map(|aabb| aabb.half_extents),
            Some(Vec2::new(0.5, 0.25))
        );
    }

    #[tokio::test]
    async fn process_plays_animation_for_bound_entity() {
        let mut world = World::new();
        let target = world.spawn().finish();
        let other = world.spawn().finish();
        world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .emit_domain(DomainEvent::collision(target, other, Vec2::new(1.0, 0.0)));

        let mut rules = RuleSet::new();
        rules.add(Rule {
            id: RuleId("play-hit".into()),
            event_name: "collision".into(),
            match_spec: MatchSpec {
                params: vec!["a".into(), "b".into()],
                filters: vec![EntityMatch::default(), EntityMatch::default()],
            },
            actions: vec![Action::PlayAnimation {
                target: ParamId(0),
                clip: "hit".into(),
                priority: 7,
            }],
        });
        let processor = RuleProcessor::new(rules);
        let (bus, _endpoints) = Bus::new(4, 4);
        let mut presentation_rx = bus.presentation.subscribe();

        processor.process(&mut world, &bus, &TickContext { tick: 1, dt: 0.0 }, 16);

        let animator = world
            .get::<Animator>(target)
            .expect("play_animation should set Animator");
        assert_eq!(animator.clip.name, "hit");
        assert!((animator.elapsed - 0.0).abs() < f32::EPSILON);
        assert!((animator.speed - 1.0).abs() < f32::EPSILON);
        assert!(!animator.looping);

        let command = presentation_rx
            .recv()
            .await
            .expect("presentation command should arrive");
        match command {
            PresentationCommand::PlayAnimation {
                entity,
                clip,
                priority,
            } => {
                assert_eq!(entity, target);
                assert_eq!(clip, "hit");
                assert_eq!(priority, 7);
            }
        }
    }

    #[tokio::test]
    async fn process_emits_failure_event_and_continues_other_rules() {
        let mut world = World::new();
        let missing_velocity = world.spawn().finish();
        let valid_target = world.spawn().with(Velocity(Vec2::ZERO)).finish();
        world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .emit_domain(DomainEvent::collision(
                missing_velocity,
                valid_target,
                Vec2::new(1.0, 0.0),
            ));

        let mut rules = RuleSet::new();
        rules.add(Rule {
            id: RuleId("bad-rule".into()),
            event_name: "collision".into(),
            match_spec: MatchSpec {
                params: vec!["a".into(), "b".into()],
                filters: vec![EntityMatch::default(), EntityMatch::default()],
            },
            actions: vec![Action::SetVelocity {
                target: ParamId(0),
                velocity: Vec2::new(1.0, 1.0),
            }],
        });
        rules.add(Rule {
            id: RuleId("good-rule".into()),
            event_name: "collision".into(),
            match_spec: MatchSpec {
                params: vec!["a".into(), "b".into()],
                filters: vec![EntityMatch::default(), EntityMatch::default()],
            },
            actions: vec![Action::SetVelocity {
                target: ParamId(1),
                velocity: Vec2::new(7.0, 8.0),
            }],
        });
        let processor = RuleProcessor::new(rules);
        let (bus, _endpoints) = Bus::new(4, 4);
        let mut domain_rx = bus.domain.subscribe();

        processor.process(&mut world, &bus, &TickContext { tick: 1, dt: 0.0 }, 16);
        drop(bus);

        let original = domain_rx
            .recv()
            .await
            .expect("original event should arrive");
        let failure = domain_rx.recv().await.expect("failure event should arrive");

        assert_eq!(original.name, "collision");
        assert_eq!(failure.name, "rule_action_failed");
        assert_eq!(
            failure.payload.get("rule"),
            Some(&serde_json::json!("bad-rule"))
        );
        assert_eq!(
            failure.payload.get("action_index"),
            Some(&serde_json::json!(0))
        );
        assert_eq!(
            world
                .get::<Velocity>(valid_target)
                .map(|velocity| velocity.0),
            Some(Vec2::new(7.0, 8.0))
        );
    }

    #[tokio::test]
    async fn process_emits_domain_event_with_bound_entities() {
        let mut world = World::new();
        let actor = world.spawn().finish();
        let other = world.spawn().finish();
        world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .emit_domain(DomainEvent::collision(actor, other, Vec2::new(1.0, 0.0)));

        let mut rules = RuleSet::new();
        rules.add(Rule {
            id: RuleId("emit-bounced".into()),
            event_name: "collision".into(),
            match_spec: MatchSpec {
                params: vec!["a".into(), "b".into()],
                filters: vec![EntityMatch::default(), EntityMatch::default()],
            },
            actions: vec![Action::Emit {
                event: "bounced".into(),
                payload: json!({
                    "who": "$a",
                    "other": "$b",
                    "label": "wall_hit",
                    "nested": { "actor": "$a" },
                    "items": ["$b", "literal"]
                }),
            }],
        });
        let processor = RuleProcessor::new(rules);
        let (bus, _endpoints) = Bus::new(4, 4);
        let mut domain_rx = bus.domain.subscribe();

        processor.process(&mut world, &bus, &TickContext { tick: 1, dt: 0.0 }, 16);
        drop(bus);

        let original = domain_rx
            .recv()
            .await
            .expect("original event should arrive");
        let emitted = domain_rx.recv().await.expect("emitted event should arrive");

        assert_eq!(original.name, "collision");
        assert_eq!(emitted.name, "bounced");
        assert_eq!(
            emitted.payload.get("who"),
            Some(&json!({ "index": actor.index, "generation": actor.generation }))
        );
        assert_eq!(
            JsonValue::Object(emitted.payload.clone()).pointer("/nested/actor"),
            Some(&json!({ "index": actor.index, "generation": actor.generation }))
        );
        assert_eq!(
            JsonValue::Object(emitted.payload.clone()).pointer("/items/0"),
            Some(&json!({ "index": other.index, "generation": other.generation }))
        );
        assert_eq!(emitted.payload.get("label"), Some(&json!("wall_hit")));
    }
}
