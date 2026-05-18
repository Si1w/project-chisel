use crate::ecs::schedule::{System, TickContext};
use crate::ecs::world::World;
use crate::event::bus::Bus;
use crate::runtime::rules::model::RuleSet;

/// Consumes `domain` and `marker` events queued during the tick and
/// applies their matched rules' actions. Publishes
/// `DomainEvent::rule_action_failed` on action failures (entity already
/// despawned, etc.) without aborting the rest of the tick.
pub struct RuleApply {
    rules: RuleSet,
}

impl RuleApply {
    #[must_use]
    pub fn new(rules: RuleSet) -> Self {
        Self { rules }
    }

    #[must_use]
    pub fn rules(&self) -> &RuleSet {
        &self.rules
    }
}

impl System for RuleApply {
    fn name(&self) -> &str {
        "runtime.rules.apply"
    }

    fn run(&mut self, _world: &mut World, _bus: &Bus, _ctx: &TickContext) {
        todo!()
    }
}
