use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::runtime::schema::rule::EntityMatchSchema;

#[derive(Debug, Default, Deserialize)]
pub struct InputSchema {
    #[serde(default, rename = "map")]
    pub maps: Vec<InputMapSchema>,
}

#[derive(Debug, Deserialize)]
pub struct InputMapSchema {
    pub trigger: InputTriggerSchema,

    /// Same shape as `RuleSchema::match_spec` — filter on entities by
    /// tag. Param names referenced by `emit.payload` strings starting
    /// with `$`.
    #[serde(default, rename = "match")]
    pub match_spec: HashMap<String, EntityMatchSchema>,

    pub emit: InputEmitSchema,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputTriggerSchema {
    KeyPress { key: String },
    KeyRelease { key: String },
}

#[derive(Debug, Deserialize)]
pub struct InputEmitSchema {
    /// Domain event name to publish when the trigger fires and match
    /// passes.
    #[serde(rename = "type")]
    pub event_type: String,

    /// Payload fields. `"$name"` strings substitute matched entities at
    /// emit time. Lives in its own `[map.emit.payload]` subtable to keep
    /// emit metadata (future: `delay`, `priority`) separate from event
    /// data.
    #[serde(default)]
    pub payload: JsonValue,
}
