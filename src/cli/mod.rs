pub mod args;

use std::io::{BufRead, BufReader, Write};

use anyhow::{Context, Result, bail};
use clap::Parser;
use serde::Serialize;

use crate::cli::args::{Cli, Command};
use crate::event::channel::Channel;
use crate::event::envelope::BusEnvelope;
use crate::event::payload::InputEvent;
use crate::event::queue::EventQueue;
use crate::runtime::bootstrap::bootstrap;
use crate::runtime::run::{MAX_RULE_ITERATIONS, run_jsonl_loop, run_ticks};
use crate::runtime::snapshot::snapshot_world;
use crate::{ecs::schedule::TickContext, event::payload::DomainEvent};

const DEFAULT_STEP_DT: f32 = 0.016;

/// Parse process arguments and execute the requested CLI command.
///
/// # Errors
///
/// Returns an error if argument execution fails or runtime JSONL cannot be
/// written to stdout.
pub fn run_cli() -> Result<()> {
    let cli = Cli::parse();
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();
    let mut stdout = std::io::stdout().lock();
    run_command_with_input(cli, &mut stdin, &mut stdout)
}

/// Execute a parsed CLI command and write runtime JSONL to `output`.
///
/// # Errors
///
/// Returns an error if the command is not implemented yet, bootstrapping
/// fails, ticking fails, or JSONL output cannot be written.
pub fn run_command(cli: Cli, output: &mut impl Write) -> Result<()> {
    let mut input = BufReader::new(std::io::empty());
    run_command_with_input(cli, &mut input, output)
}

