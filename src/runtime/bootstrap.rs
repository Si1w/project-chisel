use std::path::Path;

use anyhow::Result;

use crate::ecs::schedule::Schedule;
use crate::ecs::world::World;
use crate::event::bus::{Bus, BusEndpoints};
use crate::runtime::rules::system::RuleProcessor;
use crate::runtime::schema::paths::ManifestPaths;

/// Everything the engine needs to start ticking. Returned by `bootstrap`;
/// also reconstructed by `command:reload`.
///
/// Per-tick flow at the runtime layer:
/// 1. `schedule.tick(&mut world, ctx)` — ECS systems only; emits to
///    `EventQueue` resource on `world`.
/// 2. `rule_processor.process(&mut world, &bus, &ctx, MAX)` — drains
///    the queue, fans out to rules, forwards events to `bus`.
pub struct EngineState {
    pub paths: ManifestPaths,
    pub world: World,
    pub schedule: Schedule,
    pub bus: Bus,
    pub bus_endpoints: BusEndpoints,
    pub rule_processor: RuleProcessor,
}

/// Reads all `.toml` files under `root`, validates them, builds the
/// initial engine state, and returns it ready for the first
/// `Schedule::tick`.
///
/// # Errors
///
/// Any failure — missing `game.toml`, unknown action name, undefined
/// tag, schema version mismatch — surfaces as `anyhow::Error` with
/// file/rule context. No partial state.
///
/// Resources inserted into the returned `World`:
/// - `TagRegistry` (auto, via `World::new`)
/// - `EventQueue` (auto, via `World::new`)
/// - `Gravity` (from `[physics.gravity]` in `game.toml`, if present)
/// - `TemplateStore` (populated from `entities/*.toml`)
pub fn bootstrap(_root: &Path) -> Result<EngineState> {
    todo!()
}
