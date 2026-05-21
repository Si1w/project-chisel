use anyhow::Result;

use crate::ecs::schedule::TickContext;
use crate::event::payload::DomainEvent;
use crate::runtime::bootstrap::EngineState;

pub const MAX_RULE_ITERATIONS: u32 = 1024;

/// Advance an engine state by a fixed number of ticks and return the domain
/// events observed during those ticks.
///
/// # Errors
///
/// Returns an error if the bus subscriber lags or closes while draining
/// events.
pub fn run_ticks(state: &mut EngineState, dt: f32, max_ticks: u64) -> Result<Vec<DomainEvent>> {
    let mut domain_rx = state.bus.domain.subscribe();
    let mut events = Vec::new();

    for tick in 1..=max_ticks {
        let ctx = TickContext { tick, dt };
        state.schedule.tick(&mut state.world, ctx);
        state
            .rule_processor
            .process(&mut state.world, &state.bus, &ctx, MAX_RULE_ITERATIONS);
        drain_domain_events(&mut domain_rx, &mut events)?;
    }

    Ok(events)
}

fn drain_domain_events(
    domain_rx: &mut crate::event::bus::OutboundRx<DomainEvent>,
    events: &mut Vec<DomainEvent>,
) -> Result<()> {
    while let Some(event) = domain_rx.try_recv()? {
        events.push(event);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::component::spatial::Velocity;
    use crate::ecs::entity::Entity;
    use crate::runtime::bootstrap::bootstrap;
    use crate::tag::set::TagSet;

    use super::*;

    #[test]
    fn run_ticks_drives_ball_collision_example() {
        let mut state =
            bootstrap(Path::new("example/ball_collision")).expect("example should bootstrap");

        let events = run_ticks(&mut state, 0.5, 1).expect("example should run");

        assert!(events.iter().any(|event| event.name == "collision"));
        let ball = entity_with_tag(&state, "Ball");
        assert_eq!(
            state
                .world
                .get::<Velocity>(ball)
                .map(|velocity| velocity.0.x),
            Some(-3.0)
        );
    }

    fn entity_with_tag(state: &EngineState, tag: &str) -> Entity {
        let tag = state
            .world
            .tag_registry()
            .lookup(tag)
            .expect("tag should be interned");
        state
            .world
            .entities()
            .find(|entity| {
                state
                    .world
                    .get::<TagSet>(*entity)
                    .is_some_and(|tags| tags.contains(tag))
            })
            .expect("entity with tag should exist")
    }
}
