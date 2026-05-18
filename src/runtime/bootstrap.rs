use std::path::Path;

use anyhow::Result;

use crate::ecs::schedule::Schedule;
use crate::ecs::world::World;
use crate::event::bus::{Bus, BusEndpoints};
use crate::runtime::rules::model::RuleSet;
use crate::runtime::schema::paths::ManifestPaths;

/// Everything the engine needs to start ticking. Returned by `bootstrap`;
/// also reconstructed by `command:reload`.
pub struct EngineState {
    pub paths: ManifestPaths,
    pub world: World,
    pub schedule: Schedule,
    pub bus: Bus,
    pub bus_endpoints: BusEndpoints,
    pub rules: RuleSet,
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
pub fn bootstrap(_root: &Path) -> Result<EngineState> {
    todo!()
}
