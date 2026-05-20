pub mod args;

use std::io::Write;

use anyhow::{Context, Result, bail};
use clap::Parser;

use crate::cli::args::{Cli, Command};
use crate::event::channel::Channel;
use crate::event::envelope::BusEnvelope;
use crate::runtime::bootstrap::bootstrap;
use crate::runtime::run::run_ticks;

/// Parse process arguments and execute the requested CLI command.
///
/// # Errors
///
/// Returns an error if argument execution fails or runtime JSONL cannot be
/// written to stdout.
pub fn run_cli() -> Result<()> {
    let cli = Cli::parse();
    let mut stdout = std::io::stdout().lock();
    run_command(cli, &mut stdout)
}

/// Execute a parsed CLI command and write runtime JSONL to `output`.
///
/// # Errors
///
/// Returns an error if the command is not implemented yet, bootstrapping
/// fails, ticking fails, or JSONL output cannot be written.
pub fn run_command(cli: Cli, output: &mut impl Write) -> Result<()> {
    match cli.command {
        Command::Run {
            root,
            dt,
            max_ticks,
        } => {
            let max_ticks = max_ticks.context("--max-ticks is required for v0 run")?;
            let mut state = bootstrap(&root)?;
            let events = run_ticks(&mut state, dt, max_ticks)?;
            for event in events {
                serde_json::to_writer(&mut *output, &BusEnvelope::new(Channel::Domain, &event))?;
                writeln!(output)?;
            }
            Ok(())
        }
        Command::New { .. }
        | Command::Step { .. }
        | Command::Inspect { .. }
        | Command::Emit { .. } => bail!("command is not implemented yet"),
    }
}

#[cfg(test)]
mod tests {
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
}
