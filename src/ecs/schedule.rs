use crate::ecs::world::World;

/// Per-tick context shared by every system in a schedule. Replaces a
/// generic Resources view for these two universally-needed values.
#[derive(Copy, Clone, Debug)]
pub struct TickContext {
    pub tick: u64,
    pub dt: f32,
}

/// One unit of work per tick. Sync; sees only `&mut World` plus the
/// `TickContext`.
///
/// **Does not get `&Bus`** — invariant 1 of the architecture: ECS
/// systems don't subscribe to the bus. To emit a domain or marker
/// event, write to the `EventQueue` resource via
/// `world.resource_mut::<EventQueue>()`; the runtime forwards drained
/// events to `Bus` after the tick.
pub trait System: Send {
    /// Stable identifier used for logs and debug dumps.
    fn name(&self) -> &str;

    fn run(&mut self, world: &mut World, ctx: &TickContext);
}

/// Ordered list of systems. v0 runs them sequentially on the engine
/// thread; the `tick` signature is the same once v1+ swaps in a
/// topological scheduler.
#[derive(Default)]
pub struct Schedule {
    systems: Vec<Box<dyn System>>,
}

impl Schedule {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a system to the end. Returns `&mut Self` for chaining.
    pub fn add(&mut self, system: Box<dyn System>) -> &mut Self {
        self.systems.push(system);
        self
    }

    /// Run every system once, in insertion order.
    pub fn tick(&mut self, world: &mut World, ctx: TickContext) {
        for system in &mut self.systems {
            system.run(world, &ctx);
        }
    }
}
