use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value as JsonValue;

#[derive(Debug, Deserialize)]
pub struct SceneSchema {
    pub name: String,

    /// Entity instances to spawn when this scene loads.
    #[serde(default)]
    pub entities: Vec<SceneEntitySchema>,

    /// Domain events to publish immediately after scene spawn.
    #[serde(default)]
    pub initial_events: Vec<JsonValue>,
}

#[derive(Debug, Deserialize)]
pub struct SceneEntitySchema {
    /// Must match an `EntitySchema::name` loaded from `entities/`.
    pub template: String,

    /// Per-instance component overrides on top of template defaults.
    /// Same shape as `EntitySchema::components`.
    #[serde(default)]
    pub overrides: HashMap<String, JsonValue>,
}
