use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Top-level CLI. Provisional binary name; will be renamed before v0
/// ships.
#[derive(Debug, Parser)]
#[command(name = "chisel", version, about = "CLI-first 2D game engine")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Scaffold a new project directory.
    New { dir: PathBuf },

    /// Drive the engine to completion (or to `--max-ticks`).
    Run {
        #[arg(default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value_t = 0.016)]
        dt: f32,
        #[arg(long)]
        max_ticks: Option<u64>,
    },

    /// Advance `N` ticks (1 by default).
    Step {
        #[arg(default_value = ".")]
        root: PathBuf,
        #[arg(default_value_t = 1)]
        count: u32,
    },

    /// Dump a JSONL world snapshot.
    Inspect {
        #[arg(default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        query: Option<String>,
    },

    /// Inject a simulated input event for testing.
    Emit {
        #[arg(default_value = ".")]
        root: PathBuf,
        event: String,
    },
}
