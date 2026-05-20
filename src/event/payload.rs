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
        Self::custom("tick", JsonMap::new())
    }

    #[must_use]
    pub fn collision(a: Entity, b: Entity, normal: Vec2) -> Self {
        let mut payload = JsonMap::new();
        payload.insert("a".into(), entity_value(a));
        payload.insert("b".into(), entity_value(b));
        payload.insert("normal".into(), vec2_value(normal));
        Self::custom("collision", payload)
    }

    #[must_use]
    pub fn rule_action_failed(rule: &str, action_index: u32, reason: &str) -> Self {
        let mut payload = JsonMap::new();
        payload.insert("rule".into(), JsonValue::from(rule));
        payload.insert("action_index".into(), JsonValue::from(action_index));
        payload.insert("reason".into(), JsonValue::from(reason));
        Self::custom("rule_action_failed", payload)
    }

    #[must_use]
    pub fn custom(name: impl Into<String>, payload: JsonMap<String, JsonValue>) -> Self {
        Self {
            name: name.into(),
            payload,
        }
    }
}

fn entity_value(entity: Entity) -> JsonValue {
    let mut value = JsonMap::new();
    value.insert("index".into(), JsonValue::from(entity.index));
    value.insert("generation".into(), JsonValue::from(entity.generation));
    JsonValue::Object(value)
}

fn vec2_value(vector: Vec2) -> JsonValue {
    let mut value = JsonMap::new();
    value.insert("x".into(), JsonValue::from(vector.x));
    value.insert("y".into(), JsonValue::from(vector.y));
    JsonValue::Object(value)
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

#[cfg(test)]
mod tests {
    use serde_json::{Map as JsonMap, json};

    use crate::event::channel::Channel;
    use crate::event::envelope::BusEnvelope;

    use super::*;

    #[test]
    fn tick_event_serializes_as_domain_type_only() {
        let event = DomainEvent::tick();

        let json = serde_json::to_value(BusEnvelope::new(Channel::Domain, &event))
            .expect("domain event should serialize");

        assert_eq!(json, json!({ "channel": "domain", "type": "tick" }));
    }

    #[test]
    fn collision_event_serializes_entities_and_normal() {
        let a = Entity {
            index: 1,
            generation: 0,
        };
        let b = Entity {
            index: 2,
            generation: 3,
        };

        let event = DomainEvent::collision(a, b, Vec2::new(0.0, 1.0));

        let json = serde_json::to_value(BusEnvelope::new(Channel::Domain, &event))
            .expect("domain event should serialize");

        assert_eq!(
            json,
            json!({
                "channel": "domain",
                "type": "collision",
                "a": { "index": 1, "generation": 0 },
                "b": { "index": 2, "generation": 3 },
                "normal": { "x": 0.0, "y": 1.0 }
            })
        );
    }

    #[test]
    fn rule_action_failed_event_serializes_rule_failure_fields() {
        let event = DomainEvent::rule_action_failed("bounce", 2, "missing velocity");

        let json = serde_json::to_value(BusEnvelope::new(Channel::Domain, &event))
            .expect("domain event should serialize");

        assert_eq!(
            json,
            json!({
                "channel": "domain",
                "type": "rule_action_failed",
                "rule": "bounce",
                "action_index": 2,
                "reason": "missing velocity"
            })
        );
    }

    #[test]
    fn custom_event_preserves_name_and_object_payload() {
        let mut payload = JsonMap::new();
        payload.insert("score".into(), json!(7));
        payload.insert("label".into(), json!("checkpoint"));

        let event = DomainEvent::custom("score_changed", payload);

        let json = serde_json::to_value(BusEnvelope::new(Channel::Domain, &event))
            .expect("domain event should serialize");

        assert_eq!(
            json,
            json!({
                "channel": "domain",
                "type": "score_changed",
                "score": 7,
                "label": "checkpoint"
            })
        );
    }
}
