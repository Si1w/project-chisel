use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::math::vec2::Vec2;
use crate::tag::id::TagId;

/// Stable identifier for a loaded rule. v0 uses the `rules/*.toml`
/// basename (e.g., `"ball-bounce"`); v1+ may intern to a typed handle.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RuleId(pub String);

/// Index into a `Rule`'s param list. v0 caps at 255 params per rule —
/// `collision` has 2, custom events typically 1–3.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ParamId(pub u8);

/// One rule, post-load. Tag names have been resolved to interned
/// `TagId`s; param refs are validated against `match_spec.params`.
#[derive(Clone, Debug)]
pub struct Rule {
    pub id: RuleId,

    /// The `domain` or `marker` event name this rule subscribes to.
    pub event_name: String,

    pub match_spec: MatchSpec,
    pub actions: Vec<Action>,
}

/// Per-event-parameter entity filter.
#[derive(Clone, Debug, Default)]
pub struct MatchSpec {
    /// Param names in declaration order. `ParamId(i).0 as usize` indexes
    /// this vec; the string is used at runtime to look up the entity in
    /// `event.payload`.
    pub params: Vec<String>,

    /// Filter for each param. Parallel to `params`.
    pub filters: Vec<EntityMatch>,
}

/// Tag-based filter on the entity bound to one match parameter.
///
/// Match passes when the entity's `TagSet` is a superset of `required`
/// and disjoint from `forbidden`.
#[derive(Clone, Debug, Default)]
pub struct EntityMatch {
    pub required: Vec<TagId>,
    pub forbidden: Vec<TagId>,
}

/// One step in `Rule::actions`. Executed in declaration order; failure
/// aborts the rest of the rule's actions and publishes
/// `DomainEvent::rule_action_failed`.
#[derive(Clone, Debug)]
pub enum Action {
    SetVelocity {
        target: ParamId,
        velocity: Vec2,
    },
    ReverseVelocity {
        target: ParamId,
        axis: ReverseAxis,
    },
    Spawn {
        template: String,
        position: Vec2,
    },
    Despawn {
        target: ParamId,
    },
    Emit {
        event: String,
        /// Payload tree with `"$name"` strings substituted with bound
        /// entities at emit time.
        payload: JsonValue,
    },
    PlayAnimation {
        target: ParamId,
        clip: String,
        priority: u8,
    },
}

/// Which velocity components to flip.
///
/// `Both` is the default because it always succeeds — `FromNormal`
/// requires the triggering event to carry a `normal` field, which is
/// only true for `collision`.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReverseAxis {
    X,
    Y,
    #[default]
    Both,
    FromNormal,
}

/// All loaded rules, indexed by their `event_name` for O(1) lookup when
/// an event arrives on the bus. Rules sharing an event name keep
/// insertion order (lexicographic by file at load time).
#[derive(Default)]
pub struct RuleSet {
    by_event: HashMap<String, Vec<Rule>>,
    total: usize,
}

impl RuleSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a rule. v0 has no priority — declaration order wins.
    pub fn add(&mut self, rule: Rule) {
        self.by_event
            .entry(rule.event_name.clone())
            .or_default()
            .push(rule);
        self.total += 1;
    }

    #[must_use]
    pub fn rules_for(&self, event_name: &str) -> &[Rule] {
        self.by_event.get(event_name).map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.total
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    /// Iterator over all rules (any event), in `(event_name, rule)`
    /// pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Rule)> + '_ {
        self.by_event
            .iter()
            .flat_map(|(event, rules)| rules.iter().map(move |r| (event.as_str(), r)))
    }
}
