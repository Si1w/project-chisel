use std::path::Path;

use anyhow::Result;

use crate::ecs::world::World;
use crate::runtime::rules::model::RuleSet;

/// Loads all `*.toml` files from `dir` into a `RuleSet`. Tag names are
/// interned into the world's `TagRegistry` as they're encountered.
///
/// Files load in lexicographic order so rules sharing an event keep a
/// deterministic precedence.
///
/// # Errors
///
/// First failure short-circuits with `anyhow` context describing which
/// file / rule / action failed. v0 callers (`bootstrap`, `reload`
/// command handler) just surface the chain to stderr — no programmatic
/// recovery is expected from a malformed manifest.
pub fn load_rules(_dir: &Path, _world: &mut World) -> Result<RuleSet> {
    todo!()
}
