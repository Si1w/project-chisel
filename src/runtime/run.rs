use std::io::{BufRead, Write};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::ecs::schedule::TickContext;
use crate::event::bus::OutboundRx;
use crate::event::channel::Channel;
use crate::event::envelope::BusEnvelope;
use crate::event::payload::{
    AckStatus, CommandAckEvent, CommandEvent, DomainEvent, InputEvent, MarkerEvent,
    PresentationCommand,
};
use crate::event::queue::EventQueue;
use crate::runtime::bootstrap::EngineState;
use crate::runtime::snapshot::snapshot_world;

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

/// Process a persistent JSONL runtime session until `input` reaches EOF.
///
/// # Errors
///
/// Returns an error if reading a line from `input` or writing to `output`
/// fails.
pub fn run_jsonl_loop(
    state: &mut EngineState,
    input: &mut impl BufRead,
    output: &mut impl Write,
    dt: f32,
) -> Result<()> {
    let mut line = String::new();
    let mut tick = 0;
    while input.read_line(&mut line)? != 0 {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            process_jsonl_line(state, trimmed, output, dt, &mut tick)?;
            output.flush()?;
        }
        line.clear();
    }
    output.flush()?;
    Ok(())
}

fn process_jsonl_line(
    state: &mut EngineState,
    line: &str,
    output: &mut impl Write,
    dt: f32,
    tick: &mut u64,
) -> Result<()> {
    match parse_jsonl_line(line) {
        Ok((Channel::Command, payload)) => match serde_json::from_value::<CommandEvent>(payload) {
            Ok(command) => run_command_event(state, command, output, dt, tick),
            Err(error) => write_ack(
                output,
                AckStatus::Error,
                Some(format!("parse command: {error}")),
            ),
        },
        Ok((Channel::Input, payload)) => match serde_json::from_value::<InputEvent>(payload) {
            Ok(input) => run_input_event(state, &input, output, *tick, dt),
            Err(error) => write_ack(
                output,
                AckStatus::Error,
                Some(format!("parse input: {error}")),
            ),
        },
        Ok((channel, _)) => write_ack(
            output,
            AckStatus::Error,
            Some(format!("unsupported inbound channel {channel:?}")),
        ),
        Err(error) => write_ack(output, AckStatus::Error, Some(error.to_string())),
    }
}

fn parse_jsonl_line(line: &str) -> Result<(Channel, JsonValue)> {
    let JsonValue::Object(mut object) =
        serde_json::from_str::<JsonValue>(line).context("parse JSONL line")?
    else {
        bail!("JSONL line must be an object");
    };
    let channel = object
        .remove("channel")
        .context("JSONL line must contain channel")?;
    let channel = serde_json::from_value::<Channel>(channel).context("parse channel")?;

    Ok((channel, JsonValue::Object(object)))
}

fn run_command_event(
    state: &mut EngineState,
    command: CommandEvent,
    output: &mut impl Write,
    dt: f32,
    tick: &mut u64,
) -> Result<()> {
    match command {
        CommandEvent::Step { count } => {
            let mut receivers = OutputReceivers::new(state);
            for _ in 0..count {
                *tick += 1;
                let ctx = TickContext { tick: *tick, dt };
                state.schedule.tick(&mut state.world, ctx);
                state.rule_processor.process(
                    &mut state.world,
                    &state.bus,
                    &ctx,
                    MAX_RULE_ITERATIONS,
                );
            }
            receivers.write(output)?;
            write_ack(output, AckStatus::Ok, None)
        }
        CommandEvent::Inspect { query } => {
            if query.is_some() {
                return write_ack(
                    output,
                    AckStatus::Error,
                    Some("inspect query is not implemented yet".into()),
                );
            }
            for event in snapshot_world(&state.world, *tick) {
                write_record(output, Channel::Snapshot, &event)?;
            }
            write_ack(output, AckStatus::Ok, None)
        }
        CommandEvent::SimulateInput { event } => {
            run_input_event(state, &event, output, *tick, dt)?;
            write_ack(output, AckStatus::Ok, None)
        }
        CommandEvent::Save | CommandEvent::Reload | CommandEvent::Pause | CommandEvent::Resume => {
            write_ack(
                output,
                AckStatus::Error,
                Some("command is not implemented yet".into()),
            )
        }
    }
}

fn run_input_event(
    state: &mut EngineState,
    input: &InputEvent,
    output: &mut impl Write,
    tick: u64,
    dt: f32,
) -> Result<()> {
    let mut receivers = OutputReceivers::new(state);
    let events = state.input_mapper.map(&state.world, input)?;
    {
        let queue = state
            .world
            .resource_mut::<EventQueue>()
            .context("EventQueue resource is missing")?;
        for event in events {
            queue.emit_domain(event);
        }
    }
    let ctx = TickContext { tick, dt };
    state
        .rule_processor
        .process(&mut state.world, &state.bus, &ctx, MAX_RULE_ITERATIONS);
    receivers.write(output)
}

fn write_ack(output: &mut impl Write, status: AckStatus, message: Option<String>) -> Result<()> {
    write_record(
        output,
        Channel::CommandAck,
        &CommandAckEvent {
            command_id: None,
            status,
            message,
        },
    )
}

fn write_record<T: Serialize>(output: &mut impl Write, channel: Channel, event: &T) -> Result<()> {
    serde_json::to_writer(&mut *output, &BusEnvelope::new(channel, event))?;
    writeln!(output)?;
    Ok(())
}

struct OutputReceivers {
    domain: OutboundRx<DomainEvent>,
    marker: OutboundRx<MarkerEvent>,
    presentation: OutboundRx<PresentationCommand>,
}

impl OutputReceivers {
    fn new(state: &EngineState) -> Self {
        Self {
            domain: state.bus.domain.subscribe(),
            marker: state.bus.marker.subscribe(),
            presentation: state.bus.presentation.subscribe(),
        }
    }

    fn write(&mut self, output: &mut impl Write) -> Result<()> {
        while let Some(event) = self.domain.try_recv()? {
            write_record(output, Channel::Domain, &event)?;
        }
        while let Some(event) = self.marker.try_recv()? {
            write_record(output, Channel::Marker, &event)?;
        }
        while let Some(event) = self.presentation.try_recv()? {
            write_record(output, Channel::Presentation, &event)?;
        }

        Ok(())
    }
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
