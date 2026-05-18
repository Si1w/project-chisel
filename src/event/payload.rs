use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};

use crate::ecs::entity::Entity;
use crate::math::vec2::Vec2;

/// Raw player input. v0 keyboard only.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputEvent {
    KeyPress { key: String },
    KeyRelease { key: String },
}

/// Control-plane directive from the agent. Single internal consumer
/// (the command handler); never visible to rules.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandEvent {
    Step { count: u32 },
    Inspect { query: Option<String> },
    Save,
    Reload,
    Pause,
    Resume,
    SimulateInput { event: InputEvent },
}

/// "Something happened in the game world." Fully untyped on the wire:
/// the variant of meaning lives in `name`, with all parameters inside
/// `payload`. Built-in events are constructed via helpers so callers
/// don't hand-roll `serde_json::json!` everywhere.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainEvent {
    #[serde(rename = "type")]
    pub name: String,
    #[serde(flatten)]
    pub payload: JsonMap<String, JsonValue>,
}

impl DomainEvent {
    #[must_use]
    pub fn tick() -> Self {
        todo!()
    }

    #[must_use]
    pub fn collision(_a: Entity, _b: Entity, _normal: Vec2) -> Self {
        todo!()
    }

    #[must_use]
    pub fn rule_action_failed(_rule: &str, _action_index: u32, _reason: &str) -> Self {
        todo!()
    }

    #[must_use]
    pub fn custom(_name: impl Into<String>, _payload: JsonMap<String, JsonValue>) -> Self {
        todo!()
    }
}

/// ECS animation system milestone notifications. v0 fires none; v2+ will.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MarkerEvent {
    Reached { entity: Entity, marker: String },
}

/// Imperative "show this" / "play that" commands produced by the logic
/// layer. v0 only logs them to stdout; v3 renderer consumes them.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PresentationCommand {
    PlayAnimation {
        entity: Entity,
        clip: String,
        priority: u8,
    },
}

/// Engine's reply to a `CommandEvent`. Surfaces parse / validation
/// errors without crashing the runtime.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandAckEvent {
    pub command_id: Option<String>,
    pub status: AckStatus,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AckStatus {
    Ok,
    Error,
}

/// State-dump output. Streamed `BeginSnapshot` / `Entity` / `EndSnapshot`
/// so a large world produces JSONL line by line rather than one blob.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SnapshotEvent {
    BeginSnapshot {
        tick: u64,
    },
    Entity {
        entity: Entity,
        components: JsonValue,
    },
    EndSnapshot,
}
