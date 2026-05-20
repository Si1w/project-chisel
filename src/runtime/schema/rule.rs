use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::math::vec2::Vec2;
use crate::runtime::rules::model::ReverseAxis;

#[derive(Debug, Deserialize)]
pub struct RuleSchema {
    /// Event name consumed by the rule processor. v0 has no global event-name
    /// registry, so custom domain event names are accepted.
    pub event: String,

    /// `[match.<param>]` sections. Empty for `tick` and other no-param
    /// events.
    #[serde(default, rename = "match")]
    pub match_spec: HashMap<String, EntityMatchSchema>,

    /// `[[do]]` array. Empty list is rejected by the loader.
    #[serde(default, rename = "do")]
    pub actions: Vec<ActionSchema>,
}

#[derive(Debug, Default, Deserialize)]
pub struct EntityMatchSchema {
    /// Tag names the entity must have.
    #[serde(default)]
    pub with: Vec<String>,

    /// Tag names the entity must not have.
    #[serde(default)]
    pub without: Vec<String>,
}

/// One `[[do]]` entry. Internally tagged on `action` so each entry
/// reads `action = "set_velocity"` etc.
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ActionSchema {
    SetVelocity {
        entity: String,
        x: f32,
        y: f32,
    },
    ReverseVelocity {
        entity: String,
        #[serde(default)]
        axis: ReverseAxis,
    },
    Spawn {
        template: String,
        position: Vec2,
    },
    Despawn {
        entity: String,
    },
    Emit {
        event: String,
        #[serde(default)]
        payload: JsonValue,
    },
    PlayAnimation {
        entity: String,
        clip: String,
        #[serde(default = "default_priority")]
        priority: u8,
    },
}

fn default_priority() -> u8 {
    0
}