/// Execute a parsed CLI command, reading runtime JSONL commands from
/// `input` when the command enters session mode, and write runtime JSONL
/// records to `output`.
///
/// # Errors
///
/// Returns an error if the command is not implemented yet, bootstrapping
/// fails, ticking fails, JSONL input cannot be read, or JSONL output
/// cannot be written.
pub fn run_command_with_input(
    cli: Cli,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Result<()> {
    match cli.command {
        Command::Run {
            root,
            dt,
            max_ticks,
        } => {
            let mut state = bootstrap(&root)?;
            if let Some(max_ticks) = max_ticks {
                let events = run_ticks(&mut state, dt, max_ticks)?;
                for event in events {
                    write_event(output, Channel::Domain, &event)?;
                }
                Ok(())
            } else {
                run_jsonl_loop(&mut state, input, output, dt)
            }
        }
        Command::Step { root, count } => {
            let mut state = bootstrap(&root)?;
            let events = run_ticks(&mut state, DEFAULT_STEP_DT, u64::from(count))?;
            for event in events {
                write_event(output, Channel::Domain, &event)?;
            }
            for event in snapshot_world(&state.world, u64::from(count)) {
                write_event(output, Channel::Snapshot, &event)?;
            }
            Ok(())
        }
        Command::Inspect { root, query } => {
            if query.is_some() {
                bail!("inspect query is not implemented yet");
            }
            let state = bootstrap(&root)?;
            let events = snapshot_world(&state.world, 0);
            for event in events {
                write_event(output, Channel::Snapshot, &event)?;
            }
            Ok(())
        }
        Command::Emit { root, event } => {
            let mut state = bootstrap(&root)?;
            let input = serde_json::from_str::<InputEvent>(&event)
                .with_context(|| format!("parse input event JSON {event:?}"))?;
            let mut domain_rx = state.bus.domain.subscribe();
            let events = state.input_mapper.map(&state.world, &input)?;
            {
                let queue = state
                    .world
                    .resource_mut::<EventQueue>()
                    .context("EventQueue resource is missing")?;
                for event in events {
                    queue.emit_domain(event);
                }
            }
            let ctx = TickContext {
                tick: 0,
                dt: DEFAULT_STEP_DT,
            };
            state
                .rule_processor
                .process(&mut state.world, &state.bus, &ctx, MAX_RULE_ITERATIONS);
            write_domain_queue(&mut domain_rx, output)?;
            Ok(())
        }
        Command::New { .. } => {
            bail!("command is not implemented yet")
        }
    }
}

fn write_domain_queue(
    domain_rx: &mut crate::event::bus::OutboundRx<DomainEvent>,
    output: &mut impl Write,
) -> Result<()> {
    while let Some(event) = domain_rx.try_recv()? {
        write_event(output, Channel::Domain, &event)?;
    }

    Ok(())
}

fn write_event<T: Serialize>(output: &mut impl Write, channel: Channel, event: &T) -> Result<()> {
    serde_json::to_writer(&mut *output, &BusEnvelope::new(channel, event))?;
    writeln!(output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::PathBuf;

    use crate::cli::args::Command;

    use super::*;

    #[test]
    fn run_command_writes_domain_jsonl() {
        let cli = Cli {
            command: Command::Run {
                root: PathBuf::from("example/ball_collision"),
                dt: 0.5,
                max_ticks: Some(1),
            },
        };
        let mut output = Vec::new();

        run_command(cli, &mut output).expect("run command should execute");

        let output = String::from_utf8(output).expect("output should be utf8");
        assert!(output.lines().any(|line| {
            line.contains(r#""channel":"domain""#) && line.contains(r#""type":"collision""#)
        }));
    }

    #[test]
    fn run_command_without_max_ticks_accepts_empty_jsonl_session() {
        let cli = Cli {
            command: Command::Run {
                root: PathBuf::from("example/ball_collision"),
                dt: 0.5,
                max_ticks: None,
            },
        };
        let mut output = Vec::new();

        run_command(cli, &mut output).expect("empty JSONL session should exit cleanly");

        assert!(output.is_empty());
    }

    #[test]
    fn run_command_jsonl_session_steps_then_inspects_same_world() {
        let cli = Cli {
            command: Command::Run {
                root: PathBuf::from("example/ball_collision"),
                dt: 0.016,
                max_ticks: None,
            },
        };
        let input = concat!(
            r#"{"channel":"command","type":"step","count":21}"#,
            "\n",
            r#"{"channel":"command","type":"inspect","query":null}"#,
            "\n"
        );
        let mut input = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        run_command_with_input(cli, &mut input, &mut output).expect("JSONL session should execute");

        let events = jsonl_values(output);
        assert!(events.iter().any(|event| {
            event.get("channel") == Some(&serde_json::json!("domain"))
                && event.get("type") == Some(&serde_json::json!("collision"))
        }));
        assert!(events.iter().any(|event| {
            event.get("channel") == Some(&serde_json::json!("snapshot"))
                && event.get("type") == Some(&serde_json::json!("begin_snapshot"))
                && event.get("tick") == Some(&serde_json::json!(21))
        }));
        assert_eq!(
            events
                .iter()
                .filter(|event| {
                    event.get("channel") == Some(&serde_json::json!("command-ack"))
                        && event.get("status") == Some(&serde_json::json!("ok"))
                })
                .count(),
            2
        );
    }

    #[test]
    fn run_command_jsonl_session_recovers_after_malformed_line() {
        let cli = Cli {
            command: Command::Run {
                root: PathBuf::from("example/ball_collision"),
                dt: 0.016,
                max_ticks: None,
            },
        };
        let input = concat!(
            "not-json\n",
            r#"{"channel":"command","type":"inspect","query":null}"#,
            "\n"
        );
        let mut input = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        run_command_with_input(cli, &mut input, &mut output)
            .expect("malformed JSONL line should not abort the session");

        let events = jsonl_values(output);
        assert!(events.iter().any(|event| {
            event.get("channel") == Some(&serde_json::json!("command-ack"))
                && event.get("status") == Some(&serde_json::json!("error"))
        }));
        assert!(events.iter().any(|event| {
            event.get("channel") == Some(&serde_json::json!("snapshot"))
                && event.get("type") == Some(&serde_json::json!("begin_snapshot"))
        }));
        assert!(events.iter().any(|event| {
            event.get("channel") == Some(&serde_json::json!("command-ack"))
                && event.get("status") == Some(&serde_json::json!("ok"))
        }));
    }

    #[test]
    fn inspect_command_writes_snapshot_jsonl() {
        let cli = Cli {
            command: Command::Inspect {
                root: PathBuf::from("example/ball_collision"),
                query: None,
            },
        };
        let mut output = Vec::new();

        run_command(cli, &mut output).expect("inspect command should execute");

        let output = String::from_utf8(output).expect("output should be utf8");
        assert!(output.lines().any(|line| {
            line.contains(r#""channel":"snapshot""#) && line.contains(r#""type":"begin_snapshot""#)
        }));
        assert!(
            output
                .lines()
                .any(|line| line.contains(r#""type":"entity""#) && line.contains(r#""Ball""#))
        );
        assert!(
            output
                .lines()
                .any(|line| line.contains(r#""type":"end_snapshot""#))
        );
    }

    #[test]
    fn step_command_writes_domain_and_snapshot_jsonl() {
        let cli = Cli {
            command: Command::Step {
                root: PathBuf::from("example/ball_collision"),
                count: 21,
            },
        };
        let mut output = Vec::new();

        run_command(cli, &mut output).expect("step command should execute");

        let output = String::from_utf8(output).expect("output should be utf8");
        assert!(output.lines().any(|line| {
            line.contains(r#""channel":"domain""#) && line.contains(r#""type":"collision""#)
        }));
        assert!(output.lines().any(|line| {
            line.contains(r#""channel":"snapshot""#)
                && line.contains(r#""type":"begin_snapshot""#)
                && line.contains(r#""tick":21"#)
        }));
        assert!(output.lines().any(|line| {
            line.contains(r#""type":"entity""#)
                && line.contains(r#""Ball""#)
                && line.contains(r#""velocity":{"x":-3.0"#)
        }));
    }

    #[test]
    fn emit_command_maps_input_to_domain_jsonl() {
        let cli = Cli {
            command: Command::Emit {
                root: PathBuf::from("example/ball_collision"),
                event: r#"{"type":"key_press","key":"Space"}"#.into(),
            },
        };
        let mut output = Vec::new();

        run_command(cli, &mut output).expect("emit command should execute");

        let output = String::from_utf8(output).expect("output should be utf8");
        let events = output
            .lines()
            .map(serde_json::from_str::<serde_json::Value>)
            .collect::<Result<Vec<_>, _>>()
            .expect("output should be JSONL");
        assert!(events.iter().any(|event| {
            event.get("channel") == Some(&serde_json::json!("domain"))
                && event.get("type") == Some(&serde_json::json!("ball_input"))
                && event.get("source") == Some(&serde_json::json!("keyboard"))
                && event.pointer("/actor/index").is_some()
                && event.pointer("/actor/generation").is_some()
        }));
    }

    fn jsonl_values(output: Vec<u8>) -> Vec<serde_json::Value> {
        let output = String::from_utf8(output).expect("output should be utf8");
        output
            .lines()
            .map(serde_json::from_str::<serde_json::Value>)
            .collect::<Result<Vec<_>, _>>()
            .expect("output should be JSONL")
    }
}
