use serde::{Deserialize, Serialize};

/// Generational entity handle. Two fields: a stable index and a
/// generation counter that bumps each time the index is reused.
///
/// Comparing both fields on lookup prevents the ABA aliasing bug where a
/// despawned entity's index gets recycled and an old handle silently
/// refers to the new entity.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Entity {
    pub index: u32,
    pub generation: u32,
}
