use crate::ecs::schedule::{System, TickContext};
use crate::ecs::world::World;
use crate::physics::engine::PhysicsEngine;

/// v0 physics: naive O(n²) AABB integrator + overlap detector. No
/// resolution — rules respond to `Collision` events.
///
/// Per-tick:
///   1. `Velocity += gravity * dt`
///   2. `Position += Velocity * dt`
///   3. Pair-scan entities with both `Position` and `Aabb`; for each
///      overlap, push `DomainEvent::collision(a, b, normal)` into
///      the `EventQueue` resource.
///
/// Reads the `Gravity` resource each tick; if absent, gravity is zero.
#[derive(Default)]
pub struct AabbEngine;

impl AabbEngine {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl System for AabbEngine {
    fn name(&self) -> &str {
        "physics.aabb"
    }

    fn run(&mut self, _world: &mut World, _ctx: &TickContext) {
        todo!()
    }
}

impl PhysicsEngine for AabbEngine {}
