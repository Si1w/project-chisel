use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value as JsonValue;

/// One entity template. The basename of `entities/<file>.toml` is not
/// the template identifier — `name` here is what scenes reference.
#[derive(Debug, Deserialize)]
pub struct EntitySchema {
    pub name: String,

    /// Tag names attached to every instance of this template. Loader
    /// interns each into the world's `TagRegistry`.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Map of component name to its JSON value. Loader dispatches on
    /// the name; in v0 only `position` / `velocity` / `aabb` /
    /// `animator` are accepted, unknown keys are a load error. v1+
    /// swaps the hardcoded dispatch for a `ComponentRegistry`.
    #[serde(default)]
    pub components: HashMap<String, JsonValue>,
}
