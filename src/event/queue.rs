use std::collections::VecDeque;

use crate::ecs::resource::Resource;
use crate::event::payload::{DomainEvent, MarkerEvent};

/// Tick-local FIFO of events emitted by systems. `Resource` on the
/// `World`; populated by ECS systems during `Schedule::tick`, drained by
/// `RuleProcessor::process` between ticks. Each drained event is also
/// forwarded to `Bus` for observers.
///
/// Lives as a queue (not a buffer) so cascading is natural —
/// `RuleProcessor` does `next_domain` -> run rules -> rule actions push
/// more events to the back -> loop, with a hard `max_iterations` cap.
/// Events beyond the cap stay queued for the next tick.
///
/// `domain` and `marker` are separate FIFOs because rules subscribe to
/// them as distinct event categories; in-channel order is preserved,
/// cross-channel order is not.
#[derive(Debug, Default)]
pub struct EventQueue {
    domain: VecDeque<DomainEvent>,
    marker: VecDeque<MarkerEvent>,
}

impl Resource for EventQueue {}

impl EventQueue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn emit_domain(&mut self, event: DomainEvent) {
        self.domain.push_back(event);
    }

    pub fn emit_marker(&mut self, event: MarkerEvent) {
        self.marker.push_back(event);
    }

    pub fn next_domain(&mut self) -> Option<DomainEvent> {
        self.domain.pop_front()
    }

    pub fn next_marker(&mut self) -> Option<MarkerEvent> {
        self.marker.pop_front()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.domain.is_empty() && self.marker.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.domain.len() + self.marker.len()
    }

    #[must_use]
    pub fn domain_len(&self) -> usize {
        self.domain.len()
    }
}
