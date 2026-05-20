use serde_json::Value as JsonValue;

use crate::component::spatial::Velocity;
use crate::ecs::entity::Entity;
use crate::ecs::schedule::TickContext;
use crate::ecs::world::World;
use crate::event::bus::Bus;
use crate::event::payload::DomainEvent;
use crate::event::queue::EventQueue;
use crate::math::vec2::Vec2;
use crate::runtime::rules::model::{Action, EntityMatch, ParamId, ReverseAxis, Rule, RuleSet};
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
    /// currently-supported rule actions (`set_velocity` and
    /// `reverse_velocity`). Marker events are forwarded only.
    ///
    /// `spawn`, `despawn`, `emit`, `play_animation`, and
    /// `max_iterations` cascade handling are deferred until their
    /// runtime dependencies land.
    ///
    /// # Panics
    ///
    /// Panics if the world's `EventQueue` resource has been removed.
    pub fn process(&self, world: &mut World, bus: &Bus, _ctx: &TickContext, _max_iterations: u32) {
        while let Some(event) = world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .next_domain()
        {
            let _ = bus.domain.emit(event.clone());
            self.process_domain_event(world, &event);
        }

        while let Some(event) = world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .next_marker()
        {
            let _ = bus.marker.emit(event);
        }
    }

    fn process_domain_event(&self, world: &mut World, event: &DomainEvent) {
        for rule in self.rules.rules_for(&event.name) {
            let Some(bindings) = bind_event(world, rule, event) else {
                continue;
            };
            run_actions(world, rule, &bindings, event);
        }
    }
}

struct RuleBindings {
    entities: Vec<Entity>,
}

impl RuleBindings {
    fn entity(&self, param: ParamId) -> Option<Entity> {
        self.entities.get(param.0 as usize).copied()
    }
}

fn bind_event(world: &World, rule: &Rule, event: &DomainEvent) -> Option<RuleBindings> {
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
        entities.push(entity);
    }

    Some(RuleBindings { entities })
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

fn run_actions(world: &mut World, rule: &Rule, bindings: &RuleBindings, event: &DomainEvent) {
    for (action_index, action) in rule.actions.iter().enumerate() {
        if let Err(reason) = run_action(world, action, bindings, event) {
            emit_rule_action_failed(world, rule, action_index, &reason);
            break;
        }
    }
}

fn run_action(
    world: &mut World,
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
        Action::Spawn { .. }
        | Action::Despawn { .. }
        | Action::Emit { .. }
        | Action::PlayAnimation { .. } => Err("action not implemented".into()),
    }
}

fn bound_entity(bindings: &RuleBindings, target: ParamId) -> Result<Entity, String> {
    bindings
        .entity(target)
        .ok_or_else(|| format!("param {} is not bound", target.0))
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
    use serde_json::Map as JsonMap;

    use crate::component::spatial::Velocity;
    use crate::event::payload::{DomainEvent, MarkerEvent};
    use crate::math::vec2::Vec2;
    use crate::runtime::rules::model::{
        Action, EntityMatch, MatchSpec, ParamId, ReverseAxis, Rule, RuleId,
    };

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
}
