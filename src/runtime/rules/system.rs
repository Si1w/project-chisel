use crate::ecs::schedule::TickContext;
use crate::ecs::world::World;
use crate::event::bus::Bus;
use crate::event::queue::EventQueue;
use crate::runtime::rules::model::RuleSet;

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

    /// Drains the `EventQueue` resource and forwards each drained event
    /// to `bus` so external observers (stdout JSONL writer, future
    /// renderer) see it.
    ///
    /// Matching rules, applying actions, and `max_iterations` cascade
    /// handling are the next layer after this minimal drain/forward
    /// loop.
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
            let _ = bus.domain.emit(event);
        }

        while let Some(event) = world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .next_marker()
        {
            let _ = bus.marker.emit(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Map as JsonMap;

    use crate::event::payload::{DomainEvent, MarkerEvent};

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
}
