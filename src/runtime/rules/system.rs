use crate::ecs::schedule::TickContext;
use crate::ecs::world::World;
use crate::event::bus::Bus;
use crate::runtime::rules::model::RuleSet;

/// Processes events from the `EventQueue` between ticks. **Not** a
/// `System` — runs outside `Schedule::tick` so it can publish drained
/// events to `Bus` (which systems aren't allowed to touch) and so
/// cascading rule actions can push more events back into the queue
/// without violating invariant 1.
pub struct RuleProcessor {
    rules: RuleSet,
}

impl RuleProcessor {
    #[must_use]
    pub fn new(rules: RuleSet) -> Self {
        Self { rules }
    }

    #[must_use]
    pub fn rules(&self) -> &RuleSet {
        &self.rules
    }

    /// Drains the `EventQueue` resource: for each event, finds matching
    /// rules, applies their actions. Rule actions may push new events
    /// to the queue (cascading); processing continues until the queue
    /// is empty or `max_iterations` is hit.
    ///
    /// Each drained event is also forwarded to `bus` so external
    /// observers (stdout JSONL writer, future renderer) see it.
    ///
    /// Hitting `max_iterations` publishes a `rule_cascade_limit` domain
    /// event so the agent can see the overflow in the JSONL stream.
    pub fn process(
        &self,
        _world: &mut World,
        _bus: &Bus,
        _ctx: &TickContext,
        _max_iterations: u32,
    ) {
        todo!()
    }
}
